//! **The file of the settings** — section 7 of SPEC: the name of the file and its schema, the model
//! it is read into, the steps between the versions of the schema, where the file lives, its reading,
//! the policy of its writing, the quarantine of a broken file and the writing itself.
//!
//! Moved out of `settings.rs` whole by task T-120-23 — only moved: no body and no name changed.
//! A child of `settings`, so it sees what it saw there; `settings` re-exports every public item, so
//! every old path — `settings::Config`, `settings::read_or_default`, `settings::write_to` — holds.

use super::*;

/// File name of the configuration inside the program's application data directory.
///
/// Section 7 of SPEC: `%APPDATA%\Lang_Switcher\config.toml`.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// The schema version this build writes and fully understands.
///
/// Version 2 arrived with FR-42а: the default of `[replacement] method` moved from
/// `backspace` to `auto`, and [`step_1_to_2`] carries the files of the old default over to
/// the new one.
///
/// Version 3 arrived with FR-100 and task Т-21-5: the section `[feedback]` and its one field.
/// [`step_2_to_3`] carries the files of schema 2 over, and carries nothing but the stamp — see
/// the rung for why that is the whole of it.
///
/// Version 4 arrived with вопрос **94.1** and task Т-29-1: the ten locales решение 93 added to
/// [`Language`] became a schema of their own. Nothing in the *file* changed meaning — that is
/// why [`step_3_to_4`] is a bare stamp — and the whole of what the number buys is in the other
/// direction: a build that knows two locales now recognises a file of the twelve by its stamp,
/// **before** its parser refuses one of the ten values, and so leaves that file alone instead of
/// moving it to `.bad`. See [`Language`], where the cost this pays off is written out.
///
/// Version 5 arrived with **решение 97.3** and task Т-30-5: Hebrew and Arabic. The same coin
/// spent one tier up, and for the same thing — [`step_4_to_5`] is a bare stamp, and what the
/// number buys is that a twelve-locale build meeting `language = "he"` leaves the file whole
/// instead of quarantining it.
///
/// Version 6 arrived with **вопрос 101** and task Т-32-1: the section `[letters]` of FR-101 and
/// FR-102. The first rung since [`step_1_to_2`] that is **not** a bare stamp — it has a
/// decision to carry, and [`step_5_to_6`] spells it out: a machine that is being updated has
/// already been used, so it must not meet «Привет», and it must meet «Что нового» exactly once.
///
/// Version 7 arrived with **вопрос 157** and task T-93-1: `[hotkey] modifiers`, the combinations
/// `Ctrl`/`Alt`/`Shift` + a key. [`step_6_to_7`] is a bare stamp — an absent list is the bare key a
/// schema 6 file meant — and the number buys the other direction once more: a build that has
/// never heard of the list meets a stamp it does not know, reads the file and leaves it alone,
/// instead of dropping the list on its next write.
///
/// Version 8 arrived with **вопрос 159** and task T-95-1: `[hotkey] trigger`, the double press of
/// `Shift`. [`step_7_to_8`] is a bare stamp again — an absent trigger is `press`, which is what a
/// schema 7 hotkey was — and the number is spent on `0.93.0` and the builds before it: they meet a
/// stamp they do not know and leave the file alone instead of dropping the trigger (measured on the
/// base — посылка П4 of stage Э95: `FromNewerSchema`, the hotkey `Pause`, the bytes untouched).
///
/// Version 9 arrived with **stage E108**, task T-108-2: the thirty languages of group A in the
/// closed set of `general.language`. [`step_8_to_9`] is a bare stamp, and the number is spent on
/// 1.2.0 and the builds before it — they meet a stamp they do not know and leave a file that says
/// `language = "hi"` alone instead of quarantining it.
///
/// Version 10 arrived with **stage E109**, task T-109-1: the twenty-two languages of group B in
/// the closed set. [`step_9_to_10`] is a bare stamp, and the number is spent on 1.3.0 and the
/// builds before it, which would otherwise quarantine a file that says `language = "nb"`.
pub const CURRENT_SCHEMA_VERSION: u32 = 10;

/// The version this build assigns to a file that carries no `schema_version` field.
///
/// Nobody ever wrote this number into a file: it is the label given to the earliest form
/// of the configuration so that the migration ladder in [`Config::migrate`] has a rung to
/// start from.
const PRE_VERSION_SCHEMA: u32 = 0;

/// Serde default for [`Config::schema_version`], see [`PRE_VERSION_SCHEMA`].
///
/// Deliberately *not* the current version: a file that omits the field comes from the
/// earliest schema and has to travel through migration like any other old file.
fn pre_version_schema() -> u32 {
    PRE_VERSION_SCHEMA
}

/// Layout switching mode, `layouts.mode` of section 7. Values `pair` and `cycle`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Switch between [`Layouts::pair_source`] and [`Layouts::pair_target`]. The default
    /// of section 7, confirmed by decision 19: there are exactly two layouts.
    #[default]
    Pair,
    /// Walk the [`Layouts::cycle`] list.
    Cycle,
}

/// Text replacement method, `replacement.method` of section 7. Values `auto`, `backspace`
/// and `selection`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplacementMethod {
    /// Choose between the two real methods by the class of the foreground window at the
    /// moment of the replacement — **FR-42а**, and the default of section 7.
    ///
    /// The choice itself lives in `inject::effective_method`, next to the one
    /// `GetClassNameW` call it is made from; this module only names the value. Schema 2 made
    /// it the default: the measurement of task T-10-7 showed that neither pure method holds
    /// everywhere — `selection` breaks the COOKED input of consoles, `backspace` races the
    /// erasure in ordinary edit controls on repeated presses — so the default became the
    /// choice by window class, and the two words below remain as manual overrides.
    #[default]
    Auto,
    /// Erase the typed run with backspaces and retype it. The default of section 7 up to
    /// schema 1; an explicit manual override since schema 2.
    Backspace,
    /// Select the run just typed with `Shift+Left` and let the insertion replace it — the
    /// compatibility mode of **FR-42**.
    ///
    /// ⚠ **The clipboard takes no part in this.** What is selected is the program's own
    /// freshly typed run, character for character, and the replacement is still delivered by
    /// `KEYEVENTF_UNICODE` (module `inject`). The clipboard route is a different requirement
    /// altogether — the selection *path* of FR-60 to FR-65, driven by `[selection] enabled`
    /// and implemented in module `selection` — and confusing the two is what the earlier
    /// wording of this comment did. Section 4.3 of `STATE.md` carried it as a documentation
    /// debt; task T-08-1 is where it is paid.
    Selection,
}

/// Section `[general]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct General {
    /// Whether correction is armed at all. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether the program starts with the session. Default `true`.
    #[serde(default = "default_true")]
    pub autostart: bool,
    /// Interface language. Default `ru`.
    #[serde(default)]
    pub language: Language,
    /// Theme of everything visible, `general.theme` of section 7 — **FR-92а**. Default
    /// [`ThemeSetting::System`].
    ///
    /// The vocabulary of the value — the three words and the rule that anything else reads
    /// as the default — belongs to module `theme`, which FR-92а names the single owner.
    /// This field only carries the setting through the file: the serde bridge below hands
    /// the string to [`ThemeSetting::from_config_str`] on the way in and writes
    /// [`ThemeSetting::as_config_str`] on the way out, so no theme word is ever spelled
    /// in this module. Note the asymmetry with the neighbours: `language` outside its set
    /// fails the read loudly, while an unknown theme word reads as `system` — that is not
    /// an inconsistency of this module but the letter of FR-92а, built into
    /// `from_config_str` itself.
    #[serde(
        default = "default_theme",
        serialize_with = "theme_to_toml",
        deserialize_with = "theme_from_toml"
    )]
    pub theme: ThemeSetting,
    /// The flag of the layout on the tray icon — task T-119-2, решения 208.88, 208.89, the check
    /// box «Флаг раскладки в трее» of FR-92. Default `false`, and **off means 1.6.1 to the
    /// letter**: no flag, no new subscription, hook or thread (the guard is in `tests\tray.rs`).
    ///
    /// ⚠ `schema_version` does not move for it — the reasoning of `idle_timeout_s` in section 7:
    /// there is no `deny_unknown_fields` here, so an older build reads the file past the key, and
    /// its save drops it, which is the default and a build that has no flag anyway.
    #[serde(default)]
    pub tray_flag: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: true,
            language: Language::Ru,
            theme: ThemeSetting::System,
            tray_flag: false,
        }
    }
}

/// Section `[hotkey]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    /// Name of the virtual key. Default `Pause`.
    ///
    /// Kept as a string here on purpose: turning a key name into a virtual key code and
    /// back is the business of the hotkey capture UI (FR-94, task T-08-2), and this
    /// module must not grow a key table that would then exist in two places.
    #[serde(default = "default_hotkey_key")]
    pub key: String,
    /// **The modifiers held with the key — task T-93-1, вопрос 157.** Empty — the default and
    /// what every file before schema 7 means — is the bare key.
    ///
    /// Names from the closed set `Ctrl`, `Alt`, `Shift` (`hook::HOTKEY_MODIFIERS`); the program
    /// writes them in that order, and a capture never writes anything else. Kept as strings for
    /// the reason [`Self::key`] is: the file is also written by hand, a name nobody knows must not
    /// cost the whole document, and the reading is `hook`'s — [`Self::binding`] answers `None`
    /// for an unknown name and the **whole** hotkey stays the default `Pause`, exactly as for an
    /// unknown key.
    #[serde(default)]
    pub modifiers: Vec<String>,
    /// **How the key is pressed — task T-95-1, вопрос 159**: [`TRIGGER_PRESS`], the default and
    /// what every file before schema 8 means, or [`TRIGGER_DOUBLE_TAP`] — two taps of `Shift` in a
    /// row (157.17: the double press is `Shift`'s alone). Always written, as `modifiers = []` is.
    ///
    /// A string for the reason [`Self::key`] is one: a word nobody knows must not cost the whole
    /// document. [`Self::binding`] answers `None` for it and the **whole** hotkey stays the default
    /// `Pause` — the fate of an unknown key name; and `double_tap` is lawful with `key = "Shift"`
    /// and no modifiers only, with anything else it is the same `None`.
    #[serde(default = "default_hotkey_trigger")]
    pub trigger: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            key: default_hotkey_key(),
            modifiers: Vec::new(),
            trigger: default_hotkey_trigger(),
        }
    }
}

impl Hotkey {
    /// The code and the modifier set this hotkey means — task T-93-1, вопрос 157 — or `None` when
    /// the key or one of the modifiers is a name this build does not know.
    ///
    /// `None` is read by every caller as «the default of section 7 acts»: the start-up publication
    /// leaves `Pause` in place (`app::publish_configuration`), the help names `Pause`
    /// ([`effective_hotkey_name`]) and the settings window says why ([`hotkey_note`]).
    ///
    /// ⭐ **The double press of `Shift` — task T-95-1, вопрос 159**: `trigger = "double_tap"` with
    /// `key = "Shift"` and no modifiers means `VK_SHIFT` with the flag `hook::DOUBLE_TAP` and
    /// nothing else; the trigger with anything else, and a word outside its closed set, is `None`
    /// like an unknown name — the whole hotkey stays `Pause`. A single `Shift` is no hotkey
    /// (157.1): `hook::vk_from_name` knows no modifier, so `key = "Shift"` that is pressed is `None`
    /// too. Names are read the way key names are, in any case and with the blanks around ignored.
    pub fn binding(&self) -> Option<(u16, u16)> {
        let modifiers = crate::hook::modifiers_from_names(&self.modifiers)?;
        let trigger = self.trigger.trim();

        if trigger.eq_ignore_ascii_case(TRIGGER_DOUBLE_TAP) {
            let lawful = modifiers == 0 && self.key.trim().eq_ignore_ascii_case(DOUBLE_TAP_KEY);

            return lawful.then_some((VK_SHIFT.0, crate::hook::DOUBLE_TAP));
        }

        if !trigger.eq_ignore_ascii_case(TRIGGER_PRESS) {
            return None;
        }

        let vk = crate::hook::vk_from_name(&self.key)?;

        Some((vk, modifiers))
    }

