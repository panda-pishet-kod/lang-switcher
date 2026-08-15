//! Integration checks of module `guard` — password-field detection, FR-70 to FR-73, SEC-06.
//!
//! Task **T-06-1**.
//!
//! # What is checked here and what is not
//!
//! The decisions of module `guard` are functions of their arguments wherever they could be made
//! ones, and those are driven by the unit tests inside `src\guard.rs`. What needs a **live
//! window** is here, and it is exactly level 2 of FR-72: a real control created with
//! `ES_PASSWORD`, asked the question FR-72 says to ask, through the call FR-72 says to use, with
//! the timeout FR-72 writes.
//!
//! ⚠ **No password is ever typed, in this file or anywhere else in the project.** The window
//! below is created empty, asked for the *mask character it paints* — a property of the control,
//! identical for every password box in the world — and destroyed. Nothing is typed into it and
//! its contents are never read; there is no call here that could return them (SEC-01, SEC-07).
//!
//! What is **not** here:
//!
//! * level 1 — it needs a credential process, and section 10 limitation 1 puts `LogonUI.exe` and
//!   `consent.exe` on the secure desktop, where this program never runs. The name comparison is a
//!   pure function and is driven in `src\guard.rs`;
//! * level 3 — `IUIAutomation::GetFocusedElement` answers about whatever has the keyboard focus
//!   **on the whole desktop**, so a test asserting on it would be asserting on what the machine
//!   happened to be doing. It is verified on the running program and by the bench of §11.5,
//!   position 14;
//! * the wiring of the gate into the input thread — it lives in `src\app.rs`, needs the three
//!   threads and a hook, and is verified on the running program.
//!
//! The last group of checks reads the sources of `src\` as text. That is not a stylistic
//! preference: acceptance points 9 and 17 of the task are stated as sweeps over `src\` — "UI
//! Automation is not called from the hook callback", "`SendMessage` is used nowhere" — and a
//! sweep somebody runs by hand once is a sweep that stops being true on the next commit.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use lang_switcher::guard::{self, Field};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, ES_PASSWORD, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::w;

/// Serialises the tests that write the **process-global** state of module `guard` — the published
/// field, the counters, the pending-probe flag and the exclusion table.
///
/// `cargo test` runs the tests of a binary in parallel, and every one of those is one location
/// for the whole process. The three tests below that write any of them take this first; the rest
/// of the file reads sources and asserts on pure functions and needs nothing.
///
/// The same device, and the same reason, as `MIRROR` in `tests\control.rs`. Task **T-06-3** added
/// it when it added a third writer: two of the three were already sharing the pending-probe flag
/// without saying so.
static GLOBAL_STATE: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------
// A control of our own — the only password field this project ever creates
// ---------------------------------------------------------------------------------------

/// A live `EDIT` control owned by this test, destroyed when the value is dropped.
///
/// Created as a `WS_POPUP` with no parent and never shown: the class is what level 2 looks at
/// and the style bit is what it asks about, and neither needs the control to be visible, to have
/// a parent or to have the focus. Nothing is typed into it — see the module note.
struct Editor(HWND);

impl Editor {
    /// Creates one, with `ES_PASSWORD` or without it.
    fn new(password: bool) -> Self {
        let style = if password {
            WINDOW_STYLE(WS_POPUP.0 | ES_PASSWORD as u32)
        } else {
            WS_POPUP
        };

        // SAFETY: `EDIT` is a system class registered for every process, so the `'static`
        // literal names a class that exists without this test registering anything. The window
        // name is null — the documented way to ask for no text — and no `lpParam` is passed, so
        // nothing of ours is handed to a window procedure. `None` for the parent, the menu and
        // the instance are all documented values for a popup created by the calling process. The
        // handle is checked below.
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                None,
                style,
                0,
                0,
                120,
                24,
                None,
                None,
                None,
                None,
            )
        }
        .expect("an EDIT control of our own must be creatable");

        assert!(!handle.is_invalid(), "the control has a handle");

        Self(handle)
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from a successful `CreateWindowExW` above and is destroyed
        // exactly once — this type is neither `Copy` nor `Clone`. `DestroyWindow` requires the
        // calling thread to be the one that created the window, and it is: the value never
        // leaves the test body that made it.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

