//! The window of the settings (FR-92) on the shell.
//!
//! Five sections — «Общие», «Горячая клавиша», «Раскладки», «Исключения», «Диагностика» — and one
//! button, «Закрыть». **Every change acts at once:** a switch, a segment, a choice, a check box of
//! a list, «Выше» and «Ниже», «Добавить» and «Удалить» each put the new configuration in force the
//! way «Применить» did — written to the file and published to the thread of the input — and there
//! is no draft in the window. When the file refuses a change, the change is stepped back and a
//! note says so at the top of the section.

use crate::layouts::LayoutId;
use crate::settings::{
    self, CaptureEvent, CaptureSession, CaptureStep, CaptureTarget, Config, ExclusionPick,
    ExclusionVerdict, Hotkey, IDS_AUTOSTART, IDS_CAPTURE_HINT, IDS_CAPTURE_PROMPT,
    IDS_CAPTURE_SHIFT_AGAIN, IDS_CLOSE, IDS_CYCLE_DOWN, IDS_CYCLE_HINT, IDS_CYCLE_UP,
    IDS_DIALOG_CAPTION, IDS_EXCLUSION_ADD, IDS_EXCLUSION_ADDED, IDS_EXCLUSION_ALREADY,
    IDS_EXCLUSION_AUTO, IDS_EXCLUSION_HINT, IDS_EXCLUSION_MANUAL, IDS_EXCLUSION_NOT_PICKED,
    IDS_EXCLUSION_PICK, IDS_EXCLUSION_PICK_COUNT, IDS_EXCLUSION_PICK_NOTE, IDS_EXCLUSION_REMOVE,
    IDS_GROUP_DIAGNOSTICS, IDS_GROUP_EXCLUSIONS, IDS_GROUP_GENERAL, IDS_GROUP_HOTKEY,
    IDS_GROUP_LAYOUTS, IDS_HOTKEY_LABEL, IDS_HOTKEY_SET, IDS_HOTKEY_STOP, IDS_LANGUAGE_LABEL,
    IDS_LOG_DIR_LABEL, IDS_LOG_ENABLED, IDS_LOG_HINT, IDS_LOG_OPEN, IDS_LOG_SAVE,
    IDS_LOG_SAVE_FAILED, IDS_LOG_SAVED, IDS_MODE_CYCLE, IDS_MODE_PAIR, IDS_NEW_BADGE,
    IDS_PAIR_SOURCE, IDS_PAIR_TARGET, IDS_SELECTION_ENABLED, IDS_SETTINGS_NOT_SAVED, IDS_SOUND,
    IDS_THEME_DARK, IDS_THEME_LABEL, IDS_THEME_LIGHT, IDS_THEME_SYSTEM, IDS_TRANSLATION_REPORT,
    IDS_TRAY_FLAG, IDS_TRAY_FLAG_HINT, IDS_UPDATES_SWITCH, IDS_WRITE_TO_AUTHOR, Language,
    LanguageOrder, LayoutMode, LayoutRow, Modifiers, Refusal,
};
use crate::settings::{
    IDS_AUTOSTART_PRESENT, IDS_CYCLE_ARROW, IDS_SECTION_DIAGNOSTICS_SUB,
    IDS_SECTION_EXCLUSIONS_SUB, IDS_SECTION_GENERAL_SUB, IDS_SECTION_HOTKEY_SUB,
    IDS_SECTION_LAYOUTS_SUB, IDS_SETTINGS_EYEBROW, IDS_STATE_AUTOSTART, IDS_STATE_HEALTH,
    IDS_STATE_JOINED, IDS_STATE_LAYOUTS, IDS_STATE_NO_FAILURES, IDS_STATE_PAIR, IDS_STATE_WORKING,
    IDS_STATUS_AUTOSTART_OFF, IDS_STATUS_AUTOSTART_ON, IDS_STATUS_COPY, IDS_STATUS_DETAILS,
    IDS_STATUS_FIXES, IDS_STATUS_FIXES_NONE, IDS_STATUS_HOTKEY,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{self, Content, Reply};
use crate::ui::scene::{
    Art, Brand, ButtonKind, Event, FieldKind, ListItem, Node, Role, Surface, Window, WindowKind,
};
use crate::ui::windows::tray_menu;
use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::UI::WindowsAndMessaging::GetMessageTime;
use std::time::{Duration, Instant};

/// `VK_RCONTROL` — the side of a tap of `Ctrl` the warning names «Правый Ctrl» (решение 208.228).
const RIGHT_CTRL: u16 = ::windows::Win32::UI::Input::KeyboardAndMouse::VK_RCONTROL.0;

/// The elements of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Sections,
    Autostart,
    Sound,
    Selection,
    TrayFlag,
    Updates,
    Language,
    Translation,
    Theme,
    Hotkey,
    HotkeySet,
    Mode,
    PairSource,
    PairTarget,
    Cycle,
    Up,
    Down,
    Exclusions,
    Remove,
    Pick,
    ExclusionName,
    Add,
    Log,
    OpenFolder,
    SaveJournal,
    Write,
    Details,
    Close,
    /// The address under the sections, in the panel (AUDIT-180 Н2).
    Site,
    /// «Одна клавиша | Две клавиши» — the mode of the section «Горячая клавиша» (решение 208.228).
    HotkeyMode,
    /// The field of the second key, «Переключить раскладку».
    SwitchKey,
    /// «Задать» beside it.
    SwitchKeySet,
    /// «Всё равно установить» of the warning about a single `Ctrl` (В-4).
    CtrlWarnSet,
    /// «Выбрать другую клавишу» of the warning.
    CtrlWarnOther,
}

/// The name of the author's site under the sections, as in «О программе» and «От автора»: the host
/// of the program's page, read out of `letters::links`.
fn site() -> String {
    let page = crate::letters::links::PROGRAM_URL;
    let rest = page.strip_prefix("https://").unwrap_or(page);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

/// The sections, in their order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    General,
    Hotkey,
    Layouts,
    Exclusions,
    Diagnostics,
}

impl Section {
    pub const ALL: &'static [Self] = &[
        Self::General,
        Self::Hotkey,
        Self::Layouts,
        Self::Exclusions,
        Self::Diagnostics,
    ];

    fn title(self) -> u16 {
        match self {
            Self::General => IDS_GROUP_GENERAL,
            Self::Hotkey => IDS_GROUP_HOTKEY,
            Self::Layouts => IDS_GROUP_LAYOUTS,
            Self::Exclusions => IDS_GROUP_EXCLUSIONS,
            Self::Diagnostics => IDS_GROUP_DIAGNOSTICS,
        }
    }

    /// The line under the title — the mock-up's `sub` (C3 of T-120-20, решение 208.160).
    fn subtitle(self) -> u16 {
        match self {
            Self::General => IDS_SECTION_GENERAL_SUB,
            Self::Hotkey => IDS_SECTION_HOTKEY_SUB,
            Self::Layouts => IDS_SECTION_LAYOUTS_SUB,
            Self::Exclusions => IDS_SECTION_EXCLUSIONS_SUB,
            Self::Diagnostics => IDS_SECTION_DIAGNOSTICS_SUB,
        }
    }
}

/// What puts a configuration in force and answers whether the file took it: in the product
/// `tray::apply_settings_now`, the road «Применить» took.
pub type Apply = Box<dyn FnMut(&Config) -> bool>;

