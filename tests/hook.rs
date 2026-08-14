//! Integration tests for task T-03-1 — the low-level keyboard hook.
//!
//! # Why almost everything here is a pure-function test
//!
//! The decision the callback takes is the requirement; the `SetWindowsHookExW` around it is
//! plumbing. Module `hook` is split along exactly that line: [`lang_switcher::hook::classify`]
//! is a pure function of a `Mode`, a `HotkeyState` and a `KeyEvent`, so FR-03, FR-08 and
//! FR-95 can be driven through every combination that matters, deterministically, in
//! microseconds and with no keyboard in the loop.
//!
//! **No test in this file installs a real hook, and that is deliberate.** A `WH_KEYBOARD_LL`
//! hook installed by a process that is not pumping messages does not merely fail to work: the
//! system waits `LowLevelHooksTimeout` on every stroke before giving up on it, so a test
//! binary that installed one would freeze the keyboard of whoever is running `cargo test` for
//! seconds at a time. Section 4.11 of SPEC exists to prevent precisely that, and a test suite
//! is the last place to make an exception. Installation, removal and the emergency
//! combination are verified against the running program instead — points 9 to 16 of the
//! acceptance criterion — which is the only place they can be verified honestly anyway.

use lang_switcher::hook::{
    self, Decision, Edge, HotkeyState, INJECTED_SIGNATURE, KeyEvent, MAX_CONSECUTIVE_PANICS, Mode,
    Outcome,
};

// -------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------

/// Virtual-key code of `Pause`, the default hotkey of section 7.
const VK_PAUSE: u16 = 0x13;

/// Virtual-key code of `A` — a stroke that is not the hotkey.
const VK_A: u16 = 0x41;

/// `WM_KEYDOWN`, `WM_KEYUP`, `WM_SYSKEYDOWN`, `WM_SYSKEYUP`.
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;

/// A signature some other program might set. Anything that is not ours.
const FOREIGN_SIGNATURE: usize = 0x00CA_FE01;

/// The ordinary running state: armed, healthy, `Pause` as the hotkey.
fn armed() -> Mode {
    Mode {
        active: true,
        fail_safe: false,
        hotkey_vk: VK_PAUSE,
    }
}

/// The outcome every "not ours" path has to produce: hand it on, start nothing.
fn passed_on() -> Outcome {
    Outcome {
        decision: Decision::Pass,
        fire_hotkey: false,
    }
}

/// A stroke of the user's own hand.
fn user_key(vk: u16, edge: Edge) -> KeyEvent {
    KeyEvent {
        vk,
        edge,
        extra_info: FOREIGN_SIGNATURE,
    }
}

// -------------------------------------------------------------------------------------
// FR-02, FR-95 — the hotkey is recognised in the callback and always suppressed
// -------------------------------------------------------------------------------------

#[test]
fn pressing_the_hotkey_suppresses_it_and_starts_a_conversion() {
    let mut state = HotkeyState::default();

    let outcome = hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));

    // FR-95: suppressed. FR-02: recognised, and the conversion is asked for.
    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(outcome.fire_hotkey);
}

#[test]
fn releasing_the_hotkey_is_suppressed_too_and_starts_nothing() {
    let mut state = HotkeyState::default();

    hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    let outcome = hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Up));

    // A press that was swallowed and a release that was not would leave the application
    // holding a key it never saw go down.
    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(!outcome.fire_hotkey);
}

#[test]
fn an_ordinary_stroke_is_passed_on_untouched() {
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Up] {
        let outcome = hook::classify(armed(), &mut state, user_key(VK_A, edge));

        assert_eq!(outcome, passed_on(), "an ordinary stroke, {edge:?}");
    }
}

#[test]
fn the_hotkey_is_whatever_was_published_not_whatever_pause_is() {
    let mut state = HotkeyState::default();

    let mode = Mode {
        hotkey_vk: VK_A,
        ..armed()
    };

    // `A` is now the hotkey and is suppressed...
    let on_a = hook::classify(mode, &mut state, user_key(VK_A, Edge::Down));
    assert_eq!(on_a.decision, Decision::Suppress);
    assert!(on_a.fire_hotkey);

    // ...and `Pause` is an ordinary key that goes through.
    let on_pause = hook::classify(mode, &mut state, user_key(VK_PAUSE, Edge::Down));
    assert_eq!(on_pause, passed_on());
}

