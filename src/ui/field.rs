//! The text field of the shell (T-120-18): an `EDIT` of `user32` inside the wrapper the shell draws
//! (`Part::Input`). Only the field is Windows' own — typing in every language, the IME, the
//! clipboard and undo stay the system's; the wrapper, its frame and the keyboard ring are the
//! shell's.
//!
//! * **No frame of its own,** placed inside the wrapper at the paddings of the table: a line field
//!   centred on the wrapper's height, a text field filling it, its scroll bar the system's (in the
//!   dark theme it stays light — the accepted price, T-120-18 п. 6).
//! * **Colours from the table** through `WM_CTLCOLOREDIT` (the window answers it with
//!   [`Fields::colours`]); the font is the role `body`'s face and size, made for the scale now; a field of code is set in the
//!   monospace face of the role `code` (T-120-19).
//! * **The hint** of an empty field is drawn in `ink3` by the field's own procedure after Windows
//!   paints it: the cue banner of Windows (`EM_SETCUEBANNER`) takes no colour (◆).
//! * **Right to left:** the reading order and the alignment of the language.
//! * **The keys:** Tab, Shift+Tab, Esc and, in a line field, Enter go on to the window, which moves
//!   the keyboard ring or presses; Ctrl+A is `widgets::keys` — the one body of the old windows;
//!   paste and undo are the field's own.
//!
//! FR-72: what the shell asks of its own fields goes through `SendMessageTimeoutW`.

use crate::ui::tokens::Rgb;
use ::windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use ::windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET,
    DT_EDITCONTROL, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_RTLREADING, DT_SINGLELINE, DT_WORDBREAK,
    DeleteObject, DrawTextW, GetDC, GetTextMetricsW, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect,
    OUT_DEFAULT_PRECIS, ReleaseDC, SelectObject, SetBkColor, SetBkMode, SetTextColor, TEXTMETRICW,
    TRANSPARENT,
};
use ::windows::Win32::UI::Controls::EM_SETMARGINS;
use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, SetFocus, VK_ESCAPE, VK_RETURN, VK_TAB,
};
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExW, DestroyWindow, EC_LEFTMARGIN, EC_RIGHTMARGIN, ES_AUTOHSCROLL,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_RIGHT, ES_WANTRETURN, GWLP_WNDPROC, GetClientRect, GetParent,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, MSG, SMTO_ABORTIFHUNG, SWP_NOACTIVATE,
    SWP_NOZORDER, SendMessageTimeoutW, SetWindowPos, SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_CHAR, WM_CLEAR, WM_CUT, WM_GETFONT, WM_KEYDOWN, WM_NCDESTROY, WM_PAINT, WM_PASTE,
    WM_SETFONT, WM_SETTEXT, WM_UNDO, WNDPROC, WS_CHILD, WS_EX_RTLREADING, WS_VISIBLE, WS_VSCROLL,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    ES_READONLY, PostMessageW, WM_APP, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETFOCUS,
    WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use ::windows::core::{HSTRING, w};
use std::cell::{Cell, RefCell};

/// A key of a field of a key in its capture, to the window it stands in (T-120-20), sent at once:
/// `wparam` — the virtual code, and 1 above its 16 bits when the key went up; `lparam` — the field.
pub const WM_FIELD_KEY: u32 = WM_APP + 2;

/// A field of a key in its capture lost the keyboard (T-120-20), posted — the window that took it
/// may be in the middle of its own message: `wparam` — the window that took the keyboard, `lparam`
/// — the field.
pub const WM_FIELD_LEFT: u32 = WM_APP + 3;

/// The number the shell's procedure is kept under in each field.
const SUBCLASS: usize = 1;

/// How long a send to a field waits — a window of this very thread, so the call is direct and the
/// number is only the bound FR-72 asks of every send (the same as `widgets::keys`).
const FIELD_TIMEOUT_MS: u32 = 50;