/// The window of the settings: the configuration in force and what the window itself holds —
/// the section open, the rows of the lists, the name being typed.
pub struct Settings {
    /// The configuration in force: what the window shows is always this.
    pub config: Config,
    pub language: Language,
    pub section: Section,
    /// The layouts of this session, and the rows of the cycle in their order.
    session: Vec<LayoutId>,
    rows: Vec<LayoutRow>,
    rows_at_open: Vec<LayoutRow>,
    cycle_at_open: Vec<String>,
    languages: LanguageOrder,
    cycle_selected: usize,
    /// The names of «Исключения» marked for «Удалить».
    marked: Vec<bool>,
    exclusion_name: String,
    /// The file of the configuration refused the last change.
    refused: bool,
    /// The journal was just saved; or its saving failed.
    journal: Option<bool>,
    /// «Журнал сохранён» stands on the button till then (`settings::JOURNAL_SAVED_MS`).
    journal_until: Option<Instant>,
    /// The capture of the hotkey, while the field takes every key.
    capture: Option<Capture>,
    /// ⭐ **A lone `Ctrl` tapped in a capture of the second key, waiting for the person's choice** —
    /// решение 208.228, В-4: its side; the warning stands under the second field till «Всё равно
    /// установить» or «Выбрать другую клавишу».
    ctrl_warning: Option<u16>,
    /// The last capture took a combination or the double press: the line under the field says
    /// once which half stays with the programs (task T-93-2), till the next capture.
    combination_taken: bool,
    /// «Выбрать окно программы» counts: the moment the window in front is taken.
    pick_at: Option<Instant>,
    /// What the last pick found, said under its button; none — the line at rest.
    pick_note: Option<String>,
    /// What the block «Состояние» of «Диагностика» shows; none — no block (T-120-21).
    program: Option<Program>,
    /// «Сохранить журнал»: the journal into its folder, and whether it was written.
    save_journal: fn() -> bool,
    apply: Apply,
}

/// An armed capture of the hotkey (FR-94): the hotkey is suspended while it lives — the session
/// gives it back when dropped, the window closing among the ways (FR-02).
struct Capture {
    session: CaptureSession,
    /// The line under the field.
    line: CaptureLine,
    /// A `Shift` went down on its own: the field says «Shift…» till then (`hook::DOUBLE_TAP_MS`
    /// after the last tap, the timer of the pair of task T-95-2).
    pair_until: Option<Instant>,
}

/// What the line under the field says in a capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptureLine {
    /// «Esc или клик мимо — отмена».
    Hint,
    /// Why the last press cannot be the hotkey; the capture stays armed.
    Refused(Refusal),
    /// «Ещё раз Shift — двойное нажатие; …» — stays till the next press.
    ShiftAgain,
}

/// The program as the block «Состояние» shows it (T-120-21): whether it works, and the lines for the
/// author — `settings::state_details` in the product, read when the window opens and after every
/// change.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    /// `general.enabled` in force: working or suspended.
    pub enabled: bool,
    /// The program refused to resume (FR-96, `tray::resume_is_refused`): a restart is needed.
    pub fail_safe: bool,
    /// The four lines of the old group «Состояние», word for word.
    pub details: Vec<String>,
    /// The command of the `Run` key, where it is there: shown on a line of its own.
    pub path: Option<String>,
}

impl Settings {
    /// The window over `config`, in `language`, putting changes in force by `apply`.
    pub fn new(config: Config, language: Language, apply: Apply) -> Self {
        let session = crate::layouts::enumerate().unwrap_or_default();
        Self::with_session(config, language, session, apply)
    }

