//! The canvas of the interface shell: **the one place that draws**. Every shape, line, clip and
//! picture of every window of the shell goes through a [`Painter`] of this file, and nothing else
//! in `src\ui\` names Direct2D (`tests\ui_guard.rs`, rule 3).
//!
//! # How it draws
//!
//! Direct2D in **software mode** (`D2D1_RENDER_TARGET_TYPE_SOFTWARE`): the video card is not used,
//! so a window looks and costs the same on every machine. A window draws into a render target of
//! its own; a canvas in memory draws into a WIC bitmap, which is what the tests read and what the
//! element bench saves.
//!
//! **Units.** Every length handed to a [`Painter`] is in points at 100 % (96 DPI) — a device
//! independent pixel of Direct2D. The scale of the screen is given to the render target as its
//! DPI, here and nowhere else, so no caller multiplies by it (lesson of stage Э83: a buffer that
//! had to rediscover the scale lost it).
//!
//! **Nothing at rest.** `d2d1.dll` is not imported: the factory is made through
//! `LoadLibraryExW` + `GetProcAddress` when the first canvas draws its first frame, so a process
//! that never opens a window of the shell never loads it (`tests\ui_idle.rs`). The factory is
//! shared by the canvases of a thread and released with the last of them; the module itself stays
//! mapped once loaded — what that leaves in the process is measured by the probe (T-120-9).
//!
//! # When Direct2D does not come up
//!
//! A render target that could not be made twice in a row turns the canvas, for the rest of its
//! life, into the **plain look**: the same primitives drawn a second time by GDI — rectangles
//! without rounding, circles, lines, text; no pattern, no pictures. The switch is written to the
//! journal; there is no setting for it. A target lost on the way (`D2DERR_RECREATE_TARGET`) is made
//! again and the frame drawn again.
//!
//! Under the `testing` feature two variables force a mode for the probe:
//! `LANGSW_TESTING_UI_GDI` — the plain look; `LANGSW_TESTING_UI_HARDWARE` — a window target through
//! the video card, for comparison. A build without the feature has neither.
//!
//! Knows nothing of elements or windows.

use crate::diag;
use crate::ui::text::Laid;
use crate::ui::tokens::Rgb;
use ::windows::Win32::Foundation::{
    COLORREF, D2DERR_RECREATE_TARGET, HMODULE, HWND, RECT, RPC_E_CHANGED_MODE,
};
use ::windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_IGNORE, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F,
    D2D1_PIXEL_FORMAT,
};
use ::windows::Win32::Graphics::Direct2D::Common::{
    D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_END_CLOSED,
};
use ::windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DEBUG_LEVEL_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE,
    D2D1_ELLIPSE, D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_FEATURE_LEVEL_DEFAULT, D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_LAYER_OPTIONS_NONE,
    D2D1_LAYER_PARAMETERS, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE, D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_ROUNDED_RECT, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1Layer, ID2D1RenderTarget,
    ID2D1SolidColorBrush, ID2D1StrokeStyle,
};
use ::windows::Win32::Graphics::Direct2D::{D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, ID2D1Bitmap};
use ::windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use ::windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmap, IWICImagingFactory,
    WICBitmapCacheOnLoad,
};
use ::windows::Win32::Graphics::Imaging::{WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut};
use ::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::WindowsAndMessaging::{
    DI_NORMAL, DestroyIcon, DrawIconEx, HICON, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW,
};
use ::windows::core::{Error, PCWSTR};
use ::windows::core::{GUID, HRESULT, Interface, Result as WinResult};
use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::sync::OnceLock;

/// The DPI of a point at 100 %: a device independent pixel of Direct2D.
pub const POINT_DPI: f32 = 96.0;

/// Render targets that could not be made, in a row, before the canvas turns to the plain look.
pub const REFUSALS_BEFORE_PLAIN_LOOK: u8 = 2;

/// The journal's name for a render target Direct2D refused (`src\diag.rs`, the vocabulary).
pub const TARGET_REFUSED: &str = "ui canvas: target refused";
/// The journal's name for the turn of a canvas to the plain look of GDI.
pub const PLAIN_LOOK: &str = "ui canvas: plain look";

/// A rectangle in points at 100 %.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    /// This rectangle with `by` taken off every side; never narrower than nothing.
    pub fn inset(self, by: f32) -> Self {
        Self {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - 2.0 * by).max(0.0),
            h: (self.h - 2.0 * by).max(0.0),
        }
    }
}

/// How a canvas draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Direct2D, software.
    Direct2D,
    /// The plain look: GDI, no rounding, no pattern, no pictures.
    Gdi,
}

/// The pixels of a canvas in memory, row by row from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pixels: Vec<Rgb>,
}

