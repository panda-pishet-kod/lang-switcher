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
        hotkey_modifiers: 0,
        hotkey_double_tap: false,
        hotkey_yields: false,
    }
}

/// `hook::classify` with **no modifier held and no mouse button pressed** — tasks T-93-1, T-95-1.
///
/// The decision has taken the held modifiers as a fourth argument since вопрос 157, and the count
/// of mouse buttons as a fifth since вопрос 159; every test of this file written before them is
/// about a stroke made with nothing held and no mouse: the hotkey of FR-02 and FR-08, the
/// signature of FR-03, the states of FR-90 and FR-99, the probe of FR-21. The tests of the
/// combinations and of the double press call `hook::classify` themselves, with what they hold.
fn classify(mode: Mode, state: &mut HotkeyState, key: KeyEvent) -> Outcome {
    hook::classify(mode, state, key, || 0, || 0)
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
// Task T-36-2 — the hook starts disarmed (finding С19, variant 1)
// -------------------------------------------------------------------------------------

/// **Nothing is swallowed and nothing is buffered before the configuration is published —
/// task T-36-2.**
///
/// The two threads of section 6.1 do not wait for each other. The input thread installs the
/// hook at once, because NFR-08 gives the program under fifty milliseconds to have one; the UI
/// thread meanwhile creates its window, loads icons, reads `config.toml`, builds the tray, and
/// only then calls `app::publish_configuration`, whose `hook::set_active(config.general.enabled)`
/// is the first word the callback hears about the user's settings. Between the two there is a
/// window of tens to hundreds of milliseconds, and finding С19 is about what the program does in
/// it: on an armed default it suppresses `Pause` in whatever application has the focus, records
/// strokes into a buffer the user may have switched off, and honours no exclusions, because none
/// have been published either.
///
/// So the default is "disarmed", and this test states it as a value and then as behaviour: the
/// mode the callback really computes before any publication ([`hook::current_mode`] would read
/// the same statics) passes the hotkey on and buffers nothing.
///
/// ⚠ The default of the **hotkey** is untouched and must stay so: `classify` answers on
/// `!mode.active` before it ever compares `key.vk` with `mode.hotkey_vk`, so `Pause` stops being
/// swallowed without touching [`hook::DEFAULT_HOTKEY_VK`].
#[test]
fn the_hook_starts_disarmed_until_the_configuration_is_published() {
    // Через переменную, а не `assert!(!hook::DEFAULT_ACTIVE, …)`: clippy справедливо зовёт
    // утверждение над константой утверждением с постоянным значением. Значение от этого не
    // меняется — тест краснеет ровно тогда, когда умолчание вернётся к «вооружён».
    let default_the_callback_sees = hook::DEFAULT_ACTIVE;

    assert!(
        !default_the_callback_sees,
        "С19: the hook must go up disarmed — the callback sees this value until \
         `app::publish_configuration` publishes the user's own `general.enabled`"
    );

    let unpublished = Mode {
        active: hook::DEFAULT_ACTIVE,
        fail_safe: false,
        hotkey_vk: hook::DEFAULT_HOTKEY_VK,
        hotkey_modifiers: 0,
        hotkey_double_tap: false,
        hotkey_yields: false,
    };

    // FR-95 suppresses the hotkey "когда программа активна", and this program is not yet known
    // to be. The press of the default hotkey goes to whoever owns the focus.
    let mut state = HotkeyState::default();
    let hotkey = classify(unpublished, &mut state, user_key(VK_PAUSE, Edge::Down));

    assert_eq!(
        hotkey,
        passed_on(),
        "the default hotkey is not swallowed before the configuration is read"
    );
    assert!(
        !state.hotkey_down,
        "and no hold is remembered from a press this program did not take"
    );

    // The other half of С19: an ordinary stroke must not reach the ring either. `classify`
    // returns above `buffer::record` on `!active`, and this measures that from the buffer's own
    // side rather than by reading the source.
    {
        use lang_switcher::buffer::{self, Recorder};

        buffer::install_recorder(Recorder::with_capacity(8));
        assert_eq!(buffer::len(), 0, "the ring starts empty");

        let typed = classify(unpublished, &mut state, user_key(b'A'.into(), Edge::Down));

        assert_eq!(
            typed,
            passed_on(),
            "an ordinary stroke is handed on untouched"
        );
        assert_eq!(
            buffer::len(),
            0,
            "С19: a stroke made before the configuration was published must not be recorded — \
             the user may have left the program suspended (FR-90)"
        );

        buffer::uninstall();
    }
}

// -------------------------------------------------------------------------------------
// FR-02, FR-95 — the hotkey is recognised in the callback and always suppressed
// -------------------------------------------------------------------------------------

#[test]
fn pressing_the_hotkey_suppresses_it_and_starts_a_conversion() {
    let mut state = HotkeyState::default();

    let outcome = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));

    // FR-95: suppressed. FR-02: recognised, and the conversion is asked for.
    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(outcome.fire_hotkey);
}

#[test]
fn releasing_the_hotkey_is_suppressed_too_and_starts_nothing() {
    let mut state = HotkeyState::default();

    classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    let outcome = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Up));

    // A press that was swallowed and a release that was not would leave the application
    // holding a key it never saw go down.
    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(!outcome.fire_hotkey);
}