/// **Level 2 of FR-72 against a real `ES_PASSWORD` control.**
///
/// Three separate claims, and each of them is a line of the requirement:
///
/// * the class of such a control is the `Edit` FR-72 names, so level 2 applies to it at all;
/// * `SendMessageTimeout(EM_GETPASSWORDCHAR)` answers, and answers **non-zero**;
/// * a non-zero answer is the verdict `Password`, which is «буферизация отключена».
#[test]
fn a_real_es_password_control_is_recognised_by_level_two() {
    let editor = Editor::new(true);

    let class = guard::class_name(editor.0);
    assert!(
        guard::class_is_edit(&class),
        "an ES_PASSWORD control is of the class FR-72 names, and this one is {class:?}"
    );

    let mask = guard::password_char(editor.0);
    assert!(
        mask.is_some(),
        "a control of this process answers inside the fifty milliseconds of FR-72"
    );
    assert_ne!(
        mask,
        Some(0),
        "FR-72: an ES_PASSWORD control paints a mask character"
    );

    assert_eq!(
        guard::field_for_password_char(mask),
        Some(Field::Password),
        "FR-72 level 2: «ненулевой результат → буферизация отключена»"
    );
    assert!(!guard::buffering_allowed_for(Field::Password), "FR-70");
}

/// The same control **without** the style bit, so that the check above is measuring the bit and
/// not merely the fact that an `EDIT` exists.
///
/// A plain editor is not a verdict of its own — FR-72 names no level that concludes "ordinary"
/// before level 3 — but it must not be mistaken for a password field, which is what this pins.
#[test]
fn a_plain_edit_control_paints_no_mask() {
    let editor = Editor::new(false);

    assert!(guard::class_is_edit(&guard::class_name(editor.0)));

    assert_eq!(
        guard::password_char(editor.0),
        Some(0),
        "a control without ES_PASSWORD has no mask character"
    );
    assert_eq!(
        guard::field_for_password_char(guard::password_char(editor.0)),
        Some(Field::Ordinary)
    );
}

/// A window that is not an `Edit` at all never reaches level 2 — the class test is what stops
/// `EM_GETPASSWORDCHAR` from being sent to a control that has no idea what it means.
#[test]
fn a_window_that_is_not_an_edit_does_not_reach_level_two() {
    // SAFETY: `STATIC` is a system class registered for every process; every argument is the same
    // documented shape as in `Editor::new`, which the safety note there covers.
    let handle = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            None,
            WS_POPUP,
            0,
            0,
            120,
            24,
            None,
            None,
            None,
            None,
        )
    }
    .expect("a STATIC control must be creatable");

    let class = guard::class_name(handle);
    assert!(
        !guard::class_is_edit(&class),
        "a STATIC is not an Edit, and this one reports {class:?}"
    );

    // SAFETY: the handle came from the successful call above, is destroyed once, and this is the
    // thread that created it.
    let _ = unsafe { DestroyWindow(handle) };
}

// ---------------------------------------------------------------------------------------
// The published state — FR-71's interval and FR-73's default
// ---------------------------------------------------------------------------------------

/// **The FR-73 fall-back when there is nobody to answer.**
///
/// A test process has no watcher thread and therefore no watcher window, so
/// [`guard::note_focus_moved`] cannot hand the probe anywhere. Leaving the state at `Pending`
/// then would be a program that never buffers again; FR-73 says what an undeterminable field
/// means, and this is that path.
///
/// ⚠ This test writes process-global state — the published field and the counters — so it is the
/// only one in this file that does, and everything it asserts it asserts about values it set
/// itself in the same body.
#[test]
fn a_focus_change_with_no_watcher_thread_ends_in_the_fr73_default() {
    let _serialised = GLOBAL_STATE.lock().unwrap_or_else(PoisonError::into_inner);

    let before = guard::counters();

    guard::note_focus_moved();

    assert_eq!(
        guard::field(),
        Field::Undetermined,
        "FR-73: with nobody able to determine the field, buffering is on"
    );
    assert!(guard::buffering_allowed());
    assert!(
        !guard::password_field(),
        "SEC-04a: the flag is 0 when this is not a password field"
    );

    let after = guard::counters();
    assert_eq!(after.focus_changes, before.focus_changes + 1);
    assert_eq!(
        after.undetermined_verdicts,
        before.undetermined_verdicts + 1,
        "the FR-73 verdict is counted as one"
    );
    assert_eq!(
        after.probes, before.probes,
        "no probe ran: there was no thread to run it on"
    );
}