impl Image {
    /// The pixel at `x`, `y` — in pixels, not points.
    pub fn at(&self, x: u32, y: u32) -> Rgb {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Every pixel, row by row from the top.
    pub fn pixels(&self) -> &[Rgb] {
        &self.pixels
    }
}

// ---- The shared factory ---------------------------------------------------------------------

/// The factory of this thread and how many canvases hold it.
struct Shared {
    factory: ID2D1Factory,
    holders: usize,
}

thread_local! {
    static SHARED: RefCell<Option<Shared>> = const { RefCell::new(None) };
}

/// `d2d1.dll`, loaded once by the first canvas of the process and kept (see the module notes).
static D2D1_MODULE: OnceLock<usize> = OnceLock::new();

/// The signature of `D2D1CreateFactory`, d2d1.h.
type CreateFactory = unsafe extern "system" fn(
    D2D1_FACTORY_TYPE,
    *const GUID,
    *const D2D1_FACTORY_OPTIONS,
    *mut *mut c_void,
) -> HRESULT;

/// A new single-threaded factory, from `d2d1.dll` loaded here rather than imported.
fn create_factory() -> WinResult<ID2D1Factory> {
    use ::windows::Win32::System::LibraryLoader::{
        GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
    };
    use ::windows::core::{s, w};

    let module = match D2D1_MODULE.get() {
        Some(&address) => HMODULE(address as *mut c_void),
        None => {
            // SAFETY: a NUL-terminated bare file name, searched for in System32 only. The module
            // is never freed: factories made from it may still be released after the last canvas.
            let module =
                unsafe { LoadLibraryExW(w!("d2d1.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }?;
            HMODULE(*D2D1_MODULE.get_or_init(|| module.0 as usize) as *mut c_void)
        }
    };

    // SAFETY: the documented export of d2d1.dll, by its name.
    let Some(export) = (unsafe { GetProcAddress(module, s!("D2D1CreateFactory")) }) else {
        return Err(::windows::core::Error::from_thread());
    };
    // SAFETY: `D2D1CreateFactory` has exactly the signature of [`CreateFactory`] (d2d1.h).
    let create: CreateFactory = unsafe { std::mem::transmute(export) };

    let options = D2D1_FACTORY_OPTIONS {
        debugLevel: D2D1_DEBUG_LEVEL_NONE,
    };
    let mut raw = std::ptr::null_mut();

    // SAFETY: the interface asked for is the one whose pointer is written to `raw`; the options
    // live across the call.
    unsafe {
        create(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory::IID,
            &options,
            &mut raw,
        )
    }
    .ok()?;

    // SAFETY: on success `raw` holds an owned reference to an `ID2D1Factory`.
    Ok(unsafe { ID2D1Factory::from_raw(raw) })
}

/// The factory of this thread, made on first need; the caller now holds it until
/// [`release_factory`].
fn acquire_factory() -> WinResult<ID2D1Factory> {
    SHARED.with(|cell| {
        let mut shared = cell.borrow_mut();

        if shared.is_none() {
            *shared = Some(Shared {
                factory: create_factory()?,
                holders: 0,
            });
        }

        let shared = shared.as_mut().expect("made above");
        shared.holders += 1;
        Ok(shared.factory.clone())
    })
}

/// One holder fewer; the last one releases the factory.
fn release_factory() {
    SHARED.with(|cell| {
        let mut shared = cell.borrow_mut();

        if let Some(entry) = shared.as_mut() {
            entry.holders = entry.holders.saturating_sub(1);
            if entry.holders == 0 {
                // Testing: release the last reference by hand and keep what Direct2D answers —
                // 0 when the factory dies here, more when something still holds it.
                #[cfg(feature = "testing")]
                {
                    let factory = entry.factory.clone();
                    *shared = None;
                    // SAFETY: `factory` holds one owned reference, released once here and then
                    // forgotten so that its `Drop` does not release it a second time.
                    let left =
                        unsafe { (Interface::vtable(&factory).base__.Release)(factory.as_raw()) };
                    std::mem::forget(factory);
                    FACTORY_REFS_LEFT.store(i64::from(left), std::sync::atomic::Ordering::Relaxed);
                }
                #[cfg(not(feature = "testing"))]
                {
                    *shared = None;
                }
            }
        }
    });
}

/// Testing: the references the factory of Direct2D still had when the last canvas let it go —
/// 0 means it died there (−1: not yet released). The probe of the shell reports it.
#[cfg(feature = "testing")]
pub static FACTORY_REFS_LEFT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);

/// Whether this thread holds a factory now — for the test of release.
pub fn factory_is_held() -> bool {
    SHARED.with(|cell| cell.borrow().is_some())
}

// ---- Values in Direct2D's terms -------------------------------------------------------------

fn colour(value: Rgb) -> D2D1_COLOR_F {
    let channel = |byte: u8| f32::from(byte) / 255.0;

    D2D1_COLOR_F {
        r: channel(value.r),
        g: channel(value.g),
        b: channel(value.b),
        a: 1.0,
    }
}

fn d2d_rect(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.x,
        top: rect.y,
        right: rect.right(),
        bottom: rect.bottom(),
    }
}

/// An ellipse centred on `x`, `y`. ⚠ Its point is a `windows_numerics::Vector2`, a crate this one
/// does not depend on and must not (`cargo tree` stays at 83 lines): the type is never named, its
/// fields are filled in place.
fn ellipse(x: f32, y: f32, radius: f32) -> D2D1_ELLIPSE {
    let mut ellipse = D2D1_ELLIPSE {
        radiusX: radius,
        radiusY: radius,
        ..Default::default()
    };
    ellipse.point.X = x;
    ellipse.point.Y = y;
    ellipse
}

fn target_properties(
    kind: D2D1_RENDER_TARGET_TYPE,
    scale: f32,
    opaque: bool,
) -> D2D1_RENDER_TARGET_PROPERTIES {
    D2D1_RENDER_TARGET_PROPERTIES {
        r#type: kind,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: if opaque {
                D2D1_ALPHA_MODE_IGNORE
            } else {
                D2D1_ALPHA_MODE_PREMULTIPLIED
            },
        },
        dpiX: POINT_DPI * scale,
        dpiY: POINT_DPI * scale,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    }
}

/// The type of a window's render target: software, or under `testing` the video card on demand.
fn window_target_type() -> D2D1_RENDER_TARGET_TYPE {
    #[cfg(feature = "testing")]
    if std::env::var_os("LANGSW_TESTING_UI_HARDWARE").is_some() {
        return ::windows::Win32::Graphics::Direct2D::D2D1_RENDER_TARGET_TYPE_DEFAULT;
    }

    D2D1_RENDER_TARGET_TYPE_SOFTWARE
}

/// Whether `testing` forces the plain look.
fn plain_look_forced() -> bool {
    #[cfg(feature = "testing")]
    if std::env::var_os("LANGSW_TESTING_UI_GDI").is_some() {
        return true;
    }

    false
}

// ---- The canvas -----------------------------------------------------------------------------

/// The COM apartment a canvas in memory entered to reach WIC, left with it. A thread already in
/// the other apartment kind (`RPC_E_CHANGED_MODE`) is in COM all the same and owes nothing.
struct ComEntered(bool);

impl ComEntered {
    fn enter() -> WinResult<Self> {
        // SAFETY: no reserved pointer; the calling thread's apartment only.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

        if result == RPC_E_CHANGED_MODE {
            return Ok(Self(false));
        }
        result.ok()?;
        Ok(Self(true))
    }
}

impl Drop for ComEntered {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: balances the successful `CoInitializeEx` of `enter`, on the same thread.
            unsafe { CoUninitialize() };
        }
    }
}

/// Where a canvas puts its pixels.
enum Place {
    Window(HWND),
    /// A WIC bitmap; the field order drops the bitmap before COM is left.
    Memory {
        bitmap: IWICBitmap,
        _com: ComEntered,
    },
}

/// A render target and what draws with it.
struct Target {
    render: ID2D1RenderTarget,
    window: Option<ID2D1HwndRenderTarget>,
    brush: ID2D1SolidColorBrush,
}

enum Surface {
    Direct2D {
        factory: ID2D1Factory,
        target: Option<Target>,
        place: Place,
    },
    Gdi {
        window: Option<HWND>,
        buffer: Option<gdi::Buffer>,
    },
}

/// A surface to draw whole frames on: a window, or a picture in memory.
pub struct Canvas {
    scale: f32,
    width: u32,
    height: u32,
    refusals: u8,
    surface: Surface,
}

impl Canvas {
    /// The canvas of a window whose client area is `width` × `height` pixels on a screen of
    /// `scale` (1.0 at 96 DPI). Nothing is made until the first frame.
    pub fn for_window(window: HWND, width: u32, height: u32, scale: f32) -> Self {
        let surface = if plain_look_forced() {
            Surface::Gdi {
                window: Some(window),
                buffer: None,
            }
        } else {
            match acquire_factory() {
                Ok(factory) => Surface::Direct2D {
                    factory,
                    target: None,
                    place: Place::Window(window),
                },
                Err(error) => {
                    note_plain_look(&error);
                    Surface::Gdi {
                        window: Some(window),
                        buffer: None,
                    }
                }
            }
        };

        Self {
            scale,
            width,
            height,
            refusals: 0,
            surface,
        }
    }

