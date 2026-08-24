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

/// The outcome every "not ours" path has to produce: hand it on, start nothing, ask nothing.
fn passed_on() -> Outcome {
    Outcome {
        decision: Decision::Pass,
        fire_hotkey: false,
        probe_layout: false,
    }
}

/// A stroke of the user's own hand.
///
/// The physical half — `scan`, `flags`, `time` — is zero throughout this file, and that is
/// deliberate rather than lazy: task T-03-2a made those three fields of `KeyEvent`, and not
/// one of the decisions this file drives reads them. FR-99, FR-03, FR-90, FR-02, FR-08 and
/// FR-95 are decided on the virtual key, the edge and `dwExtraInfo` alone, so a test that gave
/// the three fields values would be asserting that they are ignored — which they are, and
/// which `tests\buffer.rs` shows from the side that does read them.
fn user_key(vk: u16, edge: Edge) -> KeyEvent {
    KeyEvent {
        vk,
        edge,
        extra_info: FOREIGN_SIGNATURE,
        scan: 0,
        flags: 0,
        time: 0,
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
        scan: 0,
        flags: 0,
        time: 0,
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
                scan: 0,
                flags: 0,
                time: 0,
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
                        scan: 0,
                        flags: 0,
                        time: 0,
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
// FR-21 — the layout probe of task T-03-3c
// -------------------------------------------------------------------------------------

/// The modifier keys, sided and neutral, and the two keys used as counter-examples.
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_MENU: u16 = 0x12;
const VK_CAPITAL: u16 = 0x14;
const VK_SPACE: u16 = 0x20;
const VK_LWIN: u16 = 0x5B;
const VK_RWIN: u16 = 0x5C;
const VK_LSHIFT: u16 = 0xA0;
const VK_RSHIFT: u16 = 0xA1;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;

/// Every modifier the probe answers to.
const PROBE_MODIFIERS: [u16; 11] = [
    VK_SHIFT,
    VK_LSHIFT,
    VK_RSHIFT,
    VK_CONTROL,
    VK_LCONTROL,
    VK_RCONTROL,
    VK_MENU,
    VK_LMENU,
    VK_RMENU,
    VK_LWIN,
    VK_RWIN,
];

/// **The measurement the whole of part 2 rests on: all three layout switchers end with a
/// modifier being released** — task T-03-3c, point 15, and rule Р-41.
///
/// Four mechanisms for closing the open limit of FR-21 have been rejected before this one, and
/// the fourth was rejected because a property was *asserted* of the code instead of being read
/// out of it: `Alt+Shift` and `Ctrl+Shift` were said to reach the command row of FR-10, and
/// they do not. The fifth mechanism rests on a different property — that all three switchers
/// **end with a modifier release** — and this test is that property measured rather than
/// assumed. If it were false for any one of the three, the mechanism would be as dead as the
/// four before it and this test would say so.
///
/// Each switcher is driven through [`hook::classify`] one event at a time, exactly as the
/// callback delivers them: a press per key going down and a release per key going up, in the
/// order the fingers make them. What is asserted of each is that its **last** event asks for a
/// probe, which is the claim; the exact number of probes per switch is asserted beside it,
/// because that number is what the mechanism costs and it should not be able to grow unnoticed.
#[test]
fn every_layout_switcher_of_fr11_ends_in_a_stroke_that_asks_for_the_probe() {
    /// One layout switcher as the hook sees it: its name, the events it is made of, and the
    /// number of probes it is expected to cost.
    type Switcher = (&'static str, &'static [(u16, Edge)], u32);

    let switchers: [Switcher; 3] = [
        (
            "Alt+Shift",
            &[
                (VK_LMENU, Edge::Down),
                (VK_LSHIFT, Edge::Down),
                (VK_LSHIFT, Edge::Up),
                (VK_LMENU, Edge::Up),
            ],
            2,
        ),
        (
            "Ctrl+Shift",
            &[
                (VK_LCONTROL, Edge::Down),
                (VK_LSHIFT, Edge::Down),
                (VK_LSHIFT, Edge::Up),
                (VK_LCONTROL, Edge::Up),
            ],
            2,
        ),
        (
            // The one switcher of the three with a key in it, which is why it is also the only
            // one that reaches the command row of FR-10 — `tests\buffer.rs` measures that half.
            "Win+Space",
            &[
                (VK_LWIN, Edge::Down),
                (VK_SPACE, Edge::Down),
                (VK_SPACE, Edge::Up),
                (VK_LWIN, Edge::Up),
            ],
            1,
        ),
    ];

    for (name, events, expected_probes) in switchers {
        let mut state = HotkeyState::default();
        let mut probes = 0;
        let mut last_asked = false;

        for &(vk, edge) in events {
            let outcome = hook::classify(armed(), &mut state, user_key(vk, edge));

            // None of the six keys involved is the hotkey, so every one of them is passed on
            // to the application untouched — a layout switch must still switch the layout.
            assert_eq!(
                outcome.decision,
                Decision::Pass,
                "{name}, {vk:#04x} {edge:?}"
            );
            assert!(!outcome.fire_hotkey, "{name}, {vk:#04x} {edge:?}");

            last_asked = outcome.probe_layout;

            if last_asked {
                probes += 1;
            }
        }

        assert!(
            last_asked,
            "{name} does not end in a stroke that asks for a probe — the mechanism of T-03-3c \
             does not cover it"
        );
        assert_eq!(probes, expected_probes, "{name} costs this many probes");
    }
}

#[test]
fn the_probe_is_asked_for_by_a_modifier_going_up_and_by_nothing_else() {
    for vk in PROBE_MODIFIERS {
        let mut state = HotkeyState::default();

        // The press must not ask. The system performs the switch after this callback has
        // returned, so a probe fired here would read the layout that is on its way out.
        assert!(
            !hook::classify(armed(), &mut state, user_key(vk, Edge::Down)).probe_layout,
            "the press of {vk:#04x} must not ask"
        );
        assert!(
            hook::classify(armed(), &mut state, user_key(vk, Edge::Up)).probe_layout,
            "the release of {vk:#04x} must ask"
        );
    }

    // Not modifiers, on either edge. `CapsLock` is in this list on purpose: module `buffer`
    // counts it a modifier, and the probe deliberately does not — it is a toggle, it switches
    // no layout, and every release of it would be a probe that can never find anything.
    for vk in [VK_A, VK_SPACE, VK_CAPITAL] {
        let mut state = HotkeyState::default();

        for edge in [Edge::Down, Edge::Up] {
            assert!(
                !hook::classify(armed(), &mut state, user_key(vk, edge)).probe_layout,
                "vk {vk:#04x}, {edge:?} is not a layout switch"
            );
        }
    }
}

#[test]
fn a_program_that_is_not_listening_asks_for_no_probe() {
    // FR-99. A fail-safe program looks at nothing at all, and that has to include this.
    let tripped = Mode {
        fail_safe: true,
        ..armed()
    };
    // FR-90. A suspended program records nothing, so it has no layout to keep up to date.
    let suspended = Mode {
        active: false,
        ..armed()
    };

    for mode in [tripped, suspended] {
        let mut state = HotkeyState::default();

        for vk in PROBE_MODIFIERS {
            assert!(
                !hook::classify(mode, &mut state, user_key(vk, Edge::Up)).probe_layout,
                "vk {vk:#04x}"
            );
        }
    }

    // FR-03. Our own injected input switches no layout, and it is filtered out before the
    // buffer for the same reason it must be filtered out before this.
    let mut state = HotkeyState::default();

    for vk in PROBE_MODIFIERS {
        let ours = KeyEvent {
            vk,
            edge: Edge::Up,
            extra_info: INJECTED_SIGNATURE,
            scan: 0,
            flags: 0,
            time: 0,
        };

        assert!(
            !hook::classify(armed(), &mut state, ours).probe_layout,
            "our own stroke, vk {vk:#04x}"
        );
    }

    // And the hotkey branch asks for nothing either, even in the one configuration where the
    // hotkey of section 7 would be a modifier: FR-95 suppresses the stroke, so the application
    // never sees it and no layout can have changed.
    let hotkey_is_a_modifier = Mode {
        hotkey_vk: VK_LSHIFT,
        ..armed()
    };
    let mut state = HotkeyState::default();

    let outcome = hook::classify(
        hotkey_is_a_modifier,
        &mut state,
        user_key(VK_LSHIFT, Edge::Up),
    );

    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(!outcome.probe_layout);
}

/// **What the probe costs on an ordinary phrase, as a number** — NFR-10, and point 18 of the
/// task, which asks for a figure rather than a word.
///
/// A probe is fired by *every* modifier release, and the release of `Shift` after a capital
/// letter is one. That is the design and not an oversight — the probe asks a question, it does
/// not rebuild anything, and `app::layout_refresh_needed` answers "unchanged" and returns
/// before the sweep of FR-20 is even considered — but the frequency has to be known rather
/// than guessed at, so it is counted here out of the production rule itself.
///
/// The sample is one ordinary Russian sentence, typed on ЙЦУКЕН:
/// "Привет! Как твои дела? Всё хорошо, спасибо." — 43 characters, six of which need `Shift`:
/// the three capitals `П`, `К`, `В`, the `!` and the `?`, and the comma, which is the shifted
/// half of the key whose unshifted half is the full stop. Each shifted character is typed the
/// way a typist types it — `Shift` down, the key, `Shift` up — and each unshifted one on its
/// own.
///
/// The answer the assertions below fix: **six probes for 43 characters, that is 14 probes per
/// 100 characters typed**, and 98 key events in total, so slightly under one probe per sixteen
/// events. The cost of one is a single `PostMessageW` on the callback path and three cheap
/// reads on the input thread.
#[test]
fn typing_an_ordinary_phrase_costs_fourteen_probes_per_hundred_characters() {
    /// Length of the sample sentence, in characters.
    const CHARACTERS: usize = 43;
    /// Zero-based positions of the characters that need `Shift`.
    const SHIFTED: [usize; 6] = [0, 6, 8, 21, 23, 33];

    let mut state = HotkeyState::default();
    let mut probes = 0_usize;
    let mut events = 0_usize;

    for position in 0..CHARACTERS {
        let shifted = SHIFTED.contains(&position);

        let mut stroke = |vk, edge| {
            events += 1;

            if hook::classify(armed(), &mut state, user_key(vk, edge)).probe_layout {
                probes += 1;
            }
        };

        if shifted {
            stroke(VK_LSHIFT, Edge::Down);
        }

        stroke(VK_A, Edge::Down);
        stroke(VK_A, Edge::Up);

        if shifted {
            stroke(VK_LSHIFT, Edge::Up);
        }
    }

    assert_eq!(events, 98, "43 characters and six of them shifted");
    assert_eq!(probes, SHIFTED.len(), "one probe per shifted character");
    assert_eq!(
        probes * 100 / CHARACTERS,
        13,
        "13.95 probes per hundred characters, integer division"
    );
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

// -------------------------------------------------------------------------------------
// Task T-08-3 — FR-96 is decided first, and it is not subject to FR-03
// -------------------------------------------------------------------------------------

/// **FR-96 is answered before any other logic of the callback** — the text of the requirement.
///
/// # Why this test reads the source
///
/// The property is an *ordering* inside `keyboard_hook_proc`, and that function is `unsafe
/// extern "system"`, is called by the system with a pointer only the system can produce, and
/// ends in `TerminateProcess`. There is no way to call it from a test and no way to observe the
/// order from its return value. What can be checked, and checked honestly, is the shape of the
/// function itself — which is also what a reviewer checks, and this puts a failing test under
/// the reviewer's judgement instead of leaving it to memory.
///
/// # What is asserted
///
/// Inside the body of `keyboard_hook_proc`, the emergency test stands before:
///
/// * `edge_of(message)` — the first thing that can decide the stroke is none of ours;
/// * the construction of the `KeyEvent`, which is where `dwExtraInfo` is first read;
/// * `guarded_decision`, which is `catch_unwind`, `classify`, FR-03, FR-90 and the hotkey.
///
/// Only two things may precede it, and both are asserted to be exactly what they are: the
/// `code != HC_ACTION` test, which is the system's contract and the precondition of the
/// dereference, and the dereference itself, which is what makes there be a key to speak of.
///
/// It also asserts there is **no `cfg` on the emergency block**. §4.11 calls FR-96 a safeguard
/// for the user rather than a debugging aid, and the decision on question 18 makes its presence
/// in both build configurations the condition of running the program at all.
#[test]
fn fr_96_is_decided_before_any_other_logic_of_the_callback() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable");

    let body = source
        .split_once("unsafe extern \"system\" fn keyboard_hook_proc")
        .expect("the callback must be in this file")
        .1;

    let at = |needle: &str| -> usize {
        body.find(needle)
            .unwrap_or_else(|| panic!("the callback no longer contains {needle:?}"))
    };

    let emergency = at("if is_emergency_key(vk, message) && emergency_modifiers_held()");
    let hc_action = at("if code != HC_ACTION as i32");
    let deref = at("let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) }");
    let edge = at("let Some(edge) = edge_of(message)");
    let key_event = at("let key = KeyEvent {");
    let decision = at("let outcome = guarded_decision(key)");

    println!(
        "offsets in the callback: HC_ACTION={hc_action} deref={deref} FR-96={emergency} \
         edge_of={edge} KeyEvent={key_event} guarded_decision={decision}"
    );

    assert!(
        emergency < edge,
        "FR-96 must be answered before the message is classified as an edge"
    );
    assert!(
        emergency < key_event,
        "FR-96 must be answered before dwExtraInfo is even read — it is not subject to FR-03"
    );
    assert!(
        emergency < decision,
        "FR-96 must be answered before catch_unwind, classify, FR-03, FR-90 and the hotkey"
    );

    // The only two things allowed in front of it, and they are there. (Under the `testing`
    // feature — and only there — task T-10-1's QPC probe stands above the HC_ACTION test
    // too: one leaf counter read that cannot fail, loop or block, absent from every shipped
    // build. The exact-list assertion below covers the stretch from the dereference to
    // FR-96, which the probe does not enter.)
    assert!(hc_action < emergency, "the system's contract comes first");
    assert!(
        deref < emergency && hc_action < deref,
        "the dereference is what makes there be a key event, and it is guarded by HC_ACTION"
    );

    // Nothing else stands between the dereference and FR-96 but the two field reads it needs.
    // Comments are dropped first: this is about what the callback *does*, and a prose line
    // containing the word "for" is not a loop.
    let between: String = body[deref..emergency]
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    println!("what really stands between the dereference and FR-96:\n{between}");

    // Asserted as the exact list rather than as the absence of a few keywords: a keyword list
    // is a guess about what a later edit might add, and this is not a guess. Three statements —
    // the dereference and the two field reads FR-96 needs — and an emergency exit standing
    // behind anything that can fail, loop or block is not an emergency exit.
    let statements: Vec<&str> = between.lines().filter(|line| !line.is_empty()).collect();

    assert_eq!(
        statements,
        vec![
            "let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };",
            "let vk = event.vkCode as u16;",
            "let message = wparam.0 as u32;",
        ],
        "something new stands between the dereference and FR-96"
    );

    // FR-96 in both configurations — no `cfg` anywhere in the block.
    let block = &body[emergency..edge];
    assert!(
        !block.contains("#[cfg"),
        "the FR-96 block must carry no cfg: §4.11 and the decision on question 18"
    );
}

/// FR-96 asks nothing about `dwExtraInfo`, so the filter of FR-03 cannot silence it.
///
/// The half of FR-96 that does not need Win32 is [`hook::is_emergency_key`], and its signature is
/// the proof: it takes a virtual-key code and a message and has no way to see a signature. This
/// pins that, so that a later task cannot "tidy" the emergency test into `classify`, where
/// FR-03 would swallow the program's own injected input — and with it the one way out of a
/// wedged hook.
///
/// Measured against the running program as well: task T-08-3 sent the combination stamped with
/// `INJECTED_SIGNATURE` itself and the process still ended with `EXIT_EMERGENCY`.
#[test]
fn fr_96_is_not_subject_to_the_filter_of_fr_03() {
    // The emergency key, going down, is the emergency key whatever else is true of the stroke.
    assert!(hook::is_emergency_key(0x7B, WM_KEYDOWN));
    assert!(hook::is_emergency_key(0x7B, WM_SYSKEYDOWN));
    assert!(!hook::is_emergency_key(0x7B, WM_KEYUP));
    assert!(!hook::is_emergency_key(0x7B, WM_SYSKEYUP));
    assert!(!hook::is_emergency_key(VK_A, WM_KEYDOWN));

    // And `classify` — which is where FR-03 lives — never gets a say: a stroke carrying the
    // product's own signature is passed on by it, which is exactly why the emergency test
    // cannot live there.
    let mut state = HotkeyState::default();
    let signed = KeyEvent {
        vk: 0x7B,
        edge: Edge::Down,
        extra_info: INJECTED_SIGNATURE,
        scan: 0,
        flags: 0,
        time: 0,
    };

    let outcome = hook::classify(hook::current_mode(), &mut state, signed);

    assert_eq!(
        outcome.decision,
        Decision::Pass,
        "FR-03 passes our own injected input on — so FR-96 must be answered before this point"
    );
    assert!(!outcome.fire_hotkey);
    assert!(!outcome.probe_layout);
    assert_eq!(hook::EXIT_EMERGENCY, 3);
}

// -------------------------------------------------------------------------------------
// Task T-10-1 — the QPC instrument of criterion 2 §13, feature `testing` only
// -------------------------------------------------------------------------------------

/// The histogram of the latency instrument, driven with a **known** distribution.
///
/// No test installs a hook (the header of this file says why), so the real probe cannot be
/// exercised here; what can be, deterministically, is the arithmetic every verdict of
/// position 23 rests on. `hook::record_callback_ticks` is the funnel the probe's drop feeds,
/// so this drives the very path the measurement uses, minus the two QPC reads.
///
/// One test and not three, on purpose: the histogram is a process-wide static and the tests
/// of one binary run concurrently, so the empty reading must be asserted before anything
/// records, inside the same test. Three acts, in order.
#[cfg(feature = "testing")]
#[test]
fn the_latency_instrument_reports_the_recorded_distribution() {
    use windows::Win32::System::Performance::QueryPerformanceFrequency;

    // Act 1 — an instrument nothing has touched reports zeros, not remnants.
    let empty = hook::callback_latency();
    assert_eq!(empty.samples, 0, "no callback has run in this process");
    assert_eq!((empty.p50_ns, empty.p99_ns, empty.max_ns), (0, 0, 0));

    // The same clock the instrument converts against, read the same way. The frequency is
    // documented constant since boot, so the expected values below are exact, not close.
    let mut frequency = 0i64;
    // SAFETY: writes one `i64` through a pointer to a live local; nothing else is touched.
    unsafe { QueryPerformanceFrequency(&mut frequency) }
        .expect("QueryPerformanceFrequency is documented to succeed since Windows XP");
    assert!(frequency > 0);
    let to_ns = |ticks: u64| ticks * 1_000_000_000 / frequency as u64;

    // Act 2 — a distribution whose percentiles are arithmetic: ninety-nine samples of two
    // ticks and a single spike of five hundred.
    for _ in 0..99 {
        hook::record_callback_ticks(2);
    }
    hook::record_callback_ticks(500);

    // Act 3 — the reading matches the arithmetic.
    let read = hook::callback_latency();
    assert_eq!(read.samples, 100);
    assert_eq!(
        read.p50_ns,
        to_ns(3),
        "rank 50 of 100 falls in the [2,3) cell, reported as its upper bound"
    );
    assert_eq!(
        read.p99_ns,
        to_ns(3),
        "rank 99 of 100 is the ninety-ninth two-tick sample, still the [2,3) cell"
    );
    assert_eq!(
        read.max_ns,
        to_ns(500),
        "the maximum is exact — the spike itself, not a cell bound"
    );
    assert!(
        read.p50_ns <= read.p99_ns,
        "one histogram, one rank rule: the median cannot exceed the 99th percentile"
    );
}

// -------------------------------------------------------------------------------------
// Task T-13-4 — the resumption of FR-90 seeds `CapsLock` (point 3)
// -------------------------------------------------------------------------------------

/// **Point 3 of task T-13-4, driven through the product's own path.**
///
/// While FR-90 has the program suspended, `classify` answers `PASS` before `record` is reached,
/// so a `CapsLock` pressed during the pause never reaches the tracker and the belief comes out of
/// the pause **inverted** — for the rest of the session, in every application. The resumption
/// therefore asks the input thread to read the machine again, and this is that ask arriving:
/// `hook::set_active` publishes the resumption on the UI thread and posts
/// [`hook::WM_APP_SEED_CAPS`], and `handle_input_message` answers it where the buffer lives.
///
/// # Why the assertion is against `caps_lock_on()` and not against `true`
///
/// The machine's toggle is the machine's. A test that asserted `true` would pass or fail by
/// whether whoever is running `cargo test` happens to have `CapsLock` on, which is a state of the
/// room and not of the code. Asserting that the tracker ends up **equal to the reading the
/// product makes** is the property the repair is about, and it is the same on every machine.
///
/// # Why the missed press is stated rather than pressed
///
/// A press that reaches `Recorder::record` is exactly the press this defect is *not* about. What
/// is stated here is the state such a press leaves behind when it is missed — the tracker
/// disagreeing with the machine — which no keystroke this binary could make would produce,
/// because a test binary installs no hook (the header of this file says why).
#[test]
fn the_resumption_of_fr90_seeds_capslock_on_the_thread_that_owns_the_buffer() {
    use lang_switcher::buffer::{self, Recorder};
    use windows::Win32::Foundation::{LPARAM, WPARAM};

    let seed = || hook::handle_input_message(hook::WM_APP_SEED_CAPS, WPARAM(0), LPARAM(0));

    // Act 1 — a thread that owns no typing buffer, which is every thread of this process but
    // the input one. The message is answered and nothing whatever happens: the probe is called
    // inside `buffer::with`, so `GetKeyState` is not reached at all. That is the rule "do not
    // ask the UI thread about a queue it does not have", enforced by construction.
    assert!(
        !buffer::is_installed(),
        "this test must start on a thread with no buffer"
    );
    assert!(
        seed().is_some(),
        "the message is this module's and is answered wherever it lands"
    );

    // Act 2 — the input thread's situation: a buffer, and a tracker that disagrees with the
    // machine because the press that would have flipped it never reached `record`.
    buffer::install_recorder(Recorder::with_capacity(8));

    let machine = hook::caps_lock_on();
    buffer::with(|recorder| recorder.set_caps_lock(!machine));

    assert_eq!(
        buffer::with(|recorder| recorder.held().caps()),
        Some(!machine),
        "the state a missed CapsLock leaves behind, as the baseline of this test"
    );

    // Act 3 — the resumption arrives, and the belief is the machine's again.
    assert!(seed().is_some());

    assert_eq!(
        buffer::with(|recorder| recorder.held().caps()),
        Some(machine),
        "point 3: coming back from the pause of FR-90 must re-read the machine's CapsLock"
    );

    buffer::uninstall();
}

/// **NFR-01 to NFR-05: task T-13-4 put nothing into the hook callback**, and this is where the
/// boundary runs.
///
/// # Why this test reads the source
///
/// The property is an *absence* on a path that cannot be called from a test —
/// `keyboard_hook_proc` is `unsafe extern "system"`, is called by the system with a pointer only
/// the system can produce, and no return value of it can show which calls it did not make. What
/// can be checked honestly is the shape of the functions themselves, which is also what a
/// reviewer checks; `fr_96_is_decided_before_any_other_logic_of_the_callback` above reads the
/// source for the same reason and in the same way.
///
/// # What is asserted
///
/// The three functions the callback is made of — the callback itself, the `catch_unwind` wrapper
/// of FR-98 and the decision of FR-10 — contain neither the seeding probe nor the Win32 call
/// behind it. The seeding of task T-13-4 lives in `install` and in the message loop of the input
/// thread, on the other side of that line.
#[test]
fn the_capslock_seed_is_outside_the_callback() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable");

    // A top-level function closes with a brace in the first column, which is what bounds the
    // body here. **Every** definition of a name is checked, not the first: `guarded_decision`
    // has two, one under `cfg(panic = "unwind")` and one without, and a rule that held for one
    // of them would be no rule at all. Comments are kept in the body: a mention of
    // `GetKeyState` in prose inside the callback would be worth failing over too, since the
    // next reader would take it for a licence.
    for signature in [
        "unsafe extern \"system\" fn keyboard_hook_proc",
        "fn guarded_decision(",
        "pub fn classify(",
    ] {
        let mut definitions = 0;

        for (at, _) in source.match_indices(signature) {
            let after = &source[at..];
            let end = after
                .find("\n}\n")
                .expect("a top-level function closes in the first column");
            let body = &after[..end];

            definitions += 1;

            assert!(
                !body.contains("caps_lock_on"),
                "{signature}: the CapsLock seed of task T-13-4 must stay out of the callback"
            );
            assert!(
                !body.contains("GetKeyState"),
                "{signature}: NFR-01 to NFR-05 give the callback no new system call"
            );
            assert!(
                !body.contains("set_caps_lock"),
                "{signature}: the seeding entry point is not reachable from the callback"
            );
        }

        assert!(
            definitions > 0,
            "the function {signature:?} must be in this file"
        );
    }
}