    /// The same over a given list of layouts — the instrument of fitting and the tests name it.
    pub fn with_session(
        config: Config,
        language: Language,
        session: Vec<LayoutId>,
        apply: Apply,
    ) -> Self {
        let rows = settings::layout_rows(&config.layouts, &session);
        let languages = settings::language_order_on_this_machine(&session);
        Self {
            marked: vec![false; config.exclusions.processes.len()],
            rows_at_open: rows.clone(),
            cycle_at_open: config.layouts.cycle.clone(),
            rows,
            session,
            languages,
            config,
            language,
            section: Section::General,
            cycle_selected: 0,
            exclusion_name: String::new(),
            refused: false,
            journal: None,
            journal_until: None,
            capture: None,
            ctrl_warning: None,
            combination_taken: false,
            pick_at: None,
            pick_note: None,
            program: None,
            save_journal: save_journal_in_force,
            apply,
        }
    }

    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }

    /// The sentence of the note over a change the file refused: the note goes with the next
    /// change the file takes; the refusal itself is in the journal (`CONFIG_WRITE_FAILED`).
    fn refused_sentence(&self) -> String {
        self.line(IDS_SETTINGS_NOT_SAVED)
    }

    /// Puts the configuration changed by `edit` in force; the file refused it — the change is
    /// stepped back, and the note says so.
    fn change(&mut self, edit: impl FnOnce(&mut Config)) {
        // Nothing is put in force while the hotkey is suspended: publishing the configuration
        // would give the callback its hotkey back in the middle of a capture (C6).
        self.capture = None;
        let mut next = self.config.clone();
        edit(&mut next);
        if next == self.config {
            return;
        }
        if (self.apply)(&next) {
            self.config = next;
            self.refused = false;
        } else {
            let _ = (self.apply)(&self.config);
            self.refused = true;
        }
        self.language = self.config.general.language.in_force();
    }

    fn cycle_changed(&mut self) {
        let cycle = settings::cycle_after_dialog(
            &self.cycle_at_open,
            &self.rows_at_open,
            &self.rows,
            &self.session,
        );
        self.change(|config| config.layouts.cycle = cycle);
    }

    fn layout_index(&self, spec: &str) -> usize {
        self.session
            .iter()
            .position(|layout| settings::spec_text(*layout, &self.session) == spec)
            .unwrap_or(0)
    }

    fn general(&self) -> Vec<Node<Id>> {
        let language = self.config.general.language.in_force();
        // The switches in one group of rows (208.209, AUDIT-180 Н3), a line between two of them.
        let mut nodes = vec![Node::Group(vec![
            toggle(
                Id::Autostart,
                self.line(IDS_AUTOSTART),
                self.config.general.autostart,
            ),
            toggle(Id::Sound, self.line(IDS_SOUND), self.config.feedback.sound),
            toggle(
                Id::Selection,
                self.line(IDS_SELECTION_ENABLED),
                self.config.selection.enabled,
            ),
            Node::Toggle {
                id: Id::TrayFlag,
                label: self.line(IDS_TRAY_FLAG),
                // What the flag is, under its label (Н4, the mock-up of the window).
                hint: Some(self.line(IDS_TRAY_FLAG_HINT)),
                on: self.config.general.tray_flag,
                // The setting came with 1.7.0 (T-119-2); the badge «новое» stays while the
                // window is the window of 1.8.0 (T-120-20, passport §6).
                badge: Some(self.line(IDS_NEW_BADGE)),
            },
            // The setting of «От автора» (T-120-20, решение 208.119): the window «Обновлено» —
            // `[letters] release_window`; the feed is not this switch (Д6).
            toggle(
                Id::Updates,
                self.line(IDS_UPDATES_SWITCH),
                self.config.letters.release_window,
            ),
        ])];
        let mut language_column = vec![Node::Choice {
            id: Id::Language,
            label: self.line(IDS_LANGUAGE_LABEL),
            options: self
                .languages
                .languages
                .iter()
                .map(|language| language.native_name().to_owned())
                .collect(),
            selected: self
                .languages
                .languages
                .iter()
                .position(|candidate| *candidate == language)
                .unwrap_or(0),
        }];
        // T-120-22: under «Язык интерфейса», the quiet «Нашли ошибку в переводе?» — in every
        // language of the window but Russian, the language the others are translated from.
        // ⚠ Not while the note of a refused write stands over the section (C6 of the sheet, (г)):
        // with the note, the button pushes «Общие» of the narrow window out of it in ta, ml, tt
        // and ka; the note goes with the next change, and the button comes back with it.
        if self.language != Language::Ru && !self.refused {
            language_column.push(Node::Buttons(vec![Node::Button {
                id: Id::Translation,
                label: self.line(IDS_TRANSLATION_REPORT),
                kind: ButtonKind::Quiet,
                browser: false,
            }]));
        }
        // «Язык интерфейса», then «Оформление» on a line of its own across the column, their labels
        // over them (208.209, AUDIT-180 Н5, О5). Not a pair: three segments are wider than half the
        // column at the type of the window, and in a half they turned into a choice field (the
        // frame of the stand, 23.11) — the rule of the segments comes first.
        nodes.extend(language_column);
        nodes.push(Node::Segments {
            id: Id::Theme,
            label: self.line(IDS_THEME_LABEL),
            options: [IDS_THEME_SYSTEM, IDS_THEME_LIGHT, IDS_THEME_DARK]
                .map(|id| self.line(id))
                .to_vec(),
            selected: settings::theme_combo_index(self.config.general.theme),
        });
        nodes
    }

    /// The field of the hotkey and «Задать» beside it; under them, one line: at rest the ways a
    /// hotkey may be, once after a capture what stays with the programs, in a capture the way out
    /// or why the press was refused (решение 82.6, task T-95-2).
    ///
    /// ⭐ **Решение 208.228 — one key or two.** Above the field, the segment «Одна клавиша | Две
    /// клавиши» (the element of «Пара | Несколько раскладок», no label, as in «Раскладки»). With two
    /// keys, two groups: «Исправить набранное:» over the first field with its line, and «Переключить
    /// раскладку:» over the second with «Только переключает раскладку, набранное не трогает.» under
    /// it (the mock-up, В-5). The mode set does not assign the second key and arms no capture: the
    /// field says «Не задана» in the colour of a hint till «Задать».
    fn hotkey(&self) -> Vec<Node<Id>> {
        let two = self.config.switch_hotkey.enabled;
        // While the warning about a single `Ctrl` waits for the choice it is the question of the
        // section: the segment and the first group step aside, so that the warning fits the window
        // in every language (FIT-E120).
        let warning = two && self.ctrl_warning.is_some();
        let mut nodes = Vec::new();
        if !warning {
            nodes.push(Node::Segments {
                id: Id::HotkeyMode,
                label: String::new(),
                options: vec![
                    self.line(settings::IDS_HOTKEY_MODE_ONE),
                    self.line(settings::IDS_HOTKEY_MODE_TWO),
                ],
                selected: usize::from(two),
            });
            nodes.extend(self.key_group(CaptureTarget::Fix, two));
        }
        if two {
            nodes.extend(self.key_group(CaptureTarget::Switch, two));
        }
        nodes
    }

    /// One field of a hotkey, «Задать» beside it and the line under them — the first key's or the
    /// second's (решение 208.228). The capture of the one shows in its own group; the other stands
    /// at rest.
    fn key_group(&self, target: CaptureTarget, two: bool) -> Vec<Node<Id>> {
        let (field, set_button) = match target {
            CaptureTarget::Fix => (Id::Hotkey, Id::HotkeySet),
            CaptureTarget::Switch => (Id::SwitchKey, Id::SwitchKeySet),
        };
        let label = match (target, two) {
            (CaptureTarget::Fix, false) => IDS_HOTKEY_LABEL,
            (CaptureTarget::Fix, true) => settings::IDS_HOTKEY_FIX_LABEL,
            (CaptureTarget::Switch, _) => settings::IDS_HOTKEY_SWITCH_LABEL,
        };
        let capture = self
            .capture
            .as_ref()
            .filter(|capture| capture.session.target() == target);
        let (value, hint, set, line) = match capture {
            Some(capture) => (
                if capture.pair_until.is_some() {
                    settings::pending_shift_name()
                } else {
                    self.line(IDS_CAPTURE_PROMPT)
                },
                String::new(),
                IDS_HOTKEY_STOP,
                match capture.line {
                    CaptureLine::Hint => self.line(IDS_CAPTURE_HINT),
                    CaptureLine::Refused(refusal) => self.line(refusal.string_id()),
                    CaptureLine::ShiftAgain => self.line(IDS_CAPTURE_SHIFT_AGAIN),
                },
            ),
            None if target == CaptureTarget::Fix => (
                settings::effective_hotkey_name(&self.config.hotkey),
                String::new(),
                IDS_HOTKEY_SET,
                settings::hotkey_line_in(
                    self.language,
                    &self.config.hotkey,
                    self.combination_taken,
                ),
            ),
            None => {
                let line = settings::switch_hotkey_note(&self.config)
                    .unwrap_or(settings::IDS_SWITCH_KEY_HINT);
                match self.config.switch_binding() {
                    // «Не задана» in the colour of a hint — the field is empty (the mock-up).
                    None => (
                        String::new(),
                        self.line(settings::IDS_SWITCH_KEY_NONE),
                        IDS_HOTKEY_SET,
                        self.line(line),
                    ),
                    Some(_) => (
                        settings::switch_hotkey_name_in(self.language, &self.config),
                        String::new(),
                        IDS_HOTKEY_SET,
                        self.line(line),
                    ),
                }
            }
        };
        // The warning: the `Ctrl` tapped is the candidate, shown in the colour of a hint — it is not
        // set yet — and the warning stands in the place of the line under the field.
        let warning = self
            .ctrl_warning
            .filter(|_| target == CaptureTarget::Switch && capture.is_none());
        let (value, hint) = match warning {
            Some(side) => (
                String::new(),
                self.line(if side == RIGHT_CTRL {
                    settings::IDS_KEY_RIGHT_CTRL
                } else {
                    settings::IDS_KEY_LEFT_CTRL
                }),
            ),
            None => (value, hint),
        };
        let mut nodes = vec![Node::Row(vec![
            Node::Field {
                id: field,
                label: self.line(label),
                value,
                hint,
                kind: FieldKind::Key {
                    capturing: capture.is_some(),
                },
            },
            button(set_button, self.line(set)),
        ])];
        nodes.push(match warning {
            Some(_) => self.ctrl_warning_note(),
            None => Node::Text(Role::Small, line),
        });
        nodes
    }

    /// ⭐ **The warning about a single `Ctrl`** — решение 208.228, В-4: what may break and what it
    /// touches, then the person's choice — «Выбрать другую клавишу» (the capture again) or «Всё равно
    /// установить». A note of the section, as the note of a refused write is (the shell of 1.8.0).
    fn ctrl_warning_note(&self) -> Node<Id> {
        Node::Surface(
            Surface::Note,
            Box::new(Node::Rows(vec![
                Node::Text(Role::Strong, self.line(settings::IDS_CTRL_WARN_TITLE)),
                Node::Text(Role::Body, self.line(settings::IDS_CTRL_WARN_TEXT)),
                Node::Buttons(vec![
                    button(Id::CtrlWarnOther, self.line(settings::IDS_CTRL_WARN_OTHER)),
                    button(Id::CtrlWarnSet, self.line(settings::IDS_CTRL_WARN_SET)),
                ]),
            ])),
        )
    }

    /// Arms a capture of `target` (FR-94; решение 208.228): the other hotkey in force is what the
    /// press may not share its main key with — the second key in force for a capture of the first,
    /// the first as it is published (its binding, or `Pause`) for a capture of the second.
    fn arm_capture(&mut self, target: CaptureTarget) {
        let (previous, other) = match target {
            CaptureTarget::Fix => (self.config.hotkey.clone(), self.config.switch_binding()),
            CaptureTarget::Switch => (
                self.config.switch_hotkey.as_hotkey(),
                Some(
                    self.config
                        .hotkey
                        .binding()
                        .unwrap_or((crate::hook::DEFAULT_HOTKEY_VK, 0)),
                ),
            ),
        };
        self.capture = Some(Capture {
            session: CaptureSession::arm_for(target, previous, other),
            line: CaptureLine::Hint,
            pair_until: None,
        });
        self.combination_taken = false;
        self.ctrl_warning = None;
    }

    fn layouts(&self) -> Vec<Node<Id>> {
        let mode = match self.config.layouts.mode {
            LayoutMode::Pair => 0,
            LayoutMode::Cycle => 1,
        };
        let mut nodes = vec![Node::Segments {
            id: Id::Mode,
            label: String::new(),
            options: vec![self.line(IDS_MODE_PAIR), self.line(IDS_MODE_CYCLE)],
            selected: mode,
        }];
        let labels = settings::layout_labels(&self.session, &self.session);
        if self.config.layouts.mode == LayoutMode::Pair {
            // «Источник» and «Цель» as a pair, their labels over them (208.209, AUDIT-180 Н9).
            nodes.push(Node::Pair(
                Box::new(Node::Choice {
                    id: Id::PairSource,
                    label: self.line(IDS_PAIR_SOURCE),
                    options: labels.clone(),
                    selected: self.layout_index(&self.config.layouts.pair_source),
                }),
                Box::new(Node::Choice {
                    id: Id::PairTarget,
                    label: self.line(IDS_PAIR_TARGET),
                    options: labels,
                    selected: self.layout_index(&self.config.layouts.pair_target),
                }),
            ));
        } else {
            let names: Vec<LayoutId> = self.rows.iter().map(|row| row.layout).collect();
            let row_labels = settings::layout_labels(&names, &self.session);
            nodes.push(Node::Text(Role::Small, self.line(IDS_CYCLE_HINT)));
            // The list takes what the section leaves (the mock-up's `.grow`).
            nodes.push(Node::Fill(Box::new(Node::List {
                id: Id::Cycle,
                rows: self
                    .rows
                    .iter()
                    .zip(row_labels)
                    .map(|(row, label)| ListItem {
                        label,
                        check: Some(row.checked),
                        mark: false,
                    })
                    .collect(),
                selected: self.cycle_selected,
                first: 0,
            })));
            nodes.push(Node::Buttons(vec![
                button(Id::Up, self.line(IDS_CYCLE_UP)),
                button(Id::Down, self.line(IDS_CYCLE_DOWN)),
            ]));
        }
        nodes
    }

    fn exclusions(&self) -> Vec<Node<Id>> {
        vec![
            // The list takes what the section leaves (the mock-up's `.grow`).
            Node::Fill(Box::new(Node::List {
                id: Id::Exclusions,
                rows: self
                    .config
                    .exclusions
                    .processes
                    .iter()
                    .zip(self.marked.iter())
                    .map(|(name, marked)| ListItem {
                        label: name.clone(),
                        check: Some(*marked),
                        mark: false,
                    })
                    .collect(),
                selected: 0,
                first: 0,
            })),
            Node::Buttons(vec![button(Id::Remove, self.line(IDS_EXCLUSION_REMOVE))]),
            // The two ways to add one, side by side as in the mock-up (`.form2`).
            Node::Pair(
                Box::new(Node::Rows(vec![
                    Node::Line(Role::FieldLabel, self.line(IDS_EXCLUSION_AUTO)),
                    Node::Buttons(vec![button(Id::Pick, self.pick_caption())]),
                    Node::Text(
                        Role::Small,
                        self.pick_note
                            .clone()
                            .unwrap_or_else(|| self.line(IDS_EXCLUSION_PICK_NOTE)),
                    ),
                ])),
                Box::new(Node::Row(vec![
                    Node::Field {
                        id: Id::ExclusionName,
                        label: self.line(IDS_EXCLUSION_MANUAL),
                        value: self.exclusion_name.clone(),
                        hint: self.line(IDS_EXCLUSION_HINT),
                        kind: FieldKind::Line,
                    },
                    button(Id::Add, self.line(IDS_EXCLUSION_ADD)),
                ])),
            ),
        ]
    }

    fn diagnostics(&self) -> Vec<Node<Id>> {
        let folder = crate::diag::log_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let save = if self.journal == Some(true) {
            IDS_LOG_SAVED
        } else {
            IDS_LOG_SAVE
        };
        let mut nodes = vec![
            // «Вести журнал» in a frame, what the journal holds under it (208.209, AUDIT-180 Н10).
            Node::Group(vec![Node::Toggle {
                id: Id::Log,
                label: self.line(IDS_LOG_ENABLED),
                hint: Some(self.line(IDS_LOG_HINT)),
                on: self.config.diagnostics.log_enabled,
                badge: None,
            }]),
            Node::Line(Role::FieldLabel, self.line(IDS_LOG_DIR_LABEL)),
            Node::Line(Role::Code, folder),
            Node::Buttons(vec![
                button(Id::OpenFolder, self.line(IDS_LOG_OPEN)),
                button(Id::SaveJournal, self.line(save)),
                button(Id::Write, self.line(IDS_WRITE_TO_AUTHOR)),
            ]),
        ];
        if self.journal == Some(false) {
            nodes.insert(0, note(self.line(IDS_LOG_SAVE_FAILED)));
        }
        if let Some(program) = &self.program {
            nodes.extend(self.status_block(program));
        }
        nodes
    }

    /// **«Состояние» in the words of a person** (T-120-21): the state of the program with the dot of
    /// the head of the menu, what the program fixes, the key, the start with Windows; the lines for
    /// the author only in the window «Подробности для автора» opens ([`Details`], C6 (а)).
    fn status_block(&self, program: &Program) -> Vec<Node<Id>> {
        let (state, words) = tray_menu::status(program.enabled, program.fail_safe);
        let autostart = if self.config.general.autostart {
            IDS_STATUS_AUTOSTART_ON
        } else {
            IDS_STATUS_AUTOSTART_OFF
        };
        let mut lines = vec![
            Node::Status {
                state,
                words: self.line(words),
            },
            Node::Text(Role::Body, self.fixes()),
            Node::Text(
                Role::Body,
                self.line(IDS_STATUS_HOTKEY)
                    .replace("{0}", &settings::effective_hotkey_name(&self.config.hotkey)),
            ),
        ];
        // Решение 208.228: the second key, under the first, in the mode of two keys.
        if self.config.two_keys() {
            lines.push(Node::Text(
                Role::Body,
                self.line(settings::IDS_STATUS_SWITCH_KEY).replace(
                    "{0}",
                    &settings::switch_hotkey_name_in(self.language, &self.config),
                ),
            ));
        }
        lines.extend([
            Node::Text(Role::Body, self.line(autostart)),
            Node::Buttons(vec![Node::Button {
                id: Id::Details,
                label: self.line(IDS_STATUS_DETAILS),
                kind: ButtonKind::Quiet,
                browser: false,
            }]),
        ]);
        lines
    }

    /// «Исправляет: English → Русский»: the pair source → target, or the layouts of the cycle in
    /// their order — one direction in every window (T-120-21 п. 3), the arrow of the language.
    fn fixes(&self) -> String {
        let names: Vec<LayoutId> = match self.config.layouts.mode {
            LayoutMode::Pair => [
                &self.config.layouts.pair_source,
                &self.config.layouts.pair_target,
            ]
            .into_iter()
            .filter_map(|spec| {
                self.session
                    .iter()
                    .copied()
                    .find(|layout| settings::spec_text(*layout, &self.session) == *spec)
            })
            .collect(),
            LayoutMode::Cycle => self
                .rows
                .iter()
                .filter(|row| row.checked)
                .map(|row| row.layout)
                .collect(),
        };
        if names.len() <= 1 {
            return self.line(IDS_STATUS_FIXES_NONE);
        }
        let labels = settings::layout_labels(&names, &self.session);
        self.line(IDS_STATUS_FIXES)
            .replace("{0}", &labels.join(&self.line(IDS_CYCLE_ARROW)))
    }

    /// One event of the capture, decided by its session with the memory of a pair of `Shift`, and
    /// carried out (tasks Т-23-5, T-95-2: the machine is `settings::CapturePair`, not copied).
    fn capture_step(&mut self, event: CaptureEvent) {
        // SAFETY: reads the time of the message this thread dispatches — the key's own, since the
        // shell sends `Event::KeyDown` inside it. The cast is the documented reading of a tick.
        let time = unsafe { GetMessageTime() } as u32;
        let Some(step) = self
            .capture
            .as_mut()
            .map(|capture| capture.session.step(event, time))
        else {
            return;
        };
        match step {
            CaptureStep::Ignore => {}
            CaptureStep::Cancel => self.capture = None,
            CaptureStep::Take(hotkey) => self.take_hotkey(hotkey),
            // Решение 208.228, В-4: the capture ends — the hotkeys come back — and the warning asks.
            CaptureStep::CtrlTapped(side) => {
                self.capture = None;
                self.ctrl_warning = Some(side);
            }
            step => {
                let Some(capture) = self.capture.as_mut() else {
                    return;
                };
                match step {
                    // The capture stays armed: «not that one», not the end of the question. The
                    // refusal is counted and journaled where it is carried out (task T-71-3), and it
                    // ends an open pair.
                    CaptureStep::Refuse(refusal) => {
                        settings::note_capture_refused();
                        capture.line = CaptureLine::Refused(refusal);
                        capture.pair_until = None;
                    }
                    CaptureStep::AwaitShift => {
                        capture.line = CaptureLine::ShiftAgain;
                        capture.pair_until = Some(
                            Instant::now()
                                + Duration::from_millis(crate::hook::DOUBLE_TAP_MS.into()),
                        );
                    }
                    // The line «Ещё раз Shift…» stays till the next press (157.18).
                    _ => capture.pair_until = None,
                }
            }
        }
    }

    /// A press taken (FR-94): the capture ends — its session gives the callback the hotkey it had —
    /// and only then is the new one put in force, like any other change.
    ///
    /// Решение 208.228: into the field it was captured for — the second key keeps the mode it has.
    fn take_hotkey(&mut self, hotkey: Hotkey) {
        let target = self
            .capture
            .take()
            .map_or(CaptureTarget::Fix, |capture| capture.session.target());
        if target == CaptureTarget::Switch {
            self.change(|config| config.switch_hotkey = config.switch_hotkey.with_key(hotkey));
            return;
        }
        let combination = !hotkey.modifiers.is_empty() || hotkey.is_double_tap();
        self.change(|config| config.hotkey = hotkey);
        self.combination_taken = combination && !self.refused;
    }

    /// The timer of the window: whatever moment has come is carried out.
    fn timer_came(&mut self) {
        let now = Instant::now();
        let pair_over = self
            .capture
            .as_ref()
            .and_then(|capture| capture.pair_until)
            .is_some_and(|until| until <= now);
        if pair_over {
            self.capture_step(CaptureEvent::PairTimeout);
            if let Some(capture) = self.capture.as_mut() {
                capture.pair_until = None;
            }
        }
        if self.journal_until.is_some_and(|until| until <= now) {
            self.journal_until = None;
            self.journal = None;
        }
        if self.pick_at.is_some_and(|at| at <= now) {
            self.pick_at = None;
            self.pick_now();
        }
    }

    /// The count of «Выбрать окно программы» is over: the program of the window in front goes into
    /// the list, or the line under the button says why not.
    fn pick_now(&mut self) {
        let captured = crate::guard::capture_foreground(crate::guard::Asker::Process)
            .and_then(|found| found.process);
        let pick = settings::exclusion_pick(captured.as_deref(), &self.config.exclusions.processes);
        if pick == ExclusionPick::Refused {
            crate::app::sound_refusal();
        }
        let note = match pick {
            ExclusionPick::Added(name) => {
                self.change(|config| config.exclusions.processes.push(name.clone()));
                self.marked = vec![false; self.config.exclusions.processes.len()];
                // A refused write says so at the top of the section, and nothing was added.
                (!self.refused).then(|| self.line(IDS_EXCLUSION_ADDED).replace("{0}", &name))
            }
            ExclusionPick::AlreadyListed(_, name) => {
                Some(self.line(IDS_EXCLUSION_ALREADY).replace("{0}", &name))
            }
            ExclusionPick::NotPicked | ExclusionPick::Refused => {
                Some(self.line(IDS_EXCLUSION_NOT_PICKED))
            }
        };
        self.pick_note = note;
    }

    /// Brings every moment the window waits for `by` nearer — the tests do not wait them out.
    #[cfg(feature = "testing")]
    pub fn hurry(&mut self, by: Duration) {
        let sooner = |at: &mut Instant| *at = at.checked_sub(by).unwrap_or(*at);
        if let Some(until) = self
            .capture
            .as_mut()
            .and_then(|capture| capture.pair_until.as_mut())
        {
            sooner(until);
        }
        if let Some(at) = self.pick_at.as_mut() {
            sooner(at);
        }
        if let Some(until) = self.journal_until.as_mut() {
            sooner(until);
        }
    }

    /// «Сохранить журнал» writes through `save` — a test does not write the journal of the program
    /// installed on the machine it runs on.
    #[cfg(feature = "testing")]
    pub fn saving_the_journal_with(&mut self, save: fn() -> bool) {
        self.save_journal = save;
    }

    /// A capture of the hotkey is armed.
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }

    /// The block «Состояние» shows `program` (T-120-21).
    pub fn showing_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    /// The layouts of this session, in the order the lists show them.
    pub fn session(&self) -> &[LayoutId] {
        &self.session
    }

    /// «Выбрать окно программы», or while it counts, the seconds left: «Выбор через {0} с…».
    fn pick_caption(&self) -> String {
        match self.pick_at {
            Some(at) => self
                .line(IDS_EXCLUSION_PICK_COUNT)
                .replace("{0}", &seconds_to(at).max(1).to_string()),
            None => self.line(IDS_EXCLUSION_PICK),
        }
    }

    /// The moments the window must look again at: the end of a pair of `Shift`, the next second
    /// of the count of «Выбрать окно программы», the end of «Журнал сохранён».
    fn deadlines(&self) -> impl Iterator<Item = Instant> {
        let pair = self.capture.as_ref().and_then(|capture| capture.pair_until);
        // The count says one second less at the moment minus the seconds it will say then.
        let pick = self
            .pick_at
            .map(|at| at - Duration::from_secs(seconds_to(at).saturating_sub(1)));
        pair.into_iter().chain(pick).chain(self.journal_until)
    }

    /// The elements unavailable now.
    fn disabled(&self) -> Vec<Id> {
        let mut off = Vec::new();
        if !self.marked.iter().any(|marked| *marked) {
            off.push(Id::Remove);
        }
        if self.cycle_selected == 0 {
            off.push(Id::Up);
        }
        if self.cycle_selected + 1 >= self.rows.len() {
            off.push(Id::Down);
        }
        let name = self.exclusion_name.trim();
        if name.is_empty()
            || settings::exclusion_verdict(name, &self.config.exclusions.processes)
                != ExclusionVerdict::Add
        {
            off.push(Id::Add);
        }
        if settings::exclusions_are_full(self.config.exclusions.processes.len()) {
            off.push(Id::Add);
            off.push(Id::Pick);
        }
        off
    }
}