#[test]
fn an_ordinary_stroke_is_passed_on_untouched() {
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Up] {
        let outcome = classify(armed(), &mut state, user_key(VK_A, edge));

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
    let on_a = classify(mode, &mut state, user_key(VK_A, Edge::Down));
    assert_eq!(on_a.decision, Decision::Suppress);
    assert!(on_a.fire_hotkey);

    // ...and `Pause` is an ordinary key that goes through.
    let on_pause = classify(mode, &mut state, user_key(VK_PAUSE, Edge::Down));
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
            let outcome = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));

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
            if classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down)).fire_hotkey {
                fired += 1;
            }
        }

        classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Up));
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

    let outcome = classify(armed(), &mut state, ours);

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

        let outcome = classify(
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
        let outcome = classify(suspended, &mut state, user_key(VK_PAUSE, edge));
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
                let outcome = classify(
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
    classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    assert!(state.hotkey_down);

    // ...the program is suspended while the key is still down...
    let suspended = Mode {
        active: false,
        ..armed()
    };
    classify(suspended, &mut state, user_key(VK_PAUSE, Edge::Down));
    assert!(!state.hotkey_down, "the remembered state must not go stale");

    // ...and when it comes back, the next press is a press and not a repeat.
    let outcome = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
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
/// The top-row `1` and the grave key — the "switch to this language" combinations of FR-11,
/// task T-52-2.
const VK_1: u16 = 0x31;
const VK_OEM_3: u16 = 0xC0;

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

    let switchers: [Switcher; 6] = [
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
        // ⭐ **The "switch to this language" combinations of FR-11 — task T-52-2.** They were
        // outside FR-11 until that task and are inside it now, so the mechanism of T-03-3c has
        // to cover them too: after any of them the layout has moved, and the buffer has to be
        // told which layout the next word is being typed in. Nothing in `is_layout_probe`
        // needed changing — each of them ends in the release of a modifier it already watches —
        // and this is the measurement that says so rather than the assumption.
        (
            "Ctrl+Shift+1",
            &[
                (VK_LCONTROL, Edge::Down),
                (VK_LSHIFT, Edge::Down),
                (VK_1, Edge::Down),
                (VK_1, Edge::Up),
                (VK_LSHIFT, Edge::Up),
                (VK_LCONTROL, Edge::Up),
            ],
            2,
        ),
        (
            "Alt+Shift+1",
            &[
                (VK_LMENU, Edge::Down),
                (VK_LSHIFT, Edge::Down),
                (VK_1, Edge::Down),
                (VK_1, Edge::Up),
                (VK_LSHIFT, Edge::Up),
                (VK_LMENU, Edge::Up),
            ],
            2,
        ),
        (
            "Ctrl+Shift+гравис",
            &[
                (VK_LCONTROL, Edge::Down),
                (VK_LSHIFT, Edge::Down),
                (VK_OEM_3, Edge::Down),
                (VK_OEM_3, Edge::Up),
                (VK_LSHIFT, Edge::Up),
                (VK_LCONTROL, Edge::Up),
            ],
            2,
        ),
    ];

    for (name, events, expected_probes) in switchers {
        let mut state = HotkeyState::default();
        let mut probes = 0;
        let mut last_asked = false;

        for &(vk, edge) in events {
            let outcome = classify(armed(), &mut state, user_key(vk, edge));

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
            !classify(armed(), &mut state, user_key(vk, Edge::Down)).probe_layout,
            "the press of {vk:#04x} must not ask"
        );
        assert!(
            classify(armed(), &mut state, user_key(vk, Edge::Up)).probe_layout,
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
                !classify(armed(), &mut state, user_key(vk, edge)).probe_layout,
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
                !classify(mode, &mut state, user_key(vk, Edge::Up)).probe_layout,
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
            !classify(armed(), &mut state, ours).probe_layout,
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

    let outcome = classify(
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

            if classify(armed(), &mut state, user_key(vk, edge)).probe_layout {
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

// -------------------------------------------------------------------------------------
// Task T-13-9, point (а) — the SEC-05 gate on `WM_APP_FAIL_SAFE`
//
// The audit of 2026-08-24: this was the one private message of the program that acted on
// nothing but the sender's word. Any process at the same integrity level finds the windows
// by the class name `LangSwitcher.Hidden` or through `FindWindowEx(HWND_MESSAGE)`, and one
// `PostMessage(WM_APP + 4)` suspended the program — writing `general.enabled = false` to the
// disk on the UI window, and silently disarming the hook with `set_active(false)` on the
// input one. Every other message of the program was already built the other way round: a
// forgery finds nothing pending and does nothing.
//
// ⚠ **The one flag and the one writer.** `hook::fail_safe()` is raised by
// `FAIL_SAFE.swap(true, …)` inside `count_callback_panic` and by nothing else in the
// program, and that line is reached only by the fourth consecutive panic inside a callback
// the system alone can call. So no test binary can raise it, which is why the gate is
// measured here with the flag in its real state — down — and the work **behind** the gate is
// measured through `hook::suspend_for_fail_safe`, exactly as `classify` is measured against a
// `Mode` handed in rather than against the atomics. The live proof of FR-99 end to end stays
// where it has always been: the acceptance run of §11.5.
// -------------------------------------------------------------------------------------

/// Serialises the tests that read or move the mode the UI thread publishes.
///
/// `ACTIVE` is process-wide and the tests of one binary run on parallel threads, so the test
/// below — which suspends the program and puts it back — and
/// `the_program_starts_armed_on_the_default_hotkey`, which asserts what the value is, must not
/// overlap.
static PUBLISHED_MODE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// **Criterion 5 of task T-13-9.** A forged `WM_APP_FAIL_SAFE` finds the flag down and nothing
/// at all happens.
///
/// The measurement is of the half that was never thread-bound. `with_tray` answers `None` on
/// any thread but the UI one, so the tray was always out of a forgery's reach *on this thread*;
/// `set_active(false)` was not, and disarming the hook while the icon went on saying "активна"
/// is what the finding calls out for the input thread's own window. The file half of the same
/// criterion — that no configuration is written — is measured in `tests\tray.rs`, where there
/// is a tray and a folder to watch.
#[test]
fn a_forged_fail_safe_message_finds_the_flag_down_and_does_nothing() {
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};

    let _mode = PUBLISHED_MODE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    assert!(
        !hook::fail_safe(),
        "no callback of this process has panicked four times running"
    );

    // ⚠ **Armed explicitly since task T-36-2, and the previous value is put back below.** This
    // test is about what a forgery can do to an **armed** program, and it used to take the
    // arming from the default of `hook::ACTIVE`, which was `true`. Finding С19 moved that
    // default to "disarmed" — the hook goes up before the configuration is published and must
    // swallow nothing in that window — and a test binary publishes no configuration, so the
    // arming has to be stated here. The pattern is the one
    // `the_genuine_fail_safe_message_still_disarms_the_program` below already uses.
    let was_active = hook::is_active();
    hook::set_active(true);

    let before = (
        hook::is_active(),
        hook::hotkey_handoffs(),
        hook::post_failures(),
        hook::consecutive_panics(),
    );

    for _ in 0..1_000 {
        let answered = hook::handle_input_message(hook::WM_APP_FAIL_SAFE, WPARAM(0), LPARAM(0));

        assert_eq!(
            answered,
            Some(LRESULT(0)),
            "the message is still one of ours — the list of `handle_input_message` is closed \
             and explicit (SEC-05); a gate that answered `None` would hand it to DefWindowProcW"
        );
    }

    let after = (
        hook::is_active(),
        hook::hotkey_handoffs(),
        hook::post_failures(),
        hook::consecutive_panics(),
    );

    println!("1000 forged WM_APP_FAIL_SAFE: before={before:?} after={after:?}");

    assert_eq!(
        after, before,
        "SEC-05: a thousand forgeries buy the sender one atomic load each and nothing else"
    );
    assert!(
        hook::is_active(),
        "the program is still armed — FR-90, FR-95"
    );

    hook::set_active(was_active);
}

/// **Criterion 6 of task T-13-9.** Past the gate the message does what it always did.
///
/// The two halves of the arm, in the order the arm runs them: the tray is asked to show
/// "приостановлена" — `with_tray` finds none on a test thread, which is the same `None` the
/// input and watcher threads get and is why the gate and not `with_tray` is the defence — and
/// the program is disarmed with `set_active(false)`, which is the half a forgery used to reach.
///
/// The state is put back at the end. `set_active(true)` is the resumption of FR-90 and posts
/// [`hook::WM_APP_SEED_CAPS`] at the input thread's window; there is none in a test binary, so
/// the post is dropped, which is what the counters below check.
#[test]
fn the_genuine_fail_safe_message_still_disarms_the_program() {
    let _mode = PUBLISHED_MODE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let was_active = hook::is_active();
    let failures_before = hook::post_failures();

    hook::suspend_for_fail_safe();

    println!(
        "after suspend_for_fail_safe: is_active={} post_failures={} (was {failures_before})",
        hook::is_active(),
        hook::post_failures()
    );

    assert!(
        !hook::is_active(),
        "FR-99: the program is disarmed and every stroke is passed through untouched"
    );

    // And the decision follows the published mode, which is the whole point of disarming it:
    // an ordinary stroke is passed on and the hotkey is no longer suppressed (FR-90, FR-95).
    let mut state = HotkeyState::default();
    let stroke = KeyEvent {
        vk: hook::hotkey_vk(),
        edge: Edge::Down,
        extra_info: 0,
        scan: 0,
        flags: 0,
        time: 0,
    };
    let outcome = classify(hook::current_mode(), &mut state, stroke);

    assert_eq!(outcome.decision, Decision::Pass);
    assert!(!outcome.fire_hotkey);

    hook::set_active(was_active);

    assert_eq!(hook::is_active(), was_active, "the mode is put back");
    assert_eq!(
        hook::post_failures(),
        failures_before,
        "NFR-13: nothing failed on the way — with no input window the seed of task T-13-4 is \
         dropped rather than counted"
    );
}

/// **Criterion 8 of task T-13-9.** Every arm of `handle_input_message` answers to something the
/// sender cannot write.
///
/// Swept over the source, for the reason `the_capslock_seed_is_outside_the_callback` below
/// sweeps it: what is being asserted is a property of the *shape* of a function whose inputs
/// come from the system. The list of arms is read out of the file so that an arm added later
/// cannot slip past this test by not being named in it.
#[test]
fn every_private_message_of_this_module_is_gated_on_state_the_sender_cannot_write() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable")
    // The canonical checkout of this repository is CRLF (`.gitattributes`: `eol=crlf`), so a
    // needle written with `\n` has to be matched against a text that holds `\n`. Every
    // source-reading test in this file normalises for that reason — see
    // `the_capslock_seed_is_outside_the_callback`.
    .replace("\r\n", "\n");

    let at = source
        .find("pub fn handle_input_message(")
        .expect("the function must be in this file");
    let body = &source[at..];
    let end = body
        .find("\n}")
        .expect("a top-level function closes its brace");
    let body = &body[..end];

    let arms: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("WM_APP_") && line.ends_with("=> {"))
        .collect();

    println!("arms of handle_input_message: {arms:?}");

    assert_eq!(
        arms,
        vec![
            "WM_APP_HOTKEY => {",
            "WM_APP_FAIL_SAFE => {",
            "WM_APP_SEED_CAPS => {",
        ],
        "three arms, and a fourth would have to be argued here before it is added"
    );

    // The one this task is about. The other two are argued at their constants and neither can
    // be driven by what the message carries: `WM_APP_HOTKEY` converts text the sender cannot
    // see, and `WM_APP_SEED_CAPS` replaces this program's belief with the system's own reading.
    let gate = body
        .find("if !fail_safe() {")
        .expect("the WM_APP_FAIL_SAFE arm must be gated on the flag only this process writes");
    let arm = body
        .find("WM_APP_FAIL_SAFE => {")
        .expect("the arm must be in this function");
    let work = body
        .find("suspend_for_fail_safe();")
        .expect("the work of the arm must be behind the gate");

    assert!(
        arm < gate && gate < work,
        "the gate is the first thing the arm does, and the work comes after it"
    );
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

/// What the callback matches against before anything has been published — the window NFR-08
/// exists to keep short.
///
/// ⚠ **This test carried the previous default and was moved by task T-36-2** (finding С19). It
/// used to be `the_program_starts_armed_on_the_default_hotkey` and to assert `mode.active` with
/// the reason «a resident utility starts armed». That is what the finding is about: the hook is
/// up tens to hundreds of milliseconds before `app::publish_configuration` says whether the user
/// left the program running or suspended, and acting on a guess in that window swallows `Pause`
/// in other applications and records strokes the user switched off. The default is now
/// [`hook::DEFAULT_ACTIVE`], the arming comes from the configuration, and the canon moved by
/// decision 123 rather than by being squeezed into the old number.
#[test]
fn the_program_starts_disarmed_on_the_default_hotkey() {
    // ⚠ `ACTIVE` is process-wide and the tests of one binary run on parallel threads. Task
    // T-13-9 added a test that moves it and puts it back — see [`PUBLISHED_MODE`] — and this
    // one asserts what the value is, so the two take the same lock.
    let _mode = PUBLISHED_MODE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let mode = hook::current_mode();

    assert_eq!(
        mode.active,
        hook::DEFAULT_ACTIVE,
        "С19: until the UI thread publishes `general.enabled`, the callback sees the default — \
         and that default is 'disarmed'"
    );
    assert!(!mode.active, "and the default is disarmed");
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

    let outcome = classify(hook::current_mode(), &mut state, signed);

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
///
/// # ⚠ What this test does **not** measure — task T-36-1
///
/// **It checks the tracker, not the reading of the machine.** The name says so since T-36-1: the
/// reference on both sides of the seed is `hook::caps_lock_on()` itself, so a reading stuck on any
/// constant — which is exactly what finding С2 of the audit of 2026-09-04 suspects — passes every
/// line here. The reading is measured against the cause instead by
/// [`the_capslock_reading_follows_the_toggle_this_test_moves`], which moves the toggle itself.
#[test]
fn the_resumption_of_fr90_seeds_the_tracker_from_the_reading_of_the_machine() {
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
///
/// # ⚠ The line endings, and why the source is normalised — task T-13-9, by the controller's
/// instruction
///
/// This test used to read the file and look for `"\n}\n"`. `.gitattributes` declares
/// `* text=auto eol=crlf`, so **the canonical checkout of this repository is CRLF** and the
/// file really holds `"\r\n}\r\n"` — in which that needle does not occur once. The first
/// `.expect` then fired and the test was red on every fresh `git worktree`, whatever the code
/// said; it passed only in a working tree that happened to hold LF. The verdict of a test
/// about the callback must not depend on how a checkout stored the newlines, so the text is
/// normalised before it is parsed and every needle below is written in the one form that
/// remains. Nothing else about the test changed: the same three functions, every definition of
/// each, comments kept in the body, the same three forbidden names.
#[test]
fn the_capslock_seed_is_outside_the_callback() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable")
    // The one line the eol fix adds. `\r` carries no meaning for anything asserted below.
    .replace("\r\n", "\n");

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

// -------------------------------------------------------------------------------------
// Task T-52-3 — FR-84 and FR-95: in an excluded process the hotkey is the application's
// -------------------------------------------------------------------------------------

/// The ordinary running state, with the foreground process excluded — FR-84.
fn armed_in_an_excluded_process() -> Mode {
    Mode {
        hotkey_yields: true,
        ..armed()
    }
}

#[test]
fn in_an_excluded_process_the_hotkey_reaches_the_application() {
    // ⭐ **The finding of task T-52-3.** FR-95 names the remedy for an application that needs
    // the hotkey for itself — «предусмотрен список исключений (FR-84)» — and the remedy was not
    // in the code. `Mode` knew three things and none of them was the exclusion; the hotkey
    // branch stands *above* every use of the buffer, and the exclusion gate lived below it, in
    // `buffer::record`. So in an excluded game `Pause` was swallowed, the conversion refused
    // and the dull thud of FR-100 played — the requirement's own remedy did nothing.
    let mut state = HotkeyState::default();

    let outcome = classify(
        armed_in_an_excluded_process(),
        &mut state,
        user_key(VK_PAUSE, Edge::Down),
    );

    assert_eq!(
        outcome.decision,
        Decision::Pass,
        "в исключённом процессе Pause подавлена — приложение её не получило"
    );
    assert!(
        !outcome.fire_hotkey,
        "в исключённом процессе Pause запустила конвертацию"
    );

    // And the release goes the same way, so the application sees a whole keystroke.
    let up = classify(
        armed_in_an_excluded_process(),
        &mut state,
        user_key(VK_PAUSE, Edge::Up),
    );

    assert_eq!(up.decision, Decision::Pass);
    assert!(!up.fire_hotkey);
}

#[test]
fn the_release_of_the_hotkey_repeats_the_fate_of_its_press() {
    // ⚠ **The verdict can change between the press and the release** — the user alt-tabs, or
    // the watcher's answer for the new foreground window arrives in the middle of the
    // keystroke. Whichever way it moves, the application must see a whole key or none of it: a
    // suppressed press whose release was let through leaves it holding a key it never saw go
    // down, and a passed press whose release was swallowed leaves it holding one for ever.
    //
    // So the fate is decided once, on the press, and remembered.

    // Pressed in an excluded process, released after the exclusion has gone.
    let mut state = HotkeyState::default();

    assert_eq!(
        classify(
            armed_in_an_excluded_process(),
            &mut state,
            user_key(VK_PAUSE, Edge::Down)
        )
        .decision,
        Decision::Pass
    );
    assert_eq!(
        classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Up)).decision,
        Decision::Pass,
        "нажатие пропущено, а отпускание подавлено — приложение держит клавишу вечно"
    );

    // Pressed in an ordinary process, released after the process became excluded.
    let mut state = HotkeyState::default();

    let down = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));
    assert_eq!(down.decision, Decision::Suppress);
    assert!(down.fire_hotkey);

    assert_eq!(
        classify(
            armed_in_an_excluded_process(),
            &mut state,
            user_key(VK_PAUSE, Edge::Up)
        )
        .decision,
        Decision::Suppress,
        "нажатие подавлено, а отпускание пропущено — приложение получило клавишу, \
         которую не видело нажатой"
    );
}

#[test]
fn the_auto_repeat_of_a_yielded_hotkey_reaches_the_application_as_it_is() {
    // FR-08 suppresses *conversions*, not keystrokes. A key the program has decided not to take
    // is not its business at all, so the repeats the system sends go through exactly as the
    // first press did — which is what makes `Pause` pause the output of an excluded console.
    let mut state = HotkeyState::default();

    for _ in 0..5 {
        let outcome = classify(
            armed_in_an_excluded_process(),
            &mut state,
            user_key(VK_PAUSE, Edge::Down),
        );

        assert_eq!(outcome.decision, Decision::Pass);
        assert!(
            !outcome.fire_hotkey,
            "FR-84: конвертации нет ни на одном повторе"
        );
    }

    assert_eq!(
        classify(
            armed_in_an_excluded_process(),
            &mut state,
            user_key(VK_PAUSE, Edge::Up)
        )
        .decision,
        Decision::Pass
    );

    // And the very next press outside the exclusion is an ordinary hotkey press again.
    let outcome = classify(armed(), &mut state, user_key(VK_PAUSE, Edge::Down));

    assert_eq!(outcome.decision, Decision::Suppress);
    assert!(outcome.fire_hotkey, "вне исключения горячая клавиша наша");
}

#[test]
fn the_exclusion_changes_nothing_for_an_ordinary_stroke() {
    // FR-84 takes the *buffer* away through the gate on the input thread, and this file's
    // concern is only the hotkey. An ordinary key is passed on either way, and the flag must
    // not turn into a second, contradictory gate on the callback path.
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Up] {
        assert_eq!(
            classify(
                armed_in_an_excluded_process(),
                &mut state,
                user_key(VK_A, edge)
            ),
            passed_on(),
            "an ordinary stroke in an excluded process, {edge:?}"
        );
    }
}

#[test]
fn the_states_above_the_exclusion_still_answer_first() {
    // The order of the rows of `classify` is the requirement, and the exclusion joins the
    // bottom of it. FR-99 and FR-90 already pass the hotkey on; the exclusion must not make
    // either of them fire a conversion, and their remembered state must stay as it was.
    for mode in [
        Mode {
            fail_safe: true,
            ..armed_in_an_excluded_process()
        },
        Mode {
            active: false,
            ..armed_in_an_excluded_process()
        },
    ] {
        let mut state = HotkeyState::default();

        let outcome = classify(mode, &mut state, user_key(VK_PAUSE, Edge::Down));

        assert_eq!(outcome.decision, Decision::Pass);
        assert!(!outcome.fire_hotkey);
        assert!(!state.hotkey_down, "the remembered state must not go stale");
    }
}

#[test]
fn the_exclusion_verdict_reaches_the_callback_through_this_modules_own_static() {
    // The publication path, from the outside: the input thread's gate stores the verdict here,
    // and `current_mode` — what the callback actually uses — reads it back. Section 6.3 puts
    // the reader of the published verdict on the input thread's message loop, and this is the
    // seam where it hands the answer over to the callback.
    let restore = hook::hotkey_yields();

    hook::set_hotkey_yields(true);
    assert!(hook::hotkey_yields());
    assert!(
        hook::current_mode().hotkey_yields,
        "current_mode не читает опубликованный вердикт FR-84"
    );

    hook::set_hotkey_yields(false);
    assert!(!hook::current_mode().hotkey_yields);

    hook::set_hotkey_yields(restore);
}

// -------------------------------------------------------------------------------------
// Task T-36-7 — installation is one indivisible claim (finding Н35)
// -------------------------------------------------------------------------------------

/// **The four states of the cell `HOOK`, and what each of them answers — task T-36-7.**
///
/// # Why the protocol is a value and not three lines
///
/// Installation used to be three steps — check that the cell is empty, call
/// `SetWindowsHookExW`, publish the handle — and between the second and the third **the hook
/// existed in the system and the program did not know it**. Removal is one operation and is
/// called from other threads: FR-96 from inside the callback, FR-98 from the panic hook, FR-97
/// from the timeout thread, FR-83 from the UI. A removal that landed in that window found an
/// empty cell, answered "nothing to remove", and left the hook standing — the program believing
/// it had let the keyboard go while it had not. The other side of the same window is two hooks at
/// once, which FR-01 forbids.
///
/// The repair puts a **mark** in the cell for the length of the call, so the invariant is held by
/// the machine. This test drives the interpretation of that cell — `hook::cell_of` — over every
/// state, because the transitions themselves cannot be called from a test binary: they end in
/// `SetWindowsHookExW`, and the header of this file says why no test here installs a hook.
///
/// The real transitions are checked by the sweep below
/// (`the_installation_claims_the_cell_before_the_system_call`), which reads the two functions and
/// shows that each state of this table is the one they act on.
#[test]
fn the_cell_of_the_hook_has_four_states_and_a_claim_is_not_an_installed_hook() {
    use lang_switcher::hook::{HookCell, cell_is_installed, cell_of};

    const NO_HANDLE: usize = 0;
    const INSTALLING: usize = usize::MAX;
    const CANCELLED: usize = usize::MAX - 1;
    // Any value that is neither: this stands for the handle `SetWindowsHookExW` returns.
    const HANDLE: usize = 0x0000_1234_5678_9ABC;

    assert_eq!(cell_of(NO_HANDLE), HookCell::Free);
    assert_eq!(cell_of(INSTALLING), HookCell::Installing);
    assert_eq!(cell_of(CANCELLED), HookCell::Cancelled);
    assert_eq!(cell_of(HANDLE), HookCell::Installed(HANDLE));

    // ⭐ The property the watchdog rests on: a claim in flight is **not** a hook. `is_installed`
    // answers «does this program have one?», and a claim that is cancelled a moment later never
    // becomes one; a `true` here would make `watchdog::reinstall_hook` believe a hook it never
    // got, and `ABSENT_AT_CHECK` would stop counting the absences it exists to count.
    assert!(cell_is_installed(HANDLE), "a handle is an installed hook");
    assert!(!cell_is_installed(NO_HANDLE), "an empty cell is not");
    assert!(
        !cell_is_installed(INSTALLING),
        "Н35: a claim in flight is not an installed hook — it may still be cancelled"
    );
    assert!(
        !cell_is_installed(CANCELLED),
        "and neither is a claim somebody has already asked to undo"
    );

    // The marks are not handles. `HHOOK` is a pointer into the user-mode address space, so
    // neither of these can ever be returned by the system — which is what makes them usable as
    // marks in the same word.
    assert_ne!(INSTALLING, CANCELLED);
    for mark in [INSTALLING, CANCELLED] {
        assert!(
            mark > 0x0000_7FFF_FFFF_FFFF,
            "a mark must be outside every address a hook handle can have: {mark:#x}"
        );
    }
}

/// **The two functions really act on that table — task T-36-7, the sweep.**
///
/// Honest red is out of reach here for the reason the task specification states: `install` cannot
/// be called from a test binary at all (it would register a real `WH_KEYBOARD_LL` in a process
/// that pumps no messages and freeze the machine's keyboard for `LowLevelHooksTimeout` on every
/// stroke). So the shape is read, as `fr_96_is_decided_before_any_other_logic_of_the_callback`
/// reads the ordering of the callback — and, as there, a reviewer checks the same thing.
///
/// What must hold, and each of these is a way the old three-step shape could come back:
///
/// * the claim is a `compare_exchange` from the free state to the mark, and it is the **first**
///   thing `install` does — before `SetWindowsHookExW`;
/// * the handle is published by a `compare_exchange` from the mark, not by a `store`: a `store`
///   would overwrite a cancellation instead of seeing it;
/// * every failing path of `install` puts the cell back to free, or a claim would be stuck for
///   the life of the process and every later installation refused;
/// * `uninstall` no longer takes the cell with an unconditional `swap`, and it answers the claim
///   with the cancellation mark;
/// * nothing on either path blocks or sleeps — FR-96 runs this inside the callback.
#[test]
fn the_installation_claims_the_cell_before_the_system_call() {
    // ⚠ **Flattened before anything is matched.** `rustfmt` breaks a `compare_exchange` with four
    // arguments across five lines as soon as the line grows, and a needle written as one line
    // stops matching without anything having changed — the trap of stage Э32, met by the first
    // draft of this very test. Collapsing every run of whitespace to a single space makes the
    // needles say what they mean: these tokens, in this order.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable")
    .replace("\r\n", "\n")
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
    // …and the brackets closed up again: `rustfmt` puts each argument of a long call on its own
    // line, so a flattened call reads `compare_exchange( INSTALLING, …` with a space the source
    // never had. These three replacements make the text read the way the call is written.
    .replace("( ", "(")
    .replace(" )", ")")
    .replace(" ,", ",");

    let install = source
        .split_once("pub fn install(notify: HWND, instance: HINSTANCE, memory: HotkeyMemory)")
        .expect("install must be in this file")
        .1
        .split_once(" pub fn ")
        .expect("the next function follows it")
        .0;

    let uninstall = source
        .split_once("pub fn uninstall() -> bool {")
        .expect("uninstall must be in this file")
        .1
        .split_once(" pub fn ")
        .expect("the next function follows it")
        .0;

    let at = |body: &str, needle: &str| -> usize {
        body.find(needle)
            .unwrap_or_else(|| panic!("Н35: the protocol no longer contains {needle:?}"))
    };

    // The order inside `install`: the claim, then the system call, then the publication.
    let claim = at(install, "compare_exchange(NO_HANDLE, INSTALLING");
    let system_call = at(install, "SetWindowsHookExW(WH_KEYBOARD_LL");
    let publish = at(install, "compare_exchange(INSTALLING, hook.0 as usize");

    println!("install: claim at {claim}, SetWindowsHookExW at {system_call}, publish at {publish}");

    assert!(
        claim < system_call,
        "Н35: the cell must be claimed BEFORE the hook exists — a claim after the call is the \
         three-step shape this task removed, with the window still in it"
    );
    assert!(
        system_call < publish,
        "and the handle is published after the call that produced it"
    );

    // Every way out of `install` frees the cell again. Three: the call refused, the handle was
    // null, the claim was cancelled under it.
    assert_eq!(
        install
            .matches("HOOK.store(NO_HANDLE, Ordering::Release)")
            .count(),
        3,
        "Н35: each failing path of the installation gives the claim back — a claim left behind \
         would refuse every later installation, the watchdog's included"
    );
    assert!(
        install.contains("UnhookWindowsHookEx(hook)"),
        "Н35: a cancelled claim takes the hook it has just registered back off — the caller \
         that asked for the removal was told there was nothing to remove"
    );

    // `uninstall`: no unconditional swap, and the claim is answered with the mark.
    assert!(
        !uninstall.contains("HOOK.swap("),
        "Н35: an unconditional swap would take a claim in flight out of the cell, and the \
         installer would then publish its handle into an empty one — the same window, from the \
         other side"
    );
    assert!(
        uninstall.contains("compare_exchange(INSTALLING, CANCELLED"),
        "Н35: a removal that meets a claim in flight marks it cancelled"
    );
    assert!(
        uninstall.contains("compare_exchange(handle, NO_HANDLE"),
        "and an installed hook is taken out by its own handle"
    );

    // NFR-04, FR-96, FR-98: nothing on either path may block.
    for forbidden in ["Mutex", "RwLock", "OnceLock", "sleep", "park", "yield_now"] {
        for (what, body) in [("install", install), ("uninstall", uninstall)] {
            assert!(
                !body.contains(forbidden),
                "{what} must not {forbidden}: it is called from inside the callback (FR-96) and \
                 from the panic hook (FR-98)"
            );
        }
    }
}

// -------------------------------------------------------------------------------------
// Task T-36-4 — a press that reached nobody is answered with a sound (finding Н40)
// -------------------------------------------------------------------------------------

/// **Both failure branches of `post_hotkey` ask for the idle tone — task T-36-4, finding Н40.**
///
/// # Why this test reads the source
///
/// `post_hotkey` is private, runs inside the callback and is reached only from
/// `keyboard_hook_proc`, which the system calls with a pointer no test can produce (the header of
/// this file says why no test installs a hook). Its two failure branches need a window that has
/// just been destroyed or a message queue that is full — neither is producible on demand. So what
/// is checked is the shape, which is also what a reviewer checks; the behaviour of the gate the
/// post has to pass is measured separately, as a pure function, in `src\app.rs`.
///
/// # What is asserted
///
/// Inside the body of `post_hotkey`: **two** counted failures, **two** calls to
/// `answer_lost_hotkey`, and that the function it calls posts `WM_APP_SOUND_IDLE` — the idle tone
/// and not `WM_APP_SOUND_REFUSED`, which task Т-49-2 gave to refusals the program *decides*
/// (FR-70, FR-84). A lost press is not a decision.
///
/// The needles are identifiers with their opening bracket, never whole call lines: a line splits
/// when `rustfmt` reflows it and a needle written as one would stop matching without anything
/// having changed (the trap of stage Э32). The positive control is below — each needle is
/// counted, and a count of zero fails.
#[test]
fn a_lost_hotkey_press_is_answered_with_the_idle_tone() {
    // ⚠ Normalised first, for the reason `the_capslock_seed_is_outside_the_callback` states:
    // `.gitattributes` makes the canonical checkout CRLF, and a verdict about the shape of a
    // function must not depend on how a checkout stored its newlines.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable")
    .replace("\r\n", "\n");

    let post_hotkey = source
        .split_once("fn post_hotkey() {")
        .expect("post_hotkey must be in this file")
        .1
        .split_once("\n}\n")
        .expect("the body must end")
        .0;

    let count = |haystack: &str, needle: &str| haystack.matches(needle).count();

    let lost_counted = count(post_hotkey, "LOST_HOTKEYS.fetch_add(");
    let answered = count(post_hotkey, "answer_lost_hotkey(");
    let post_failures = count(post_hotkey, "POST_FAILURES.fetch_add(");

    println!(
        "post_hotkey: POST_FAILURES x{post_failures}, LOST_HOTKEYS x{lost_counted}, \
         answer_lost_hotkey x{answered}"
    );

    assert_eq!(
        post_failures, 2,
        "the two failure branches of the handoff — no window, and a refused post"
    );
    assert_eq!(
        lost_counted, 2,
        "Н40: each of them is a press the user made that reached nobody, and each is counted \
         apart from the shared POST_FAILURES"
    );
    assert_eq!(
        answered, 2,
        "Н40: and each of them answers with a sound — silence is what makes a lost press \
         indistinguishable from a broken program"
    );

    // The tone itself, in the function those two calls reach.
    let answer = source
        .split_once("fn answer_lost_hotkey() {")
        .expect("answer_lost_hotkey must be in this file")
        .1
        .split_once("\n}\n")
        .expect("the body must end")
        .0;

    assert_eq!(
        count(answer, "WM_APP_SOUND_IDLE"),
        1,
        "the idle tone, once: a press that reached nobody did nothing, which is what «нечего \
         конвертировать» sounds like — not WM_APP_SOUND_REFUSED, which Т-49-2 gave to the \
         refusals this program decides (FR-70, FR-84)"
    );
    assert_eq!(
        count(answer, "WM_APP_SOUND_REFUSED"),
        0,
        "and not the refusal tone"
    );

    // NFR-01…NFR-05 on the path this task touched: one atomic load, one PostMessageW, and
    // nothing that allocates, locks or journals.
    for forbidden in [
        "format!",
        "to_string",
        "String::",
        "Vec::",
        "Mutex",
        "OnceLock",
        "crate::diag",
    ] {
        assert_eq!(
            count(answer, forbidden),
            0,
            "NFR-01…NFR-05: {forbidden} has no place on the callback's path"
        );
    }
    assert_eq!(
        count(answer, "PostMessageW("),
        1,
        "exactly one Win32 call, and it is the one NFR-05 allows"
    );
}

// -------------------------------------------------------------------------------------
// Task T-36-3 — the belief about a held hotkey is cleared by the reason, not by the clock
// (finding Н39, variant 1)
// -------------------------------------------------------------------------------------

/// **The whole table of `watchdog::clears_hotkey_state` — five reasons × two × two.**
///
/// FR-80 puts the hook back every thirty seconds whether anything happened or not (§10 п. 11, an
/// accepted price), and until task T-36-3 every one of those ticks cleared the program's belief
/// that the hotkey was held. That belief is what FR-08 answers "press or auto-repeat?" from, so a
/// user holding the key across a tick had the hold forgotten under their finger: the next repeat
/// arrived as a first press and the word was converted a second time. The opposite mistake is the
/// one task Т-22-1 repaired (finding м2): coming back from a real absence still believing the key
/// is down loses the next press entirely.
///
/// The rule is therefore "forget when there really was an absence". Only the planned tick that
/// found its own hook standing keeps the belief — in that one case the hook was up throughout and
/// every release reached the callback.
///
/// Written as a table rather than as four cases in prose so that the count is visible: twenty
/// rows, and the single `Keep` among them is the one situation in which nothing can have been
/// missed.
#[test]
fn clearing_the_held_hotkey_is_decided_by_the_reason_and_not_by_the_timer() {
    use lang_switcher::hook::HotkeyMemory::{Forget, Keep};
    use lang_switcher::watchdog::{Reason, clears_hotkey_state};

    let table = [
        // reason,              believed_installed, silently_removed, expected
        (Reason::None, true, true, Forget),
        (Reason::None, true, false, Forget),
        (Reason::None, false, true, Forget),
        (Reason::None, false, false, Forget),
        // ⭐ The one row that keeps: the planned tick of FR-80 that found its own hook standing
        // and took it off itself. Microseconds, on this very thread, with no stroke in between.
        (Reason::Timer, true, false, Keep),
        // The same planned tick, but the system had taken the hook away behind the program's
        // back — the case FR-80 exists for, and a stretch in which a release could be missed.
        (Reason::Timer, true, true, Forget),
        // The program already knew it had no hook.
        (Reason::Timer, false, true, Forget),
        (Reason::Timer, false, false, Forget),
        (Reason::DesktopSwitch, true, true, Forget),
        (Reason::DesktopSwitch, true, false, Forget),
        (Reason::DesktopSwitch, false, true, Forget),
        (Reason::DesktopSwitch, false, false, Forget),
        (Reason::SessionChange, true, true, Forget),
        (Reason::SessionChange, true, false, Forget),
        (Reason::SessionChange, false, true, Forget),
        (Reason::SessionChange, false, false, Forget),
        (Reason::PowerResume, true, true, Forget),
        (Reason::PowerResume, true, false, Forget),
        (Reason::PowerResume, false, true, Forget),
        (Reason::PowerResume, false, false, Forget),
    ];

    assert_eq!(
        table.len(),
        20,
        "five reasons, each with both flags both ways"
    );

    for (reason, believed, silently, expected) in table {
        assert_eq!(
            clears_hotkey_state(reason, believed, silently),
            expected,
            "Н39: reason {reason:?}, believed_installed {believed}, silently_removed {silently}"
        );
    }

    assert_eq!(
        table.iter().filter(|row| row.3 == Keep).count(),
        1,
        "exactly one situation keeps the belief — the planned tick whose hook was standing; \
         any other count means the rule has drifted into forgetting too much or too little"
    );
}

// -------------------------------------------------------------------------------------
// Task T-36-1 — a CapsLock probe that can fail (finding С2, variant 3)
// -------------------------------------------------------------------------------------

/// **The reading of `CapsLock` has to follow the toggle — task T-36-1.**
///
/// # Why the existing test could not answer this
///
/// [`the_resumption_of_fr90_seeds_the_tracker_from_the_reading_of_the_machine`] checks
/// `hook::caps_lock_on()` against **itself**: it reads the function, puts the opposite into the
/// tracker, and demands that the seed bring the tracker back to what that same call answered. A
/// function that always answered `false` would satisfy every line of it, which is exactly what
/// finding С2 of the audit of 2026-09-04 suspects — `GetKeyState` answers "as of the last
/// keyboard message this thread dispatched", and the input thread dispatches none. An instrument
/// that cannot fail is not a measurement (`bad-instruments-list`).
///
/// # What is the reference here
///
/// **The cause, not a second reading of the same place.** This test moves the toggle itself with
/// `SendInput` and counts how many times it moved it: one move must invert the answer and two
/// must bring it back. A `caps_lock_on` stuck on any constant fails the first assertion, and so
/// does one that reads a state frozen at thread start.
///
/// The starting point is one reading of the function — there is nothing else on the machine to
/// anchor to — but nothing is asserted **about** it: what is asserted is the difference the tap
/// makes, and a constant has no difference.
///
/// # What it touches and gives back
///
/// The `CapsLock` of whoever is running the tests, which is why it is `#[ignore]`. The guard
/// restores the toggle even if an assertion unwinds, and the restoration is verified on the way
/// out as well as recorded on the way in — a probe that moves a person's setting checks it at
/// both ends.
///
/// ⛔ No `WH_KEYBOARD_LL` is installed: the header of this file says why, and nothing here needs
/// one. `SendInput` is real input, so it resets `GetLastInputInfo` (FR-101's schedule) and any
/// live copy of the product hears the tap — run with the product stopped where that matters.
#[test]
#[ignore = "moves the machine's CapsLock; run deliberately with --ignored --test-threads=1"]
fn the_capslock_reading_follows_the_toggle_this_test_moves() {
    /// Puts the toggle back where it was found, whatever happened in between.
    struct Toggle {
        /// How the machine was found, as the product's own reading saw it.
        at_entry: bool,
        /// Taps this test has issued. Odd means the machine is inverted right now.
        taps: u32,
    }

    impl Drop for Toggle {
        fn drop(&mut self) {
            if self.taps % 2 == 1 {
                tap_capslock();
            }
        }
    }

    let mut toggle = Toggle {
        at_entry: hook::caps_lock_on(),
        taps: 0,
    };

    tap_capslock();
    toggle.taps += 1;
    assert_eq!(
        hook::caps_lock_on(),
        !toggle.at_entry,
        "one tap of CapsLock must change what the program reads (finding С2): entry was {}",
        toggle.at_entry
    );

    tap_capslock();
    toggle.taps += 1;
    assert_eq!(
        hook::caps_lock_on(),
        toggle.at_entry,
        "two taps must bring the reading back to where it started"
    );

    // The machine is already back at `at_entry` here — the guard has nothing to do — and this
    // is the exit check the rule asks for: entry and exit are both stated, not assumed.
    assert_eq!(
        hook::caps_lock_on(),
        toggle.at_entry,
        "the machine is given back as it was found"
    );
}

/// One press and release of `CapsLock`, as the machine's own input — the reference of
/// [`the_capslock_reading_follows_the_toggle_this_test_moves`].
///
/// Written with the plain signature (`dwExtraInfo` zero): this is deliberately **not** the
/// program's own injection, which carries `hook::INJECTED_SIGNATURE`, because the point is to
/// move the real toggle the way a person's finger does.
fn tap_capslock() {
    use std::{thread::sleep, time::Duration};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_CAPITAL,
    };

    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_CAPITAL,
                ..Default::default()
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VK_CAPITAL,
                dwFlags: KEYEVENTF_KEYUP,
                ..Default::default()
            },
        },
    };

    let events = [down, up];

    // SAFETY: `SendInput` reads the slice it is given and copies the events into the system's
    // input queue; it dereferences nothing of ours afterwards, and the size argument is the one
    // the structure really has. The return value is the count accepted — examined below rather
    // than discarded (NFR-13), because a refusal (UIPI, a wrong size) is silent otherwise and
    // would turn this whole test into a green nothing.
    let sent = unsafe { SendInput(&events, size_of::<INPUT>() as i32) };

    assert_eq!(
        sent as usize,
        events.len(),
        "SendInput refused the tap: {} of {} events accepted — the probe measured nothing",
        sent,
        events.len()
    );

    // The toggle is the system's state, set while the events are dispatched; the reading that
    // follows has to happen after that, and this is the shortest wait that is still a wait.
    sleep(Duration::from_millis(120));
}