    /// A canvas in memory of `width` × `height` points at `scale`, drawn by `mode` — for the tests
    /// and the element bench.
    pub fn in_memory(width: f32, height: f32, scale: f32, mode: Mode) -> WinResult<Self> {
        let (width, height) = (
            (width * scale).ceil().max(1.0) as u32,
            (height * scale).ceil().max(1.0) as u32,
        );

        let surface = match mode {
            Mode::Gdi => Surface::Gdi {
                window: None,
                buffer: Some(gdi::Buffer::new(width, height)?),
            },
            Mode::Direct2D => {
                let com = ComEntered::enter()?;
                // SAFETY: COM is entered on this thread above; the class is WIC's documented one.
                let wic: IWICImagingFactory = unsafe {
                    CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
                }?;
                // SAFETY: a fresh bitmap of the given size in premultiplied BGRA, the format a
                // render target of Direct2D can draw into.
                let bitmap = unsafe {
                    wic.CreateBitmap(
                        width,
                        height,
                        &GUID_WICPixelFormat32bppPBGRA,
                        WICBitmapCacheOnLoad,
                    )
                }?;
                Surface::Direct2D {
                    factory: acquire_factory()?,
                    target: None,
                    place: Place::Memory { bitmap, _com: com },
                }
            }
        };

        Ok(Self {
            scale,
            width,
            height,
            refusals: 0,
            surface,
        })
    }

    /// How this canvas draws now.
    pub fn mode(&self) -> Mode {
        match self.surface {
            Surface::Direct2D { .. } => Mode::Direct2D,
            Surface::Gdi { .. } => Mode::Gdi,
        }
    }

    /// The scale of the screen this canvas draws for.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The size in pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The client area of the window is now `width` × `height` pixels.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        match &mut self.surface {
            Surface::Direct2D { target, .. } => {
                if let Some(window) = target.as_ref().and_then(|target| target.window.as_ref()) {
                    // SAFETY: a size by pointer to a local; the target is alive.
                    if unsafe { window.Resize(&D2D_SIZE_U { width, height }) }.is_err() {
                        *target = None;
                    }
                }
            }
            Surface::Gdi { buffer, .. } => *buffer = None,
        }
    }

    /// The window moved to a screen of `scale`.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;

        if let Surface::Direct2D {
            target: Some(target),
            ..
        } = &self.surface
        {
            // SAFETY: plain values; the target is alive.
            unsafe { target.render.SetDpi(POINT_DPI * scale, POINT_DPI * scale) };
        }
    }

    /// Draws one whole frame with `draw` and shows it; answers the mode it was drawn in. A target
    /// lost on the way is made again and the frame drawn again; a target refused twice in a row
    /// turns the canvas to the plain look, and the frame is drawn in it.
    pub fn frame(&mut self, draw: impl FnMut(&mut Painter)) -> Mode {
        self.frame_with(None, draw)
    }

    /// [`Canvas::frame`] with the pictures of the panda drawn by `pictures` (`art::Art`); without
    /// them a picture is its ground alone.
    pub fn frame_with(
        &mut self,
        mut pictures: Option<&mut (dyn Pictures + '_)>,
        mut draw: impl FnMut(&mut Painter),
    ) -> Mode {
        // Two refusals and one loss are the most a frame can meet before it is drawn somehow.
        for _ in 0..4 {
            if let Surface::Gdi { window, buffer } = &mut self.surface {
                gdi::frame(
                    *window,
                    buffer,
                    self.width,
                    self.height,
                    self.scale,
                    &mut draw,
                );
                return Mode::Gdi;
            }

            match self.draw_direct2d(&mut draw, pictures.as_deref_mut()) {
                Ok(()) => {
                    self.refusals = 0;
                    return Mode::Direct2D;
                }
                Err(error) if error.code() == D2DERR_RECREATE_TARGET => {
                    if let Surface::Direct2D { target, .. } = &mut self.surface {
                        *target = None;
                    }
                }
                Err(error) => {
                    diag::record(
                        diag::Operation::from_name(TARGET_REFUSED),
                        diag::OsCode::of(&error),
                    );
                    self.refusals = self.refusals.saturating_add(1);

                    if let Surface::Direct2D { target, .. } = &mut self.surface {
                        *target = None;
                    }
                    if self.refusals >= REFUSALS_BEFORE_PLAIN_LOOK {
                        self.turn_to_plain_look(&error);
                    }
                }
            }
        }

        self.mode()
    }

    /// One frame by Direct2D, the target made first when there is none.
    fn draw_direct2d<'o>(
        &mut self,
        draw: &mut impl FnMut(&mut Painter),
        pictures: Option<&mut (dyn Pictures + 'o)>,
    ) -> WinResult<()> {
        let (scale, width, height) = (self.scale, self.width, self.height);
        let Surface::Direct2D {
            factory,
            target,
            place,
        } = &mut self.surface
        else {
            return Ok(());
        };

        if target.is_none() {
            *target = Some(make_target(factory, place, width, height, scale)?);
        }
        let target = target.as_ref().expect("made above");

        // SAFETY: a frame on a live target, ended below on every path.
        unsafe { target.render.BeginDraw() };
        let mut painter = Painter {
            scale,
            clips: Vec::new(),
            pictures,
            kind: Kind::Direct2D {
                target: &target.render,
                brush: &target.brush,
                factory,
            },
        };
        draw(&mut painter);
        painter.unclip_all();

        // SAFETY: ends the frame begun above; no tags asked for.
        unsafe { target.render.EndDraw(None, None) }
    }

    fn turn_to_plain_look(&mut self, error: &::windows::core::Error) {
        let window = match &self.surface {
            Surface::Direct2D {
                place: Place::Window(window),
                ..
            } => Some(*window),
            _ => None,
        };

        if matches!(self.surface, Surface::Direct2D { .. }) {
            self.surface = Surface::Gdi {
                window,
                buffer: None,
            };
            release_factory();
            note_plain_look(error);
        }
    }

    /// The pixels of a canvas in memory after its last frame; `None` for a window.
    pub fn image(&self) -> Option<Image> {
        match &self.surface {
            Surface::Gdi {
                window: None,
                buffer: Some(buffer),
            } => Some(buffer.image()),
            Surface::Direct2D {
                place: Place::Memory { bitmap, .. },
                ..
            } => {
                let mut bytes = vec![0u8; (self.width * self.height * 4) as usize];
                // SAFETY: the whole bitmap (null rectangle) into a buffer of exactly its size.
                unsafe { bitmap.CopyPixels(std::ptr::null(), self.width * 4, &mut bytes) }.ok()?;
                Some(Image {
                    width: self.width,
                    height: self.height,
                    pixels: bytes
                        .chunks_exact(4)
                        .map(|bgra| Rgb {
                            r: bgra[2],
                            g: bgra[1],
                            b: bgra[0],
                        })
                        .collect(),
                })
            }
            _ => None,
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        if let Surface::Direct2D { target, .. } = &mut self.surface {
            *target = None;
            self.surface = Surface::Gdi {
                window: None,
                buffer: None,
            };
            release_factory();
        }
    }
}

/// The journal entry of a canvas that turned to the plain look.
fn note_plain_look(error: &::windows::core::Error) {
    diag::record(
        diag::Operation::from_name(PLAIN_LOOK),
        diag::OsCode::of(error),
    );
}

/// A render target for `place` and its one brush.
fn make_target(
    factory: &ID2D1Factory,
    place: &Place,
    width: u32,
    height: u32,
    scale: f32,
) -> WinResult<Target> {
    let (render, window) = match place {
        Place::Window(hwnd) => {
            let properties = target_properties(window_target_type(), scale, true);
            let window_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: *hwnd,
                pixelSize: D2D_SIZE_U { width, height },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            // SAFETY: both property blocks are locals that outlive the call.
            let window =
                unsafe { factory.CreateHwndRenderTarget(&properties, &window_properties) }?;
            (window.cast::<ID2D1RenderTarget>()?, Some(window))
        }
        Place::Memory { bitmap, .. } => {
            let properties = target_properties(D2D1_RENDER_TARGET_TYPE_SOFTWARE, scale, false);
            // SAFETY: the bitmap is alive and in a format Direct2D draws into.
            let render = unsafe { factory.CreateWicBitmapRenderTarget(bitmap, &properties) }?;
            (render, None)
        }
    };

    // SAFETY: a colour by pointer to a temporary that outlives the call; the brush's colour is
    // set again before every use.
    let brush = unsafe { render.CreateSolidColorBrush(&colour(Rgb::from_hex(0)), None) }?;

    Ok(Target {
        render,
        window,
        brush,
    })
}

// ---- The painter ----------------------------------------------------------------------------

/// A clip pushed and not yet popped.
enum Clip {
    Rect,
    Layer,
    Gdi(i32),
}

enum Kind<'a> {
    Direct2D {
        target: &'a ID2D1RenderTarget,
        brush: &'a ID2D1SolidColorBrush,
        factory: &'a ID2D1Factory,
    },
    Gdi {
        dc: ::windows::Win32::Graphics::Gdi::HDC,
    },
}