/// «Сохранить журнал» in the product: the journal into its folder (`diag::write_to`).
fn save_journal_in_force() -> bool {
    crate::diag::log_path().is_some_and(|target| crate::diag::write_to(&target).is_ok())
}

/// The whole seconds left till `at`, counted up: none once it has come.
fn seconds_to(at: Instant) -> u64 {
    let left = at.saturating_duration_since(Instant::now());
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

fn toggle(id: Id, label: String, on: bool) -> Node<Id> {
    Node::Toggle {
        id,
        label,
        hint: None,
        on,
        badge: None,
    }
}

fn button(id: Id, label: String) -> Node<Id> {
    Node::Button {
        id,
        label,
        kind: ButtonKind::Normal,
        browser: false,
    }
}

fn note(sentence: String) -> Node<Id> {
    Node::Surface(Surface::Note, Box::new(Node::Text(Role::Body, sentence)))
}

impl Content for Settings {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let mut content = match self.section {
            Section::General => self.general(),
            Section::Hotkey => self.hotkey(),
            Section::Layouts => self.layouts(),
            Section::Exclusions => self.exclusions(),
            Section::Diagnostics => self.diagnostics(),
        };
        if self.refused {
            content.insert(0, note(self.refused_sentence()));
        }
        let mut body = vec![
            Node::Header {
                eyebrow: Some(self.line(IDS_SETTINGS_EYEBROW)),
                title: self.line(self.section.title()),
                // Решение 208.228: with two keys, «Горячая клавиша» says what each one does.
                subtitle: Some(
                    if self.section == Section::Hotkey && self.config.switch_hotkey.enabled {
                        self.line(settings::IDS_SECTION_HOTKEY_SUB_TWO)
                    } else {
                        self.line(self.section.subtitle())
                    },
                ),
            },
            Node::Sections {
                id: Id::Sections,
                items: Section::ALL
                    .iter()
                    .map(|section| self.line(section.title()))
                    .collect(),
                current: Section::ALL
                    .iter()
                    .position(|section| *section == self.section)
                    .unwrap_or(0),
            },
        ];
        // The section takes the rest of the column, so that a list in it can take the rest of the
        // section.
        body.push(Node::Fill(Box::new(Node::Rows(content))));

        Window {
            kind: WindowKind::Settings,
            // The scene of its own (208.163, AUDIT-180 Н1) and the name and the address under the
            // sections, as under the picture of the other windows (Н2).
            art: Some(Art::Settings),
            brand: Some(Brand {
                name: "Panda Koder".to_owned(),
                address: site(),
                link: Id::Site,
            }),
            body: Node::Column(body),
            buttons: vec![Node::Button {
                id: Id::Close,
                label: self.line(IDS_CLOSE),
                kind: ButtonKind::Primary,
                browser: false,
            }],
            disabled: self.disabled(),
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        // The keys of the capture, its end and the timer; anything else ends a capture first —
        // the hotkey is suspended only while the field takes the keys.
        //
        // Решение 208.228: either field takes the keys of its own capture; «Задать» of either arms
        // its capture, and stops it while it is armed.
        let event = match event {
            Event::KeyDown {
                id: Id::Hotkey | Id::SwitchKey,
                vk,
            } => {
                self.capture_step(CaptureEvent::KeyDown(vk, Modifiers::held_now()));
                return Reply::Stay;
            }
            Event::KeyUp {
                id: Id::Hotkey | Id::SwitchKey,
                vk,
            } => {
                self.capture_step(CaptureEvent::KeyUp(vk));
                return Reply::Stay;
            }
            Event::FocusLost {
                id: id @ (Id::Hotkey | Id::SwitchKey),
                to,
            } => {
                let own = if id == Id::Hotkey {
                    Id::HotkeySet
                } else {
                    Id::SwitchKeySet
                };
                self.capture_step(CaptureEvent::FocusLost {
                    to_capture_button: to == Some(own),
                });
                return Reply::Stay;
            }
            Event::Timer => {
                self.timer_came();
                return Reply::Stay;
            }
            Event::Pressed(id @ (Id::HotkeySet | Id::SwitchKeySet | Id::CtrlWarnOther)) => {
                let target = if id == Id::HotkeySet {
                    CaptureTarget::Fix
                } else {
                    CaptureTarget::Switch
                };
                let armed = self.capture.take().map(|capture| capture.session.target());
                if armed != Some(target) || id == Id::CtrlWarnOther {
                    self.arm_capture(target);
                }
                return Reply::Stay;
            }
            Event::Pressed(Id::CtrlWarnSet) => {
                self.capture = None;
                if let Some(side) = self.ctrl_warning.take()
                    && let Some(tap) = self.config.switch_hotkey.ctrl_tap(side)
                {
                    self.change(|config| config.switch_hotkey = tap);
                }
                return Reply::Stay;
            }
            event => {
                self.capture = None;
                event
            }
        };
        match event {
            Event::Pressed(Id::Close) => return Reply::Close,
            Event::Selected(Id::Sections, index) => {
                if let Some(section) = Section::ALL.get(index) {
                    self.section = *section;
                }
                self.ctrl_warning = None;
            }
            // Решение 208.228: the mode; the second key is kept in the file either way.
            Event::Selected(Id::HotkeyMode, index) => {
                self.ctrl_warning = None;
                self.change(|config| config.switch_hotkey.enabled = index == 1);
            }
            Event::Toggled(id, on) => self.change(|config| match id {
                Id::Autostart => config.general.autostart = on,
                Id::Sound => config.feedback.sound = on,
                Id::Selection => config.selection.enabled = on,
                Id::TrayFlag => config.general.tray_flag = on,
                Id::Updates => config.letters.release_window = on,
                Id::Log => config.diagnostics.log_enabled = on,
                _ => {}
            }),
            Event::Selected(Id::Language, index) => {
                if let Some(picked) = self.languages.languages.get(index).copied() {
                    self.change(|config| {
                        if picked != config.general.language.in_force() {
                            config.general.language = picked;
                        }
                    });
                }
            }
            Event::Selected(Id::Theme, index) => {
                let setting = settings::theme_from_combo_index(index as isize);
                self.change(|config| config.general.theme = setting);
            }
            Event::Selected(Id::Mode, index) => {
                let mode = if index == 1 {
                    LayoutMode::Cycle
                } else {
                    LayoutMode::Pair
                };
                self.change(|config| config.layouts.mode = mode);
            }
            Event::Selected(id @ (Id::PairSource | Id::PairTarget), index) => {
                if let Some(layout) = self.session.get(index).copied() {
                    let spec = settings::spec_text(layout, &self.session);
                    self.change(|config| {
                        if id == Id::PairSource {
                            config.layouts.pair_source = spec;
                        } else {
                            config.layouts.pair_target = spec;
                        }
                    });
                }
            }
            Event::Selected(Id::Cycle, index) => self.cycle_selected = index,
            Event::RowChecked(Id::Cycle, index, on) => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.checked = on;
                    self.cycle_changed();
                }
            }
            Event::Pressed(id @ (Id::Up | Id::Down)) => {
                let from = self.cycle_selected;
                let to = if id == Id::Up {
                    from.checked_sub(1)
                } else {
                    Some(from + 1).filter(|to| *to < self.rows.len())
                };
                if let Some(to) = to {
                    self.rows.swap(from, to);
                    self.cycle_selected = to;
                    self.cycle_changed();
                }
            }
            Event::RowChecked(Id::Exclusions, index, on) => {
                if let Some(marked) = self.marked.get_mut(index) {
                    *marked = on;
                }
            }
            Event::Pressed(Id::Remove) => {
                let keep: Vec<String> = self
                    .config
                    .exclusions
                    .processes
                    .iter()
                    .zip(self.marked.iter())
                    .filter(|(_, marked)| !**marked)
                    .map(|(name, _)| name.clone())
                    .collect();
                self.change(|config| config.exclusions.processes = keep);
                self.marked = vec![false; self.config.exclusions.processes.len()];
            }
            Event::Edited(Id::ExclusionName, typed) => self.exclusion_name = typed,
            Event::Pressed(Id::Add) => {
                let name = self.exclusion_name.trim().to_owned();
                if settings::exclusion_verdict(&name, &self.config.exclusions.processes)
                    == ExclusionVerdict::Add
                {
                    self.change(|config| config.exclusions.processes.push(name));
                    if !self.refused {
                        self.exclusion_name.clear();
                    }
                    self.marked = vec![false; self.config.exclusions.processes.len()];
                }
            }
            Event::Pressed(Id::OpenFolder) => {
                if let Some(folder) = crate::diag::log_dir() {
                    let _ = settings::open_in_the_shell(&folder.display().to_string());
                }
            }
            Event::Pressed(Id::SaveJournal) => {
                let saved = (self.save_journal)();
                self.journal = Some(saved);
                self.journal_until = saved.then(|| {
                    Instant::now() + Duration::from_millis(settings::JOURNAL_SAVED_MS.into())
                });
            }
            // The second press stops the count (as the old button did); the first starts it.
            Event::Pressed(Id::Pick) => {
                self.pick_note = None;
                self.pick_at = match self.pick_at {
                    Some(_) => None,
                    None => Some(
                        Instant::now() + Duration::from_secs(crate::guard::CAPTURE_SECONDS.into()),
                    ),
                };
            }
            _ => {}
        }
        Reply::Stay
    }

    fn title(&self) -> String {
        self.line(IDS_DIALOG_CAPTION)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.config.general.theme
    }

    /// Only while something waits — a pair of `Shift`, the count of a pick, «Журнал сохранён»: at
    /// rest nothing ticks (NFR-10). The period is what is left till the nearest moment, so the
    /// timer is armed anew whenever that moment moves.
    fn timer(&self) -> Option<u32> {
        let now = Instant::now();
        self.deadlines().min().map(|at| {
            let left = at.saturating_duration_since(now).as_millis();
            u32::try_from(left).unwrap_or(u32::MAX).max(1)
        })
    }
}

