//! **The memory a window of the shell leaves behind** — the instrument of the controller's question
//! (2026-11-20, the stand of T-120-13: «О программе» in the tray program held +8,8…11,4 MB after
//! closing, growing by 0,1 MB a time). It opens and closes the window again and again and writes the
//! private memory of the process after each: a line that keeps rising is something held, a line that
//! levels off is a cache. Run on purpose: `--ignored --nocapture`; `LANGSW_UI_MEMORY` — a file for the
//! series.

#![cfg(feature = "testing")]

use lang_switcher::settings::Language;
use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::host;
use lang_switcher::ui::windows::about::About;
use windows::Win32::Foundation::{HANDLE, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, SendMessageW, WM_CLOSE, WM_PAINT,
};

/// `PROCESS_MEMORY_COUNTERS_EX` of `psapi.h`, declared by hand (as `tests\e2e\scenarios.rs` does).
#[repr(C)]
#[derive(Default)]
struct Counters {
    cb: u32,
    page_faults: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged: usize,
    quota_paged: usize,
    quota_peak_non_paged: usize,
    quota_non_paged: usize,
    pagefile: usize,
    peak_pagefile: usize,
    private: usize,
}

#[link(name = "user32")]
// SAFETY: the documented export of user32, same signature.
unsafe extern "system" {
    fn GetGuiResources(process: HANDLE, flags: u32) -> u32;
}

/// The GDI and USER objects of this process.
fn objects() -> (u32, u32) {
    // SAFETY: the pseudo handle of this process; 0 — GDI objects, 1 — USER objects.
    unsafe {
        (
            GetGuiResources(GetCurrentProcess(), 0),
            GetGuiResources(GetCurrentProcess(), 1),
        )
    }
}

#[link(name = "kernel32")]
// SAFETY: the declaration matches the documented export of kernel32 behind psapi's
// `GetProcessMemoryInfo`, same signature.
unsafe extern "system" {
    fn K32GetProcessMemoryInfo(process: HANDLE, counters: *mut Counters, cb: u32) -> i32;
}

/// The private bytes and the working set of this process, MB.
fn memory() -> (f64, f64) {
    let mut c = Counters {
        cb: size_of::<Counters>() as u32,
        ..Default::default()
    };
    // SAFETY: the pseudo handle of this process; the structure is a local of the size it says.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    assert_ne!(ok, 0, "GetProcessMemoryInfo");
    let mb = |bytes: usize| bytes as f64 / 1_048_576.0;
    (mb(c.private), mb(c.working_set))
}

fn pump() {
    let mut message = MSG::default();
    // SAFETY: the message queue of this thread; each message goes to its window.
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: a message just taken from the queue.
        unsafe { DispatchMessageW(&message) };
    }
}