// -------------------------------------------------------------------------------------
// Task T-93-1, вопрос 157 — the hotkey with modifiers: `Ctrl`/`Alt`/`Shift` + a key
// -------------------------------------------------------------------------------------
//
// The owner's word (157.1, 157.6): a combination of `Ctrl`, `Alt` and `Shift` with a key is a
// lawful hotkey, and it answers **only to its own set** — `Ctrl+F12` neither to `F12`, nor to
// `Ctrl+Shift+F12`, nor with a `Win` held; the bare key goes to the application. The set held is
// the fourth argument of `classify`, so every row below is a pure call with the set written out.

/// Virtual-key code of `F12` — the key of the emergency combination, and the one the owner's
/// laptop example is about (`Ctrl+F12` instead of a `Pause` it does not have).
const VK_F12: u16 = 0x7B;

/// The ordinary running state with `Ctrl+F12` as the hotkey.
fn armed_on_ctrl_f12() -> Mode {
    Mode {
        hotkey_vk: VK_F12,
        hotkey_modifiers: hook::MOD_CTRL,
        ..armed()
    }
}

/// `hook::classify` with the set `held` written out — the tests of this section.
fn classify_holding(mode: Mode, state: &mut HotkeyState, key: KeyEvent, held: u16) -> Outcome {
    hook::classify(mode, state, key, || held, || 0)
}