    /// The `[hotkey]` that means `vk` with `modifiers` — task T-93-1: the key under the name
    /// section 7 stores ([`key_name`]) and the modifiers **in the canonical order** `Ctrl`, `Alt`,
    /// `Shift` (`hook::HOTKEY_MODIFIERS`), whatever order they were pressed in.
    ///
    /// `None` for a key section 7 has no name for, and for a set with a bit no hotkey carries —
    /// `hook::MOD_WIN` is held, never written.
    ///
    /// The flag `hook::DOUBLE_TAP` reads back to [`Self::double_shift`] — with a `Shift` and
    /// nothing else beside it (task T-95-1); with anything else it is `None`.
    pub fn from_binding(vk: u16, modifiers: u16) -> Option<Self> {
        if modifiers & crate::hook::DOUBLE_TAP != 0 {
            let shift = SHIFT_KEYS.contains(&vk);

            return (shift && modifiers == crate::hook::DOUBLE_TAP).then(Self::double_shift);
        }

        let known = crate::hook::HOTKEY_MODIFIERS
            .iter()
            .fold(0, |set, (_, bit)| set | bit);

        if modifiers & !known != 0 {
            return None;
        }

        Some(Self {
            key: key_name(vk)?,
            modifiers: crate::hook::HOTKEY_MODIFIERS
                .iter()
                .filter(|(_, bit)| modifiers & bit != 0)
                .map(|(name, _)| (*name).to_owned())
                .collect(),
            trigger: default_hotkey_trigger(),
        })
    }

    /// Whether this hotkey is the double press of `Shift` — what [`Self::binding`] reads, so a
    /// `double_tap` that is not lawful is not one (task T-95-2).
    pub fn is_double_tap(&self) -> bool {
        self.binding()
            .is_some_and(|(_, set)| set & crate::hook::DOUBLE_TAP != 0)
    }

    /// The double press of `Shift` as section 7 stores it — `key = "Shift"`, `modifiers = []`,
    /// `trigger = "double_tap"` (task T-95-1, вопрос 159): what a capture of two taps writes.
    pub fn double_shift() -> Self {
        Self {
            key: DOUBLE_TAP_KEY.to_owned(),
            modifiers: Vec::new(),
            trigger: TRIGGER_DOUBLE_TAP.to_owned(),
        }
    }
}

/// `[hotkey] trigger` of a hotkey that is pressed — the default of section 7 (task T-95-1).
pub const TRIGGER_PRESS: &str = "press";

/// `[hotkey] trigger` of the double press of `Shift` — task T-95-1, вопрос 159.
pub const TRIGGER_DOUBLE_TAP: &str = "double_tap";

/// The one key a double press is assigned to — 157.17, «Только Shift»: the name section 7 stores
/// for it, and each half of the name the windows show, «Shift, Shift» (task T-95-1).
pub const DOUBLE_TAP_KEY: &str = "Shift";

/// Section `[layouts]` of section 7, with the defaults confirmed by decision 19.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layouts {
    /// Switching mode. Default `pair`.
    #[serde(default)]
    pub mode: LayoutMode,
    /// Source layout identifier of the pair. Default `0x00000409`, en-US.
    #[serde(default = "default_pair_source")]
    pub pair_source: String,
    /// Target layout identifier of the pair. Default `0x00000419`, ru-RU.
    #[serde(default = "default_pair_target")]
    pub pair_target: String,
    /// Layout identifiers walked in `cycle` mode. Default the same two, in this order.
    #[serde(default = "default_cycle")]
    pub cycle: Vec<String>,
}

impl Default for Layouts {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Pair,
            pair_source: default_pair_source(),
            pair_target: default_pair_target(),
            cycle: default_cycle(),
        }
    }
}

/// Section `[replacement]` of section 7.
///
/// Both defaults of section 7 — `auto` and `0` — coincide with the defaults of the
/// field types, so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    /// How the typed run is replaced. Default `auto` — FR-42а.
    #[serde(default)]
    pub method: ReplacementMethod,
    /// Pause inserted between synthesised input events, milliseconds. Default `0`. Soft —
    /// решение 120.2, `soft_whole_number`.
    #[serde(default, deserialize_with = "soft_inter_event_delay_ms")]
    pub inter_event_delay_ms: u32,
}

/// Section `[selection]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Whether the selection path is available at all. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// How long to wait for the clipboard to carry the selection, milliseconds.
    /// Default `300`. Soft — решение 120.2, `soft_whole_number`.
    #[serde(
        default = "default_clipboard_timeout_ms",
        deserialize_with = "soft_clipboard_timeout_ms"
    )]
    pub clipboard_timeout_ms: u32,
    /// How long to wait before restoring the previous clipboard, milliseconds.
    /// Default `200`. Soft — решение 120.2, `soft_whole_number`.
    #[serde(
        default = "default_clipboard_restore_delay_ms",
        deserialize_with = "soft_clipboard_restore_delay_ms"
    )]
    pub clipboard_restore_delay_ms: u32,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            enabled: true,
            clipboard_timeout_ms: default_clipboard_timeout_ms(),
            clipboard_restore_delay_ms: default_clipboard_restore_delay_ms(),
        }
    }
}

/// Section `[buffer]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buffer {
    /// Capacity of the typing buffer in characters. Default `256`. Soft — решение 120.2,
    /// `soft_whole_number`.
    #[serde(
        default = "default_buffer_capacity",
        deserialize_with = "soft_buffer_capacity"
    )]
    pub capacity: usize,
    /// **How long a word may sit in the buffer with nothing happening to it, seconds — FR-15**,
    /// task T-52-4. Default `300`; `0` switches the rule off.
    ///
    /// Published clamped to [`crate::buffer::MIN_IDLE_TIMEOUT_S`] …
    /// [`crate::buffer::MAX_IDLE_TIMEOUT_S`], with `0` kept as `0`. The file may hold anything —
    /// it is edited by hand — and the ceilings are applied where the value crosses to the input
    /// thread, exactly as the three millisecond fields of task T-13-13 are.
    ///
    /// ⚠ **`schema_version` did not move for this key, and 6 is deliberate.** There is no
    /// `deny_unknown_fields` in this module, so a file written by this build is read by the
    /// previous one without complaint — the key is simply ignored there. Raising the schema to
    /// 7 would do the opposite: a user who installed this build and then went back to `e51`
    /// would find their configuration quarantined as «из будущего»
    /// ([`ReadOutcome::FromNewerSchema`]), which is a real cost paid for a key whose absence
    /// already has a defined meaning.
    ///
    /// Soft — решение 120.2, `soft_whole_number`: a value that is not a whole number this field
    /// can hold is its default, and costs nothing else of the file.
    #[serde(
        default = "default_buffer_idle_timeout_s",
        deserialize_with = "soft_buffer_idle_timeout_s"
    )]
    pub idle_timeout_s: u32,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            capacity: default_buffer_capacity(),
            idle_timeout_s: default_buffer_idle_timeout_s(),
        }
    }
}

/// Section `[exclusions]` of section 7.
///
/// The default of section 7 — an empty list — coincides with the default of the field
/// type, so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusions {
    /// Process names in which the program stays silent. Default an empty list.
    #[serde(default)]
    pub processes: Vec<String>,
}

/// Section `[feedback]` of section 7 — **FR-100**, the sound of a press.
///
/// The user asked for it (question 77, finding №10 of the audit of 2026-08-31): a press that
/// does nothing is indistinguishable from a press that worked, because the program's whole
/// output is text somebody else's window draws. One sound says the replacement happened and
/// another says the press was idle.
///
/// Default **on**, which is not the default of `bool` — hence `default_true`, exactly as
/// `[selection] enabled` does it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feedback {
    /// Whether a press answers with a system sound. Default `true`.
    #[serde(default = "default_true")]
    pub sound: bool,
}

impl Default for Feedback {
    fn default() -> Self {
        Self { sound: true }
    }
}

/// Section `[diagnostics]` of section 7.
///
/// The default of section 7 — logging off — coincides with the default of the field type,
/// so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// Whether the diagnostic log is written to a file. Default `false`.
    ///
    /// SEC-07 constrains what the log may contain; it does not belong to this module.
    #[serde(default)]
    pub log_enabled: bool,
}

