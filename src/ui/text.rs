//! Text of the interface shell: DirectWrite — the system's faces by role of the passport, the
//! measure of a run of text, wrapping by words, an ellipsis on one line, and the reading direction
//! of the program's language. The canvas draws what this file lays out
//! ([`crate::ui::canvas::Painter::text`]); nothing else in `src\ui\` names DirectWrite.
//!
//! **The faces are the system's** (passport §2, решение 208.169): «Segoe UI Variable Text» where
//! Windows has it, «Segoe UI» where it does not (Windows 10); «Cascadia Mono», else «Consolas».
//! Chosen once, when a [`Text`] is made, from the system font collection. A script the face lacks
//! is drawn by the font Windows falls back to; the program carries no font files.
//!
//! **Names inside a right-to-left line** — a Latin name, a path, a version, a key — are set by the
//! caller as embeddings through [`crate::theme::embedded_name`] (LRE or RLE … PDF between RLMs),
//! never as the isolates of Unicode 6.3: the rule of stage E108 and of the passport §7, kept for
//! DirectWrite as it was for GDI so that one string reads the same in both looks.
//!
//! **Digits are written as the strings carry them**: no number substitution is asked of DirectWrite,
//! so «1.8.0» shows European digits in every language, as the program's strings and the stands'
//! locales show them today (`letters::format_date`, the note on `LOCALE_IDIGITSUBSTITUTION`).
//!
//! **Nothing at rest**: `dwrite.dll` is loaded through `LoadLibraryExW` by the first [`Text`], not
//! imported (`tests\ui_idle.rs`).
//!
//! Knows nothing of windows.

use crate::ui::tokens::{self, Face, Role};
use ::windows::Win32::Foundation::{HMODULE, LPARAM};
use ::windows::Win32::Globalization::{LCMAP_LINGUISTIC_CASING, LCMAP_UPPERCASE, LCMapStringEx};
use ::windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_HIT_TEST_METRICS, DWRITE_LINE_SPACING_METHOD_UNIFORM, DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
    DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP,
    IDWriteFactory, IDWriteFontCollection, IDWriteTextLayout, IDWriteTextLayout1,
};
use ::windows::core::{BOOL, GUID, HRESULT, HSTRING, Interface, Result as WinResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::OnceLock;

/// The main faces, in the order they are looked for.
pub const MAIN_FACES: [&str; 2] = ["Segoe UI Variable Text", "Segoe UI"];
/// The monospace faces, in the order they are looked for.
pub const MONO_FACES: [&str; 2] = ["Cascadia Mono", "Consolas"];

/// A layout's height when nothing bounds it: far beyond any window.
const UNBOUNDED: f32 = 1.0e6;

/// The reading direction of the program's language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    LeftToRight,
    RightToLeft,
}

/// How a run of text takes a width it does not fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// More lines, broken between words.
    Words,
    /// One line, cut with an ellipsis.
    OneLine,
}

/// Where the lines stand in their width. `Start` is the edge the language reads from: the left
/// in a left-to-right language, the right in a right-to-left one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Centre,
    End,
}

/// How a run of text is set: a role of the passport at a window's type size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// Points.
    pub size: f32,
    /// The line height as a multiple of `size`.
    pub line: f32,
    pub weight: u16,
    pub face: Face,
    pub caps: bool,
    /// Letter spacing as a share of `size`.
    pub tracking: f32,
}

impl Style {
    /// `role` in a window of type size `b` whose client area is `h` points high — the floor of
    /// 12 included ([`tokens::role_size`]).
    pub fn of(role: Role, b: f32, h: f32) -> Self {
        let style = role.style();

        Self {
            size: tokens::role_size(role, b, h),
            line: style.line,
            weight: style.weight,
            face: style.face,
            caps: style.caps,
            tracking: style.tracking,
        }
    }

    /// The same style in another weight: the main button, the name in the brand line.
    pub fn with_weight(self, weight: u16) -> Self {
        Self { weight, ..self }
    }

    /// The height of one line, in points.
    pub fn line_height(self) -> f32 {
        self.size * self.line
    }
}

/// What a run of text takes, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
    pub lines: u32,
}

/// What measures text. Two implementations: [`Text`], the real one, and [`FixedAdvance`], a
/// stand-in the tests of the layout use so that their answers do not hang on a font.
pub trait Measure {
    /// `text` set in `style` within `width` points.
    fn measure(&self, text: &str, style: Style, wrap: Wrap, width: f32) -> Size;
}

/// A stand-in measure: every character advances its share of the size; lines break between words,
/// greedily, and a word wider than the line takes a line of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedAdvance(pub f32);