/// SEC-05: a probe nobody asked for does nothing.
///
/// [`guard::WM_APP_PROBE`] can be posted to this program's watcher window by any process at the
/// same integrity level. What that buys the sender is this: `false`, and not one Win32 call.
#[test]
fn a_probe_that_was_not_asked_for_does_nothing() {
    let _serialised = GLOBAL_STATE.lock().unwrap_or_else(PoisonError::into_inner);

    assert!(
        !guard::run_pending_probe(),
        "nothing is pending in a test process, so the forged message finds nothing"
    );
}

/// The four states and the two answers, as the rest of the program sees them.
#[test]
fn the_flag_of_sec04a_is_one_bit_and_follows_the_state() {
    // The pure rule, for all four states at once. The live flag is asserted against the state
    // the test above establishes; asserting it here as well would be asserting on the order two
    // tests happened to run in.
    assert!(!guard::buffering_allowed_for(Field::Password));
    assert!(!guard::buffering_allowed_for(Field::Pending));
    assert!(guard::buffering_allowed_for(Field::Ordinary));
    assert!(guard::buffering_allowed_for(Field::Undetermined));
}

/// The budget of the interval is a number, and it is the sum of the bounds the three levels are
/// given rather than a wish.
#[test]
fn the_pending_interval_is_bounded_by_the_budgets_of_the_levels() {
    assert_eq!(guard::PASSWORD_CHAR_TIMEOUT_MS, 50, "FR-72 writes fifty");

    // FR-73 is unreachable without a bound on level 3, and the interval in which nothing is
    // buffered has to stay bounded by a small number of seconds — a user who typed for longer
    // than that without the buffer following would notice. Both are properties of constants, so
    // they are checked where a constant is checked: at compile time.
    const _: () = assert!(guard::UIA_TIMEOUT_MS > 0);
    const _: () = assert!(guard::PROBE_BUDGET_MS <= 2_000);

    assert_eq!(
        guard::PROBE_BUDGET_MS,
        guard::PASSWORD_CHAR_TIMEOUT_MS + 2 * guard::UIA_TIMEOUT_MS
    );
}

// ---------------------------------------------------------------------------------------
// The sweeps of acceptance points 9 and 17 — over the sources, every time
// ---------------------------------------------------------------------------------------

/// Every `.rs` file under `src\`, with its path.
fn sources() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();

    for entry in fs::read_dir(&root).expect("src\\ must be readable") {
        let path = entry.expect("a readable directory entry").path();

        if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path).expect("a readable source file");
            out.push((path, text));
        }
    }

    assert!(out.len() >= 12, "every module of section 6.2 was read");
    out
}

/// The file name of a path under `src\`, as the assertion messages spell it.
fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// One named module of `src\`, read as text.
fn source_of(module: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);

    fs::read_to_string(&path).unwrap_or_else(|_| panic!("src\\{module} must be readable"))
}

/// Lines of `text` that contain `needle` and are not comment lines.
///
/// The whole point of these sweeps is to separate a **call** from a sentence about a call: this
/// file and `src\guard.rs` both discuss `SendMessage` and UI Automation at length, and a sweep
/// that counted prose would be a sweep nobody could keep green.
fn code_lines_with<'a>(text: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| {
            !line.starts_with("//") && !line.starts_with("///") && !line.starts_with("//!")
        })
        .filter(|(_, line)| line.contains(needle))
        .collect()
}