/// Section `[letters]` of section 7 — **FR-101 and FR-102**, вопрос 101, task Т-32-1.
///
/// The state of the five letters and of the feed: when this installation started counting,
/// which letters it has shown, which news it has read and when it last looked at the feed.
/// Nothing here is a keystroke, a window title or anything a person typed (SEC-01, SEC-07):
/// the section holds dates, two version strings, a handful of flags and the numbers the author
/// gave the entries of the feed.
///
/// # Only one field of this section is meant to be edited by hand
///
/// `feed` is, and section 7 says so out loud: for the first ninety days it is the **only** way
/// to turn the feed off (FR-102), because the switch in the «От автора» window does not appear
/// until the feed has actually produced a letter. The rest is bookkeeping — a person is free to
/// edit it, and the worst that comes of it is a letter shown twice or not at all.
///
/// # The order of the fields is load-bearing
///
/// TOML puts every plain value of a table before its sub-tables, so the two maps go last and
/// this section goes last in [`Config`]. A map written above a scalar would make the file
/// unreadable to the very parser that wrote it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Letters {
    /// Whether the author's feed is read at all — FR-102, and **on** by default (вопрос 101
    /// п. 3). This is the field SEC-03 names as the way to refuse the one network operation
    /// this program has.
    #[serde(default = "default_true")]
    pub feed: bool,

    /// The day this installation was first used, from which every cadence of FR-101 counts.
    ///
    /// Empty on a file that has never been used — a fresh install and a file just raised from
    /// schema 5 alike — and filled by [`crate::letters::initialise`] on the first run, which
    /// is the day the migration happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_run: Option<letters::Date>,

    /// Whether «Привет» has been shown. False only on a machine that had no configuration file
    /// at all (FR-101); the migration of schema 5 → 6 sets it true, because a machine that is
    /// being updated has been used.
    #[serde(default)]
    pub welcome_shown: bool,

    /// The state of the «Спасибо» letter — `pending`, `snoozed` or `done`.
    #[serde(
        default,
        deserialize_with = "thanks_from_toml",
        serialize_with = "thanks_to_toml"
    )]
    pub thanks: letters::Thanks,

    /// The day «Спасибо» becomes due: thirty days from the first run, or seven from a snooze.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thanks_due: Option<letters::Date>,

    /// The version «Что нового» was last shown for. Empty means «show it once», which is what
    /// the migration leaves behind for a person who has just updated.
    #[serde(default)]
    pub last_seen_version: String,

    /// The day the last letter was put on the screen — the one-letter-a-day rule of FR-101.
    ///
    /// ⚠ **Not in the mock-up's печать of section 7, and added by the executor of Э32 with the
    /// reason written down in the report.** Every other letter is stopped from coming back by a
    /// mark of its own — the version it was about, the day it was first shown, `done` — and
    /// none of those stops *another* letter from following it an hour later, across a restart,
    /// on the same day. FR-101 says «не более одного письма в сутки», and a rule that has to
    /// survive a restart needs a day written in the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_letter: Option<letters::Date>,

    /// The day the feed was last read **and its signature verified** — FR-102. A failed read
    /// does not move it; what keeps a broken host from being asked again the same day is
    /// `feed_last_try` below, and since решение 189.2 the next good read is due the next day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_last_read: Option<letters::Date>,

    /// The day the feed was last **asked**, whatever came of it — FR-102, task T-88-1, решение
    /// 148.8.
    ///
    /// Written by the tick of the letters **before** the read starts. `feed_last_read` alone
    /// cannot say «already asked today»: a failed read does not move it, so until `e88` a host
    /// that did not answer was asked again at every tick, once an hour. A day in the file, like
    /// `last_letter` above, because a rule that has to survive a restart needs one.
    ///
    /// ⚠ **Not in the печать of section 7 of `SPEC.md`**, which stage Э88 does not touch
    /// (решение 148.5). The schema stays 6: an older build ignores the key, as it ignored
    /// `idle_timeout_s`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_last_try: Option<letters::Date>,

    /// The day the first letter out of the feed was shown — half of the condition the feed
    /// switch of FR-102 appears on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_feed_letter: Option<letters::Date>,

    /// The newest version the «Обновление» letter has already been shown for — FR-101, «один
    /// раз на версию».
    #[serde(default)]
    pub latest_known: String,

    /// The day a read of the feed first named a version newer than this build — TZ E104 §0
    /// п. 20, task T-104-7: since stage Э104 the updater installs a new version by itself, and
    /// the letter «Обновление» with the entry «Доступна версия X…» are its **fallback**, shown
    /// only when the feed has named the newer version for [`letters::UPDATE_FALLBACK_DAYS`] days
    /// and this build is still the old one. Cleared by the first read that names nothing newer.
    ///
    /// ⚠ Not in the печать of section 7 and **no schema of its own**, as `feed_last_try` before
    /// it: there is no `deny_unknown_fields` here, an older build reads the file past this key,
    /// and if it drops the key on a write the cost is three more days of the fallback's wait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_noticed: Option<letters::Date>,

    /// Whether the window «Обновлено» comes up after an important release — task T-119-6
    /// (решения 208.93, 208.119). The check box «Больше не показывать окна обновлений» of that
    /// window and the switch «Сообщать об обновлениях программы» of «От автора» (T-119-10) and of
    /// «Общие» of the settings (T-120-20, Д6) are this one setting; it silences only that window —
    /// the entry of the menu and the fallback letter stay. On by default. No schema of its own, as
    /// `update_noticed` before it.
    #[serde(default = "default_true")]
    pub release_window: bool,

    /// While the window «Обновлено» is due and not yet shown, the version the program was updated
    /// **from** («Было Y» of its text) — task T-119-6: written when the start after an important
    /// update finds it due, emptied the moment the window is shown — so it is shown **once**, even
    /// when it was never closed and the computer restarted, and still shown when the program
    /// stopped before its quiet moment came. Empty — nothing is due.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub release_due: String,

    /// The day of the last update — task T-119-7: the entry «Что нового в X…» of the menu stands
    /// for a week from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_on: Option<letters::Date>,

    /// ⭐ **The mark of the first run — stage E116, task T-116-12, решение 208.59.** `due` only in
    /// a file this run created ([`read_from_via`], no file at all); `sent` after any HTTP answer to
    /// the read of the feed that carried `?first=1`. Absent — a file of an older build, an update —
    /// means never. See [`letters::FirstMark`].
    ///
    /// ⚠ Not in the печать of section 7 and **no schema of its own**, as `feed_last_try` and
    /// `update_noticed` before it: an older build reads past the key, and an unknown word is read
    /// as no mark (never sent), not as damage.
    #[serde(
        default,
        deserialize_with = "first_mark_from_toml",
        serialize_with = "first_mark_to_toml",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_mark: Option<letters::FirstMark>,

    /// The identifiers of the news items that have been read — FR-101, and «Прочитано» is the
    /// only thing that ever puts one here.
    #[serde(default)]
    pub read_ids: Vec<u64>,

    /// How many reminders each unread news item has had — at most
    /// [`crate::letters::REMINDERS_PER_NEWS`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reminders: letters::NewsMap<u32>,

    /// The day each news item was **first** shown, which is what its two reminders are counted
    /// from.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub first_shown: letters::NewsMap<letters::Date>,
}

impl Default for Letters {
    /// The section as section 7 prints it for a machine that has never run this program: the
    /// feed on, nothing shown, nothing read, no day counted from yet.
    fn default() -> Self {
        Self {
            feed: true,
            first_run: None,
            welcome_shown: false,
            thanks: letters::Thanks::Pending,
            thanks_due: None,
            last_seen_version: String::new(),
            last_letter: None,
            feed_last_read: None,
            feed_last_try: None,
            first_feed_letter: None,
            latest_known: String::new(),
            update_noticed: None,
            release_window: true,
            release_due: String::new(),
            updated_on: None,
            first_mark: None,
            read_ids: Vec::new(),
            reminders: BTreeMap::new(),
            first_shown: BTreeMap::new(),
        }
    }
}

impl Letters {
    /// Whether this news item has been read — FR-101.
    pub fn is_read(&self, id: u64) -> bool {
        self.read_ids.contains(&id)
    }

    /// Marks a news item read. Idempotent: «Прочитано» pressed twice is one entry.
    pub fn mark_read(&mut self, id: u64) {
        if !self.is_read(id) {
            self.read_ids.push(id);
            self.read_ids.sort_unstable();
        }

        self.reminders.remove(&letters::key_of(id));
    }

    /// The day this news item was first shown, if it has been.
    pub fn first_shown_on(&self, id: u64) -> Option<letters::Date> {
        self.first_shown.get(&letters::key_of(id)).copied()
    }

    /// Records the day a news item was first shown.
    pub fn set_first_shown(&mut self, id: u64, day: letters::Date) {
        self.first_shown.insert(letters::key_of(id), day);
    }

    /// How many reminders this news item has already had.
    pub fn reminders_sent(&self, id: u64) -> u32 {
        self.reminders
            .get(&letters::key_of(id))
            .copied()
            .unwrap_or(0)
    }

    /// Counts one more reminder for this news item, and never past
    /// [`crate::letters::REMINDERS_PER_NEWS`] — a saturating count, because the number is read
    /// back out of a file anybody may edit (SEC-05).
    pub fn count_reminder(&mut self, id: u64) {
        let sent = self.reminders_sent(id);

        self.reminders.insert(
            letters::key_of(id),
            sent.saturating_add(1).min(letters::REMINDERS_PER_NEWS),
        );
    }

    /// Records that the feed has produced a letter — the day of the **first** one, which is
    /// half of the condition the switch of FR-102 appears on, and it is never moved again.
    pub fn note_feed_letter(&mut self, day: letters::Date) {
        if self.first_feed_letter.is_none() {
            self.first_feed_letter = Some(day);
        }
    }

    /// Forgets every news item that is not in `kept` — FR-101: the news the feed no longer
    /// carries goes, and its reminders and its read mark go with it.
    ///
    /// Keys that are not numbers at all — a hand-edited file — are dropped by the same sweep:
    /// nothing can ever match them again, and leaving them would grow the file for ever.
    pub fn retain_news(&mut self, kept: &[u64]) {
        self.read_ids.retain(|id| kept.contains(id));
        self.reminders
            .retain(|key, _| letters::id_of(key).is_some_and(|id| kept.contains(&id)));
        self.first_shown
            .retain(|key, _| letters::id_of(key).is_some_and(|id| kept.contains(&id)));
    }
}

// Serde bridge for `letters.thanks`, on the terms `general.theme` is on: the vocabulary of the
// three words belongs to [`letters::Thanks`], and a derive here would mint a second spelling of
// them. Neither function knows a single word of it.

/// Reads `letters.thanks`: the word goes to the owner of the vocabulary, which answers for
/// every string — see [`letters::Thanks`] for why an unknown word is not an error here.
fn thanks_from_toml<'de, D>(deserializer: D) -> Result<letters::Thanks, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(letters::Thanks::from_config_str(&text))
}

/// Writes `letters.thanks`: the word is the owner's, verbatim.
fn thanks_to_toml<S>(thanks: &letters::Thanks, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(thanks.as_config_str())
}

/// Reads `letters.first_mark` (stage E116, task T-116-12): the word goes to the owner of the
/// vocabulary, [`letters::FirstMark`]; a word it does not know is no mark — never sent — and not
/// damage to the file.
fn first_mark_from_toml<'de, D>(deserializer: D) -> Result<Option<letters::FirstMark>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(letters::FirstMark::from_config_str(&text))
}

/// Writes `letters.first_mark`: the owner's word, verbatim; no mark is not written at all
/// (`skip_serializing_if`).
fn first_mark_to_toml<S>(
    mark: &Option<letters::FirstMark>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match mark {
        Some(mark) => serializer.serialize_str(mark.as_config_str()),
        None => serializer.serialize_none(),
    }
}

/// The whole configuration file of section 7.
///
/// Every field carries a serde default, which is the machine-checkable form of the rule of
/// section 7: *unknown fields are ignored, missing ones are filled from the defaults*. The
/// ignoring half comes for free — no `deny_unknown_fields` anywhere in this module, and
/// two tests hold that in place.
///
/// Section 6.3. The type is plain owned data: `Clone`, `Send`, `Sync` and `'static`, with
/// no interior references and no locks. That is what lets the UI thread clone a whole
/// configuration and publish it to the input thread through an atomic pointer, with no
/// mutex anywhere near the hook path (NFR-04). Publishing itself is not this task's work;
/// not making it impossible is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Schema version of the file. Current value [`CURRENT_SCHEMA_VERSION`].
    ///
    /// A file without the field is treated as [`PRE_VERSION_SCHEMA`] and migrated; see
    /// [`Config::migrate`].
    #[serde(default = "pre_version_schema")]
    pub schema_version: u32,
    /// Section `[general]`.
    #[serde(default)]
    pub general: General,
    /// Section `[hotkey]`.
    #[serde(default)]
    pub hotkey: Hotkey,
    /// Section `[layouts]`.
    #[serde(default)]
    pub layouts: Layouts,
    /// Section `[replacement]`.
    #[serde(default)]
    pub replacement: Replacement,
    /// Section `[selection]`.
    #[serde(default)]
    pub selection: Selection,
    /// Section `[buffer]`.
    #[serde(default)]
    pub buffer: Buffer,
    /// Section `[exclusions]`.
    #[serde(default)]
    pub exclusions: Exclusions,
    /// Section `[feedback]` — FR-100.
    #[serde(default)]
    pub feedback: Feedback,
    /// Section `[diagnostics]`.
    #[serde(default)]
    pub diagnostics: Diagnostics,
    /// Section `[letters]` — FR-101 and FR-102.
    ///
    /// ⚠ **Last on purpose.** It is the only section holding maps, TOML writes a table's
    /// sub-tables after its plain values, and a section that follows one would be swallowed by
    /// it. See [`Letters`].
    #[serde(default)]
    pub letters: Letters,
}