impl Measure for FixedAdvance {
    fn measure(&self, text: &str, style: Style, wrap: Wrap, width: f32) -> Size {
        let advance = self.0 * style.size;
        let length = |word: &str| word.chars().count() as f32 * advance;

        if wrap == Wrap::OneLine || text.is_empty() {
            return Size {
                width: length(text).min(width),
                height: style.line_height(),
                lines: 1,
            };
        }

        let mut lines = 1;
        let mut widest = 0.0_f32;
        let mut current = 0.0_f32;
        for word in text.split(' ') {
            let word = length(word);
            let joined = if current > 0.0 {
                current + advance + word
            } else {
                word
            };

            if joined > width && current > 0.0 {
                lines += 1;
                widest = widest.max(current);
                current = word;
            } else {
                current = joined;
            }
        }
        widest = widest.max(current);

        Size {
            width: widest,
            height: style.line_height() * lines as f32,
            lines,
        }
    }
}

// ---- DirectWrite ----------------------------------------------------------------------------

/// `dwrite.dll`, loaded once by the first [`Text`] of the process and kept.
static DWRITE_MODULE: OnceLock<usize> = OnceLock::new();

/// The signature of `DWriteCreateFactory`, dwrite.h.
type CreateFactory =
    unsafe extern "system" fn(DWRITE_FACTORY_TYPE, *const GUID, *mut *mut c_void) -> HRESULT;

