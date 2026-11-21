//! **The icons of the shell are drawn on a thread that entered no COM** — the stand of 2026-11-20
//! (Windows 11): the window in the corner came without the program's icon and its picture of the
//! taskbar without the flags; the thread of the windows enters no apartment, and WIC refused there.
//! A test binary of its own: no other thread of the process makes an apartment that would hide it.

#![cfg(feature = "testing")]

use lang_switcher::layouts::LayoutId;
use lang_switcher::settings::Language;
use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::canvas;
use lang_switcher::ui::host::{self, Content, Reply};
use lang_switcher::ui::scene::{Event, Taskbar, Window};
use lang_switcher::ui::windows::corner;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, IsWindow, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WM_CLOSE, WM_PAINT,
};

/// «Обновлено» with its picture of the taskbar: the flags of two layouts.
struct Updated;

impl Content for Updated {
    type Id = u16;

    fn describe(&self) -> Window<u16> {
        let content = lang_switcher::corner::release_content_in(Language::Ru, "1.6.1");
        let picture = Taskbar {
            layouts: [
                LayoutId::from_raw(0x0409_0409),
                LayoutId::from_raw(0x0419_0419),
            ],
            codes: ["ENG".to_owned(), "RUS".to_owned()],
            shown: 0,
            plate: true,
            cursor: Some(0.5),
            list: true,
            hover: true,
            icons_left: false,
        };
        corner::describe(&content, false, Some(picture)).map(&|id| id as u16)
    }

    fn event(&mut self, _: Event<u16>) -> Reply {
        Reply::Stay
    }

    fn title(&self) -> String {
        "Updated".to_owned()
    }

    fn language(&self) -> (String, bool) {
        ("ru-RU".to_owned(), false)
    }

    fn theme(&self) -> ThemeSetting {
        ThemeSetting::Dark
    }
}

fn send(hwnd: HWND, message: u32) {
    // SAFETY: a window of this thread; the message goes straight to its procedure.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(0)), Some(LPARAM(0))) };
}

#[test]
fn the_flags_of_the_picture_are_drawn_on_a_thread_without_com() {
    let hwnd = host::create(Updated, None).expect("the window of the shell");
    send(hwnd, WM_PAINT);
    let (made, refused) = canvas::icon_bitmaps();
    send(hwnd, WM_CLOSE);

    assert_eq!(refused, 0, "WIC refused the icons on a thread without COM");
    assert!(made >= 2, "the flags of both layouts: {made}");
}

/// Д4 (stand of 21.11, a crash at «Выход», 3 of 3): the bitmaps kept for a render target — the flags
/// here, the program's icon of the menu's head on the stand (a test binary has no icon resource) —
/// are let go with the target. A bitmap kept after its window holds the device of Direct3D to the
/// end of the thread, where it is let go under the loader lock, and `ExitProcess` fell in DXGI.
#[test]
fn the_bitmaps_of_a_window_go_with_it() {
    let hwnd = host::create(Updated, None).expect("the window of the shell");
    send(hwnd, WM_PAINT);
    let kept = canvas::kept_bitmaps();
    send(hwnd, WM_CLOSE);
    let mut message = MSG::default();
    // SAFETY: the message queue of this thread; each message goes to its window.
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: a message just taken from the queue.
        unsafe { DispatchMessageW(&message) };
    }
    // SAFETY: a handle by value.
    let gone = !unsafe { IsWindow(Some(hwnd)) }.as_bool();

    assert!(
        kept >= 2,
        "the picture kept the flags of both layouts: {kept}"
    );
    assert!(gone, "the window did not close");
    assert_eq!(
        canvas::kept_bitmaps(),
        0,
        "bitmaps of the window's render target outlived the window"
    );
}