/// The window the menu opens: the settings and what the program adds to them.
struct Product {
    settings: Settings,
    /// The program's window: the owner of this one and of the wizard it opens.
    owner: HWND,
    /// The record `settings::dialog_is_open` reads, dropped with the window's content.
    _session: settings::DialogSession,
}

impl Product {
    /// The program as it is now, for «Состояние»: the tray's state and the lines for the author.
    fn read_program(&mut self) {
        let Some(now) = crate::tray::menu_now() else {
            return;
        };
        let details = settings::state_details(
            self.settings.config.diagnostics.log_enabled,
            self.settings.session(),
        );
        self.settings.showing_program(Program {
            enabled: now.enabled,
            fail_safe: now.fail_safe,
            details,
            path: settings::autostart_value(),
        });
    }
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.settings.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        // Д6: `[letters]` is the program's own state — «От автора», the letters and the clock of
        // the feed write it while this window is open — and a change here puts in force the whole
        // configuration. So the section is the one in force at every event: the switch «Сообщать
        // об обновлениях программы» shows what «От автора» wrote, and a change made here does not
        // bring back the copy of the opening.
        if let Some(now) = crate::tray::config_now() {
            self.settings.config.letters = now.letters;
        }
        let reply = match event {
            Event::Pressed(Id::Write) => {
                crate::letters::open_wizard(self.owner);
                Reply::Stay
            }
            // The address under the sections opens the page of the program in the language of the
            // window, through the gate of `letters::open_link`, as in «О программе».
            Event::Pressed(Id::Site) => {
                crate::letters::open_link(crate::letters::links::program_url(
                    self.settings.language,
                ));
                Reply::Stay
            }
            // T-120-22: the wizard on the page of the idea, marked with the language read here.
            Event::Pressed(Id::Translation) => {
                crate::letters::open_wizard_for_translation(self.owner, self.settings.language);
                Reply::Stay
            }
            // C6 (а) of T-120-21: the lines for the author in a small window of their own.
            Event::Pressed(Id::Details) => {
                if let (Some(window), Some(program)) =
                    (settings::open_window(), self.settings.program.clone())
                {
                    open_details(
                        window,
                        self.settings.language,
                        self.settings.config.general.theme,
                        program,
                    );
                }
                Reply::Stay
            }
            event => self.settings.event(event),
        };
        // «Состояние» follows what was just put in force (and what the menu did meanwhile).
        if reply == Reply::Stay {
            self.read_program();
        }
        reply
    }

    fn title(&self) -> String {
        self.settings.title()
    }

    fn language(&self) -> (String, bool) {
        self.settings.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.settings.theme()
    }

    fn timer(&self) -> Option<u32> {
        self.settings.timer()
    }
}