impl Default for Config {
    /// The configuration of section 7 exactly as printed there, `schema_version` included.
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            general: General::default(),
            hotkey: Hotkey::default(),
            layouts: Layouts::default(),
            replacement: Replacement::default(),
            selection: Selection::default(),
            buffer: Buffer::default(),
            exclusions: Exclusions::default(),
            feedback: Feedback::default(),
            diagnostics: Diagnostics::default(),
            letters: Letters::default(),
        }
    }
}

// Serde default helpers. They exist because `#[serde(default)]` on a field reaches for the
// default of the field's *type*, which for `bool` is `false` and for `String` is empty —
// neither of which is what section 7 says.

/// Serde default for the three boolean fields of section 7 that start out on.
fn default_true() -> bool {
    true
}

/// Serde default for `hotkey.trigger` — [`TRIGGER_PRESS`]: a file before schema 8 has no such
/// field, and its hotkey is pressed (task T-95-1).
fn default_hotkey_trigger() -> String {
    TRIGGER_PRESS.to_owned()
}

/// Serde default for `layouts.pair_source`, en-US (decision 19).
fn default_pair_source() -> String {
    "0x00000409".to_owned()
}

/// Serde default for `layouts.pair_target`, ru-RU (decision 19).
fn default_pair_target() -> String {
    "0x00000419".to_owned()
}

/// Serde default for `layouts.cycle`, the same two layouts in this order (decision 19).
fn default_cycle() -> Vec<String> {
    vec![default_pair_source(), default_pair_target()]
}

/// Serde default for `selection.clipboard_timeout_ms`.
fn default_clipboard_timeout_ms() -> u32 {
    300
}

/// Serde default for `selection.clipboard_restore_delay_ms`.
fn default_clipboard_restore_delay_ms() -> u32 {
    200
}

/// Serde default for `buffer.capacity`.
fn default_buffer_capacity() -> usize {
    256
}

/// Serde default for `buffer.idle_timeout_s` — **FR-15**, task T-52-4.
///
/// Five minutes. Long enough that a user who stopped to read something and came back finds the
/// word they were typing still there, short enough that a machine left alone is not keeping a
/// half-typed line in memory for the rest of the day (SEC-01).
fn default_buffer_idle_timeout_s() -> u32 {
    300
}

thread_local! {
    /// How many soft fields the last [`Config::from_toml_str`] on this thread read as their
    /// default — task T-55-7. Taken by the caller with [`take_softened_fields`].
    static SOFTENED: Cell<u32> = const { Cell::new(0) };
}

/// How many soft fields the last read of a configuration on this thread read as their default,
/// and zero from then on — task T-55-7, решение 120.2.
///
/// A side channel, and deliberately one: this module writes no journal, and the value of a field
/// that fell back to its default is, by construction, not in the configuration any more. The tray
/// reads the configuration on its own thread, takes this count right after the read and puts one
/// line in the journal per field. A read that failed as a whole leaves zero — nothing of it is
/// used.
pub fn take_softened_fields() -> u32 {
    SOFTENED.with(|count| count.replace(0))
}

/// Counts one soft field read as its default.
fn note_softened() {
    SOFTENED.with(|count| count.set(count.get().saturating_add(1)));
}

/// A number of section 7 that is soft — решение 120.2, task T-55-7.
///
/// **Soft means: a value of the document that is not a whole number this field can hold is the
/// default of this field**, and it is counted ([`take_softened_fields`]). A negative number, a
/// fraction, a string, a boolean, a number too large for the field — every one of them costs the
/// field and nothing else. Until that task any of them declared the whole document damaged, and the
/// hotkey, the exclusions, the cycle, the language and the theme went to quarantine with it.
///
/// **Soft is about the type, not the range.** A whole number the field can hold passes through as
/// it is, however large or small: the ceilings of `buffer` and `inject` take it to their bounds
/// where the value is published (finding Н49 — words in section 7, not code here), and softness
/// must not be mistaken for them.
///
/// **Five fields, by name, and no more:** `replacement.inter_event_delay_ms`,
/// `selection.clipboard_timeout_ms`, `selection.clipboard_restore_delay_ms`, `buffer.capacity` and
/// `buffer.idle_timeout_s` — the numbers решение 81 left to be edited by hand. The closed sets of
/// section 7 (`general.language`, `layouts.mode`, `replacement.method`) and `schema_version` stay
/// loud: a value outside a closed set must not pass silently, and a stamp read as a default would
/// blind the recognition of a file from a newer schema. `tests\settings.rs` sweeps this file for
/// exactly these five.
///
/// ⚠ A number with a unit glued to it (`300ms`) is not a value at all but a syntax error of the
/// document, which no field ever sees. That typo is answered by the narrow recovery of решение
/// 120.9, task T-55-7a: [`Config::from_toml_str`] takes the one line out of a copy of the text,
/// counts the field here, and reads again.
fn soft_whole_number<'de, D, T>(deserializer: D, default: T) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: TryFrom<i64>,
{
    let value = toml::Value::deserialize(deserializer)?;

    let whole = match value {
        toml::Value::Integer(number) => T::try_from(number).ok(),
        _ => None,
    };

    Ok(whole.unwrap_or_else(|| {
        note_softened();
        default
    }))
}

/// `replacement.inter_event_delay_ms`, soft — [`soft_whole_number`].
fn soft_inter_event_delay_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, 0)
}

/// `selection.clipboard_timeout_ms`, soft — [`soft_whole_number`].
fn soft_clipboard_timeout_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_clipboard_timeout_ms())
}

/// `selection.clipboard_restore_delay_ms`, soft — [`soft_whole_number`].
fn soft_clipboard_restore_delay_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_clipboard_restore_delay_ms())
}

/// `buffer.capacity`, soft — [`soft_whole_number`].
fn soft_buffer_capacity<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_buffer_capacity())
}

/// `buffer.idle_timeout_s`, soft — [`soft_whole_number`].
fn soft_buffer_idle_timeout_s<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_buffer_idle_timeout_s())
}

/// The five soft keys by their `[section]` and name — решение 120.9, task T-55-7a.
///
/// The fields [`soft_whole_number`] softens, spelled here the way they stand in the file, because
/// the recovery of [`Config::from_toml_str`] reads text and not fields. `tests\settings.rs` drives
/// every one of them with a syntax typo, and drives the closed sets, a soft name in a foreign
/// section, a dotted key and the stamp to show that no other line is ever taken out.
const SOFT_KEYS: [(&str, &str); 5] = [
    ("replacement", "inter_event_delay_ms"),
    ("selection", "clipboard_timeout_ms"),
    ("selection", "clipboard_restore_delay_ms"),
    ("buffer", "capacity"),
    ("buffer", "idle_timeout_s"),
];

/// The byte range of the line of `text` that holds `offset`, without its line break — **if** that
/// line assigns one of [`SOFT_KEYS`] in the canonical form, `key = …`, under its own `[section]`.
///
/// The narrow recovery of решение 120.9 asks this about the position a refused parse stopped at.
/// Everything that is not exactly that answers `None`, and the document is then as damaged as it
/// always was: a closed set written without its quotes, a soft name in another section, a dotted
/// key at the top level, a quoted key, a stamp with a unit glued to it.
fn soft_key_line_at(text: &str, offset: usize) -> Option<std::ops::Range<usize>> {
    let offset = offset.min(text.len());
    let start = text.get(..offset)?.rfind('\n').map_or(0, |at| at + 1);
    let end = text
        .get(offset..)?
        .find('\n')
        .map_or(text.len(), |at| offset + at);

    let (key, _) = text[start..end].split_once('=')?;
    let key = key.trim();

    // The section is the nearest header above the line: `[name]`, and never `[[name]]`.
    let header = text[..start]
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('['))?;
    let section = header
        .strip_prefix('[')
        .filter(|rest| !rest.starts_with('['))?
        .split(']')
        .next()?
        .trim();

    SOFT_KEYS.contains(&(section, key)).then_some(start..end)
}

/// Serde default for `general.theme` — the default FR-92а names.
///
/// A function rather than `#[serde(default)]` because [`ThemeSetting`] implements no
/// `Default`: module `theme` keeps the type bare, and giving it one from here would mean
/// editing the owner. The same shape as [`default_true`] and its neighbours.
fn default_theme() -> ThemeSetting {
    ThemeSetting::System
}

// Serde bridge for `general.theme`. [`ThemeSetting`] deliberately carries no serde
// derives: the three words of the value and the rule about every other word belong to
// `ThemeSetting::from_config_str` (FR-92а names module `theme` the single owner), and a
// derive here would mint a second spelling of them. The two functions below only carry
// the string between the file and the owner — neither knows a single theme word.

/// Reads `general.theme` from the file: the string goes to the owner of the vocabulary.
///
/// No error path for the *value*: by the construction of
/// [`ThemeSetting::from_config_str`], every string is an answer — an unknown word is the
/// default, which is how section 7 has garbage pass silently. What does fail is a value
/// that is not a string at all, and that failure is the ordinary type error of the
/// deserializer, same as for every other string field of the schema.
fn theme_from_toml<'de, D>(deserializer: D) -> Result<ThemeSetting, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(ThemeSetting::from_config_str(&text))
}

/// Writes `general.theme` to the file: the word is the owner's, verbatim.
fn theme_to_toml<S>(setting: &ThemeSetting, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(setting.as_config_str())
}

/// What reading a configuration ended up being.
///
/// Returned next to the configuration rather than folded into it, because the caller acts
/// on it: a missing file asks to be created, a migrated file asks to be written back, and
/// a file from a newer schema asks to be left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// There was no file. The defaults are current by construction; nothing was migrated.
    NoFile,
    /// The file already carried [`CURRENT_SCHEMA_VERSION`].
    Current,
    /// The file carried an older schema and was raised to the current one in memory. The
    /// file on disk is untouched until somebody calls [`write_to`].
    Migrated {
        /// The version the file came in with.
        from: u32,
    },
    /// The file was written by a newer build. It was parsed as far as this schema allows
    /// and its `schema_version` was left as it was: this build is not required to
    /// understand such a file, but it is required not to damage it.
    FromNewerSchema {
        /// The version found in the file.
        version: u32,
    },
}

/// Everything that can go wrong reading or writing the configuration.
///
/// SEC-01 and SEC-07. No variant carries text taken from the file. A configuration file
/// can be edited by hand, pasted into or otherwise filled with anything at all, so a
/// parse failure is described by its position and nothing else.
#[derive(Debug)]
pub enum ConfigError {
    /// The file, or the directory holding it, could not be read, written or replaced.
    Io(io::Error),
    /// The file is not valid TOML, or a field holds a value the schema does not admit —
    /// an unknown value of an enumerated field, or a value of the wrong type.
    ///
    /// `at` is the one-based line and column the parser stopped at, absent when it could
    /// not name a position.
    Malformed {
        /// One-based line and column.
        at: Option<(usize, usize)>,
    },
    /// The configuration could not be turned into TOML.
    ///
    /// Carries no detail on purpose: the serializer's own message can quote the offending
    /// value, and a value can have come from a file. Serialising [`Config`] cannot in
    /// practice fail — every field is a plain scalar, string or array of strings — so the
    /// variant exists to keep the error path total rather than to be acted upon.
    Serialize,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "configuration file could not be accessed: {err}"),
            Self::Malformed {
                at: Some((line, column)),
            } => write!(
                f,
                "configuration file is malformed at line {line}, column {column}"
            ),
            Self::Malformed { at: None } => write!(f, "configuration file is malformed"),
            Self::Serialize => write!(f, "configuration could not be serialised to TOML"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Malformed { .. } | Self::Serialize => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Turns a byte offset into the document into a one-based line and column.
///
/// Only the offset reported by the parser is used; the text is walked, never copied and
/// never handed on, which is what keeps SEC-01 intact on the error path. Continuation
/// bytes of multi-byte characters are skipped so that the column counts characters.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let stop = offset.min(text.len());
    let mut line = 1usize;
    let mut column = 1usize;
    for byte in &text.as_bytes()[..stop] {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else if byte & 0b1100_0000 != 0b1000_0000 {
            column += 1;
        }
    }
    (line, column)
}

