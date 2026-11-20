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

/// The private memory after each of `rounds` openings and closings of the window `open` makes, MB.
/// The numbers go into a buffer made before the series: the series itself leaves nothing in the
/// heap (a line of a journal kept per round was the one block alive per opening that the measures
/// of 208.183 found — the instrument's own).
fn series(rounds: usize, mut open: impl FnMut() -> windows::Win32::Foundation::HWND) -> Vec<f64> {
    let mut after = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let hwnd = open();
        // SAFETY: a window of this thread; the messages go straight to its procedure.
        unsafe {
            SendMessageW(hwnd, WM_PAINT, Some(WPARAM(0)), Some(LPARAM(0)));
            SendMessageW(hwnd, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        pump();
        after.push(memory().0);
    }
    after
}

/// **The memory does not grow with every opening** — the gate of 208.183 (the word of the owner;
/// T-120-24, a gate of the release). In a fresh process, the memory after closing:
///
/// * (а) 300 openings of «О программе»: the 300th no more than the 20th + 1,5 MB. The warming of a
///   fresh process gives +0,68…0,87 MB here; without the free memory given back after the last
///   window (`canvas::give_back_free_memory`) +4,8…7,0.
/// * (б) right after it, 300 openings of the letter with the panda: the 300th no more than the
///   20th + 0,5 MB — no growth with every opening, the process warm (+0,08…0,26 measured).
///
/// «The 20th» and «the 300th» are levels, not single openings: the median of the openings 11…30
/// and of the openings 281…300. The memory after one closing can catch a spike — committed for a
/// moment, given back within some 25 openings, 0,25…0,45 MB measured — and a spike on either
/// single opening turned the gate red one run in two (+0,59 and +0,60 MB on a flat line).
///
/// The only test of this binary run without `--ignored`: alone in its process, as the gate needs.
#[test]
fn the_memory_does_not_grow_with_every_opening() {
    use lang_switcher::letters::{self, Letter};
    use lang_switcher::ui::windows::letter::LetterView;

    const ROUNDS: usize = 300;
    const WARMING_MB: f64 = 1.5;
    const GROWTH_MB: f64 = 0.5;

    with_product_strings();
    let context = letters::PlanContext {
        version: "1.8.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: true,
        item: None,
    };
    let about = series(ROUNDS, || {
        host::create(
            About {
                language: Language::Ru,
                key: "Pause".to_owned(),
                theme: ThemeSetting::Light,
            },
            None,
        )
        .expect("the window of the shell")
    });
    let letter = series(ROUNDS, || {
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
    });
    // The level about the n-th opening: the median of the 20 openings `from..=to` (counted from 1).
    let level = |after: &[f64], from: usize, to: usize| {
        let mut window = after[from - 1..to].to_vec();
        window.sort_by(f64::total_cmp);
        let half = window.len() / 2;
        (window[half - 1] + window[half]) / 2.0
    };
    let (about_20, about_300) = (level(&about, 11, 30), level(&about, 281, ROUNDS));
    let (letter_20, letter_300) = (level(&letter, 11, 30), level(&letter, 281, ROUNDS));
    let line = |name: &str, after: &[f64], at_20: f64, at_300: f64| {
        format!(
            "{name}: 1st {:.2}, 20th {:.2} (level {at_20:.2}), 300th {:.2} (level {at_300:.2}) MB — \
             {:+.2} from the 20th",
            after[0],
            after[19],
            after[ROUNDS - 1],
            at_300 - at_20
        )
    };
    let report = format!(
        "{}; {}",
        line("«О программе»", &about, about_20, about_300),
        line("the letter", &letter, letter_20, letter_300)
    );
    println!("{report}");
    assert!(
        about_300 <= about_20 + WARMING_MB,
        "(а) «О программе» grows past the warming of a fresh process: {report}"
    );
    assert!(
        letter_300 <= letter_20 + GROWTH_MB,
        "(б) the letter grows with every opening: {report}"
    );
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
