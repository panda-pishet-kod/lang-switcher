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
    self, CaptureEvent, CaptureSession, CaptureStep, Config, ExclusionPick, ExclusionVerdict,
    Hotkey, IDS_AUTOSTART, IDS_CAPTURE_HINT, IDS_CAPTURE_PROMPT, IDS_CAPTURE_SHIFT_AGAIN,
    IDS_CLOSE, IDS_CYCLE_DOWN, IDS_CYCLE_HINT, IDS_CYCLE_UP, IDS_DIALOG_CAPTION, IDS_EXCLUSION_ADD,
    IDS_EXCLUSION_ADDED, IDS_EXCLUSION_ALREADY, IDS_EXCLUSION_AUTO, IDS_EXCLUSION_HINT,
    IDS_EXCLUSION_MANUAL, IDS_EXCLUSION_NOT_PICKED, IDS_EXCLUSION_PICK, IDS_EXCLUSION_PICK_COUNT,
    IDS_EXCLUSION_PICK_NOTE, IDS_EXCLUSION_REMOVE, IDS_GROUP_DIAGNOSTICS, IDS_GROUP_EXCLUSIONS,
    IDS_GROUP_GENERAL, IDS_GROUP_HOTKEY, IDS_GROUP_LAYOUTS, IDS_HOTKEY_LABEL, IDS_HOTKEY_SET,
    IDS_HOTKEY_STOP, IDS_LANGUAGE_LABEL, IDS_LOG_DIR_LABEL, IDS_LOG_ENABLED, IDS_LOG_OPEN,
    IDS_LOG_SAVE, IDS_LOG_SAVE_FAILED, IDS_LOG_SAVED, IDS_MODE_CYCLE, IDS_MODE_PAIR, IDS_NEW_BADGE,
    IDS_PAIR_SOURCE, IDS_PAIR_TARGET, IDS_SELECTION_ENABLED, IDS_SETTINGS_NOT_SAVED, IDS_SOUND,
    IDS_THEME_DARK, IDS_THEME_LABEL, IDS_THEME_LIGHT, IDS_THEME_SYSTEM, IDS_TRAY_FLAG,
    IDS_UPDATES_SWITCH, IDS_WIZARD_ATTACH_SETTINGS, IDS_WRITE_TO_AUTHOR, Language, LanguageOrder,
    LayoutMode, LayoutRow, Modifiers, Refusal,
};
use crate::theme::ThemeSetting;
use crate::ui::host::{self, Content, Reply};
use crate::ui::scene::{
    Art, ButtonKind, Event, FieldKind, ListItem, Node, Role, Surface, Window, WindowKind,
};
use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::UI::WindowsAndMessaging::GetMessageTime;
use std::time::{Duration, Instant};

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
    Close,
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
    /// The last capture took a combination or the double press: the line under the field says
    /// once which half stays with the programs (task T-93-2), till the next capture.
    combination_taken: bool,
    /// «Выбрать окно программы» counts: the moment the window in front is taken.
    pick_at: Option<Instant>,
    /// What the last pick found, said under its button; none — the line at rest.
    pick_note: Option<String>,
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
            combination_taken: false,
            pick_at: None,
            pick_note: None,
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
        vec![
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
                hint: None,
                on: self.config.general.tray_flag,
                // The setting came with 1.7.0 (T-119-2); the badge «новое» stays while the
                // window is the window of 1.8.0 (T-120-20, passport §6).
                badge: Some(self.line(IDS_NEW_BADGE)),
            },
            toggle(
                Id::Updates,
                self.line(IDS_UPDATES_SWITCH),
                self.config.letters.feed,
            ),
            Node::Choice {
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
            },
            Node::Segments {
                id: Id::Theme,
                label: self.line(IDS_THEME_LABEL),
                options: [IDS_THEME_SYSTEM, IDS_THEME_LIGHT, IDS_THEME_DARK]
                    .map(|id| self.line(id))
                    .to_vec(),
                selected: settings::theme_combo_index(self.config.general.theme),
            },
        ]
    }

    /// The field of the hotkey and «Задать» beside it; under them, one line: at rest the ways a
    /// hotkey may be, once after a capture what stays with the programs, in a capture the way out
    /// or why the press was refused (решение 82.6, task T-95-2).
    fn hotkey(&self) -> Vec<Node<Id>> {
        let (value, set, line) = match &self.capture {
            Some(capture) => (
                if capture.pair_until.is_some() {
                    settings::pending_shift_name()
                } else {
                    self.line(IDS_CAPTURE_PROMPT)
                },
                IDS_HOTKEY_STOP,
                match capture.line {
                    CaptureLine::Hint => self.line(IDS_CAPTURE_HINT),
                    CaptureLine::Refused(refusal) => self.line(refusal.string_id()),
                    CaptureLine::ShiftAgain => self.line(IDS_CAPTURE_SHIFT_AGAIN),
                },
            ),
            None => (
                settings::effective_hotkey_name(&self.config.hotkey),
                IDS_HOTKEY_SET,
                settings::hotkey_line_in(
                    self.language,
                    &self.config.hotkey,
                    self.combination_taken,
                ),
            ),
        };
        vec![
            Node::Row(vec![
                Node::Field {
                    id: Id::Hotkey,
                    label: self.line(IDS_HOTKEY_LABEL),
                    value,
                    hint: String::new(),
                    kind: FieldKind::Key {
                        capturing: self.capture.is_some(),
                    },
                },
                button(Id::HotkeySet, self.line(set)),
            ]),
            Node::Text(Role::Small, line),
        ]
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
            nodes.push(Node::Choice {
                id: Id::PairSource,
                label: self.line(IDS_PAIR_SOURCE),
                options: labels.clone(),
                selected: self.layout_index(&self.config.layouts.pair_source),
            });
            nodes.push(Node::Choice {
                id: Id::PairTarget,
                label: self.line(IDS_PAIR_TARGET),
                options: labels,
                selected: self.layout_index(&self.config.layouts.pair_target),
            });
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
            toggle(
                Id::Log,
                self.line(IDS_LOG_ENABLED),
                self.config.diagnostics.log_enabled,
            ),
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
        nodes
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
    fn take_hotkey(&mut self, hotkey: Hotkey) {
        self.capture = None;
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
                eyebrow: Some(self.line(IDS_WIZARD_ATTACH_SETTINGS)),
                title: self.line(self.section.title()),
                subtitle: None,
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
            art: Some(Art::About),
            brand: None,
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
        let event = match event {
            Event::KeyDown { id: Id::Hotkey, vk } => {
                self.capture_step(CaptureEvent::KeyDown(vk, Modifiers::held_now()));
                return Reply::Stay;
            }
            Event::KeyUp { id: Id::Hotkey, vk } => {
                self.capture_step(CaptureEvent::KeyUp(vk));
                return Reply::Stay;
            }
            Event::FocusLost { id: Id::Hotkey, to } => {
                self.capture_step(CaptureEvent::FocusLost {
                    to_capture_button: to == Some(Id::HotkeySet),
                });
                return Reply::Stay;
            }
            Event::Timer => {
                self.timer_came();
                return Reply::Stay;
            }
            Event::Pressed(Id::HotkeySet) => {
                if self.capture.take().is_none() {
                    self.capture = Some(Capture {
                        session: CaptureSession::arm(self.config.hotkey.clone()),
                        line: CaptureLine::Hint,
                        pair_until: None,
                    });
                    self.combination_taken = false;
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
            }
            Event::Toggled(id, on) => self.change(|config| match id {
                Id::Autostart => config.general.autostart = on,
                Id::Sound => config.feedback.sound = on,
                Id::Selection => config.selection.enabled = on,
                Id::TrayFlag => config.general.tray_flag = on,
                Id::Updates => config.letters.feed = on,
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

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.settings.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Pressed(Id::Write) => {
                crate::letters::open_wizard(self.owner);
                Reply::Stay
            }
            event => self.settings.event(event),
        }
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
    let product = Product {
        settings: Settings::new(
            config,
            language,
            Box::new(|config: &Config| crate::tray::apply_settings_now(config).unwrap_or(false)),
        ),
        owner,
        _session: settings::DialogSession::open(),
    };

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
    all
}