impl Config {
    /// The configuration the program starts with when it has **no usable file** — **вопрос 95**,
    /// task Т-29-3, and since task T-55-9 (findings С20 and Н67) every road on which the program
    /// lives by defaults.
    ///
    /// [`Config::default`] with one field chosen instead of fixed: `general.language` is the
    /// interface language of this user's Windows when this build has it, and `en` when it has
    /// not — [`system_language_for_a_first_run`].
    ///
    /// ⚠ **Who calls it, by name — and why that is still вопрос 95.** Until task T-55-9 this doc
    /// said «exactly one caller», the `NotFound` arm of [`read_from`], and every other road —
    /// a file that could not be read or parsed, a file from a newer schema, no `%APPDATA%` —
    /// reached [`Config::default`] and its hard `ru`: somebody who rolled a build back met a
    /// Russian menu, and on the road of a newer file could not even change it. The callers now
    /// are the roads on which **there is no file the program could use**:
    ///
    /// * the `NotFound` arm of [`read_from_via`] — no file at all, вопрос 95 itself;
    /// * the branch of [`Config::from_toml_str_via`] for a file from a newer schema that did not
    ///   parse — none of its fields were understood;
    /// * the `Err` arm of [`read_or_default_via`] — a file that could not be read or parsed;
    /// * `tray::Tray::install_at` with no `%APPDATA%` — nothing was read at all (Н67).
    ///
    /// **No byte of any file is touched on any of them.** «Существующий конфиг не трогается
    /// никогда» is about what the file holds, and it holds what it held; what changes is only the
    /// language the program speaks in memory while it has no usable file. A file that parses —
    /// current or migrated — is somebody's file, and a language missing from it is still the `ru`
    /// of section 7, never the system's.
    ///
    /// It is deliberately **not** `Default::default`. A `Default` that reads the operating
    /// system is a `Default` that answers differently on two machines, and this type is
    /// compared for equality all over the tests; the difference has to be visible at the call
    /// site, which is what a named constructor gives.
    pub fn for_a_first_run() -> Self {
        Self::for_a_first_run_in(system_language_for_a_first_run())
    }

    /// [`Config::for_a_first_run`] with the language handed in — task T-55-9. Pure, so that a test
    /// on a machine whose Windows is Russian can still tell the language of the system from the
    /// `ru` of section 7.
    pub fn for_a_first_run_in(language: Language) -> Self {
        Self {
            general: General {
                language,
                ..General::default()
            },
            ..Self::default()
        }
    }

    /// Parses a TOML document and brings it to the current schema version.
    ///
    /// Unknown fields are ignored and missing ones are filled from the defaults, both by
    /// construction rather than by extra code. An empty document therefore parses into
    /// exactly [`Config::default`].
    ///
    /// # A file from a newer schema is recognised even when it will not parse — task T-19-4
    ///
    /// ⭐ **Finding 9 of the audit of 2026-08-31.** Serde ignores fields it has no name for, so
    /// a later schema that only *adds* keys parses here without complaint and reaches
    /// [`Config::migrate`], which stamps it [`ReadOutcome::FromNewerSchema`] and so
    /// [`SavePolicy::Forbidden`]. A later schema that adds one **value** to a field does not:
    /// the three enumerated fields of section 7 are closed on purpose — "a value outside it
    /// must not pass silently" — and the strict parse below refuses the whole document.
    ///
    /// Until this task that refusal came first and `schema_version` was never read at all. The
    /// file became [`ConfigError::Malformed`], `for_read` answered
    /// [`SavePolicy::QuarantineFirst`], and a configuration written by a newer build was moved
    /// into the single `.bad` slot and replaced by the defaults of this schema — the exact
    /// damage `FromNewerSchema` exists to prevent, done to the file the promise names.
    ///
    /// So a refused parse is asked one more question before it is called damage: what schema
    /// does the document claim? The strict parse still runs **first**, and every file of this
    /// schema or older takes exactly the path it always took — a version at or below
    /// [`CURRENT_SCHEMA_VERSION`] is `Malformed` as before, and a newer file that *does* parse
    /// still keeps every field this build understood, which an early return could not have
    /// given it.
    pub fn from_toml_str(text: &str) -> Result<(Self, ReadOutcome), ConfigError> {
        Self::from_toml_str_via(text, &system_language_for_a_first_run)
    }

    /// [`Config::from_toml_str`] with the language of a first run handed in — task T-55-9, finding
    /// С20.
    ///
    /// The one branch that asks for it is the file from a newer schema that did not parse: none of
    /// its fields were understood, so the program lives by defaults, and since that task their
    /// language is the system's ([`Config::for_a_first_run_in`]) rather than the hard `ru` of
    /// section 7 — a person who rolled a build back no longer meets a Russian menu that, on the
    /// road of a newer file, they could not even change. The argument is lazy and no other road
    /// calls it: a document that parses keeps the language it names, and no byte of the file is
    /// touched on any road. [`Config::from_toml_str`] hands in
    /// [`system_language_for_a_first_run`]; `tests\settings.rs` hands in a staged language, the
    /// only way to tell the two answers apart on a machine whose Windows is Russian.
    pub fn from_toml_str_via(
        text: &str,
        system_language: &dyn Fn() -> Language,
    ) -> Result<(Self, ReadOutcome), ConfigError> {
        // Task T-55-7: the count of softened fields belongs to this read and to nothing before it.
        SOFTENED.with(|count| count.set(0));

        let error = match toml::from_str::<Self>(text) {
            Ok(mut config) => {
                let outcome = config.migrate();
                return Ok((config, outcome));
            }
            Err(error) => error,
        };

        // ⭐ **Решение 120.9, task T-55-7a** — a syntax typo on the line of a soft number costs
        // that number and nothing else: see [`Config::without_broken_soft_lines`].
        if let Some(recovered) = Self::without_broken_soft_lines(text, &error) {
            return Ok(recovered);
        }

        // A read that failed as a whole uses nothing it softened on the way.
        SOFTENED.with(|count| count.set(0));

        // The parser refused. If the document names a schema this build is too old for, that
        // refusal is the expected answer to a file from the future and not a verdict on it.
        if let Some(version) = claimed_schema_version(text)
            && version > CURRENT_SCHEMA_VERSION
        {
            // The configuration of a first run, carrying the version the file claims — task
            // T-55-9, finding С20: its language is the system's. Nothing of the file itself is
            // kept — none of it was understood — and nothing of it is lost either:
            // `FromNewerSchema` forbids the write, so the bytes stay where they are.
            let config = Self {
                schema_version: version,
                ..Self::for_a_first_run_in(system_language())
            };

            return Ok((config, ReadOutcome::FromNewerSchema { version }));
        }

        Err(ConfigError::Malformed {
            at: error.span().map(|span| line_and_column(text, span.start)),
        })
    }

    /// The narrow recovery of **решение 120.9** — variant Б of решение 120.2, task T-55-7a.
    ///
    /// A soft number with a typo that breaks the syntax — `300ms`, `3 00` — is not a value its
    /// field could soften: the parser refuses the document before any field is read. So when the
    /// refused parse stopped on the line of a soft key in its own section ([`soft_key_line_at`]),
    /// that line is taken out of a copy of the text, the field is counted as softened, and the
    /// copy is read again — once per such line, and never more times than there are soft keys.
    /// The file is not touched: this is a copy in memory, and the person's typo stays on the disk
    /// where they can see it (an unchanged configuration is not written back, task T-55-4).
    ///
    /// Anything else — a stop on a line that is not a soft key, a copy that still refuses for a
    /// reason outside those lines — answers `None`, and the caller reports the **original**
    /// refusal, position included, exactly as before.
    fn without_broken_soft_lines(
        text: &str,
        error: &toml::de::Error,
    ) -> Option<(Self, ReadOutcome)> {
        let mut copy = text.to_owned();
        let mut stopped_at = error.span()?.start;

        for _ in 0..SOFT_KEYS.len() {
            let line = soft_key_line_at(&copy, stopped_at)?;

            copy.replace_range(line, "");
            note_softened();

            match toml::from_str::<Self>(&copy) {
                Ok(mut config) => {
                    let outcome = config.migrate();
                    return Some((config, outcome));
                }
                Err(next) => stopped_at = next.span()?.start,
            }
        }

        None
    }

    /// Renders the configuration as a TOML document holding every section of section 7.
    ///
    /// The whole structure is written every time, so a field the caller never touched
    /// survives the write. A field that is not part of the schema does not: section 7 has
    /// it ignored on the way in, and there is nothing left to write it back from.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        toml::to_string(self).map_err(|_| ConfigError::Serialize)
    }

    /// Raises the configuration to [`CURRENT_SCHEMA_VERSION`], one version at a time.
    ///
    /// The ladder below is the whole mechanism, and it is meant to be extended by adding
    /// rungs, never by rewriting: schema 2 costs one `step_1_to_2` function and one `if`
    /// underneath the existing one. Files from the future are recognised and returned
    /// untouched — this build cannot know what a later schema means, and guessing would
    /// destroy the very fields it does not understand.
    fn migrate(&mut self) -> ReadOutcome {
        let from = self.schema_version;
        if from > CURRENT_SCHEMA_VERSION {
            return ReadOutcome::FromNewerSchema { version: from };
        }
        if from == CURRENT_SCHEMA_VERSION {
            return ReadOutcome::Current;
        }
        if self.schema_version < 1 {
            step_0_to_1(self);
        }
        if self.schema_version < 2 {
            step_1_to_2(self);
        }
        if self.schema_version < 3 {
            step_2_to_3(self);
        }
        if self.schema_version < 4 {
            step_3_to_4(self);
        }
        if self.schema_version < 5 {
            step_4_to_5(self);
        }
        if self.schema_version < 6 {
            step_5_to_6(self);
        }
        if self.schema_version < 7 {
            step_6_to_7(self);
        }
        if self.schema_version < 8 {
            step_7_to_8(self);
        }
        if self.schema_version < 9 {
            step_8_to_9(self);
        }
        if self.schema_version < 10 {
            step_9_to_10(self);
        }
        ReadOutcome::Migrated { from }
    }
}

/// The `schema_version` a document claims, read without the schema of this build — task
/// **T-19-4**.
///
/// Deliberately weak, and every way it can answer nothing is a way of saying "not a file from
/// the future": a document that is not TOML at all, one with no `schema_version`, one whose
/// stamp is not an integer, one whose stamp will not fit a `u32`. Each of those is a file this
/// build cannot read *and* cannot excuse, so each falls through to [`ConfigError::Malformed`]
/// and its quarantine, which is exactly where it belongs.
///
/// The document is parsed a second time here rather than once into a [`toml::Table`] and then
/// deserialised out of it. This runs on the failure path only — once per session, on a file that
/// is already refused — and the strict parse above stays the plain, obvious call it has always
/// been.
fn claimed_schema_version(text: &str) -> Option<u32> {
    let document = text.parse::<toml::Table>().ok()?;

    u32::try_from(document.get("schema_version")?.as_integer()?).ok()
}

