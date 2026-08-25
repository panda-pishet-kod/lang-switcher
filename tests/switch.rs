//! Module `switch` — the one method of FR-50, the scope of FR-51, the reading of FR-52 with its
//! addendum, and the refusal of an IME target from FR-35. Tasks **T-05-1** and **Т-14-4**.
//!
//! # What is checked here and what is not
//!
//! The requirement is a **rule about failure**: FR-50 counts as failed when the layout did not
//! become the target, and never because the call that attempted it returned an error — decision
//! R-32. That rule is invisible from outside code that obeys it. On a machine where the posted
//! message is honoured, a correct implementation and one that trusts `PostMessage` behave
//! identically, every single time.
//!
//! So the switch is driven through [`lang_switcher::switch::Machine`], the seam the module
//! provides, and the case that tells the two apart is built explicitly: a machine whose
//! `post_request` answers `true` and whose layout never moves. See
//! [`a_post_that_reports_success_without_changing_the_layout_is_still_a_failure`].
//!
//! # ⭐ What task Т-14-4 changed in this file
//!
//! FR-50 used to prescribe a chain of three methods, and roughly a third of this file existed to
//! pin the order of that chain: method 2 after method 1, method 3 after method 2, the handover to
//! the watcher thread, its single-use channel, the scope reaching the two methods that had one.
//! The user struck methods 2 and 3 out of the requirement on 2026-08-25 (question 63 of
//! `DECISIONS.md`) on the measurements of T-13-10 and Т-14-2 — across 504 executions the two
//! fallbacks gave a verdict not once, and method 2 hung the calling thread without limit against
//! a window that had stopped pumping. Those tests are gone: a test that goes on asserting a
//! requirement that no longer exists is not a check, it is a second copy of the defect.
//!
//! What arrived in their place is the addendum of FR-52, which the same measurement produced: in
//! a classic console window `GetGUIThreadInfo` refuses 120 times out of 120, so no verdict can be
//! taken there at all, and the switch counts as **sent without confirmation**. Three tests drive
//! that case through the seam, and a fourth reads `src\switch.rs` itself and asserts that not one
//! of the calls the retired methods were built out of is left in its code.
//!
//! The behavioural half — a real switch of a real window — is at the bottom of this file, marked
//! `#[ignore]`. It is not part of `cargo test` on purpose: it changes the keyboard layout of the
//! machine a person is sitting at, and it is run deliberately, with
//! `cargo test --test switch -- --ignored --nocapture`. Everything it touches is a window this
//! test created itself, and the layout it found is put back however it ends (decision R-42).

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use lang_switcher::layouts::LayoutId;
use lang_switcher::switch::{
    self, Failures, FocusThread, Machine, Outcome, Reading, Scope, SwitchError, VERIFY_BUDGET_MS,
    VERIFY_POLL_MS,
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

/// What the fake machine's `PostMessage` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    /// What the Win32 call answered. `true` is "the call succeeded", which decision R-32 says is
    /// **not** the same statement as "the layout changed".
    returned: bool,
    /// Whether the layout actually became the target.
    switched: bool,
}

impl Report {
    /// The trap of this task: the call reports success and nothing happens. This is what every
    /// application that ignores `WM_INPUTLANGCHANGEREQUEST` looks like from here.
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

    /// R-32 with the sign reversed: the call reported failure and the effect happened anyway.
    const WORKS_ANYWAY: Self = Self {
        returned: false,
        switched: true,
    };
}

/// A machine whose every answer the test writes.
///
/// It records what was asked of it, which is how "no wait was performed" is checked: not waiting
/// is not observable from outside a function that does not wait.
#[derive(Clone, Debug)]
struct Fake {
    /// What [`Machine::read`] answers as the layout.
    layout: LayoutId,
    /// **FR-52's addendum.** What [`Machine::read`] answers for [`Reading::blind`] — `true` is a
    /// classic console window, where `GetGUIThreadInfo` refuses and nothing readable is a verdict.
    blind: bool,
    /// What the one method of FR-50 does.
    method: Report,
    /// What [`Machine::wait`] answers it waited, in milliseconds.
    slice_ms: u32,

    // ---- what the switch did ----
    posts: u32,
    reads: u32,
    waits: u32,
    waited_ms: u32,
    targets: Vec<LayoutId>,
}

impl Fake {
    /// A machine sitting on `layout`, whose judge can see, and on which the method lies (see
    /// [`Report::LIES`]).
    fn on(layout: LayoutId) -> Self {
        Self {
            layout,
            blind: false,
            method: Report::LIES,
            slice_ms: VERIFY_POLL_MS,
            posts: 0,
            reads: 0,
            waits: 0,
            waited_ms: 0,
            targets: Vec::new(),
        }
    }

    /// Makes the method behave as `report` says.
    fn method(mut self, report: Report) -> Self {
        self.method = report;
        self
    }

    /// **FR-52's addendum.** Makes the judge blind — `GetGUIThreadInfo` refused, which is what a
    /// `ConsoleWindowClass` window does every time.
    fn blind(mut self) -> Self {
        self.blind = true;
        self
    }
}

impl Machine for Fake {
    fn read(&mut self) -> Reading {
        self.reads += 1;

        Reading {
            layout: self.layout,
            blind: self.blind,
        }
    }