/// **(а) The combination pressed with its own set is the hotkey**: suppressed (FR-95) and the
/// conversion asked for (FR-02) — and its release is suppressed too, so the application sees
/// nothing of the keystroke.
#[test]
fn a_combination_pressed_with_its_own_set_is_the_hotkey() {
    let mut state = HotkeyState::default();

    let pressed = classify_holding(
        armed_on_ctrl_f12(),
        &mut state,
        user_key(VK_F12, Edge::Down),
        hook::MOD_CTRL,
    );

    assert_eq!(
        pressed.decision,
        Decision::Suppress,
        "Ctrl+F12 должна подавляться"
    );
    assert!(pressed.fire_hotkey, "Ctrl+F12 должна запускать конвертацию");

    let released = classify_holding(
        armed_on_ctrl_f12(),
        &mut state,
        user_key(VK_F12, Edge::Up),
        hook::MOD_CTRL,
    );

    assert_eq!(released.decision, Decision::Suppress);
    assert!(!released.fire_hotkey);
    assert_eq!(state, HotkeyState::default(), "the hold is over");
}

/// **(б) The bare key of a combination reaches the application** — вопрос 157: «голая клавиша
/// при назначенном сочетании проходит в программы». Its release goes the same way, so the
/// application sees a whole keystroke.
#[test]
fn the_bare_key_of_a_combination_reaches_the_application() {
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Down, Edge::Up] {
        let outcome = classify_holding(armed_on_ctrl_f12(), &mut state, user_key(VK_F12, edge), 0);

        assert_eq!(
            outcome,
            passed_on(),
            "голая F12 при горячей клавише Ctrl+F12 должна уйти программе, {edge:?}"
        );
    }

    assert_eq!(state, HotkeyState::default(), "and nothing is left held");
}

