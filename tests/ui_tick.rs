//! The tick of a window of the shell (stage Э120, T-120-25): the period by the rule — shown, not
//! minimised, Windows' animation on — the tick reaching the content as an event and a new frame,
//! and no timer left once the window is gone («в покое — ноль»). Nothing is on the screen: the
//! window is made and never shown; the testing build counts a hidden window as shown
//! (`LANGSW_TESTING_UI_TICK_HIDDEN`), so that its timer can be counted.

use lang_switcher::ui::host;

/// The rule of the period: what the content asks, while the window is shown, not minimised and
/// Windows moves what it shows.
#[test]
fn the_window_ticks_only_when_shown_and_moving() {
    assert_eq!(host::tick_period(true, false, true, Some(130)), Some(130));
    assert_eq!(
        host::tick_period(false, false, true, Some(130)),
        None,
        "hidden"
    );
    assert_eq!(
        host::tick_period(true, true, true, Some(130)),
        None,
        "minimised"
    );
    assert_eq!(
        host::tick_period(true, false, false, Some(130)),
        None,
        "animation off"
    );
    assert_eq!(
        host::tick_period(true, false, true, None),
        None,
        "not asked"
    );
}

#[cfg(feature = "testing")]
mod live {
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::{self, Content, Reply};
    use lang_switcher::ui::scene::{Event, Node, Window, WindowKind};
    use lang_switcher::ui::tokens::Role;
    use std::cell::Cell;
    use std::rc::Rc;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, IsWindow, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WM_CLOSE, WM_PAINT,
        WM_TIMER,
    };

    /// The id of the tick timer of the host.
    const TICK_TIMER: usize = 3;

    struct Clock {
        ticks: Rc<Cell<u32>>,
    }

    impl Content for Clock {
        type Id = u8;

        fn describe(&self) -> Window<u8> {
            Window {
                kind: WindowKind::Ordinary,
                art: None,
                brand: None,
                body: Node::Text(Role::Body, format!("frame {}", self.ticks.get())),
                buttons: Vec::new(),
                disabled: Vec::new(),
            }
        }

        fn event(&mut self, event: Event<u8>) -> Reply {
            if event == Event::Tick {
                self.ticks.set(self.ticks.get() + 1);
            }
            Reply::Stay
        }

        fn title(&self) -> String {
            "Clock".to_owned()
        }

        fn language(&self) -> (String, bool) {
            ("en-US".to_owned(), false)
        }

        fn theme(&self) -> ThemeSetting {
            ThemeSetting::Light
        }

        fn tick(&self) -> Option<u32> {
            Some(130)
        }
    }

    fn send(hwnd: HWND, message: u32, wparam: usize) {
        // SAFETY: a window of this thread; the message goes straight to its procedure.
        unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(0))) };
    }

    fn pump() {
        let mut message = MSG::default();
        // SAFETY: the message queue of this thread; each message goes to its window.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: a message just taken from the queue.
            unsafe { DispatchMessageW(&message) };
        }
    }

    /// The tick reaches the content and makes a frame; the window's timer runs while it is open and
    /// is gone with it — no tick timer left in the process.
    #[test]
    fn the_tick_reaches_the_content_and_dies_with_the_window() {
        // SAFETY: the variable of this test process, set before any window of it.
        unsafe { std::env::set_var("LANGSW_TESTING_UI_TICK_HIDDEN", "1") };
        let ticks = Rc::new(Cell::new(0));
        let hwnd = host::create(
            Clock {
                ticks: ticks.clone(),
            },
            None,
        )
        .expect("the window");
        send(hwnd, WM_PAINT, 0);
        assert_eq!(host::live_tick_timers(), 1, "the window does not tick");

        let frames = host::probe(hwnd).expect("a live host").frames;
        for _ in 0..3 {
            send(hwnd, WM_TIMER, TICK_TIMER);
            send(hwnd, WM_PAINT, 0);
        }
        assert_eq!(ticks.get(), 3, "three ticks, three events");
        assert!(
            host::probe(hwnd).expect("a live host").frames >= frames + 3,
            "a tick without a frame"
        );

        send(hwnd, WM_CLOSE, 0);
        pump();
        // SAFETY: a handle by value.
        assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
        assert_eq!(
            host::live_tick_timers(),
            0,
            "a tick timer outlived its window"
        );
    }
}