    fn post_request(&mut self, target: LayoutId) -> bool {
        self.posts += 1;
        self.targets.push(target);

        if self.method.switched {
            self.layout = target;
        }

        self.method.returned
    }

    fn wait(&mut self, ms: u32) -> u32 {
        assert_eq!(
            ms, VERIFY_POLL_MS,
            "the wait must be asked for one slice at a time, so that a switch that took effect \
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
/// The call answers success and the layout does not move. `PostMessage` returns success when the
/// message is **queued**, and an application is free to ignore it, so this is not a contrived
/// case — it is what happens in every application that does not handle
/// `WM_INPUTLANGCHANGEREQUEST`, and Т-14-2 measured it against a window that had stopped pumping
/// messages, 60 times out of 60.
///
/// An implementation that read return values would report a switch. This one must re-read the
/// layout by FR-52, see that nothing moved, and say so.
#[test]
fn a_post_that_reports_success_without_changing_the_layout_is_still_a_failure() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, RU);

    // Not `Ok(Outcome::Switched)`. That is the whole assertion.
    assert_eq!(outcome, Err(SwitchError::NotSwitched));

    assert_eq!(fake.posts, 1, "the message went out exactly once");

    let counted = switch::failures();
    assert_eq!(counted.post_message, 1, "the failure is counted");
    assert_eq!(
        counted.post_rejected, 0,
        "the call itself reported success — the verdict came from re-reading, not from it"
    );
    assert_eq!(
        counted.sent_unconfirmed, 0,
        "the judge could see; this is a failure and not an unconfirmed send"
    );
}

/// The mirror image, so that the test above cannot pass by an implementation that always fails.
#[test]
fn a_post_that_really_switches_is_a_verified_switch() {
    let _guard = counters();

    let mut fake = Fake::on(US).method(Report::WORKS);
    let outcome = switch::to_in(&mut fake, RU);

    assert_eq!(outcome, Ok(Outcome::Switched));
    assert_eq!(fake.posts, 1);
    assert_eq!(fake.layout, RU);
    assert_eq!(switch::failures(), Failures::default());
}

/// NFR-13 against R-32, in both directions.
///
/// A call that reports failure is **counted**; and the verdict still comes from re-reading, so a
/// call whose return value said "no" while the effect happened is a success all the same.
#[test]
fn a_call_that_reports_failure_is_counted_and_is_still_not_the_verdict() {
    let _guard = counters();

    let mut fake = Fake::on(US).method(Report::WORKS_ANYWAY);
    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Switched));

    let counted = switch::failures();
    assert_eq!(
        counted.post_rejected, 1,
        "NFR-13: the return value was examined and its failure counted"
    );
    assert_eq!(
        counted.post_message, 0,
        "R-32: and it was not the verdict — the layout did become the target"
    );

    // The other direction: the call failed and so did the effect.
    switch::reset_failures();

    let mut fake = Fake::on(US).method(Report::REFUSED);
    assert_eq!(switch::to_in(&mut fake, RU), Err(SwitchError::NotSwitched));

    let counted = switch::failures();
    assert_eq!(counted.post_rejected, 1);
    assert_eq!(counted.post_message, 1);
}

/// The message carries the target the caller named, and no other layout.
#[test]
fn the_message_carries_the_target_the_caller_named() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let _ = switch::to_in(&mut fake, RU);

    assert_eq!(fake.targets, vec![RU]);
}

// ---------------------------------------------------------------------------------------
// ⭐ FR-52's addendum — the classic console window, task Т-14-4
// ---------------------------------------------------------------------------------------

/// **FR-52's addendum, question 63.** When `GetGUIThreadInfo` refuses for the window in front,
/// the switch is **sent and not waited for**.
///
/// # What the requirement says and what it costs
///
/// «Вердикт по FR-52 в таких окнах невыносим: переключение считается отправленным без
/// подтверждения, ожидания подтверждения не выполняются.» Т-14-2 measured why: in a
/// `ConsoleWindowClass` window `GetWindowThreadProcessId` answers with a counterfeit thread id,
/// `GetGUIThreadInfo` refuses on it 120 times out of 120, and the fallback then reads
/// `0x00000000` — also 120 of 120 — while the switch has in fact happened, 60 of 60. Waiting
/// there re-reads the same blind zero until the budget runs out; with the old chain of three that
/// was about 95 ms of the input thread on **every press**.
///
/// So: one post, **zero waits**, and an outcome that is neither a success nor a failure.
#[test]
fn a_blind_reading_sends_the_switch_and_waits_for_nothing() {
    let _guard = counters();

    // A console window as this module sees it: the judge refuses, and the number it hands back
    // is the zero the fallback read off a counterfeit thread id.
    let mut fake = Fake::on(LayoutId::default()).blind();
    let outcome = switch::to_in(&mut fake, RU);

    assert_eq!(outcome, Ok(Outcome::Sent));
    assert_eq!(fake.posts, 1, "the switch was sent");
    assert_eq!(fake.targets, vec![RU]);
    assert_eq!(
        fake.waits, 0,
        "«ожидания подтверждения не выполняются» — not one slice of the budget"
    );
    assert_eq!(fake.waited_ms, 0);
    assert_eq!(
        fake.reads, 1,
        "one reading, the one that found the judge blind; re-reading it would answer the same \
         zero for ever"
    );
}