/// Opens the window of the settings on `owner`, the program's window, over the configuration in
/// force; answers the window, or none. One at a time: while it or «О программе» is up nothing is
/// made — the menu greys «Настройки…» for the same reason (`tray::dialog_locks_command`).
pub fn open(owner: HWND) -> Option<HWND> {
    if settings::dialog_is_open() || settings::about_is_open() {
        return None;
    }
    let config = crate::tray::config_now()?;
    let language = config.general.language.in_force();
    let mut product = Product {
        settings: Settings::new(
            config,
            language,
            Box::new(|config: &Config| crate::tray::apply_settings_now(config).unwrap_or(false)),
        ),
        owner,
        _session: settings::DialogSession::open(),
    };
    product.read_program();

    #[cfg(feature = "testing")]
    let made = if testing::hidden() {
        host::create(product, Some(owner))
    } else {
        host::open(product, Some(owner))
    };
    #[cfg(not(feature = "testing"))]
    let made = host::open(product, Some(owner));

    match made {
        Ok(window) => {
            settings::DialogSession::record(window);
            Some(window)
        }
        Err(error) => {
            crate::app::report_non_critical("CreateWindowExW", &error);
            None
        }
    }
}

/// The door of the tests: [`open`] makes the window as the product makes it and leaves it hidden.
#[cfg(feature = "testing")]
pub mod testing {
    use std::cell::Cell;

