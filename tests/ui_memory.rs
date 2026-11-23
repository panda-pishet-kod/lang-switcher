//! **The memory a window of the shell leaves behind** — the instrument of the controller's question
//! (2026-11-20, the stand of T-120-13: «О программе» in the tray program held +8,8…11,4 MB after
//! closing, growing by 0,1 MB a time). It opens and closes the window again and again and writes the
//! private memory of the process after each: a line that keeps rising is something held, a line that
//! levels off is a cache. The instruments run on purpose: `--ignored --nocapture`;
//! `LANGSW_UI_MEMORY` — a file for the series.
//!
//! One test runs in the battery: the gate of 208.183,
//! [`the_memory_does_not_grow_with_every_opening`] — alone in this binary, so alone in its process.

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
                switch_key: None,
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
        let refs =
            lang_switcher::ui::canvas::FACTORY_REFS_LEFT.load(std::sync::atomic::Ordering::Relaxed);
        lines.push(format!(
            "{round:2}: private {private:.2} MB (+{:.2}), working set {working:.2} MB, gdi {gdi}, user {user}, factory refs left {refs}",
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
            switch_key: None,
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
            switch_key: None,
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
                switch_key: None,
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

/// **The memory while one «О программе» is made larger and smaller** (Fable's first measure): the
/// picture of the panel is drawn afresh at each new size in pixels — a line that rises with every
/// size would make dragging the frame cost megabytes; a level one ties the growth to the life of a
/// window. 200 sizes, each new.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_memory_while_the_about_window_is_resized() {
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
    };

    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(200);
    let strip = std::env::var("LANGSW_UI_MEMORY_STRIP").unwrap_or_default();
    let hwnd = host::create(
        Without(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                switch_key: None,
                theme: ThemeSetting::Light,
            },
            strip,
        ),
        None,
    )
    .expect("the window of the shell");
    // SAFETY: a window of this thread.
    unsafe { SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0))) };
    let (before, _) = memory();
    let mut lines = vec![format!("resize: before private {before:.2} MB")];
    for round in 1..=rounds {
        let (w, h) = (960 + round as i32 * 3, 600 + (round as i32 % 40) * 2);
        // SAFETY: a window of this thread; plain values.
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        let (private, _) = memory();
        lines.push(format!(
            "{round:3}: {w}x{h} private {private:.2} MB (+{:.2})",
            private - before
        ));
    }
    // SAFETY: a window of this thread.
    unsafe { SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0))) };
    pump();
    let (closed, _) = memory();
    lines.push(format!(
        "closed: private {closed:.2} MB (+{:.2})",
        closed - before
    ));
    let text = lines.join("\r\n");
    println!("{text}");
    if let Some(path) = std::env::var_os("LANGSW_UI_MEMORY") {
        std::fs::write(path, text + "\r\n").expect("the series written");
    }
}

/// «О программе» without parts (`LANGSW_UI_MEMORY_STRIP`), for the series of sizes.
struct Without(About, String);

impl lang_switcher::ui::host::Content for Without {
    type Id = <About as lang_switcher::ui::host::Content>::Id;

    fn describe(&self) -> lang_switcher::ui::scene::Window<Self::Id> {
        let mut window = self.0.describe();
        if self.1.contains("art") {
            window.art = None;
        }
        if self.1.contains("brand") {
            window.brand = None;
        }
        if self.1.contains("body") {
            window.body = lang_switcher::ui::scene::Node::Text(
                lang_switcher::ui::tokens::Role::Body,
                "body taken away".to_owned(),
            );
        }
        if self.1.contains("buttons") {
            window.buttons.clear();
        }
        window
    }