/// What draws the pictures of the panda into their circles: `art::Art` (T-120-12), which keeps each
/// picture finished at its size and scale and makes it again only when they change.
pub trait Pictures {
    /// The picture `name` into the circle of `radius` points centred on `centre`, on `target`.
    fn draw(
        &mut self,
        target: &ID2D1RenderTarget,
        name: &str,
        centre: (f32, f32),
        radius: f32,
    ) -> WinResult<()>;
}

/// The pictures of `resrt`, kept finished by `art.rs` at their size and scale.
impl Pictures for crate::ui::art::Art {
    fn draw(
        &mut self,
        target: &ID2D1RenderTarget,
        name: &str,
        centre: (f32, f32),
        radius: f32,
    ) -> WinResult<()> {
        crate::ui::art::Art::draw(self, target, name, centre, radius)
    }
}

/// What a frame draws with. Lengths are points at 100 %; colours come from the style table.
pub struct Painter<'a, 'o> {
    scale: f32,
    clips: Vec<Clip>,
    pictures: Option<&'a mut (dyn Pictures + 'o)>,
    kind: Kind<'a>,
}

impl Painter<'_, '_> {
    /// How this frame is drawn.
    pub fn mode(&self) -> Mode {
        match self.kind {
            Kind::Direct2D { .. } => Mode::Direct2D,
            Kind::Gdi { .. } => Mode::Gdi,
        }
    }

    /// Brushes the solid brush to `value` and hands it out.
    fn brush(brush: &ID2D1SolidColorBrush, value: Rgb) -> &ID2D1SolidColorBrush {
        // SAFETY: a colour by pointer to a temporary that outlives the call.
        unsafe { brush.SetColor(&colour(value)) };
        brush
    }

    /// The whole surface in one colour.
    pub fn clear(&mut self, value: Rgb) {
        match self.kind {
            // SAFETY: inside a frame; the colour outlives the call.
            Kind::Direct2D { target, .. } => unsafe { target.Clear(Some(&colour(value))) },
            Kind::Gdi { dc } => gdi::clear(dc, value),
        }
    }

