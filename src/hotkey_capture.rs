//! **Capturing the hotkey by pressing it** — FR-94: what one press amounts to, the refusals, the
//! double press of `Shift`, and the session that suspends the hotkey while a capture is armed.
//!
//! Moved out of `settings.rs` whole by task T-120-23 — only moved: no body and no name changed.
//! A child of `settings`, so it sees what it saw there; `settings` re-exports every public item.

use super::*;

// =========================================================================================
// FR-94 — capturing the hotkey by pressing it
// =========================================================================================

/// The modifiers held at the moment a key went down.
///
/// A value rather than a query, so that the decision below is a function of its arguments and
/// can be checked without a keyboard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Either `Ctrl`.
    pub ctrl: bool,
    /// Either `Alt`.
    pub alt: bool,
    /// Either `Shift`.
    pub shift: bool,
    /// Either `Windows` key.
    pub win: bool,
}

impl Modifiers {
    /// Whether anything at all is held.
    pub const fn any(self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    /// The three modifiers of FR-96, read exactly as `hook::emergency_modifiers_held` reads
    /// them: all three down, and the `Windows` key is not part of the question.
    pub const fn emergency(self) -> bool {
        self.ctrl && self.alt && self.shift
    }

    /// The same press as the set `hook` compares — `hook::MOD_CTRL`, `MOD_ALT`, `MOD_SHIFT`, and
    /// `MOD_WIN` for a `Windows` key (task T-93-2, вопрос 157).
    pub const fn set(self) -> u16 {
        use crate::hook::{MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN};

        (if self.ctrl { MOD_CTRL } else { 0 })
            | (if self.alt { MOD_ALT } else { 0 })
            | (if self.shift { MOD_SHIFT } else { 0 })
            | (if self.win { MOD_WIN } else { 0 })
    }

    /// What is held right now, from the message queue.
    ///
    /// `GetKeyState` and not `GetAsyncKeyState`: the answer wanted is the state as of the
    /// keystroke being handled, which is what the queue-synchronised call gives, and which is
    /// what the user actually pressed together.
    pub(crate) fn held_now() -> Self {
        Self {
            ctrl: key_is_down(VK_CONTROL),
            alt: key_is_down(VK_MENU),
            shift: key_is_down(VK_SHIFT),
            win: key_is_down(VK_LWIN) || key_is_down(VK_RWIN),
        }
    }
}

/// Whether one virtual key is down, as the message queue sees it.
///
/// `pub(crate)` с задачи Т-46-2: правило нажатия глифа живёт теперь в `widgets::glyph`, и его
/// `arrow_is_down` спрашивает то же самое тем же телом — второго спрашивающего у этой мерки
/// не заводится.
pub(crate) fn key_is_down(key: VIRTUAL_KEY) -> bool {
    // SAFETY: takes a virtual-key code by value and touches no memory of ours. The high bit of
    // the `i16` it answers is "down", which for a signed value is "negative".
    unsafe { GetKeyState(i32::from(key.0)) < 0 }
}

/// Why a press cannot become the hotkey — FR-94, FR-95, FR-96.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A modifier on its own. `Ctrl` is not a hotkey, it is half of one.
    ///
    /// ⚠ **`Combination` stood after this until task T-93-2** — a key with a modifier held was
    /// refused, because section 7 stored one key name and the callback compared one code. Вопрос
    /// 157 widened both (`[hotkey] modifiers`, schema 7; the set compared in `hook::classify`), so
    /// a combination of `Ctrl`, `Alt` and `Shift` is now taken, and the refusal went with its
    /// fourteen sentences. A modifier on its own is still refused — the single tap is rejected by
    /// 157.1, and the double one is stage 2.
    Modifier,
    /// `Ctrl+Alt+Shift+F12`, the emergency exit of FR-96. Never assignable, in either build.
    Emergency,
    /// A key the system owns and an application never gets as a plain key — and, since task
    /// T-93-2, **any press with a `Windows` key held**: `Win` is not one of the modifiers a
    /// hotkey may carry (вопрос 157), and the shell acts on its release whatever anybody else
    /// does with the press.
    Reserved,
    /// ⭐ **A key typing itself needs** — task T-36-5, finding Н9, решение 123.3.
    ///
    /// `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown` and the four arrows: row 3 of the
    /// FR-10 table, the keys that move the caret or take text away. Two things went wrong when
    /// one of them was accepted, and the second is the one a person pays for:
    ///
    /// * the row of FR-10 quietly stopped working — a press that is the hotkey never reaches the
    ///   buffer, so the boundary it was supposed to be never happened;
    /// * **the key was lost in every application.** FR-95 suppresses the hotkey whenever the
    ///   program is active, so somebody who assigned `Delete` had no `Delete` anywhere, with no
    ///   warning in the file and none in the dialog.
    ///
    /// ⛔ Not [`Self::Reserved`]: «the system owns this key» would be a lie about a key the
    /// system does not own. The list is read from `buffer::is_editing_key` — the table of FR-10
    /// lives there and is not copied here.
    ///
    /// ⚠ **The file is not touched by this.** `[hotkey] key = "Delete"` written by hand is still
    /// read, published and acted on: refusing it there would silently return the hotkey to
    /// `Pause`, which is substituting this program's choice for the person's own.
    ///
    /// ⚠ **With modifiers too — 157.12, task T-93-2.** `Shift` + an arrow selects text, `Ctrl` + an
    /// arrow walks a word, `Ctrl+Home` goes to the top: the combinations of these keys are the
    /// editing typing needs just as much, so the key decides and the set does not.
    Editing,
    /// ⭐ **A key that types a character** — task Т-23-5, решение 82.6.
    ///
    /// Until that task a text key was **taken**: the capture ended, the file got `Q`, and the
    /// note under the field carried the warning of FR-95 afterwards. The user asked for the
    /// other order — «текстовая клавиша — предупреждение и захват НЕ гаснет» — and accepted
    /// the live mock-up that behaves that way: the press is refused, the warning appears, and
    /// the capture goes on waiting for the next key instead of quietly assigning the one that
    /// was pressed by accident.
    ///
    /// ⚠ **A text key remains a legal hotkey** and FR-95 is untouched: `[hotkey] key = "Q"`
    /// written by hand is read, published and warned about exactly as before. What changed is
    /// the one road that used to assign one without asking — the capture.
    ///
    /// It says the FR-92 warning and not a sentence of its own ([`IDS_NOTE_TEXT_KEY`]): the
    /// warning already says the true and useful thing — while the program is active that key
    /// stops typing its character.
    ///
    /// ⚠ **The set decides since task T-93-2 — 157.10, вариант 1 until the owner's word.** A text
    /// key with `Ctrl` or `Alt` types nothing and is taken (`Ctrl+K`); with nothing held, with
    /// `Shift` (a capital, a symbol) or with `Ctrl` and `Alt` together (`AltGr`) it does type, and
    /// is refused — see `types_a_character`.
    Text,
    /// A key section 7 has no name for, so it could not be written to the file and read back.
    Nameless,
    /// ⭐ **The key already fixes the typed text** — a capture of the second key, решение 208.228
    /// (the moment «Конфликт» of the mock-up): the main key of the first hotkey, with any set (§5 of
    /// the design — `Shift+Pause` beside `Pause` too), or a second double press of `Shift`.
    TakenByFix,
    /// **The key already switches the layout** — a capture of the first key while a second is in
    /// force.
    TakenBySwitch,
    /// ⭐ **Windows switches the input language with it** — решение 208.237 п. 1: `Alt+Shift` and
    /// `Ctrl+Shift` (a modifier on its own, as [`Self::Modifier`], with nothing else held) and
    /// `Win+Space` (a press under `Win`, as [`Self::Reserved`]). Refused as before; the line says why.
    WindowsSwitch,
}

