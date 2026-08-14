//! Module `switch` — the fallback chain of FR-50, the scope of FR-51, the reading of FR-52 and
//! the refusal of an IME target from FR-35. Task **T-05-1**.
//!
//! # What is checked here and what is not
//!
//! The requirement of this task is a **rule about failure**: a method of FR-50 counts as failed
//! when the layout did not become the target, and never because the call that attempted it
//! returned an error — decision R-32. That rule is invisible from outside a chain that obeys it.
//! On a machine where method 1 works, a correct chain and a chain that trusts `PostMessage`
//! behave identically, every single time, and the whole fallback mechanism of methods 2 and 3
//! would be dead code with nothing to notice it by.
//!
//! So the chain is driven through [`lang_switcher::switch::Machine`], the seam the module
//! provides, and the case that tells the two apart is built explicitly: a machine whose
//! `post_request` answers `true` and whose layout never moves. See
//! [`a_method_that_reports_success_without_changing_the_layout_is_still_a_failure`].
//!
//! The behavioural half — a real switch of a real window — is at the bottom of this file, marked
//! `#[ignore]`. It is not part of `cargo test` on purpose: it changes the keyboard layout of the
//! machine a person is sitting at, and it is run deliberately, with
//! `cargo test --test switch -- --ignored --nocapture`. Everything it touches is a window this
//! test created itself, and the layout it found is put back however it ends (decision R-42).

use std::sync::{Mutex, MutexGuard};

use lang_switcher::layouts::LayoutId;
use lang_switcher::switch::{
    self, Failures, Machine, Method, Outcome, Scope, SwitchError, VERIFY_BUDGET_MS, VERIFY_POLL_MS,
};

// ---------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------

/// The counters of module `switch` are process-wide statics and `cargo test` runs test
/// functions in parallel, so every test that asserts a count takes this first.
static COUNTERS: Mutex<()> = Mutex::new(());

/// Takes [`COUNTERS`] and zeroes the counters, so that a test starts from a known point.
///
/// A panicking test poisons the lock; its guard is still usable, and failing every later test
/// because an earlier one failed would hide the real cause.
fn counters() -> MutexGuard<'static, ()> {
    let guard = COUNTERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    switch::reset_failures();
    guard
}

/// US, `0x04090409`. An ordinary keyboard layout, not a text service.
const US: LayoutId = LayoutId::from_raw(0x0409_0409);

/// Russian, `0x04190419`.
const RU: LayoutId = LayoutId::from_raw(0x0419_0419);

/// Chinese Simplified, Microsoft Pinyin: `0xE0200804`.
///
/// A **TSF/IME** profile, and the reason FR-35 exists — the top nibble `0xE` of the device
/// handle is what Windows reserves for text services that compose characters.
const PINYIN: LayoutId = LayoutId::from_raw(0xE020_0804);

/// What a method of the fake machine did to the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    /// What the method's Win32 call answered. `true` is "the call succeeded", which decision
    /// R-32 says is **not** the same statement as "the layout changed".
    returned: bool,
    /// Whether the layout actually became the target.
    switched: bool,
}

impl Report {
    /// The trap of this task: the call reports success and nothing happens.
    const LIES: Self = Self {
        returned: true,
        switched: false,
    };

    /// The call reports success and the layout really changes.
    const WORKS: Self = Self {
        returned: true,
        switched: true,
    };

    /// The call itself failed and nothing happened.
    const REFUSED: Self = Self {
        returned: false,
        switched: false,
    };
}

/// A machine whose every answer the test writes.
///
/// It records what the chain asked of it, which is how the order of FR-50 is checked: an order
/// is not observable from outside a function that performs it.
#[derive(Clone, Debug)]
struct Fake {
    /// What [`Machine::current`] answers.
    layout: LayoutId,
    /// What each of the three methods does, in the order of FR-50.
    methods: [Report; 3],
    /// What [`Machine::wait`] answers it waited, in milliseconds.
    slice_ms: u32,

    // ---- what the chain did ----
    posts: u32,
    activates: u32,
    handovers: u32,
    reads: u32,
    waits: u32,
    waited_ms: u32,
    scopes: Vec<Scope>,
    targets: Vec<LayoutId>,
}

impl Fake {
    /// A machine sitting on `layout`, on which every method lies (see [`Report::LIES`]).
    fn on(layout: LayoutId) -> Self {
        Self {
            layout,
            methods: [Report::LIES; 3],
            slice_ms: VERIFY_POLL_MS,
            posts: 0,
            activates: 0,
            handovers: 0,
            reads: 0,
            waits: 0,
            waited_ms: 0,
            scopes: Vec::new(),
            targets: Vec::new(),
        }
    }

    /// Makes method `index` (`0`, `1` or `2` in the order of FR-50) the one that really works.
    fn working(mut self, index: usize) -> Self {
        self.methods[index] = Report::WORKS;
        self
    }

    /// Makes method `index` fail its own call as well as its effect.
    fn refusing(mut self, index: usize) -> Self {
        self.methods[index] = Report::REFUSED;
        self
    }

    /// Applies method `index` and answers what its call reported.
    fn attempt(&mut self, index: usize, target: LayoutId, scope: Option<Scope>) -> bool {
        self.targets.push(target);

        if let Some(scope) = scope {
            self.scopes.push(scope);
        }

        let report = self.methods[index];

        if report.switched {
            self.layout = target;
        }

        report.returned
    }

    /// How many times the chain called any of the three methods.
    fn attempts(&self) -> u32 {
        self.posts + self.activates + self.handovers
    }
}

impl Machine for Fake {
    fn current(&mut self) -> LayoutId {
        self.reads += 1;
        self.layout
    }

    fn post_request(&mut self, target: LayoutId) -> bool {
        self.posts += 1;
        self.attempt(0, target, None)
    }

    fn activate(&mut self, target: LayoutId, scope: Scope) -> bool {
        self.activates += 1;
        self.attempt(1, target, Some(scope))
    }

    fn hand_over(&mut self, target: LayoutId, scope: Scope) -> bool {
        self.handovers += 1;
        self.attempt(2, target, Some(scope))
    }

    fn wait(&mut self, ms: u32) -> u32 {
        assert_eq!(
            ms, VERIFY_POLL_MS,
            "the chain must ask for one slice at a time, so that a switch that took effect \
             early is noticed early"
        );

        self.waits += 1;
        self.waited_ms += self.slice_ms;
        self.slice_ms
    }
}