/// The blind case has a counter of its own, and it is not one of the failure counters.
///
/// ⚠ The point of a separate counter is that «отправлено без подтверждения» must never be
/// readable as either "confirmed" or "failed". A number in [`Failures::sent_unconfirmed`] is the
/// count of switches this program sent and cannot vouch for — Т-14-2 measured that they do in
/// fact happen, 60 times out of 60 — and it is the only place that fact is visible.
#[test]
fn a_blind_reading_is_counted_apart_from_every_failure() {
    let _guard = counters();

    let mut fake = Fake::on(LayoutId::default()).blind();
    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Sent));

    let counted = switch::failures();

    assert_eq!(counted.sent_unconfirmed, 1);
    assert_eq!(
        counted.no_foreground, 0,
        "a blind zero is not «there is no foreground window» — the two are the same number and \
         different facts"
    );
    assert_eq!(counted.post_message, 0, "and it is not a failed switch");
    assert_eq!(
        counted,
        Failures {
            sent_unconfirmed: 1,
            ..Failures::default()
        },
        "nothing else moved"
    );

    // And it is not a confirmation either: the stamp of FR-04 must not follow it.
    assert!(
        !switch::confirmed(Ok(Outcome::Sent)),
        "«отправлено» is not «подтверждено»"
    );
}

/// A blind reading is decided **before** the two branches that would read its number, and both
/// of those branches would be wrong.
///
/// * `layout == 0` would answer [`SwitchError::NoForeground`] — a refusal that sends nothing, in
///   a window where the switch works every time.
/// * `layout == target` would answer [`Outcome::AlreadyActive`] — silently swallowing a switch
///   the user asked for, on the strength of a number that is not a statement about that window.
#[test]
fn a_blind_reading_is_never_mistaken_for_no_foreground_or_for_already_active() {
    let _guard = counters();

    // The zero of a console window.
    let mut fake = Fake::on(LayoutId::default()).blind();
    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Sent));
    assert_eq!(fake.posts, 1, "a refusal would have sent nothing");

    // A blind reading that happens to carry the target. It is still not a statement about the
    // window in front, so the switch goes out.
    switch::reset_failures();

    let mut fake = Fake::on(RU).blind();
    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Sent));
    assert_eq!(
        fake.posts, 1,
        "an untrustworthy reading must not silence a switch"
    );
    assert_eq!(switch::failures().sent_unconfirmed, 1);
}

/// NFR-13 holds on the blind path too: the return value of the post is examined and counted.
#[test]
fn the_post_of_a_blind_switch_still_examines_its_return_value() {
    let _guard = counters();

    let mut fake = Fake::on(LayoutId::default())
        .blind()
        .method(Report::REFUSED);

    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Sent));

    let counted = switch::failures();
    assert_eq!(counted.post_rejected, 1, "NFR-13 does not lapse here");
    assert_eq!(
        counted.sent_unconfirmed, 1,
        "and the outcome is still what FR-52's addendum says it is"
    );
}

/// A reading that goes blind **in the middle** of the wait is not a confirmation.
///
/// The foreground window changed under the switch, to one no verdict can be taken about. FR-52's
/// addendum is about the window that was in front when the switch was made; this is a different
/// event, and the honest answer to it is "not settled" rather than a success invented out of a
/// number nobody may believe.
#[test]
fn a_reading_that_goes_blind_during_the_wait_is_not_a_confirmation() {
    let _guard = counters();

    /// A machine that answers the target from the first re-read — but blind.
    struct GoesBlind {
        reads: u32,
        waits: u32,
    }

    impl Machine for GoesBlind {
        fn read(&mut self) -> Reading {
            self.reads += 1;

            Reading {
                layout: if self.reads == 1 { US } else { RU },
                blind: self.reads > 1,
            }
        }

        fn post_request(&mut self, _target: LayoutId) -> bool {
            true
        }

        fn wait(&mut self, ms: u32) -> u32 {
            self.waits += 1;
            ms
        }
    }

    let mut machine = GoesBlind { reads: 0, waits: 0 };

    assert_eq!(
        switch::to_in(&mut machine, RU),
        Err(SwitchError::NotSwitched),
        "a blind reading equal to the target is still not a verdict"
    );
    assert_eq!(
        machine.waits, VERIFY_BUDGET_MS,
        "and the budget was spent rather than cut short by a value nobody may believe"
    );
    assert_eq!(switch::failures().sent_unconfirmed, 0);
}

// ---------------------------------------------------------------------------------------
// Point 13 — the wait is bounded above
// ---------------------------------------------------------------------------------------

