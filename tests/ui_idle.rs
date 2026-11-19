//! **Nothing at rest** (stage Э120, README §1): a process that has not opened a window of the
//! interface shell has not loaded Direct2D. This file is a test binary of its own, with one test,
//! so that nothing else of the test run can have drawn before it looks.
//!
//! What it catches: an import of `d2d1.dll` in the program's import table — a direct call of
//! `D2D1CreateFactory` through the `windows` crate puts one there, and the loader then maps the
//! module at start — and any canvas made before the first window.

use lang_switcher::ui::canvas::{Canvas, Mode};
use lang_switcher::ui::tokens::{Color, Theme};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::w;

fn d2d1_is_loaded() -> bool {
    // SAFETY: a NUL-terminated module name; the handle is only tested, not used.
    unsafe { GetModuleHandleW(w!("d2d1.dll")) }.is_ok()
}

#[test]
fn direct2d_is_loaded_by_the_first_canvas_and_not_before() {
    assert!(
        !d2d1_is_loaded(),
        "d2d1.dll is in the process before any canvas was made: it is imported, not loaded on demand"
    );

    let mut canvas = Canvas::in_memory(10.0, 10.0, 1.0, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| painter.clear(Color::Bg.get(Theme::Light)));

    assert!(d2d1_is_loaded(), "the first canvas loads d2d1.dll");
}