impl Refusal {
    /// The interface string that says this to the user.
    pub const fn string_id(self) -> u16 {
        match self {
            Self::Modifier => IDS_CAPTURE_MODIFIER,
            Self::Emergency => IDS_CAPTURE_EMERGENCY,
            Self::Reserved => IDS_CAPTURE_RESERVED,
            // Task T-36-5: a sentence of its own — the key is not the system's, it is the
            // typing's, and what the person loses by taking it is different.
            Self::Editing => IDS_CAPTURE_TYPING,
            // Task Т-23-5: the warning FR-92 already asks for, said at the moment of the
            // press instead of after it.
            Self::Text => IDS_NOTE_TEXT_KEY,
            Self::Nameless => IDS_CAPTURE_NAMELESS,
            // Решение 208.228: what the key already does, said by the key that does it.
            Self::TakenByFix => IDS_CAPTURE_TAKEN_FIX,
            Self::TakenBySwitch => IDS_CAPTURE_TAKEN_SWITCH,
            // Решение 208.237 п. 1.
            Self::WindowsSwitch => IDS_CAPTURE_WINDOWS_SWITCH,
        }
    }
}

/// **The refusal of a modifier on its own** — [`Refusal::Modifier`], or, решение 208.237 п. 1,
/// [`Refusal::WindowsSwitch`] when what is down with the press is exactly `Alt+Shift` or `Ctrl+Shift`:
/// the switches of the input language in Windows. `vk` is the modifier pressed, `modifiers` what the
/// queue holds with it.
pub fn modifier_refusal(vk: u16, modifiers: Modifiers) -> Refusal {
    let shift = modifiers.shift || SHIFT_KEYS.contains(&vk);
    let ctrl = modifiers.ctrl || CTRL_KEYS.contains(&vk);
    let alt = modifiers.alt || ALT_KEYS.contains(&vk);

    if shift && ctrl != alt && !modifiers.win {
        Refusal::WindowsSwitch
    } else {
        Refusal::Modifier
    }
}

/// **Which hotkey a capture assigns** — решение 208.228: the first, «Исправить набранное», or the
/// second, «Переключить раскладку». A lone `Ctrl` is the second's alone (В-4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CaptureTarget {
    /// The first hotkey — `[hotkey]`.
    #[default]
    Fix,
    /// The second hotkey — `[switch_hotkey]`.
    Switch,
}

/// What one press during a capture amounts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    /// The press is the hotkey. The value is the `[hotkey]` section 7 stores — the key's name and,
    /// since task T-93-2, the modifiers held with it in the canonical order — and
    /// [`Hotkey::binding`] reads it back to the very code and set it came from.
    Taken(Hotkey),
    /// **A `Shift` on its own — the beginning of a double press**, task T-95-2, вопрос 159: not a
    /// refusal any more (157.17 — the double press is `Shift`'s). What it becomes is decided by
    /// what follows, and that is the memory of [`CapturePair`]: a second tap — the double press; a
    /// key with the `Shift` held — a combination, as in stage 1; nothing — the invitation again.
    Shift,
    /// The press cannot be the hotkey. The capture stays armed and the reason is shown.
    Refused(Refusal),
}

/// One thing that can happen to an armed capture — task Т-23-5, решение 82.6.
///
/// The events the window procedures of the dialog can see, named apart from the Win32 messages
/// that carry them so that the machine below can be exercised without a window. Task T-95-2
/// added two, for the double press of `Shift`: the release of a key and the end of the window of
/// the pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEvent {
    /// A key went down while the field held the focus — the virtual code and the modifiers
    /// held with it.
    KeyDown(u16, Modifiers),
    /// **A key went up while the field held the focus** — task T-95-2: the release of `Shift`
    /// ends a tap of the double press; the release of any other key changes nothing.
    KeyUp(u16),
    /// **The window of the double press ran out** — task T-95-2: the one-shot timer
    /// [`CAPTURE_PAIR_TIMER`] of the dialog, armed by a press of `Shift`, has fired.
    PairTimeout,
    /// The field lost the keyboard focus. The flag says whether it went to the capture button
    /// itself, which is the one window that may take it without ending the capture: that
    /// button is about to report a click, and ending here would turn the click into a fresh
    /// arming.
    FocusLost { to_capture_button: bool },
    /// A press landed on the dialog's own ground — not on the field, not on the button, and
    /// not on any control that takes the focus (those arrive as [`Self::FocusLost`]).
    ClickBeside,
}