/// The shared factory of DirectWrite, from `dwrite.dll` loaded here rather than imported.
fn create_factory() -> WinResult<IDWriteFactory> {
    use ::windows::Win32::System::LibraryLoader::{
        GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
    };
    use ::windows::core::{s, w};

    let module = match DWRITE_MODULE.get() {
        Some(&address) => HMODULE(address as *mut c_void),
        None => {
            // SAFETY: a NUL-terminated bare file name, searched for in System32 only; never freed.
            let module =
                unsafe { LoadLibraryExW(w!("dwrite.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }?;
            HMODULE(*DWRITE_MODULE.get_or_init(|| module.0 as usize) as *mut c_void)
        }
    };

    // SAFETY: the documented export of dwrite.dll, by its name.
    let Some(export) = (unsafe { GetProcAddress(module, s!("DWriteCreateFactory")) }) else {
        return Err(::windows::core::Error::from_thread());
    };
    // SAFETY: `DWriteCreateFactory` has exactly the signature of [`CreateFactory`] (dwrite.h).
    let create: CreateFactory = unsafe { std::mem::transmute(export) };
    let mut raw = std::ptr::null_mut();

    // SAFETY: the interface asked for is the one whose pointer is written to `raw`.
    unsafe { create(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID, &mut raw) }.ok()?;

    // SAFETY: on success `raw` holds an owned reference to an `IDWriteFactory`.
    Ok(unsafe { IDWriteFactory::from_raw(raw) })
}

/// The ascent and descent of a face, as shares of its em.
#[derive(Clone, Copy, Debug)]
struct Vertical {
    ascent: f32,
    descent: f32,
}

/// The faces of the passport and the language of a window: what lays text out.
pub struct Text {
    factory: IDWriteFactory,
    collection: IDWriteFontCollection,
    main: String,
    mono: String,
    locale: String,
    direction: Direction,
    verticals: RefCell<HashMap<(Face, u16), Vertical>>,
}

impl Text {
    /// The system's faces (passport §2), for a window in the language of `locale`, read in
    /// `direction`.
    pub fn new(locale: &str, direction: Direction) -> WinResult<Self> {
        let mut text = Self::with_faces(MAIN_FACES[1], MONO_FACES[1], locale, direction)?;
        let first = |faces: [&str; 2]| {
            faces
                .into_iter()
                .find(|face| text.has_face(face))
                .unwrap_or(faces[1])
                .to_owned()
        };
        let (main, mono) = (first(MAIN_FACES), first(MONO_FACES));
        text.main = main;
        text.mono = mono;
        Ok(text)
    }

    /// The named faces — for the tests, which run with each main face in turn.
    pub fn with_faces(
        main: &str,
        mono: &str,
        locale: &str,
        direction: Direction,
    ) -> WinResult<Self> {
        let factory = create_factory()?;
        let mut collection = None;
        // SAFETY: the out parameter is a local; no update check is asked for.
        unsafe { factory.GetSystemFontCollection(&mut collection, false) }?;
        let collection = collection.ok_or_else(::windows::core::Error::empty)?;

        Ok(Self {
            factory,
            collection,
            main: main.to_owned(),
            mono: mono.to_owned(),
            locale: locale.to_owned(),
            direction,
            verticals: RefCell::new(HashMap::new()),
        })
    }

    /// Whether the system has a face of this family name.
    pub fn has_face(&self, family: &str) -> bool {
        self.family_index(family).is_some()
    }

    fn family_index(&self, family: &str) -> Option<u32> {
        let (mut index, mut exists) = (0, BOOL(0));
        // SAFETY: the name lives across the call; the out parameters are locals.
        unsafe {
            self.collection
                .FindFamilyName(&HSTRING::from(family), &mut index, &mut exists)
        }
        .ok()?;
        exists.as_bool().then_some(index)
    }

    /// The main face in use.
    pub fn main_face(&self) -> &str {
        &self.main
    }

    /// The monospace face in use.
    pub fn mono_face(&self) -> &str {
        &self.mono
    }

    /// The reading direction of the window.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    fn family(&self, face: Face) -> &str {
        match face {
            Face::Main => &self.main,
            Face::Mono => &self.mono,
        }
    }

    /// The ascent and descent of `face` in `weight`; a face that cannot be read answers the
    /// proportions of Segoe UI, which are what the passport's line heights were drawn with.
    fn vertical(&self, face: Face, weight: u16) -> Vertical {
        if let Some(found) = self.verticals.borrow().get(&(face, weight)) {
            return *found;
        }

        let read = || -> Option<Vertical> {
            let index = self.family_index(self.family(face))?;
            // SAFETY: an index the collection itself answered.
            let family = unsafe { self.collection.GetFontFamily(index) }.ok()?;
            // SAFETY: plain values.
            let font = unsafe {
                family.GetFirstMatchingFont(
                    DWRITE_FONT_WEIGHT(i32::from(weight)),
                    DWRITE_FONT_STRETCH_NORMAL,
                    DWRITE_FONT_STYLE_NORMAL,
                )
            }
            .ok()?;
            let mut metrics = DWRITE_FONT_METRICS::default();
            // SAFETY: the out parameter is a local.
            unsafe { font.GetMetrics(&mut metrics) };
            let em = f32::from(metrics.designUnitsPerEm.max(1));
            Some(Vertical {
                ascent: f32::from(metrics.ascent) / em,
                descent: f32::from(metrics.descent) / em,
            })
        };
        // Segoe UI: 2210 and 514 units of 2048.
        let vertical = read().unwrap_or(Vertical {
            ascent: 1.079,
            descent: 0.251,
        });

        self.verticals.borrow_mut().insert((face, weight), vertical);
        vertical
    }

    /// `text` in `style`, within `width` points, ready for the canvas.
    pub fn lay(
        &self,
        text: &str,
        style: Style,
        wrap: Wrap,
        align: Align,
        width: f32,
    ) -> WinResult<Laid> {
        let shown = if style.caps {
            upper(&self.locale, text)
        } else {
            text.to_owned()
        };
        let wide: Vec<u16> = shown.encode_utf16().collect();
        let family = self.family(style.face).to_owned();

        // SAFETY: the strings live across the call.
        let format = unsafe {
            self.factory.CreateTextFormat(
                &HSTRING::from(family.as_str()),
                None,
                DWRITE_FONT_WEIGHT(i32::from(style.weight)),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                style.size,
                &HSTRING::from(self.locale.as_str()),
            )
        }?;

        // The baseline of a uniform line puts the face's own height in the middle of the line, as
        // the line-height of CSS does in the mock-ups.
        let vertical = self.vertical(style.face, style.weight);
        let line = style.line_height();
        let baseline = (line - (vertical.ascent + vertical.descent) * style.size) / 2.0
            + vertical.ascent * style.size;

        // SAFETY: plain values on a live format; the trimming options are a local.
        unsafe {
            format.SetReadingDirection(match self.direction {
                Direction::LeftToRight => DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
                Direction::RightToLeft => DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
            })?;
            format.SetTextAlignment(alignment(align))?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            format.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, line, baseline)?;

            match wrap {
                Wrap::Words => format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?,
                Wrap::OneLine => {
                    format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
                    let sign = self.factory.CreateEllipsisTrimmingSign(&format)?;
                    let trimming = DWRITE_TRIMMING {
                        granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                        ..Default::default()
                    };
                    format.SetTrimming(&trimming, &sign)?;
                }
            }
        }

        let height = if wrap == Wrap::OneLine {
            line
        } else {
            UNBOUNDED
        };
        // SAFETY: the string and the format live across the call.
        let layout = unsafe { self.factory.CreateTextLayout(&wide, &format, width, height) }?;

        if style.tracking > 0.0 {
            let range = DWRITE_TEXT_RANGE {
                startPosition: 0,
                length: wide.len() as u32,
            };
            // SAFETY: a range of the layout's own text; IDWriteTextLayout1 is Windows 8 and up.
            unsafe {
                layout.cast::<IDWriteTextLayout1>()?.SetCharacterSpacing(
                    0.0,
                    style.tracking * style.size,
                    0.0,
                    range,
                )
            }?;
        }

        Ok(Laid {
            layout,
            shown,
            family,
            style,
            baseline,
            direction: self.direction,
            align,
            wrap,
            width,
        })
    }
}

impl Measure for Text {
    /// A layout that cannot be made measures as nothing: the window is laid out without the run
    /// rather than not at all (NFR-13).
    fn measure(&self, text: &str, style: Style, wrap: Wrap, width: f32) -> Size {
        self.lay(text, style, wrap, Align::Start, width).map_or(
            Size {
                width: 0.0,
                height: 0.0,
                lines: 0,
            },
            |laid| laid.size(),
        )
    }
}

fn alignment(align: Align) -> DWRITE_TEXT_ALIGNMENT {
    match align {
        Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
        Align::Centre => DWRITE_TEXT_ALIGNMENT_CENTER,
        Align::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
    }
}

/// `text` in capitals by the rules of `locale` — a Turkish «i» becomes «İ», as text-transform of
/// CSS does it in the mock-ups. A refusal leaves the text as it was.
fn upper(locale: &str, text: &str) -> String {
    let source: Vec<u16> = text.encode_utf16().collect();
    let locale = HSTRING::from(locale);
    let flags = LCMAP_UPPERCASE | LCMAP_LINGUISTIC_CASING;

    // SAFETY: the source and the locale live across both calls; the first asks for the length.
    let length = unsafe { LCMapStringEx(&locale, flags, &source, None, None, None, LPARAM(0)) };
    if length <= 0 {
        return text.to_owned();
    }
    let mut target = vec![0u16; length as usize];
    // SAFETY: as above, with a buffer of the length the first call answered.
    let written = unsafe {
        LCMapStringEx(
            &locale,
            flags,
            &source,
            Some(&mut target),
            None,
            None,
            LPARAM(0),
        )
    };
    if written <= 0 {
        return text.to_owned();
    }

    String::from_utf16_lossy(&target[..written as usize])
}

/// How far the ink of a layout goes beyond its box, in points; positive is outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overhang {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Where one character stands in a layout, in points from the layout's left edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub left: f32,
    pub width: f32,
    /// The bidi embedding level: even reads left to right, odd right to left.
    pub level: u32,
}