    fn event(
        &mut self,
        event: lang_switcher::ui::scene::Event<Self::Id>,
    ) -> lang_switcher::ui::host::Reply {
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

/// The heaps of the process walked (Fable's fourth measure): busy bytes and blocks, free bytes — a
/// leak adds busy blocks, a scattered heap adds free bytes.
mod heaps {
    use std::ffi::c_void;

    #[repr(C)]
    struct Entry {
        data: *mut c_void,
        size: u32,
        overhead: u8,
        region: u8,
        flags: u16,
        rest: [u64; 3],
    }

    #[link(name = "kernel32")]
    // SAFETY: the documented exports of kernel32, same signatures.
    unsafe extern "system" {
        fn GetProcessHeaps(count: u32, heaps: *mut *mut c_void) -> u32;
        fn HeapWalk(heap: *mut c_void, entry: *mut Entry) -> i32;
        fn HeapLock(heap: *mut c_void) -> i32;
        fn HeapUnlock(heap: *mut c_void) -> i32;
    }

    const BUSY: u16 = 0x0004;
    const REGION: u16 = 0x0001;
    const UNCOMMITTED: u16 = 0x0002;

    /// Busy bytes, busy blocks, free bytes, over every heap of the process.
    pub fn walk() -> (u64, u64, u64) {
        let mut handles = vec![std::ptr::null_mut(); 256];
        // SAFETY: a buffer of the length given.
        let count = unsafe { GetProcessHeaps(handles.len() as u32, handles.as_mut_ptr()) } as usize;
        let (mut busy, mut blocks, mut free) = (0u64, 0u64, 0u64);
        for &heap in handles.iter().take(count.min(handles.len())) {
            // SAFETY: a heap of this process, locked while it is walked.
            unsafe {
                if HeapLock(heap) == 0 {
                    continue;
                }
                let mut entry: Entry = std::mem::zeroed();
                while HeapWalk(heap, &mut entry) != 0 {
                    if entry.flags & BUSY != 0 {
                        busy += u64::from(entry.size);
                        blocks += 1;
                    } else if entry.flags & (REGION | UNCOMMITTED) == 0 {
                        free += u64::from(entry.size);
                    }
                }
                HeapUnlock(heap);
            }
        }
        (busy, blocks, free)
    }
}

/// The series of sizes with the heaps walked at its start, its 100th and 200th size and after the
/// window closed (`LANGSW_UI_MEMORY_STRIP` as for the series of sizes).
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_heaps_while_the_about_window_is_resized() {
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE,
    };

    let strip = std::env::var("LANGSW_UI_MEMORY_STRIP").unwrap_or_default();
    let show = |label: &str| {
        let (busy, blocks, free) = heaps::walk();
        let (private, _) = memory();
        println!(
            "{label}: private {private:.2} MB, busy {:.2} MB in {blocks} blocks, free {:.2} MB",
            busy as f64 / 1_048_576.0,
            free as f64 / 1_048_576.0
        );
    };
    let hwnd = host::create(
        Without(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                switch_key: None,
                theme: ThemeSetting::Light,
            },
            strip,
        ),
        None,
    )
    .expect("the window of the shell");
    // SAFETY: a window of this thread.
    unsafe { SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0))) };
    // `LANGSW_UI_MEMORY_DRAG`: the sizes come as a drag of the frame (WM_ENTERSIZEMOVE … WM_EXITSIZEMOVE).
    let drag = std::env::var_os("LANGSW_UI_MEMORY_DRAG").is_some();
    if drag {
        // SAFETY: a window of this thread.
        unsafe { SendMessageW(hwnd, WM_ENTERSIZEMOVE, Some(WPARAM(0)), Some(LPARAM(0))) };
    }
    show("start");
    for round in 1..=200 {
        let (w, h) = (960 + round * 3, 600 + (round % 40) * 2);
        // SAFETY: a window of this thread; plain values.
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        if round == 100 || round == 200 {
            show(&format!("size {round}"));
        }
    }
    if drag {
        // SAFETY: a window of this thread.
        unsafe {
            SendMessageW(hwnd, WM_EXITSIZEMOVE, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        show("let go");
    }
    // SAFETY: a window of this thread.
    unsafe { SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0))) };
    pump();
    show("closed");
}

/// The product's strings, from the product binary mapped as data (as `tests\ui_fit.rs` does): the
/// letter is planned from them.
fn with_product_strings() {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
    use windows::core::PCWSTR;

    static PRODUCT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let raw = *PRODUCT.get_or_init(|| {
        let exe = std::env::current_exe()
            .expect("the test executable has a path")
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");
        let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated path owned by this frame; mapped for its resources only.
        let module =
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
                .expect("the product binary opens as a data file");
        module.0 as usize
    });
    lang_switcher::settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
}

/// Openings counted together: the level of a block is the median of the private memory after its
/// openings, and the busy bytes of the heaps are walked at the end of each.
const BLOCK: usize = 20;