/// What one event does to the capture — task Т-23-5, решение 82.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStep {
    /// Nothing at all: no capture is armed, or the event is one the machine deliberately
    /// lets pass.
    Ignore,
    /// The capture ends and the name that stood in the field before it comes back.
    Cancel,
    /// The press is the hotkey; the value is the `[hotkey]` section 7 stores — the key and,
    /// since task T-93-2, its modifiers.
    Take(Hotkey),
    /// The press cannot be the hotkey. **The capture stays armed** — a refusal is «not that
    /// one», not an end to the question — and the note says which of the six reasons it was.
    Refuse(Refusal),
    /// **A `Shift` went down on its own — the first tap of a double press or its second** (task
    /// T-95-2, the moment 3 of the mock-up of 157.18): the field says «Shift…», the line under it
    /// [`IDS_CAPTURE_SHIFT_AGAIN`], and the one-shot timer of the pair runs for
    /// `hook::DOUBLE_TAP_MS`. The capture stays armed.
    AwaitShift,
    /// **The second `Shift` did not come in time, or a tap was held too long** — the moment 4: the
    /// field says the invitation again, the line under it stays until the next press, and the timer
    /// is dead. The capture stays armed.
    ShiftLapsed,
    /// ⭐ **A lone `Ctrl` was tapped in a capture of the second key** — решение 208.228, В-4: the
    /// value is the side, `VK_LCONTROL` or `VK_RCONTROL`. Not taken: the window ends the capture and
    /// shows the warning — «Всё равно установить» takes it, «Выбрать другую клавишу» arms the
    /// capture again.
    CtrlTapped(u16),
    /// ⭐ **The tap of `Ctrl+Alt`** — решение 208.237 п. 2, a capture of either key: the left `Alt` and a
    /// `Ctrl` down together and up within `hook::DOUBLE_TAP_MS`, nothing between. Not taken: the
    /// window ends the capture and shows its warning, as for [`Self::CtrlTapped`].
    CtrlAltTapped,
}

/// **The double press of `Shift` as a capture waits for it** — task T-95-2, вопрос 159, the moments
/// of the mock-up of 157.18.
///
/// The memory the capture gained: [`capture_step`] decides one press at a time and cannot tell a
/// second tap from a first. The rule of times is the hook's own, [`crate::hook::Taps`] — one body
/// for the window that assigns the double press and the callback that answers to it, so the field
/// takes exactly the pair the hook will act on. What only a window can know is here: a `Shift` is
/// down (its auto-repeat is no new press), and what the window shows while a pair is open.
///
/// The times are the times of the window messages (`GetMessageTime`), as the hook's are the times of
/// its events; the modifiers held are the queue's (`GetKeyState`, [`Modifiers::held_now`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapturePair {
    /// A `Shift` is down, as the field saw it go down — what tells its auto-repeat from a new press.
    held: bool,
    /// The taps made so far, and the rule they are counted by.
    taps: crate::hook::Taps,
}

impl CapturePair {
    /// One event of an armed capture, with the memory of the pair — task T-95-2. `time` is the
    /// time of the message the event came in.
    ///
    /// * A press of `Shift` on its own ([`capture_step`] answers [`CaptureStep::AwaitShift`]) is a
    ///   tap begun — the first or the second — unless it is the auto-repeat of the `Shift` already
    ///   down, which is nothing.
    /// * The release of `Shift` ends the tap: the second tap within the window — the double press,
    ///   taken; the first — the pair waits, nothing changes on the screen; a tap held too long —
    ///   [`CaptureStep::ShiftLapsed`].
    /// * The end of the window ([`CaptureEvent::PairTimeout`]) with a pair open —
    ///   [`CaptureStep::ShiftLapsed`]; with none, a stale tick, nothing.
    /// * Any other press ends the pair and is decided as [`capture_step`] decides it: a key with
    ///   `Shift` held is a combination (stage 1), `Ctrl` or `Alt` on their own are refused, `Esc`
    ///   cancels. A focus lost or a click beside cancel the capture, the pair with it.
    pub fn step(&mut self, event: CaptureEvent, time: u32) -> CaptureStep {
        match event {
            CaptureEvent::KeyDown(vk, _) if SHIFT_KEYS.contains(&vk) => {
                let repeat = std::mem::replace(&mut self.held, true);

                match capture_step(true, event) {
                    CaptureStep::AwaitShift if repeat => CaptureStep::Ignore,
                    CaptureStep::AwaitShift => {
                        self.taps.press(time, true);
                        CaptureStep::AwaitShift
                    }
                    // A `Shift` under `Ctrl` or `Alt`: refused, and no tap.
                    other => {
                        self.taps.interrupt();
                        other
                    }
                }
            }

            CaptureEvent::KeyUp(vk) if SHIFT_KEYS.contains(&vk) => {
                if !std::mem::replace(&mut self.held, false) {
                    // The release of a `Shift` this capture never saw go down.
                    return CaptureStep::Ignore;
                }

                let pressing = self.taps.pressing();

                if self.taps.release(time, true) {
                    return CaptureStep::Take(Hotkey::double_shift());
                }

                if pressing && !self.taps.waiting() {
                    // The press was no tap — held longer than the window.
                    return CaptureStep::ShiftLapsed;
                }

                CaptureStep::Ignore
            }

            CaptureEvent::KeyUp(_) => CaptureStep::Ignore,

            CaptureEvent::PairTimeout => {
                if !self.open() {
                    return CaptureStep::Ignore;
                }

                self.taps.interrupt();
                CaptureStep::ShiftLapsed
            }

            CaptureEvent::KeyDown(..)
            | CaptureEvent::FocusLost { .. }
            | CaptureEvent::ClickBeside => {
                if matches!(event, CaptureEvent::KeyDown(..)) {
                    self.taps.interrupt();
                }

                capture_step(true, event)
            }
        }
    }

    /// Whether a pair is open — a tap made, or a press that may be one: the field says «Shift…»
    /// and the timer of the pair runs.
    pub fn open(&self) -> bool {
        self.taps.pressing() || self.taps.waiting()
    }

