//! **Nothing at rest** (stage Э120, README §1): a process that has not opened a window of the
//! interface shell has loaded neither Direct2D nor DirectWrite. This file is a test binary of its
//! own, with one test, so that nothing else of the test run can have drawn before it looks.
//!
//! What it catches: an import of `d2d1.dll` or `dwrite.dll` in the program's import table — a
//! direct call of `D2D1CreateFactory` or `DWriteCreateFactory` through the `windows` crate puts one
//! there, and the loader then maps the module at start — and any canvas or text made before the
//! first window.

use lang_switcher::ui::canvas::{Canvas, Mode};
use lang_switcher::ui::text::{Direction, Text};
use lang_switcher::ui::tokens::{Color, Theme};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::{PCWSTR, w};

fn is_loaded(module: PCWSTR) -> bool {
    // SAFETY: a NUL-terminated module name; the handle is only tested, not used.
    unsafe { GetModuleHandleW(module) }.is_ok()
}

#[test]
fn direct2d_and_directwrite_are_loaded_by_their_first_use_and_not_before() {
    for (module, name) in [
        (w!("d2d1.dll"), "d2d1.dll"),
        (w!("dwrite.dll"), "dwrite.dll"),
    ] {
        assert!(
            !is_loaded(module),
            "{name} is in the process before any canvas or text was made: it is imported, not \
             loaded on demand"
        );
    }

    let mut canvas = Canvas::in_memory(10.0, 10.0, 1.0, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| painter.clear(Color::Bg.get(Theme::Light)));
    assert!(is_loaded(w!("d2d1.dll")), "the first canvas loads d2d1.dll");

    let text = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    assert!(
        is_loaded(w!("dwrite.dll")),
        "the first text loads dwrite.dll"
    );
    drop(text);
}