// -------------------------------------------------------------------------------------
// FR-08 — auto-repeat produces one conversion, not a stream
// -------------------------------------------------------------------------------------

#[test]
fn holding_the_hotkey_fires_once_however_long_it_is_held() {
    let mut state = HotkeyState::default();

    // Auto-repeat is what the system sends: a run of key-down events with no key-up in
    // between. Ten of them stand for a key held for a second.
    let fired = (0..10)
        .filter(|_| {
            let outcome = hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));

            // FR-95 does not weaken for repeats: every one of them is still suppressed.
            assert_eq!(outcome.decision, Decision::Suppress);

            outcome.fire_hotkey
        })
        .count();

    assert_eq!(fired, 1, "FR-08: one conversion, not a stream of them");
}

#[test]
fn the_hotkey_fires_again_after_it_has_been_released() {
    let mut state = HotkeyState::default();

    let mut fired = 0;

    for _ in 0..3 {
        // Press, hold a little, release: three separate presses by the user.
        for _ in 0..4 {
            if hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down)).fire_hotkey {
                fired += 1;
            }
        }

        hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Up));
    }

    assert_eq!(fired, 3, "FR-08 suppresses repeats, not presses");
}

#[test]
fn a_system_key_message_is_the_same_key() {
    // `Alt` held turns `WM_KEYDOWN` into `WM_SYSKEYDOWN`. A hook that knew only the first
    // pair would stop recognising its own hotkey the moment `Alt` was down — and would stop
    // recognising the emergency combination of FR-96, which always has `Alt` in it.
    assert_eq!(hook::edge_of(WM_KEYDOWN), Some(Edge::Down));
    assert_eq!(hook::edge_of(WM_SYSKEYDOWN), Some(Edge::Down));
    assert_eq!(hook::edge_of(WM_KEYUP), Some(Edge::Up));
    assert_eq!(hook::edge_of(WM_SYSKEYUP), Some(Edge::Up));

    // Anything else is not a key transition and must not be read as one.
    assert_eq!(hook::edge_of(0), None);
    assert_eq!(hook::edge_of(0x0200), None);
}

// -------------------------------------------------------------------------------------
// FR-03 — our own injected input, by signature and never by LLKHF_INJECTED
// -------------------------------------------------------------------------------------

#[test]
fn our_own_injected_stroke_is_filtered_out_and_still_delivered() {
    let mut state = HotkeyState::default();

    let ours = KeyEvent {
        vk: VK_PAUSE,
        edge: Edge::Down,
        extra_info: INJECTED_SIGNATURE,
    };

    let outcome = hook::classify(armed(), &mut state, ours);

    // Passed on, not suppressed: it was sent *to* the application on purpose, and swallowing
    // it would mean replacing the user's text with nothing.
    assert_eq!(outcome.decision, Decision::Pass);
    // And it starts nothing: our own replacement must not look like the user asking for
    // another one.
    assert!(!outcome.fire_hotkey);
    // The FR-08 state is untouched by our own input.
    assert_eq!(state, HotkeyState::default());
}

#[test]
fn a_foreign_injected_stroke_is_ordinary_user_input() {
    // The on-screen keyboard, an accessibility tool, a password manager filling a field: all
    // of them carry `LLKHF_INJECTED`, and none of them carries our signature. FR-03 forbids
    // filtering on the flag precisely so that these keep working.
    for extra_info in [0, 1, FOREIGN_SIGNATURE, usize::MAX, INJECTED_SIGNATURE ^ 1] {
        let mut state_here = HotkeyState::default();

        let outcome = hook::classify(
            armed(),
            &mut state_here,
            KeyEvent {
                vk: VK_PAUSE,
                edge: Edge::Down,
                extra_info,
            },
        );

        assert_eq!(
            outcome.decision,
            Decision::Suppress,
            "dwExtraInfo {extra_info:#x} is not ours and must be treated as the user"
        );
        assert!(outcome.fire_hotkey, "dwExtraInfo {extra_info:#x}");
    }
}