    /// Something else was pressed — the `Ctrl` of a capture of the second key (решение 208.228):
    /// the pair is broken, as by any key.
    pub fn interrupt(&mut self) {
        self.taps.interrupt();
    }
}

/// The whole decision of the capture, as a function of the state and one event — task Т-23-5.
///
/// Split out of the window procedures for the reason [`capture`] itself is split out of them:
/// what a program does is worth closing without a window, and a machine spread over a
/// `WM_KEYDOWN` arm, a `WM_KILLFOCUS` arm and a `WM_LBUTTONDOWN` arm is a machine no test can
/// see whole. The procedures keep exactly two jobs — turning a message into a
/// [`CaptureEvent`], and carrying out the [`CaptureStep`] this function answers.
///
/// ⚠ **Escape is the way out and is therefore the one key a capture cannot assign.** It stays
/// assignable by hand: `hook::vk_from_name` still reads `Escape` and `Esc` out of the file, and
/// the dialog shows whatever it finds there.
pub fn capture_step(armed: bool, event: CaptureEvent) -> CaptureStep {
    if !armed {
        return CaptureStep::Ignore;
    }

    match event {
        CaptureEvent::KeyDown(vk, _) if vk == VK_ESCAPE.0 => CaptureStep::Cancel,

        CaptureEvent::KeyDown(vk, modifiers) => match capture(vk, modifiers) {
            Capture::Taken(hotkey) => CaptureStep::Take(hotkey),
            // Task T-95-2: one press at a time, a `Shift` on its own is the beginning of a pair —
            // what it becomes, [`CapturePair::step`] decides.
            Capture::Shift => CaptureStep::AwaitShift,
            Capture::Refused(refusal) => CaptureStep::Refuse(refusal),
        },

        // Task T-95-2: a release, and the end of the window of a pair, mean something only to the
        // memory of a pair ([`CapturePair::step`]); one press at a time, they are nothing.
        CaptureEvent::KeyUp(_) | CaptureEvent::PairTimeout => CaptureStep::Ignore,

        CaptureEvent::FocusLost { to_capture_button } => {
            if to_capture_button {
                CaptureStep::Ignore
            } else {
                CaptureStep::Cancel
            }
        }

        CaptureEvent::ClickBeside => CaptureStep::Cancel,
    }
}

/// The keys an application never receives as a plain key, because the shell takes them first.
///
/// Only the two `Windows` keys are here, and they are here because the shell acts on their
/// *release* whatever anybody else does with the press. Everything else the system reserves —
/// `Ctrl+Alt+Del`, `Win+L` — is taken below the level any window can see, so it never arrives
/// at the capture at all: nothing to refuse, and the dialog simply goes on waiting.
const SYSTEM_RESERVED: &[u16] = &[VK_LWIN.0, VK_RWIN.0];

/// The three modifiers, in all their forms — the neutral code the queue reports and the two
/// sided ones a keyboard can send.
const MODIFIER_KEYS: &[u16] = &[
    VK_CONTROL.0,
    VK_LCONTROL.0,
    VK_RCONTROL.0,
    VK_MENU.0,
    VK_LMENU.0,
    VK_RMENU.0,
    VK_SHIFT.0,
    VK_LSHIFT.0,
    VK_RSHIFT.0,
];

/// The whole capture decision, as a function of the key and of what was held with it.
///
/// The order of the tests is not free:
///
/// * **FR-96 first**, exactly as `hook::keyboard_hook_proc` puts it first, and for the same
///   reason: the emergency combination is not a candidate hotkey under any circumstance. In
///   practice the callback has already ended the process by the time this could run — this is
///   the answer for the case where the hook is not installed at all;
/// * then the `Windows` key — pressed, or held under another key — which is neither a modifier
///   of this program's vocabulary nor a key an application ever gets;
/// * then a modifier on its own, which is a press the user has not finished making — except a
///   `Shift` with nothing else held, which since task T-95-2 begins a double press
///   ([`Capture::Shift`]);
/// * then a key that would type a character with what is held ([`Refusal::Text`]) and a key
///   typing itself needs ([`Refusal::Editing`]);
/// * and last the vocabulary of section 7, because a name that cannot be written to the file
///   is a hotkey that would not survive a restart.
///
/// ⭐ **A combination is taken since task T-93-2 — вопрос 157, устройство А.** `Ctrl`, `Alt` and
/// `Shift` held with a key make a hotkey of the key *and* the set (`Ctrl + F12`): the capture
/// understands what was pressed, with no switch to choose a way first (157.6).
///
/// **SEC-01, SEC-07.** The value that comes out is a *name of a key* — `F8`, `Pause`, `A` — and
/// the names of the modifiers held with it, built from the virtual-key code and the set and from
/// nothing else. No character is produced, nothing is translated through a keyboard layout, and
/// nothing here reaches the journal: the caller shows the name in the field and writes it to
/// `[hotkey]`, which is where section 7 keeps it anyway.
pub fn capture(vk: u16, modifiers: Modifiers) -> Capture {
    if vk == crate::hook::EMERGENCY_VK && modifiers.emergency() {
        return Capture::Refused(Refusal::Emergency);
    }

    if SYSTEM_RESERVED.contains(&vk) || modifiers.win {
        // ⭐ Решение 208.237 п. 1: `Win+Space` is the switch of the input language — the line says so.
        if vk == VK_SPACE.0 && modifiers.win {
            return Capture::Refused(Refusal::WindowsSwitch);
        }

        return Capture::Refused(Refusal::Reserved);
    }

    if MODIFIER_KEYS.contains(&vk) {
        // ⭐ Task T-95-2, вопрос 159: a `Shift` with nothing but itself held is the beginning of
        // a double press — 157.17 gives the double press to `Shift` alone. `Ctrl` and `Alt` on
        // their own, and a `Shift` pressed under either (`Ctrl+Shift`, `Alt+Shift` — the switches
        // of the layout), stay the refusal they were; its line names the way that is left
        // («дважды — только Shift») — and since решение 208.237 п. 1 `Ctrl+Shift` and `Alt+Shift`
        // name the reason instead ([`modifier_refusal`]).
        if SHIFT_KEYS.contains(&vk) && !modifiers.ctrl && !modifiers.alt {
            return Capture::Shift;
        }

        return Capture::Refused(modifier_refusal(vk, modifiers));
    }

    let set = modifiers.set();

    // ⭐ Task Т-23-5, решение 82.6. Before this line a letter, a digit or an OEM key was taken
    // and the warning of FR-95 followed the assignment; the user asked for the warning to come
    // *instead* of it, so that a stray press cannot quietly become the hotkey. See
    // [`Refusal::Text`] for what did **not** change: a text key written into the file by hand
    // is still a hotkey, and FR-95 still suppresses it. Since task T-93-2 the set decides with
    // the key — `Ctrl+K` types nothing and is taken, `Shift+K` types a capital and is not
    // (157.10, вариант 1 until the owner's word).
    if types_a_character(vk, set) {
        return Capture::Refused(Refusal::Text);
    }

    // ⭐ Task T-36-5, finding Н9, решение 123.3 — the keys typing itself needs. `Delete` and
    // `Home` were taken until this line, and taking one cost the person that key in every
    // application (FR-95) with no warning anywhere. The list is the FR-10 table and is read from
    // where that table lives; see [`Refusal::Editing`].
    //
    // Below `is_text_key` because the two never overlap and the order of the two answers does
    // not matter — and above `key_name`, because six of these keys **do** have names and would
    // otherwise be accepted, while the four arrows would be refused as nameless, which is true
    // and useless.
    if crate::buffer::is_editing_key(vk) {
        return Capture::Refused(Refusal::Editing);
    }

    // The key under its name, and the set in the canonical order whatever order it was pressed
    // in — task T-93-2. No `Win` reaches this line, so the set always has a name.
    match Hotkey::from_binding(vk, set) {
        Some(hotkey) => Capture::Taken(hotkey),
        None => Capture::Refused(Refusal::Nameless),
    }
}

