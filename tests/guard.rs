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

use lang_switcher::guard::{self, Field};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, ES_PASSWORD, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::w;

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