// ---------------------------------------------------------------------------------------
// Point 10 — the trap of the task
// ---------------------------------------------------------------------------------------

/// **Point 10, FR-50, decision R-32.** The one test this task exists for.
///
/// Every method's call answers success and not one of them changes the layout. `PostMessage`
/// returns success when the message is **queued**, and an application is free to ignore it, so
/// this is not a contrived case — it is what happens in every application that does not handle
/// `WM_INPUTLANGCHANGEREQUEST`.
///
/// A chain that read return values would stop at method 1 and report success. This chain must
/// re-read the layout by FR-52, see that nothing moved, and go on to methods 2 and 3.
#[test]
fn a_method_that_reports_success_without_changing_the_layout_is_still_a_failure() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    // Not `Switched(PostMessage)`. That is the whole assertion.
    assert_eq!(outcome, Ok(Outcome::HandedOver));

    assert_eq!(fake.posts, 1, "method 1 ran");
    assert_eq!(
        fake.activates, 1,
        "method 2 ran, which a chain trusting the return value of method 1 would never do"
    );
    assert_eq!(fake.handovers, 1, "method 3 was handed over");

    let counted = switch::failures();
    assert_eq!(counted.post_message, 1, "method 1 is counted as failed");
    assert_eq!(counted.attach_activate, 1, "method 2 is counted as failed");
    assert_eq!(
        counted.post_rejected, 0,
        "the call itself reported success — the verdict came from re-reading, not from it"
    );
}

/// The mirror image, so that the test above cannot pass by a chain that always runs all three.
#[test]
fn a_method_that_really_switches_ends_the_chain() {
    let _guard = counters();

    let mut fake = Fake::on(US).working(0);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Ok(Outcome::Switched(Method::PostMessage)));
    assert_eq!(fake.posts, 1);
    assert_eq!(
        fake.activates, 0,
        "method 2 must not run after method 1 worked"
    );
    assert_eq!(fake.handovers, 0);
    assert_eq!(switch::failures(), Failures::default());
}

/// NFR-13 against R-32: a call that reports failure is **counted**, and the verdict still comes
/// from re-reading — so a method whose call failed but whose effect happened is a success.
#[test]
fn a_call_that_reports_failure_is_counted_and_is_still_not_the_verdict() {
    let _guard = counters();

    // Method 1's call fails outright.
    let mut fake = Fake::on(US).refusing(0).working(1);
    assert_eq!(
        switch::to_in(&mut fake, Scope::PerWindow, RU),
        Ok(Outcome::Switched(Method::AttachActivate))
    );

    let counted = switch::failures();
    assert_eq!(
        counted.post_rejected, 1,
        "NFR-13: the return value was examined"
    );
    assert_eq!(
        counted.post_message, 1,
        "and the method is counted as failed too"
    );
    assert_eq!(counted.activate_rejected, 0);
}

// ---------------------------------------------------------------------------------------
// Points 11 and 12 — the order of the chain
// ---------------------------------------------------------------------------------------

/// **Point 11, FR-50.** Method 2 runs when method 1 did not change the layout.
#[test]
fn method_two_runs_when_method_one_did_not_change_the_layout() {
    let _guard = counters();

    let mut fake = Fake::on(US).working(1);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Ok(Outcome::Switched(Method::AttachActivate)));
    assert_eq!(fake.posts, 1, "method 1 was tried first");
    assert_eq!(fake.activates, 1);
    assert_eq!(
        fake.handovers, 0,
        "method 3 must not run after method 2 worked"
    );
    assert_eq!(fake.layout, RU);

    assert_eq!(switch::failures().post_message, 1);
    assert_eq!(switch::failures().attach_activate, 0);
}

/// **Point 12, FR-50 and decision R-31.** Method 3 is reached when neither of the first two
/// changed the layout — and it is *handed over*, not performed here.
#[test]
fn method_three_runs_when_neither_of_the_first_two_changed_the_layout() {
    let _guard = counters();

    let mut fake = Fake::on(US).working(2);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Ok(Outcome::HandedOver));
    assert_eq!((fake.posts, fake.activates, fake.handovers), (1, 1, 1));

    let counted = switch::failures();
    assert_eq!(counted.post_message, 1);
    assert_eq!(counted.attach_activate, 1);
    assert_eq!(
        counted.text_services, 0,
        "method 3's verdict belongs to the watcher thread, not to the chain"
    );
}

/// The end of the chain: methods 1 and 2 failed and the watcher thread was not there to take
/// method 3.
#[test]
fn a_handover_that_finds_no_watcher_thread_ends_the_chain() {
    let _guard = counters();

    let mut fake = Fake::on(US).refusing(2);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Err(SwitchError::Exhausted));
    assert_eq!(switch::failures().exhausted, 1);
}

/// Every method is asked for the target the caller named, and for no other layout.
#[test]
fn every_method_is_asked_for_the_target_the_caller_named() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let _ = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(fake.targets, vec![RU, RU, RU]);
}

// ---------------------------------------------------------------------------------------
// Point 13 — the wait is bounded above
// ---------------------------------------------------------------------------------------

/// **Point 13, section 6.1 and FR-80.** The wait between an attempt and its check has a
/// ceiling, and the ceiling is on the time really waited.
///
/// The input thread holds the low-level hook, and a thread that stops being available for
/// longer than `LowLevelHooksTimeout` — about 5000 ms — loses the hook silently. The chain's
/// worst case is the two methods that run on that thread.
#[test]
fn the_wait_between_an_attempt_and_its_check_is_bounded() {
    let _guard = counters();

    // Nothing ever settles, which is the worst case: every method spends its whole budget.
    let mut fake = Fake::on(US);
    let _ = switch::to_in(&mut fake, Scope::PerWindow, RU);

    let ceiling = 2 * (VERIFY_BUDGET_MS + VERIFY_POLL_MS);
    assert!(
        fake.waited_ms <= ceiling,
        "the chain waited {} ms, ceiling {ceiling} ms",
        fake.waited_ms
    );

    // And the ceiling itself is far below the timeout that costs the program its hook.
    assert!(
        u64::from(ceiling) * 100 < 5000,
        "the whole chain must stay at least two orders of magnitude inside LowLevelHooksTimeout"
    );
}