/// **30 openings and closings of «О программе»** — the private memory after each, printed and written
/// to `LANGSW_UI_MEMORY`.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_memory_after_thirty_openings_of_the_about_window() {
    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let (before, _) = memory();
    let mut lines = vec![format!("before: private {before:.2} MB")];
    for round in 1..=rounds {
        let hwnd = host::create(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell");
        // SAFETY: a window of this thread; the messages go straight to its procedure.
        unsafe {
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        let (private, working) = memory();
        let (gdi, user) = objects();
        lines.push(format!(
            "{round:2}: private {private:.2} MB (+{:.2}), working set {working:.2} MB, gdi {gdi}, user {user}",
            private - before
        ));
    }
    let text = lines.join("\r\n");
    println!("{text}");
    if let Some(path) = std::env::var_os("LANGSW_UI_MEMORY") {
        std::fs::write(path, text + "\r\n").expect("the series written");
    }
}

/// The same series for the plainest window of the shell — one line of text, no picture, no link: what
/// grows in both is the shell's, what grows only in «О программе» is that window's.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_memory_after_openings_of_a_plain_window() {
    use lang_switcher::ui::host::{Content, Reply};
    use lang_switcher::ui::scene::{Event, Node, Window, WindowKind};
    use lang_switcher::ui::tokens::Role;

    struct Plain;

    impl Content for Plain {
        type Id = u8;

        fn describe(&self) -> Window<u8> {
            Window {
                kind: WindowKind::Ordinary,
                art: None,
                brand: None,
                body: Node::Text(Role::Body, "A plain window".to_owned()),
                buttons: Vec::new(),
                disabled: Vec::new(),
            }
        }

        fn event(&mut self, _: Event<u8>) -> Reply {
            Reply::Stay
        }

        fn title(&self) -> String {
            "Plain".to_owned()
        }

        fn language(&self) -> (String, bool) {
            ("en-US".to_owned(), false)
        }

        fn theme(&self) -> ThemeSetting {
            ThemeSetting::Light
        }
    }

    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let (before, _) = memory();
    let mut lines = vec![format!("before: private {before:.2} MB")];
    for round in 1..=rounds {
        let hwnd = host::create(Plain, None).expect("the window of the shell");
        // SAFETY: a window of this thread; the messages go straight to its procedure.
        unsafe {
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        let (private, _) = memory();
        lines.push(format!(
            "{round:2}: private {private:.2} MB (+{:.2})",
            private - before
        ));
    }
    println!("{}", lines.join("\r\n"));
}

/// «О программе» with parts taken away (`LANGSW_UI_MEMORY_STRIP`: `art`, `brand`, `body`, `buttons`,
/// comma-separated) — to find which part holds what grows.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_memory_after_openings_of_the_about_window_with_parts_taken_away() {
    use lang_switcher::ui::host::{Content, Reply};
    use lang_switcher::ui::scene::{Event, Node, Window};
    use lang_switcher::ui::tokens::Role;

    struct Stripped(About, String);

    impl Content for Stripped {
        type Id = <About as Content>::Id;

        fn describe(&self) -> Window<Self::Id> {
            let mut window = self.0.describe();
            if self.1.contains("art") {
                window.art = None;
            }
            if self.1.contains("brand") {
                window.brand = None;
            }
            if self.1.contains("body") {
                window.body = Node::Text(Role::Body, "body taken away".to_owned());
            }
            if self.1.contains("buttons") {
                window.buttons.clear();
            }
            if let Some(brand) = window.brand.as_mut() {
                if self.1.contains("address") {
                    brand.address.clear();
                }
                if self.1.contains("name") {
                    brand.name.clear();
                }
            }
            window
        }

        fn event(&mut self, event: Event<Self::Id>) -> Reply {
            self.0.event(event)
        }

        fn title(&self) -> String {
            self.0.title()
        }

        fn language(&self) -> (String, bool) {
            self.0.language()
        }

        fn theme(&self) -> ThemeSetting {
            self.0.theme()
        }
    }

    let strip = std::env::var("LANGSW_UI_MEMORY_STRIP").unwrap_or_default();
    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    // `LANGSW_UI_MEMORY_KEEP`: a plain window stays open the whole series, so the factory of
    // Direct2D shared by the windows of the thread is never let go between them.
    let keep = std::env::var_os("LANGSW_UI_MEMORY_KEEP").map(|_| {
        let about = About {
            language: Language::Ru,
            key: "Pause".to_owned(),
            theme: ThemeSetting::Light,
        };
        let hwnd = host::create(Stripped(about, "art,brand,body,buttons".to_owned()), None)
            .expect("the window kept open");
        // SAFETY: a window of this thread.
        unsafe { SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0))) };
        hwnd
    });
    let (before, _) = memory();
    let mut lines = vec![format!("strip [{strip}] before: private {before:.2} MB")];
    for round in 1..=rounds {
        let about = About {
            language: Language::Ru,
            key: "Pause".to_owned(),
            theme: ThemeSetting::Light,
        };
        let hwnd = host::create(Stripped(about, strip.clone()), None).expect("the window");
        // SAFETY: a window of this thread; the messages go straight to its procedure.
        unsafe {
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        let (private, _) = memory();
        lines.push(format!(
            "{round:2}: private {private:.2} MB (+{:.2})",
            private - before
        ));
    }
    println!("{}", lines.join("\r\n"));
    if let Some(hwnd) = keep {
        // SAFETY: a window of this thread.
        unsafe { SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0))) };
        pump();
    }
}

/// The words Direct2D's debug layer writes to the debugger, caught inside the process: a debug string
/// raises an exception a vectored handler sees first, with or without a debugger.
mod debug_words {
    use std::ffi::c_void;
    use std::sync::Mutex;

    #[repr(C)]
    struct Record {
        code: u32,
        flags: u32,
        record: *mut Record,
        address: *mut c_void,
        parameters: u32,
        information: [usize; 15],
    }

    #[repr(C)]
    struct Pointers {
        record: *mut Record,
        context: *mut c_void,
    }