/// **(в) Another set is not the hotkey**: a larger one, a different one, and the set with a
/// `Win` in it all reach the application — the exactness of вопрос 157 is the whole of it.
#[test]
fn a_combination_with_another_set_held_reaches_the_application() {
    for (held, what) in [
        (hook::MOD_CTRL | hook::MOD_SHIFT, "Ctrl+Shift+F12"),
        (hook::MOD_CTRL | hook::MOD_WIN, "Win+Ctrl+F12"),
        (hook::MOD_ALT, "Alt+F12"),
        (hook::MOD_SHIFT, "Shift+F12"),
        (hook::MOD_CTRL | hook::MOD_ALT, "Ctrl+Alt+F12"),
    ] {
        let mut state = HotkeyState::default();

        let pressed = classify_holding(
            armed_on_ctrl_f12(),
            &mut state,
            user_key(VK_F12, Edge::Down),
            held,
        );
        let released = classify_holding(
            armed_on_ctrl_f12(),
            &mut state,
            user_key(VK_F12, Edge::Up),
            held,
        );

        assert_eq!(
            pressed,
            passed_on(),
            "{what} при горячей клавише Ctrl+F12 должна уйти программе"
        );
        assert_eq!(released, passed_on(), "{what}: и её отпускание тоже");
    }
}

/// **The set is decided on the press and kept for the whole hold** — the shape FR-84 gave the
/// fate of a press (task T-52-3), and for the same reason: an application must see a whole
/// keystroke or none of it.
///
/// Two ways the modifiers move during a hold: the `Ctrl` of a fired `Ctrl+F12` let go before
/// `F12` — the rest of the hold is still the hotkey's, swallowed, and the next press fires again;
/// and a `Ctrl` pressed while a bare `F12` repeats — the hold stays the application's to the end,
/// with no conversion in the middle of it and no release swallowed under it.
#[test]
fn the_set_of_a_press_is_kept_for_the_whole_hold() {
    let mut state = HotkeyState::default();
    let mode = armed_on_ctrl_f12();

    let fired = classify_holding(
        mode,
        &mut state,
        user_key(VK_F12, Edge::Down),
        hook::MOD_CTRL,
    );
    assert!(fired.fire_hotkey);

    // The `Ctrl` goes up first: the repeats and the release of `F12` arrive with nothing held.
    let repeat = classify_holding(mode, &mut state, user_key(VK_F12, Edge::Down), 0);
    let release = classify_holding(mode, &mut state, user_key(VK_F12, Edge::Up), 0);

    assert_eq!(
        (repeat.decision, repeat.fire_hotkey),
        (Decision::Suppress, false),
        "the repeat of a fired hold is swallowed and converts nothing (FR-08, FR-95)"
    );
    assert_eq!(
        release.decision,
        Decision::Suppress,
        "the release of a swallowed press must not reach the application"
    );

    // And the next press with the set is a press again.
    let again = classify_holding(
        mode,
        &mut state,
        user_key(VK_F12, Edge::Down),
        hook::MOD_CTRL,
    );
    assert!(
        again.fire_hotkey,
        "the hotkey fires again after the release"
    );
    classify_holding(mode, &mut state, user_key(VK_F12, Edge::Up), hook::MOD_CTRL);

    // A bare `F12` held, and the `Ctrl` pressed while it repeats.
    let bare = classify_holding(mode, &mut state, user_key(VK_F12, Edge::Down), 0);
    let repeat_with_ctrl = classify_holding(
        mode,
        &mut state,
        user_key(VK_F12, Edge::Down),
        hook::MOD_CTRL,
    );
    let release_with_ctrl =
        classify_holding(mode, &mut state, user_key(VK_F12, Edge::Up), hook::MOD_CTRL);

    assert_eq!(bare, passed_on());
    assert_eq!(
        repeat_with_ctrl,
        passed_on(),
        "a hold the application got does not become the hotkey half way"
    );
    assert_eq!(
        release_with_ctrl,
        passed_on(),
        "and its release is the application's"
    );
    assert_eq!(state, HotkeyState::default());
}

/// **(д) FR-08 for a combination**: a held `Ctrl+F12` converts once however long it is held, and
/// the set is asked **once** — on the first press — not on every repeat.
#[test]
fn holding_a_combination_fires_once_and_asks_for_the_set_once() {
    let mut state = HotkeyState::default();
    let asked = std::cell::Cell::new(0_u32);

    let fired = (0..10)
        .filter(|_| {
            let outcome = hook::classify(
                armed_on_ctrl_f12(),
                &mut state,
                user_key(VK_F12, Edge::Down),
                || {
                    asked.set(asked.get() + 1);
                    hook::MOD_CTRL
                },
                || 0,
            );

            assert_eq!(
                outcome.decision,
                Decision::Suppress,
                "every repeat is swallowed"
            );

            outcome.fire_hotkey
        })
        .count();

    assert_eq!(fired, 1, "FR-08: one conversion, not a stream of them");
    assert_eq!(asked.get(), 1, "the set is asked on the first press only");
}

/// **NFR-01 and NFR-02 — an ordinary letter costs no question about the modifiers, and neither
/// does a bare hotkey.** The system calls behind the set (`hook::physical_modifiers`, seven reads
/// of the asynchronous key state) are made for the first press of the key of a **combination**
/// and for nothing else — the shape FR-96 already has: the cheap comparison first, the system
/// call only when the answer is almost «yes».
///
/// ⚠ The negative control of this guard is a mutant that asks first and compares after
/// (`scratchpad-E93`): it turns the first count red.
#[test]
fn neither_a_letter_nor_a_bare_hotkey_asks_for_the_modifiers() {
    let asked = std::cell::Cell::new(0_u32);
    let counting = || {
        asked.set(asked.get() + 1);
        0
    };

    // Every letter and digit, both edges, with `Ctrl+F12` as the hotkey.
    let mut state = HotkeyState::default();

    for vk in (0x30..=0x39).chain(0x41..=0x5A) {
        for edge in [Edge::Down, Edge::Up] {
            hook::classify(
                armed_on_ctrl_f12(),
                &mut state,
                user_key(vk, edge),
                counting,
                || 0,
            );
        }
    }

    assert_eq!(
        asked.get(),
        0,
        "a letter asked the system about the modifiers"
    );

    // The bare hotkey of section 7 — `Pause` — pressed, repeated and released.
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Down, Edge::Up] {
        hook::classify(
            armed(),
            &mut state,
            user_key(VK_PAUSE, edge),
            counting,
            || 0,
        );
    }

    assert_eq!(
        asked.get(),
        0,
        "a bare hotkey asked the system about the modifiers"
    );
}