/// The ceiling is on **time**, not on a count of slices.
///
/// `Sleep(1)` on Windows rounds up to the system timer resolution, about 15.6 ms by default. A
/// loop that counted twenty one-millisecond slices would sit on the input thread for a third of
/// a second; a loop that adds up what the waits report ends after the first overshoot.
#[test]
fn a_wait_that_overshoots_its_slice_ends_the_verification_at_once() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    fake.slice_ms = 1_000;

    let _ = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(
        fake.waits, 2,
        "one slice per method, because one slice already spent the whole budget"
    );
}

/// A switch that takes effect early does not spend the rest of the budget.
#[test]
fn a_switch_that_takes_effect_at_once_costs_one_slice() {
    let _guard = counters();

    let mut fake = Fake::on(US).working(0);
    let _ = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(fake.waits, 1);
    assert_eq!(fake.waited_ms, VERIFY_POLL_MS);
}

// ---------------------------------------------------------------------------------------
// Point 14 — method 3 belongs to the watcher thread
// ---------------------------------------------------------------------------------------

/// **Point 14, decision R-31 and section 6.1.** The chain hands method 3 over and does not wait
/// for it: no verification follows the handover, so the input thread returns at once.
#[test]
fn the_chain_does_not_wait_for_method_three() {
    let _guard = counters();

    let mut fake = Fake::on(US).working(2);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Ok(Outcome::HandedOver));

    // One read before the chain starts, then one per slice of the two verified methods. If the
    // handover were verified here as well there would be more, and the input thread would be
    // waiting on COM performed by another thread.
    let verified_reads = 2 * VERIFY_BUDGET_MS / VERIFY_POLL_MS;
    assert_eq!(fake.reads, 1 + verified_reads);
    assert_eq!(fake.waits, verified_reads);
}

/// **Point 14 and SEC-05.** The channel the handover of R-31 travels on.
///
/// The message carries nothing; the target lives in an atomic this process alone writes. So a
/// forged `WM_APP_SWITCH` from another process finds nothing pending and does nothing, and one
/// request can never become two switches.
#[test]
fn the_handover_channel_is_single_use_and_empty_until_it_is_filled() {
    let _guard = counters();

    assert_eq!(
        switch::take_pending(),
        None,
        "SEC-05: a forged message finds nothing"
    );

    switch::publish_pending(RU, Scope::Session);
    assert_eq!(switch::take_pending(), Some((RU, Scope::Session)));
    assert_eq!(switch::take_pending(), None, "one request, one switch");

    switch::publish_pending(US, Scope::PerWindow);
    assert_eq!(switch::take_pending(), Some((US, Scope::PerWindow)));
}

/// `run_pending` on a machine with nothing pending does nothing at all — which is what a forged
/// message gets, and what makes it safe to call from the shared window procedure.
#[test]
fn run_pending_with_nothing_pending_does_nothing() {
    let _guard = counters();

    assert_eq!(switch::take_pending(), None);
    assert_eq!(switch::run_pending(), None);
    assert_eq!(switch::failures(), Failures::default());
}

// ---------------------------------------------------------------------------------------
// Point 16 — FR-51, the scope
// ---------------------------------------------------------------------------------------

/// **Point 16, FR-51.** The scope reaches both methods whose behaviour depends on it, unchanged.
#[test]
fn the_scope_of_fr_51_reaches_the_two_methods_that_have_one() {
    let _guard = counters();

    for scope in [Scope::PerWindow, Scope::Session] {
        let mut fake = Fake::on(US);
        let _ = switch::to_in(&mut fake, scope, RU);

        assert_eq!(
            fake.scopes,
            vec![scope, scope],
            "method 2 and the handover of method 3 both take the scope FR-51 decides"
        );
    }
}

/// **FR-51.** The Windows setting is on by default, so the default of the type is the scope that
/// setting means.
#[test]
fn the_default_scope_is_the_windows_default() {
    assert_eq!(Scope::default(), Scope::PerWindow);
}

/// **Point 16, FR-51 on the real machine.** The setting is read from the OS and answers one of
/// the two scopes, without a failure being swallowed.
#[test]
fn the_setting_of_fr_51_is_readable_on_this_machine() {
    let _guard = counters();

    let scope = switch::scope();

    assert!(matches!(scope, Scope::PerWindow | Scope::Session));
    assert_eq!(
        switch::failures().scope_unreadable,
        0,
        "SPI_GETTHREADLOCALINPUTSETTINGS answered; a failure would have been counted here"
    );

    println!("FR-51 on this machine: {scope:?}");
}

// ---------------------------------------------------------------------------------------
// Point 17 — FR-35, an IME can never be the target
// ---------------------------------------------------------------------------------------

/// **Point 17, FR-35.** A TSF/IME layout as the target is a refusal and a count, not a switch.
///
/// Module `layouts` keeps IME-based layouts out of the participating set, so a caller that asks
/// for one has a defect. The chain must not attempt anything: composition in an IME breaks the
/// correspondence between keystrokes and the field's contents, which is what FR-35 is about.
#[test]
fn an_ime_target_is_refused_and_counted_and_nothing_is_attempted() {
    let _guard = counters();

    assert!(PINYIN.is_ime(), "the fixture really is an IME handle");

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, PINYIN);

    assert_eq!(outcome, Err(SwitchError::ImeTarget));
    assert_eq!(fake.attempts(), 0, "no method may run for an IME target");
    assert_eq!(fake.reads, 0, "not even the layout is read");
    assert_eq!(switch::failures().ime_target, 1);
}

/// The zero handle names no layout and is refused for the same reason.
#[test]
fn the_zero_handle_is_refused_and_counted() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, LayoutId::default());

    assert_eq!(outcome, Err(SwitchError::NoTarget));
    assert_eq!(fake.attempts(), 0);
    assert_eq!(switch::failures().no_target, 1);
}

/// No foreground window — the secure desktop, or a desktop switch in progress — is a refusal and
/// not an attempt to switch a window that is not there.
#[test]
fn no_foreground_window_is_refused_and_counted() {
    let _guard = counters();

    let mut fake = Fake::on(LayoutId::default());
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Err(SwitchError::NoForeground));
    assert_eq!(fake.attempts(), 0);
    assert_eq!(switch::failures().no_foreground, 1);
}

// ---------------------------------------------------------------------------------------
// Point 24 — asking for the layout that is already active
// ---------------------------------------------------------------------------------------