/// Raises a file from the pre-versioning form to schema 1.
///
/// Schema 0 is the configuration as it was before the version marker existed: the same
/// sections under the same names, only unstamped. No field changed name, type or meaning,
/// so everything a schema 0 file holds has already been read by the current parser by the
/// time this runs, and raising the file means stamping the version onto it. Later steps
/// are where field surgery goes; the shape of the ladder is what this one establishes.
fn step_0_to_1(config: &mut Config) {
    config.schema_version = 1;
}

/// Raises a file from schema 1 to schema 2 — **FR-42а**, the new default of section 7.
///
/// Up to schema 1 the default of `[replacement] method` was `backspace`, and the file always
/// carried the field, because [`write_to`] writes the whole structure. So `backspace` in a
/// schema ≤ 1 file is what the absence of a choice looks like — the value the program put
/// there itself — and it is raised to the new default, `auto`.
///
/// ⚠ **The other half of that reading — решение 120.3, finding Т9.** A file without
/// `schema_version`, or with one below 2, is taken for one the program wrote, and nothing in it
/// can say otherwise: a `backspace` typed into such a file by hand is raised to `auto` the same
/// way, and the choice it stood for is lost. Section 7 of SPEC.md says so where a person reads
/// about the file.
///
/// `selection` is left exactly as it is: it was never a default of any schema, so a file
/// holding it holds a decision a person made (the compatibility mode of FR-42), and a
/// migration that overrode a decision would be damage, not maintenance.
///
/// A **current** file with `backspace` in it never reaches this rung — [`Config::migrate`]
/// runs it for versions below 2 only — which is what keeps `backspace` usable as an explicit
/// manual override from schema 2 onwards.
fn step_1_to_2(config: &mut Config) {
    if config.replacement.method == ReplacementMethod::Backspace {
        config.replacement.method = ReplacementMethod::Auto;
    }
    config.schema_version = 2;
}

/// Raises a file from schema 2 to schema 3 — **FR-100**, the section `[feedback]`.
///
/// **The stamp and nothing else, and that is the whole decision rather than an omission.** A
/// schema 2 file was written by a build that had never heard of `[feedback]`, so it carries no
/// such section — and the serde default of the one field it would carry is `true`, the same
/// value a fresh configuration of this build gets. There is therefore nothing to raise: the
/// absence of the section already means exactly what the new default means.
///
/// Contrast [`step_1_to_2`], which does have surgery to do: `backspace` in an old file was the
/// value the *program* had put there, so it had to be told apart from a decision a person made.
/// A section that never existed carries no decision of anybody's to preserve.
fn step_2_to_3(config: &mut Config) {
    config.schema_version = 3;
}

/// Raises a file from schema 3 to schema 4 — **вопрос 94.1**, the twelve locales of решение 93.
///
/// **The stamp and nothing else, and — as with [`step_2_to_3`] — that is the whole decision
/// rather than an omission.** Решение 93 added ten admissible **values** to `general.language`;
/// it renamed no field, retyped none and changed the meaning of none. So every value a schema 3
/// file can hold means under schema 4 exactly what it meant before, and there is nothing to
/// raise: `ru` is still `ru`, and a file that says nothing about the language still gets the
/// default of section 7.
///
/// The number is not therefore idle. It is spent entirely in the **other** direction, on a build
/// that has *not* been updated: from here on a file of the twelve locales carries a stamp no
/// two-locale build ever wrote, so such a build recognises it as a file from the future by the
/// stamp — before its parser refuses one of the ten new words — and leaves it whole instead of
/// moving it to `.bad`. That is the cost [`Language`] used to write down as accepted, and this
/// rung is where it stops being paid.
///
/// Contrast [`step_1_to_2`], the one rung that does have surgery to do: `backspace` in an old
/// file was a value the *program* had put there, so it had to be told apart from a decision a
/// person made. A value that was already the person's own carries no such ambiguity.
fn step_3_to_4(config: &mut Config) {
    config.schema_version = 4;
}

/// Raises a file from schema 4 to schema 5 — **решение 97.3**, Hebrew and Arabic.
///
/// **The stamp and nothing else**, for the same reason [`step_3_to_4`] is one rung down: вопрос
/// 97 adds two admissible **values** to `general.language` and renames no field, retypes none
/// and changes the meaning of none. Every value a schema 4 file can hold means under schema 5
/// exactly what it meant before.
///
/// And the number is spent in the same direction — on the build that has *not* been updated.
/// `Language` is a **closed** set, so a twelve-locale build meeting `language = "he"` refuses
/// the whole document; without a stamp it had never seen, it would read its own schema number
/// back out of the file, call it damaged, and move the person's configuration to `.bad`. That is
/// the cost вопрос 94.1 measured at the tier below and paid with schema 4; this rung is where it
/// is paid again, for the same coin, one tier up.
///
/// ⚠ The letters window of the parallel work planned schema 5 for itself and moves on to **6**
/// (вопрос 97.3): two windows cannot spend the same number.
fn step_4_to_5(config: &mut Config) {
    config.schema_version = 5;
}

/// Raises a file from schema 5 to schema 6 — **вопрос 101**, the section `[letters]` of FR-101
/// and FR-102.
///
/// **The first rung since [`step_1_to_2`] that carries a decision rather than a stamp**, and
/// the decision is about who the file belongs to: somebody who has been using this program
/// already. Three things follow from that, and the mandate of Э32 names all three.
///
/// * `welcome_shown = true`. «Привет» is the letter of a first run, and this is not one. The
///   absence of the section would otherwise read as «never shown» — which is exactly what it
///   means on a machine with no file at all, and exactly what it must not mean here.
/// * `last_seen_version` left **empty**. A person who has just updated is the one person «Что
///   нового» is written for, and an empty version differs from every build's, so they meet it
///   once and then never again.
/// * `thanks` and the rest left at their defaults. `first_run` stays empty and is filled on the
///   first run after the migration — [`crate::letters::initialise`] — which is the day of the
///   migration, because this function runs while the program is starting. It is not written
///   here for one reason: [`Config::migrate`] is a pure function of the document, it reads no
///   clock, and every test of the ladder would otherwise answer differently tomorrow.
///
/// ⚠ The `feed` field is left at its default, which is **on** (вопрос 101 п. 3). That is the
/// one thing in this rung that is a change of behaviour for an existing installation, and it is
/// the decision the user took: the feed is announced in the «От автора» window from the first
/// day, refusable in this very file from the first day, and refusable with a switch once it has
/// actually shown a letter.
fn step_5_to_6(config: &mut Config) {
    config.letters.welcome_shown = true;
    config.letters.last_seen_version = String::new();
    config.schema_version = 6;
}

/// Raises a file from schema 6 to schema 7 — **вопрос 157**, task T-93-1: `[hotkey] modifiers`.
///
/// **The stamp and nothing else, and that is the whole decision.** A schema 6 file carries no
/// `modifiers`, and its absence reads as the empty list — the bare key, which is exactly what a
/// schema 6 hotkey was. There is nothing to raise.
///
/// The number is spent in the other direction, as [`step_3_to_4`]'s was. There is no
/// `deny_unknown_fields` in this module, so a build that has never heard of the list would read a
/// file of the combinations as one of its own, answer to the bare key, and — under an unchanged
/// stamp — drop the list the next time it writes the file (on «Применить», on the date of the
/// letters). Under a stamp it has never seen, it reads the file and leaves it alone
/// ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]).
fn step_6_to_7(config: &mut Config) {
    config.schema_version = 7;
}

/// Raises a file from schema 7 to schema 8 — **вопрос 159**, task T-95-1: `[hotkey] trigger`.
///
/// **The stamp and nothing else**, as [`step_6_to_7`] one rung down: a schema 7 file carries no
/// `trigger`, and its absence reads as `press` — the hotkey a schema 7 file meant. There is nothing
/// to raise.
///
/// The number is spent in the other direction once more. `0.93.0` has no `deny_unknown_fields`
/// either, so under an unchanged stamp it would read a file of the double press, find a key named
/// `Shift` it does not know, answer to `Pause` — and drop the trigger the next time it writes the
/// file. Under a stamp it has never seen, it reads the file and leaves it alone
/// ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]).
fn step_7_to_8(config: &mut Config) {
    config.schema_version = 8;
}

/// Raises a file from schema 8 to schema 9 — **stage E108**, task T-108-2: the thirty languages
/// of group A in the closed set of `general.language`.
///
/// **A bare stamp**, as [`step_3_to_4`] and [`step_4_to_5`] were for the same reason: nothing in a
/// schema 8 file changes meaning — it names one of the fourteen, and they are all still here. The
/// number buys the other direction (память `closed-enum-downgrade-cost`): 1.2.0 and the builds
/// before it meet `language = "hi"` under a stamp they have never written, read the file and leave
/// it alone ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]) instead of refusing the
/// word and moving the whole configuration to `.bad`.
fn step_8_to_9(config: &mut Config) {
    config.schema_version = 9;
}

/// Raises a file from schema 9 to schema 10 — **stage E109**, task T-109-1: the twenty-two
/// languages of group B in the closed set of `general.language`.
///
/// **A bare stamp** for the reason of [`step_8_to_9`]: a schema 9 file names one of the forty-four,
/// and they are all still here. 1.3.0 and the builds before it meet `language = "nb"` under a stamp
/// they have never written and leave the file alone instead of moving it to `.bad`.
fn step_9_to_10(config: &mut Config) {
    config.schema_version = 10;
}

/// Builds the configuration path inside an arbitrary application data directory.
///
/// Split out from [`default_config_path`] so that the layout of the path can be asserted
/// without an environment and without touching the real `%APPDATA%`.
pub fn config_path_in(app_data: &Path) -> PathBuf {
    app_data.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME)
}

/// The standard configuration path, `%APPDATA%\Lang_Switcher\config.toml` (section 7).
///
/// `None` when `APPDATA` is not set in the environment, which for an interactive Windows
/// session does not happen; the caller decides whether that is fatal. Read from the
/// environment rather than through `SHGetKnownFolderPath`: the value is already in the
/// process, and a Win32 call would mean an `unsafe` block (NFR-14) and a return value to
/// check (NFR-13) for nothing gained.
pub fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| config_path_in(Path::new(&app_data)))
}

/// The name of **the last good feed kept on disk** — task T-119-9, решение 208.98: the signed
/// document exactly as the site gave it, beside the configuration.
pub const KEPT_FEED_FILE_NAME: &str = "news.toml";

/// Where the last good feed is kept inside an arbitrary application data directory —
/// `%APPDATA%\Lang_Switcher\news.toml` for the product ([`kept_feed_path`]).
pub fn kept_feed_path_in(app_data: &Path) -> PathBuf {
    app_data.join(CONFIG_DIR_NAME).join(KEPT_FEED_FILE_NAME)
}

/// The standard place of the kept feed — [`kept_feed_path_in`] over `%APPDATA%`, `None` without it.
pub fn kept_feed_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| kept_feed_path_in(Path::new(&app_data)))
}

