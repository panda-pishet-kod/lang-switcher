//! **The window in the corner** — task T-119-6 (решения 208.90–208.94, 208.119, 208.141): one window
//! by the notification area for the hint of the flag (T-119-5), «Обновлено» and, in the next stage,
//! the announcement of a letter.
//!
//! # On the shell (T-120-14)
//!
//! What the window says and when — [`Content`], the hint, «Обновлено», «Что нового в X…» — is this
//! module's, E119's behaviour unchanged. How it looks is the shell's: the description of
//! `ui::windows::corner` (the sizes of the passport, the mirror by the language of the program) in
//! a popup of the shell (`ui::host::open_popup`) — no caption, above the windows, not taking the
//! focus, by the notification area (`TrayNotifyWnd` of the taskbar), not «the lower right corner».
//! The drawing of E119 (GDI) went with T-120-14, and its tests of the look with it.

use crate::layouts::LayoutId;
use windows::Win32::Foundation::{HWND, RECT};

// -----------------------------------------------------------------------------------------
// What the window says
// -----------------------------------------------------------------------------------------

/// What a button of the window does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Closes the window.
    Close,
    /// Opens an address of the program's own pages (through `settings::open_in_the_shell`) and
    /// closes the window.
    Open(&'static str),
}

/// What the window in the corner says — the parts in the order of the mock-up. The words are the
/// caller's, read from the string tables.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    /// What the window is about — the end of the header «LANG SWITCHER · …», written in capitals.
    pub about: String,
    /// The title.
    pub title: String,
    /// The text, up to three lines.
    pub text: String,
    /// The link under the text and its address, if any.
    pub link: Option<(String, &'static str)>,
    /// The main button — on the right, filled, two shares of the width.
    pub main: (String, Action),
    /// The second button — on the left, framed, one share; none for a window of one button.
    pub second: Option<(String, Action)>,
    /// The picture under the buttons — the animation of the flag (T-119-6).
    pub picture: bool,
    /// The label of the check box at the bottom, if any.
    pub check: Option<String>,
}

impl Content {
    /// The text of the window — for the description of the shell, which names no `text` (the
    /// guard of the window descriptions, rule 2).
    pub fn words(&self) -> &str {
        &self.text
    }
}

/// How the window ended — what its caller is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct End {
    /// Which way: the main button, the second, the ✕ or `Esc`.
    pub by: EndBy,
    /// Whether the check box was ticked when it ended.
    pub checked: bool,
}

/// The way a window in the corner ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndBy {
    /// The main button.
    Main,
    /// The second button.
    Second,
    /// The ✕ or `Esc`.
    Close,
}

// -----------------------------------------------------------------------------------------
// The picture — the animation of the flag, drawn by code with the person's own layouts
// -----------------------------------------------------------------------------------------

/// The length of one round of the animation, ms (the mock-up: 5,6 s).
pub const ROUND_MS: u32 = 5600;

/// What the picture shows at a moment of its round — the mock-up's timing: the flag of the first
/// layout; from 1,0 s the plate «Alt + Shift» and at 1,4 s the flag of the second; from 2,3 s the
/// cursor goes to the flag, from 2,8 s the list is open, from 3,5 s the first row is hovered, at
/// 4,2 s the list closes on the first layout again. Pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Which of the two layouts the flag shows: 0 or 1.
    pub flag: usize,
    /// The plate of the keys is up.
    pub plate: bool,
    /// The cursor: none, or its share of the way to the flag in thousandths.
    pub cursor: Option<i32>,
    /// The list is open.
    pub list: bool,
    /// The first row of the list is hovered.
    pub hover: bool,
}

/// [`Frame`] at `ms` into the round. Pure.
pub fn frame(ms: u32) -> Frame {
    let t = ms % ROUND_MS;

    Frame {
        flag: usize::from((1400..4200).contains(&t)),
        plate: (1000..1900).contains(&t),
        cursor: (2300..4300).contains(&t).then(|| {
            let travel = i32::try_from((t - 2300).min(500)).unwrap_or(500);
            travel * 2
        }),
        list: (2800..4200).contains(&t),
        hover: (3500..4200).contains(&t),
    }
}