/// One series of the gate: the private memory after each step, KB, and the busy bytes of the heaps
/// at its start and after every [`BLOCK`] steps. The numbers go into buffers made before the series:
/// the series itself leaves nothing in the heap (a line of a journal kept per round was the one
/// block alive per opening that the measures of 208.183 found — the instrument's own).
struct Series {
    private: Vec<f64>,
    busy: Vec<u64>,
}

/// The median of `values` (sorted in place).
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let half = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[half - 1] + values[half]) / 2.0
    } else {
        values[half]
    }
}

impl Series {
    /// `steps` steps of `step`, each followed by the queue of this thread emptied and a measure.
    fn run(steps: usize, mut step: impl FnMut()) -> Self {
        let mut private = Vec::with_capacity(steps);
        let mut busy = Vec::with_capacity(steps / BLOCK + 1);
        busy.push(heaps::walk().0);
        for n in 1..=steps {
            step();
            pump();
            private.push(memory().0 * 1024.0);
            if n % BLOCK == 0 {
                busy.push(heaps::walk().0);
            }
        }
        Self { private, busy }
    }

    /// The growth per step, KB: the median of the rises from block to block, each block at the
    /// median of its private memory. A leak raises every block by the same; a cache taken once
    /// raises one block, and a spike given back within a block or two moves one median at most —
    /// neither moves the median of the rises.
    fn growth(&self) -> f64 {
        let levels: Vec<f64> = self
            .private
            .chunks_exact(BLOCK)
            .map(|block| median(&mut block.to_vec()))
            .collect();
        let mut rises: Vec<f64> = levels.windows(2).map(|pair| pair[1] - pair[0]).collect();
        median(&mut rises) / BLOCK as f64
    }

    /// The busy bytes of the heaps added per step, by the same median of the rises from block to block.
    fn busy(&self) -> f64 {
        let mut rises: Vec<f64> = self
            .busy
            .windows(2)
            .map(|pair| pair[1] as f64 - pair[0] as f64)
            .collect();
        median(&mut rises) / BLOCK as f64
    }

    /// The private memory after the last step, KB.
    fn last(&self) -> f64 {
        self.private.last().copied().unwrap_or_default()
    }

    /// The level of the last [`BLOCK`] steps: the median of their private memory, KB.
    fn end(&self) -> f64 {
        median(&mut self.private[self.private.len().saturating_sub(BLOCK)..].to_vec())
    }
}

/// One opening of the window `hwnd`: painted, closed.
fn opening(hwnd: windows::Win32::Foundation::HWND) {
    // SAFETY: a window of this thread; the messages go straight to its procedure.
    unsafe {
        SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
        SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
    }
}