    thread_local! {
        static HIDDEN: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn hidden() -> bool {
        HIDDEN.with(Cell::get)
    }

    /// Runs `f` with the window of [`super::open`] left hidden on this thread.
    pub fn hidden_while<R>(f: impl FnOnce() -> R) -> R {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                HIDDEN.with(|hidden| hidden.set(false));
            }
        }

        HIDDEN.with(|hidden| hidden.set(true));
        let _reset = Reset;
        f()
    }
}

/// Every state the window shows, for the instrument of fitting: its five sections, and
/// «Раскладки» once more with «Несколько раскладок». The layouts are three common ones, named by
/// the rule the window names them by.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let session = settings::layouts_for_fitting();
    let mut config = Config::default();
    // More names than the list shows: the list stands at its full height, the worst case.
    config.exclusions.processes = [
        "game.exe",
        "example.exe",
        "setup.exe",
        "telegram.exe",
        "notepad.exe",
        "photoshop.exe",
        "obs64.exe",
        "steam.exe",
    ]
    .map(str::to_owned)
    .to_vec();
    let mut window = Settings::with_session(config, language, session, Box::new(|_| true));
    let mut all = Vec::new();
    // «Диагностика» with its block «Состояние» (T-120-21), the program working.
    window.showing_program(sample_program(language, true, false));
    for section in Section::ALL.iter().copied() {
        window.section = section;
        all.push((format!("settings {section:?}"), window.describe()));
    }
    // The note of a refused write at the top of «Общие» (C6 of the sheet).
    window.section = Section::General;
    window.refused = true;
    all.push(("settings General refused".to_owned(), window.describe()));
    window.refused = false;
    window.section = Section::Layouts;
    window.config.layouts.mode = LayoutMode::Cycle;
    all.push(("settings Layouts cycle".to_owned(), window.describe()));
    // «Состояние»: the two other states of the program.
    window.section = Section::Diagnostics;
    for (name, enabled, fail_safe) in [("paused", false, false), ("restart", false, true)] {
        window.showing_program(sample_program(language, enabled, fail_safe));
        all.push((format!("settings Diagnostics {name}"), window.describe()));
    }
    // Решение 208.228: «Две клавиши» — the second key not set, set (and the line of «Состояние» it
    // adds), and the warning about a single `Ctrl`, the tallest of the section.
    window.config.switch_hotkey.enabled = true;
    window.section = Section::Hotkey;
    all.push(("settings Hotkey two empty".to_owned(), window.describe()));
    window.config.switch_hotkey.key = "ScrollLock".to_owned();
    all.push(("settings Hotkey two".to_owned(), window.describe()));
    window.section = Section::Diagnostics;
    window.showing_program(sample_program(language, true, false));
    all.push((
        "settings Diagnostics two keys".to_owned(),
        window.describe(),
    ));
    window.section = Section::Hotkey;
    window.ctrl_warning = Some(RIGHT_CTRL);
    all.push(("settings Hotkey ctrl warning".to_owned(), window.describe()));
    all
}