/// One field the layout placed: its wrapper's inside in client pixels and what it shows.
pub struct Spec<'a, I> {
    pub id: I,
    /// The wrapper less its paddings.
    pub inner: RECT,
    pub multiline: bool,
    /// A field of code (T-120-19): the monospace face of [`Look`].
    pub code: bool,
    /// A field of a key (T-120-20): none — a field to type in; whether it captures a key now.
    pub capture: Option<bool>,
    pub value: &'a str,
    pub hint: &'a str,
    pub disabled: bool,
    /// The text and the background, by the state of the wrapper.
    pub ink: Rgb,
    pub fill: Rgb,
}

/// What every field of a window shares: the font of `body` at the scale now, the colour of the
/// hint and the direction of the language.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub face: String,
    /// The type size in pixels.
    pub px: i32,
    pub weight: u16,
    /// The monospace face of the fields of code: the role `code` at the scale now.
    pub code_face: String,
    pub code_px: i32,
    pub code_weight: u16,
    pub hint: Rgb,
    pub rtl: bool,
}

/// What the field's own procedure reads: kept by [`Fields`] until the procedure is removed.
struct Data {
    hint: RefCell<Vec<u16>>,
    colour: Cell<COLORREF>,
    multiline: bool,
    rtl: bool,
    /// A field of a key: whether it captures now.
    capture: Cell<Option<bool>>,
}

struct Item<I> {
    id: I,
    hwnd: HWND,
    multiline: bool,
    key: bool,
    rtl: bool,
    ink: Rgb,
    fill: Rgb,
    font: HFONT,
    data: Box<Data>,
}

/// The fields of one window.
pub struct Fields<I> {
    items: Vec<Item<I>>,
    font: Option<(FontKey, HFONT)>,
    /// The monospace font, while the window has a field of code.
    code_font: Option<(FontKey, HFONT)>,
    /// The height of a line of the font, in pixels.
    line: i32,
    brushes: Vec<(Rgb, HBRUSH)>,
}

impl<I> Default for Fields<I> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            font: None,
            code_font: None,
            line: 0,
            brushes: Vec::new(),
        }
    }
}

fn colorref(colour: Rgb) -> COLORREF {
    COLORREF(
        u32::from(colour.r)
            | (u32::from(colour.g) << u8::BITS)
            | (u32::from(colour.b) << u16::BITS),
    )
}

/// Asks the field `hwnd`, through the timeout form of FR-72.
fn tell(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> usize {
    let mut answer = 0_usize;
    // SAFETY: a window of this thread; no pointer travels in the messages sent here, and `answer`
    // is a live local.
    let _ = unsafe {
        SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(wparam),
            LPARAM(lparam),
            SMTO_ABORTIFHUNG,
            FIELD_TIMEOUT_MS,
            Some(&mut answer),
        )
    };
    answer
}

/// The text of the field `hwnd`.
pub fn text_of(hwnd: HWND) -> String {
    // SAFETY: a window of this thread.
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0_u16; usize::try_from(length).unwrap_or(0) + 1];
    // SAFETY: the buffer is a live local of the length passed.
    let read = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..usize::try_from(read).unwrap_or(0)])
}

fn is_empty(hwnd: HWND) -> bool {
    // SAFETY: a window of this thread.
    unsafe { GetWindowTextLengthW(hwnd) == 0 }
}