    /// `rect` filled, its corners rounded by `radius`.
    pub fn fill_rect(&mut self, rect: Rect, radius: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                // SAFETY: inside a frame; the shapes are locals that outlive the calls.
                unsafe {
                    if radius > 0.0 {
                        target.FillRoundedRectangle(&rounded(rect, radius), brush);
                    } else {
                        target.FillRectangle(&d2d_rect(rect), brush);
                    }
                }
            }
            Kind::Gdi { dc } => gdi::fill_rect(dc, self.scale, rect, value),
        }
    }

    /// A frame of `width` drawn **inside** `rect`, its outer corners rounded by `radius`.
    pub fn stroke_rect(&mut self, rect: Rect, radius: f32, width: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                let half = width / 2.0;
                // SAFETY: inside a frame; the shape is a local that outlives the call.
                unsafe {
                    target.DrawRoundedRectangle(
                        &rounded(rect.inset(half), (radius - half).max(0.0)),
                        brush,
                        width,
                        None::<&ID2D1StrokeStyle>,
                    );
                }
            }
            Kind::Gdi { dc } => gdi::stroke_rect(dc, self.scale, rect, width, value),
        }
    }

    /// A disc of `radius` centred on `x`, `y`.
    pub fn fill_circle(&mut self, x: f32, y: f32, radius: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                // SAFETY: inside a frame; the shape is a local that outlives the call.
                unsafe { target.FillEllipse(&ellipse(x, y, radius), brush) };
            }
            Kind::Gdi { dc } => gdi::circle(dc, self.scale, x, y, radius, None, value),
        }
    }

    /// A ring of `width` drawn inside the circle of `radius` centred on `x`, `y`.
    pub fn stroke_circle(&mut self, x: f32, y: f32, radius: f32, width: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                let inner = (radius - width / 2.0).max(0.0);
                // SAFETY: inside a frame; the shape is a local that outlives the call.
                unsafe {
                    target.DrawEllipse(
                        &ellipse(x, y, inner),
                        brush,
                        width,
                        None::<&ID2D1StrokeStyle>,
                    )
                };
            }
            Kind::Gdi { dc } => gdi::circle(dc, self.scale, x, y, radius, Some(width), value),
        }
    }

    /// A straight line of `width` from `x0`, `y0` to `x1`, `y1`.
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                let (from, to) = (ellipse(x0, y0, 0.0).point, ellipse(x1, y1, 0.0).point);
                // SAFETY: inside a frame; plain values.
                unsafe { target.DrawLine(from, to, brush, width, None::<&ID2D1StrokeStyle>) };
            }
            Kind::Gdi { dc } => gdi::line(dc, self.scale, (x0, y0), (x1, y1), width, value),
        }
    }

    /// `laid` with its top left corner at `x`, `y` — the corner of the width it was laid out in,
    /// so a right-to-left or centred run stands inside that width by itself.
    pub fn text(&mut self, laid: &Laid, x: f32, y: f32, value: Rgb) {
        match self.kind {
            Kind::Direct2D { target, brush, .. } => {
                let brush = Self::brush(brush, value);
                let origin = ellipse(x, y, 0.0).point;
                // SAFETY: inside a frame; the layout is alive.
                unsafe {
                    target.DrawTextLayout(origin, laid.layout(), brush, D2D1_DRAW_TEXT_OPTIONS_NONE)
                };
            }
            Kind::Gdi { dc } => gdi::text(dc, self.scale, laid, x, y, value),
        }
    }

    /// The picture `name` in the circle of `radius` centred on `x`, `y`. Without pictures (before
    /// `art.rs` is in, a picture it does not have, the plain look) — the ground of the pictures
    /// alone: a work stand-in, not the look.
    pub fn picture(&mut self, name: &str, x: f32, y: f32, radius: f32) {
        let drawn = match (&self.kind, self.pictures.as_deref_mut()) {
            (Kind::Direct2D { target, .. }, Some(pictures)) => {
                pictures.draw(target, name, (x, y), radius).is_ok()
            }
            _ => false,
        };
        if !drawn {
            // The ground every picture of the panda is drawn on (res\art).
            self.fill_circle(
                x,
                y,
                radius,
                crate::ui::tokens::Color::Accent.get(crate::ui::tokens::Theme::Dark),
            );
        }
    }

    /// The program's own icon (FR-90, the icon resource of `app.rc`) in `rect`, at the pixels of the
    /// frame: the header of the window in the corner. Its bitmap is kept for the render target it
    /// was made for and remade when the target or the size changes.
    pub fn app_icon(&mut self, rect: Rect) {
        let side = (rect.w * self.scale).round().max(1.0) as i32;
        match self.kind {
            Kind::Direct2D { target, .. } => {
                let key = (target.as_raw() as usize, side);
                let bitmap = APP_ICON.with(|kept| {
                    let mut kept = kept.borrow_mut();
                    if let Some((known, bitmap)) = kept.as_ref()
                        && *known == key
                    {
                        return Some(bitmap.clone());
                    }
                    let bitmap = icon_bitmap(target, side).ok()?;
                    *kept = Some((key, bitmap.clone()));
                    Some(bitmap)
                });
                if let Some(bitmap) = bitmap {
                    let at = D2D_RECT_F {
                        left: rect.x,
                        top: rect.y,
                        right: rect.right(),
                        bottom: rect.bottom(),
                    };
                    // SAFETY: inside a frame; the bitmap belongs to this target.
                    unsafe {
                        target.DrawBitmap(
                            &bitmap,
                            Some(&at),
                            1.0,
                            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                            None,
                        );
                    }
                }
            }
            Kind::Gdi { dc } => {
                let Some(icon) = program_icon(side) else {
                    return;
                };
                let px = |points: f32| (points * self.scale).round() as i32;
                // SAFETY: the DC of this frame; the icon made above, destroyed below.
                unsafe {
                    let _ = DrawIconEx(
                        dc,
                        px(rect.x),
                        px(rect.y),
                        icon,
                        side,
                        side,
                        0,
                        None,
                        DI_NORMAL,
                    );
                    let _ = DestroyIcon(icon);
                }
            }
        }
    }
    /// The menu icon `name` (`res\art\icons`) in `value` into `rect` — a few lines drawn afresh
    /// by `art`. The plain look draws none.
    pub fn menu_icon(&mut self, name: &str, rect: Rect, value: Rgb) {
        if let Kind::Direct2D { target, .. } = self.kind {
            let _ = crate::ui::art::draw_icon(target, name, value, (rect.x, rect.y), rect.w);
        }
    }

    /// A closed polygon of `points`, filled. The plain look draws none (the cursor of a picture).
    pub fn fill_polygon(&mut self, points: &[(f32, f32)], value: Rgb) {
        let Kind::Direct2D {
            target,
            brush,
            factory,
        } = self.kind
        else {
            return;
        };
        let Some((first, rest)) = points.split_first() else {
            return;
        };
        let point = |(x, y): (f32, f32)| {
            let mut at = D2D1_ELLIPSE::default().point;
            at.X = x;
            at.Y = y;
            at
        };
        let brush = Self::brush(brush, value);
        // SAFETY: inside a frame; the geometry is made, closed and drawn here.
        unsafe {
            let Ok(geometry) = factory.CreatePathGeometry() else {
                return;
            };
            let Ok(sink) = geometry.Open() else {
                return;
            };
            sink.BeginFigure(point(*first), D2D1_FIGURE_BEGIN_FILLED);
            for at in rest {
                sink.AddLine(point(*at));
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            if sink.Close().is_ok() {
                target.FillGeometry(&geometry, brush, None);
            }
        }
    }

    /// The flag of `layout` as the tray shows it (`flags::icon`), in `rect`, at the pixels of the
    /// frame; `dark_bar` — the frame of a flag on a dark taskbar. Its bitmap is kept for the target.
    pub fn flag(&mut self, layout: crate::layouts::LayoutId, rect: Rect, dark_bar: bool) {
        let side = (rect.w * self.scale).round().max(1.0) as i32;
        let make = || {
            let look = crate::flags::Look {
                size: u32::try_from(side).unwrap_or(1),
                paused: false,
                unread: false,
                dark_bar,
            };
            crate::flags::icon(&crate::flags::face_of(layout), look)
        };
        match self.kind {
            Kind::Direct2D { target, .. } => {
                let key = (target.as_raw() as usize, layout, side, dark_bar);
                let bitmap = FLAGS.with(|kept| {
                    let mut kept = kept.borrow_mut();
                    if let Some((_, bitmap)) = kept.iter().find(|(known, _)| *known == key) {
                        return Some(bitmap.clone());
                    }
                    let bitmap = bitmap_of_icon(target, make()?).ok()?;
                    // A few flags at a time: the two layouts of the picture, at one size.
                    kept.retain(|(known, _)| known.0 == key.0 && known.2 == key.2);
                    kept.push((key, bitmap.clone()));
                    Some(bitmap)
                });
                if let Some(bitmap) = bitmap {
                    let at = D2D_RECT_F {
                        left: rect.x,
                        top: rect.y,
                        right: rect.right(),
                        bottom: rect.bottom(),
                    };
                    // SAFETY: inside a frame; the bitmap belongs to this target.
                    unsafe {
                        target.DrawBitmap(
                            &bitmap,
                            Some(&at),
                            1.0,
                            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                            None,
                        );
                    }
                }
            }
            Kind::Gdi { dc } => {
                let Some(icon) = make() else {
                    return;
                };
                let px = |points: f32| (points * self.scale).round() as i32;
                // SAFETY: the DC of this frame; the icon made above, destroyed below.
                unsafe {
                    let _ = DrawIconEx(
                        dc,
                        px(rect.x),
                        px(rect.y),
                        icon,
                        side,
                        side,
                        0,
                        None,
                        DI_NORMAL,
                    );
                    let _ = DestroyIcon(icon);
                }
            }
        }
    }

    /// The pattern of a panel: dots of `diameter` at the centre of every `step` × `step` cell of
    /// `area`. The plain look has no pattern.
    pub fn dots(&mut self, area: Rect, diameter: f32, step: f32, value: Rgb) {
        let Kind::Direct2D { target, brush, .. } = self.kind else {
            return;
        };
        if step <= 0.0 {
            return;
        }

        let brush = Self::brush(brush, value);
        let mut y = area.y + step / 2.0;
        while y < area.bottom() {
            let mut x = area.x + step / 2.0;
            while x < area.right() {
                // SAFETY: inside a frame; the shape is a local that outlives the call.
                unsafe { target.FillEllipse(&ellipse(x, y, diameter / 2.0), brush) };
                x += step;
            }
            y += step;
        }
    }

    /// What follows is drawn only inside `rect`, until [`Painter::unclip`].
    pub fn clip_rect(&mut self, rect: Rect) {
        match self.kind {
            Kind::Direct2D { target, .. } => {
                // SAFETY: inside a frame; popped by `unclip` or at the end of the frame.
                unsafe {
                    target.PushAxisAlignedClip(&d2d_rect(rect), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE)
                };
                self.clips.push(Clip::Rect);
            }
            Kind::Gdi { dc } => self.clips.push(Clip::Gdi(gdi::clip(dc, self.scale, rect))),
        }
    }

    /// What follows is drawn only inside the circle of `radius` centred on `x`, `y`, until
    /// [`Painter::unclip`]. The plain look, and a geometry Direct2D refused, clip to its square.
    pub fn clip_circle(&mut self, x: f32, y: f32, radius: f32) {
        let square = Rect::new(x - radius, y - radius, 2.0 * radius, 2.0 * radius);
        let Kind::Direct2D {
            target, factory, ..
        } = self.kind
        else {
            return self.clip_rect(square);
        };

        // SAFETY: the shape is a local that outlives the call.
        let Ok(geometry) = (unsafe { factory.CreateEllipseGeometry(&ellipse(x, y, radius)) })
        else {
            return self.clip_rect(square);
        };
        let Ok(mask) = geometry.cast() else {
            return self.clip_rect(square);
        };

        let mut parameters = D2D1_LAYER_PARAMETERS {
            contentBounds: D2D_RECT_F {
                left: f32::MIN,
                top: f32::MIN,
                right: f32::MAX,
                bottom: f32::MAX,
            },
            geometricMask: ManuallyDrop::new(Some(mask)),
            maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            opacity: 1.0,
            opacityBrush: ManuallyDrop::new(None),
            layerOptions: D2D1_LAYER_OPTIONS_NONE,
            ..Default::default()
        };
        // The identity transform. ⚠ The default of `Matrix3x2` is all zeros, which would fold the
        // mask to a point and hide everything; its type is not named, as with `Vector2` above.
        parameters.maskTransform.M11 = 1.0;
        parameters.maskTransform.M22 = 1.0;

        // SAFETY: inside a frame; the layer may be null since Windows 8 (Direct2D makes and keeps
        // one); the parameters outlive the call, which takes its own reference to the mask.
        unsafe { target.PushLayer(&parameters, None::<&ID2D1Layer>) };
        // SAFETY: the reference this function made, dropped once.
        unsafe { ManuallyDrop::drop(&mut parameters.geometricMask) };
        self.clips.push(Clip::Layer);
    }

    /// Ends the last clip.
    pub fn unclip(&mut self) {
        let Some(clip) = self.clips.pop() else {
            return;
        };

        match (&self.kind, clip) {
            // SAFETY: pops what `clip_rect` pushed, inside the same frame.
            (Kind::Direct2D { target, .. }, Clip::Rect) => unsafe { target.PopAxisAlignedClip() },
            // SAFETY: pops what `clip_circle` pushed, inside the same frame.
            (Kind::Direct2D { target, .. }, Clip::Layer) => unsafe { target.PopLayer() },
            (Kind::Gdi { dc }, Clip::Gdi(saved)) => gdi::unclip(*dc, saved),
            _ => {}
        }
    }

    /// Every clip still pushed, ended: a frame must close what it opened.
    fn unclip_all(&mut self) {
        while !self.clips.is_empty() {
            self.unclip();
        }
    }
}