    #[link(name = "kernel32")]
    // SAFETY: the documented exports of kernel32, same signatures.
    unsafe extern "system" {
        fn AddVectoredExceptionHandler(
            first: u32,
            handler: unsafe extern "system" fn(*mut Pointers) -> i32,
        ) -> *mut c_void;
        fn RemoveVectoredExceptionHandler(handle: *mut c_void) -> u32;
        fn OutputDebugStringW(text: *const u16);
    }

    const PRINT_ANSI: u32 = 0x4001_0006;
    const PRINT_WIDE: u32 = 0x4001_000A;

    static WORDS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    unsafe extern "system" fn catch(pointers: *mut Pointers) -> i32 {
        // SAFETY: the system hands a live record for the exception being dispatched; the string of
        // a debug print lives across the dispatch.
        unsafe {
            let record = &*(*pointers).record;
            let (length, address) = (record.information[0], record.information[1]);
            let words = match record.code {
                PRINT_WIDE => Some(String::from_utf16_lossy(std::slice::from_raw_parts(
                    address as *const u16,
                    length.saturating_sub(1),
                ))),
                PRINT_ANSI => Some(
                    String::from_utf8_lossy(std::slice::from_raw_parts(
                        address as *const u8,
                        length.saturating_sub(1),
                    ))
                    .into_owned(),
                ),
                _ => None,
            };
            if let Some(words) = words
                && let Ok(mut all) = WORDS.lock()
            {
                all.push(words);
            }
        }
        0
    }

    /// Catches the debug strings of the process until the value goes.
    pub struct Catch(*mut c_void);

    impl Catch {
        pub fn new() -> Self {
            // SAFETY: a handler of this module, first in the chain.
            Self(unsafe { AddVectoredExceptionHandler(1, catch) })
        }

        /// A debug string of the test's own — the positive control of the catch.
        pub fn say(words: &str) {
            let wide: Vec<u16> = words.encode_utf16().chain([0]).collect();
            // SAFETY: a NUL-terminated string that lives across the call.
            unsafe { OutputDebugStringW(wide.as_ptr()) };
        }

        pub fn words() -> Vec<String> {
            WORDS.lock().map(|all| all.clone()).unwrap_or_default()
        }
    }

    impl Drop for Catch {
        fn drop(&mut self) {
            // SAFETY: the handle the call above answered.
            unsafe { RemoveVectoredExceptionHandler(self.0) };
        }
    }
}

/// **What Direct2D's debug layer says of «О программе»'s picture** — the series of the picture of the
/// panel with the debug layer on (`LANGSW_TESTING_D2D_DEBUG`, set here): its words about objects left
/// alive, caught in the process and printed. Needs the Graphics Tools of Windows (the stand
/// `lab-gfx`, 208.181); without them the layer says nothing — the positive control of the catch is
/// printed either way.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_debug_layer_names_what_the_picture_leaves() {
    // SAFETY: the variable of this test process, set before any window of it.
    unsafe { std::env::set_var("LANGSW_TESTING_D2D_DEBUG", "1") };
    let catch = debug_words::Catch::new();
    debug_words::Catch::say("ui_memory: the catch hears");
    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    let (before, _) = memory();
    for _ in 0..rounds {
        let hwnd = host::create(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell");
        // SAFETY: a window of this thread; the messages go straight to its procedure.
        unsafe {
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
    }
    let (after, _) = memory();
    let words = debug_words::Catch::words();
    drop(catch);

    let mut counted: Vec<(String, usize)> = Vec::new();
    for line in &words {
        let line = line.trim().to_owned();
        match counted.iter_mut().find(|(known, _)| *known == line) {
            Some((_, count)) => *count += 1,
            None => counted.push((line, 1)),
        }
    }
    let mut text = format!(
        "rounds {rounds}, private +{:.2} MB; debug strings {} ({} kinds)\r\n",
        after - before,
        words.len(),
        counted.len()
    );
    for (line, count) in &counted {
        text.push_str(&format!("{count:5} × {line}\r\n"));
    }
    println!("{text}");
    if let Some(path) = std::env::var_os("LANGSW_UI_MEMORY") {
        std::fs::write(path, &text).expect("the words written");
    }
    assert!(
        words.iter().any(|line| line.contains("the catch hears")),
        "the catch heard nothing, not even itself"
    );
}