/// The named keys of section 7, in the spelling this program *writes*.
///
/// ⚠ The mirror of `hook::NAMED_KEYS`, narrowed to one spelling per code: that table accepts
/// `Break`, `PgUp` and `Esc` because a person writes them by hand, and this one has to choose
/// which of them a capture puts in the file. Kept honest by a test that runs every code this
/// function names back through `hook::vk_from_name` and demands the same code out — the reverse
/// direction of the one function that reads the file.
const NAMED_KEYS: &[(&str, VIRTUAL_KEY)] = &[
    ("Pause", VK_PAUSE),
    ("Escape", VK_ESCAPE),
    ("Insert", VK_INSERT),
    ("Delete", VK_DELETE),
    ("Home", VK_HOME),
    ("End", VK_END),
    ("PageUp", VK_PRIOR),
    ("PageDown", VK_NEXT),
    ("ScrollLock", VK_SCROLL),
    ("NumLock", VK_NUMLOCK),
    ("CapsLock", VK_CAPITAL),
    ("PrintScreen", VK_SNAPSHOT),
    ("Apps", VK_APPS),
];

/// Highest function key Windows has a virtual-key code for — `hook::HIGHEST_FUNCTION_KEY`.
const HIGHEST_FUNCTION_KEY: u16 = 24;

/// The name section 7 stores for a virtual-key code, or `None` when it has none.
///
/// The inverse of `hook::vk_from_name`, and it has to be written here rather than there:
/// `src\hook.rs` is the accepted code of the hot path and this task does not touch it. What
/// keeps the two from drifting is not discipline but a test — see [`NAMED_KEYS`].
pub fn key_name(vk: u16) -> Option<String> {
    // A single ASCII letter or digit is its own virtual-key code, which is the documented
    // property `hook::vk_from_name` reads in the other direction.
    if let Ok(byte) = u8::try_from(vk)
        && (byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Some(char::from(byte).to_string());
    }

    if (VK_F1.0..VK_F1.0 + HIGHEST_FUNCTION_KEY).contains(&vk) {
        return Some(format!("F{}", vk - VK_F1.0 + 1));
    }

    NAMED_KEYS
        .iter()
        .find(|(_, code)| code.0 == vk)
        .map(|(name, _)| (*name).to_owned())
}

/// The virtual-key code a capture publishes while it is armed.
///
/// Zero is not a key: `KBDLLHOOKSTRUCT::vkCode` is documented to be in 1..=254, so no stroke can
/// ever equal it and `hook::classify` takes the "ordinary key" branch for every press. That is
/// exactly the state a capture needs — see [`CaptureSession`].
const NO_HOTKEY_VK: u16 = 0;