/// **Acceptance point 9, as a test rather than as a grep somebody ran once.**
///
/// FR-71 calls UI Automation inside the hook «категорически запрещено». The callback lives in
/// `src\hook.rs`, and so does everything it calls that is not another module's public function,
/// so the check is: **not one UI Automation name occurs in `src\hook.rs` at all**, in code or in
/// prose, and every occurrence anywhere under `src\` is in `src\guard.rs`.
///
/// That is stronger than "the callback does not call it" and is what makes it checkable: a
/// future edit that put a UI Automation call anywhere on the hook's path would have to name one
/// of these symbols in `hook.rs` first.
#[test]
fn no_ui_automation_name_occurs_anywhere_near_the_hook() {
    const UIA_NAMES: [&str; 6] = [
        "IUIAutomation",
        "CUIAutomation",
        "GetFocusedElement",
        "CurrentIsPassword",
        "UIA_IsPassword",
        "UIAutomationCore",
    ];

    for (path, text) in sources() {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();

        for symbol in UIA_NAMES {
            let hits: Vec<usize> = text
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(symbol))
                .map(|(index, _)| index + 1)
                .collect();

            if hits.is_empty() {
                continue;
            }

            assert_eq!(
                name, "guard.rs",
                "FR-71: {symbol} occurs in src\\{name} at lines {hits:?}; UI Automation belongs to \
                 module guard and to the watcher thread, and above all not to the hook"
            );
        }
    }
}

/// **Acceptance point 17.** `SendMessageTimeout` is what FR-72 names, and a plain `SendMessage`
/// is what it forbids by implication: to a window whose thread is not pumping, `SendMessage`
/// blocks the caller for ever, and the caller here holds the `WinEvent` subscriptions of §6.1.
///
/// The sweep is over the whole of `src\` and not only over `guard.rs`, because the requirement is
/// about the program and not about one module.
#[test]
fn send_message_is_used_nowhere_and_the_timeout_is_the_fifty_of_fr72() {
    for (path, text) in sources() {
        for (line_number, line) in code_lines_with(&text, "SendMessage") {
            assert!(
                line.contains("SendMessageTimeout"),
                "FR-72: a bare SendMessage in {} at line {line_number}: {line}",
                path.display()
            );
        }
    }

    // And the one call that exists is made with the number FR-72 writes, taken from the constant
    // rather than spelled out at the call site.
    let guard_source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("guard.rs"),
    )
    .expect("src\\guard.rs must be readable");

    assert!(
        guard_source.contains("PASSWORD_CHAR_TIMEOUT_MS,"),
        "the timeout argument is the constant, so the two cannot drift apart"
    );
    assert_eq!(guard::PASSWORD_CHAR_TIMEOUT_MS, 50);
}

/// **Acceptance point 12.** The subscription of `EVENT_OBJECT_FOCUS` is the one task T-03-3 made,
/// and this task raised no second one: `SetWinEventHook` occurs in `src\watchdog.rs` and nowhere
/// else.
#[test]
fn the_focus_subscription_is_still_the_only_one_in_the_program() {
    let mut owners = Vec::new();

    for (path, text) in sources() {
        if !code_lines_with(&text, "SetWinEventHook").is_empty() {
            owners.push(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }

    assert_eq!(
        owners,
        ["watchdog.rs"],
        "the WinEvent subscriptions belong to module watchdog and to no other"
    );
}

/// **SEC-01, SEC-07 over the module that reads other programs' windows.** Nothing module `guard`
/// learns may become text that is stored: no journal line, no panic, no file.
///
/// The two things it *does* report through `app::report_non_critical` are operation **names** —
/// `'static` strings written here — and `diag::Operation::from_name` narrows even those onto a
/// closed table. What this pins is that no formatting macro anywhere in the module builds a
/// string out of something it read from another process.
#[test]
fn module_guard_formats_nothing_it_learned_about_another_program() {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("guard.rs"),
    )
    .expect("src\\guard.rs must be readable");

    // Everything below `mod tests` is test code and is allowed its assertion messages.
    let product = text
        .split_once("mod tests {")
        .map_or(text.as_str(), |(before, _)| before);

    for forbidden in [
        "format!",
        "println!",
        "eprintln!",
        "panic!",
        "write!",
        "writeln!",
        "unwrap()",
        "expect(",
        "dbg!",
        "WM_GETTEXT",
        "ValuePattern",
        "TextPattern",
        "CurrentName",
    ] {
        let hits = code_lines_with(product, forbidden);

        assert!(
            hits.is_empty(),
            "SEC-01/SEC-07/NFR-14: {forbidden} occurs in the product half of src\\guard.rs at \
             {hits:?}"
        );
    }
}

/// **NFR-14.** Every `unsafe` block in the module carries a `// SAFETY:` note above it.
#[test]
fn every_unsafe_in_module_guard_carries_a_safety_note() {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("guard.rs"),
    )
    .expect("src\\guard.rs must be readable");

    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if !trimmed.contains("unsafe {") || trimmed.starts_with("//") {
            continue;
        }

        blocks += 1;

        // The note is the nearest comment above the statement the block belongs to, so the search
        // walks up over the argument lines of a multi-line call.
        let noted = lines[..index]
            .iter()
            .rev()
            .take(12)
            .any(|above| above.trim_start().starts_with("// SAFETY:"));

        assert!(
            noted,
            "NFR-14: the unsafe block at src\\guard.rs line {} has no // SAFETY: note",
            index + 1
        );
    }

    assert!(
        blocks > 0,
        "the sweep found the unsafe blocks it was aimed at"
    );
}