fn rounded(rect: Rect, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: d2d_rect(rect),
        radiusX: radius,
        radiusY: radius,
    }
}

// ---- The plain look -------------------------------------------------------------------------

/// The plain look: the primitives of [`Painter`] by GDI — no rounding, no pattern, no pictures.
/// Drawn into a buffer of its own and copied to the window in one piece, so a frame never shows
/// half drawn.
mod gdi {
    use super::{COLORREF, HWND, Image, Laid, Painter, RECT, Rect, Rgb};
    use crate::ui::text::{Align, Direction, Wrap};
    use ::windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
        CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen, CreateSolidBrush,
        DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOCLIP,
        DT_NOPREFIX, DT_RIGHT, DT_RTLREADING, DT_SINGLELINE, DT_WORDBREAK, DeleteDC, DeleteObject,
        DrawTextW, Ellipse, FillRect, GdiFlush, GetDC, GetStockObject, HBITMAP, HDC, HGDIOBJ,
        IntersectClipRect, LineTo, MoveToEx, NULL_BRUSH, NULL_PEN, OUT_DEFAULT_PRECIS,
        PS_INSIDEFRAME, PS_SOLID, ReleaseDC, RestoreDC, SRCCOPY, SaveDC, SelectObject, SetBkMode,
        SetTextColor, TRANSPARENT,
    };
    use ::windows::Win32::UI::WindowsAndMessaging::IsWindow;
    use ::windows::core::{HSTRING, Result as WinResult};
    use std::ffi::c_void;

    /// A top-down 32-bit picture selected into a memory DC.
    pub(super) struct Buffer {
        dc: HDC,
        bitmap: HBITMAP,
        previous: HGDIOBJ,
        bits: *mut c_void,
        width: u32,
        height: u32,
    }

    impl Buffer {
        pub(super) fn new(width: u32, height: u32) -> WinResult<Self> {
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            // SAFETY: a memory DC compatible with the screen; deleted in `Drop`.
            let dc = unsafe { CreateCompatibleDC(None) };
            let mut bits = std::ptr::null_mut();
            // SAFETY: the header describes the picture; `bits` receives its memory, owned by the
            // bitmap, which `Drop` deletes.
            let bitmap = match unsafe {
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            } {
                Ok(bitmap) => bitmap,
                Err(error) => {
                    // SAFETY: the DC made above, unused.
                    let _ = unsafe { DeleteDC(dc) };
                    return Err(error);
                }
            };
            // SAFETY: both handles are alive; the previous object is put back in `Drop`.
            let previous = unsafe { SelectObject(dc, bitmap.into()) };

            Ok(Self {
                dc,
                bitmap,
                previous,
                bits,
                width,
                height,
            })
        }

        pub(super) fn image(&self) -> Image {
            // SAFETY: no arguments; GDI finishes what it queued on this thread.
            let _ = unsafe { GdiFlush() };
            let count = (self.width * self.height) as usize;
            // SAFETY: the DIB holds `width × height` pixels of four bytes, alive with `self`.
            let words = unsafe { std::slice::from_raw_parts(self.bits.cast::<u32>(), count) };

            Image {
                width: self.width,
                height: self.height,
                // A DIB pixel is blue, green, red and an unused byte, in memory order.
                pixels: words
                    .iter()
                    .map(|word| {
                        let [b, g, r, _] = word.to_le_bytes();
                        Rgb { r, g, b }
                    })
                    .collect(),
            }
        }
    }

    impl Drop for Buffer {
        fn drop(&mut self) {
            // SAFETY: the handles made in `new`, released once, the bitmap deselected first.
            unsafe {
                SelectObject(self.dc, self.previous);
                let _ = DeleteObject(self.bitmap.into());
                let _ = DeleteDC(self.dc);
            }
        }
    }

    /// One frame of the plain look: into the buffer, then onto the window if there is one.
    pub(super) fn frame(
        window: Option<HWND>,
        buffer: &mut Option<Buffer>,
        width: u32,
        height: u32,
        scale: f32,
        draw: &mut impl FnMut(&mut Painter),
    ) {
        if buffer.is_none() {
            *buffer = Buffer::new(width.max(1), height.max(1)).ok();
        }
        let Some(buffer) = buffer.as_ref() else {
            return;
        };

        // SAFETY: the DC is restored to this level after the frame.
        let level = unsafe { SaveDC(buffer.dc) };
        let mut painter = Painter {
            scale,
            clips: Vec::new(),
            pictures: None,
            kind: super::Kind::Gdi { dc: buffer.dc },
        };
        draw(&mut painter);
        painter.unclip_all();
        // SAFETY: back to the level saved above.
        let _ = unsafe { RestoreDC(buffer.dc, level) };

        // SAFETY: a window handle by value; nothing is drawn unless it names a live window.
        if let Some(window) = window.filter(|window| unsafe { IsWindow(Some(*window)) }.as_bool()) {
            // SAFETY: the window's DC, released below; the buffer is alive.
            unsafe {
                let dc = GetDC(Some(window));
                let _ = BitBlt(
                    dc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    Some(buffer.dc),
                    0,
                    0,
                    SRCCOPY,
                );
                ReleaseDC(Some(window), dc);
            }
        }
    }

    fn px(value: f32, scale: f32) -> i32 {
        (value * scale).round() as i32
    }

    fn rect(area: Rect, scale: f32) -> RECT {
        RECT {
            left: px(area.x, scale),
            top: px(area.y, scale),
            right: px(area.right(), scale),
            bottom: px(area.bottom(), scale),
        }
    }

    fn colorref(value: Rgb) -> COLORREF {
        COLORREF(u32::from(value.r) | (u32::from(value.g) << 8) | (u32::from(value.b) << 16))
    }

    /// Runs `paint` with a pen and a brush selected, and deletes what it made.
    fn with(dc: HDC, pen: Option<HGDIOBJ>, brush: Option<HGDIOBJ>, paint: impl FnOnce()) {
        // SAFETY: stock objects need no deleting; what is selected is put back below.
        let (pen, made_pen) = pen.map_or((unsafe { GetStockObject(NULL_PEN) }, false), |pen| {
            (pen, true)
        });
        let (brush, made_brush) = brush
            .map_or((unsafe { GetStockObject(NULL_BRUSH) }, false), |brush| {
                (brush, true)
            });

        // SAFETY: live handles; the previous ones are restored before the new ones are deleted.
        unsafe {
            let old_pen = SelectObject(dc, pen);
            let old_brush = SelectObject(dc, brush);
            paint();
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            if made_pen {
                let _ = DeleteObject(pen);
            }
            if made_brush {
                let _ = DeleteObject(brush);
            }
        }
    }

    /// The text of `laid` by `DrawTextW`, in a GDI font of the same family, size and weight.
    pub(super) fn text(dc: HDC, scale: f32, laid: &Laid, x: f32, y: f32, value: Rgb) {
        let style = laid.style();
        let rtl = laid.direction() == Direction::RightToLeft;
        let mut wide: Vec<u16> = laid.shown().encode_utf16().collect();
        let mut flags = DT_NOPREFIX | DT_NOCLIP;
        flags |= match laid.wrap() {
            Wrap::Words => DT_WORDBREAK,
            Wrap::OneLine => DT_SINGLELINE | DT_END_ELLIPSIS,
        };
        flags |= match (laid.align(), rtl) {
            (Align::Centre, _) => DT_CENTER,
            (Align::Start, false) | (Align::End, true) => DT_LEFT,
            _ => DT_RIGHT,
        };
        if rtl {
            flags |= DT_RTLREADING;
        }
        let height = laid.size().height.max(style.line_height());
        let mut area = rect(Rect::new(x, y, laid.width(), height), scale);

        // SAFETY: a font deleted below, after the previous one is selected back; the text and the
        // rectangle are locals.
        unsafe {
            let font = CreateFontW(
                -px(style.size, scale),
                0,
                0,
                0,
                i32::from(style.weight),
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                0,
                &HSTRING::from(laid.family()),
            );
            let previous = SelectObject(dc, font.into());
            SetTextColor(dc, colorref(value));
            SetBkMode(dc, TRANSPARENT);
            DrawTextW(dc, &mut wide, &mut area, flags);
            SelectObject(dc, previous);
            let _ = DeleteObject(font.into());
        }
    }

    pub(super) fn clear(dc: HDC, value: Rgb) {
        // SAFETY: a brush deleted below; a rectangle wider than any buffer, clipped by GDI.
        unsafe {
            let brush = CreateSolidBrush(colorref(value));
            FillRect(
                dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: i32::MAX / 2,
                    bottom: i32::MAX / 2,
                },
                brush,
            );
            let _ = DeleteObject(brush.into());
        }
    }

    pub(super) fn fill_rect(dc: HDC, scale: f32, area: Rect, value: Rgb) {
        // SAFETY: a brush deleted below.
        unsafe {
            let brush = CreateSolidBrush(colorref(value));
            FillRect(dc, &rect(area, scale), brush);
            let _ = DeleteObject(brush.into());
        }
    }

    fn pen(scale: f32, width: f32, value: Rgb, inside: bool) -> HGDIOBJ {
        let style = if inside { PS_INSIDEFRAME } else { PS_SOLID };
        // SAFETY: plain values; the pen is deleted by `with`.
        unsafe { CreatePen(style, px(width, scale).max(1), colorref(value)) }.into()
    }

    pub(super) fn stroke_rect(dc: HDC, scale: f32, area: Rect, width: f32, value: Rgb) {
        let r = rect(area, scale);
        with(dc, Some(pen(scale, width, value, true)), None, || {
            // SAFETY: the pen of `with` is selected; plain values.
            let _ = unsafe {
                ::windows::Win32::Graphics::Gdi::Rectangle(dc, r.left, r.top, r.right, r.bottom)
            };
        });
    }

    pub(super) fn circle(
        dc: HDC,
        scale: f32,
        x: f32,
        y: f32,
        radius: f32,
        ring: Option<f32>,
        value: Rgb,
    ) {
        let r = rect(
            Rect::new(x - radius, y - radius, 2.0 * radius, 2.0 * radius),
            scale,
        );
        let (pen, brush) = match ring {
            Some(width) => (Some(pen(scale, width, value, true)), None),
            // SAFETY: a brush deleted by `with`.
            None => (
                None,
                Some(unsafe { CreateSolidBrush(colorref(value)) }.into()),
            ),
        };
        with(dc, pen, brush, || {
            // SAFETY: the objects of `with` are selected; plain values.
            let _ = unsafe { Ellipse(dc, r.left, r.top, r.right, r.bottom) };
        });
    }

    pub(super) fn line(
        dc: HDC,
        scale: f32,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        value: Rgb,
    ) {
        with(dc, Some(pen(scale, width, value, false)), None, || {
            // SAFETY: the pen of `with` is selected; plain values.
            unsafe {
                let _ = MoveToEx(dc, px(from.0, scale), px(from.1, scale), None);
                let _ = LineTo(dc, px(to.0, scale), px(to.1, scale));
            }
        });
    }

    /// Clips to `area`; answers the level to restore.
    pub(super) fn clip(dc: HDC, scale: f32, area: Rect) -> i32 {
        let r = rect(area, scale);
        // SAFETY: the level is restored by `unclip`.
        unsafe {
            let level = SaveDC(dc);
            IntersectClipRect(dc, r.left, r.top, r.right, r.bottom);
            level
        }
    }

    pub(super) fn unclip(dc: HDC, level: i32) {
        // SAFETY: a level `clip` saved on this DC.
        let _ = unsafe { RestoreDC(dc, level) };
    }
}