/// **Point 24.** A switch to the layout the window already has is not a failure, and it costs no
/// switching at all.
#[test]
fn a_target_that_is_already_active_is_not_a_failure_and_switches_nothing() {
    let _guard = counters();

    let mut fake = Fake::on(RU);
    let outcome = switch::to_in(&mut fake, Scope::PerWindow, RU);

    assert_eq!(outcome, Ok(Outcome::AlreadyActive));
    assert_eq!(fake.attempts(), 0, "no redundant WM_INPUTLANGCHANGEREQUEST");
    assert_eq!(fake.waits, 0, "and no time spent on the input thread");
    assert_eq!(switch::failures(), Failures::default(), "no counter moves");
}

// ---------------------------------------------------------------------------------------
// Point 18 — the counters are public
// ---------------------------------------------------------------------------------------

/// **Point 18, SEC-04a and section 11.5.** Every kind of failure this module knows is counted
/// and the counts are reachable from outside the crate — this test is itself the proof, since it
/// is an integration test and can only see the public interface.
#[test]
fn every_kind_of_failure_is_counted_and_the_counts_are_public() {
    let _guard = counters();
    assert_eq!(switch::failures(), Failures::default());

    // FR-35, the zero handle, no foreground window.
    let mut fake = Fake::on(US);
    let _ = switch::to_in(&mut fake, Scope::PerWindow, PINYIN);
    let _ = switch::to_in(&mut fake, Scope::PerWindow, LayoutId::default());
    let _ = switch::to_in(&mut Fake::on(LayoutId::default()), Scope::PerWindow, RU);

    // Methods 1 and 2 failing, method 3 not even handed over.
    let _ = switch::to_in(
        &mut Fake::on(US).refusing(0).refusing(2),
        Scope::PerWindow,
        RU,
    );

    let counted = switch::failures();
    assert_eq!(counted.ime_target, 1);
    assert_eq!(counted.no_target, 1);
    assert_eq!(counted.no_foreground, 1);
    assert_eq!(counted.post_message, 1);
    assert_eq!(counted.attach_activate, 1);
    assert_eq!(counted.post_rejected, 1);
    assert_eq!(counted.exhausted, 1);
}

// ---------------------------------------------------------------------------------------
// Point 20 — SEC-01 and SEC-07
// ---------------------------------------------------------------------------------------

/// **Point 20, SEC-01 and SEC-07.** Nothing that could carry a keystroke exists in the counters
/// or in the error values, and their rendered forms are fixed strings and digits.
///
/// The counters are twelve `u32`s and the errors are four fieldless variants, so there is
/// nowhere for a character, a scan code or a piece of the typing buffer to be. This test pins
/// that shape: a field or a variant that started carrying data would break it.
#[test]
fn nothing_that_could_carry_a_keystroke_reaches_the_counters_or_the_errors() {
    let _guard = counters();

    for error in [
        SwitchError::ImeTarget,
        SwitchError::NoTarget,
        SwitchError::NoForeground,
        SwitchError::Exhausted,
    ] {
        let rendered = format!("{error:?}");
        assert!(
            rendered.chars().all(|ch| ch.is_ascii_alphabetic()),
            "an error value must be a bare name: {rendered}"
        );
    }

    for outcome in [
        Outcome::AlreadyActive,
        Outcome::HandedOver,
        Outcome::Switched(Method::PostMessage),
        Outcome::Switched(Method::AttachActivate),
        Outcome::Switched(Method::TextServices),
    ] {
        let rendered = format!("{outcome:?}");
        assert!(
            rendered
                .chars()
                .all(|ch| ch.is_ascii_alphabetic() || ch == '(' || ch == ')'),
            "an outcome must be a bare name: {rendered}"
        );
    }

    // The counters after a run that failed every way it could: names, digits and punctuation of
    // the derived `Debug`, and no value that came from a keystroke, because there is none to
    // come from.
    let _ = switch::to_in(&mut Fake::on(US), Scope::PerWindow, RU);
    let rendered = format!("{:?}", switch::failures());

    assert!(
        rendered
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || " ,:{}_".contains(ch)),
        "the counters render as names and numbers only: {rendered}"
    );
}

// ---------------------------------------------------------------------------------------
// Point 9 — FR-52
// ---------------------------------------------------------------------------------------

/// **Point 9, FR-52.** The layout is read by the expression the requirement writes:
/// `GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), null))`.
///
/// The test computes that expression itself, from the same three functions, and compares. It
/// cannot prove the module contains those three calls — that is a reading of the source, and it
/// is in the report — but it does pin the *answer*: a re-implementation that read
/// `GetKeyboardLayout(0)`, the calling thread's layout, would disagree as soon as the foreground
/// window belongs to another process, which is the case while `cargo test` runs.
#[test]
fn fr_52_reads_the_layout_of_the_thread_that_owns_the_foreground_window() {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: the three calls of FR-52, each taking values and returning a value, none of them
    // writing through a pointer of ours. `None` asks `GetWindowThreadProcessId` for the thread
    // id alone. A null foreground window yields thread zero, which is handled below.
    let expected = unsafe {
        let foreground = GetForegroundWindow();

        if foreground.is_invalid() {
            LayoutId::default()
        } else {
            let thread = GetWindowThreadProcessId(foreground, None);

            if thread == 0 {
                LayoutId::default()
            } else {
                LayoutId::from_raw(GetKeyboardLayout(thread).0 as usize)
            }
        }
    };

    assert_eq!(switch::current(), expected);
}

// ---------------------------------------------------------------------------------------
// Behavioural — points 21, 22, 24 and 25, on a window this test creates itself
// ---------------------------------------------------------------------------------------