#[test]
fn the_signature_is_not_a_value_anybody_sets_by_accident() {
    // Third parties that use `dwExtraInfo` at all put small things in it: a window handle, a
    // device index, a zero. A collision is not impossible, but it must not be *likely*, and
    // the value must be recognisable in a debugger when it happens.
    assert_ne!(INJECTED_SIGNATURE, 0);
    assert_ne!(
        INJECTED_SIGNATURE >> 32,
        0,
        "the signature must not fit in the low 32 bits, where small values live"
    );
}

// -------------------------------------------------------------------------------------
// FR-90, FR-95, FR-99 — the two states in which nothing is suppressed
// -------------------------------------------------------------------------------------

#[test]
fn a_suspended_program_suppresses_nothing_at_all() {
    let mut state = HotkeyState::default();

    let suspended = Mode {
        active: false,
        ..armed()
    };

    // FR-95 makes the hotkey vanish only "когда программа активна". A suspended program has
    // to give `Pause` back to whatever else wants it.
    for edge in [Edge::Down, Edge::Up] {
        let outcome = hook::classify(suspended, &mut state, user_key(VK_PAUSE, edge));
        assert_eq!(outcome, passed_on(), "suspended, {edge:?}");
    }
}

#[test]
fn the_fail_safe_of_fr99_passes_everything_through() {
    let mut state = HotkeyState::default();

    let tripped = Mode {
        fail_safe: true,
        ..armed()
    };

    // FR-99: "весь ввод пропускается без обработки". Including the hotkey, and including
    // input carrying our own signature.
    for extra_info in [FOREIGN_SIGNATURE, INJECTED_SIGNATURE] {
        for vk in [VK_PAUSE, VK_A] {
            for edge in [Edge::Down, Edge::Up] {
                let outcome = hook::classify(
                    tripped,
                    &mut state,
                    KeyEvent {
                        vk,
                        edge,
                        extra_info,
                    },
                );

                assert_eq!(outcome, passed_on(), "fail-safe, vk {vk:#x}, {edge:?}");
            }
        }
    }
}

#[test]
fn a_hotkey_held_across_a_state_change_does_not_stay_stuck_down() {
    let mut state = HotkeyState::default();

    // Held down while armed...
    hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    assert!(state.hotkey_down);

    // ...the program is suspended while the key is still down...
    let suspended = Mode {
        active: false,
        ..armed()
    };
    hook::classify(suspended, &mut state, user_key(VK_PAUSE, Edge::Down));
    assert!(!state.hotkey_down, "the remembered state must not go stale");

    // ...and when it comes back, the next press is a press and not a repeat.
    let outcome = hook::classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    assert!(outcome.fire_hotkey);
}

// -------------------------------------------------------------------------------------
// FR-96 — the emergency combination
// -------------------------------------------------------------------------------------

#[test]
fn the_emergency_key_is_f12_going_down_and_nothing_else() {
    const VK_F12: u16 = 0x7B;
    const VK_F11: u16 = 0x7A;

    // With `Alt` held — which the combination always has — the message is `WM_SYSKEYDOWN`,
    // so both spellings of "going down" have to be recognised.
    assert!(hook::is_emergency_key(VK_F12, WM_KEYDOWN));
    assert!(hook::is_emergency_key(VK_F12, WM_SYSKEYDOWN));

    // The release is not the trigger: the process is already gone by then.
    assert!(!hook::is_emergency_key(VK_F12, WM_KEYUP));
    assert!(!hook::is_emergency_key(VK_F12, WM_SYSKEYUP));

    // Neighbours must not do it.
    assert!(!hook::is_emergency_key(VK_F11, WM_KEYDOWN));
    assert!(!hook::is_emergency_key(VK_PAUSE, WM_KEYDOWN));
}

#[test]
fn the_emergency_key_is_the_f12_of_the_constant() {
    assert_eq!(hook::EMERGENCY_VK, 0x7B);
}

// -------------------------------------------------------------------------------------
// Section 7 — the name of the hotkey
// -------------------------------------------------------------------------------------

#[test]
fn the_default_of_section_seven_is_understood() {
    assert_eq!(hook::vk_from_name("Pause"), Some(hook::DEFAULT_HOTKEY_VK));
    assert_eq!(hook::DEFAULT_HOTKEY_VK, VK_PAUSE);
}