impl<I: Copy + PartialEq> Fields<I> {
    /// The field of `id`, made, placed and filled as the layout says; the fields no longer
    /// described are destroyed.
    pub fn sync(&mut self, parent: HWND, specs: &[Spec<'_, I>], look: &Look) {
        let (font, old) = self.font_for(look);
        let (code_font, old_code) = if specs.iter().any(|spec| spec.code) {
            let key = (look.code_face.clone(), look.code_px, look.code_weight);
            let (code_font, old_code) = made(&mut self.code_font, key);
            (Some(code_font), old_code)
        } else {
            (None, self.code_font.take().map(|(_, font)| font))
        };
        let (keep, gone): (Vec<_>, Vec<_>) = self.items.drain(..).partition(|item| {
            item.rtl == look.rtl
                && specs.iter().any(|spec| {
                    spec.id == item.id
                        && spec.multiline == item.multiline
                        && spec.capture.is_some() == item.key
                })
        });
        self.items = keep;
        gone.into_iter().for_each(destroy);

        for spec in specs {
            let at = match self.items.iter().position(|item| item.id == spec.id) {
                Some(at) => at,
                None => match make(parent, spec, look) {
                    Some(item) => {
                        self.items.push(item);
                        self.items.len() - 1
                    }
                    None => continue,
                },
            };
            let line = self.line;
            let font = match code_font {
                Some(code_font) if spec.code => code_font,
                _ => font,
            };
            let item = &mut self.items[at];
            // Told before anything moves the keyboard: a capture that has ended reports no loss.
            item.data.capture.set(spec.capture);

            if item.font != font {
                item.font = font;
                tell(item.hwnd, WM_SETFONT, font.0 as usize, 0);
                tell(
                    item.hwnd,
                    EM_SETMARGINS,
                    (EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize,
                    0,
                );
            }

            let inner = spec.inner;
            let (w, h) = (inner.right - inner.left, inner.bottom - inner.top);
            let (y, h) = if spec.multiline || line <= 0 || line >= h {
                (inner.top, h)
            } else {
                (inner.top + (h - line) / 2, line)
            };
            // SAFETY: a child window of this thread; plain values.
            let _ = unsafe {
                SetWindowPos(
                    item.hwnd,
                    None,
                    inner.left,
                    y,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };

            if text_of(item.hwnd) != spec.value {
                // SAFETY: as above; the string lives across the call.
                let _ = unsafe { SetWindowTextW(item.hwnd, &HSTRING::from(spec.value)) };
            }

            let hint: Vec<u16> = spec.hint.encode_utf16().collect();
            let colour = colorref(look.hint);
            let changed = *item.data.hint.borrow() != hint
                || item.data.colour.get() != colour
                || item.ink != spec.ink
                || item.fill != spec.fill;
            if changed {
                *item.data.hint.borrow_mut() = hint;
                item.data.colour.set(colour);
                item.ink = spec.ink;
                item.fill = spec.fill;
                // SAFETY: as above.
                let _ = unsafe { InvalidateRect(Some(item.hwnd), None, true) };
            }
            // SAFETY: as above.
            let _ = unsafe { EnableWindow(item.hwnd, !spec.disabled) };
        }
        for old in [old, old_code].into_iter().flatten() {
            // SAFETY: a font this object made, which no field holds any more.
            let _ = unsafe { DeleteObject(HGDIOBJ(old.0)) };
        }
    }

    /// The font of the body of `look`, made when it changed, and the font it replaces — deleted by
    /// the caller once no field holds it.
    fn font_for(&mut self, look: &Look) -> (HFONT, Option<HFONT>) {
        let before = self.font.as_ref().map(|(_, font)| *font);
        let (font, old) = made(&mut self.font, (look.face.clone(), look.px, look.weight));
        if before != Some(font) {
            self.line = line_of(font);
        }
        (font, old)
    }

    /// The identifier of the field `hwnd`.
    pub fn id_of(&self, hwnd: HWND) -> Option<I> {
        self.items
            .iter()
            .find(|item| item.hwnd == hwnd)
            .map(|item| item.id)
    }

    /// The window of the field `id`.
    pub fn hwnd_of(&self, id: I) -> Option<HWND> {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.hwnd)
    }

    /// The fields, as the tests read them.
    /// How many fonts the fields hold now: the font of `body`, and the monospace one while a field
    /// of code is there (the testing build; the probe of the host).
    #[cfg(feature = "testing")]
    pub fn fonts(&self) -> usize {
        usize::from(self.font.is_some()) + usize::from(self.code_font.is_some())
    }

    pub fn all(&self) -> Vec<(I, HWND)> {
        self.items.iter().map(|item| (item.id, item.hwnd)).collect()
    }

    /// Gives the keyboard to the field `id`; false when there is none.
    pub fn focus(&self, id: I) -> bool {
        let Some(hwnd) = self.hwnd_of(id) else {
            return false;
        };
        // SAFETY: a child window of this thread.
        if unsafe { GetFocus() } != hwnd {
            let _ = unsafe { SetFocus(Some(hwnd)) };
        }
        true
    }

    /// Whether a field holds the keyboard now.
    pub fn has_keyboard(&self) -> bool {
        // SAFETY: reads the focus of this thread.
        let focus = unsafe { GetFocus() };
        self.items.iter().any(|item| item.hwnd == focus)
    }

    /// The answer to `WM_CTLCOLOREDIT` and `WM_CTLCOLORSTATIC` for the field `hwnd`: its colours
    /// set on `dc`, its background's brush.
    pub fn colours(&mut self, dc: HDC, hwnd: HWND) -> Option<LRESULT> {
        let (ink, fill) = self
            .items
            .iter()
            .find(|item| item.hwnd == hwnd)
            .map(|item| (item.ink, item.fill))?;
        let brush = match self.brushes.iter().find(|(colour, _)| *colour == fill) {
            Some((_, brush)) => *brush,
            None => {
                // SAFETY: a plain value; the brush is deleted by `release`.
                let brush = unsafe { CreateSolidBrush(colorref(fill)) };
                self.brushes.push((fill, brush));
                brush
            }
        };
        // SAFETY: the DC Windows passed with the message.
        unsafe {
            SetTextColor(dc, colorref(ink));
            SetBkColor(dc, colorref(fill));
        }
        Some(LRESULT(brush.0 as isize))
    }

    /// Takes the fields out of the window and frees what they held: on `WM_DESTROY` of the window,
    /// before its children go.
    pub fn release(&mut self) {
        self.items.drain(..).for_each(destroy);
        for (_, font) in [self.font.take(), self.code_font.take()]
            .into_iter()
            .flatten()
        {
            // SAFETY: a font this object made, which no field holds any more.
            let _ = unsafe { DeleteObject(HGDIOBJ(font.0)) };
        }
        for (_, brush) in self.brushes.drain(..) {
            // SAFETY: a brush this object made.
            let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
        }
    }
}

impl<I> Drop for Fields<I> {
    fn drop(&mut self) {
        self.items.drain(..).for_each(destroy);
        for (_, font) in [self.font.take(), self.code_font.take()]
            .into_iter()
            .flatten()
        {
            // SAFETY: as in `release`.
            let _ = unsafe { DeleteObject(HGDIOBJ(font.0)) };
        }
        for (_, brush) in self.brushes.drain(..) {
            // SAFETY: as in `release`.
            let _ = unsafe { DeleteObject(HGDIOBJ(brush.0)) };
        }
    }
}

/// The face, the size in pixels and the weight of a font of the fields.
type FontKey = (String, i32, u16);

/// The font of `key` in `slot`, made when it changed, and the font it replaces — deleted by the
/// caller once no field holds it.
fn made(slot: &mut Option<(FontKey, HFONT)>, key: FontKey) -> (HFONT, Option<HFONT>) {
    if let Some((made, font)) = slot
        && *made == key
    {
        return (*font, None);
    }
    let face = HSTRING::from(key.0.as_str());
    // SAFETY: plain values and a string that lives across the call.
    let font = unsafe {
        CreateFontW(
            -key.1,
            0,
            0,
            0,
            i32::from(key.2),
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            &face,
        )
    };
    let old = slot.replace((key, font)).map(|(_, old)| old);
    (font, old)
}

/// The height of a line of `font`, in pixels.
fn line_of(font: HFONT) -> i32 {
    let mut metrics = TEXTMETRICW::default();
    // SAFETY: the screen's DC, released below; the font is selected and put back; the out
    // parameter is a local.
    unsafe {
        let dc = GetDC(None);
        let old = SelectObject(dc, HGDIOBJ(font.0));
        let _ = GetTextMetricsW(dc, &mut metrics);
        SelectObject(dc, old);
        ReleaseDC(None, dc);
    }
    metrics.tmHeight
}

fn make<I: Copy>(parent: HWND, spec: &Spec<'_, I>, look: &Look) -> Option<Item<I>> {
    let mut style = WS_CHILD.0 | WS_VISIBLE.0;
    style |= if spec.multiline {
        WS_VSCROLL.0 | (ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32
    } else {
        ES_AUTOHSCROLL as u32
    };
    // A field of a key shows the key; it is captured, never typed (104.3).
    if spec.capture.is_some() {
        style |= ES_READONLY as u32;
    }
    let mut extended = WINDOW_EX_STYLE::default();
    if look.rtl {
        style |= ES_RIGHT as u32;
        extended |= WS_EX_RTLREADING;
    }
    let inner = spec.inner;
    // SAFETY: the system class of a field, a child of a window of this thread; the strings live
    // across the call.
    let hwnd = unsafe {
        CreateWindowExW(
            extended,
            w!("EDIT"),
            &HSTRING::from(spec.value),
            WINDOW_STYLE(style),
            inner.left,
            inner.top,
            inner.right - inner.left,
            inner.bottom - inner.top,
            Some(parent),
            None,
            None,
            None,
        )
    }
    .ok()?;

    let data = Box::new(Data {
        hint: RefCell::new(spec.hint.encode_utf16().collect()),
        colour: Cell::new(colorref(look.hint)),
        multiline: spec.multiline,
        rtl: look.rtl,
        capture: Cell::new(spec.capture),
    });
    // SAFETY: the procedure below and the box kept in the item until `destroy` removes the
    // procedure first.
    let _ =
        unsafe { SetWindowSubclass(hwnd, Some(procedure), SUBCLASS, (&raw const *data) as usize) };
    Some(Item {
        id: spec.id,
        hwnd,
        multiline: spec.multiline,
        key: spec.capture.is_some(),
        rtl: look.rtl,
        ink: spec.ink,
        fill: spec.fill,
        font: HFONT::default(),
        data,
    })
}

/// The field goes: its procedure first, then the window, then what the procedure read.
fn destroy<I>(item: Item<I>) {
    // SAFETY: a child window of this thread, or one already gone — then both calls refuse.
    unsafe {
        let _ = RemoveWindowSubclass(item.hwnd, Some(procedure), SUBCLASS);
        let _ = DestroyWindow(item.hwnd);
    }
    drop(item.data);
}

/// Hands a key to the window the field stands in, as if it had pressed it there.
fn forward(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    // SAFETY: a window of this thread.
    let Ok(parent) = (unsafe { GetParent(hwnd) }) else {
        return;
    };
    // SAFETY: the procedure Windows keeps for the parent, a window of this thread.
    let raw = unsafe { GetWindowLongPtrW(parent, GWLP_WNDPROC) };
    // SAFETY: the value is that procedure's address, as Windows stored it.
    let procedure = unsafe { std::mem::transmute::<isize, WNDPROC>(raw) };
    // SAFETY: the parent's own procedure, called on its thread with a key message.
    unsafe { CallWindowProcW(procedure, parent, message, wparam, lparam) };
}

/// What a field of a key answers itself (T-120-20, as the field of the old window: FR-94, 82.6,
/// 104.3). Out of its capture it only shows the key: the mouse does not reach it and the keyboard
/// does not settle in it — no caret, no selection. In its capture every key goes to the window as
/// [`WM_FIELD_KEY`] — Tab, Esc, Enter, F10 and the keys with Alt too — none is typed, and the loss
/// of the keyboard is posted as [`WM_FIELD_LEFT`]. **SEC-01:** the virtual code and nothing else.
fn key_field(hwnd: HWND, message: u32, wparam: WPARAM, capturing: bool) -> Option<LRESULT> {
    match message {
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP
        | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN => Some(LRESULT(0)),
        // The field's own handler makes the caret and selects the text: neither belongs here.
        WM_SETFOCUS => Some(LRESULT(0)),
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP if capturing => {
            let up = usize::from(matches!(message, WM_KEYUP | WM_SYSKEYUP)) << u16::BITS;
            let key = (wparam.0 & usize::from(u16::MAX)) | up;
            forward(hwnd, WM_FIELD_KEY, WPARAM(key), LPARAM(hwnd.0 as isize));
            Some(LRESULT(0))
        }
        WM_CHAR | WM_SYSCHAR if capturing => Some(LRESULT(0)),
        WM_KILLFOCUS if capturing => {
            // SAFETY: a window of this thread.
            if let Ok(parent) = unsafe { GetParent(hwnd) } {
                // SAFETY: a message to the window the field stands in, of this thread.
                let _ = unsafe {
                    PostMessageW(Some(parent), WM_FIELD_LEFT, wparam, LPARAM(hwnd.0 as isize))
                };
            }
            Some(LRESULT(0))
        }
        _ => None,
    }
}

/// The hint of an empty field, in its colour, where Windows would put the text.
fn paint_hint(hwnd: HWND, data: &Data) {
    let mut hint = data.hint.borrow().clone();
    if hint.is_empty() {
        return;
    }
    let mut rect = RECT::default();
    // SAFETY: the out parameter is a local.
    let _ = unsafe { GetClientRect(hwnd, &mut rect) };
    let font = tell(hwnd, WM_GETFONT, 0, 0);
    let mut format = DT_NOPREFIX | DT_EDITCONTROL;
    format |= if data.multiline {
        DT_WORDBREAK
    } else {
        DT_SINGLELINE
    };
    format |= if data.rtl {
        DT_RIGHT | DT_RTLREADING
    } else {
        DT_LEFT
    };
    // SAFETY: the field's DC, released below; its own font selected and put back; the text and
    // the rectangle are locals.
    unsafe {
        let dc = GetDC(Some(hwnd));
        let old = SelectObject(dc, HGDIOBJ(font as *mut _));
        SetTextColor(dc, data.colour.get());
        SetBkMode(dc, TRANSPARENT);
        DrawTextW(dc, &mut hint, &mut rect, format);
        SelectObject(dc, old);
        ReleaseDC(Some(hwnd), dc);
    }
}

/// The shell's procedure in every field: the keys of the window go to the window, the hint is
/// drawn, an emptied or filled field is drawn again.
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass: usize,
    data: usize,
) -> LRESULT {
    // SAFETY: the box `make` passed, kept until `destroy` removes this procedure.
    let data = unsafe { &*(data as *const Data) };

    if let Some(capturing) = data.capture.get()
        && let Some(answer) = key_field(hwnd, message, wparam, capturing)
    {
        return answer;
    }

    match message {
        WM_KEYDOWN => {
            let key = wparam.0 as u16;
            if key == VK_TAB.0 || key == VK_ESCAPE.0 || (key == VK_RETURN.0 && !data.multiline) {
                forward(hwnd, message, wparam, lparam);
                return LRESULT(0);
            }
            let pressed = MSG {
                hwnd,
                message,
                wParam: wparam,
                lParam: lparam,
                ..Default::default()
            };
            if crate::widgets::keys::select_all_on_ctrl_a(&pressed) {
                return LRESULT(0);
            }
        }
        WM_CHAR => {
            // The characters of the keys the window took, and Ctrl+A's: none is typed.
            let eaten = [Some('\t'), Some('\u{1b}'), Some('\u{1}')]
                .contains(&char::from_u32(wparam.0 as u32))
                || (char::from_u32(wparam.0 as u32) == Some('\r') && !data.multiline);
            if eaten {
                return LRESULT(0);
            }
        }
        WM_PAINT => {
            // SAFETY: the field's own painting first.
            let answer = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
            if is_empty(hwnd) {
                paint_hint(hwnd, data);
            }
            return answer;
        }
        WM_NCDESTROY => {
            // SAFETY: the procedure leaves a window that goes.
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS) };
        }
        _ => {}
    }

    let watched = matches!(
        message,
        WM_CHAR | WM_KEYDOWN | WM_SETTEXT | WM_CUT | WM_PASTE | WM_CLEAR | WM_UNDO
    );
    let before = watched && is_empty(hwnd);
    // SAFETY: the field's own answer.
    let answer = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if watched && before != is_empty(hwnd) {
        // SAFETY: a window of this thread; drawn again with or without the hint.
        let _ = unsafe { InvalidateRect(Some(hwnd), None, true) };
    }
    answer
}
