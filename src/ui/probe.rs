//! The probe of the interface shell (stage Э120, T-120-9), testing build only: the window
//! «О программе» on the shell opened alone by `LangSwitcher.exe --ui-probe-about`, with the marks
//! the instruments of the stand read. Not a window description: it holds the instruments.

use crate::settings;
use crate::theme::ThemeSetting;
use crate::ui::windows::about::About;

/// The entry of the probe, testing build only: `LangSwitcher.exe --ui-probe-about` opens the window
/// in the language of the interface and runs until it is closed. For the instruments of the stand
/// (`LANGSW_TESTING_UI_PROBE` names the file the marks go to, each with the wall clock):
///
/// * `LANGSW_TESTING_UI_PROBE_WAIT_MS` — wait so long before asking for the window (`asked_unix_ms`),
///   so that the memory before it can be read;
/// * `LANGSW_TESTING_UI_PROBE_CLOSE_MS` — the window closes itself after so many milliseconds;
/// * `LANGSW_TESTING_UI_PROBE_LINGER_MS` — after the window is gone, stay so long at each step:
///   closed (`closed_unix_ms`, the pages by kind, the references the factory of Direct2D still had),
///   then after `HeapCompact` of the process heap (`compacted_unix_ms`);
/// * `LANGSW_TESTING_UI_PROBE_TWICE` — then open the window a second time and close it again
///   (`asked2_unix_ms`, `first_frame2_unix_ms`, `closed2_unix_ms`).
pub fn run_about() -> std::process::ExitCode {
    use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use ::windows::Win32::System::Memory::{GetProcessHeap, HEAP_NONE, HeapCompact};
    use ::windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, IsWindow, MSG, PostMessageW, TranslateMessage, WM_CLOSE,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let now = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis())
    };
    let millis = |name: &str| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_millis)
    };
    let report = std::env::var_os("LANGSW_TESTING_UI_PROBE");
    let mut lines = String::new();
    let mut mark = |line: String| {
        lines.push_str(&line);
        lines.push_str("\r\n");
        if let Some(path) = &report {
            let _ = std::fs::write(path, &lines);
        }
    };
    let linger = || {
        if let Some(linger) = millis("LANGSW_TESTING_UI_PROBE_LINGER_MS") {
            std::thread::sleep(linger);
        }
    };

    // One window: asked, drawn, closed by itself after CLOSE_MS, its loop run to the end.
    let show = |mark: &mut dyn FnMut(String), suffix: &str| -> bool {
        mark(format!("asked{suffix}_unix_ms={}", now()));
        let about = About {
            language: settings::ui_language(),
            key: "Pause".to_owned(),
            theme: ThemeSetting::System,
        };
        let Ok(window) = crate::ui::host::open(about, None) else {
            return false;
        };
        mark(format!("first_frame{suffix}_unix_ms={}", now()));
        if let Some(after) = millis("LANGSW_TESTING_UI_PROBE_CLOSE_MS") {
            let raw = window.0 as usize;
            std::thread::spawn(move || {
                std::thread::sleep(after);
                // SAFETY: a posted message to the probe's window; a window already gone ignores it.
                let _ = unsafe {
                    PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0))
                };
            });
        }
        let mut message = MSG::default();
        // SAFETY: the message loop of this thread; it ends when the window is gone.
        while unsafe { IsWindow(Some(window)) }.as_bool()
            && unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool()
        {
            // SAFETY: a message just taken from the queue.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        mark(format!("closed{suffix}_unix_ms={}", now()));
        true
    };

    if let Some(wait) = millis("LANGSW_TESTING_UI_PROBE_WAIT_MS") {
        std::thread::sleep(wait);
    }
    mark(format!("pages_before={}", pages()));
    if !show(&mut mark, "") {
        return std::process::ExitCode::FAILURE;
    }
    mark(format!("pages_after={}", pages()));
    mark(format!(
        "d2d_factory_refs_left={}",
        crate::ui::canvas::FACTORY_REFS_LEFT.load(std::sync::atomic::Ordering::Relaxed)
    ));
    linger();

    // SAFETY: the heap of this process; only free blocks are given back.
    let largest =
        unsafe { GetProcessHeap() }.map_or(0, |heap| unsafe { HeapCompact(heap, HEAP_NONE) });
    mark(format!(
        "compacted_unix_ms={} largest_free={largest}",
        now()
    ));
    mark(format!("pages_compacted={}", pages()));
    linger();

    if std::env::var_os("LANGSW_TESTING_UI_PROBE_TWICE").is_some() && show(&mut mark, "2") {
        mark(format!("pages_after2={}", pages()));
        linger();
    }
    std::process::ExitCode::SUCCESS
}

/// The committed pages of this process by kind, in bytes — private, image, mapped — by walking the
/// address space with `VirtualQuery`: what a window leaves behind, by where it lives.
fn pages() -> String {
    use ::windows::Win32::System::Memory::{
        MEM_COMMIT, MEM_IMAGE, MEM_MAPPED, MEM_PRIVATE, MEMORY_BASIC_INFORMATION, VirtualQuery,
    };

    let (mut private, mut image, mut mapped) = (usize::MIN, usize::MIN, usize::MIN);
    let mut address = usize::MIN;
    loop {
        let mut region = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: a query of this process's own address space into a local.
        let answered = unsafe {
            VirtualQuery(
                Some(address as *const _),
                &mut region,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if answered == usize::MIN || region.RegionSize == usize::MIN {
            break;
        }
        if region.State == MEM_COMMIT {
            match region.Type {
                kind if kind == MEM_PRIVATE => private += region.RegionSize,
                kind if kind == MEM_IMAGE => image += region.RegionSize,
                kind if kind == MEM_MAPPED => mapped += region.RegionSize,
                _ => {}
            }
        }
        let Some(next) = (region.BaseAddress as usize).checked_add(region.RegionSize) else {
            break;
        };
        address = next;
    }
    format!("private:{private} image:{image} mapped:{mapped}")
}