/// Reads the configuration from `path`.
///
/// A missing file is not a failure: on a first run there is none, and section 7 has
/// missing values come from the defaults, so [`ReadOutcome::NoFile`] is returned together
/// with [`Config::default`]. A file that cannot be parsed *is* a failure and is reported
/// as one — the caller that must not fail because of it is [`read_or_default`].
///
/// Reading never writes anything back, migration included: what to do with a file that
/// was migrated, or with one from a newer schema, is the caller's decision.
pub fn read_from(path: &Path) -> Result<(Config, ReadOutcome), ConfigError> {
    read_from_via(path, |path| fs::read(path))
}

/// [`read_from`] with the reading of the bytes handed in — task T-55-6, решение 120.1.
///
/// # A file that could not be read is not a damaged file
///
/// Until that task every failure to read went the way of a file that could not be parsed: the
/// session got [`SavePolicy::QuarantineFirst`], and the file was moved to `config.toml.bad` and
/// replaced by defaults at the first save. A file held open by another program for a moment, or
/// closed to this account, is most likely whole and fine — so a read that fails is **tried once
/// more, at once, inside the same attempt** (not a loop, and not «in N seconds»), and a read that
/// fails twice is answered as [`ConfigError::Io`], which [`SavePolicy::for_read`] turns into
/// [`SavePolicy::NotRead`]: nothing written and nothing moved, this session. A file that is gone by
/// the second attempt was there a moment ago — somebody is replacing it — and is answered the same
/// way rather than as a first run.
///
/// # Bytes that are not UTF-8 are damage
///
/// A file an editor saved as UTF-16, or as ANSI with a letter outside ASCII, was read whole: it is
/// the content this build cannot take, and the verdict is [`ConfigError::Malformed`] with no
/// position — the quarantine such a file always went to. Calling it «not read» would leave the
/// program unable to save, for good, until somebody found the encoding.
///
/// # Why the reading is an argument
///
/// The retry is the point, and a test cannot make the file system fail exactly once on cue:
/// `tests\settings.rs` hands in a closure that fails and counts, and [`read_from`] hands in
/// `fs::read`.
pub fn read_from_via(
    path: &Path,
    mut read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> Result<(Config, ReadOutcome), ConfigError> {
    let bytes = match read(path) {
        Ok(bytes) => bytes,
        // ⭐ **Вопрос 95, task Т-29-3 — the one road on which the system is consulted.** There
        // is no file, so there is nothing of anybody's to preserve, and this is a first run:
        // the language of the configuration becomes the language of the user's Windows if this
        // build has it, and `en` if it has not.
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let mut config = Config::for_a_first_run();

            // ⭐ Stage E116, task T-116-12 (решение 208.59): the mark of the first run is written
            // here and nowhere else — this run creates the file. Every other road of
            // `for_a_first_run` (a damaged file, a file of a newer schema, no `%APPDATA%`) has a
            // file or no place for one, and is not a first run.
            config.letters.first_mark = Some(letters::FirstMark::Due);

            return Ok((config, ReadOutcome::NoFile));
        }
        // Решение 120.1: once more, at once — and whatever the second attempt answers is the
        // answer.
        Err(_) => read(path).map_err(ConfigError::Io)?,
    };

    // Read whole, and not text: damage, not a failed read.
    let Ok(text) = String::from_utf8(bytes) else {
        return Err(ConfigError::Malformed { at: None });
    };

    Config::from_toml_str(&text)
}

/// Reads the configuration from `path` and falls back to the defaults on any failure.
///
/// This is the entry point of a resident utility at startup. A configuration file with a
/// stray bracket in it must not keep the program from running, so the configuration comes
/// back usable in every case; the second half of the pair says what actually happened, so
/// that the caller can report it (`diag`) and decide whether the file may be overwritten.
/// A failed read never leads to the file being rewritten from here: overwriting is a
/// decision, not a side effect.
pub fn read_or_default(path: &Path) -> (Config, Result<ReadOutcome, ConfigError>) {
    read_or_default_via(path, &system_language_for_a_first_run)
}

/// [`read_or_default`] with the language of a first run handed in — task T-55-9, finding С20.
///
/// A file that could not be read or parsed leaves the program on defaults, and since that task
/// their language is the system's ([`Config::for_a_first_run_in`]) rather than the hard `ru` of
/// section 7: the program has no usable file there, which is exactly the case вопрос 95 answered
/// for a file that is missing. No byte of the file is touched — a damaged file is still kept
/// aside before the first write, a file that could not be read is still left alone — and a file
/// that parses keeps the language it names. The argument is lazy and asked only on the failed
/// road; [`read_or_default`] hands in [`system_language_for_a_first_run`], `tests\settings.rs` a
/// staged language.
pub fn read_or_default_via(
    path: &Path,
    system_language: &dyn Fn() -> Language,
) -> (Config, Result<ReadOutcome, ConfigError>) {
    match read_from(path) {
        Ok((config, outcome)) => (config, Ok(outcome)),
        Err(err) => (Config::for_a_first_run_in(system_language()), Err(err)),
    }
}

// -----------------------------------------------------------------------------------------
// The decision `read_or_default` exists to be given — task T-13-6
// -----------------------------------------------------------------------------------------

/// Whether the configuration file may be written back, and what has to happen first.
///
/// **This is the delivery of the second half of the [`read_or_default`] pair to the place that
/// decides.** That pair exists so that "overwriting is a decision, not a side effect" can be
/// true of a caller which keeps the configuration in memory and writes it out again later —
/// [`crate::tray::Tray`], the one caller of [`write_to`] in the whole program. A value of this
/// type *is* that decision: taken once, at the moment of the read, and carried for the rest of
/// the session, because every later write is about the same file this read looked at.
///
/// The type is deliberately about the **file**, not about the configuration: the configuration
/// that comes back from [`read_or_default`] is usable in every case, and nothing here changes
/// that. A resident utility runs on the defaults of section 7 whatever the file says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePolicy {
    /// The file may be written whenever the owner of the configuration asks.
    ///
    /// What every successful read answers, [`ReadOutcome::Migrated`] included: everything the
    /// file held has been understood and is in memory, so writing the memory back loses none
    /// of it. This is the ordinary case and the behaviour that was here before.
    Allowed,
    /// The bytes on disk are somebody's text that this build read whole and could not parse —
    /// since task T-55-6 only that; a file that could not be read at all is
    /// [`SavePolicy::NotRead`]. They are moved to [`quarantine_path_for`] before the first write,
    /// and if they cannot be moved, **nothing is written at all**.
    ///
    /// Section 7 leaves the file editable by hand and the dialog of FR-92 has no field for
    /// `[buffer] capacity`, so editing it by hand is the only way to set one. A stray bracket
    /// in it must therefore not cost the rest of it: the exclusions, the cycle order and the
    /// hotkey are still the user's, still on the disk, and the program has no business
    /// replacing them with defaults it merely fell back on.
    QuarantineFirst,
    /// The file must not be written at all, for the whole session.
    ///
    /// ⚠ **FR-83 asks for «сохранение конфигурации» at shutdown, and this refuses to perform
    /// it. The refusal is what performs it.** The reading is the one this module has already
    /// written down for itself at [`ReadOutcome::FromNewerSchema`]: "this build is not required
    /// to understand such a file, but it is required not to damage it". A file from a newer
    /// schema holds fields this build has no name for; [`Config::to_toml_string`] writes the
    /// schema it knows and nothing else, and [`Config::migrate`] deliberately leaves the newer
    /// `schema_version` in place. One write would therefore delete the unknown fields *and*
    /// leave the stamp that tells the newer build there is nothing to migrate — a loss neither
    /// build could afterwards detect. For such a file **not saving is what saving the user's
    /// configuration means**, and that is the sense in which FR-83 is honoured here rather than
    /// broken.
    Forbidden,
    /// The file is there and could not be read at all — held open by another program, closed to
    /// this account, a failing disk — **решение 120.1**, task T-55-6.
    ///
    /// Nothing is written and nothing is moved this session. A failure to read says nothing about
    /// what is in the file: most likely it is the person's whole configuration, whole and fine,
    /// and the next start will read it. Until that task this was [`SavePolicy::QuarantineFirst`] —
    /// the file moved to `config.toml.bad` and replaced by defaults at the first save — which turned
    /// a moment of contention into lost settings. The read has already been tried twice by then
    /// ([`read_from`]); the session runs on the defaults, and the file stays exactly where it is.
    ///
    /// **Not [`SavePolicy::Forbidden`]**, although it forbids the same writes: the cause is
    /// different — the file is not from the future, it simply could not be read just now — and
    /// the journal says which of the two it was.
    NotRead,
}

impl SavePolicy {
    /// The decision the second half of a [`read_or_default`] pair calls for.
    ///
    /// **A file that could not be parsed and a file that could not be read are two causes with
    /// two fates — решение 120.1, task T-55-6.** [`ConfigError::Malformed`] is text this build
    /// read whole and did not understand: [`SavePolicy::QuarantineFirst`], the bytes kept aside
    /// before the first write. [`ConfigError::Io`] is a file that is there and could not be read,
    /// twice: [`SavePolicy::NotRead`], nothing written and nothing moved this session.
    ///
    /// ⚠ **This reverses what task T-13-6 decided and section 10 made a rule of.** Every failed
    /// read used to map to `QuarantineFirst`, on the reasoning that a file that could not be
    /// opened is just as much text this build has not seen. That is true, and it is why neither
    /// arm here overwrites the file; but moving the file aside was the wrong half of the answer —
    /// a file held open for a moment is most likely whole and fine, and the quarantine turned that
    /// moment into lost settings. There is still no failure of a read after which overwriting the
    /// file is known to be safe, and still no arm here that says it is.
    pub fn for_read(outcome: &Result<ReadOutcome, ConfigError>) -> Self {
        match outcome {
            Ok(ReadOutcome::NoFile | ReadOutcome::Current | ReadOutcome::Migrated { .. }) => {
                Self::Allowed
            }
            Ok(ReadOutcome::FromNewerSchema { .. }) => Self::Forbidden,
            Err(ConfigError::Io(_)) => Self::NotRead,
            Err(ConfigError::Malformed { .. } | ConfigError::Serialize) => Self::QuarantineFirst,
        }
    }

    /// Whether `HKCU\…\Run` may be made to follow a configuration at `moment` — **the one rule of
    /// решение 120.4 (ж) and task T-55-2**.
    ///
    /// **The rule in one sentence: the `Run` key follows only a configuration a person stands
    /// behind** — at the start, the one read whole out of their file or made by a first run
    /// ([`SavePolicy::Allowed`]); on request, the one they have just chosen, as long as their file
    /// may be written at all (neither [`SavePolicy::Forbidden`] nor [`SavePolicy::NotRead`]).
    ///
    /// The two moments differ on exactly one policy, and on purpose. Under
    /// [`SavePolicy::QuarantineFirst`] the configuration at the start is the defaults standing in
    /// for text this build could not parse: nobody chose them, and the person's own autostart is
    /// somewhere in the bytes that were not understood, so the registry is left alone. The same
    /// policy on request is a person choosing in the dialog, and their choice is what the file will
    /// carry once the damaged bytes are moved aside — so the registry follows. Under
    /// [`SavePolicy::Forbidden`] the file is from a newer build and is not written this session,
    /// and under [`SavePolicy::NotRead`] (task T-55-6) it could not even be read: a registry made
    /// to follow anything then would disagree with that file for good.
    ///
    /// ⚠ A session with no `%APPDATA%` has read nothing and is not a policy; its caller,
    /// [`crate::tray`], answers that case where it asks.
    pub fn lets_the_run_key_follow(self, moment: RunKeyMoment) -> bool {
        match self {
            Self::Allowed => true,
            Self::QuarantineFirst => moment == RunKeyMoment::Request,
            Self::Forbidden | Self::NotRead => false,
        }
    }
}