/// **Point 13, section 6.1 and FR-80.** The wait between the attempt and its check has a
/// ceiling, and the ceiling is on the time really waited.
///
/// The input thread holds the low-level hook, and a thread that stops being available for
/// longer than `LowLevelHooksTimeout` — about 5000 ms — loses the hook silently.
///
/// ⭐ **Task Т-14-4 divided this ceiling by two.** While FR-50 was a chain the worst case was two
/// budgets on the input thread, and in a console window three (about 95 ms per press, measured by
/// Т-14-2) — plus, on a window that had stopped pumping, an `ActivateKeyboardLayout` under
/// `AttachThreadInput` that did not return at all, 60 times out of 60. One method means one
/// budget, and it is asserted here.
#[test]
fn the_wait_between_the_attempt_and_its_check_is_bounded() {
    let _guard = counters();

    // Nothing ever settles, which is the worst case: the whole budget is spent.
    let mut fake = Fake::on(US);
    let _ = switch::to_in(&mut fake, RU);

    let ceiling = VERIFY_BUDGET_MS + VERIFY_POLL_MS;
    assert!(
        fake.waited_ms <= ceiling,
        "the switch waited {} ms, ceiling {ceiling} ms",
        fake.waited_ms
    );

    // And the ceiling itself is far below the timeout that costs the program its hook.
    assert!(
        u64::from(ceiling) * 200 < 5000,
        "the whole switch must stay at least two orders of magnitude inside LowLevelHooksTimeout"
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

    let _ = switch::to_in(&mut fake, RU);

    assert_eq!(
        fake.waits, 1,
        "one slice already spent the whole budget, so there is no second one"
    );
}

/// A switch that takes effect early does not spend the rest of the budget.
#[test]
fn a_switch_that_takes_effect_at_once_costs_one_slice() {
    let _guard = counters();

    let mut fake = Fake::on(US).method(Report::WORKS);
    let _ = switch::to_in(&mut fake, RU);

    assert_eq!(fake.waits, 1);
    assert_eq!(fake.waited_ms, VERIFY_POLL_MS);
}

// ---------------------------------------------------------------------------------------
// Point 16 — FR-51, the scope
// ---------------------------------------------------------------------------------------

/// **FR-51.** The Windows setting is on by default, so the default of the type is the scope that
/// setting means.
#[test]
fn the_default_scope_is_the_windows_default() {
    assert_eq!(Scope::default(), Scope::PerWindow);
}

/// **Point 16, FR-51 on the real machine.** The setting is read from the OS and answers one of
/// the two scopes, without a failure being swallowed.
///
/// ⚠ Since task Т-14-4 nothing takes the scope as an argument: the two methods whose behaviour
/// depended on it are gone, and the one method FR-50 prescribes has no scope-dependent form. What
/// FR-51 asks for is that the setting be **taken into account**, and reading it, counting a
/// failure to read it and reporting it through the diagnostics is what that now amounts to. This
/// test is the check that the reading still works and still counts.
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
/// for one has a defect. Nothing must be attempted: composition in an IME breaks the
/// correspondence between keystrokes and the field's contents, which is what FR-35 is about.
#[test]
fn an_ime_target_is_refused_and_counted_and_nothing_is_attempted() {
    let _guard = counters();

    assert!(PINYIN.is_ime(), "the fixture really is an IME handle");

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, PINYIN);

    assert_eq!(outcome, Err(SwitchError::ImeTarget));
    assert_eq!(fake.posts, 0, "nothing may be sent for an IME target");
    assert_eq!(fake.reads, 0, "not even the layout is read");
    assert_eq!(switch::failures().ime_target, 1);
}

/// The zero handle names no layout and is refused for the same reason.
#[test]
fn the_zero_handle_is_refused_and_counted() {
    let _guard = counters();

    let mut fake = Fake::on(US);
    let outcome = switch::to_in(&mut fake, LayoutId::default());

    assert_eq!(outcome, Err(SwitchError::NoTarget));
    assert_eq!(fake.posts, 0);
    assert_eq!(fake.reads, 0);
    assert_eq!(switch::failures().no_target, 1);
}

/// No foreground window — the secure desktop, or a desktop switch in progress — is a refusal and
/// not an attempt to switch a window that is not there.
///
/// ⚠ The judge is **not blind** here, and that is what tells this case from the console one: the
/// zero is FR-52's own answer about a desktop with nothing in front of it.
#[test]
fn no_foreground_window_is_refused_and_counted() {
    let _guard = counters();

    let mut fake = Fake::on(LayoutId::default());
    let outcome = switch::to_in(&mut fake, RU);

    assert_eq!(outcome, Err(SwitchError::NoForeground));
    assert_eq!(fake.posts, 0);
    assert_eq!(switch::failures().no_foreground, 1);
    assert_eq!(switch::failures().sent_unconfirmed, 0);
}

// ---------------------------------------------------------------------------------------
// Task T-10-5 — which outcomes a caller may take the target as fact from
// ---------------------------------------------------------------------------------------