/// **A bare hotkey answers with any modifiers held** — 157.11, variant А (до слова владельца):
/// `Shift` held over `Pause` is position 22 of the matrix of §11.3 («результат без искажений»),
/// and step 3 of FR-40 exists for exactly that press. The exactness of вопрос 157 is the
/// combinations' rule.
#[test]
fn a_bare_hotkey_answers_with_any_modifier_held() {
    for held in [
        0,
        hook::MOD_SHIFT,
        hook::MOD_CTRL,
        hook::MOD_ALT,
        hook::MOD_CTRL | hook::MOD_ALT | hook::MOD_SHIFT,
        hook::MOD_WIN,
    ] {
        let mut state = HotkeyState::default();
        let outcome = classify_holding(armed(), &mut state, user_key(VK_PAUSE, Edge::Down), held);

        assert_eq!(outcome.decision, Decision::Suppress, "held {held:#06b}");
        assert!(outcome.fire_hotkey, "held {held:#06b}");
    }
}

/// **The sides are one modifier** — вопрос 157: left and right `Ctrl`, `Alt` and `Shift` are the
/// same, `AltGr` (the right `Alt` with the `Ctrl` the keyboard adds) reads as `Ctrl + Alt`, and a
/// `Win` of either side is the bit no hotkey carries.
#[test]
fn the_held_set_merges_the_sides_and_names_the_windows_key() {
    use lang_switcher::buffer::Physical;

    let nothing = Physical {
        ctrl: false,
        alt_left: false,
        alt_right: false,
        win: false,
        shift_left: false,
        shift_right: false,
    };

    assert_eq!(hook::held_set(nothing), 0);
    assert_eq!(
        hook::held_set(Physical {
            ctrl: true,
            ..nothing
        }),
        hook::MOD_CTRL
    );
    assert_eq!(
        hook::held_set(Physical {
            alt_left: true,
            ..nothing
        }),
        hook::MOD_ALT
    );
    assert_eq!(
        hook::held_set(Physical {
            alt_right: true,
            ..nothing
        }),
        hook::MOD_ALT
    );
    assert_eq!(
        hook::held_set(Physical {
            shift_left: true,
            ..nothing
        }),
        hook::MOD_SHIFT
    );
    assert_eq!(
        hook::held_set(Physical {
            shift_right: true,
            ..nothing
        }),
        hook::MOD_SHIFT
    );
    assert_eq!(
        hook::held_set(Physical {
            win: true,
            ..nothing
        }),
        hook::MOD_WIN
    );
    assert_eq!(
        hook::held_set(Physical {
            ctrl: true,
            alt_right: true,
            ..nothing
        }),
        hook::MOD_CTRL | hook::MOD_ALT,
        "AltGr is Ctrl + Alt"
    );
}

/// **The key and its modifiers are one publication** — вопрос 157, task T-93-1: one word, one
/// store, one load, so no reader ever pairs a new key with the old set.
///
/// Measured twice: through the door (`set_hotkey` → `hotkey` → `current_mode`) and in the source,
/// where the one word has exactly one writer.
#[test]
fn the_key_and_its_modifiers_are_published_as_one_word() {
    let restore = hook::hotkey();

    hook::set_hotkey(VK_F12, hook::MOD_CTRL | hook::MOD_SHIFT);
    assert_eq!(hook::hotkey(), (VK_F12, hook::MOD_CTRL | hook::MOD_SHIFT));
    assert_eq!(hook::hotkey_vk(), VK_F12);

    let mode = hook::current_mode();
    assert_eq!(
        (mode.hotkey_vk, mode.hotkey_modifiers),
        (VK_F12, hook::MOD_CTRL | hook::MOD_SHIFT),
        "current_mode reads both halves of the publication"
    );

    hook::set_hotkey(VK_PAUSE, 0);
    assert_eq!(
        hook::hotkey(),
        (VK_PAUSE, 0),
        "and a bare key clears the set"
    );

    hook::set_hotkey(restore.0, restore.1);

    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("hook.rs"),
    )
    .expect("src/hook.rs must be readable");
    let product = source
        .split("\n#[cfg(test)]\nmod tests {")
        .next()
        .expect("the product half");

    assert_eq!(
        product.matches("static HOTKEY: AtomicU32").count(),
        1,
        "one word holds the hotkey"
    );
    assert_eq!(
        product.matches("HOTKEY.store(").count(),
        1,
        "and it has one writer — `set_hotkey`"
    );
    assert_eq!(
        product.matches("HOTKEY.load(").count(),
        1,
        "and one reader — `hotkey`"
    );

    // And no second word beside it: a set published apart from the key is the race this test
    // exists against. Controls: the needles see a second atomic of the modifiers when one is
    // written into a copy of the source.
    let second_word = |text: &str| {
        text.matches("MODIFIERS.store(").count() + text.matches("MODIFIERS.load(").count()
    };
    assert_eq!(
        second_word(product),
        0,
        "the modifiers travel in the word of the key, never in a static of their own"
    );
    let mutant = product.replacen(
        "pub fn set_hotkey(vk: u16, modifiers: u16) {",
        "pub fn set_hotkey(vk: u16, modifiers: u16) { HOTKEY_MODIFIERS.store(modifiers, R);",
        1,
    );
    assert_ne!(
        mutant, product,
        "the control must change the text it is made of"
    );
    assert!(
        second_word(&mutant) > 0,
        "the sweep does not see a second atomic — it cannot fail"
    );
}

/// **The names of the modifiers in section 7** — a closed set, read the way key names are read:
/// any case, blanks around, any order; an unknown name is `None` for the whole list.
#[test]
fn the_names_of_the_modifiers_are_a_closed_set() {
    let none: [&str; 0] = [];

    assert_eq!(hook::modifiers_from_names(&none), Some(0));
    assert_eq!(hook::modifiers_from_names(&["Ctrl"]), Some(hook::MOD_CTRL));
    assert_eq!(
        hook::modifiers_from_names(&["shift", " CTRL "]),
        Some(hook::MOD_CTRL | hook::MOD_SHIFT),
        "any case, blanks around, any order"
    );
    assert_eq!(
        hook::modifiers_from_names(&["Ctrl", "Alt", "Shift"]),
        Some(hook::MOD_CTRL | hook::MOD_ALT | hook::MOD_SHIFT)
    );
    assert_eq!(
        hook::modifiers_from_names(&["Ctrl", "Ctrl"]),
        Some(hook::MOD_CTRL),
        "a name twice is the modifier once"
    );

    for unknown in ["Win", "Strg", "Control", "", "Ctrl+Alt"] {
        assert_eq!(
            hook::modifiers_from_names(&["Ctrl", unknown]),
            None,
            "«{unknown}» is not a modifier section 7 knows"
        );
    }

    assert_eq!(
        hook::HOTKEY_MODIFIERS.map(|(name, _)| name),
        ["Ctrl", "Alt", "Shift"],
        "the canonical order of вопрос 157"
    );
}

// The minimal layouts of the probe П1 — `ghbdtn` on both halves, so that a stroke recorded is a
// stroke counted.
const SCAN_G: u16 = 0x22;
const SCAN_H: u16 = 0x23;
const SCAN_B: u16 = 0x30;
const SCAN_D: u16 = 0x20;
const SCAN_T: u16 = 0x14;
const SCAN_N: u16 = 0x31;

fn ghbdtn_cache() -> lang_switcher::layouts::LayoutCache {
    use lang_switcher::layouts::{KeyMapping, LayoutCache, LayoutId, LayoutMapBuilder, Mods};

    let mut english = LayoutMapBuilder::new(LayoutId::from_raw(0x0409_0409));
    let mut russian = LayoutMapBuilder::new(LayoutId::from_raw(0x0419_0419));

    for (scan, latin, cyrillic) in [
        (SCAN_G, 'g', 'п'),
        (SCAN_H, 'h', 'р'),
        (SCAN_B, 'b', 'и'),
        (SCAN_D, 'd', 'в'),
        (SCAN_T, 't', 'е'),
        (SCAN_N, 'n', 'т'),
    ] {
        english.set(scan, false, Mods::NONE, KeyMapping::from_char(latin));
        russian.set(scan, false, Mods::NONE, KeyMapping::from_char(cyrillic));
    }

    LayoutCache::from_maps(vec![english.finish(), russian.finish()]).expect("two maps")
}

/// A stroke with its scan code, as the callback delivers one.
fn scanned(vk: u16, scan: u16, edge: Edge) -> KeyEvent {
    KeyEvent {
        scan,
        ..user_key(vk, edge)
    }
}

/// **(г) Посылка П1 — the hotkey's branch returns before the command row of FR-10.** The rule
/// «`Ctrl`/`Alt`/`Win` + клавиша — полный сброс» lives in `buffer::record`, which `classify`
/// reaches only for a stroke that is not the hotkey; so `Ctrl+F12` converts the word typed before
/// it instead of throwing it away. The contrast proves the order: `Ctrl+Shift+F12`, which is not
/// the hotkey, does reach the record and does flush the ring.
#[test]
fn a_combination_hotkey_never_reaches_the_command_row_of_fr10() {
    use lang_switcher::buffer::{self, Recorder};
    use lang_switcher::layouts::LayoutId;

    const VK_LCONTROL: u16 = 0xA2;
    const VK_LSHIFT: u16 = 0xA0;

    let typed = |held: &[u16], key_held: u16| -> (usize, Outcome, usize) {
        let mut recorder = Recorder::with_capacity(64);
        recorder.set_cache(ghbdtn_cache());
        recorder.set_active_layout(LayoutId::from_raw(0x0409_0409));
        buffer::install_recorder(recorder);

        let mode = armed_on_ctrl_f12();
        let mut state = HotkeyState::default();

        for (vk, scan) in [
            (b'G', SCAN_G),
            (b'H', SCAN_H),
            (b'B', SCAN_B),
            (b'D', SCAN_D),
            (b'T', SCAN_T),
            (b'N', SCAN_N),
        ] {
            classify_holding(mode, &mut state, scanned(vk.into(), scan, Edge::Down), 0);
            classify_holding(mode, &mut state, scanned(vk.into(), scan, Edge::Up), 0);
        }

        let before = buffer::len();

        for &modifier in held {
            classify_holding(mode, &mut state, user_key(modifier, Edge::Down), 0);
        }

        let pressed = classify_holding(mode, &mut state, user_key(VK_F12, Edge::Down), key_held);
        let after = buffer::len();

        buffer::uninstall();

        (before, pressed, after)
    };

    let (before, pressed, after) = typed(&[VK_LCONTROL], hook::MOD_CTRL);
    println!("Ctrl+F12 (the hotkey): ring {before} -> {after}, {pressed:?}");

    assert_eq!(before, 6, "the probe typed six strokes");
    assert!(pressed.fire_hotkey, "Ctrl+F12 starts the conversion");
    assert_eq!(
        after, 6,
        "П1: the word typed before the hotkey is still there to be converted"
    );

    let (before, pressed, after) =
        typed(&[VK_LCONTROL, VK_LSHIFT], hook::MOD_CTRL | hook::MOD_SHIFT);
    println!("Ctrl+Shift+F12 (not the hotkey): ring {before} -> {after}, {pressed:?}");

    assert_eq!(pressed, passed_on());
    assert_eq!(
        after, 0,
        "the contrast: a combination that is not the hotkey is an ordinary command of FR-10"
    );
}

// -------------------------------------------------------------------------------------
// Task T-95-1, вопрос 159 — the double press of `Shift`
// -------------------------------------------------------------------------------------
//
// The owner's word (157.16, 157.17): two taps of `Shift` in a row start what `Pause` starts; a
// tap is a `Shift` down and up within `DOUBLE_TAP_MS` with nothing else pressed — no key, no
// `Ctrl`/`Alt`/`Win` held at the press, no mouse button; the second tap begins within the window
// of the first, and the conversion fires on the **release** of the second. Nothing is suppressed:
// the programs see both taps. Every row below is a pure call of `classify` with the time, the set
// held and the count of mouse buttons written out.