/// A run of text laid out by DirectWrite, ready for the canvas.
pub struct Laid {
    layout: IDWriteTextLayout,
    shown: String,
    family: String,
    style: Style,
    baseline: f32,
    direction: Direction,
    align: Align,
    wrap: Wrap,
    width: f32,
}

impl Laid {
    /// The size the text takes.
    pub fn size(&self) -> Size {
        let mut metrics = DWRITE_TEXT_METRICS::default();
        // SAFETY: the out parameter is a local.
        if unsafe { self.layout.GetMetrics(&mut metrics) }.is_err() {
            return Size {
                width: 0.0,
                height: 0.0,
                lines: 0,
            };
        }

        Size {
            width: metrics.width,
            height: metrics.height,
            lines: metrics.lineCount,
        }
    }

    /// How far the ink goes beyond the layout's box — for the test that no script is cut.
    pub fn overhang(&self) -> Overhang {
        // SAFETY: a query of a live layout.
        let metrics = unsafe { self.layout.GetOverhangMetrics() }.unwrap_or_default();

        Overhang {
            left: metrics.left,
            top: metrics.top,
            right: metrics.right,
            bottom: metrics.bottom,
        }
    }

    /// Where the character at `position` (in UTF-16 units of [`Laid::shown`]) stands — the
    /// instrument of the mixed lines.
    pub fn place_of(&self, position: u32) -> Option<Place> {
        let (mut x, mut y) = (0.0, 0.0);
        let mut metrics = DWRITE_HIT_TEST_METRICS::default();
        // SAFETY: the out parameters are locals.
        unsafe {
            self.layout
                .HitTestTextPosition(position, false, &mut x, &mut y, &mut metrics)
        }
        .ok()?;

        Some(Place {
            left: metrics.left,
            width: metrics.width,
            level: metrics.bidiLevel,
        })
    }

    /// Whether the character at `position` was cut away by the ellipsis.
    pub fn is_trimmed(&self, position: u32) -> bool {
        let (mut x, mut y) = (0.0, 0.0);
        let mut metrics = DWRITE_HIT_TEST_METRICS::default();
        // SAFETY: the out parameters are locals.
        unsafe {
            self.layout
                .HitTestTextPosition(position, false, &mut x, &mut y, &mut metrics)
        }
        .is_ok_and(|()| metrics.isTrimmed.as_bool())
    }

    /// The text as it is shown: in capitals where the role asks for them.
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// The family name the text is set in.
    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn align(&self) -> Align {
        self.align
    }

    pub fn wrap(&self) -> Wrap {
        self.wrap
    }

    /// The baseline of the first line, in points from the top of the layout.
    pub fn baseline(&self) -> f32 {
        self.baseline
    }

    /// The width the text was laid out in.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The layout itself, for the canvas.
    pub(crate) fn layout(&self) -> &IDWriteTextLayout {
        &self.layout
    }
}