// ---------------------------------------------------------------------------------------
// FR-84 — the process exclusion list (task T-06-3)
// ---------------------------------------------------------------------------------------

/// **Acceptance point 9, as a sweep rather than as a grep somebody ran once.**
///
/// The chain that answers "which process owns the foreground window" —
/// `GetForegroundWindow`, `GetWindowThreadProcessId`, `OpenProcess`,
/// `QueryFullProcessImageNameW` — does not fit in the hundred microseconds NFR-01 gives the
/// callback, and `OpenProcess` can be refused, which costs the same as a success. So the check is
/// the strongest checkable form of "not on the hook path": **not one of those names occurs in
/// `src\hook.rs` at all**, in code or in prose.
///
/// `GetGUIThreadInfo` rides along for the same reason: it is level 2's way of asking another
/// thread what has the focus, and it is another cross-thread call the callback must not make.
///
/// The reading of the *name* is narrower still and is asserted as such:
/// `QueryFullProcessImageNameW` occurs in `src\guard.rs` and in no other module of the program.
#[test]
fn no_call_that_reads_a_process_name_occurs_anywhere_near_the_hook() {
    const PROCESS_NAMES: [&str; 5] = [
        "GetForegroundWindow",
        "GetWindowThreadProcessId",
        "OpenProcess(",
        "QueryFullProcessImageNameW",
        "GetGUIThreadInfo",
    ];

    for (path, text) in sources() {
        let module = file_name(&path);

        for symbol in PROCESS_NAMES {
            let hits: Vec<usize> = text
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains(symbol))
                .map(|(index, _)| index + 1)
                .collect();

            if hits.is_empty() {
                continue;
            }

            assert_ne!(
                module, "hook.rs",
                "NFR-01: {symbol} occurs in src\\hook.rs at lines {hits:?}; determining the \
                 process is the watcher thread's work and never the callback's"
            );

            if symbol == "QueryFullProcessImageNameW" {
                assert_eq!(
                    module, "guard.rs",
                    "the name of a process is read in module guard and nowhere else; {symbol} \
                     occurs in src\\{module} at lines {hits:?}"
                );
            }
        }
    }
}

/// **Acceptance point 10, and it holds in a form stronger than the point asks for.**
///
/// The point is that the callback reads only an atomic flag. What is asserted is that the
/// callback reads **nothing of module `guard` whatever**: `src\hook.rs` names no item of it, so
/// there is no flag on the hook path to read, correctly or otherwise.
///
/// The flag is read one thread-hop away, by `app::apply_buffering_gate`, on the input thread's
/// message loop — which is where section 6.3 puts the reader — and what the gate does with it is
/// take the typing buffer away. A stroke in a password field or an excluded process therefore
/// meets the presence check `buffer::record` performs for every stroke on the machine and takes
/// its shorter arm: FR-70 and FR-84 cost the callback **less** than the ordinary case, not more.
#[test]
fn the_hook_callback_names_nothing_of_module_guard() {
    let hook = source_of("hook.rs");

    let hits = code_lines_with(&hook, "guard::");

    assert!(
        hits.is_empty(),
        "§6.3: src\\hook.rs reaches into module guard at {hits:?}; the flag is read by the input \
         thread's message loop and not by the callback"
    );

    // And the reader really is where this claims it is.
    let app = source_of("app.rs");

    assert!(
        !code_lines_with(&app, "guard::buffering_allowed()").is_empty(),
        "the gate on the input thread is what reads the published state"
    );
}