/// The two layouts the picture shows — the one in force first and the next of the session; one
/// layout alone shows itself twice.
pub fn picture_layouts(session: &[LayoutId], current: LayoutId) -> (LayoutId, LayoutId) {
    let first = if session.contains(&current) {
        current
    } else {
        session.first().copied().unwrap_or(current)
    };
    let second = session
        .iter()
        .copied()
        .find(|layout| *layout != first)
        .unwrap_or(first);

    (first, second)
}

fn notification_area() -> Option<RECT> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
    use windows::core::w;

    // SAFETY: constant class names; a null answer is checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: as above.
    let area =
        unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.unwrap_or(taskbar);
    let mut rect = RECT::default();

    // SAFETY: a window handle by value and a local rectangle.
    unsafe { GetWindowRect(area, &mut rect) }.ok()?;
    Some(rect)
}

// -----------------------------------------------------------------------------------------
// The window — a popup of the shell (T-120-14)
// -----------------------------------------------------------------------------------------

/// The window on the screen — one at a time, on the UI thread: its handle, whom to tell, the box.
struct Shown {
    hwnd: HWND,
    /// The main action of what it says: the hint of the flag is the one that opens the settings of
    /// the taskbar (`hint_tick`).
    main: Action,
    on_end: fn(End),
    checked: std::cell::Cell<bool>,
}

thread_local! {
    static SHOWN: std::cell::RefCell<Option<Shown>> = const { std::cell::RefCell::new(None) };
}

/// Whether a window in the corner is on the screen.
pub fn is_shown() -> bool {
    SHOWN.with(|shown| {
        shown.borrow().as_ref().is_some_and(|state| {
            // SAFETY: a handle by value; a window gone with the program's other windows answers no.
            unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(state.hwnd)) }.as_bool()
        })
    })
}

/// What the window says, as the shell shows it, and what it tells its caller.
struct CornerWindow {
    content: Content,
    checked: bool,
    language: crate::settings::Language,
    /// When the window came: the clock of its picture.
    started: std::time::Instant,
    /// The person's two layouts the picture shows.
    layouts: (LayoutId, LayoutId),
    /// The interface of Windows reads right to left: the picture is turned.
    icons_left: bool,
}

impl CornerWindow {
    /// The picture of the taskbar at this moment of its round.
    fn picture(&self) -> crate::ui::scene::Taskbar {
        let ms =
            u32::try_from(self.started.elapsed().as_millis() % u128::from(ROUND_MS)).unwrap_or(0);
        let now = frame(ms);
        let (one, two) = self.layouts;
        crate::ui::scene::Taskbar {
            layouts: [one, two],
            codes: [
                crate::flags::language_code(one),
                crate::flags::language_code(two),
            ],
            shown: now.flag,
            plate: now.plate,
            cursor: now.cursor.map(|share| share as f32 / 1000.0),
            list: now.list,
            hover: now.hover,
            icons_left: self.icons_left,
        }
    }
}

impl crate::ui::host::Content for CornerWindow {
    type Id = crate::ui::windows::corner::Id;

    fn describe(&self) -> crate::ui::scene::Window<Self::Id> {
        let picture = self.content.picture.then(|| self.picture());
        crate::ui::windows::corner::describe(&self.content, self.checked, picture)
    }

