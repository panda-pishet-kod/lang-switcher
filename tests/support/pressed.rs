//! A key pressed the way the program takes it (blocker of 1.8.0, stand of 21.11): posted, and run
//! through the one loop of the program, `app::pump_while` — with its `TranslateMessage`, so the
//! character of the key comes as in the program.

use std::cell::Cell;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    KillTimer, PostMessageW, SetTimer, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_TIMER,
};

/// Presses `key` (scan code `scan`) in `window`: the key down and up posted to it and run through
/// the loop of the program until a mark posted once the key's up has gone — after the characters
/// the loop made of the key down. A timer of 2 s ends the loop if the mark never comes.
pub fn press_through_the_loop_of_the_program(window: HWND, key: u16, scan: u32) {
    const MARK: u32 = WM_APP + 0x3A0;
    let lparam = |up: bool| {
        let flags = if up { 0xC000_0000_u32 } else { 0 };
        LPARAM((1 | (scan << 16) | flags) as i32 as isize)
    };
    let done = Cell::new(false);
    // SAFETY: posted to a window of this thread; a timer of this thread, without a window.
    let deadline = unsafe {
        let key = WPARAM(usize::from(key));
        PostMessageW(Some(window), WM_KEYDOWN, key, lparam(false)).expect("the key down");
        PostMessageW(Some(window), WM_KEYUP, key, lparam(true)).expect("the key up");
        SetTimer(None, 0, 2000, None)
    };
    lang_switcher::app::pump_while(
        |message| {
            if message.message == WM_KEYUP && message.hwnd == window {
                // SAFETY: a message to this thread, without a window.
                unsafe { PostMessageW(None, MARK, WPARAM(0), LPARAM(0)) }.expect("the mark");
            }
            let mine = message.hwnd.is_invalid()
                && (message.message == MARK
                    || (message.message == WM_TIMER && message.wParam.0 == deadline));
            if mine {
                done.set(true);
            }
            mine
        },
        || !done.get(),
    )
    .expect("the loop of the program");
    // SAFETY: the timer of this thread made above.
    let _ = unsafe { KillTimer(None, deadline) };
}