/// A capture in progress — and, for as long as it lives, the hotkey of FR-02 suspended.
///
/// # Why the ordinary work has to stop
///
/// The keys the user presses to *choose* a hotkey travel through the same global low-level hook
/// as every other key on the machine. Two things would otherwise happen at once: pressing the
/// current hotkey would run a conversion nobody asked for (FR-02), and it would be **suppressed
/// by FR-95 before reaching this window**, so the one key the user is most likely to press
/// first could never be captured at all.
///
/// # What is suspended, and why by two different switches
///
/// **The one that matters is the hotkey code.** Arming publishes [`NO_HOTKEY_VK`] through
/// [`crate::hook::set_hotkey`] — the interface this task is given for the hotkey — and
/// dropping publishes back exactly what stood there before, the key and its modifiers in one
/// store (task T-93-1). No press can equal a code no keyboard produces, so nothing is converted
/// and nothing is suppressed for as long as the capture is armed.
///
/// `hook::set_active(false)` is published as well, and it is deliberately **not** what the
/// suspension rests on.
///
/// ⛔ **What it does not do — task T-36-6, finding Н116, решение 123.1.** This paragraph used to
/// promise a second effect: that the flag «additionally stops the stroke reaching the typing
/// buffer», so nothing typed while a hotkey is being chosen goes into the buffer of FR-10. **The
/// program does not keep that promise, and the knowledge is worth more than the sentence was.**
/// `app::window_proc` re-publishes `tray.enabled()` into `hook::set_active` **after every message
/// the UI thread sees** (FR-90, FR-91 — a program the user has just suspended must stop swallowing
/// the hotkey at once), so the flag is back to `true` within one message of being cleared: moving
/// the mouse over the tray icon is enough. Nothing here can hold it down, and this task does not
/// try — the repair would need a flag of the capture's own inside `hook`, read by the callback,
/// which is a new branch on the path NFR-01…NFR-05 governs. The promise is therefore struck out
/// rather than rewritten; the second effect is in `DEFERRED.md`.
///
/// The hotkey code, by contrast, is published from `app::publish_configuration` alone, which runs
/// on «Применить» and at start-up and from no message, so it stays where a capture puts it. That
/// is why the suspension rests on it.
///
/// **FR-96 is not affected and cannot be.** The emergency combination is handled at the top of
/// the callback, before either of these values is so much as read — see
/// `hook::keyboard_hook_proc` — so `Ctrl+Alt+Shift+F12` ends the process during a capture
/// exactly as it does at any other moment. That is the condition the decision on question 18
/// puts on running a debug build at all, and nothing in this file weakens it.
///
/// The restoration is in `Drop` and not in a method on purpose: the program stays up after a
/// panic (FR-98, FR-99), and a capture that swallowed a panic would leave the program with no
/// hotkey at all and no way back short of a restart.
pub struct CaptureSession {
    /// `[hotkey]` as it stood when the capture was armed — what «отмена» puts back: the key and,
    /// since task T-93-1, the modifiers held with it.
    previous: Hotkey,
    /// The double press this capture is waiting for, if it is — task T-95-2. Born empty with the
    /// capture and gone with it, so a pair never outlives the capture it was begun in.
    pair: CapturePair,
    /// The hotkey the callback was comparing against when the capture was armed — the code and
    /// its modifier set, which `hook` publishes as one word (task T-93-1).
    ///
    /// ⚠ The only value this session remembers since task T-36-6. There used to be a second —
    /// `hook::is_active()` as it stood at arming — and it was put back in `Drop`; both halves went
    /// out with the promise above. Remembering it was worse than not: `app::window_proc`
    /// re-publishes `tray.enabled()` after every message, so the saved copy was stale within a
    /// message of being taken, and putting it back at the end of a capture wrote a value older
    /// than the one the program already had. What the flag must equal after a capture is
    /// `tray.enabled()`, and that is published by the procedure itself.
    ///
    /// ⭐ Решение 208.228: **both hotkeys** — the one word holds both, and a capture of either
    /// suspends both (§3 of the design: «пока взведён любой захват, сняты обе»).
    published: ((u16, u16), (u16, u16)),
    /// Which hotkey this capture assigns (решение 208.228).
    target: CaptureTarget,
    /// The other hotkey in force — its code and set — which the press must not share its main key
    /// with ([`crate::settings::same_main_key`]); `None` — there is no other.
    other: Option<(u16, u16)>,
    /// The `Ctrl` down in a capture of the second key — its side, zero for none (В-4).
    ctrl_side: u16,
    /// The tap of that `Ctrl`, by the hook's own rule of times ([`crate::hook::Taps::single`]).
    ctrl_taps: crate::hook::Taps,
    /// The tap of `Ctrl+Alt` this capture may be seeing (решение 208.237 п. 2) — either key's.
    chord: ChordCapture,
    /// The hotkeys were suspended by this capture and come back when it ends; `false` only for a
    /// capture drawn and fitted ([`Self::still`]), which never touched them.
    suspended: bool,
    /// The refusal of a modifier on its own held back while the press may still be the tap of
    /// `Ctrl+Alt` — said on the release ([`Self::step`]).
    held_refusal: Option<Refusal>,
}

/// ⭐ **The tap of `Ctrl+Alt` as a capture waits for it** — решение 208.237 п. 2: the rule of the hook
/// (`hook::chord_stroke`) on the window messages, timed by the hook's own [`crate::hook::Taps`].
///
/// The left `Alt` and a `Ctrl` go down together — in any order, the first with nothing else held, the
/// second with no `Shift` or `Win` — and the last of them comes up within `hook::DOUBLE_TAP_MS` of the
/// first press, no other key down in between. The right `Alt` (`AltGr`) is never a key of the tap
/// ([`sided_alt`]); its false `Ctrl` may begin one, and the right `Alt` that follows ends it.
///
/// It only watches: what each press shows meanwhile is decided as ever (the refusal of a modifier on
/// its own); the release that ends the tap is answered by [`CaptureStep::CtrlAltTapped`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ChordCapture {
    /// A `Ctrl` of the tap is down.
    ctrl: bool,
    /// The left `Alt` of the tap is down.
    alt: bool,
    /// Both have been down together.
    both: bool,
    /// The tap, timed from its first press.
    taps: crate::hook::Taps,
}

impl ChordCapture {
    /// Whether a key of the tap is down and the tap may still be made.
    fn may_tap(&self) -> bool {
        (self.ctrl || self.alt) && self.taps.pressing()
    }

    /// Whether no key of the tap is down.
    fn idle(&self) -> bool {
        !self.ctrl && !self.alt
    }

    /// One event — `true` when it ends the tap.
    fn step(&mut self, event: CaptureEvent, time: u32) -> bool {
        match event {
            CaptureEvent::KeyDown(vk, modifiers) => {
                let ctrl = CTRL_KEYS.contains(&vk);
                let alt = ALT_KEYS.contains(&vk) && sided_alt(vk) == VK_LMENU.0;

                if (ctrl && self.ctrl) || (alt && self.alt) {
                    // The auto-repeat of a key of the tap already down.
                    return false;
                }

                if !ctrl && !alt {
                    // Any other key — the right `Alt` among them.
                    self.taps.interrupt();
                    return false;
                }

                let first = !self.ctrl && !self.alt;
                let other = if ctrl { modifiers.alt } else { modifiers.ctrl };
                let clean = !modifiers.shift && !modifiers.win && (!first || !other);

                if ctrl {
                    self.ctrl = true;
                } else {
                    self.alt = true;
                }

                if first {
                    self.both = false;
                    self.taps.press(time, clean);
                } else if clean {
                    self.both = self.taps.pressing();
                } else {
                    self.taps.interrupt();
                }

                false
            }

            CaptureEvent::KeyUp(vk) => {
                let ctrl = CTRL_KEYS.contains(&vk) && self.ctrl;
                let alt = ALT_KEYS.contains(&vk) && self.alt;

                if !ctrl && !alt {
                    return false;
                }

                if ctrl {
                    self.ctrl = false;
                } else {
                    self.alt = false;
                }

                if self.ctrl || self.alt {
                    return false;
                }

                let both = std::mem::take(&mut self.both);

                self.taps.single(time, both)
            }

            CaptureEvent::FocusLost { .. } | CaptureEvent::ClickBeside => {
                *self = Self::default();
                false
            }

            CaptureEvent::PairTimeout => false,
        }
    }
}