    fn event(&mut self, event: crate::ui::scene::Event<Self::Id>) -> crate::ui::host::Reply {
        use crate::ui::host::Reply;
        use crate::ui::scene::Event;
        use crate::ui::windows::corner::Id;

        match event {
            Event::Pressed(Id::Close) => {
                end(EndBy::Close);
                Reply::Close
            }
            Event::Pressed(Id::Main) => {
                act(self.content.main.1, EndBy::Main);
                Reply::Close
            }
            Event::Pressed(Id::Second) => match self.content.second.as_ref() {
                Some((_, action)) => {
                    act(*action, EndBy::Second);
                    Reply::Close
                }
                None => Reply::Stay,
            },
            Event::Pressed(Id::Link) => {
                if let Some((_, address)) = self.content.link.as_ref() {
                    let _ = crate::settings::open_in_the_shell(address);
                }
                Reply::Stay
            }
            Event::Toggled(Id::Check, on) => {
                self.checked = on;
                SHOWN.with(|shown| {
                    if let Some(state) = shown.borrow().as_ref() {
                        state.checked.set(on);
                    }
                });
                Reply::Stay
            }
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        crate::APP_NAME.to_owned()
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    /// The theme of the system, as the window in the corner of E119 took it.
    fn theme(&self) -> crate::theme::ThemeSetting {
        crate::theme::ThemeSetting::System
    }

    fn escape(&self) -> Option<Self::Id> {
        Some(crate::ui::windows::corner::Id::Close)
    }
}

/// Shows `content` in the corner by the notification area — a popup of the shell: no caption,
/// above the windows, not taking the focus — with the notification sound when `sound`. Answers
/// false when one is already shown or Windows refused the window; `on_end` is told how it ended.
pub fn show(content: Content, sound: bool, on_end: fn(End)) -> bool {
    if is_shown() {
        return false;
    }
    let main = content.main.1;
    let session = crate::layouts::enumerate().unwrap_or_default();
    let window = CornerWindow {
        content,
        checked: false,
        language: crate::settings::ui_language(),
        started: std::time::Instant::now(),
        layouts: picture_layouts(&session, crate::switch::read().layout),
        icons_left: crate::ui::host::windows_reads_right_to_left(),
    };
    let Ok(hwnd) = crate::ui::host::open_popup(window, notification_area()) else {
        return false;
    };
    SHOWN.with(|shown| {
        *shown.borrow_mut() = Some(Shown {
            hwnd,
            main,
            on_end,
            checked: std::cell::Cell::new(false),
        });
    });
    if sound {
        play_notification_sound();
    }
    true
}

fn play_notification_sound() {
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

    // SAFETY: a constant NUL-terminated alias; asynchronous, nothing of this frame is kept.
    let _ = unsafe {
        PlaySoundW(
            windows::core::w!("Notification.Default"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        )
    };
}

/// Ends the window: it goes, its state goes, and its caller is told — outside the borrow.
/// Ends the window: its state goes, the window is asked to close, and its caller is told — once.
fn end(by: EndBy) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    let Some(state) = SHOWN.with(|shown| shown.borrow_mut().take()) else {
        return;
    };
    // SAFETY: a message to a window of this thread; one already gone ignores it.
    let _ = unsafe { PostMessageW(Some(state.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    (state.on_end)(End {
        by,
        checked: state.checked.get(),
    });
}

/// Does what a button asks.
fn act(action: Action, by: EndBy) {
    if let Action::Open(address) = action {
        let _ = crate::settings::open_in_the_shell(address);
    }

    end(by);
}

// -----------------------------------------------------------------------------------------
// «Обновлено» — the block of the last important release, and when its window comes
// -----------------------------------------------------------------------------------------

/// The block «the last important release» — **exactly one in the build** (T-119-6): its version and
/// whether it has the picture. Its words are strings under permanent names (`IDS_RELEASE_*`, no
/// version in a name); «Как пользоваться» is the page of the program in the person's language. An
/// important release replaces the block; a minor one leaves it (the order of a release).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Release {
    /// The version of the release.
    pub version: &'static str,
    /// Whether its window has the animation of the flag.
    pub picture: bool,
}

/// The block of this build.
pub const LAST_IMPORTANT: Release = Release {
    version: "1.8.0",
    picture: true,
};

/// Whether the start after an update owes the window «Обновлено» — T-119-6: the version updated
/// from (`previous`, the version last seen) is not empty — not the first install, which has
/// «Привет» — and older than the block's, and the block is not newer than the build. Pure.
pub fn release_is_due(previous: &str, build: &str, block: &str) -> bool {
    !previous.is_empty()
        && crate::letters::version_is_newer(block, previous)
        && !crate::letters::version_is_newer(block, build)
}

/// The UI window's timer that waits for the quiet moment of «Обновлено» — once a second while a
/// window is due, gone the moment it is shown.
pub const QUIET_TIMER: usize = 31;
const QUIET_MS: u32 = 1000;

/// The UI window's timer that watches whether the flag came onto the taskbar while its hint is up.
pub const HINT_TIMER: usize = 32;
const HINT_MS: u32 = 1000;

/// The seconds without a key or the mouse the window «Обновлено» waits for (T-119-6).
pub const QUIET_SECONDS: u32 = 5;

/// At the start of the program (the UI thread, the tray up, before the first tick of the letters,
/// which moves the version seen): the start after an update writes the day of it
/// (`[letters] updated_on`, T-119-7); if the release is important, the window is written as due
/// (`[letters] release_due`); a window due from before — the program stopped before its quiet moment
/// came — stays due; then the UI window waits for the quiet moment, once a second. With «Больше не
/// показывать окна обновлений» no window. Answers the version updated from, when this start is one.
pub fn plan_release_window(owner: HWND) -> Option<String> {
    use windows::Win32::UI::WindowsAndMessaging::SetTimer;

    let state = crate::letters::state_now();
    let build = crate::letters::version_string();
    let updated = crate::letters::was_updated(&state, &build);
    let previous = updated.then(|| state.last_seen_version.clone());

    if updated {
        let today = crate::letters::today();
        crate::letters::update_state(|letters| letters.updated_on = today);
    }

    if !state.release_window {
        return previous;
    }

    let due = if updated && release_is_due(&state.last_seen_version, &build, LAST_IMPORTANT.version)
    {
        let from = state.last_seen_version.clone();
        crate::letters::update_state(move |letters| {
            letters.release_due = from;
        });
        true
    } else {
        !state.release_due.is_empty()
    };

    if due {
        // SAFETY: the live UI window and an identifier of this module; replaced if it exists.
        let _ = unsafe { SetTimer(Some(owner), QUIET_TIMER, QUIET_MS, None) };
    }

    previous
}

/// [`QUIET_TIMER`]: the window «Обновлено» comes when the moment is quiet — five seconds without a
/// key or the mouse, and Windows takes notifications (not «Не беспокоить», not a full-screen
/// program — `letters::moment_is_quiet_for`). Shown, it is no longer due — once, whatever comes of
/// it. Called from the timer's message: the clock of the quiet is that message's.
pub fn quiet_tick(owner: HWND, sound: bool) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;

    let state = crate::letters::state_now();
    let stop = || {
        // SAFETY: the timer of the live UI window.
        let _ = unsafe { KillTimer(Some(owner), QUIET_TIMER) };
    };

    if state.release_due.is_empty() || !state.release_window {
        stop();
        return;
    }

    if is_shown() || !crate::letters::moment_is_quiet_for(QUIET_SECONDS) {
        return;
    }

    if show(release_content(&state.release_due), sound, release_ended) {
        crate::letters::update_state(|letters| letters.release_due.clear());
        stop();
    }
}

/// What the window «Обновлено» says — the block of the release in the language of the interface;
/// `previous` — the version updated from («Было Y»).
pub fn release_content(previous: &str) -> Content {
    release_content_in(crate::settings::ui_language(), previous)
}

/// A string of `language` with `{0}`, `{1}`… filled — `settings::format_text` in a language
/// named (the instrument of fitting goes over all of them).
fn filled(language: crate::settings::Language, id: u16, arguments: &[&str]) -> String {
    let mut filled = crate::settings::text_in(language, id);
    for (index, argument) in arguments.iter().enumerate() {
        filled = filled.replace(&format!("{{{index}}}"), argument);
    }
    filled
}

/// [`release_content`] in `language`.
pub fn release_content_in(language: crate::settings::Language, previous: &str) -> Content {
    use crate::settings;

    let text = |id| settings::text_in(language, id);
    let build = crate::letters::version_string();

    Content {
        about: text(settings::IDS_CORNER_UPDATE),
        title: filled(language, settings::IDS_RELEASE_TITLE, &[&build]),
        text: filled(language, settings::IDS_RELEASE_TEXT, &[previous]),
        link: Some((
            text(settings::IDS_RELEASE_CHANGES),
            crate::letters::links::changelog_url(language),
        )),
        // The main button is the passport's — on the right and wider (§10 (4)): «Как пользоваться»,
        // the action of the window; «Понятно» the second, as in the hint of the flag.
        main: (
            text(settings::IDS_RELEASE_HOWTO),
            Action::Open(crate::letters::links::program_url(language)),
        ),
        second: Some((text(settings::IDS_CORNER_OK), Action::Close)),
        picture: LAST_IMPORTANT.picture,
        check: Some(text(settings::IDS_RELEASE_NEVER)),
    }
}

/// The window «Обновлено» ended: the check box ticked turns the windows of updates off — the same
/// setting as the switch of «От автора» (T-119-10).
fn release_ended(end: End) {
    if end.checked {
        crate::letters::update_state(|letters| letters.release_window = false);
    }
}

// -----------------------------------------------------------------------------------------
// The hint of the flag — task T-119-5 (решения 208.88, 208.89 п. 6, 208.113, 208.141)
// -----------------------------------------------------------------------------------------

/// What the hint says: the flag is on and stands under ^; «Открыть Параметры Windows» opens the
/// page of the taskbar, «Понятно» and ✕ close it. Not a word about dragging the icon — the drag did
/// not move it (208.113 п. 7).
pub fn hint_content() -> Content {
    hint_content_in(crate::settings::ui_language())
}

/// [`hint_content`] in `language`.
pub fn hint_content_in(language: crate::settings::Language) -> Content {
    use crate::settings;

    let text = |id| settings::text_in(language, id);
    Content {
        about: text(settings::IDS_CORNER_FLAG),
        title: text(settings::IDS_HINT_TITLE),
        text: text(settings::IDS_HINT_TEXT),
        link: None,
        main: (
            text(settings::IDS_HINT_OPEN),
            Action::Open(TASKBAR_SETTINGS),
        ),
        second: Some((text(settings::IDS_CORNER_OK), Action::Close)),
        picture: false,
        check: None,
    }
}

/// The page of the taskbar in the settings of Windows.
pub const TASKBAR_SETTINGS: &str = "ms-settings:taskbar";

/// The flag was turned on: once per turning on, and only while it stands under ^, the hint comes
/// up at once (the person is at the settings window); a timer of the UI window takes it away the
/// moment the flag is on the taskbar.
pub fn show_flag_hint(owner: HWND, icon_id: u32) {
    use windows::Win32::UI::WindowsAndMessaging::SetTimer;

    if flag_on_the_bar(owner, icon_id) {
        return;
    }

    if show(hint_content(), false, |_| {}) {
        // SAFETY: the live UI window and an identifier of this module.
        let _ = unsafe { SetTimer(Some(owner), HINT_TIMER, HINT_MS, None) };
    }
}

/// [`HINT_TIMER`]: the hint goes by itself once the flag is on the taskbar; the timer goes with the
/// hint.
pub fn hint_tick(owner: HWND, icon_id: u32) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;

    let hint_up = SHOWN.with(|shown| {
        shown
            .borrow()
            .as_ref()
            .is_some_and(|state| state.main == Action::Open(TASKBAR_SETTINGS))
    });

    if !hint_up {
        // SAFETY: the timer of the live UI window.
        let _ = unsafe { KillTimer(Some(owner), HINT_TIMER) };
        return;
    }

    if flag_on_the_bar(owner, icon_id) {
        end(EndBy::Close);
        // SAFETY: as above.
        let _ = unsafe { KillTimer(Some(owner), HINT_TIMER) };
    }
}

/// Whether the flag stands on the taskbar rather than under ^ (T-119-5). Windows 11: the person's
/// choice `HKCU\Control Panel\NotifyIconSettings\<id>\IsPromoted` of the key whose
/// `ExecutablePath` is this program. Windows 10, which has no such keys: the icon's rectangle is
/// not empty and not inside ^ (the `Button` of `TrayNotifyWnd`).
pub fn flag_on_the_bar(owner: HWND, icon_id: u32) -> bool {
    if let Some(promoted) = promoted_in_settings() {
        return promoted;
    }

    let Some(icon) = icon_rect(owner, icon_id) else {
        return false;
    };

    if icon.right <= icon.left || icon.bottom <= icon.top {
        return false;
    }

    chevron_rect_10().is_none_or(|chevron| {
        !(icon.left >= chevron.left
            && icon.right <= chevron.right
            && icon.top >= chevron.top
            && icon.bottom <= chevron.bottom)
    })
}

/// The rectangle of an icon of this program's window (`Shell_NotifyIconGetRect`).
fn icon_rect(owner: HWND, icon_id: u32) -> Option<RECT> {
    use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};

    let icon = NOTIFYICONIDENTIFIER {
        cbSize: u32::try_from(size_of::<NOTIFYICONIDENTIFIER>()).unwrap_or(0),
        hWnd: owner,
        uID: icon_id,
        ..Default::default()
    };

    // SAFETY: a live local whose `cbSize` describes it; the answer is a value.
    unsafe { Shell_NotifyIconGetRect(&icon) }.ok()
}

/// Windows 10: the ^ of the tray — the `Button` child of `TrayNotifyWnd`.
fn chevron_rect_10() -> Option<RECT> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
    use windows::core::w;