/// The bench of elements writes its sheets as pictures (T-120-6); a build without `testing` has no
/// way to write a file from a canvas.
#[cfg(feature = "testing")]
impl Canvas {
    /// The pixels of a canvas in memory, after its last frame, as a PNG file at `path`.
    pub fn save_png(&self, path: &std::path::Path) -> WinResult<()> {
        use ::windows::Win32::Foundation::GENERIC_WRITE;
        use ::windows::Win32::Graphics::Imaging::{
            GUID_ContainerFormatPng, WICBitmapEncoderNoCache,
        };
        use ::windows::core::HSTRING;

        let Surface::Direct2D {
            place: Place::Memory { bitmap, .. },
            ..
        } = &self.surface
        else {
            return Err(::windows::core::Error::empty());
        };

        // SAFETY: COM is entered by this canvas (`ComEntered`); every out parameter is a local and
        // every interface lives across the calls that use it.
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let stream = wic.CreateStream()?;
            stream.InitializeFromFilename(&HSTRING::from(path.as_os_str()), GENERIC_WRITE.0)?;
            let encoder = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
            encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
            let frame = new_frame(&encoder)?;
            frame.WriteSource(bitmap, std::ptr::null())?;
            frame.Commit()?;
            encoder.Commit()
        }
    }
}