/// The running state with the double press of `Shift` as the hotkey — what
/// `app::publish_configuration` publishes for `trigger = "double_tap"`: `VK_SHIFT` with the flag.
fn armed_on_double_shift() -> Mode {
    Mode {
        hotkey_vk: VK_SHIFT,
        hotkey_modifiers: 0,
        hotkey_double_tap: true,
        ..armed()
    }
}

/// One event of a sequence, at a time, with the set held and the mouse count the callback would
/// read: `(code, edge, time in ms, set held, mouse buttons so far)`.
type At = (u16, Edge, u32, u16, u32);

/// Drives `events` through `classify` in `mode` and answers the index of every event that fired,
/// with each outcome checked to be passed on — the double press suppresses nothing.
fn fired_in(mode: Mode, events: &[At]) -> Vec<usize> {
    let mut state = HotkeyState::default();

    events
        .iter()
        .enumerate()
        .filter_map(|(index, &(vk, edge, time, held, mouse))| {
            let outcome = hook::classify(
                mode,
                &mut state,
                KeyEvent {
                    time,
                    ..user_key(vk, edge)
                },
                || held,
                || mouse,
            );

            assert_eq!(
                outcome.decision,
                Decision::Pass,
                "event {index} ({vk:#04x} {edge:?} at {time}) must reach the application"
            );

            outcome.fire_hotkey.then_some(index)
        })
        .collect()
}

/// The events of the double press of `Shift` that fired.
fn fired(events: &[At]) -> Vec<usize> {
    fired_in(armed_on_double_shift(), events)
}

/// A tap of `key` from `down` to `up`, nothing held, no mouse.
fn tap(key: u16, down: u32, up: u32) -> [At; 2] {
    [(key, Edge::Down, down, 0, 0), (key, Edge::Up, up, 0, 0)]
}

/// Two taps of the left `Shift`, the second beginning at `second`.
fn two_taps(second: u32) -> Vec<At> {
    [
        tap(VK_LSHIFT, 1_000, 1_080),
        tap(VK_LSHIFT, second, second + 80),
    ]
    .concat()
}

/// **(а) Two taps within the window fire once, on the release of the second** — and every event
/// is passed on. ⭐ The red «before» of task T-95-1: on the base nothing ever fires here.
///
/// The moment is part of the claim: a fire on the second *press* (the mutant of the TZ) would leave
/// the person's finger on `Shift` while the conversion runs.
#[test]
fn two_taps_of_shift_fire_once_on_the_release_of_the_second() {
    assert_eq!(
        fired(&two_taps(1_200)),
        [3],
        "the second release, and only it"
    );

    // The window is the owner's 400 ms (157.16), to the millisecond: a second press 400 ms after
    // the first is in it.
    assert_eq!(
        fired(&two_taps(1_000 + hook::DOUBLE_TAP_MS)),
        [3],
        "a second tap that begins exactly 400 ms after the first is the pair"
    );
}

/// **(б) A slow pair is no pair**: the second tap begins 401 ms after the first.
#[test]
fn a_second_tap_beginning_after_the_window_is_no_pair() {
    assert_eq!(fired(&two_taps(1_401)), [] as [usize; 0], "401 ms");
    assert_eq!(
        fired(&two_taps(1_000 + 3_999)),
        [] as [usize; 0],
        "four seconds"
    );

    // ⚠ And the slow second tap is not lost: it is the first of the next pair.
    let mut events = two_taps(1_401);
    events.extend(tap(VK_LSHIFT, 1_600, 1_650));
    assert_eq!(
        fired(&events),
        [5],
        "the tap that came too late begins the next pair"
    );

    // The owner's number, written here apart from the code (157.16) — last, so that a mutant of
    // the window is caught by the behaviour above and not only by the number.
    assert_eq!(hook::DOUBLE_TAP_MS, 400, "the owner's number, 157.16");
}

/// **(в) A hold longer than the window is no tap** — of the first `Shift` or of the second.
#[test]
fn a_shift_held_longer_than_the_window_is_no_tap() {
    let long_first = [tap(VK_LSHIFT, 1_000, 1_401), tap(VK_LSHIFT, 1_402, 1_450)].concat();
    let long_second = [tap(VK_LSHIFT, 1_000, 1_050), tap(VK_LSHIFT, 1_100, 1_501)].concat();
    let exact = [tap(VK_LSHIFT, 1_000, 1_400), tap(VK_LSHIFT, 1_400, 1_800)].concat();

    assert_eq!(
        fired(&long_first),
        [] as [usize; 0],
        "the first held 401 ms"
    );
    assert_eq!(
        fired(&long_second),
        [] as [usize; 0],
        "the second held 401 ms"
    );
    assert_eq!(fired(&exact), [3], "400 ms of hold is still a tap");
}

/// **(г) Another key between the press and the release is no tap** — `Shift` + a letter is a
/// capital — and neither is a pair with a key between the taps: «подряд» (159.20).
#[test]
fn a_key_pressed_with_shift_or_between_the_taps_breaks_the_pair() {
    let capital_first = [
        (VK_LSHIFT, Edge::Down, 1_000, 0, 0),
        (VK_A, Edge::Down, 1_020, hook::MOD_SHIFT, 0),
        (VK_A, Edge::Up, 1_040, hook::MOD_SHIFT, 0),
        (VK_LSHIFT, Edge::Up, 1_060, 0, 0),
        (VK_LSHIFT, Edge::Down, 1_100, 0, 0),
        (VK_LSHIFT, Edge::Up, 1_150, 0, 0),
    ];
    assert_eq!(
        fired(&capital_first),
        [] as [usize; 0],
        "Shift+A, then a tap"
    );

    let capital_second = [
        tap(VK_LSHIFT, 1_000, 1_050).to_vec(),
        vec![
            (VK_LSHIFT, Edge::Down, 1_100, 0, 0),
            (VK_A, Edge::Down, 1_120, hook::MOD_SHIFT, 0),
            (VK_LSHIFT, Edge::Up, 1_150, 0, 0),
        ],
    ]
    .concat();
    assert_eq!(
        fired(&capital_second),
        [] as [usize; 0],
        "a tap, then Shift+A"
    );

    let between = [
        tap(VK_LSHIFT, 1_000, 1_050).to_vec(),
        tap(VK_A, 1_060, 1_070).to_vec(),
        tap(VK_LSHIFT, 1_100, 1_150).to_vec(),
    ]
    .concat();
    assert_eq!(
        fired(&between),
        [] as [usize; 0],
        "a letter between the taps"
    );

    // A phrase with capitals typed fast — «Hello World» — fires nothing.
    let mut phrase = Vec::new();
    let mut at = 1_000;
    for (vk, shifted) in [
        (0x48, true),
        (0x45, false),
        (0x4C, false),
        (0x4C, false),
        (0x4F, false),
        (0x20, false),
        (0x57, true),
        (0x4F, false),
        (0x52, false),
        (0x4C, false),
        (0x44, false),
    ] {
        if shifted {
            phrase.push((VK_LSHIFT, Edge::Down, at, 0, 0));
        }
        phrase.push((vk, Edge::Down, at + 10, 0, 0));
        phrase.push((vk, Edge::Up, at + 30, 0, 0));
        if shifted {
            phrase.push((VK_LSHIFT, Edge::Up, at + 40, 0, 0));
        }
        at += 60;
    }
    assert_eq!(fired(&phrase), [] as [usize; 0], "«Hello World»");
}

/// **(д) A mouse button pressed during a tap, or between the taps, breaks the pair** — `Shift` and
/// a click select text, and two of them must not convert the selection (FR-13's count).
///
/// ⚠ The negative control of this sentry is the mutant of the TZ that removes the comparison of the
/// count (`scratchpad-E95`): it fires on every row here.
#[test]
fn a_mouse_button_pressed_in_the_pair_breaks_it() {
    let click_in_the_first = [
        (VK_LSHIFT, Edge::Down, 1_000, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_080, 0, 8),
        (VK_LSHIFT, Edge::Down, 1_200, 0, 8),
        (VK_LSHIFT, Edge::Up, 1_280, 0, 8),
    ];
    let click_in_the_second = [
        (VK_LSHIFT, Edge::Down, 1_000, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_080, 0, 7),
        (VK_LSHIFT, Edge::Down, 1_200, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_280, 0, 8),
    ];
    let click_between = [
        (VK_LSHIFT, Edge::Down, 1_000, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_080, 0, 7),
        (VK_LSHIFT, Edge::Down, 1_200, 0, 8),
        (VK_LSHIFT, Edge::Up, 1_280, 0, 8),
    ];
    let still = [
        (VK_LSHIFT, Edge::Down, 1_000, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_080, 0, 7),
        (VK_LSHIFT, Edge::Down, 1_200, 0, 7),
        (VK_LSHIFT, Edge::Up, 1_280, 0, 7),
    ];

    assert_eq!(
        fired(&click_in_the_first),
        [] as [usize; 0],
        "a click in the first tap"
    );
    assert_eq!(
        fired(&click_in_the_second),
        [] as [usize; 0],
        "a click in the second"
    );
    assert_eq!(
        fired(&click_between),
        [] as [usize; 0],
        "a click between them"
    );
    assert_eq!(
        fired(&still),
        [3],
        "the control: the same pair with the mouse still fires"
    );
}

/// **(е) `Ctrl`, `Alt` or `Win` held at the press of `Shift` is no tap** — `Alt+Shift` twice
/// switches the layout twice and must convert nothing.
///
/// ⚠ The negative control is the mutant that drops the question about the set held.
#[test]
fn shift_pressed_under_ctrl_alt_or_win_is_no_tap() {
    for (held, what) in [
        (hook::MOD_ALT, "Alt+Shift"),
        (hook::MOD_CTRL, "Ctrl+Shift"),
        (hook::MOD_WIN, "Win+Shift"),
    ] {
        let both = [
            (VK_LSHIFT, Edge::Down, 1_000, held, 0),
            (VK_LSHIFT, Edge::Up, 1_080, 0, 0),
            (VK_LSHIFT, Edge::Down, 1_200, held, 0),
            (VK_LSHIFT, Edge::Up, 1_280, 0, 0),
        ];
        let second = [
            (VK_LSHIFT, Edge::Down, 1_000, 0, 0),
            (VK_LSHIFT, Edge::Up, 1_080, 0, 0),
            (VK_LSHIFT, Edge::Down, 1_200, held, 0),
            (VK_LSHIFT, Edge::Up, 1_280, 0, 0),
        ];

        assert_eq!(fired(&both), [] as [usize; 0], "{what} twice");
        assert_eq!(fired(&second), [] as [usize; 0], "a tap, then {what}");
    }

    // The control: `Shift` held by itself is in the set held (the press of `Shift` holds it), and
    // it does not spoil the tap.
    let shift_itself = [
        (VK_LSHIFT, Edge::Down, 1_000, hook::MOD_SHIFT, 0),
        (VK_LSHIFT, Edge::Up, 1_080, 0, 0),
        (VK_LSHIFT, Edge::Down, 1_200, hook::MOD_SHIFT, 0),
        (VK_LSHIFT, Edge::Up, 1_280, 0, 0),
    ];
    assert_eq!(fired(&shift_itself), [3]);
}