    // SAFETY: constant class names; null answers are checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: as above.
    let notify = unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.ok()?;
    // SAFETY: as above.
    let button = unsafe { FindWindowExW(Some(notify), None, w!("Button"), None) }.ok()?;
    let mut rect = RECT::default();

    // SAFETY: a window handle by value and a local rectangle.
    unsafe { GetWindowRect(button, &mut rect) }.ok()?;
    Some(rect)
}

/// Windows 11: `IsPromoted` of this program's key under `HKCU\Control Panel\NotifyIconSettings`;
/// `None` when the keys are not there (Windows 10) or none is this program's.
fn promoted_in_settings() -> Option<bool> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey,
        RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    };
    use windows::core::{PCWSTR, PWSTR, w};

    let me = std::env::current_exe().ok()?;
    let me = me.to_string_lossy().to_lowercase();
    let mut key = HKEY::default();

    // SAFETY: a constant NUL-terminated name and a live local; the key is closed below.
    if unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Control Panel\\NotifyIconSettings"),
            None,
            KEY_READ,
            &mut key,
        )
    }
    .is_err()
    {
        return None;
    }

    let mut found = None;

    for index in 0..512u32 {
        let mut name = [0u16; 64];
        let mut length = u32::try_from(name.len()).unwrap_or(0);

        // SAFETY: the open key; a live local of `length` units.
        let status = unsafe {
            RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut length,
                None,
                None,
                None,
                None,
            )
        };

        if status.is_err() {
            break;
        }

        let sub: Vec<u16> = name[..usize::try_from(length).unwrap_or(0)]
            .iter()
            .copied()
            .chain(Some(0))
            .collect();
        let mut path = [0u16; 520];
        let mut bytes = u32::try_from(size_of_val(&path)).unwrap_or(0);

        // SAFETY: the open key, NUL-terminated names, a live buffer of `bytes` bytes the call does
        // not write past.
        let read = unsafe {
            RegGetValueW(
                key,
                PCWSTR(sub.as_ptr()),
                w!("ExecutablePath"),
                RRF_RT_REG_SZ,
                None,
                Some(path.as_mut_ptr().cast()),
                Some(&mut bytes),
            )
        };

        if read.is_err() {
            continue;
        }

        let units = usize::try_from(bytes).unwrap_or(0) / 2;
        let text = String::from_utf16_lossy(&path[..units.min(path.len())]);

        if text.trim_end_matches('\0').to_lowercase() != me {
            continue;
        }

        let mut promoted = 0u32;
        let mut size = u32::try_from(size_of::<u32>()).unwrap_or(4);

        // SAFETY: as above, a DWORD into a local of its size.
        let read = unsafe {
            RegGetValueW(
                key,
                PCWSTR(sub.as_ptr()),
                w!("IsPromoted"),
                RRF_RT_REG_DWORD,
                None,
                Some((&raw mut promoted).cast()),
                Some(&mut size),
            )
        };

        found = Some(read.is_ok() && promoted == 1);
        break;
    }

    // SAFETY: closes exactly the key opened above, once.
    let _ = unsafe { RegCloseKey(key) };

    found
}