/// **`confirmed` is true for exactly the two outcomes decision R-32 has verified, and for
/// nothing else.**
///
/// # What rests on it
///
/// Step 5 of FR-40 is the one place in the program that changes the keyboard layout without the
/// user touching anything, and until task T-10-5 nothing told the typing buffer about it: the
/// stamp of FR-04 kept the previous layout, every stroke of the next word was recorded under it,
/// and FR-26 then converted into the layout the text was already typed in — the acceptance
/// session's «первое нажатие моргает». `inject::System::switch_layout` now publishes the target
/// as the new stamp, and this function is the gate it does that through.
///
/// So both halves of the assertion carry weight, and the `false` half carries more. Publishing
/// after a refusal would tell the buffer the window moved when it did not, which is the same
/// defect with the sign reversed and would be *harder* to see: it would strike only on machines
/// where FR-50 fails.
///
/// The cases are enumerated over the whole of both enums rather than sampled, so that an outcome
/// added later cannot quietly default to either answer.
#[test]
fn only_a_verified_switch_is_confirmed() {
    // The two R-32 has re-read FR-52 for: "the message went out and the layout became the
    // target", and "it was the target before anything was sent".
    for outcome in [Ok(Outcome::Switched), Ok(Outcome::AlreadyActive)] {
        assert!(
            switch::confirmed(outcome),
            "{outcome:?} is a verified switch and the stamp may follow it"
        );
    }

    // ⭐ FR-52's addendum. The switch was sent to a window no verdict can be taken about, and
    // Т-14-2 measured that in such a window it really happens (60 of 60) — but this program
    // cannot see it happen, 120 times out of 120, and "almost certainly" is not "verified".
    assert!(
        !switch::confirmed(Ok(Outcome::Sent)),
        "an unconfirmable send is not a confirmation, however likely it is to have worked"
    );

    // Every refusal: nothing was sent, or nothing took, and the window kept its layout.
    for error in [
        SwitchError::ImeTarget,
        SwitchError::NoTarget,
        SwitchError::NoForeground,
        SwitchError::NotSwitched,
    ] {
        assert!(
            !switch::confirmed(Err(error)),
            "{error:?} moved no layout and the stamp must not move either"
        );
    }
}

/// The gate answers over the real path, not only over hand-built values.
///
/// The five cases below are driven through `to_in` against the seam, so what is asserted is what
/// step 5 of FR-40 will actually see: a machine that switches, a machine already on the target, a
/// machine nothing works on, a console window, and a refusal. A test built only from literals
/// would still pass if the module stopped producing one of them.
#[test]
fn the_gate_answers_over_the_real_path() {
    let _guard = counters();

    // The post works: the layout really became the target.
    let mut fake = Fake::on(US).method(Report::WORKS);
    assert!(switch::confirmed(switch::to_in(&mut fake, RU)));

    // Already there — nothing is sent at all.
    let mut fake = Fake::on(RU);
    assert!(switch::confirmed(switch::to_in(&mut fake, RU)));

    // The post lies and the layout never moves.
    let mut fake = Fake::on(US);
    assert!(!switch::confirmed(switch::to_in(&mut fake, RU)));

    // ⭐ A console window: sent, and unconfirmable.
    let mut fake = Fake::on(LayoutId::default()).blind();
    assert_eq!(switch::to_in(&mut fake, RU), Ok(Outcome::Sent));

    let mut fake = Fake::on(LayoutId::default()).blind();
    assert!(!switch::confirmed(switch::to_in(&mut fake, RU)));

    // A refusal before anything is attempted — FR-35.
    let mut fake = Fake::on(US);
    assert!(!switch::confirmed(switch::to_in(&mut fake, PINYIN)));
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
    let outcome = switch::to_in(&mut fake, RU);

    assert_eq!(outcome, Ok(Outcome::AlreadyActive));
    assert_eq!(fake.posts, 0, "no redundant WM_INPUTLANGCHANGEREQUEST");
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
    let _ = switch::to_in(&mut fake, PINYIN);
    let _ = switch::to_in(&mut fake, LayoutId::default());
    let _ = switch::to_in(&mut Fake::on(LayoutId::default()), RU);

    // The post refused and the layout did not move.
    let _ = switch::to_in(&mut Fake::on(US).method(Report::REFUSED), RU);

    // FR-52's addendum: a console window.
    let _ = switch::to_in(&mut Fake::on(LayoutId::default()).blind(), RU);

    let counted = switch::failures();
    assert_eq!(counted.ime_target, 1);
    assert_eq!(counted.no_target, 1);
    assert_eq!(counted.no_foreground, 1);
    assert_eq!(counted.post_message, 1);
    assert_eq!(counted.post_rejected, 1);
    assert_eq!(counted.sent_unconfirmed, 1);
}

// ---------------------------------------------------------------------------------------
// Point 20 — SEC-01 and SEC-07
// ---------------------------------------------------------------------------------------

/// **Point 20, SEC-01 and SEC-07.** Nothing that could carry a keystroke exists in the counters
/// or in the error values, and their rendered forms are fixed strings and digits.
///
/// The counters are nine `u32`s and the errors are four fieldless variants, so there is nowhere
/// for a character, a scan code or a piece of the typing buffer to be. This test pins that shape:
/// a field or a variant that started carrying data would break it.
#[test]
fn nothing_that_could_carry_a_keystroke_reaches_the_counters_or_the_errors() {
    let _guard = counters();

    for error in [
        SwitchError::ImeTarget,
        SwitchError::NoTarget,
        SwitchError::NoForeground,
        SwitchError::NotSwitched,
    ] {
        let rendered = format!("{error:?}");
        assert!(
            rendered.chars().all(|ch| ch.is_ascii_alphabetic()),
            "an error value must be a bare name: {rendered}"
        );
    }

    for outcome in [Outcome::AlreadyActive, Outcome::Switched, Outcome::Sent] {
        let rendered = format!("{outcome:?}");
        assert!(
            rendered.chars().all(|ch| ch.is_ascii_alphabetic()),
            "an outcome must be a bare name: {rendered}"
        );
    }

    // The counters after a run that failed every way it could: names, digits and punctuation of
    // the derived `Debug`, and no value that came from a keystroke, because there is none to
    // come from.
    let _ = switch::to_in(&mut Fake::on(US), RU);
    let rendered = format!("{:?}", switch::failures());

    assert!(
        rendered
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || " ,:{}_".contains(ch)),
        "the counters render as names and numbers only: {rendered}"
    );
}