/// **(ж) The left and the right `Shift` are one key** — a pair of one of each fires — and the
/// auto-repeat of a held `Shift` is no new press.
#[test]
fn left_and_right_shift_make_one_pair_and_a_repeat_is_no_new_press() {
    let left_right = [tap(VK_LSHIFT, 1_000, 1_080), tap(VK_RSHIFT, 1_200, 1_280)].concat();
    let right_left = [tap(VK_RSHIFT, 1_000, 1_080), tap(VK_LSHIFT, 1_200, 1_280)].concat();

    assert_eq!(fired(&left_right), [3], "left, then right");
    assert_eq!(fired(&right_left), [3], "right, then left");

    // A held `Shift` repeats as presses with no release between them: still one tap, and the set
    // is asked once for it.
    let asked = std::cell::Cell::new(0_u32);
    let mut state = HotkeyState::default();
    let mut fires = Vec::new();

    for (index, (edge, time)) in [
        (Edge::Down, 1_000),
        (Edge::Down, 1_030),
        (Edge::Down, 1_060),
        (Edge::Up, 1_090),
        (Edge::Down, 1_200),
        (Edge::Up, 1_250),
    ]
    .into_iter()
    .enumerate()
    {
        let outcome = hook::classify(
            armed_on_double_shift(),
            &mut state,
            KeyEvent {
                time,
                ..user_key(VK_LSHIFT, edge)
            },
            || {
                asked.set(asked.get() + 1);
                0
            },
            || 0,
        );

        if outcome.fire_hotkey {
            fires.push(index);
        }
    }

    assert_eq!(fires, [5], "the repeats were one tap");
    assert_eq!(
        asked.get(),
        2,
        "the set is asked once per press, not per repeat"
    );
}

/// **(з) The third and fourth taps are the second pair** — the return of the word, as a second
/// `Pause` would be — and a third tap alone is no second fire.
#[test]
fn the_third_and_fourth_taps_are_the_next_pair() {
    let three = [
        tap(VK_LSHIFT, 1_000, 1_050),
        tap(VK_LSHIFT, 1_150, 1_200),
        tap(VK_LSHIFT, 1_300, 1_350),
    ]
    .concat();
    let four = [
        tap(VK_LSHIFT, 1_000, 1_050),
        tap(VK_LSHIFT, 1_150, 1_200),
        tap(VK_LSHIFT, 1_300, 1_350),
        tap(VK_LSHIFT, 1_450, 1_500),
    ]
    .concat();

    assert_eq!(fired(&three), [3], "the pair, and a third tap waiting");
    assert_eq!(fired(&four), [3, 7], "two pairs, two fires");
}

/// **(и) Our own injected strokes are not seen** — посылка П2: the signature of FR-03 is answered
/// before the hotkey, so a stroke of the replacement neither breaks a pair nor makes one.
#[test]
fn our_own_strokes_neither_break_a_pair_nor_make_one() {
    let mut state = HotkeyState::default();
    let mode = armed_on_double_shift();
    let mut fires = Vec::new();

    let user = |vk, edge, time| KeyEvent {
        time,
        ..user_key(vk, edge)
    };
    let ours = |vk, edge, time| KeyEvent {
        extra_info: INJECTED_SIGNATURE,
        time,
        ..user_key(vk, edge)
    };

    for (index, key) in [
        user(VK_LSHIFT, Edge::Down, 1_000),
        user(VK_LSHIFT, Edge::Up, 1_050),
        ours(VK_A, Edge::Down, 1_060),
        ours(VK_A, Edge::Up, 1_061),
        ours(VK_LSHIFT, Edge::Down, 1_070),
        ours(VK_LSHIFT, Edge::Up, 1_071),
        user(VK_LSHIFT, Edge::Down, 1_150),
        user(VK_LSHIFT, Edge::Up, 1_200),
        ours(VK_LSHIFT, Edge::Down, 1_300),
        ours(VK_LSHIFT, Edge::Up, 1_301),
        ours(VK_LSHIFT, Edge::Down, 1_302),
        ours(VK_LSHIFT, Edge::Up, 1_303),
    ]
    .into_iter()
    .enumerate()
    {
        if hook::classify(mode, &mut state, key, || 0, || 0).fire_hotkey {
            fires.push(index);
        }
    }

    assert_eq!(
        fires,
        [7],
        "the user's pair fires through our strokes, and our own taps fire nothing"
    );
}

/// **(к) While the hotkey is pressed (`trigger = "press"`), taps of `Shift` do nothing** — stage 1
/// is whole: no fire, and the state of the double press is not even touched.
#[test]
fn with_a_pressed_hotkey_taps_of_shift_do_nothing() {
    for mode in [armed(), armed_on_ctrl_f12()] {
        let mut state = HotkeyState::default();

        for (vk, edge, time, held, _) in two_taps(1_200) {
            let outcome = hook::classify(
                mode,
                &mut state,
                KeyEvent {
                    time,
                    ..user_key(vk, edge)
                },
                || held,
                || panic!("a pressed hotkey must never ask for the mouse"),
            );

            assert!(!outcome.fire_hotkey, "{mode:?}: a tap of Shift fired");
        }

        assert_eq!(
            state,
            HotkeyState::default(),
            "{mode:?}: the state of the double press was touched"
        );
    }
}

/// **(л) With the double press as the hotkey, a bare `Pause` is an ordinary key** — passed on,
/// converting nothing, asking nothing.
#[test]
fn with_the_double_press_pause_is_an_ordinary_key() {
    let mut state = HotkeyState::default();

    for edge in [Edge::Down, Edge::Down, Edge::Up] {
        let outcome = hook::classify(
            armed_on_double_shift(),
            &mut state,
            user_key(VK_PAUSE, edge),
            || panic!("Pause asked for the modifiers"),
            || panic!("Pause asked for the mouse"),
        );

        assert_eq!(outcome, passed_on(), "Pause {edge:?}");
    }

    // ⚠ And the neutral `VK_SHIFT` the word carries is no key of a hotkey either: a program that
    // sends one is not swallowed.
    let neutral = hook::classify(
        armed_on_double_shift(),
        &mut state,
        user_key(VK_SHIFT, Edge::Down),
        || 0,
        || 0,
    );
    assert_eq!(
        neutral.decision,
        Decision::Pass,
        "VK_SHIFT is not swallowed"
    );
}

/// **FR-84 — in an excluded process the double press converts nothing**, as `Pause` does not
/// there (task T-52-3): the keys are the application's.
#[test]
fn in_an_excluded_process_the_double_press_converts_nothing() {
    let excluded = Mode {
        hotkey_yields: true,
        ..armed_on_double_shift()
    };

    assert_eq!(fired_in(excluded, &two_taps(1_200)), [] as [usize; 0]);
}

/// **NFR-01 — with the double press as the hotkey a letter asks for nothing**, and a release of
/// `Shift` that cannot end a tap does not read the mouse.
#[test]
fn with_the_double_press_a_letter_asks_for_nothing() {
    let asked = std::cell::Cell::new(0_u32);
    let mut state = HotkeyState::default();

    for vk in (0x30..=0x39).chain(0x41..=0x5A) {
        for edge in [Edge::Down, Edge::Up] {
            hook::classify(
                armed_on_double_shift(),
                &mut state,
                user_key(vk, edge),
                || {
                    asked.set(asked.get() + 1);
                    0
                },
                || {
                    asked.set(asked.get() + 1);
                    0
                },
            );
        }
    }

    assert_eq!(asked.get(), 0, "a letter asked the system something");

    // `Alt+Shift`: the press is no tap, so its release has no pair to check the mouse for.
    let mouse = std::cell::Cell::new(0_u32);
    let mut state = HotkeyState::default();

    for (edge, held) in [(Edge::Down, hook::MOD_ALT), (Edge::Up, 0)] {
        hook::classify(
            armed_on_double_shift(),
            &mut state,
            user_key(VK_LSHIFT, edge),
            || held,
            || {
                mouse.set(mouse.get() + 1);
                0
            },
        );
    }

    assert_eq!(mouse.get(), 0, "a press that is no tap read the mouse");
}

/// **(м) Посылка П1 — the taps of the double press leave the word in the buffer**, so the pair
/// converts the word typed before it, and the next pair — after the conversion has been noted —
/// finds the session of the last row of FR-10 still open: the return, as a second `Pause`.
#[test]
fn the_double_press_leaves_the_word_and_the_session_for_the_next_pair() {
    use lang_switcher::buffer::{self, Recorder};
    use lang_switcher::layouts::LayoutId;

    let mut recorder = Recorder::with_capacity(64);
    recorder.set_cache(ghbdtn_cache());
    recorder.set_active_layout(LayoutId::from_raw(0x0409_0409));
    buffer::install_recorder(recorder);

    let mode = armed_on_double_shift();
    let mut state = HotkeyState::default();
    let mut at = 1_000;

    for (vk, scan) in [
        (b'G', SCAN_G),
        (b'H', SCAN_H),
        (b'B', SCAN_B),
        (b'D', SCAN_D),
        (b'T', SCAN_T),
        (b'N', SCAN_N),
    ] {
        for edge in [Edge::Down, Edge::Up] {
            hook::classify(
                mode,
                &mut state,
                KeyEvent {
                    time: at,
                    ..scanned(vk.into(), scan, edge)
                },
                || 0,
                || 0,
            );
            at += 20;
        }
    }

    let typed = buffer::len();
    let mut pair = |start: u32| {
        [
            (Edge::Down, start),
            (Edge::Up, start + 50),
            (Edge::Down, start + 150),
            (Edge::Up, start + 200),
        ]
        .into_iter()
        .filter(|&(edge, time)| {
            hook::classify(
                mode,
                &mut state,
                KeyEvent {
                    time,
                    ..scanned(VK_LSHIFT, 0x2A, edge)
                },
                || 0,
                || 0,
            )
            .fire_hotkey
        })
        .count()
    };

    let first = pair(at + 100);
    let after_first = buffer::len();

    // The input thread notes the conversion it ran on the fire.
    assert!(buffer::note_conversion());

    let second = pair(at + 1_000);
    let after_second = buffer::len();
    let session = buffer::with(|recorder| recorder.in_conversion());

    buffer::uninstall();

    assert_eq!(typed, 6, "the probe typed six strokes");
    assert_eq!(first, 1, "the first pair fired");
    assert_eq!(after_first, 6, "П1: the taps left the word to be converted");
    assert_eq!(second, 1, "the second pair fired");
    assert_eq!(after_second, 6, "and left it again");
    assert_eq!(
        session,
        Some(true),
        "the session of the last row of FR-10 is still open — the next fire returns the word"
    );
}

/// **The rule of times, at `Taps` itself** — the body the capture of the settings window shares
/// (task T-95-2): what the window waits for is what the hook answers to.
#[test]
fn the_rule_of_the_double_press_is_one_body() {
    let mut taps = hook::Taps::default();

    assert_eq!(taps, hook::Taps::NONE);
    taps.press(1_000, true);
    assert!(taps.pressing() && !taps.waiting());
    assert!(!taps.release(1_080, true), "the first tap does not fire");
    assert!(
        !taps.pressing() && taps.waiting(),
        "and is waiting for the second"
    );
    taps.press(1_200, true);
    assert!(taps.release(1_280, true), "the second tap fires");
    assert_eq!(taps, hook::Taps::NONE, "and the pair is over");

    // A press stamped before the first one began — a clock out of order — is no second tap.
    taps.press(5_000, true);
    taps.release(5_050, true);
    taps.press(4_990, true);
    assert!(
        !taps.release(5_060, true),
        "an earlier stamp reads as far away"
    );
}