// -----------------------------------------------------------------------------------------
// The entry «Что нового в X…» — task T-119-7 (решения 208.93 п. 5, 208.94 п. 5)
// -----------------------------------------------------------------------------------------

/// The days the entry stands after an update.
pub const WHATS_NEW_DAYS: i64 = 7;

/// Whether the entry «Что нового в X…» stands in the menu today — a week from the day of the last
/// update, **any** update, important or not. Pure.
pub fn whats_new_stands(
    updated_on: Option<crate::letters::Date>,
    today: Option<crate::letters::Date>,
) -> bool {
    match (updated_on, today) {
        (Some(updated), Some(today)) => {
            today >= updated && today < updated.plus_days(WHATS_NEW_DAYS)
        }
        _ => false,
    }
}

/// The entry was chosen: the window «Обновлено» once more if it has not been seen — it was due and
/// its quiet moment has not come — else the page of the changes on the site.
pub fn whats_new() {
    let state = crate::letters::state_now();

    if !state.release_due.is_empty()
        && !is_shown()
        && show(release_content(&state.release_due), false, release_ended)
    {
        crate::letters::update_state(|letters| letters.release_due.clear());
        return;
    }

    let _ = crate::settings::open_in_the_shell(crate::letters::links::changelog_url(
        crate::settings::ui_language(),
    ));
}