// ---------------------------------------------------------------------------------------
// ⭐ Task Т-14-4 — the retired methods are gone from the code, not merely unreachable
// ---------------------------------------------------------------------------------------

/// **The abolition of methods 2 and 3, as a test rather than as a grep somebody ran once.**
///
/// The old FR-50 was a chain, and its two fallbacks were built out of exactly five names. Т-14-2
/// measured what they were worth: a verdict **not once in 504 executions** across two tasks, and
/// — for `ActivateKeyboardLayout` under `AttachThreadInput` — an unbounded block of the calling
/// thread against a window that had stopped pumping, 60 times out of 60. In this program that
/// caller is the input thread with the low-level hook of section 6.1, so the block is the whole
/// keyboard of the session hanging until Windows removes the hook (FR-80), silently.
///
/// The check is therefore stronger than "the module does not call them": **not one of these names
/// occurs in the code of `src\switch.rs` at all**. Prose is deliberately exempt — the module
/// documentation names them to say why they are gone, and a history that cannot name what it
/// buried is not a history.
#[test]
fn no_call_of_the_retired_methods_is_left_in_the_code_of_the_module() {
    /// The five names methods 2 and 3 were built out of, plus the two COM calls TSF needed.
    const RETIRED: [&str; 7] = [
        "AttachThreadInput",
        "ActivateKeyboardLayout",
        "DetachThreadInput",
        "ITfInputProcessorProfileMgr",
        "ActivateProfile",
        "CoInitialize",
        "CoCreateInstance",
    ];

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("switch.rs");

    let source = std::fs::read_to_string(&path).expect("src\\switch.rs is readable");

    for name in RETIRED {
        let hits: Vec<usize> = source
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let line = line.trim_start();

                !line.starts_with("//") && !line.starts_with("///") && !line.starts_with("//!")
            })
            .filter(|(_, line)| line.contains(name))
            .map(|(index, _)| index + 1)
            .collect();

        assert!(
            hits.is_empty(),
            "{name} occurs in the code of src\\switch.rs at lines {hits:?}; methods 2 and 3 of \
             FR-50 were struck out of the requirement by question 63 and this module may not \
             call them"
        );
    }
}

/// The handover of decision R-31 is gone, and the number it travelled on is **kept reserved**.
///
/// `WM_APP + 8` was the message with which the input thread gave method 3 to the watcher thread.
/// Method 3 is gone, and so is everything that posted or answered that message — but the number
/// is an entry in a register: the map of `WM_APP + N` is written out in the documentation of
/// `hook`, `guard`, `watchdog` and `settings`, and four tests of this repository assert that no
/// two private messages of this process share a number. Freeing it for reuse would make those
/// documents wrong in the one way that fails silently.
#[test]
fn the_retired_message_number_is_still_reserved_and_still_unique() {
    assert_eq!(switch::WM_APP_SWITCH, 0x8000 + 8);

    for occupied in [
        lang_switcher::hook::WM_APP_HOTKEY,
        lang_switcher::hook::WM_APP_FAIL_SAFE,
        lang_switcher::hook::WM_APP_SEED_CAPS,
        lang_switcher::watchdog::WM_APP_FLUSH,
        lang_switcher::watchdog::WM_APP_LAYOUT,
        lang_switcher::watchdog::WM_APP_REHOOK,
        lang_switcher::guard::WM_APP_PROBE,
        lang_switcher::guard::WM_APP_FIELD,
        lang_switcher::selection::WM_APP_SELECTION,
        lang_switcher::selection::WM_APP_BUFFER_PATH,
    ] {
        assert_ne!(
            switch::WM_APP_SWITCH,
            occupied,
            "the retired number must not be handed to a live message"
        );
    }
}

// ---------------------------------------------------------------------------------------
// Point 9 — FR-52
// ---------------------------------------------------------------------------------------