/// The three series of the gate in one process: «О программе» warmed by `warm` openings, then
/// `steps` openings of it; rest — `steps` steps of nothing but the time one opening took on average;
/// the letter with the panda warmed and opened the same way.
fn the_series_of_the_gate(
    warm: usize,
    steps: usize,
) -> (Option<(f64, f64)>, Series, Series, Series) {
    use lang_switcher::letters::{self, Letter};
    use lang_switcher::ui::windows::letter::LetterView;

    with_product_strings();
    let context = letters::PlanContext {
        version: "1.8.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: true,
        item: None,
    };
    let about = || {
        host::create(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                switch_key: None,
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell")
    };
    let letter = || {
        host::create(
            LetterView {
                letter: Letter::Welcome,
                plan: letters::plan_in(Language::Ru, Letter::Welcome, &context),
                key: "Pause".to_owned(),
                tick: 0,
                language: Language::Ru,
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell")
    };

    let started = std::time::Instant::now();
    let mut warming = Vec::with_capacity(warm);
    for _ in 0..warm {
        opening(about());
        pump();
        warming.push(memory().0 * 1024.0);
    }
    let about_series = Series::run(steps, || opening(about()));
    let pace = started.elapsed() / (warm + steps).max(1) as u32;
    let rest = Series::run(steps, || std::thread::sleep(pace));
    for _ in 0..warm {
        opening(letter());
        pump();
    }
    let letter_series = Series::run(steps, || opening(letter()));
    // The warming of a fresh process, KB: the level of the openings 11…30 and that of the last 20 of
    // the warming (none with a warming shorter than 50 openings).
    let warmed = (warm >= 50).then(|| {
        (
            median(&mut warming[10..30].to_vec()),
            median(&mut warming[warm - BLOCK..].to_vec()),
        )
    });
    (warmed, about_series, rest, letter_series)
}

/// **The memory does not grow with every opening** — the gate of 208.183 (the word of the owner: the
/// program does not eat more memory with every opening of a window; T-120-24, a gate of the release).
/// In a fresh process «О программе» is opened `WARM` times to warm (the first window loads
/// Direct2D, the caches of a fresh process fill), then `STEPS` times more; then the process rests
/// as long; then the letter with the panda is warmed and opened the same way. Two parts:
///
/// * **(а) the warming of a fresh process** — the level of the last 20 openings of the warming
///   (the median) no more than `WARMING_KB` above that of the openings 11…30, the limit of the
///   decision 208.183 (+0,59…0,78 MB measured). A memory not given back after the last window
///   (`canvas::give_back_free_memory`) grows here: +5,6…6,5 MB.
/// * **The growth per opening after the warming**, each series ([`Series::growth`],
///   [`Series::busy`]) less the rest — what grows with the time alone is not the openings': the
///   private memory no more than `LIMIT_KB` per opening; the busy bytes of the heaps (`HeapWalk`,
///   exact, no pages) no more than `LIMIT_BUSY` per opening — a block kept per opening shows here
///   even where the pages hide it.
///
/// The series is not judged by the difference of its two ends: the memory after one closing catches
/// spikes (committed for a moment, given back within some 25 openings, 0,25…0,45 MB measured) and a
/// cache taken once is a step — the median of the rises from block to block is moved by neither.
/// The experiments of 208.183 (T-120-24): the free memory not given back and 4 KB kept per opening
/// turn it red; 1 MB taken once amid the series does not.
///
/// The only test of this binary run without `--ignored`: alone in its process, as the gate needs.
#[test]
fn the_memory_does_not_grow_with_every_opening() {
    const WARM: usize = 200;
    const STEPS: usize = 200;
    const LIMIT_KB: f64 = 2.0;
    const LIMIT_BUSY: f64 = 512.0;
    const WARMING_KB: f64 = 1.5 * 1024.0;

    let (warmed, about, rest, letter) = the_series_of_the_gate(WARM, STEPS);
    let (early, warm) = warmed.expect("a warming of 50 openings and more");
    let warming = warm - early;
    let line = |name: &str, series: &Series| {
        format!(
            "{name}: {:+.2} KB and {:+.0} busy bytes per opening, less the rest {:+.2} KB and {:+.0} B (after the last {:.0} KB)",
            series.growth(),
            series.busy(),
            series.growth() - rest.growth(),
            series.busy() - rest.busy(),
            series.last()
        )
    };
    let report = format!(
        "«О программе» warming, the openings 11…30 to {}…{WARM}: {early:.0} -> {warm:.0} KB, {warming:+.0} KB (the last {BLOCK} of the series {:.0} KB); {}; {}; rest {:+.2} KB and {:+.0} B per step",
        WARM - BLOCK + 1,
        about.end(),
        line("«О программе»", &about),
        line("the letter", &letter),
        rest.growth(),
        rest.busy()
    );
    println!("{report}");
    // Every limit is checked and every one passed is named: an experiment shows which part caught it.
    let mut failed = Vec::new();
    if warming > WARMING_KB {
        failed.push("(а) «О программе» grows past the warming of a fresh process".to_owned());
    }
    for (name, series) in [("(а) «О программе»", &about), ("(б) the letter", &letter)]
    {
        if series.growth() - rest.growth() > LIMIT_KB {
            failed.push(format!("{name} grows with every opening"));
        }
        if series.busy() - rest.busy() > LIMIT_BUSY {
            failed.push(format!("{name} keeps heap blocks with every opening"));
        }
    }
    assert!(failed.is_empty(), "{}: {report}", failed.join("; "));
}

/// **The series of the gate, written out** — the instrument behind its limits: the series of
/// [`the_memory_does_not_grow_with_every_opening`] with `LANGSW_UI_MEMORY_WARM` (200) and
/// `LANGSW_UI_MEMORY_STEPS` (200); the growth per 100 openings and the whole series into
/// `LANGSW_UI_MEMORY`.
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_series_of_the_gate_written_out() {
    let number = |name: &str, default: usize| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    };
    let (warm, steps) = (
        number("LANGSW_UI_MEMORY_WARM", 200),
        number("LANGSW_UI_MEMORY_STEPS", 200),
    );
    let (_, about, rest, letter) = the_series_of_the_gate(warm, steps);
    let mut lines = Vec::new();
    for (name, series) in [("about", &about), ("rest", &rest), ("letter", &letter)] {
        lines.push(format!(
            "{name}: growth {:+.2} KB/step, busy {:+.0} B/step, last {:.0} KB",
            series.growth(),
            series.busy(),
            series.last()
        ));
        for (part, chunk) in series.private.chunks(100).enumerate() {
            if chunk.len() == 100 {
                let piece = Series {
                    private: chunk.to_vec(),
                    busy: series.busy[part * 5..=(part * 5 + 5).min(series.busy.len() - 1)]
                        .to_vec(),
                };
                lines.push(format!(
                    "  {name} {}..{}: growth {:+.2} KB/step, busy {:+.0} B/step",
                    part * 100 + 1,
                    part * 100 + 100,
                    piece.growth(),
                    piece.busy()
                ));
            }
        }
    }
    let text = lines.join("\r\n");
    println!("{text}");
    if let Some(path) = std::env::var_os("LANGSW_UI_MEMORY") {
        let mut all = text + "\r\n";
        for (name, series) in [("about", &about), ("rest", &rest), ("letter", &letter)] {
            all += &format!("{name} private KB: ");
            all += &series
                .private
                .iter()
                .map(|kb| format!("{kb:.0}"))
                .collect::<Vec<_>>()
                .join(" ");
            all += &format!("\r\n{name} busy B: ");
            all += &series
                .busy
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            all += "\r\n";
        }
        std::fs::write(path, all).expect("the series written");
    }
}

/// **The series of the stand, in the test process** (the growth of 20.11 on the stand, 208.183): each
/// round opens the menu of the icon over a hidden owner that is no window of the shell — as the
/// hidden window of the program — paints it and closes it, then opens «О программе», paints it and
/// closes it. The rounds of the stand job, without the hook. `LANGSW_UI_MEMORY_ROUNDS` (600);
/// `LANGSW_UI_MEMORY_NO_MENU` leaves the menu out (the series of the gate, for comparison).
#[test]
#[ignore = "an instrument: run on purpose with --ignored --nocapture"]
fn the_memory_with_the_menu_of_the_icon() {
    use lang_switcher::tray::{Menu, MenuNow, Pending};
    use lang_switcher::ui::windows::tray_menu::TrayMenu;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
    };
    use windows::core::w;

    with_product_strings();
    let rounds: usize = std::env::var("LANGSW_UI_MEMORY_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(600);
    let with_menu = std::env::var_os("LANGSW_UI_MEMORY_NO_MENU").is_none();
    // SAFETY: a system class, a hidden window of this thread with no parent; destroyed below.
    let owner = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )
    }
    .expect("the hidden owner");
    let menu = || TrayMenu {
        now: MenuNow {
            menu: Menu::build_in(
                Language::Ru,
                true,
                true,
                false,
                false,
                false,
                &Pending::default(),
            ),
            enabled: true,
            fail_safe: false,
            key: "Pause".to_owned(),
            theme: ThemeSetting::Light,
        },
        language: Language::Ru,
    };

    let objects_before = objects();
    let mut after = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        if with_menu {
            let hwnd = host::create_menu(menu(), Some(owner), POINT { x: 800, y: 600 }, false)
                .expect("the menu of the icon");
            // SAFETY: a window of this thread; the message goes straight to its procedure.
            unsafe { SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0))) };
            host::close(hwnd);
            pump();
        }
        let about = host::create(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                switch_key: None,
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell");
        // SAFETY: as above.
        unsafe {
            SendMessageW(about, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(about, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        after.push(memory().0);
    }
    let objects_after = objects();
    // SAFETY: the window made above, of this thread.
    let _ = unsafe { DestroyWindow(owner) };

    let at = |n: usize| {
        after
            .get(n.min(after.len()) - 1)
            .copied()
            .unwrap_or_default()
    };
    println!(
        "menu {with_menu}: 1st {:.2}, 20th {:.2}, 100th {:.2}, 300th {:.2}, 600th {:.2} MB; 20->300 {:+.2}, 300->600 {:+.2}; gdi/user {:?} -> {:?}",
        at(1),
        at(20),
        at(100),
        at(300),
        at(600),
        at(300) - at(20),
        at(600) - at(300),
        objects_before,
        objects_after
    );
}