/// **Acceptance point 16 — FR-96 works in an excluded process, and it is a property of position.**
///
/// The emergency combination is handled at the top of `keyboard_hook_proc`, before `classify` is
/// reached and therefore before `buffer::record` exists as a possibility. Nothing of this task can
/// stand in front of it, because nothing of this task is in that file at all (see the test above),
/// and what FR-84 does — take the typing buffer away — happens below the return that FR-96 never
/// gets to.
///
/// So the check is on the order of the lines: within `keyboard_hook_proc`, the test of FR-96
/// precedes the decision, and no call into `buffer` or `guard` precedes either.
#[test]
fn the_emergency_combination_is_reached_before_any_logic_this_task_touches() {
    let hook = source_of("hook.rs");

    let callback = hook
        .split_once("unsafe extern \"system\" fn keyboard_hook_proc")
        .map(|(_, after)| after)
        .expect("src\\hook.rs must contain the callback of FR-01");

    let first = |needle: &str| {
        code_lines_with(callback, needle)
            .first()
            .map(|(line, _)| *line)
    };

    let emergency = first("is_emergency_key").expect("FR-96 is tested in the callback");
    let exit = first("emergency_exit()").expect("and acted on there");
    let decision = first("guarded_decision").expect("the ordinary decision follows it");

    assert!(
        emergency < exit && exit < decision,
        "FR-96: the emergency test is at {emergency}, the exit at {exit} and the decision at \
         {decision}; the combination is handled first, before any other logic of the callback"
    );

    for later in ["buffer::", "guard::", "classify("] {
        for (line, text) in code_lines_with(callback, later) {
            assert!(
                line > emergency,
                "FR-96: {text:?} at line {line} of the callback stands before the emergency test \
                 at {emergency}"
            );
        }
    }
}

/// **Acceptance points 12 and 19.** The heavy part is out of the `WinEvent` callback, and nothing
/// on the path that reads the exclusion list takes a lock.
///
/// Point 12 is the same sweep as the one for `SetWinEventHook` above, read from the other side:
/// module `guard` contains no `WinEvent` callback, so the heavy part cannot be in one. Where it
/// *is* — the watcher thread's message loop, reached through the `WM_APP_FLUSH` the subscription
/// of task T-03-3 already posts — is asserted by `app.rs` naming `run_pending_probe` on the
/// watcher window.
///
/// Point 19 is NFR-04 and section 6.3: the list is **published** into a table of atomics and
/// searched without a lock, so an update cannot block a reader and a reader cannot block the
/// program. A module that names no blocking primitive cannot put one on a read path.
#[test]
fn the_list_is_published_and_the_heavy_part_is_out_of_the_callback() {
    let guard_source = source_of("guard.rs");

    let product = guard_source
        .split_once("mod tests {")
        .map_or(guard_source.as_str(), |(before, _)| before);

    for blocking in [
        "Mutex", "RwLock", "Condvar", "OnceLock", "LazyLock", ".lock(", "Barrier", "mpsc",
    ] {
        let hits = code_lines_with(product, blocking);

        assert!(
            hits.is_empty(),
            "NFR-04, §6.3: {blocking} occurs in the product half of src\\guard.rs at {hits:?}; the \
             exclusion list is published, not shared"
        );
    }

    // Nor does the module go to the file itself: §6.1 gives the configuration to the UI thread,
    // and what crosses to the watcher thread is a published value.
    for reader in ["fs::", "File::", "read_to_string", "settings::"] {
        let hits = code_lines_with(product, reader);

        assert!(
            hits.is_empty(),
            "§6.1: {reader} occurs in the product half of src\\guard.rs at {hits:?}"
        );
    }

    // And the module raises no subscription of its own — point 11 from this side.
    assert!(
        code_lines_with(product, "SetWinEventHook").is_empty(),
        "the subscription of EVENT_SYSTEM_FOREGROUND is the one task T-03-3 made"
    );

    // The heavy part runs on the watcher window, which is what puts it off both callbacks.
    let app = source_of("app.rs");

    let probe: Vec<(usize, &str)> = code_lines_with(&app, "run_pending_probe");

    assert!(
        !probe.is_empty(),
        "the probe is dispatched from the window procedure"
    );

    assert!(
        app.contains("crate::guard::WM_APP_PROBE && is_watcher_window(hwnd)"),
        "NFR-10, §6.1: the probe runs on the watcher window and on no other"
    );
}