/// **Points 21, 22, 24 and 25.** A real switch of a real window, and the layout put back.
///
/// ⚠ **Decision R-42.** The window is created by this test, the foreground is asked for and
/// **checked**, and nothing is switched unless our own window is the one in front. A test that
/// could not get the foreground says so and switches nothing, which is the required behaviour:
/// there is no fighting other people's windows.
///
/// ⚠ The layout the machine was on is restored however this ends, panic included.
///
/// `#[ignore]` on purpose: this changes the keyboard layout of the machine a person is sitting
/// at, so it runs only when it is asked for by name —
/// `cargo test --test switch -- --ignored --nocapture`.
/// # Why the window lives on a thread of its own
///
/// Method 1 is a **posted** message: it takes effect when the window that owns the queue runs
/// its own message loop. A window created and then left alone by the test thread never pumps,
/// so method 1 could never work and the run would measure nothing but the fallback. The bench
/// therefore puts the window on a thread that pumps, which is also what a real application is,
/// and drives the chain from the test thread — so `AttachThreadInput` in method 2 is a real
/// attach between two real threads rather than a self-attach that is skipped.
#[test]
#[ignore = "changes the keyboard layout of the machine; run deliberately with --ignored"]
fn behavioural_a_real_window_is_switched_and_the_layout_is_put_back() {
    use own_window::{Bench, Restore};

    let scope = switch::scope();
    println!("FR-51 scope on this machine: {scope:?}");

    let session = lang_switcher::layouts::enumerate().expect("the session has usable layouts");
    let names: Vec<String> = session.iter().map(ToString::to_string).collect();
    println!("layouts in this session: {names:?}");

    let us = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0409);
    let ru = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0419);

    let (Some(us), Some(ru)) = (us, ru) else {
        println!("SKIPPED: this session does not carry both US and Russian");
        return;
    };

    let Some(bench) = Bench::open() else {
        // Decision R-42: record it, do not fight for the foreground.
        println!("SKIPPED: our own window did not reach the foreground; nothing was switched");
        return;
    };

    let started_on = switch::current();
    println!("layout of our window before anything: {started_on}");
    let _restore = Restore::to(started_on, bench.handle());

    switch::reset_failures();

    // ---- the run starts from a known layout ---------------------------------------------
    let outcome = switch::to(us).expect("switching to US must not be refused");
    println!("normalising to US: {outcome:?}");
    assert_eq!(switch::current(), us, "the window is on US");

    // ---- point 21: US -> RU -------------------------------------------------------------
    let mut timed = Timed::default();
    let outcome = switch::to_in(&mut timed, scope, ru);
    println!(
        "point 21: US -> RU  outcome {outcome:?}, settled after {} ms in {} slices",
        timed.waited_ms, timed.waits
    );
    assert!(matches!(outcome, Ok(Outcome::Switched(_))), "{outcome:?}");
    assert_eq!(switch::current(), ru, "point 21: the layout is now Russian");

    // ---- point 22: RU -> US -------------------------------------------------------------
    let mut timed = Timed::default();
    let outcome = switch::to_in(&mut timed, scope, us);
    println!(
        "point 22: RU -> US  outcome {outcome:?}, settled after {} ms in {} slices",
        timed.waited_ms, timed.waits
    );
    assert!(matches!(outcome, Ok(Outcome::Switched(_))), "{outcome:?}");
    assert_eq!(switch::current(), us, "point 22: the layout is back on US");

    // ---- point 24: asking again for what is already there --------------------------------
    let mut timed = Timed::default();
    let outcome = switch::to_in(&mut timed, scope, us);
    println!(
        "point 24: US -> US  outcome {outcome:?}, {} waits",
        timed.waits
    );
    assert_eq!(outcome, Ok(Outcome::AlreadyActive));
    assert_eq!(timed.waits, 0, "point 24: no redundant switching");

    // ---- FR-51, measured rather than assumed --------------------------------------------
    //
    // The scope decides whether method 2 has to attach to the foreground window's thread. The
    // claim behind the `Scope::Session` branch is that with the setting off the input language
    // is one value for the whole session, so activating a layout on *this* thread reaches the
    // window on the other one. That is measurable here, and it is measured rather than assumed.
    let reached = own_window::activate_without_attach_reaches(ru);
    println!(
        "FR-51 probe: ActivateKeyboardLayout on this thread, no attach, reached the other \
         thread's window: {reached}   (scope {scope:?})"
    );

    println!("counters after the run: {:?}", switch::failures());

    // ---- point 25: the layout the machine was on is put back -----------------------------
    //
    // Done here, while our own window is still the foreground one, and asserted. `Restore` stays
    // as the net under a panic: it is the same call, and by the time it runs there is nothing
    // left for it to do.
    let outcome = switch::to(started_on);
    println!("point 25: restoring {started_on} -> {outcome:?}");
    assert_eq!(
        switch::current(),
        started_on,
        "point 25: the layout is back where the test found it"
    );
}

/// **Point 11 on the real machine, and the proof that the fallback chain is not dead code.**
///
/// A window that never runs a message loop is a window that never processes a posted message.
/// `PostMessage` still reports success — the message *was* queued — so this is exactly the case
/// decision R-32 was written about, and it happens on real applications, not only in a test:
/// a console host, a window busy in a modal operation, a control that swallows the message.
///
/// The chain must therefore come out of this on **method 2**. A chain that read the return value
/// of `PostMessage` would come out of it reporting method 1 and leaving the layout unchanged.
#[test]
#[ignore = "changes the keyboard layout of the machine; run deliberately with --ignored"]
fn behavioural_a_window_that_never_pumps_falls_through_to_method_two() {
    use own_window::{Restore, TestWindow};

    let scope = switch::scope();
    let session = lang_switcher::layouts::enumerate().expect("the session has usable layouts");

    let us = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0409);
    let ru = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0419);

    let (Some(us), Some(ru)) = (us, ru) else {
        println!("SKIPPED: this session does not carry both US and Russian");
        return;
    };

    // Created on *this* thread and never pumped again — the whole point of the case.
    let Some(window) = TestWindow::open_foreground() else {
        println!("SKIPPED: our own window did not reach the foreground; nothing was switched");
        return;
    };

    let started_on = switch::current();
    let _restore = Restore::to(started_on, window.handle());

    switch::reset_failures();

    let target = if started_on == ru { us } else { ru };
    let outcome = switch::to(target);

    println!("a window that never pumps: {started_on} -> {target}, outcome {outcome:?}");
    println!("counters: {:?}", switch::failures());

    assert_eq!(
        outcome,
        Ok(Outcome::Switched(Method::AttachActivate)),
        "method 1 cannot work against a queue nobody drains, so method 2 must"
    );
    assert_eq!(switch::current(), target);
    assert_eq!(
        switch::failures().post_message,
        1,
        "and method 1 must be counted as failed, from the re-read and not from its return value"
    );
    assert_eq!(
        switch::failures().post_rejected,
        0,
        "PostMessageW itself reported success — decision R-32 in one line"
    );

    let outcome = switch::to(started_on);
    println!("restoring {started_on} -> {outcome:?} (scope {scope:?})");
    assert_eq!(switch::current(), started_on);
}