/// **Point 9, FR-52.** The layout is read of the thread that owns the window with the **keyboard
/// focus**.
///
/// ⚠ **This test was rewritten by task T-10-20, and it is the only existing test that task
/// touched.** It was not adjusted to make new code pass — it was a transcription of FR-52's old
/// text, `GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), null))`, and the user
/// replaced that text in `SPEC.md` (commit `13bd084`) on the measurement of task T-10-19: the
/// layout is a property **of a thread**, and in a packaged application of Windows 11 the
/// foreground window and the window that receives the input are different threads of one process.
/// A test that goes on asserting a requirement that no longer exists is not a check, it is a
/// second copy of the defect.
///
/// The test computes the requirement's new expression itself, from the same four functions, and
/// compares. It cannot prove the module contains those calls — that is a reading of the source,
/// and it is in the report — but it pins the *answer*. On this machine, while `cargo test` runs,
/// the foreground window is a classic one whose two threads coincide, so this case alone would
/// **not** tell the old reading from the new one; that is what
/// [`fr_52_falls_back_to_the_foreground_thread_exactly_where_the_requirement_says_to`] and the
/// bench detector of task T-10-20 are for.
#[test]
fn fr_52_reads_the_layout_of_the_thread_that_owns_the_focus_window() {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{
        GUITHREADINFO, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
    };

    // SAFETY: the calls of FR-52, each taking values and returning a value. `None` asks
    // `GetWindowThreadProcessId` for the thread id alone, and `info` is a live, aligned
    // `GUITHREADINFO` of this frame whose `cbSize` describes it — the contract `GetGUIThreadInfo`
    // takes before it writes. A null foreground window yields thread zero, handled below.
    let expected = unsafe {
        let foreground = GetForegroundWindow();

        if foreground.is_invalid() {
            LayoutId::default()
        } else {
            let front = GetWindowThreadProcessId(foreground, None);

            if front == 0 {
                LayoutId::default()
            } else {
                let mut info = GUITHREADINFO {
                    cbSize: u32::try_from(size_of::<GUITHREADINFO>()).unwrap_or(0),
                    ..Default::default()
                };

                // The requirement's own fallback: a refusal or an empty `hwndFocus` means the
                // foreground thread, which is the behaviour that was there before.
                let thread = if GetGUIThreadInfo(front, &raw mut info).is_err()
                    || info.hwndFocus.is_invalid()
                {
                    front
                } else {
                    match GetWindowThreadProcessId(info.hwndFocus, None) {
                        0 => front,
                        focus => focus,
                    }
                };

                LayoutId::from_raw(GetKeyboardLayout(thread).0 as usize)
            }
        }
    };

    assert_eq!(switch::current(), expected);
}

/// **Task Т-14-4.** `current` is [`switch::read`] with the addendum's flag dropped, and nothing
/// else.
///
/// That equality is what keeps every caller outside module `switch` — `app::refresh_layout_and_
/// cache`, `buffer::Recorder::restamp`, the selection path of FR-60/FR-61 — untouched by the
/// addendum: they ask what the layout is, and they get the answer FR-52 has always given them.
/// The extra fact travels beside it and only `to_in` reads it.
#[test]
fn current_is_the_reading_of_fr_52_without_its_verdict_flag() {
    let reading = switch::read();

    assert_eq!(switch::current(), reading.layout);

    // On the machine a `cargo test` runs on, the foreground window is a classic one and the
    // judge can see it. Printed rather than asserted: a console window really would be blind
    // here, and this test must not fail because of where it was started from.
    println!("FR-52 on this machine right now: {reading:?}");
}

/// **FR-52, sentence 2 — the fallback, all three branches, driven rather than hoped for.**
///
/// ⭐ This test exists because of a number in the review of task T-10-19: over **660 circles and
/// three applications** `GetGUIThreadInfo` refused **0 times** and answered «no focus window» **0
/// times**. Neither fallback branch can be reached by running the program on an ordinary window,
/// so neither of them is known to work from any live measurement — and an untested fallback in a
/// requirement that has already been got wrong four times is exactly the place the fifth mistake
/// would live.
///
/// ⚠ One of the two has since been seen, and constantly: Т-14-2 measured the refusal **120 times
/// out of 120** in a classic console window, which is where FR-52's addendum came from.
///
/// [`switch::reading_thread`] is the rule with the Win32 taken out of it, so all three cases are
/// ordinary values here:
///
/// | `GetGUIThreadInfo` said | FR-52 says to read | asserted |
/// |---|---|---|
/// | a focus window on thread `F` | thread `F` | ✔ |
/// | answered, `hwndFocus` empty | the foreground thread | ✔ |
/// | refused | the foreground thread — *the previous behaviour* | ✔ |
#[test]
fn fr_52_falls_back_to_the_foreground_thread_exactly_where_the_requirement_says_to() {
    const FOREGROUND: u32 = 4444;
    const FOCUS: u32 = 7777;

    assert_eq!(
        switch::reading_thread(FOREGROUND, FocusThread::Thread(FOCUS)),
        FOCUS,
        "sentence 1: the thread of the window with the keyboard focus"
    );

    assert_eq!(
        switch::reading_thread(FOREGROUND, FocusThread::NoFocus),
        FOREGROUND,
        "sentence 2, «hwndFocus пуст»: the foreground thread, the previous behaviour"
    );

    assert_eq!(
        switch::reading_thread(FOREGROUND, FocusThread::Refused),
        FOREGROUND,
        "sentence 2, «GetGUIThreadInfo отказал»: the foreground thread, the previous behaviour"
    );
}

/// ⚠ **The two fallbacks are different facts and stay different facts.**
///
/// They have the same *effect* on which thread is read — the foreground one — and that is
/// precisely the shape in which an `Option<u32>` would have collapsed them into one. A thread
/// that has no focus window is a state of the machine; a call that refused is a failure of a
/// call, and NFR-13 is about the second.
///
/// ⭐ Since task Т-14-4 they differ in more than provenance: only the refusal makes a verdict
/// impossible (FR-52's addendum), so collapsing them would turn every thread with no focus window
/// into an unconfirmable send.
#[test]
fn the_two_fallbacks_of_fr_52_are_not_the_same_answer() {
    assert_ne!(FocusThread::NoFocus, FocusThread::Refused);
    assert_eq!(FocusThread::Thread(1), FocusThread::Thread(1));
    assert_ne!(FocusThread::Thread(1), FocusThread::Thread(2));
}