/// **The live half of the name read, against a name that is known** — NFR-13.
///
/// `OpenProcess` and `QueryFullProcessImageNameW` are what level 1 of FR-72 and the whole of
/// FR-84 stand on, and until this task a `None` from them was invisible: it let the chain fall
/// through to levels 2 and 3, which is what a correct read of an ordinary process does as well.
/// FR-84 has no level to fall through to, so the read is checked here against the one process
/// whose name a test knows for certain — its own.
#[test]
fn the_live_process_name_read_answers_for_this_process() {
    let expected = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .expect("a running test binary has a file name");

    let read = guard::process_file_name(std::process::id())
        .expect("NFR-13: this process can read its own image name");

    assert_eq!(
        guard::fold_process_name(&read),
        guard::fold_process_name(&expected),
        "the name Win32 reports for this process is the name of its own binary"
    );

    // And the value really is a bare name rather than the path it was cut out of.
    assert!(!read.contains('\\') && !read.contains('/'), "{read:?}");
}

/// **The exclusion list against this process's own name** — acceptance point 18, and acceptance
/// point 23 in the small.
///
/// The name is the one Windows would report for this very binary, taken from `current_exe`, so
/// what is checked is the real string and not a convenient one. Nothing is excluded before the
/// list names it, everything the list names is matched however it is spelled, and the default of
/// section 7 is put back at the end.
#[test]
fn the_published_list_recognises_this_process_by_its_own_name() {
    let _serialised = GLOBAL_STATE.lock().unwrap_or_else(PoisonError::into_inner);

    let exe = std::env::current_exe().expect("a running test binary has a path");

    let name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .expect("and a name")
        .to_owned();

    // Section 7's default: `processes = []`, and nothing is excluded.
    assert_eq!(guard::publish_exclusions(&[]), 0);
    assert!(
        !guard::is_excluded_name(&name),
        "FR-84: the default list excludes nothing, this process included"
    );

    // The user writes the name in whatever case they please.
    assert_eq!(guard::publish_exclusions(&[name.to_uppercase()]), 1);

    for spelling in [name.clone(), name.to_uppercase(), name.to_lowercase()] {
        assert!(
            guard::is_excluded_name(&spelling),
            "FR-84: {spelling} is the name that was published, in another case"
        );
    }

    // Or writes the whole path, which FR-84 says is the same program: «имён процессов».
    let path = exe.to_string_lossy().into_owned();

    assert_eq!(guard::publish_exclusions(&[path]), 1);
    assert!(
        guard::is_excluded_name(&name),
        "FR-84: a configured path is compared by its last component"
    );

    // A neighbour of the same name plus something is a different program.
    let mut neighbour = name.clone();
    neighbour.push_str(".exe");

    assert!(!guard::is_excluded_name(&neighbour));
    assert!(!guard::is_excluded_name("notepad.exe"));

    // Section 7's default put back — this is process-global state and the rest of the run must see
    // the program as it starts.
    assert_eq!(guard::publish_exclusions(&[]), 0);
    assert!(!guard::is_excluded_name(&name));
    assert_eq!(
        guard::counters().exclusion_read_retries,
        0,
        "no search had to be repeated: nothing else in this process publishes"
    );
}

// ---------------------------------------------------------------------------------------
// The key of SEC-04a — condition 2, a flag and not the content
// ---------------------------------------------------------------------------------------

/// **Acceptance point 22.** The channel carries `password_field`, it carries it as `0` or `1`,
/// and it carries nothing else that this task added.
#[cfg(feature = "testing")]
#[test]
fn the_channel_publishes_the_flag_and_not_the_field() {
    use lang_switcher::control;

    assert!(
        control::KEYS.contains(&"password_field"),
        "SEC-04a: the key SEC-06 is proved through is emitted"
    );

    let text = control::render(&control::snapshot());

    let line = text
        .lines()
        .find(|line| line.starts_with("password_field="))
        .expect("the key is on the wire");

    let value = line.split_once('=').expect("key=value").1;

    assert!(
        value == "0" || value == "1",
        "condition 2 of SEC-04a: a flag and not the content, and this is {value:?}"
    );

    assert!(
        text.is_ascii(),
        "no decoded character can be riding in the payload"
    );
}