/// The two moments at which `HKCU\…\Run` could be made to follow a configuration — решение 120.4
/// (ж), tasks T-55-1 and T-55-2. What each may do is [`SavePolicy::lets_the_run_key_follow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKeyMoment {
    /// The start of the program: the configuration is whatever the one read produced.
    Start,
    /// A person asked — «Применить» of FR-92 or the check mark of FR-91 — and the configuration is
    /// what they have just chosen.
    Request,
}

/// What is appended to the name of a configuration file that is moved out of the way.
///
/// One name, with nothing unique in it, so that exactly one copy is ever kept: a program that
/// left a numbered trail of unreadable configurations in somebody's `%APPDATA%` would be
/// solving a small problem by making a permanent one.
pub const QUARANTINE_SUFFIX: &str = ".bad";

/// Where [`quarantine`] moves a configuration file this build could not read.
///
/// Beside the original and named after it — `config.toml.bad` next to `config.toml` — so that
/// whoever opens the folder of section 7 finds the two together and can see what happened by
/// looking. Same shape as [`temporary_path_for`], and same reason for the shape: a rename is
/// only atomic within one directory.
pub fn quarantine_path_for(path: &Path) -> PathBuf {
    let mut name = match path.file_name() {
        Some(name) => name.to_os_string(),
        None => OsString::from(CONFIG_FILE_NAME),
    };
    name.push(QUARANTINE_SUFFIX);
    path.with_file_name(name)
}

/// What [`quarantine`] found at the path it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quarantined {
    /// A file was there and is now at [`quarantine_path_for`], byte for byte.
    Moved,
    /// There was nothing to move. Nothing of the user's is at risk, so the caller may write.
    NothingThere,
}

/// Moves a configuration this build could not read out of the way, byte for byte.
///
/// **A rename and never a rewrite.** The point of the copy is the text a person typed, which
/// this build by definition failed to parse — so there is nothing "understood" to write out,
/// and writing out the part that did parse would be the very loss this exists to prevent. The
/// file is moved with its bytes untouched: not re-encoded, not re-serialised, not truncated.
///
/// A previous `.bad` is replaced: `fs::rename` overwrites an existing destination on Windows,
/// which is what keeps the count of copies at one.
///
/// # The two ways it ends
///
/// [`Quarantined::NothingThere`] is answered when the source is gone — the file was deleted
/// between the read and the save. Source and destination share a directory by construction, so
/// a `NotFound` can only be about the source, and a source that does not exist has no bytes to
/// lose: the caller is free to write.
///
/// Any other failure comes back as [`ConfigError::Io`] and is an instruction to the caller,
/// not a detail to log: the bytes on the disk are still the only copy, so nothing may be
/// written over them (NFR-13 — the result is acted on, and the direction it is acted on in is
/// the one that keeps the file).
pub fn quarantine(path: &Path) -> Result<Quarantined, ConfigError> {
    crate::sandbox::guard_path(path);
    match fs::rename(path, quarantine_path_for(path)) {
        Ok(()) => Ok(Quarantined::Moved),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Quarantined::NothingThere),
        Err(err) => Err(ConfigError::Io(err)),
    }
}

/// Writes the configuration to `path`, atomically, creating the directory if needed.
///
/// The write goes to a temporary file beside the target, is flushed to the device, and
/// only then replaces the target with a single rename. The rename is same-directory and
/// therefore same-volume, which is what makes it atomic; `fs::rename` replaces an existing
/// destination on Windows — **unless the destination carries the read-only attribute**, which
/// [`replace_read_only_once`] answers (task T-55-3). A crash, a power cut or a killed process
/// leaves either the old file or the new one, never half of either — the configuration is
/// written by the UI thread while the program runs, and a truncated file would silently reset
/// every setting the user has.
///
/// The temporary file is removed if anything after its creation fails, so a failed write
/// leaves no litter next to the configuration.
pub fn write_to(path: &Path, config: &Config) -> Result<WriteOutcome, ConfigError> {
    let text = config.to_toml_string()?;

    write_bytes_to(path, text.as_bytes())
}

/// **The body of [`write_to`] for any bytes** — task T-119-9: the last good feed is kept beside the
/// configuration ([`kept_feed_path_in`]) the same way, temporary file, flush, one rename, so that a
/// killed process leaves the old copy or the new one and never half of either.
pub fn write_bytes_to(path: &Path, bytes: &[u8]) -> Result<WriteOutcome, ConfigError> {
    crate::sandbox::guard_path(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path_for(path);
    if let Err(err) = write_and_sync(&temporary, bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(err);
    }
    match replace_read_only_once(&temporary, path) {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let _ = fs::remove_file(&temporary);
            Err(ConfigError::Io(err))
        }
    }
}

/// What [`write_to`] did — task T-55-3.
///
/// The ordinary answer is [`WriteOutcome::Written`]. The other one is a write that landed only
/// after the read-only attribute of the target was cleared, and the caller — the tray, the one
/// writer of this file — names it in the journal: a person may have set that attribute by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The file was replaced by the one rename.
    Written,
    /// The rename was refused for the read-only attribute; the attribute was cleared, and the
    /// second rename replaced the file. The attribute stays cleared — the doc of [`write_to`].
    WrittenAfterClearingReadOnly,
}

/// Puts `temporary` over `path` — the one rename of [`write_to`], and **one** retry for a target
/// that carries the read-only attribute — task T-55-3, finding Н26.
///
/// # The finding
///
/// `fs::rename` over a file with `FILE_ATTRIBUTE_READONLY` answers `ERROR_ACCESS_DENIED`, and it
/// answers it every time (premise П5 of the stage, measured: `raw_os_error = 5`, the temporary
/// left behind). The refusal reached only the journal: «Применить» closed the dialog as if nothing
/// had happened, and the change was gone at the next start. This module had no word for the
/// attribute at all.
///
/// # What is done, and what is not
///
/// * **Only after the refusal, and only for that refusal.** The attribute is not looked at before a
///   rename fails, so the ordinary write is the one it always was. A refusal of another kind — or
///   `PermissionDenied` on a file that is *not* read-only, which then has another cause, a handle
///   held open or the permissions of the folder — is handed back as it came.
/// * **Once, not in a loop.** The attribute is cleared, the rename is tried a second time, and
///   whatever the second attempt answers is the answer. Nothing waits and nothing repeats.
/// * **The attribute is left cleared**, whatever the second attempt answered. Putting it back after
///   a write that landed would make the next save fail in exactly the same way, and the one after
///   that — the very defect this repairs; putting it back after a write that did not land would buy
///   nothing, because the write is about to be reported as failed anyway. A person who set the
///   attribute to keep the program away is told: a write that landed is named in the journal by
///   the caller (`configuration read-only attribute cleared`), and one that did not is reported as
///   the failed write it is.
///
/// Atomicity is untouched: the retry is the same single rename of the same finished temporary.
fn replace_read_only_once(temporary: &Path, path: &Path) -> io::Result<WriteOutcome> {
    let refusal = match fs::rename(temporary, path) {
        Ok(()) => return Ok(WriteOutcome::Written),
        Err(refusal) => refusal,
    };

    let Ok(metadata) = fs::metadata(path) else {
        return Err(refusal);
    };
    let mut permissions = metadata.permissions();

    if refusal.kind() != io::ErrorKind::PermissionDenied || !permissions.readonly() {
        return Err(refusal);
    }

    // Windows: this clears `FILE_ATTRIBUTE_READONLY` and touches no mode bits — the Unix concern of
    // the lint does not exist on the one platform this program is built for.
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);

    if fs::set_permissions(path, permissions).is_err() {
        return Err(refusal);
    }

    fs::rename(temporary, path).map(|()| WriteOutcome::WrittenAfterClearingReadOnly)
}

/// Names the temporary file used by [`write_to`], beside the target.
///
/// Beside it and not in `%TEMP%`, because a rename is only atomic within one volume. The
/// process id keeps two instances of the program from colliding; within one process the
/// configuration is written by the UI thread alone (section 6.3), so nothing finer is
/// needed.
///
/// Public since task T-55-5, so that a test can take the name a write leaves from here and hand it
/// to [`remove_abandoned_temporaries`] — the mask of the sweep is never written out a second time.
pub fn temporary_path_for(path: &Path) -> PathBuf {
    let mut name = match path.file_name() {
        Some(name) => name.to_os_string(),
        None => OsString::from(CONFIG_FILE_NAME),
    };
    name.push(format!(".{}{TEMPORARY_SUFFIX}", std::process::id()));
    path.with_file_name(name)
}

/// What [`temporary_path_for`] puts after the process id, and what [`remove_abandoned_temporaries`]
/// looks for — task T-55-5. One constant for both, so that the name a write leaves and the name a
/// start removes cannot drift apart.
const TEMPORARY_SUFFIX: &str = ".tmp";

/// What [`remove_abandoned_temporaries`] did — task T-55-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swept {
    /// Temporaries of an interrupted write that were removed.
    pub removed: usize,
    /// Temporaries of an interrupted write that were found and would not go.
    pub refused: usize,
}

/// Removes the temporaries an interrupted [`write_to`] left beside `path` — task T-55-5, finding
/// Т8.
///
/// A process stopped between the write of its temporary and the rename — killed, or cut off by a
/// power failure — leaves `config.toml.<pid>.tmp` behind, and until this task nothing ever removed
/// it: no start looked, and the uninstaller had no `[UninstallDelete]` for it. Litter like that is
/// at odds with section 8.4, which promises an uninstall that leaves nothing of the program behind.
///
/// **What counts** is a file in the folder of `path` named exactly the way [`temporary_path_for`]
/// names one: the name of `path`, a dot, decimal digits and [`TEMPORARY_SUFFIX`]. The kept copy of
/// an unreadable file (`config.toml.bad`, [`QUARANTINE_SUFFIX`]) is not one and is precious, and no
/// other file of the folder is looked at — the journal keeps temporaries of its own there.
///
/// **When:** once, at the start, where the configuration is read — before this process has written
/// anything, so none of the temporaries can be its own. Not on a timer, and not by asking whether
/// the process named in a file is still alive: FR-82 keeps one instance per session, and a name
/// alone cannot prove who is still writing.
///
/// A refusal is not a failure of the program's work — the configuration is read all the same — and
/// it is not silence either: it is counted in [`Swept::refused`], and the caller names it in the
/// journal.
pub fn remove_abandoned_temporaries(path: &Path) -> Swept {
    let mut swept = Swept::default();

    let (Some(folder), Some(name)) = (
        path.parent(),
        path.file_name().and_then(|name| name.to_str()),
    ) else {
        return swept;
    };

    // No folder yet — a first run — has nothing in it to sweep.
    let Ok(entries) = fs::read_dir(folder) else {
        return swept;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();

        let abandoned = file_name
            .to_str()
            .and_then(|candidate| candidate.strip_prefix(name))
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|rest| rest.strip_suffix(TEMPORARY_SUFFIX))
            .is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            });

        if !abandoned || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }

        match fs::remove_file(entry.path()) {
            Ok(()) => swept.removed += 1,
            Err(_) => swept.refused += 1,
        }
    }

    swept
}

/// Writes `bytes` to `path` and flushes them to the device before returning.
///
/// The flush is the point: without it the rename can reach the disk ahead of the contents
/// and a power cut leaves an empty configuration where a whole one was promised.
fn write_and_sync(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