/// A program for the pictures and the fitting: the lines for the author as `settings::state_details`
/// makes them, in `language`, with a path of the `Run` key as long as an installed program has.
fn sample_program(language: Language, enabled: bool, fail_safe: bool) -> Program {
    let line = |id| settings::text_in(language, id);
    let path = r"C:\Program Files\Lang_Switcher\LangSwitcher.exe".to_owned();
    let joined =
        |a: String, b: String| line(IDS_STATE_JOINED).replace("{0}", &a).replace("{1}", &b);
    let none = "0";
    let three = "3";
    let health = line(IDS_STATE_HEALTH)
        .replace("{0}", none)
        .replace("{1}", none)
        .replace("{2}", none);
    let counts = line(IDS_STATE_LAYOUTS)
        .replace("{0}", three)
        .replace("{1}", three)
        .replace("{2}", none);
    let autostart = line(IDS_STATE_AUTOSTART)
        .replace("{0}", &line(IDS_AUTOSTART_PRESENT).replace("{0}", &path));
    let pair = line(IDS_STATE_PAIR)
        .replace("{0}", "English (United States)")
        .replace("{1}", "Русский (Россия)");
    Program {
        enabled,
        fail_safe,
        details: vec![
            joined(line(IDS_STATE_WORKING), health),
            counts,
            autostart,
            joined(pair, line(IDS_STATE_NO_FAILURES)),
        ],
        path: Some(path),
    }
}

/// The elements of the window of «Подробности для автора».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailsId {
    Copy,
    Close,
}

/// What «Копировать» puts the words on the clipboard with, and whether it did.
pub type Copier = Box<dyn Fn(&str) -> bool>;

/// **The small window of «Подробности для автора»** (T-120-21 Б, C6 (а) of the controller): the
/// four lines of the author word for word, in the face of code, the path of the `Run` key on a line
/// of its own (the mock-up's `.spath`); «Копировать» puts them on the clipboard, «Закрыть» and Esc
/// close. An ordinary window of the shell at the least size of the passport, owned by the window of
/// the settings.
pub struct Details {
    pub language: Language,
    pub theme: ThemeSetting,
    pub program: Program,
    copy: Copier,
}

impl Details {
    /// The window over `program`; «Копировать» goes through `copy`.
    pub fn new(language: Language, theme: ThemeSetting, program: Program, copy: Copier) -> Self {
        Self {
            language,
            theme,
            program,
            copy,
        }
    }

    fn line(&self, id: u16) -> String {
        settings::text_in(self.language, id)
    }

    /// The lines as the window shows them: the line of the `Run` key without its path, the path on
    /// a line of its own.
    pub fn shown_lines(&self) -> Vec<String> {
        let mut shown = Vec::new();
        for line in &self.program.details {
            match self
                .program
                .path
                .as_deref()
                .filter(|path| line.contains(*path))
            {
                Some(path) => {
                    let head = line.replace(path, "");
                    shown.push(head.trim_end_matches([',', ' ']).to_owned());
                    shown.push(path.to_owned());
                }
                None => shown.push(line.clone()),
            }
        }
        shown
    }

    /// What «Копировать» puts on the clipboard: the four lines word for word, one to a line.
    pub fn clipboard_words(&self) -> String {
        self.program.details.join("\r\n")
    }
}

impl Content for Details {
    type Id = DetailsId;

    fn describe(&self) -> Window<DetailsId> {
        let mut body = vec![Node::Header {
            eyebrow: Some(self.line(IDS_SETTINGS_EYEBROW)),
            title: self.line(IDS_STATUS_DETAILS),
            subtitle: None,
        }];
        body.push(Node::Rows(
            self.shown_lines()
                .into_iter()
                .map(|line| Node::Text(Role::Code, line))
                .collect(),
        ));
        Window {
            kind: WindowKind::Ordinary,
            art: None,
            brand: None,
            body: Node::Column(body),
            buttons: vec![
                Node::Button {
                    id: DetailsId::Copy,
                    label: self.line(IDS_STATUS_COPY),
                    kind: ButtonKind::Normal,
                    browser: false,
                },
                Node::Button {
                    id: DetailsId::Close,
                    label: self.line(IDS_CLOSE),
                    kind: ButtonKind::Primary,
                    browser: false,
                },
            ],
            disabled: Vec::new(),
        }
    }

    fn event(&mut self, event: Event<DetailsId>) -> Reply {
        match event {
            Event::Pressed(DetailsId::Copy) => {
                let _ = (self.copy)(&self.clipboard_words());
                Reply::Stay
            }
            Event::Pressed(DetailsId::Close) => Reply::Close,
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        self.line(IDS_STATUS_DETAILS)
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.theme
    }

    fn least(&self) -> bool {
        true
    }
}

/// Opens the window of «Подробности для автора» over `owner`, the window of the settings; under
/// the `testing` feature the hidden door of this module makes it without showing it.
pub fn open_details(
    owner: HWND,
    language: Language,
    theme: ThemeSetting,
    program: Program,
) -> Option<HWND> {
    let copy: Copier =
        Box::new(move |words: &str| crate::selection::write_unicode_text(owner, words).is_ok());
    let details = Details::new(language, theme, program, copy);

    #[cfg(feature = "testing")]
    let made = if testing::hidden() {
        host::create(details, Some(owner))
    } else {
        host::open(details, Some(owner))
    };
    #[cfg(not(feature = "testing"))]
    let made = host::open(details, Some(owner));

    match made {
        Ok(window) => Some(window),
        Err(error) => {
            crate::app::report_non_critical("CreateWindowExW", &error);
            None
        }
    }
}

/// The window of «Подробности для автора» for the instrument of fitting and the pictures, in
/// `language`.
pub fn details_states(language: Language) -> Vec<(String, Window<DetailsId>)> {
    let details = Details::new(
        language,
        ThemeSetting::Light,
        sample_program(language, true, false),
        Box::new(|_| true),
    );
    vec![("settings details".to_owned(), details.describe())]
}