/// **Point 23 — the whole scenario of section 11.3, in a window this test owns.**
///
/// Type `ghbdtn`, press the hotkey, expect **`привет`** *and* the layout of the window to be
/// Russian. Both halves, against the real program: this is the first time the second half is
/// checkable at all, and it is the sentence the acceptance matrix is built out of.
///
/// ⚠ **Decision R-42 throughout.** The only process this touches is the one it launched, and the
/// only window is the one it created. Nothing is sent until `GetForegroundWindow` has answered
/// with our own window, and if it never does the run says so and sends nothing.
///
/// ⚠ The program carries a live global keyboard hook, so it is started with
/// `LANGSW_DEBUG_TIMEOUT_SEC` (FR-97) and is killed here as well; the layout is put back.
#[test]
#[ignore = "runs the product with a live global hook; run deliberately with --ignored"]
fn behavioural_the_whole_scenario_of_section_11_3() {
    use own_window::{Bench, Product, Restore};

    const TYPED: &str = "ghbdtn";
    const EXPECTED: &str = "привет";

    let session = lang_switcher::layouts::enumerate().expect("the session has usable layouts");

    let us = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0409);
    let ru = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0419);

    let (Some(us), Some(ru)) = (us, ru) else {
        println!("SKIPPED: this session does not carry both US and Russian");
        return;
    };

    // The product first: it must have its hook up before anything is typed.
    let product = Product::start();
    println!("product started, pid {}", product.id());

    let Some(bench) = Bench::open() else {
        println!("SKIPPED: our own window did not reach the foreground; nothing was typed");
        return;
    };

    let started_on = switch::current();
    let _restore = Restore::to(started_on, bench.handle());

    // The scenario of §11.3 starts in the English layout.
    let outcome = switch::to(us);
    println!("window set to US: {outcome:?}");
    assert_eq!(
        switch::current(),
        us,
        "the scenario starts in the US layout"
    );

    // The product's own layout probe, so that it knows what this window is on. See `tap_shift`.
    own_window::tap_shift();

    // ---- набрать `ghbdtn` ----------------------------------------------------------------
    own_window::type_ascii(TYPED);
    let before = bench.wait_for_text(|text| text == TYPED);
    println!("after typing: {before:?}");
    assert_eq!(
        before.as_deref(),
        Some(TYPED),
        "the window has to contain what was typed before the hotkey means anything"
    );

    // ---- нажать горячую клавишу ------------------------------------------------------------
    own_window::press_hotkey();

    let after = bench.wait_for_text(|text| text == EXPECTED);
    let layout = switch::current();

    println!("point 23: text {after:?}, layout {layout}");

    assert_eq!(after.as_deref(), Some(EXPECTED), "point 23: the text half");
    assert_eq!(
        layout, ru,
        "point 23: the layout half — FR-50, the point of this task"
    );

    // ---- put everything back ---------------------------------------------------------------
    let outcome = switch::to(started_on);
    println!("restoring {started_on} -> {outcome:?}");
    assert_eq!(switch::current(), started_on);

    // `bench` is deliberately not dropped by hand: it is declared before `_restore`, so it
    // outlives it, and the net under a panic still has a window to check the foreground against.
    product.stop();
}

/// The real [`switch::System`], with the waits counted — how the measurement behind
/// [`VERIFY_BUDGET_MS`] is taken.
#[derive(Default)]
struct Timed {
    inner: switch::System,
    waits: u32,
    waited_ms: u32,
}

impl Machine for Timed {
    fn current(&mut self) -> LayoutId {
        self.inner.current()
    }

    fn post_request(&mut self, target: LayoutId) -> bool {
        self.inner.post_request(target)
    }

    fn activate(&mut self, target: LayoutId, scope: Scope) -> bool {
        self.inner.activate(target, scope)
    }

    fn hand_over(&mut self, target: LayoutId, scope: Scope) -> bool {
        self.inner.hand_over(target, scope)
    }

    fn wait(&mut self, ms: u32) -> u32 {
        let waited = self.inner.wait(ms);
        self.waits += 1;
        self.waited_ms += waited;
        waited
    }
}