#[test]
fn key_names_are_matched_case_insensitively_and_trimmed() {
    for spelling in ["Pause", "pause", "PAUSE", "  Pause  ", "\tpause\n", "Break"] {
        assert_eq!(
            hook::vk_from_name(spelling),
            Some(VK_PAUSE),
            "spelling {spelling:?}"
        );
    }
}

#[test]
fn function_keys_are_understood_across_the_whole_range() {
    assert_eq!(hook::vk_from_name("F1"), Some(0x70));
    assert_eq!(hook::vk_from_name("f12"), Some(0x7B));
    assert_eq!(hook::vk_from_name("F24"), Some(0x87));

    // There is no `F0` and no `F25`, and neither falls back to anything.
    assert_eq!(hook::vk_from_name("F0"), None);
    assert_eq!(hook::vk_from_name("F25"), None);

    // A bare `F` is not a malformed function key: it is the letter key `F`, whose
    // virtual-key code is its own ASCII value. The single-character rule is tried before the
    // `Fn` rule for exactly this reason, and reversing the two would cost the user a key.
    assert_eq!(hook::vk_from_name("F"), Some(0x46));
}

#[test]
fn a_single_letter_or_digit_is_its_own_virtual_key_code() {
    // A documented property of the virtual-key space: `VK_A` is 0x41 and `VK_0` is 0x30.
    assert_eq!(hook::vk_from_name("A"), Some(0x41));
    assert_eq!(hook::vk_from_name("a"), Some(0x41));
    assert_eq!(hook::vk_from_name("Z"), Some(0x5A));
    assert_eq!(hook::vk_from_name("0"), Some(0x30));
    assert_eq!(hook::vk_from_name("9"), Some(0x39));
}

#[test]
fn an_unknown_name_is_none_so_that_the_default_survives_a_typo() {
    for name in [
        "",
        "   ",
        "Paws",
        "Ctrl+Q",
        "F1F1",
        "..",
        "\u{041F}\u{0430}\u{0443}\u{0437}\u{0430}",
    ] {
        assert_eq!(hook::vk_from_name(name), None, "name {name:?}");
    }
}

// -------------------------------------------------------------------------------------
// The published surface the other tasks depend on
// -------------------------------------------------------------------------------------

#[test]
fn the_two_messages_collide_with_nothing_else_in_the_process() {
    const WM_APP: u32 = 0x8000;

    // `WM_APP + 1` is `app`'s wake-up and `WM_APP + 2` is the tray callback.
    assert_eq!(hook::WM_APP_HOTKEY, WM_APP + 3);
    assert_eq!(hook::WM_APP_FAIL_SAFE, WM_APP + 4);
    assert_ne!(hook::WM_APP_HOTKEY, hook::WM_APP_FAIL_SAFE);
}

#[test]
fn fr99_tolerates_three_panics_and_trips_on_the_fourth() {
    // FR-99 says "более трёх раз подряд" — strictly more than three.
    assert_eq!(MAX_CONSECUTIVE_PANICS, 3);
}

#[test]
fn removing_a_hook_that_is_not_there_is_safe_and_says_so() {
    // The idempotence every exit path of FR-96, FR-97, FR-98, FR-83 and `Installed::drop`
    // relies on: they all call `uninstall` without coordinating, and exactly one of them can
    // ever be the one that removed something.
    assert!(!hook::is_installed());
    assert!(!hook::uninstall());
    assert!(!hook::uninstall());
    assert!(!hook::is_installed());
}

#[test]
fn the_program_starts_armed_on_the_default_hotkey() {
    // What the callback matches against before the UI thread has published anything — the
    // window NFR-08 exists to keep short.
    let mode = hook::current_mode();

    assert!(mode.active, "a resident utility starts armed");
    assert!(!mode.fail_safe);
    assert_eq!(mode.hotkey_vk, hook::DEFAULT_HOTKEY_VK);
    assert_eq!(hook::consecutive_panics(), 0);
    assert_eq!(hook::hotkey_handoffs(), 0);
    assert_eq!(hook::post_failures(), 0);
    assert_eq!(hook::unhook_failures(), (0, 0));
    assert!(!hook::emergency_terminate_failed());
    assert!(!hook::panic_is_absorbed());
}