impl CaptureSession {
    /// Arms a capture **of the first hotkey** with no second beside it, and suspends the hotkey for
    /// its duration — [`Self::arm_for`] with [`CaptureTarget::Fix`].
    pub fn arm(previous: Hotkey) -> Self {
        Self::arm_for(CaptureTarget::Fix, previous, None)
    }

    /// Arms a capture of `target` — решение 208.228 — and suspends **both** hotkeys for its
    /// duration. `previous` is what stood in its field; `other` is the other hotkey in force, whose
    /// main key a press may not take ([`Refusal::TakenByFix`], [`Refusal::TakenBySwitch`]).
    pub fn arm_for(target: CaptureTarget, previous: Hotkey, other: Option<(u16, u16)>) -> Self {
        let published = crate::hook::hotkeys();

        crate::hook::set_hotkeys((NO_HOTKEY_VK, 0), (NO_HOTKEY_VK, 0));

        // ⚠ Task T-36-6 left this call exactly where it was and took away only the belief about
        // it: the flag is cleared, and `app::window_proc` publishes `tray.enabled()` back over it
        // at the next message it sees. Removing the call would be a change of behaviour, and this
        // task is the striking out of a promise, not a repair of one.
        crate::hook::set_active(false);

        Self {
            previous,
            pair: CapturePair::default(),
            published,
            target,
            other,
            ctrl_side: 0,
            ctrl_taps: crate::hook::Taps::NONE,
            chord: ChordCapture::default(),
            suspended: true,
            held_refusal: None,
        }
    }

    /// **A capture for the pictures and the fitting** (the catalogue of states of the window of the
    /// settings, решение 208.237): it decides as [`Self::arm_for`]'s does and **suspends nothing** —
    /// the published hotkeys and the flag of the hook are not touched, and nothing is given back.
    pub fn still(target: CaptureTarget, previous: Hotkey, other: Option<(u16, u16)>) -> Self {
        Self {
            previous,
            pair: CapturePair::default(),
            published: ((NO_HOTKEY_VK, 0), (NO_HOTKEY_VK, 0)),
            target,
            other,
            ctrl_side: 0,
            ctrl_taps: crate::hook::Taps::NONE,
            chord: ChordCapture::default(),
            suspended: false,
            held_refusal: None,
        }
    }

    /// The hotkey to go back to if the capture is cancelled — the key and its modifiers.
    pub fn previous(&self) -> &Hotkey {
        &self.previous
    }

    /// Which hotkey this capture assigns.
    pub fn target(&self) -> CaptureTarget {
        self.target
    }

    /// One event of this capture, decided with the memory of its pair — [`CapturePair::step`],
    /// task T-95-2.
    ///
    /// ⭐ Решение 208.228: a capture of the second key takes a tap of a lone `Ctrl` first
    /// ([`Self::ctrl_step`]), and a press taken by either capture whose main key the other hotkey
    /// holds is refused, the capture staying armed.
    ///
    /// ⭐ Решение 208.237 п. 2: either capture watches for the tap of `Ctrl+Alt` ([`ChordCapture`]); the
    /// release that ends it is [`CaptureStep::CtrlAltTapped`] — or the refusal of a key the other
    /// hotkey holds, when the other is the same tap.
    ///
    /// ⭐ **The refusal of a modifier on its own waits for the release** (решение 208.237 п. 2, word of
    /// the controller after the pictures): while the press of a `Ctrl` or of the left `Alt` may still
    /// become the tap of `Ctrl+Alt`, «Ctrl и Alt — только вместе с клавишей» is not said — the tap is
    /// lawful now; it is said on the release of the last of them when no tap was made, as the refusal
    /// it always was. A refusal or a key said meanwhile (`Ctrl+Shift`, `AltGr`, `Ctrl+F12`) drops it.
    pub fn step(&mut self, event: CaptureEvent, time: u32) -> CaptureStep {
        let tapped = self.chord.step(event, time);
        let step = self.key_step(event, time);

        if tapped {
            self.held_refusal = None;

            return match self.other {
                Some(other) if crate::settings::same_main_key(crate::hook::CTRL_ALT_TAP, other) => {
                    CaptureStep::Refuse(self.taken_by_other())
                }
                _ => CaptureStep::CtrlAltTapped,
            };
        }

        match (event, step) {
            (CaptureEvent::KeyDown(..), CaptureStep::Refuse(Refusal::Modifier))
                if self.chord.may_tap() =>
            {
                self.held_refusal = Some(Refusal::Modifier);
                CaptureStep::Ignore
            }
            (CaptureEvent::KeyUp(_), CaptureStep::Ignore) if self.chord.idle() => self
                .held_refusal
                .take()
                .map_or(CaptureStep::Ignore, CaptureStep::Refuse),
            (_, CaptureStep::Ignore) => CaptureStep::Ignore,
            (_, step) => {
                self.held_refusal = None;
                step
            }
        }
    }

    /// The refusal of a press whose main key the other hotkey holds.
    fn taken_by_other(&self) -> Refusal {
        match self.target {
            CaptureTarget::Fix => Refusal::TakenBySwitch,
            CaptureTarget::Switch => Refusal::TakenByFix,
        }
    }