/// A window this test owns, and the restoration of the layout it found.
///
/// Kept in a module of its own so that the Win32 it needs does not leak into the tests above,
/// none of which touch the machine at all.
mod own_window {
    use core::ffi::c_void;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use lang_switcher::layouts::LayoutId;
    use lang_switcher::switch;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        ACTIVATE_KEYBOARD_LAYOUT_FLAGS, ActivateKeyboardLayout, HKL, INPUT, INPUT_0,
        INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC,
        MapVirtualKeyW, SendInput, SetFocus, VIRTUAL_KEY, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, CreateWindowExW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
        GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, SW_SHOW, SendMessageW,
        SetForegroundWindow, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_GETTEXT, WS_BORDER, WS_POPUP, WS_VISIBLE,
    };
    use windows::core::{PCWSTR, w};

    /// How long the test thread gives the bench thread to win the foreground.
    const OPEN_TIMEOUT: Duration = Duration::from_secs(8);

    /// How long the bench waits for the window to show the text it expects.
    const TEXT_TIMEOUT: Duration = Duration::from_secs(3);

    /// `dwExtraInfo` of every keystroke this bench injects.
    ///
    /// Requirement 2 of section 11.5: it must **differ** from `hook::INJECTED_SIGNATURE`, or the
    /// program filters this input out as its own (FR-03) and the scenario tests nothing.
    const BENCH_SIGNATURE: usize = 0x0000_0000_5431_3035;

    /// How long the FR-51 probe waits for a layout change before answering "it did not reach".
    const PROBE_TIMEOUT: Duration = Duration::from_millis(200);

    /// Published instead of a window handle when the bench thread could not get the foreground.
    const FAILED: usize = usize::MAX;

    /// A window of this test's own **on a thread of its own, with a real message loop**.
    ///
    /// That is what makes the run resemble the thing the program actually switches: a posted
    /// `WM_INPUTLANGCHANGEREQUEST` only takes effect when the owning thread pumps, and
    /// `AttachThreadInput` in method 2 is only a real attach when the two threads differ.
    pub struct Bench {
        handle: HWND,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Bench {
        /// Starts the bench thread and returns once its window is the foreground one.
        ///
        /// `None` when it never gets there — decision R-42: the caller records that and switches
        /// nothing.
        pub fn open() -> Option<Self> {
            let stop = Arc::new(AtomicBool::new(false));
            let published = Arc::new(AtomicUsize::new(0));

            let (mine, theirs) = (Arc::clone(&stop), Arc::clone(&published));

            let thread = std::thread::spawn(move || {
                let Some(window) = TestWindow::open_foreground() else {
                    theirs.store(FAILED, Ordering::Release);
                    return;
                };

                theirs.store(window.handle().0 as usize, Ordering::Release);

                // The message loop. Without it a posted message is never processed and method 1
                // of FR-50 could not work here whatever the program did.
                while !mine.load(Ordering::Acquire) {
                    pump();
                    std::thread::sleep(Duration::from_millis(1));
                }

                // Destroyed on the thread that created it, as `DestroyWindow` requires.
                drop(window);
            });

            let started = Instant::now();

            while started.elapsed() < OPEN_TIMEOUT {
                match published.load(Ordering::Acquire) {
                    0 => std::thread::sleep(Duration::from_millis(20)),
                    FAILED => break,
                    raw => {
                        return Some(Self {
                            handle: HWND(raw as *mut c_void),
                            stop,
                            thread: Some(thread),
                        });
                    }
                }
            }

            stop.store(true, Ordering::Release);
            let _ = thread.join();
            None
        }

        /// The window the bench thread owns.
        pub fn handle(&self) -> HWND {
            self.handle
        }

        /// Polls the window's text until `wanted` accepts it, or until [`TEXT_TIMEOUT`] passes.
        ///
        /// Answers the last text read either way, so a failing assertion can print what really
        /// arrived instead of nothing.
        pub fn wait_for_text(&self, wanted: impl Fn(&str) -> bool) -> Option<String> {
            let started = Instant::now();
            let mut last = None;

            while started.elapsed() < TEXT_TIMEOUT {
                last = read_text(self.handle);

                if last.as_deref().is_some_and(&wanted) {
                    return last;
                }

                std::thread::sleep(Duration::from_millis(10));
            }

            last
        }
    }

    /// Reads the text of an `EDIT` control through `WM_GETTEXT`.
    fn read_text(window: HWND) -> Option<String> {
        let mut buffer = [0u16; 256];

        // SAFETY: `WM_GETTEXT` takes the buffer length in `wparam` and a pointer to that many
        // UTF-16 units in `lparam`. `buffer` is exactly that long, lives for the whole call and
        // is the only memory the receiver may write. `SendMessageW` is synchronous and the
        // window belongs to this process, so the pointer never crosses a process boundary.
        let written = unsafe {
            SendMessageW(
                window,
                WM_GETTEXT,
                Some(WPARAM(buffer.len())),
                Some(LPARAM(buffer.as_mut_ptr() as isize)),
            )
        };

        let written = usize::try_from(written.0).ok()?.min(buffer.len());

        Some(String::from_utf16_lossy(&buffer[..written]))
    }

    /// Types `text` as real keystrokes — one `SendInput` per character, down and up.
    ///
    /// ⚠ **`dwExtraInfo` is deliberately not the program's own signature.** Requirement 2 of
    /// section 11.5 and FR-03: a packet carrying `hook::INJECTED_SIGNATURE` is filtered out by
    /// the program as its own, and this input has to look like a person typing.
    pub fn type_ascii(text: &str) {
        for character in text.chars() {
            let virtual_key = character.to_ascii_uppercase() as u16;

            send_key(virtual_key, false);
            send_key(virtual_key, true);
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    /// Presses and releases the hotkey of FR-02 — `Pause`, the default of section 7.
    pub fn press_hotkey() {
        send_key(lang_switcher::hook::DEFAULT_HOTKEY_VK, false);
        std::thread::sleep(Duration::from_millis(8));
        send_key(lang_switcher::hook::DEFAULT_HOTKEY_VK, true);
    }

    /// Taps `Shift`, which is the product's own layout probe of task T-03-3c.
    ///
    /// ⚠ **Needed, and the reason is worth stating.** The product learns the layout of the
    /// foreground window from `EVENT_SYSTEM_FOREGROUND`, `EVENT_OBJECT_FOCUS` and the release of
    /// a modifier key. This run creates its window and *then* sets it to US, which is none of
    /// those three, so without this tap the product would still believe the window is on the
    /// layout it had when it first saw it — and would convert the strokes in the wrong
    /// direction. A person typing `ghbdtn` in English has already given the product one of those
    /// events; a test has to give it one on purpose.
    pub fn tap_shift() {
        send_key(VK_SHIFT.0, false);
        std::thread::sleep(Duration::from_millis(8));
        send_key(VK_SHIFT.0, true);
        std::thread::sleep(Duration::from_millis(150));
    }

    /// One key event through `SendInput`, with a signature the program will not mistake for its
    /// own.
    ///
    /// ⚠ The scan code is filled in from the virtual key with `MapVirtualKeyW`, and that is not
    /// decoration: FR-04 has the product record the **scan code** of a press and FR-06 decode it
    /// with `ToUnicodeEx`. An event with no scan code is an event with nothing to record.
    fn send_key(virtual_key: u16, up: bool) {
        let flags = if up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };

        // SAFETY: takes two integers, returns one, dereferences nothing.
        let scan = unsafe { MapVirtualKeyW(u32::from(virtual_key), MAPVK_VK_TO_VSC) } as u16;

        let event = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(virtual_key),
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: BENCH_SIGNATURE,
                },
            },
        };

        // SAFETY: one fully initialised `INPUT` in a slice this frame owns, with the size of the
        // structure passed as the OS requires. `SendInput` reads the slice and writes nothing
        // back through it. The return value is examined below (NFR-13).
        let sent = unsafe { SendInput(&[event], core::mem::size_of::<INPUT>() as i32) };

        assert_eq!(sent, 1, "SendInput refused a keystroke of the scenario");
    }

    /// The product, started for the run and stopped at the end of it.
    ///
    /// ⚠ Decision R-42: this is the **only** process this test may end, and it ends only this
    /// one, by the handle `Command::spawn` answered with.
    pub struct Product {
        child: std::process::Child,
    }

    impl Product {
        /// Starts the product with the deadline of FR-97 and waits for its hook to be up.
        pub fn start() -> Self {
            let child = std::process::Command::new(env!("CARGO_BIN_EXE_LangSwitcher"))
                // FR-97. Never started without it: the program carries a live global hook.
                .env("LANGSW_DEBUG_TIMEOUT_SEC", "45")
                .spawn()
                .expect("the product binary is built beside this test");

            // NFR-08 gives the hook fifty milliseconds from start-up; the layout sweep of FR-20
            // follows it and this is a debug build, so the wait is generous rather than tight.
            std::thread::sleep(Duration::from_millis(2_500));

            Self { child }
        }

        /// The process id, for the record in the report.
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        /// Ends the process this test started, and no other.
        pub fn stop(mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    impl Drop for Bench {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);

            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    /// **The FR-51 measurement.** Activates `target` on *this* thread without attaching to the
    /// foreground window's thread, and answers whether the foreground window followed.
    ///
    /// This is the claim the `Scope::Session` branch of method 2 rests on, put to the machine
    /// instead of being assumed. The layout is put back before the function returns whatever the
    /// answer is.
    pub fn activate_without_attach_reaches(target: LayoutId) -> bool {
        let before = switch::current();

        if before == target {
            // Nothing to observe: pick a probe target that differs, or say so.
            return false;
        }

        // SAFETY: the `HKL` is rebuilt from a numeric handle `layouts::enumerate` answered with;
        // constructing a pointer is safe and this one is never dereferenced. The call acts on
        // the calling thread's input queue and writes nothing through a pointer of ours.
        let activated = unsafe {
            ActivateKeyboardLayout(
                HKL(target.raw() as *mut c_void),
                ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0),
            )
        };

        let started = Instant::now();
        let mut reached = false;

        while started.elapsed() < PROBE_TIMEOUT {
            if switch::current() == target {
                reached = true;
                break;
            }

            std::thread::sleep(Duration::from_millis(2));
        }

        println!(
            "FR-51 probe: ActivateKeyboardLayout returned {:?}",
            activated.map(|previous| format!("0x{:08X}", previous.0 as usize))
        );

        // Put back whatever this probe moved, on this thread and through the chain, before the
        // caller's own restoration runs.
        let _ = switch::to(before);

        reached
    }

    /// `ES_MULTILINE | ES_AUTOVSCROLL` — the styles of a plain text box.
    const EDIT_STYLES: u32 = 0x0004 | 0x0040;

    /// How many attempts the window gets at the foreground, at 50 ms each — about five seconds.
    const FOREGROUND_ATTEMPTS: u32 = 100;

    /// A window created by this test, destroyed when the value is dropped.
    pub struct TestWindow {
        handle: HWND,
    }

    impl TestWindow {
        /// The handle, for callers that need to prove the foreground is still ours.
        pub fn handle(&self) -> HWND {
            self.handle
        }

        /// Creates the window and waits for it to become the foreground one.
        ///
        /// `None` when it never does — decision R-42: the caller records that and switches
        /// nothing, rather than reaching for a window it did not create.
        pub fn open_foreground() -> Option<Self> {
            // SAFETY: `EDIT` is a system window class present in every process; a null window
            // name asks for empty content, and no parent, menu, instance or creation parameter
            // is passed, which is the documented way to ask for a plain top-level window.
            let handle = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("EDIT"),
                    PCWSTR::null(),
                    WINDOW_STYLE(EDIT_STYLES) | WS_POPUP | WS_VISIBLE | WS_BORDER,
                    200,
                    200,
                    420,
                    120,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("the EDIT class is registered in every process");

            let window = Self { handle };

            // SAFETY: `handle` is the live window this frame owns; both calls take it by value
            // and touch no memory of ours. The `BOOL`s are not fatal — the loop below decides.
            unsafe {
                let _ = ShowWindow(handle, SW_SHOW);
                let _ = SetFocus(Some(handle));
            }

            for _ in 0..FOREGROUND_ATTEMPTS {
                ask_for_foreground(handle);
                pump();

                // SAFETY: takes no arguments, returns a handle by value.
                if unsafe { GetForegroundWindow() } == handle {
                    return Some(window);
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            None
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
            // destroyed exactly once — this type is neither `Copy` nor `Clone`.
            let _ = unsafe { DestroyWindow(self.handle) };
        }
    }

    /// **Point 25.** Puts the layout back, however the test ends — a panic included.
    pub struct Restore {
        layout: LayoutId,
        window: HWND,
    }

    impl Restore {
        /// Remembers `layout` as the one the machine was on before the test touched it.
        pub fn to(layout: LayoutId, window: HWND) -> Self {
            Self { layout, window }
        }
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            // Decision R-42 once more, at the one moment it is easiest to forget: restoring is
            // still switching, so it only happens while our own window is the foreground one.
            // SAFETY: takes no arguments, returns a handle by value.
            if unsafe { GetForegroundWindow() } != self.window {
                println!(
                    "point 25: our window is no longer in front; the layout was NOT put back \
                     by this test, and no window of anybody else's was touched"
                );
                return;
            }

            let outcome = switch::to(self.layout);
            println!(
                "point 25: restoring {} -> {outcome:?}, now {}",
                self.layout,
                switch::current()
            );
        }
    }

    /// Asks the system to put our own window in front, the way `tests\inject.rs` established.
    fn ask_for_foreground(handle: HWND) {
        // SAFETY: takes no arguments and returns a handle by value.
        let foreground = unsafe { GetForegroundWindow() };

        // SAFETY: a null `foreground` is answered with zero, which is examined below; `None`
        // asks for the thread id alone.
        let owner = unsafe { GetWindowThreadProcessId(foreground, None) };
        // SAFETY: takes no arguments, returns this thread's id.
        let ours = unsafe { GetCurrentThreadId() };

        let attached = owner != 0 && owner != ours;

        if attached {
            // SAFETY: both ids name live threads. A failed attach is not fatal — the call below
            // is then simply the unprivileged attempt, and the caller re-reads the fact.
            unsafe {
                let _ = AttachThreadInput(ours, owner, true);
            }
        }

        // SAFETY: `handle` is the live window the caller owns; both calls take it by value.
        unsafe {
            let _ = BringWindowToTop(handle);
            let _ = SetForegroundWindow(handle);
        }

        if attached {
            // SAFETY: undoes exactly the attachment above, with the same two ids.
            unsafe {
                let _ = AttachThreadInput(ours, owner, false);
            }
        }
    }

    /// Runs this thread's message queue dry.
    fn pump() {
        let mut message = MSG::default();

        loop {
            // SAFETY: `message` is a live, aligned `MSG` owned by this frame for the whole call
            // and is the only buffer written to. `None` asks for every message of this thread.
            let taken = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) };

            if !taken.as_bool() {
                return;
            }

            // SAFETY: `message` was just filled by `PeekMessageW` and is passed on unchanged.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