/// A new frame of `encoder`, made and initialised with no options.
///
/// ⚠ `IWICBitmapEncoder::CreateNewFrame` and `IWICBitmapFrameEncode::Initialize` take an
/// `IPropertyBag2`, and the `windows` crate shows them only under `Win32_System_Com_StructuredStorage`
/// — a feature the closed list of this program does not have (решение 208.169). Both methods are
/// documented (wincodec.h) and so is their place in the table of methods: `CreateNewFrame` follows
/// `SetPreview`, `Initialize` follows the three methods of `IUnknown`. They are taken from there,
/// beside the slots the crate does name. Testing only: the bench of elements.
#[cfg(feature = "testing")]
unsafe fn new_frame(
    encoder: &::windows::Win32::Graphics::Imaging::IWICBitmapEncoder,
) -> WinResult<::windows::Win32::Graphics::Imaging::IWICBitmapFrameEncode> {
    use ::windows::Win32::Graphics::Imaging::IWICBitmapFrameEncode;
    use ::windows::core::{IUnknown, IUnknown_Vtbl};

    type CreateNewFrame =
        unsafe extern "system" fn(*mut c_void, *mut *mut c_void, *mut *mut c_void) -> HRESULT;
    type Initialize = unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT;

    // SAFETY: the table is the live encoder's; the slot after `SetPreview` is `CreateNewFrame`.
    let create: CreateNewFrame = unsafe {
        let after = (&encoder.vtable().SetPreview as *const _ as *const usize).add(1);
        std::mem::transmute::<usize, CreateNewFrame>(*after)
    };
    let (mut frame, mut options) = (std::ptr::null_mut(), std::ptr::null_mut());
    // SAFETY: both out parameters are locals; on success each holds an owned reference.
    unsafe { create(encoder.as_raw(), &mut frame, &mut options) }.ok()?;
    // SAFETY: owned references from the call above, released when these values drop.
    let frame = unsafe { IWICBitmapFrameEncode::from_raw(frame) };
    let _options = (!options.is_null()).then(|| unsafe { IUnknown::from_raw(options) });

    // SAFETY: the table is the live frame's; the first slot after `IUnknown`'s is `Initialize`.
    let initialize: Initialize = unsafe {
        let base = &frame.vtable().base__ as *const IUnknown_Vtbl as *const usize;
        let after = base.add(size_of::<IUnknown_Vtbl>() / size_of::<usize>());
        std::mem::transmute::<usize, Initialize>(*after)
    };
    // SAFETY: no options — documented as allowed.
    unsafe { initialize(frame.as_raw(), std::ptr::null_mut()) }.ok()?;
    Ok(frame)
}

/// The icon resource of the program in `app.rc` (FR-90) — the one the window in the corner of
/// E119 put in its header.
const PROGRAM_ICON: u16 = 101;

thread_local! {
    /// The program's icon made into a bitmap of one render target at one size.
    static APP_ICON: std::cell::RefCell<Option<((usize, i32), ID2D1Bitmap)>> =
        const { std::cell::RefCell::new(None) };
    /// The flags of layouts made into bitmaps: by target, layout, side and the dark taskbar.
    static FLAGS: std::cell::RefCell<FlagBitmaps> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The flags kept for a frame: (target, layout, side, dark taskbar) and the bitmap.
type FlagBitmaps = Vec<((usize, crate::layouts::LayoutId, i32, bool), ID2D1Bitmap)>;

/// The program's icon at `side` pixels, made by Windows from its resource; the caller destroys it.
fn program_icon(side: i32) -> Option<HICON> {
    // SAFETY: `None` names this executable; the handle is borrowed.
    let module = unsafe { GetModuleHandleW(None) }.ok()?;
    // SAFETY: an icon resource of this module, by its number.
    let handle = unsafe {
        LoadImageW(
            Some(module.into()),
            PCWSTR(usize::from(PROGRAM_ICON) as *const u16),
            IMAGE_ICON,
            side,
            side,
            LR_DEFAULTCOLOR,
        )
    }
    .ok()?;
    Some(HICON(handle.0))
}

/// The program's icon as a bitmap of `target`.
fn icon_bitmap(target: &ID2D1RenderTarget, side: i32) -> WinResult<ID2D1Bitmap> {
    bitmap_of_icon(target, program_icon(side).ok_or_else(Error::from_thread)?)
}

/// `icon` as a bitmap of `target`, through WIC: the icon's own pixels, premultiplied; the icon is
/// destroyed once copied.
fn bitmap_of_icon(target: &ID2D1RenderTarget, icon: HICON) -> WinResult<ID2D1Bitmap> {
    // SAFETY: WIC of this thread (the window's COM); the icon is destroyed once copied.
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let pixels = wic.CreateBitmapFromHICON(icon);
        let _ = DestroyIcon(icon);
        let pixels = pixels?;
        let converter = wic.CreateFormatConverter()?;
        converter.Initialize(
            &pixels,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )?;
        target.CreateBitmapFromWicBitmap(&converter, None)
    }
}