/// **NFR-13 for the two calls FR-52 gained** — a refusal is counted, not swallowed.
///
/// The counters are the only trace either fallback leaves, and the review of T-10-19 named this
/// as the thing to watch: a branch that has never once been seen in 660 circles is one whose
/// first occurrence must not be silent. This checks the pair starts at zero and is reported
/// through [`switch::failures`] like every other counter of the module.
#[test]
fn the_fallbacks_of_fr_52_have_counters_of_their_own() {
    let _guard = counters();

    let counted = switch::failures();

    assert_eq!(counted.focus_probe_refused, 0);
    assert_eq!(counted.focus_window_absent, 0);
    assert_eq!(
        counted,
        Failures::default(),
        "reset_failures covers the two new counters as well"
    );
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
///
/// # Why the window lives on a thread of its own
///
/// FR-50 is a **posted** message: it takes effect when the window that owns the queue runs its
/// own message loop. A window created and then left alone by the test thread never pumps, so the
/// switch could never work and the run would measure nothing at all. The bench therefore puts the
/// window on a thread that pumps, which is also what a real application is, and drives the switch
/// from the test thread.
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
    let outcome = switch::to_in(&mut timed, ru);
    println!(
        "point 21: US -> RU  outcome {outcome:?}, settled after {} ms in {} slices",
        timed.waited_ms, timed.waits
    );
    assert_eq!(outcome, Ok(Outcome::Switched), "{outcome:?}");
    assert_eq!(switch::current(), ru, "point 21: the layout is now Russian");

    // ---- point 22: RU -> US -------------------------------------------------------------
    let mut timed = Timed::default();
    let outcome = switch::to_in(&mut timed, us);
    println!(
        "point 22: RU -> US  outcome {outcome:?}, settled after {} ms in {} slices",
        timed.waited_ms, timed.waits
    );
    assert_eq!(outcome, Ok(Outcome::Switched), "{outcome:?}");
    assert_eq!(switch::current(), us, "point 22: the layout is back on US");

    // ---- point 24: asking again for what is already there --------------------------------
    let mut timed = Timed::default();
    let outcome = switch::to_in(&mut timed, us);
    println!(
        "point 24: US -> US  outcome {outcome:?}, {} waits",
        timed.waits
    );
    assert_eq!(outcome, Ok(Outcome::AlreadyActive));
    assert_eq!(timed.waits, 0, "point 24: no redundant switching");

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

/// **Task Т-14-4 on the real machine: a window that never pumps fails, and fails *fast*.**
///
/// A window that never runs a message loop is a window that never processes a posted message.
/// `PostMessage` still reports success — the message *was* queued — so this is exactly the case
/// decision R-32 was written about.
///
/// ⚠ **This test used to assert the opposite of what it asserts now, and the change is the point
/// of task Т-14-4.** It used to end on `Outcome::Switched(Method::AttachActivate)`, because the
/// window it creates belongs to the *test thread itself* and method 2's `ActivateKeyboardLayout`
/// then moved that very thread's layout — a "success" that says nothing about a foreign
/// application. Т-14-2 put a real foreign window with a dead queue under all three methods and
/// measured the truth: **0 of 60 for every one of them**, and method 2 blocking the calling
/// thread for longer than 2000 ms, 60 times out of 60. With the fallbacks struck out of FR-50 the
/// required answer here is a plain, bounded failure.
///
/// So two things are asserted: the outcome is [`SwitchError::NotSwitched`], and the whole call
/// returns inside a fraction of the `LowLevelHooksTimeout` that FR-80 is about — which is the
/// half method 2 could not promise.
#[test]
#[ignore = "creates a foreground window of its own; run deliberately with --ignored"]
fn behavioural_a_window_that_never_pumps_fails_fast_and_does_not_hang() {
    use own_window::{Restore, TestWindow};

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

    let began = std::time::Instant::now();
    let outcome = switch::to(target);
    let elapsed = began.elapsed();

    println!("a window that never pumps: {started_on} -> {target}, outcome {outcome:?}");
    println!("the whole call took {elapsed:?}");
    println!("counters: {:?}", switch::failures());

    assert_eq!(
        outcome,
        Err(SwitchError::NotSwitched),
        "nothing can switch a queue nobody drains, and there is nothing left to fall back to"
    );
    assert_eq!(
        switch::failures().post_message,
        1,
        "the failure is counted, from the re-read and not from the return value"
    );
    assert_eq!(
        switch::failures().post_rejected,
        0,
        "PostMessageW itself reported success — decision R-32 in one line"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "the call must be bounded by the budget of FR-80; it took {elapsed:?}"
    );
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
    fn read(&mut self) -> Reading {
        self.inner.read()
    }

    fn post_request(&mut self, target: LayoutId) -> bool {
        self.inner.post_request(target)
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
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, SetFocus, VIRTUAL_KEY, VK_SHIFT,
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

    /// Published instead of a window handle when the bench thread could not get the foreground.
    const FAILED: usize = usize::MAX;

    /// A window of this test's own **on a thread of its own, with a real message loop**.
    ///
    /// That is what makes the run resemble the thing the program actually switches: a posted
    /// `WM_INPUTLANGCHANGEREQUEST` only takes effect when the owning thread pumps.
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

                // The message loop. Without it a posted message is never processed and FR-50
                // could not work here whatever the program did.
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