    /// One event of this capture without the tap of `Ctrl+Alt` — the lone `Ctrl` of the second key,
    /// the pair of `Shift`, the press.
    fn key_step(&mut self, event: CaptureEvent, time: u32) -> CaptureStep {
        if self.target == CaptureTarget::Switch
            && let Some(step) = self.ctrl_step(event, time)
        {
            return step;
        }

        match self.pair.step(event, time) {
            CaptureStep::Take(hotkey) => match (hotkey.binding(), self.other) {
                (Some(taken), Some(other)) if crate::settings::same_main_key(taken, other) => {
                    CaptureStep::Refuse(self.taken_by_other())
                }
                _ => CaptureStep::Take(hotkey),
            },
            step => step,
        }
    }

    /// **A lone `Ctrl` in a capture of the second key** — решение 208.228, В-4. `None` — the event is
    /// not this rule's, and the pair decides it.
    ///
    /// * a `Ctrl` goes down with no `Alt`, `Shift` or `Win` held — a tap may begin, the line under
    ///   the field does not change; with any of them — the refusal of a modifier on its own, as in a
    ///   capture of the first key;
    /// * the `Ctrl` comes up within `hook::DOUBLE_TAP_MS`, no key down in between —
    ///   [`CaptureStep::CtrlTapped`] with its side; held longer — nothing, the capture waits on;
    /// * a key pressed while it is down ends the tap, and the key is decided as ever — `Ctrl + F12`
    ///   is a combination.
    ///
    /// The side is the one the queue holds down: the window messages carry the neutral `VK_CONTROL`
    /// ([`sided_ctrl`]).
    fn ctrl_step(&mut self, event: CaptureEvent, time: u32) -> Option<CaptureStep> {
        match event {
            CaptureEvent::KeyDown(vk, modifiers) if CTRL_KEYS.contains(&vk) => {
                let side = sided_ctrl(vk);

                if self.ctrl_side == side {
                    // The auto-repeat of the `Ctrl` already down.
                    return Some(CaptureStep::Ignore);
                }

                let clean =
                    self.ctrl_side == 0 && !modifiers.alt && !modifiers.shift && !modifiers.win;

                self.ctrl_side = side;
                self.pair.interrupt();
                self.ctrl_taps.press(time, clean);

                Some(if clean {
                    CaptureStep::Ignore
                } else {
                    // Решение 208.237 п. 1: `Ctrl+Shift` names its reason.
                    CaptureStep::Refuse(modifier_refusal(vk, modifiers))
                })
            }

            CaptureEvent::KeyUp(vk) if CTRL_KEYS.contains(&vk) => {
                let side = std::mem::take(&mut self.ctrl_side);

                if side == 0 {
                    // The release of a `Ctrl` this capture never saw go down.
                    return Some(CaptureStep::Ignore);
                }

                // A `Ctrl` held too long is no tap and no key either: the capture waits on, its
                // line unchanged — the refusal of a modifier says «Ctrl — только вместе с
                // клавишей», which a capture of the second key does not mean.
                Some(if self.ctrl_taps.single(time, true) {
                    CaptureStep::CtrlTapped(side)
                } else {
                    CaptureStep::Ignore
                })
            }

            CaptureEvent::KeyDown(..) => {
                self.ctrl_taps.interrupt();
                None
            }

            _ => None,
        }
    }

    /// Whether this capture has a pair open — the field says «Shift…» and the timer of the pair
    /// runs (task T-95-2).
    pub fn pair_open(&self) -> bool {
        self.pair.open()
    }

    /// The hotkey the callback will be given back when this capture ends — the code and its
    /// modifier set of the first key.
    pub fn published(&self) -> (u16, u16) {
        self.published.0
    }

    /// Both hotkeys the callback will be given back when this capture ends (решение 208.228).
    pub fn published_both(&self) -> ((u16, u16), (u16, u16)) {
        self.published
    }
}

/// The three codes of `Ctrl` — the neutral one the queue reports and the two sides.
const CTRL_KEYS: [u16; 3] = [VK_CONTROL.0, VK_LCONTROL.0, VK_RCONTROL.0];

/// The three codes of `Alt` — the neutral one and the two sides (решение 208.237).
const ALT_KEYS: [u16; 3] = [VK_MENU.0, VK_LMENU.0, VK_RMENU.0];

/// **The side of an `Alt` that went down** — решение 208.237 п. 2, as [`sided_ctrl`] for `Ctrl`: the
/// window messages carry the neutral `VK_MENU`, and the queue holds the sides — the right one
/// (`AltGr`) when it is down, the left otherwise. A sided code is its own side.
pub fn sided_alt(vk: u16) -> u16 {
    if vk != VK_MENU.0 {
        return vk;
    }

    if key_is_down(VK_RMENU) {
        VK_RMENU.0
    } else {
        VK_LMENU.0
    }
}

/// **The side of a `Ctrl` that went down** — решение 208.228, В-4: the window messages carry the
/// neutral `VK_CONTROL`, and the queue holds the sides — the right one when it alone is down, the
/// left otherwise. A sided code is its own side.
pub fn sided_ctrl(vk: u16) -> u16 {
    if vk != VK_CONTROL.0 {
        return vk;
    }

    if key_is_down(VK_RCONTROL) && !key_is_down(VK_LCONTROL) {
        VK_RCONTROL.0
    } else {
        VK_LCONTROL.0
    }
}

impl Drop for CaptureSession {
    /// Gives the hotkey back — and **only** it, since task T-36-6: the code and its modifiers, in
    /// one store (task T-93-1).
    ///
    /// `hook::set_active` is not touched here: the value it must hold after a capture is
    /// `tray.enabled()`, which `app::window_proc` publishes after every message the UI thread
    /// sees, and writing back a copy taken at arming would put a staler value than the one the
    /// program already has.
    fn drop(&mut self) {
        // A capture of the pictures suspended nothing, and gives nothing back (208.237).
        if !self.suspended {
            return;
        }

        // Решение 208.228: both keys, in the one store.
        let (fix, switch) = self.published;

        crate::hook::set_hotkeys(fix, switch);
    }
}
