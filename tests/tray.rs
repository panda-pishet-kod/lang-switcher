//! Integration tests for the `tray` module, task T-01-4.
//!
//! The menu is inspected in two halves since task T-11-10 put its entries on
//! `MF_OWNERDRAW`. What Windows still knows is read back out of Windows with
//! `GetMenuItemCount`, `GetMenuState`, `GetMenuItemID` and `GetMenuItemInfoW`, exactly as
//! an outside observer would: the entry count, the rules, the owner-draw flag, the check
//! mark, and the `itemData` that SEC-05 requires to be a command number and never a
//! pointer. The labels are the half Windows no longer holds — an owner-drawn entry has no
//! string inside the menu — so they are read from the builder's own record,
//! [`Menu::items`], which is the very structure the drawing paints from. Both halves are
//! compared against literals written out here rather than against the crate's own
//! constants: a test that imported `tray::LABEL_EXIT` and then asserted that the menu
//! contains `tray::LABEL_EXIT` would pass whatever the label said.
//!
//! Two of the tests put a real icon in the notification area for a few milliseconds and
//! take it away again — there is no way to check `Shell_NotifyIcon` without calling it.
//! Nothing here writes into the real `%APPDATA%\Lang_Switcher`: every tray is installed
//! with [`Tray::install_at`] pointing at a directory under `%TEMP%` that removes itself.

use std::cell::Cell;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use lang_switcher::buffer::{self, Recorder, Stroke};
use lang_switcher::diag;
use lang_switcher::hook::{self, Edge, KeyEvent};
use lang_switcher::settings::{self, CONFIG_FILE_NAME, DialogSession, Language};
// Task T-14-3: the drawing library the menu paints with moved out of `settings` into its
// owner, `theme` (§6.2) — the whole point being that the tray no longer goes to the
// configuration module for pixels.
use lang_switcher::theme::{self, ThemeSetting};
use lang_switcher::tray::{self, Attachment, Menu, Pending, Reaction, Tray};
use lang_switcher::watchdog::{self, WM_APP_WIPE};

use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, FreeLibrary, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{CLEARTYPE_QUALITY, LOGFONTW};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, LoadResource,
    LockResource, SizeofResource,
};
use windows::Win32::UI::Controls::{MEASUREITEMSTRUCT, ODT_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetMenuItemCount, GetMenuItemID, GetMenuItemInfoW,
    GetMenuState, GetSystemMetrics, HMENU, MENU_ITEM_FLAGS, MENUITEMINFOW, MF_BYPOSITION,
    MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_OWNERDRAW, MF_SEPARATOR, MIIM_DATA, NONCLIENTMETRICSW,
    RT_DIALOG, RT_VERSION, SM_CXICON, SM_CXMENUCHECK, SM_CXSMICON, SM_CYICON, SM_CYSMICON,
    SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    WM_DRAWITEM, WM_ENDSESSION, WM_MEASUREITEM, WM_QUERYENDSESSION, WM_SETTINGCHANGE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{Error as WinError, PCWSTR, w};

// ---------------------------------------------------------------------------------------
// FR-91, read back from Windows
// ---------------------------------------------------------------------------------------

/// The third argument of [`Menu::build`] in the ordinary state of the program — task T-13-9.
///
/// `hook::fail_safe()`, which is what the product hands in from `tray::show_menu`.
/// It is false in every test but the ones that are about FR-99, and it is a name rather than a
/// bare `false` so that a reader of any of the menu tests below can see which of the three
/// booleans is which.
const NOT_FAIL_SAFE: bool = false;

/// The same argument in the state FR-99 leaves behind — the fourth consecutive panic inside
/// the hook callback has disarmed buffering and the flag is up for the rest of the process.
const FAIL_SAFE: bool = true;

/// The fourth argument of [`Menu::build`] in the ordinary state of the program — task T-13-14.
///
/// `settings::dialog_is_open()`, which is what the product hands in from `tray::show_menu`.
/// False in every test but the ones about the modal dialog of FR-92, and a name rather than a
/// bare `false` for the same reason its neighbour above is one — four booleans in a row is
/// three too many to tell apart by position.
const NO_DIALOG: bool = false;

/// The fifth argument of [`Menu::build`] in the ordinary state of the program — task Т-32-4,
/// FR-101.
///
/// What the letters of the author have to say in the menu, which for a program with no feed
/// read behind it is nothing at all: no unread news and no newer version. The menu then has
/// exactly the [`MENU_ENTRY_COUNT`] entries of FR-91, and the tests below are about those.
const NOTHING_PENDING: Pending = Pending {
    unread: false,
    update: None,
};

/// The same argument while the modal settings dialog of FR-92 is on the screen.
const DIALOG_UP: bool = true;

/// The menu of FR-91 as its block prints, top to bottom. `None` is one of the two rules.
///
/// Written out here on purpose. These seven lines are the requirement; the module's own
/// resources are the implementation, and a test must not check one against itself.
const FR_91: [Option<&str>; 8] = [
    Some("Приостановить"),
    None,
    Some("Настройки…"),
    Some("Запускать при входе в систему"),
    None,
    // FR-91's addition, task Т-32-4, решение 101 п. 9 — the permanent entry stands **above**
    // «О программе», which is where the accepted mock-up puts it.
    Some("Написать автору…"),
    Some("О программе"),
    Some("Выход"),
];

/// The same seven lines with the program suspended — the first line of FR-91 is a pair and
/// this is its other word.
const FR_91_SUSPENDED: [Option<&str>; 8] = [
    Some("Возобновить"),
    None,
    Some("Настройки…"),
    Some("Запускать при входе в систему"),
    None,
    Some("Написать автору…"),
    Some("О программе"),
    Some("Выход"),
];

/// The same seven lines in the other locale of FR-94 — task T-08-2.
///
/// ⚠ The **composition** is identical and only the words move: five commands and the same two
/// rules in the same places. A translation that quietly dropped or added an entry would be a
/// different menu, and FR-91 fixes the menu.
const FR_91_ENGLISH: [Option<&str>; 8] = [
    Some("Suspend"),
    None,
    Some("Settings…"),
    Some("Start when I sign in"),
    None,
    Some("Write to the author…"),
    Some("About"),
    Some("Exit"),
];

/// Serialises the tests that publish an interface locale — it is process-wide, and the tests of
/// one binary run on parallel threads.
///
/// ⚠ **Since task Т-31-1 the family is larger than it looks.** «Применить» now publishes the
/// locale too ([`tray::adopt_ui_language`]), so every test that calls `tray::apply_settings_via`
/// belongs here as well — including the four of task T-13-24, whose configurations carry
/// `language = "en"` and would otherwise move the locale under a neighbour reading a menu.
static LOCALE: Mutex<()> = Mutex::new(());

/// Takes the turn of [`LOCALE`], ignoring poisoning — the same medicine, and for the same
/// reason, as [`toggle_turn`] below.
///
/// Separate from [`product_strings`] because the two askings differ: a test reading labels
/// needs the string tables of the built binary *and* a published locale, while a test that
/// merely applies a configuration needs only not to collide with it.
fn locale_turn() -> MutexGuard<'static, ()> {
    LOCALE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Serialises every test that moves the state through [`Tray::toggle_state`] — task **Т-13-7**.
///
/// ⚠ **Since that task the method has a process-wide side effect**: suspending the program asks
/// the input thread for the wipe of row 9 of the FR-10 table, and the ask is counted in
/// `watchdog::counters().wipe_requests`. `the_pause_from_the_tray_asks_for_the_wipe_of_row_nine`
/// reads that counter as a delta, and the tests of one binary run on parallel threads — so a
/// neighbour toggling at the same moment would be adding to the number this one is measuring.
/// The same medicine `LOCALE` above already is, for the same disease.
///
/// Held by every test that calls `toggle_state`, whether or not it cares about the counter: a
/// lock only one side takes is not a lock.
static TOGGLE: Mutex<()> = Mutex::new(());

/// Takes the turn of [`TOGGLE`], ignoring poisoning: a panic in one test must fail that test and
/// not turn its neighbour into a second failure with an unrelated message.
fn toggle_turn() -> MutexGuard<'static, ()> {
    TOGGLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The mapping [`product_strings`] hands out, loaded once and never unmapped.
static SHARED_IMAGE: OnceLock<usize> = OnceLock::new();

/// Points `settings` at the string tables of the built binary and publishes `language`.
///
/// ⚠ Without the redirection every label would come back empty: `embed-resource` links `app.rc`
/// into the **binary** targets of the crate, so this test executable has no resource section of
/// its own — section 4.4 of STATE.md, the same barrier the icons above are loaded around. What
/// the menu is checked against is therefore the table that ships.
fn product_strings(language: settings::Language) -> MutexGuard<'static, ()> {
    let guard = locale_turn();

    let raw = *SHARED_IMAGE.get_or_init(|| ProductImage::load().0 as usize);

    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
    settings::set_ui_language(language);

    guard
}

/// Checks one built menu against one seven-line block of FR-91.
///
/// Since task T-11-10 the entries are `MF_OWNERDRAW`, so the check is in two halves. Out
/// of Windows, as an outside observer would read them: the entry count, which positions
/// are rules, that every entry — rules included since task T-11-22 — carries the owner-draw
/// flag, and what its `itemData` is (SEC-05 — a number, not a pointer): its own command
/// identifier for a command, and for a rule one number that is none of the commands, the same
/// for both rules, over a command identifier of zero. Out of the builder's record, because
/// Windows no longer holds a string for an owner-drawn entry: the labels, in order, against the
/// literals of the block.
fn assert_menu_is(menu: &Menu, block: &[Option<&str>; 8]) {
    assert_eq!(
        block.len(),
        usize::try_from(tray::MENU_ENTRY_COUNT).expect("the count fits in a usize"),
        "FR-91 lists six commands and two rules, which is eight entries — «Написать автору…» \
         arrived with task Т-32-4"
    );
    assert_eq!(
        item_count(menu.handle()),
        tray::MENU_ENTRY_COUNT,
        "and the menu Windows holds has exactly those eight"
    );

    let commands: Vec<usize> = menu
        .items()
        .iter()
        .map(|item| usize::try_from(item.command).expect("a command fits in a usize"))
        .collect();
    let mut rules: Vec<usize> = Vec::new();

    let mut items = menu.items().iter();

    for (position, expected) in block.iter().enumerate() {
        let index = u32::try_from(position).expect("a menu position fits in a u32");
        let separator = is_separator(menu.handle(), index);

        match expected {
            Some(text) => {
                let item = items
                    .next()
                    .expect("the builder must have recorded every command entry");
                let data = item_data_at(menu.handle(), index);

                println!(
                    "{position}: command={:#06x} itemData={data:#x} label={:?}",
                    item.command, item.label
                );

                assert!(
                    !separator,
                    "entry {position} of FR-91 is a command, not a rule"
                );
                assert!(
                    is_owner_drawn(menu.handle(), index),
                    "entry {position} must be owner-drawn — FR-92а"
                );
                assert_eq!(
                    item.label, *text,
                    "entry {position} of FR-91 reads differently"
                );
                assert_eq!(
                    data,
                    usize::try_from(item.command).expect("a command fits in a usize"),
                    "SEC-05: itemData of entry {position} must be its command number"
                );
            }
            None => {
                let data = item_data_at(menu.handle(), index);
                let identifier = command_at(menu.handle(), index);

                println!("{position}: separator={separator} itemData={data:#x} id={identifier:#x}");

                assert!(separator, "entry {position} of FR-91 is a rule");

                // **Task T-11-22 rewrote the two assertions below.** Until it, a rule was a
                // plain `MF_SEPARATOR` and this said «a rule stays system-drawn — the accepted
                // cost of section 10»; that cost is paid off, and the rule is now drawn by the
                // program like every other entry. It stays a rule for everybody outside it:
                // `MF_SEPARATOR` is still set, which is what the line above reads.
                assert!(
                    is_owner_drawn(menu.handle(), index),
                    "entry {position} is a rule this program draws itself — FR-92а"
                );
                assert_eq!(
                    identifier, 0,
                    "a rule carries no command identifier: `TrackPopupMenuEx` must never \
                     be able to return one"
                );
                assert_ne!(
                    data, 0,
                    "and it does carry an `itemData` — the drawing finds it by"
                );
                assert!(
                    !commands.contains(&data),
                    "SEC-05: the number a rule carries is none of the five commands"
                );

                rules.push(data);
            }
        }
    }

    assert!(
        items.next().is_none(),
        "the builder recorded exactly the five commands of FR-91"
    );

    assert_eq!(rules.len(), 2, "FR-91 has two rules");
    assert_eq!(
        rules[0], rules[1],
        "both rules carry the same number: they are the same kind of entry"
    );
}

#[test]
fn the_menu_is_the_block_of_fr_91_entry_for_entry() {
    let _locale = product_strings(settings::Language::Ru);
    let menu = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu of FR-91 must be creatable");

    assert_menu_is(&menu, &FR_91);
}

#[test]
fn the_menu_of_fr_91_is_the_same_menu_in_english() {
    // **Criterion 12 of T-08-2.** FR-94 translates the labels; FR-91 fixes what the menu
    // *is*, and the second must survive the first — same seven entries, same two rules,
    // same places.
    let _locale = product_strings(settings::Language::En);
    let menu = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu of FR-91 must be creatable");

    assert_menu_is(&menu, &FR_91_ENGLISH);

    // The suspended state moves the same entry in this locale as in the other one.
    let suspended = Menu::build(false, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");
    assert_eq!(suspended.items()[0].label, "Resume");

    settings::set_ui_language(settings::Language::Ru);
}

#[test]
fn the_first_entry_follows_the_state() {
    let _locale = product_strings(settings::Language::Ru);
    let active = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");
    let suspended = Menu::build(false, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    assert_eq!(active.items()[0].label, "Приостановить");
    assert_eq!(suspended.items()[0].label, "Возобновить");

    // Only the first entry moves. Everything below it — labels, commands and check marks
    // alike — is the same menu.
    assert_eq!(
        active.items()[1..],
        suspended.items()[1..],
        "no entry below the first may depend on the state"
    );
}

// ---------------------------------------------------------------------------------------
// Task T-13-9, point (б) — FR-99 refuses the resumption of FR-90
//
// The audit of 2026-08-24 found `FAIL_SAFE` raised once and reset nowhere, so a user who
// chose «Возобновить» after the fail-safe got `enabled = true`, the "активна" icon and
// `set_active(true)` over processing that was dead until the process restarted — against the
// signalling FR-99 names («сигнализирует состояние иконкой в трее») and the two
// distinguishable states of FR-90.
//
// ⚠ **What these tests can and cannot reach.** `hook::fail_safe()` has exactly one writer in
// the program — the fourth consecutive panic inside the hook callback, which only the system
// can call — so no test binary can raise it. That is why the rule is
// `tray::resume_is_refused(enabled, fail_safe)`, a function of its two arguments in the shape
// `hook::classify` already has and for the reason that module states: every row of it is
// reachable from here. The menu is measured the same way, with the flag handed in.
// ---------------------------------------------------------------------------------------

/// **The rule itself, all four rows.** One direction and one state, and nothing else.
#[test]
fn the_resumption_is_refused_in_exactly_one_of_the_four_states() {
    for (enabled, fail_safe, expected) in [
        // The ordinary program, armed or suspended: both directions are the user's.
        (true, NOT_FAIL_SAFE, false),
        (false, NOT_FAIL_SAFE, false),
        // FR-99 has disarmed the program and it is still showing "активна" — the transition
        // itself is a `toggle_state`, sent after the flag is already up, and refusing it
        // would leave the icon lying from the other side.
        (true, FAIL_SAFE, false),
        // The state the audit is about: disarmed, the icon honest, and the resumption asked
        // for. This is the one that is refused.
        (false, FAIL_SAFE, true),
    ] {
        let refused = tray::resume_is_refused(enabled, fail_safe);

        println!("enabled={enabled} fail_safe={fail_safe} -> refused={refused}");

        assert_eq!(
            refused, expected,
            "resume_is_refused({enabled}, {fail_safe})"
        );
    }
}

/// **The refusal has a name of its own in the journal, and it is not `UNLISTED`.**
///
/// `Operation::from_name` is a *narrowing*: a name that is not a row of the closed table of
/// `src\diag.rs` becomes `Operation::UNLISTED` and the text is dropped on the floor. That is
/// what keeps a caller from putting anything of the user's into the ring — and it is also why a
/// row that was never added would leave this event nameless and indistinguishable from every
/// other unlisted one. So the round trip is asserted here rather than taken on trust.
///
/// SEC-01, SEC-07: what is asserted is that the row exists and which group it belongs to. The
/// name itself carries no value of any kind — not the state of the configuration, not the count
/// of panics, nothing typed — and there is no field beside it that could.
#[test]
fn the_refused_resumption_has_a_row_of_its_own_in_the_vocabulary_of_diag() {
    let operation = diag::Operation::from_name("resume refused in fail-safe");

    println!(
        "«resume refused in fail-safe» -> name={:?} kind={:?} ({})",
        operation.name(),
        operation.kind(),
        operation.kind().name()
    );

    assert_ne!(
        operation,
        diag::Operation::UNLISTED,
        "the row of task T-13-9 is missing from `src\\diag.rs`: the event would reach the ring \
         nameless and keep none of its text"
    );
    assert_eq!(
        operation.name(),
        "resume refused in fail-safe",
        "and it maps back to its own row rather than to somebody else's"
    );
    assert_eq!(
        operation.kind(),
        diag::Kind::Tray,
        "the icon and the menu of FR-90 and FR-91 are the tray's"
    );

    // The unlisted answer is what this would be without the row — kept beside it so that the
    // assertion above cannot quietly become vacuous.
    let absent = diag::Operation::from_name("resume refused in fail-safe (not a row)");

    println!("a name that is not a row -> {:?}", absent.name());

    assert_eq!(absent, diag::Operation::UNLISTED);
    assert_eq!(absent.name(), "(unlisted)");
}

/// **The rule is where `general.enabled` moves, and it is the first thing there.**
///
/// Swept over the source, and for the reason two other tests of this suite already sweep it
/// (`the_ink_of_a_menu_entry_does_not_depend_on_the_cursor` here, and
/// `the_capslock_seed_is_outside_the_callback` in `tests\hook.rs`): the live path cannot be
/// entered from a test binary. `Tray::toggle_state` consults `hook::fail_safe()`, whose one
/// writer is the fourth consecutive panic inside a callback the system alone calls, so the
/// refusing branch cannot be *run* from here however the tray is driven. What can be checked
/// is that the guard is in the one method that writes the field, that it stands before the
/// write rather than after it, and that it is the shared rule and not a second copy of it.
#[test]
fn the_guard_of_fr99_stands_at_the_top_of_the_method_that_moves_the_state() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    // The canonical checkout of this repository is CRLF (`.gitattributes`: `eol=crlf`), so a
    // needle written with `\n` has to be matched against a text that holds `\n`. The reason is
    // written out at `the_capslock_seed_is_outside_the_callback` in `tests\hook.rs`, which was
    // red on every fresh worktree until task T-13-9 for exactly this.
    .replace("\r\n", "\n");

    let at = source
        .find("pub fn toggle_state(&mut self) {")
        .expect("Tray::toggle_state must be in this file");
    let body = &source[at..];
    let end = body
        .find("\n    }")
        .expect("a method closes with its brace");
    let body = &body[..end];

    println!("--- Tray::toggle_state ---\n{body}");

    let guard = body
        .find("resume_is_refused")
        .expect("toggle_state must consult the rule of task T-13-9");
    let write = body
        .find("self.config.general.enabled =")
        .expect("toggle_state must be the place `general.enabled` moves");

    assert!(
        guard < write,
        "the refusal must come before the change, not after it"
    );
    assert!(
        body.contains("crate::hook::fail_safe()"),
        "and it must be asked of the flag FR-99 raises, not of anything the caller passes"
    );

    // One rule, one definition. `Menu::build` reads the same function; a second copy of the
    // condition is how two answers to the same question start to differ.
    let definitions = source.matches("pub fn resume_is_refused(").count();

    assert_eq!(definitions, 1, "the rule is defined exactly once");

    // The refusal is not a silence: one event goes into the ring, inside the refusing branch
    // and therefore before the `return`, and it is the row the test above proves exists.
    let event = body
        .find("diag::record(")
        .expect("the refusal must leave an event in the journal");
    let name = body
        .find("RESUME_REFUSED_IN_FAIL_SAFE")
        .expect("and it must be the named row, not a string built here");

    assert!(
        guard < event && event < write,
        "the event belongs inside the refusing branch — after the rule and before the change \
         the branch never reaches"
    );
    assert!(
        event < name,
        "the name is the argument of that call and not a mention somewhere else"
    );

    // The literal behind that name is the one `src\diag.rs` carries, written out here so that
    // the two cannot drift apart without this failing.
    assert!(
        source
            .contains("const RESUME_REFUSED_IN_FAIL_SAFE: &str = \"resume refused in fail-safe\";"),
        "the journal name of the refusal must be the row of the closed table"
    );

    // SEC-01, SEC-07: the event carries no code of its own. `OsCode` has no constructor that
    // takes a number, and this is an event of the program rather than a failed Win32 call.
    assert!(
        body.contains("diag::OsCode::NONE"),
        "the refusal reports no error code"
    );
}

/// **Criterion 7 of task T-13-9.** With the flag up, «Возобновить» is in the menu, greyed, and
/// cannot be chosen — so the icon cannot go back to "активна" through the menu at all.
///
/// `MF_DISABLED` is the half that decides: `TrackPopupMenuEx` answers zero for a disabled
/// entry instead of returning `CMD_TOGGLE`, so `dispatch_command` never runs and
/// `Tray::toggle_state` — which refuses the same move a second time — is never even reached.
/// `MF_GRAYED` is the half the user sees, and the drawing of FR-92а pairs it with the
/// palette's `text_muted`.
#[test]
fn a_disarmed_program_greys_the_resumption_and_cannot_be_asked_for_it() {
    let _locale = product_strings(settings::Language::Ru);
    let menu = Menu::build(false, true, FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    println!(
        "entry 0: label={:?} enabled={} grayed={} disabled={}",
        menu.items()[0].label,
        menu.items()[0].enabled,
        is_grayed(menu.handle(), 0),
        is_disabled(menu.handle(), 0)
    );

    assert_eq!(
        menu.items()[0].label,
        "Возобновить",
        "the entry stays in the menu and stays readable — a line that vanished would say \
         nothing at all"
    );
    assert!(is_grayed(menu.handle(), 0), "and it is drawn unavailable");
    assert!(
        is_disabled(menu.handle(), 0),
        "and `TrackPopupMenuEx` can never return CMD_TOGGLE for it"
    );
    assert!(
        !menu.items()[0].enabled,
        "the builder's own record, which is what the drawing picks its ink by"
    );

    // Nothing below the first entry moves: FR-99 takes away the resumption and not the
    // settings, the autostart, the about box or the way out.
    for position in [2u32, 3, 5, 6] {
        assert!(
            !is_grayed(menu.handle(), position) && !is_disabled(menu.handle(), position),
            "entry {position} of FR-91 must stay available"
        );
    }

    for item in &menu.items()[1..] {
        assert!(item.enabled, "{:?} must stay available", item.label);
    }

    // And the block of FR-91 is still the block of FR-91 — seven entries, five commands, two
    // rules, the same words. Greying is not a second menu.
    assert_menu_is(&menu, &FR_91_SUSPENDED);
}

/// The other two states leave the first entry exactly as it was before this task.
#[test]
fn nothing_is_greyed_while_fr99_is_not_holding_and_nothing_when_it_suspends() {
    let _locale = product_strings(settings::Language::Ru);

    // The ordinary suspended program: «Возобновить» is the user's to choose.
    let ordinary = Menu::build(false, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    assert_eq!(ordinary.items()[0].label, "Возобновить");
    assert!(ordinary.items()[0].enabled);
    assert!(!is_grayed(ordinary.handle(), 0));
    assert!(!is_disabled(ordinary.handle(), 0));

    // FR-99 has just disarmed an armed program. The move it is about to make is the
    // suspension, and that one must stay available or the icon never becomes honest.
    let suspending = Menu::build(true, true, FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    assert_eq!(suspending.items()[0].label, "Приостановить");
    assert!(suspending.items()[0].enabled);
    assert!(!is_grayed(suspending.handle(), 0));
    assert!(!is_disabled(suspending.handle(), 0));
}

// ---------------------------------------------------------------------------------------
// Task T-13-14 — the tray against the open dialog
//
// The audit of 2026-08-24 found the menu of FR-91 fully alive while the modal dialog of
// FR-92 is on the screen: `settings::show_dialog` guards only against a second copy of the
// window, so «Приостановить» and «Запускать при входе в систему» went on editing the live
// configuration while the dialog was editing a copy of it — and «Применить» put the copy
// back whole, silently undoing them. Section 6.3 gives the configuration one owner.
//
// The answer has two halves and both are measured here: `Menu::build` greys the two entries
// (read back out of Windows with `GetMenuState`, not out of the builder's record), and
// `tray::dispatch_command` refuses the two commands (measured by the value in memory and by
// the bytes in the folder, not by an intention read out of the source).
//
// ⚠ **The `%APPDATA%` rule of this file holds here too, and one more beside it.** Every tray
// below is attached with `tray::attach_at` on a `TestDir` under `%TEMP%`. And **no test here
// ever dispatches `CMD_AUTOSTART` with the gate down**: that command reaches
// `settings::set_autostart`, which writes — or deletes — the real
// `HKCU\…\CurrentVersion\Run` value of whoever is running the tests (FR-93). The refused
// direction is safe by construction, because the whole point is that nothing happens; the
// allowed direction is checked on the entry and on the rule, never by firing it.
// ---------------------------------------------------------------------------------------

/// The menu positions of the two entries the open dialog takes away, and of the three it
/// leaves alone — the FR-91 block read top to bottom, rules included.
const LOCKED_POSITIONS: [u32; 2] = [0, 3];

/// «Настройки…», «О программе», «Выход».
const LIVE_POSITIONS: [u32; 3] = [2, 5, 6];

/// **Criterion 6 of task T-13-14 — the view.** Built with the dialog up, the menu carries
/// `MF_GRAYED` on exactly two entries and on no other.
///
/// Read with `GetMenuState`, which is what Windows itself holds and what accessibility reads,
/// rather than trusting the code that built the menu. The builder's own record is checked
/// beside it and not instead of it: the drawing of task T-11-10 picks its ink by that field,
/// so a greyed entry that the record called available would come up in the wrong colour.
#[test]
fn an_open_dialog_greys_the_two_entries_that_edit_the_configuration() {
    let _locale = product_strings(settings::Language::Ru);
    let menu = Menu::build(true, true, NOT_FAIL_SAFE, DIALOG_UP, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    for position in LOCKED_POSITIONS {
        println!(
            "entry {position}: grayed={} disabled={}",
            is_grayed(menu.handle(), position),
            is_disabled(menu.handle(), position)
        );

        assert!(
            is_grayed(menu.handle(), position),
            "entry {position} of FR-91 edits the configuration and must be drawn unavailable \
             while the dialog of FR-92 is editing it too"
        );
        assert!(
            is_disabled(menu.handle(), position),
            "and `TrackPopupMenuEx` must never return its command"
        );
    }

    for position in LIVE_POSITIONS {
        println!(
            "entry {position}: grayed={} disabled={}",
            is_grayed(menu.handle(), position),
            is_disabled(menu.handle(), position)
        );

        assert!(
            !is_grayed(menu.handle(), position) && !is_disabled(menu.handle(), position),
            "entry {position} of FR-91 changes no configuration and must stay available — \
             «Настройки…» leads into the window that is already open, and a program that \
             could not be closed while a window is up would be worse than the finding"
        );
    }

    // The builder's record, which is what the drawing picks its ink by. The six commands in
    // menu order: toggle, settings, autostart, write, about, exit — «Написать автору…» arrived
    // with task Т-32-4 and changes no configuration, so it stays available like the three
    // after it.
    let record: Vec<bool> = menu.items().iter().map(|item| item.enabled).collect();

    println!("builder's record of availability: {record:?}");

    assert_eq!(
        record,
        vec![false, true, false, true, true, true],
        "the record and Windows must say the same thing about every entry"
    );

    // And the block of FR-91 is still the block of FR-91. Greying is not a second menu — the
    // same seven entries, the same five commands, the same two rules, the same words.
    assert_menu_is(&menu, &FR_91);
}

/// The other side of the same coin, and the two rules composing rather than arguing.
///
/// With the dialog closed nothing this task added is greyed at all, and the entry FR-99 takes
/// away is still taken away — task T-13-9's rule and this one are read with an `&&`, so the
/// first entry needs **both** to be silent to be available.
#[test]
fn a_closed_dialog_greys_nothing_and_the_two_rules_add_up() {
    let _locale = product_strings(settings::Language::Ru);

    let ordinary = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    for position in LOCKED_POSITIONS.iter().chain(LIVE_POSITIONS.iter()) {
        assert!(
            !is_grayed(ordinary.handle(), *position) && !is_disabled(ordinary.handle(), *position),
            "with no dialog on the screen every entry of FR-91 is the user's to choose"
        );
    }

    // Both rules at once: FR-99 has disarmed a suspended program **and** the dialog is up.
    let both = Menu::build(false, true, FAIL_SAFE, DIALOG_UP, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    println!(
        "fail-safe and dialog together: entry 0 grayed={} entry 3 grayed={}",
        is_grayed(both.handle(), 0),
        is_grayed(both.handle(), 3)
    );

    for position in LOCKED_POSITIONS {
        assert!(
            is_grayed(both.handle(), position) && is_disabled(both.handle(), position),
            "two reasons to take an entry away are not two answers: entry {position} is gone"
        );
    }

    // The rule itself, driven directly on all four rows of its table.
    for (command, dialog_open, expected) in [
        (tray::CMD_TOGGLE, true, true),
        (tray::CMD_AUTOSTART, true, true),
        (tray::CMD_SETTINGS, true, false),
        (tray::CMD_ABOUT, true, false),
        (tray::CMD_EXIT, true, false),
        (tray::CMD_TOGGLE, false, false),
        (tray::CMD_AUTOSTART, false, false),
    ] {
        let locked = tray::dialog_locks_command(command, dialog_open);

        println!("dialog_locks_command({command:#06x}, {dialog_open}) -> {locked}");

        assert_eq!(
            locked, expected,
            "dialog_locks_command({command:#06x}, {dialog_open})"
        );
    }
}

/// **Criterion 5 of task T-13-14 — the ban.** With the flag up, the two commands change
/// nothing in memory and write nothing to the disk.
///
/// Measured the way the finding was: the value the tray holds, the names in the folder, and —
/// for the autostart half — the `Run` value the registry holds. `Tray::install_at` reads and
/// never writes, so `config.toml` does not exist when the commands arrive and a save of any
/// kind would make it appear. The `dispatch_command` at the end is the positive control: it
/// shows that a write really would have been seen.
#[test]
fn an_open_dialog_takes_the_two_configuration_commands_away_from_the_handler() {
    let window = TestWindow::new();
    let home = TestDir::new("dialog-gate");
    let _attached = attach_ui(&window, &home);

    let enabled_before = live(Tray::enabled);
    let autostart_before = live(Tray::autostart);
    // Read, never written — FR-93. This is the third place a `CMD_AUTOSTART` would land, and
    // it is the one that does not belong to this test suite at all.
    let registry_before = settings::autostart_value();

    assert!(
        enabled_before,
        "section 7 has `general.enabled` default to true"
    );
    assert_eq!(
        home.entries(),
        Vec::<String>::new(),
        "attaching the tray reads the configuration and writes nothing"
    );
    assert!(
        !settings::dialog_is_open(),
        "and this thread has no settings dialog yet"
    );

    let dialog = DialogSession::open();

    assert!(
        settings::dialog_is_open(),
        "the guard `show_dialog` has always used is what raises the flag"
    );

    for _ in 0..50 {
        tray::dispatch_command(window.handle, tray::CMD_TOGGLE);
        tray::dispatch_command(window.handle, tray::CMD_AUTOSTART);
    }

    println!(
        "after 50 of each command with the dialog up: entries={:?} enabled={} autostart={} \
         registry={:?}",
        home.entries(),
        live(Tray::enabled),
        live(Tray::autostart),
        settings::autostart_value()
    );

    assert_eq!(
        home.entries(),
        Vec::<String>::new(),
        "a refused command writes no file — not the configuration, not a quarantine copy, \
         not a temporary of an unfinished write"
    );
    assert_eq!(
        live(Tray::enabled),
        enabled_before,
        "and moves nothing in memory: `general.enabled` is where it was"
    );
    assert_eq!(
        live(Tray::autostart),
        autostart_before,
        "nor `general.autostart`"
    );
    assert_eq!(
        settings::autostart_value(),
        registry_before,
        "FR-93: and the `Run` key of the person running this test is untouched"
    );

    // The positive control, on the one command that is safe to fire: the same folder, watched
    // the same way, does see a write once the dialog is gone.
    drop(dialog);

    tray::dispatch_command(window.handle, tray::CMD_TOGGLE);

    println!(
        "after one command with the dialog closed: entries={:?} enabled={}",
        home.entries(),
        live(Tray::enabled)
    );

    assert_eq!(home.entries(), vec![CONFIG_FILE_NAME.to_owned()]);
    assert!(!live(Tray::enabled));
    assert!(
        fs::read_to_string(home.config())
            .expect("the file must be readable")
            .contains("enabled = false")
    );
}

/// **Criterion 7 of task T-13-14 — the return.** Closing the dialog gives back both the view
/// and the ban.
///
/// The guard is the one `show_dialog` itself uses, so this measures the very mechanism the
/// product leaves the dialog by — including the way out through a panic, which is what the
/// `Drop` is for.
///
/// ⚠ **Why `CMD_AUTOSTART` is not fired here.** The entry and the rule are checked; the
/// command is not. Firing it with the gate open would reach `settings::set_autostart`, which
/// writes the running executable's path into `HKCU\…\CurrentVersion\Run` — or deletes the
/// value that is there — and that value belongs to whoever is running the tests, not to this
/// suite. The two commands share one door and one rule (`dialog_locks_command`, one `if` in
/// front of the whole table), so the door opening for `CMD_TOGGLE` is the door opening.
#[test]
fn closing_the_dialog_gives_both_entries_back() {
    let _locale = product_strings(settings::Language::Ru);
    let window = TestWindow::new();
    let home = TestDir::new("dialog-return");
    let _attached = attach_ui(&window, &home);

    let dialog = DialogSession::open();

    let while_up = Menu::build(
        true,
        true,
        NOT_FAIL_SAFE,
        settings::dialog_is_open(),
        &NOTHING_PENDING,
    )
    .expect("the menu must be creatable");

    for position in LOCKED_POSITIONS {
        assert!(
            is_grayed(while_up.handle(), position) && is_disabled(while_up.handle(), position),
            "entry {position} is greyed while the dialog is up"
        );
    }

    tray::dispatch_command(window.handle, tray::CMD_TOGGLE);

    assert!(live(Tray::enabled), "and the command is refused");
    assert_eq!(home.entries(), Vec::<String>::new(), "and writes nothing");

    drop(dialog);

    assert!(
        !settings::dialog_is_open(),
        "the guard lowers the flag on the way out, whatever the way is"
    );

    let after = Menu::build(
        true,
        true,
        NOT_FAIL_SAFE,
        settings::dialog_is_open(),
        &NOTHING_PENDING,
    )
    .expect("the menu must be creatable");

    for position in LOCKED_POSITIONS {
        println!(
            "entry {position} after the dialog closed: grayed={} disabled={}",
            is_grayed(after.handle(), position),
            is_disabled(after.handle(), position)
        );

        assert!(
            !is_grayed(after.handle(), position) && !is_disabled(after.handle(), position),
            "entry {position} is the user's again"
        );
    }

    assert!(
        after.items().iter().all(|item| item.enabled),
        "and the builder's record agrees, entry for entry"
    );

    // The ban is gone with the greying, and this is the fact rather than the intention: the
    // same call that did nothing above now moves the state and writes the file.
    tray::dispatch_command(window.handle, tray::CMD_TOGGLE);

    println!(
        "after the same command with the dialog closed: enabled={} entries={:?}",
        live(Tray::enabled),
        home.entries()
    );

    assert!(!live(Tray::enabled), "«Приостановить» works again");
    assert_eq!(home.entries(), vec![CONFIG_FILE_NAME.to_owned()]);
    assert!(
        fs::read_to_string(home.config())
            .expect("the file must be readable")
            .contains("enabled = false")
    );

    // The autostart half of the same door, checked on the rule and on the entry rather than
    // by firing it — see the note above this test.
    assert!(!tray::dialog_locks_command(
        tray::CMD_AUTOSTART,
        settings::dialog_is_open()
    ));
}

/// **Criterion 8 of task T-13-14 — the scenario of the finding.** The same three steps, walked
/// twice: once the way the code walked them before this task, and once through the door the
/// user actually walks through.
///
/// Act I is the audit's scenario reproduced exactly. Step 2 of it is written as
/// `with_tray(Tray::toggle_state)` because that single statement **was** the whole of the
/// `CMD_TOGGLE` arm of `dispatch_command` before this task: nothing stood in front of it. The
/// numbers it prints are the finding — `enabled` goes to `false` and the file says so, and
/// «Применить» puts it back to `true` from a copy taken before the user ever touched the tray.
///
/// Act II is the same three steps through `dispatch_command`. The divergence never opens: what
/// the tray holds equals what the dialog will apply, at every point, however many times the
/// command arrives.
///
/// Act III is the one the requirement is really about — a suspension the user *did* make, made
/// while no dialog was open, surviving «Применить» intact.
#[test]
fn the_lost_update_of_the_audit_is_not_reproducible_any_more() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("lost-update");
    let _attached = attach_ui(&window, &home);

    // ----- Act I: the finding, as the code performed it -----

    // «Настройки…»: `open_settings` copies the configuration out of the tray and
    // `show_dialog` raises the flag. Both, in that order, as the product does it.
    let dialog = DialogSession::open();
    let snapshot = live(|tray| tray.config().clone());

    assert!(snapshot.general.enabled, "the copy the dialog will edit");

    // The `CMD_TOGGLE` arm of `dispatch_command` as it read before this task.
    let _ = tray::with_tray(Tray::toggle_state);

    let after_old_toggle = live(Tray::enabled);
    let file_after_old_toggle =
        fs::read_to_string(home.config()).expect("the old road wrote the file");

    // «Применить»: `apply_settings` hands the snapshot back and `replace_config` puts it in
    // whole — the line the audit quoted.
    live_mut(|tray| tray.replace_config(snapshot.clone()));

    let after_old_apply = live(Tray::enabled);
    let file_after_old_apply = fs::read_to_string(home.config()).expect("and rewrote it");

    println!(
        "act I — snapshot.enabled={} | after the toggle: enabled={} file has «enabled = false»={} \
         | after «Применить»: enabled={} file has «enabled = true»={}",
        snapshot.general.enabled,
        after_old_toggle,
        file_after_old_toggle.contains("enabled = false"),
        after_old_apply,
        file_after_old_apply.contains("enabled = true")
    );

    assert!(!after_old_toggle, "the user's suspension took effect…");
    assert!(file_after_old_toggle.contains("enabled = false"));
    assert!(
        after_old_apply,
        "…and «Применить» undid it from a copy taken before it happened — the finding"
    );
    assert!(file_after_old_apply.contains("enabled = true"));

    // ----- Act II: the same three steps through the handler -----

    assert!(
        settings::dialog_is_open(),
        "the dialog of act I is still on the screen"
    );

    for _ in 0..10 {
        tray::dispatch_command(window.handle, tray::CMD_TOGGLE);

        assert_eq!(
            live(Tray::enabled),
            snapshot.general.enabled,
            "what the tray holds never parts from what the dialog will apply"
        );
    }

    live_mut(|tray| tray.replace_config(snapshot.clone()));

    let after_new_apply = live(Tray::enabled);

    println!(
        "act II — after 10 commands and «Применить»: enabled={after_new_apply} (snapshot said {})",
        snapshot.general.enabled
    );

    assert_eq!(
        after_new_apply, snapshot.general.enabled,
        "nothing was lost because nothing diverged"
    );

    // ----- Act III: a suspension that was really made, and survives -----

    drop(dialog);

    tray::dispatch_command(window.handle, tray::CMD_TOGGLE);

    assert!(!live(Tray::enabled), "the user suspends, with no dialog up");

    // Now the dialog opens on the suspended program, the user clicks the tray a few times —
    // and «Применить».
    let dialog = DialogSession::open();
    let snapshot = live(|tray| tray.config().clone());

    assert!(!snapshot.general.enabled, "the copy carries the suspension");

    for _ in 0..10 {
        tray::dispatch_command(window.handle, tray::CMD_TOGGLE);
        tray::dispatch_command(window.handle, tray::CMD_AUTOSTART);
    }

    live_mut(|tray| tray.replace_config(snapshot.clone()));

    let file = fs::read_to_string(home.config()).expect("the file must be readable");

    println!(
        "act III — after «Применить»: enabled={} file has «enabled = false»={}",
        live(Tray::enabled),
        file.contains("enabled = false")
    );

    assert!(
        !live(Tray::enabled),
        "the suspension is still there — this is the line that would have read `true` before"
    );
    assert!(file.contains("enabled = false"), "and the file agrees");

    drop(dialog);
}

/// **The gate is one door in front of the whole table, and it reads the shared rule.**
///
/// Swept over the source for the reason the neighbouring sweeps state: what a test can drive
/// is the behaviour, and what it cannot drive is the *shape*. The behaviour above proves the
/// two commands are refused; this proves they are refused by one `if` standing before the
/// `match` rather than by two checks inside two arms, which is what keeps an arm added later
/// from walking round the lock.
///
/// Insensitive to line endings by construction — `.gitattributes` declares `* text=auto
/// eol=crlf`, so a fresh worktree holds this file in CRLF while the index holds LF, and a
/// needle written with `\n` has to be matched against a text normalised to `\n`.
#[test]
fn the_lock_is_one_door_in_front_of_the_table_and_the_rule_is_defined_once() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    assert_eq!(
        source.matches("pub fn dialog_locks_command(").count(),
        1,
        "one rule, one definition — a second copy is how two answers to one question start \
         to differ"
    );
    assert!(
        source.contains("dialog_open && matches!(command, CMD_TOGGLE | CMD_AUTOSTART)"),
        "and the rule names the two commands of FR-91 that edit the configuration, and only \
         those two"
    );

    let at = source
        .find("pub fn dispatch_command(hwnd: HWND, command: u32) {")
        .expect("dispatch_command must be in this file");
    let body = &source[at..];
    let end = body.find("\n}").expect("a function closes with its brace");
    let body = &body[..end];

    println!("--- dispatch_command ---\n{body}");

    let gate = body
        .find("dialog_locks_command(command, settings::dialog_is_open())")
        .expect("the handler must consult the rule of task T-13-14");
    let table = body
        .find("match command {")
        .expect("the handler dispatches on the command");

    assert!(
        gate < table,
        "the lock stands in front of the whole table, not inside two of its arms"
    );
    assert_eq!(
        body.matches("dialog_locks_command").count(),
        1,
        "and it is asked exactly once — one door"
    );

    // The other half of the same rule: the menu asks it for both entries, and asks this very
    // function rather than a condition of its own.
    let at = source
        .find("    pub fn build(")
        .expect("Menu::build must be in this file");
    let build = &source[at..];
    let end = build
        .find("\n    }")
        .expect("a method closes with its brace");
    let build = &build[..end];

    assert!(
        build.contains("dialog_locks_command(CMD_TOGGLE, dialog_open)")
            && build.contains("dialog_locks_command(CMD_AUTOSTART, dialog_open)"),
        "the greying is the same rule asked for the same two entries"
    );
    assert!(
        build.contains("!resume_is_refused(enabled, fail_safe)"),
        "and it composes with task T-13-9's rule instead of replacing it"
    );
}

/// **The shape of the re-entry gate — task T-13-18.**
///
/// What the gate *does* is measured where it can be measured, in the unit tests of
/// `src\tray.rs`: those reach `show_menu`, `MENU_PAINT` and the flag, all of which are private
/// to the module by design and are going to stay that way. What no test can drive is the one
/// thing this task is about — the boundary. `TrackPopupMenuEx` is modal, and a test that let
/// `show_menu` reach it would hang instead of failing. So the boundary is swept, for the reason
/// the two sweeps above this one are swept.
///
/// Three claims, and each of them is a defect the task's own terms of reference name:
///
/// * the flag goes up **before** `MenuPaint::new` and `set_menu_background` — otherwise the
///   window between them stays open and the finding reproduces inside it;
/// * it comes down **after** `TrackPopupMenuEx` has returned — a bare pair of assignments
///   around the call would not survive an unwind, so the lowering is a `Drop`;
/// * and it comes down **before** `dispatch_command` — the modal dialog of FR-92 opens there,
///   and a menu shown over that dialog is the legitimate showing task T-13-14 greys two
///   entries of. Holding the gate across it would undo that task.
///
/// Insensitive to line endings by construction: `.gitattributes` declares `* text=auto
/// eol=crlf`, so a fresh worktree holds the file in CRLF while the index holds LF, and the
/// needles below are written with `\n`. Every boundary is an `expect` and not a fallback — a
/// body whose closing brace cannot be found is a failure of this test, not an invitation to
/// read to the end of the file.
#[test]
fn the_re_entry_gate_wraps_the_modal_call_and_lets_go_before_the_dialog() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    // The product half of the file. The unit tests of the same module read and write the flag
    // too, and counting their mentions in with the product's would make the counts below say
    // nothing.
    let end = source
        .find("\n#[cfg(test)]\nmod tests {")
        .expect("src\\tray.rs must carry its unit tests at the end");
    let product = &source[..end];

    let at = product
        .find("fn show_menu(x: i32, y: i32) {")
        .expect("show_menu must be in this file");
    let body = &product[at..];
    let end = body
        .find("\n}")
        .expect("show_menu must close with a brace of its own");
    let body = &body[..end];

    println!("--- show_menu ---\n{body}");

    let raise = body
        .find("let Some(on_screen) = MenuOnScreen::raise() else {")
        .expect("show_menu must ask for the right to the screen and turn round without it");
    let borrow = body
        .find("with_tray(|tray| {")
        .expect("show_menu reads the tray through with_tray");
    let paint = body
        .find("MenuPaint::new(menu.items().to_vec(), palette)")
        .expect("show_menu builds the paint state of the showing");
    let ground = body
        .find("set_menu_background(menu.handle(), paint.window_bg)")
        .expect("and hands the ground brush to Windows");
    let track = body
        .find("TrackPopupMenuEx(")
        .expect("show_menu shows the menu with TrackPopupMenuEx");
    let take = body
        .find("MENU_PAINT.with(|slot| slot.borrow_mut().take())")
        .expect("the SEC-05 gate comes down after the modal call");
    let release = body
        .find("drop(on_screen);")
        .expect("the right to the screen is given up by name, like the frame hook");
    let dispatch = body
        .find("dispatch_command(hwnd, command)")
        .expect("and the chosen command is carried out at the end");

    println!(
        "offsets in show_menu: raise {raise}, with_tray {borrow}, MenuPaint::new {paint}, \
         set_menu_background {ground}, TrackPopupMenuEx {track}, take {take}, \
         drop(on_screen) {release}, dispatch_command {dispatch}"
    );

    assert!(
        raise < borrow,
        "the gate is the first thing in the function — a refused entry must not even reach \
         the tray"
    );
    assert!(
        raise < paint && raise < ground,
        "and it is up before the paint state is built and before the ground brush is handed \
         to Windows, or the finding reproduces in the window between"
    );
    assert!(
        track < take && take < release,
        "it comes down after TrackPopupMenuEx has returned and after the paint state has \
         been taken out"
    );
    assert!(
        release < dispatch,
        "and before dispatch_command opens the modal dialog of FR-92 — a menu shown over \
         that dialog is the legitimate showing task T-13-14 greys two entries of"
    );

    // The gates are two, and they are not each other's spare. `MENU_PAINT` answers «is our
    // drawing active», the flag answers «is a menu of ours up», and the degraded showing is
    // where the two part company.
    assert_eq!(
        body.matches("MenuOnScreen::raise()").count(),
        1,
        "one door into the showing"
    );
    assert!(
        !body.contains("MENU_PAINT.with(|slot| slot.borrow().is_some())"),
        "and it is not a reading of the paint slot — that gate is down for the whole of a \
         degraded showing"
    );
    assert!(
        body.contains("MENU_PAINT.with(|slot| slot.replace(Some(paint)))"),
        "the SEC-05 gate itself is untouched by this task"
    );

    // The lowering is a `Drop` and the flag has no other writer. A bare `set(true) …
    // set(false)` pair around the modal call is exactly what these two counts forbid.
    let writes = product
        .matches("MENU_ON_SCREEN.with(|flag| flag.set(")
        .count();
    let guard_at = product
        .find("impl Drop for MenuOnScreen {")
        .expect("the flag must be lowered by a Drop and not by a call");
    let lower = product
        .find("MENU_ON_SCREEN.with(|flag| flag.set(false));")
        .expect("and that Drop must be what lowers it");

    println!("writers of MENU_ON_SCREEN in the product half: {writes}");

    assert_eq!(
        writes, 2,
        "the flag has exactly two writers — one raise and one lowering"
    );
    assert!(
        guard_at < lower,
        "and the lowering lives inside the Drop, so no path out of a showing can miss it"
    );
}

#[test]
fn the_autostart_entry_shows_the_check_mark_of_the_configuration() {
    let _locale = product_strings(settings::Language::Ru);
    let on = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");
    let off = Menu::build(true, false, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    assert!(
        is_checked(on.handle(), 3),
        "`general.autostart` = true must show a check mark"
    );
    assert!(
        !is_checked(off.handle(), 3),
        "`general.autostart` = false must not show one"
    );

    // The builder's record agrees with what Windows shows. Not a duplicate of the above:
    // the drawing of T-11-10 paints the mark from this very field, so the two must never
    // part ways.
    assert!(on.items()[2].checked, "the record behind the drawn mark");
    assert!(!off.items()[2].checked);

    // The check mark is the only difference: FR-93 is task T-08-1's, and this task must not
    // have grown a second way of showing the same thing.
    assert_eq!(on.items()[2].label, off.items()[2].label);
}

#[test]
fn the_item_data_of_every_command_entry_is_its_command_number() {
    // **SEC-05, criterion 9 of T-11-10.** `WM_DRAWITEM` can be forged by any process of
    // our integrity level, so what our entries carry in `itemData` must be a number a
    // handler merely looks up — never a pointer it would dereference. Read back out of
    // Windows: the `itemData` of every command entry equals the command identifier of the
    // same entry, and both equal what the builder recorded.
    let _locale = product_strings(settings::Language::Ru);
    let menu = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must be creatable");

    // The six command positions of the FR-91 block — everything but the two rules. Six since
    // task Т-32-4 put «Написать автору…» at position 5.
    let command_positions = [0u32, 2, 3, 5, 6, 7];

    assert_eq!(menu.items().len(), command_positions.len());

    for (item, position) in menu.items().iter().zip(command_positions) {
        let identifier = command_at(menu.handle(), position);
        let data = item_data_at(menu.handle(), position);

        println!("{position}: id={identifier:#06x} itemData={data:#x}");

        assert_eq!(
            identifier, item.command,
            "the command identifier of entry {position} is the builder's"
        );
        assert_eq!(
            data,
            usize::try_from(identifier).expect("a command fits in a usize"),
            "SEC-05: itemData of entry {position} is the command number, nothing else"
        );
    }
}

#[test]
fn measure_and_draw_are_ignored_while_no_menu_of_ours_is_on_the_screen() {
    // **SEC-05, criterion 10 of T-11-10.** `WM_MEASUREITEM` and `WM_DRAWITEM` are handled
    // only while the program itself holds the menu on the screen. This tray has never
    // shown one, so the gate is down for the whole test, and both messages must come back
    // `Ignored` — like everything foreign.
    let window = TestWindow::new();
    let home = TestDir::new("od_gate");
    let mut tray = install(&window, &home);

    // A null lParam first: the handler must refuse before looking at any pointer.
    assert_eq!(
        tray.handle_message(WM_MEASUREITEM, WPARAM(0), LPARAM(0)),
        Reaction::Ignored,
        "WM_MEASUREITEM with the gate down"
    );
    assert_eq!(
        tray.handle_message(WM_DRAWITEM, WPARAM(0), LPARAM(0)),
        Reaction::Ignored,
        "WM_DRAWITEM with the gate down"
    );

    // A well-formed forgery next — a real `MEASUREITEMSTRUCT` naming a real command,
    // exactly what a process of our integrity level could send. The gate, not the shape
    // of the structure, is what refuses it; and the structure must come back untouched.
    let mut forged = MEASUREITEMSTRUCT {
        CtlType: ODT_MENU,
        CtlID: 0,
        itemID: tray::CMD_TOGGLE,
        itemWidth: 0xDEAD,
        itemHeight: 0xBEEF,
        itemData: usize::try_from(tray::CMD_TOGGLE).expect("a command fits in a usize"),
    };

    let reaction = tray.handle_message(
        WM_MEASUREITEM,
        WPARAM(0),
        LPARAM((&raw mut forged) as isize),
    );

    println!("forged WM_MEASUREITEM -> {reaction:?}");

    assert_eq!(reaction, Reaction::Ignored, "the gate is down: not ours");
    assert_eq!(
        (forged.itemWidth, forged.itemHeight),
        (0xDEAD, 0xBEEF),
        "and nothing was written into the forgery"
    );

    // The same forgery naming a **rule** — the second kind of entry the drawing knows since
    // task T-11-22, and the second thing that must be behind the very same gate.
    let mut forged_rule = MEASUREITEMSTRUCT {
        CtlType: ODT_MENU,
        CtlID: 0,
        itemID: 0,
        itemWidth: 0xDEAD,
        itemHeight: 0xBEEF,
        itemData: usize::try_from(tray::MENU_SEPARATOR_DATA).expect("a number fits in a usize"),
    };

    let reaction = tray.handle_message(
        WM_MEASUREITEM,
        WPARAM(0),
        LPARAM((&raw mut forged_rule) as isize),
    );

    println!("forged WM_MEASUREITEM naming a rule -> {reaction:?}");

    assert_eq!(
        reaction,
        Reaction::Ignored,
        "the gate is down for rules too"
    );
    assert_eq!(
        (forged_rule.itemWidth, forged_rule.itemHeight),
        (0xDEAD, 0xBEEF),
        "and nothing was written into that forgery either"
    );
}

// ---------------------------------------------------------------------------------------
// Task T-13-20 — `WM_SETTINGCHANGE` does not dereference a pointer nobody needs
//
// The audit of 2026-08-24 found the arm calling the reader on **every** such message: up to
// 64 UTF-16 units read from the address in `lParam` after one null check. `WM_SETTINGCHANGE`
// is posted and is therefore not marshalled, so that address is whatever the sender wrote —
// a forgery from a process of our own integrity level, or the merely wrong number a
// third-party program is known to broadcast — and reading it is a read of an arbitrary
// address. Meanwhile the only consumer of the answer left on its first line whenever no
// window of FR-92а was up, so the read was usually for nothing.
//
// ⚠ **What is measured where.** The counter that says «читателя не звали» lives beside the
// reader in `src\tray.rs`, and so do the four tests that read it: publishing a function that
// dereferences a pointer out of a window message, so that an integration test could reach it,
// would widen exactly the surface SEC-05 narrows. What is measured here is the published
// half — the rule as a function of its arguments, the real `Tray::handle_message` fed a
// pointer to nothing, and the shape of the arm in the source.
// ---------------------------------------------------------------------------------------

/// **The rule itself, all twelve rows.** Two windows and three themes, and the string is
/// worth reading in exactly two of the twelve states.
#[test]
fn the_setting_change_string_is_wanted_in_exactly_two_of_the_twelve_states() {
    for (dialog_open, about_open, setting, expected) in [
        // Nobody on the screen. Whatever the theme says, there is no window to repaint —
        // this is the row the audit found the program reading a foreign pointer in.
        (false, false, ThemeSetting::System, false),
        (false, false, ThemeSetting::Light, false),
        (false, false, ThemeSetting::Dark, false),
        // The settings dialog of FR-92 is up. Only `system` gives the system a say: under
        // the two fixed themes FR-92а pins the palette and a broadcast changes nothing.
        (true, false, ThemeSetting::System, true),
        (true, false, ThemeSetting::Light, false),
        (true, false, ThemeSetting::Dark, false),
        // The «О программе» window of task T-13-17, alone. This disjunct has to carry the
        // gate by itself, or that task's repair is undone.
        (false, true, ThemeSetting::System, true),
        (false, true, ThemeSetting::Light, false),
        (false, true, ThemeSetting::Dark, false),
        // Both records set. Not a state the program reaches — «О программе» is modal — and
        // the rule answers it all the same, because a rule with a hole in it is a rule that
        // depends on somebody keeping the hole unreachable.
        (true, true, ThemeSetting::System, true),
        (true, true, ThemeSetting::Light, false),
        (true, true, ThemeSetting::Dark, false),
    ] {
        let wanted = tray::setting_name_is_wanted(dialog_open, about_open, setting);

        println!("dialog={dialog_open} about={about_open} {setting:?} -> wanted={wanted}");

        assert_eq!(
            wanted, expected,
            "setting_name_is_wanted({dialog_open}, {about_open}, {setting:?})"
        );
    }
}

/// **Criterion 5 of task T-13-20, on the product's own road.** A real tray, a real
/// `Tray::handle_message`, a real `WM_SETTINGCHANGE` — and an `lParam` that is a pointer to
/// nothing.
///
/// The number is in the null partition: Windows reserves the first 64 KiB of every process's
/// address space and maps nothing there ever, so it cannot alias a page of this test or of
/// anybody, and a read of it is an access violation and nothing subtler. **That this test
/// finishes at all is the assertion.** A build that let execution through to the reader would
/// not fail a line below — it would take the whole test executable down, and `cargo test`
/// would report the target as failed rather than this one case.
///
/// Both halves of the gate are exercised: nobody on the screen first, then a window up with
/// the theme fixed by hand — the second is the state criterion 7 names, and it has to refuse
/// before the dereference too and not merely afterwards.
#[test]
fn a_setting_change_naming_a_pointer_to_nothing_is_survived() {
    let window = TestWindow::new();
    let home = TestDir::new("settingchange_gate");
    let mut tray = install(&window, &home);

    // Deliberately unreadable, deliberately harmless: see the doc comment above.
    let nowhere = LPARAM(0x2A2A);

    assert!(
        !settings::dialog_is_open(),
        "no settings dialog on this thread"
    );
    assert!(
        !settings::about_is_open(),
        "and no «О программе» window either"
    );
    assert_eq!(
        tray.config().general.theme,
        ThemeSetting::System,
        "section 7 has `[general].theme` default to `system` — so it is the *other* half of \
         the gate that must refuse this first message"
    );

    let reaction = tray.handle_message(WM_SETTINGCHANGE, WPARAM(0), nowhere);

    println!("WM_SETTINGCHANGE with nobody to tell -> {reaction:?}");

    assert_eq!(
        reaction,
        Reaction::Ignored,
        "the broadcast goes on to DefWindowProcW exactly as it did before this task"
    );

    // And now the other half. A window *is* up, but the theme is fixed by hand, so the
    // system's switch has no say and the string is still not worth its risk.
    let mut fixed = tray.config().clone();
    fixed.general.theme = ThemeSetting::Dark;
    tray.replace_config(fixed);

    let dialog = DialogSession::open();

    assert!(settings::dialog_is_open(), "a window is up for this half");

    let reaction = tray.handle_message(WM_SETTINGCHANGE, WPARAM(0), nowhere);

    println!("WM_SETTINGCHANGE under a hand-fixed theme -> {reaction:?}");

    assert_eq!(reaction, Reaction::Ignored, "and this one is ignored too");

    drop(dialog);
}

/// **The order of the checks is the repair — swept over the source, because no test can drive
/// an order.**
///
/// Three claims, and each of them is the task's own terms of reference:
///
/// * the arm hands the reading to one function, [`on_setting_change`], with the theme copied
///   out of the tray on the way in — so the decision has one home and not two;
/// * inside it the gate stands **before** `setting_change_name`, and the gate names *both*
///   windows of FR-92а. Written the other way round — read the string, then decide — the
///   function would be the finding again with a comment on top;
/// * `setting_change_name` has exactly that one caller in the whole module.
///
/// ⭐ **Task Т-22-9, finding м12 of the audit of 2026-09-01 — the last configuration event that
/// left no trace now leaves one, and the `TODO` that stood for it is gone.**
///
/// # Why this test reads the source
///
/// The call site is inside `Tray::save_config`, and a `Tray` needs a live window, a shell to add
/// an icon to and a `%APPDATA%` of its own; the unit tests in `src\tray.rs` drive the journalling
/// itself — the mapping of the error onto a code, and a real refusal of the real writer reaching
/// the ring — but they call the helper, not the save. What is left to check is that the save
/// **calls** it, and that is a property of the shape of one function, which is also what a
/// reviewer checks. `the_cheap_checks_stand_in_front_of_the_pointer_and_name_both_windows` below
/// reads the source for the same reason and in the same way.
///
/// # What is asserted
///
/// That the write's failure branch names the journalling helper and the row, and that neither the
/// `TODO(T-06-4)` that stood there for six stages nor the `let _ = error` beside it is left in the
/// product half of the module. The `TODO` is checked over the **whole** file: a debt that is paid
/// must not survive as a comment saying it is not.
///
/// Insensitive to line endings by construction, for the reason the test below gives.
#[test]
fn the_refused_configuration_write_is_journalled_and_the_todo_is_gone() {
    let whole = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    assert!(
        !whole.contains("TODO(T-06-4)"),
        "Т-22-9: the debt is paid, and the comment claiming it is not may not outlive it"
    );

    // The product half. The module's own unit tests name the helper too, and counting their
    // calls beside the product's would measure the suite instead of the program.
    let at = whole
        .find("\n#[cfg(test)]\nmod tests {")
        .expect("the unit-test module must be at the foot of this file");
    let source = &whole[..at];

    let save = source
        .find("fn save_config(&mut self)")
        .map(|start| &source[start..])
        .expect("src\\tray.rs must define Tray::save_config");
    let save = &save[..save
        .find("\n    }\n")
        .expect("a method closes at four spaces and a brace")];

    assert!(
        save.contains("note_configuration_failure(CONFIG_WRITE_FAILED, &error)"),
        "Т-22-9: a save that the file refused is the one configuration event of the six that \
         used to leave nothing behind"
    );
    assert!(
        !save.contains("let _ = error"),
        "and the line that dropped it is gone rather than kept beside the new one"
    );
}

/// Insensitive to line endings by construction — `.gitattributes` declares `* text=auto
/// eol=crlf`, so a fresh worktree holds this file in CRLF while the index holds LF, and a
/// needle written with `\n` has to be matched against a text normalised to `\n`.
#[test]
fn the_cheap_checks_stand_in_front_of_the_pointer_and_name_both_windows() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    // The product half of the file. The module's own unit tests reach the reader too — that
    // is what closes criteria 5 to 7 — and counting their calls beside the product's would
    // measure the suite instead of the program.
    let at = source
        .find("\n#[cfg(test)]\nmod tests {")
        .expect("the unit-test module must be at the foot of this file");
    let source = &source[..at];

    assert_eq!(
        source.matches("pub fn setting_name_is_wanted(").count(),
        1,
        "one rule, one definition"
    );
    assert!(
        source.contains("(dialog_open || about_open) && setting == theme::ThemeSetting::System"),
        "and the rule is «есть кому отдать» — either window — **and** the theme that lets the \
         system have a say"
    );

    // The arm of `handle_message`: one call, and the theme travels in as an argument.
    assert!(
        source.contains("on_setting_change(lparam, self.config().general.theme)"),
        "the arm hands the reading to one function, with the cheap local fact already in hand"
    );
    assert_eq!(
        source.matches("unsafe { setting_change_name(").count(),
        1,
        "and the product half of the module has exactly one call site for the reader"
    );

    let at = source
        .find("fn on_setting_change(lparam: LPARAM, setting: theme::ThemeSetting) {")
        .expect("on_setting_change must be in this file");
    let body = &source[at..];
    let end = body.find("\n}").expect("a function closes with its brace");
    let body = &body[..end];

    println!("--- on_setting_change ---\n{body}");

    let gate = body
        .find("if !setting_name_is_wanted(")
        .expect("the gate of task T-13-20 must be in the body");
    let read = body
        .find("setting_change_name(lparam)")
        .expect("and so must the reading it guards");

    assert!(
        gate < read,
        "the cheap local checks stand **before** any use of `lparam` — after it, the repair \
         does nothing at all"
    );
    assert!(
        body.contains("settings::dialog_is_open()") && body.contains("settings::about_is_open()"),
        "and «кому отдать» is both windows of FR-92а: task T-13-17 made the about window a \
         listener, and a gate asking only the dialog would silently undo it"
    );

    // The contract of the reader names the preconditions — criterion 8.
    let at = source
        .find("/// The string a `WM_SETTINGCHANGE` names, read within the bounds of SEC-05")
        .expect("the reader's doc comment must be in this file");
    let contract = &source[at..];
    let end = contract
        .find("\nunsafe fn setting_change_name(")
        .expect("the doc comment sits on the function");
    let contract = &contract[..end];

    println!("--- the contract of setting_change_name ---\n{contract}");

    // Flattened to one line so that the claims below are about the *words* of the contract
    // and not about where its author happened to wrap them.
    let contract: String = contract
        .lines()
        .map(|line| line.trim().trim_start_matches("///").trim())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        contract.contains("dereferences a pointer that arrived on a window message"),
        "the contract says in as many words what the function does with a foreign pointer"
    );
    assert!(
        contract.contains("necessary and not sufficient"),
        "and that the null check is not the whole of the protection — ловушка 2"
    );
    assert!(
        contract.contains("# Safety"),
        "and it states the preconditions under NFR-14's own heading"
    );
    for precondition in [
        "delivered to this thread",
        "hidden window",
        "setting_name_is_wanted",
    ] {
        assert!(
            contract.contains(precondition),
            "the preconditions must name «{precondition}» — that is what makes the pointer \
             fit to read"
        );
    }
}

// ---------------------------------------------------------------------------------------
// FR-92а, task T-11-21 — the menu drawn as the mock-up draws it
// ---------------------------------------------------------------------------------------

/// The three points of the menu's check mark as `scratchpad\chrome.ps1` strokes them, in
/// **tenths of a mock-up pixel** and measured from `($bx, $by)` — the left edge of the mark
/// and the vertical middle of the entry.
///
/// Written out here rather than imported, for the reason the FR-91 block above is written
/// out: a test that read the crate's own constant and then asserted the crate's own constant
/// would pass whatever the constant said. These are the numbers of the generator the mock-up
/// `ui-06-chrome.png` was drawn by — `(PtF $bx ($by+1))`, `(PtF ($bx+4) ($by+5))`,
/// `(PtF ($bx+11) ($by-6))`.
const CHROME_CHECK_POINTS: [(i32, i32); 3] = [(0, 10), (40, 50), (110, -60)];

/// The pen of that mark, in tenths of a mock-up pixel — `Pen($T.Fg,[single]2.1)`.
const CHROME_CHECK_PEN: i32 = 21;

/// Inset of the highlight from the left and right edges of the menu, in mock-up pixels —
/// the `($mX+5)` … `($mW-10)` of `FillR $g ($mX+5) $iy ($mW-10) $itemH $T.Hover 6`.
const CHROME_HOVER_INSET: i32 = 5;

/// Corner radius of that highlight, in mock-up pixels — the trailing `6` of the same call.
const CHROME_HOVER_RADIUS: i32 = 6;

/// The DPI the «при 96 DPI» column of the report is written at.
const SCREEN_DPI: i32 = 96;

#[test]
fn the_check_mark_of_the_menu_is_the_figure_the_mock_up_strokes() {
    // **Criterion 9 of T-11-21, the figure half.** The mark of FR-93 used to be two lines
    // of a step invented from `SM_CXMENUCHECK`; it is now the generator's own polyline,
    // moved by `MENU_CHECK_AIR` so that the square handed to the smoothing holds the pen
    // and its fading edge as well as the path.
    let air = tray::MENU_CHECK_AIR * 10;
    let across: [i32; 3] = CHROME_CHECK_POINTS.map(|point| point.0);
    let down: [i32; 3] = CHROME_CHECK_POINTS.map(|point| point.1);

    let left = across.into_iter().min().expect("three points");
    let top = down.into_iter().min().expect("three points");
    let right = across.into_iter().max().expect("three points");
    let bottom = down.into_iter().max().expect("three points");

    let expected = CHROME_CHECK_POINTS.map(|(x, y)| (x - left + air, y - top + air));

    println!("chrome.ps1 {CHROME_CHECK_POINTS:?} + air {air} -> {expected:?}");

    assert_eq!(
        tray::MENU_CHECK_MARK.points_tenths,
        expected,
        "the polyline of the menu is the generator's, moved by the air around it"
    );
    assert_eq!(
        tray::MENU_CHECK_MARK.pen_tenths,
        CHROME_CHECK_PEN,
        "and the pen is the generator's 2,1 mock-up pixels"
    );

    // The path is square — eleven mock-up pixels each way — so the square around it is that
    // plus the air on all four sides.
    assert_eq!(right - left, bottom - top, "the path of the mark is square");
    assert_eq!(
        tray::MENU_CHECK_CELL,
        (right - left) / 10 + 2 * tray::MENU_CHECK_AIR,
        "the square is the path plus the air on each side"
    );
}

#[test]
fn the_smoothing_tile_of_the_check_mark_cuts_nothing_off_it() {
    // **Criterion 9 of T-11-21, the reason `MENU_CHECK_AIR` exists.**
    // `theme::draw_check_mark` clamps the tile it smooths in to the square it is given,
    // and everything of the stroke outside that tile is simply never drawn. So the stroke —
    // the path plus half a pen plus the row the smoothed edge fades into, which is exactly
    // what `stroke_bounds` answers — has to fit inside the square at every scale.
    for dpi in [SCREEN_DPI, 120, 144, 192] {
        let side = theme::scaled(tray::MENU_CHECK_CELL, dpi);
        // Task Т-30-3: a rectangle and a mirror flag. The square is the cell the tick is drawn
        // in, and `false` is the way round the twelve left-to-right locales draw it — the
        // mirrored way round is the reflection of this one, so it fits the square exactly when
        // this one does.
        let cell = RECT {
            left: 0,
            top: 0,
            right: side,
            bottom: side,
        };
        let points = theme::check_mark_points(&cell, tray::MENU_CHECK_MARK, dpi, false);
        let thickness = theme::scaled_tenths(tray::MENU_CHECK_MARK.pen_tenths, dpi);
        let bounds = theme::stroke_bounds(&points, thickness);

        println!(
            "{dpi} DPI: square {side}, points {points:?}, pen {thickness}, stroke \
             {}..{} x {}..{}",
            bounds.left, bounds.right, bounds.top, bounds.bottom
        );

        assert!(
            bounds.left >= 0 && bounds.top >= 0,
            "the stroke may not start above or left of the square at {dpi} DPI"
        );
        assert!(
            bounds.right <= side && bounds.bottom <= side,
            "the stroke may not run past the square at {dpi} DPI"
        );
    }
}

#[test]
fn the_highlight_of_an_entry_is_the_inset_rounded_stripe_of_the_mock_up() {
    // **Criterion 9 of T-11-21, the highlight half.** The entry under the cursor used to be
    // a `FillRect` of the whole row; the mock-up holds the stripe five of its pixels off
    // each edge, gives it the whole height of the entry and rounds it by six.
    let item = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 200,
        bottom: 27,
    };

    let inset = theme::scaled(CHROME_HOVER_INSET, SCREEN_DPI);
    let stripe = tray::menu_hover_rect(&item, SCREEN_DPI);

    println!(
        "inset {CHROME_HOVER_INSET} px макета -> {inset} px at 96 DPI; stripe \
         {}..{} x {}..{}",
        stripe.left, stripe.right, stripe.top, stripe.bottom
    );

    assert_eq!(
        inset, 4,
        "five mock-up pixels are four screen pixels at 96 DPI"
    );
    assert_eq!(
        (stripe.left, stripe.right),
        (item.left + inset, item.right - inset),
        "the stripe is held off both edges"
    );
    assert_eq!(
        (stripe.top, stripe.bottom),
        (item.top, item.bottom),
        "and takes the whole height of the entry"
    );

    // The radius is the one number the generator gives every rounded figure it draws, which
    // is the constant the dialog already rounds by — there is no second radius for the menu.
    assert_eq!(theme::CORNER_RADIUS, CHROME_HOVER_RADIUS);
    assert_eq!(
        theme::scaled(theme::CORNER_RADIUS, SCREEN_DPI),
        4,
        "six mock-up pixels are four screen pixels at 96 DPI"
    );
}

/// **Criterion 10 of task T-12-9.** The mock-up hands **every** entry the same ink — line 167
/// of `chrome.ps1` gives `$brFg` to the row under the cursor and to every other row alike —
/// and the highlight is the whole of what marks the hot one. The product used to brighten
/// that row's text to `sel_fg` instead.
///
/// Swept over the source, and that is deliberate rather than lazy: the ink is chosen inside
/// the answer to `WM_DRAWITEM`, and SEC-05 keeps that answer behind a gate which is **down**
/// unless the program itself has a menu on the screen — the test right above this file's
/// drawing tests exists to prove the gate stays down. So there is no way to make the drawing
/// run from here, and the live half of the criterion is a shot of the menu with the cursor on
/// an entry plus a histogram of its glyph cores: 155 px of `text` where 155 px of `sel_fg`
/// used to be. That measurement is in `reports\T-12-9.md`; this guards the line it changed.
#[test]
fn the_ink_of_a_menu_entry_does_not_depend_on_the_cursor() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable");

    let hits: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            line.contains("sel_fg") && !code.starts_with("//") && !code.starts_with("///")
        })
        .map(|(index, _)| index + 1)
        .collect();

    println!("lines of src\\tray.rs naming sel_fg outside a comment: {hits:?}");

    assert!(
        hits.is_empty(),
        "T-12-9: src\\tray.rs picks up sel_fg at lines {hits:?}; the menu paints every entry \
         in `text`, hot or not"
    );

    // And the sweep guards something a shot can see. The two roles are the same number on
    // «Тумане», which is why the defect hid there for a whole stage, and twelve levels apart
    // on «Графите» — read from the palettes rather than written out, because what matters
    // here is that the roles *differ*, not what either of them is this month.
    let graphite = (
        lang_switcher::theme::GRAPHITE.text.0,
        lang_switcher::theme::GRAPHITE.sel_fg.0,
    );
    let fog = (
        lang_switcher::theme::FOG.text.0,
        lang_switcher::theme::FOG.sel_fg.0,
    );

    println!("graphite text/sel_fg {graphite:?}; fog text/sel_fg {fog:?}");

    assert_ne!(
        graphite.0, graphite.1,
        "«Графит» is the palette where choosing the wrong role is visible"
    );
    assert_eq!(
        fog.0, fog.1,
        "«Туман» is the palette where it was invisible — which is why the sweep above, and \
         not a shot of one theme, is what keeps it fixed"
    );
}

#[test]
fn the_check_mark_stands_inside_the_check_column_of_the_entry() {
    // The square of the mark is centred on the check column and on the middle of the entry,
    // and it may not grow out of that column — the width of an entry and the left edge of
    // its text are both built on the same metric (task T-11-10, untouched here).
    let column = check_column();
    let item = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: 200,
        bottom: 27,
    };

    let cell = tray::menu_check_cell(&item, column, SCREEN_DPI);

    println!(
        "SM_CXMENUCHECK={column}; cell {}..{} x {}..{}",
        cell.left, cell.right, cell.top, cell.bottom
    );

    assert_eq!(
        cell.right - cell.left,
        cell.bottom - cell.top,
        "the square of the mark is square"
    );
    assert_eq!(
        cell.right - cell.left,
        theme::scaled(tray::MENU_CHECK_CELL, SCREEN_DPI),
        "and is the mock-up's own side through the scale"
    );
    assert_eq!(
        (cell.top + cell.bottom) / 2,
        (item.top + item.bottom) / 2,
        "centred on the middle of the entry, as `$by = $iy + $itemH/2` centres it"
    );

    // The square fits in the column, and therefore cannot reach the text: the left edge of
    // the label is the column plus the gap of T-11-10, and that layout is not this task's.
    let text_left = item.left + tray::MENU_H_PAD + column + tray::MENU_CHECK_GAP;

    assert!(
        cell.right - cell.left <= column,
        "the square of the mark fits inside the check column"
    );
    assert!(
        cell.left >= item.left && cell.right <= text_left,
        "and stands between the edge of the entry and its text"
    );
}

/// Height of one entry as `scratchpad\chrome.ps1` lays it out, in mock-up pixels — `$itemH = 38`.
const CHROME_ITEM_H: i32 = 38;

/// Height of the stripe of one rule, in mock-up pixels — the `$sepH = 11` of the same line.
const CHROME_SEP_H: i32 = 11;

/// Inset of the line of a rule from the edge of the menu, in mock-up pixels — the `($mX+12)`
/// and `($mX+$mW-12)` of
/// `$g.DrawLine($pen, ($mX+12), ($iy + $sepH/2), ($mX+$mW-12), ($iy + $sepH/2))`.
const CHROME_SEP_INSET: i32 = 12;

/// The height the system menu face measures at 96 DPI, in screen pixels.
///
/// Measured, not chosen: `GetTextExtentPoint32W` with `lfMenuFont` answers 15 on this machine,
/// and the entry of task T-11-10 — text plus five screen pixels above and below — came out the
/// 25 px the controller measured off the live menu, which is the same 15 read backwards. The
/// mock-up draws that face at 140 %, so it is 21 of the mock-up's own pixels there.
const MENU_FACE_HEIGHT: i32 = 15;

#[test]
fn an_entry_is_as_tall_as_the_mock_up_draws_it_and_still_grows_with_the_face() {
    // **Point 5 of T-11-22.** The entry was 25 px against the mock-up's 38 mock-up pixels —
    // 27,1 screen pixels — because the padding of task T-11-10 was five *screen* pixels.
    // The mock-up number now enters as padding and not as a replacement for the measurement:
    // what is padded is still what `GetTextExtentPoint32W` said.
    let reference = theme::scaled(CHROME_ITEM_H, SCREEN_DPI);
    let ours = tray::menu_item_height(MENU_FACE_HEIGHT, SCREEN_DPI);

    println!(
        "chrome.ps1 $itemH = {CHROME_ITEM_H} px макета -> {reference} px at 96 DPI; \
         face {MENU_FACE_HEIGHT} + air -> {ours}"
    );

    assert_eq!(
        reference, 27,
        "38 mock-up pixels are 27,1 screen pixels at 96 DPI"
    );
    assert_eq!(
        ours, reference,
        "the entry of the mock-up's own face lands on the mock-up's own height"
    );

    // The measurement is still a measurement: a face two pixels taller makes a row two
    // pixels taller, which is what «высота по-прежнему растёт вместе с начертанием» means.
    for taller in [1, 2, 7, 40] {
        assert_eq!(
            tray::menu_item_height(MENU_FACE_HEIGHT + taller, SCREEN_DPI),
            ours + taller,
            "the air is added to the measurement, not substituted for it"
        );
    }

    // And the air itself scales with the display, or the row would be squat at 150 %.
    let mut previous = 0;

    for dpi in [SCREEN_DPI, 120, 144, 192] {
        let height = tray::menu_item_height(MENU_FACE_HEIGHT, dpi);

        println!("  {dpi} DPI: {height} px for a {MENU_FACE_HEIGHT} px face");

        assert!(
            height > previous,
            "the air of the entry must grow with the display scale"
        );

        previous = height;
    }
}

#[test]
fn the_rule_of_the_menu_is_the_line_the_mock_up_strokes() {
    // **Point 2 of T-11-22.** The rule used to be the system's engraved groove — two lines,
    // 160,160,160 and 255,255,255, on a 9 px band of 240,240,240. The mock-up draws a band of
    // the ground with one line of `$T.Sep` across its middle, held off both edges.
    let item = windows::Win32::Foundation::RECT {
        left: 0,
        top: 40,
        right: 214,
        bottom: 48,
    };

    let inset = theme::scaled(CHROME_SEP_INSET, SCREEN_DPI);
    let line = tray::menu_separator_line(&item, SCREEN_DPI);

    println!(
        "inset {CHROME_SEP_INSET} px макета -> {inset} px at 96 DPI; band \
         {CHROME_SEP_H} px макета -> {} px; line {}..{} x {}..{}",
        theme::scaled(CHROME_SEP_H, SCREEN_DPI),
        line.left,
        line.right,
        line.top,
        line.bottom
    );

    assert_eq!(
        theme::scaled(CHROME_SEP_H, SCREEN_DPI),
        8,
        "eleven mock-up pixels are 7,9 screen pixels at 96 DPI"
    );
    assert_eq!(
        inset, 9,
        "twelve mock-up pixels are 8,6 screen pixels at 96 DPI"
    );
    assert_eq!(
        (line.left, line.right),
        (item.left + inset, item.right - inset),
        "the line is held off both edges of the entry"
    );
    assert_eq!(
        line.bottom - line.top,
        1,
        "and is one screen pixel thick, at this scale and at every other"
    );

    // On the vertical middle of the band — `($iy + $sepH/2)`.
    for dpi in [SCREEN_DPI, 120, 144, 192] {
        let band = theme::scaled(CHROME_SEP_H, dpi);
        let stripe = windows::Win32::Foundation::RECT {
            left: 0,
            top: 40,
            right: 214,
            bottom: 40 + band,
        };
        let line = tray::menu_separator_line(&stripe, dpi);

        println!(
            "  {dpi} DPI: band {band} px, line y {}..{} x {}..{}",
            line.top, line.bottom, line.left, line.right
        );

        assert_eq!(
            line.top,
            (stripe.top + stripe.bottom) / 2,
            "the line lies on the middle of the band at {dpi} DPI"
        );
        assert!(
            line.top >= stripe.top && line.bottom <= stripe.bottom,
            "and inside it at {dpi} DPI"
        );
        assert!(
            line.left > stripe.left && line.right < stripe.right,
            "held off both edges at {dpi} DPI"
        );
    }
}

#[test]
fn the_entries_are_drawn_in_the_face_this_program_names_for_itself() {
    // **Criterion 10 of T-11-21.** The entries are measured and drawn in `lfMenuFont` of
    // `SPI_GETNONCLIENTMETRICS` put through `theme::smoothed_logfont` — one field
    // changed, the quality, and not a byte else. The metrics therefore do not move: that
    // was measured by task T-11-20 on the dialog's own face and is held by two tests of
    // `tests\settings.rs`; what is checked here is that the menu really does ask for it.
    let system = menu_logfont();
    let ours = tray::menu_item_logfont().expect("SPI_GETNONCLIENTMETRICS must answer");

    println!(
        "lfMenuFont: height={} weight={} charset={:?} quality={:?} -> quality={:?}",
        system.lfHeight, system.lfWeight, system.lfCharSet, system.lfQuality, ours.lfQuality
    );

    // ⚠ ClearType since решение 88 and grey coverage before it — see `theme::smoothed_logfont`
    // for why the mode moved. The tray menu rides along by construction: it goes through the
    // one function that names a mode, which is the whole point of there being only one.
    assert_eq!(
        ours.lfQuality, CLEARTYPE_QUALITY,
        "the face of the menu asks for the smoothing this program names for itself"
    );
    assert_ne!(
        system.lfQuality, ours.lfQuality,
        "and that is a change: the system's own menu face leaves the mode to the machine"
    );

    // Everything else is the system's, field for field — the same type face at the same
    // size and weight, so nothing a person sees moves by a pixel.
    assert_eq!(
        (
            ours.lfHeight,
            ours.lfWidth,
            ours.lfEscapement,
            ours.lfOrientation,
            ours.lfWeight,
        ),
        (
            system.lfHeight,
            system.lfWidth,
            system.lfEscapement,
            system.lfOrientation,
            system.lfWeight,
        )
    );
    assert_eq!(
        (
            ours.lfItalic,
            ours.lfUnderline,
            ours.lfStrikeOut,
            ours.lfCharSet,
            ours.lfOutPrecision,
            ours.lfClipPrecision,
            ours.lfPitchAndFamily,
        ),
        (
            system.lfItalic,
            system.lfUnderline,
            system.lfStrikeOut,
            system.lfCharSet,
            system.lfOutPrecision,
            system.lfClipPrecision,
            system.lfPitchAndFamily,
        )
    );
    assert_eq!(
        ours.lfFaceName, system.lfFaceName,
        "the type face itself is the system's"
    );
}

/// `lfMenuFont` of `SPI_GETNONCLIENTMETRICS`, read here rather than through the crate — the
/// face the system says a menu is set in, which is what the crate's answer is compared with.
fn menu_logfont() -> LOGFONTW {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: u32::try_from(size_of::<NONCLIENTMETRICSW>()).expect("the size fits in a u32"),
        ..Default::default()
    };

    // SAFETY: `metrics` is a live local of this frame whose `cbSize` describes it, which is
    // what the call checks before writing into the pointer; the pointer is not kept, and the
    // zero update-flags ask for no broadcast.
    unsafe {
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some((&raw mut metrics).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .expect("SPI_GETNONCLIENTMETRICS must answer");

    metrics.lfMenuFont
}

/// `SM_CXMENUCHECK`, the width of the check column of a menu entry.
fn check_column() -> i32 {
    // SAFETY: reads a system-wide metric, takes no pointer and touches no memory of ours.
    let metric = unsafe { GetSystemMetrics(SM_CXMENUCHECK) };

    if metric > 0 { metric } else { 16 }
}

// ---------------------------------------------------------------------------------------
// FR-90, FR-81, FR-83 — the tray itself
// ---------------------------------------------------------------------------------------

#[test]
fn the_icon_is_loaded_at_the_size_the_system_asks_for() {
    let window = TestWindow::new();
    let home = TestDir::new("icon_size");
    let tray = install(&window, &home);

    // SAFETY: `GetSystemMetrics` reads a system-wide value and touches no memory of ours.
    let expected = unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) };

    println!("SM_CXSMICON={} SM_CYSMICON={}", expected.0, expected.1);
    println!("icon requested at {:?}", tray.icon_size());

    assert_eq!(
        tray.icon_size(),
        expected,
        "FR-90: the size comes from SM_CXSMICON and is not hard-wired"
    );
}

/// **Задача Т-33а-1, находка глазом пользователя: «размытая иконка в уведомлении».**
///
/// Причина, измеренная приборами `значок-резкость.ps1` и `красное-до-1-значок.log`: шар
/// уведомления оболочка рисует размером `SM_CXICON` (32 px при 96 DPI), а `announce` отдавал ей
/// значок области уведомлений — `SM_CXSMICON`, 16 px. Растянуть 16 в 32 без потери резкости
/// нельзя, и это арифметика, а не мнение.
///
/// Тест держит обе половины лечения: величина берётся у системы, а не написана числом, и она
/// не меньше малой — иначе «большой значок» перестал бы быть большим и никто бы не заметил.
#[test]
fn the_balloon_icon_is_the_large_metric_and_not_the_small_one() {
    // SAFETY: `GetSystemMetrics` reads a system-wide value and touches no memory of ours.
    let expected = unsafe { (GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)) };

    println!("SM_CXICON={} SM_CYICON={}", expected.0, expected.1);
    println!("large_icon_size()={:?}", tray::large_icon_size());
    println!("small_icon_size()={:?}", tray::small_icon_size());

    assert_eq!(
        tray::large_icon_size(),
        expected,
        "Т-33а-1: размер значка шара идёт от SM_CXICON и не написан числом"
    );

    assert!(
        tray::large_icon_size().0 > tray::small_icon_size().0,
        "Т-33а-1: большой значок обязан быть больше малого — иначе лечения нет"
    );
}

/// **Та же задача, вторая половина: чем именно `announce` просит большой значок.**
///
/// `NIIF_USER` без `NIIF_LARGE_ICON` означает «возьми `hIcon` записи», а это и есть малый
/// значок трея. Оба имени обязаны стоять рядом в `announce`, и `hBalloonIcon` обязан
/// заполняться — без него флаг просит несуществующее и оболочка снова берёт малый.
///
/// Нечувствителен к концам строк по той же причине, что и соседи.
#[test]
fn the_balloon_asks_for_the_large_icon_by_name() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    let announce = source
        .split_once("fn announce(&mut self, title: &str, body: &str) {")
        .expect("announce must be there")
        .1
        .split_once("\n    /// ")
        .map_or_else(String::new, |(body, _)| body.to_owned());

    println!("длина тела announce: {} знаков", announce.len());

    for needle in ["NIIF_LARGE_ICON", "hBalloonIcon", "NIIF_USER"] {
        assert!(
            announce.contains(needle),
            "Т-33а-1: `{needle}` обязан стоять в `announce` — без него шар снова берёт малый \
             значок"
        );
    }

    // NFR-13: отказ загрузки большого значка не должен отменять само уведомление.
    assert!(
        announce.contains("None => NIIF_USER"),
        "NFR-13: без большого значка уведомление уходит как прежде, а не молчит"
    );

    // И набор либо целиком есть, либо целиком отсутствует — иначе шар показал бы «активна»
    // большим значком, а «пауза» малым.
    assert_eq!(
        source.matches("struct BalloonIcons {").count(),
        1,
        "набор больших значков — один тип, и он один"
    );
}

#[test]
fn the_icon_reaches_the_notification_area() {
    let window = TestWindow::new();
    let home = TestDir::new("add");
    let tray = install(&window, &home);

    println!(
        "NIM_ADD calls={} icon present={}",
        tray.add_calls(),
        tray.icon_present()
    );

    assert_eq!(
        tray.add_calls(),
        1,
        "installation issues exactly one NIM_ADD"
    );
    assert!(
        tray.icon_present(),
        "Shell_NotifyIcon(NIM_ADD) must have succeeded with the shell running"
    );
}

#[test]
fn taskbar_created_adds_the_icon_a_second_time() {
    let window = TestWindow::new();
    let home = TestDir::new("taskbar_created");
    let mut tray = install(&window, &home);

    let message = tray.taskbar_created_message();
    assert_ne!(message, 0, "RegisterWindowMessageW must have succeeded");
    assert_eq!(tray.add_calls(), 1);

    // FR-81: this is the message the shell broadcasts when it restarts.
    let reaction = tray.handle_message(message, WPARAM(0), LPARAM(0));

    println!(
        "TaskbarCreated = {message}, reaction {reaction:?}, NIM_ADD calls now {}",
        tray.add_calls()
    );

    assert_eq!(reaction, Reaction::Handled(LRESULT(0)));
    assert_eq!(tray.add_calls(), 2, "FR-81: the icon is added again");
    assert!(
        tray.icon_present(),
        "and it is back in the notification area"
    );
}

#[test]
fn switching_the_state_reaches_the_icon_and_the_file() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("toggle");
    let mut tray = install(&window, &home);

    assert!(
        tray.enabled(),
        "section 7 has `general.enabled` default to true"
    );

    tray.toggle_state();

    assert!(!tray.enabled());
    let after_first = fs::read_to_string(home.config()).expect("the file must have been written");
    println!("--- after the first switch ---\n{after_first}");
    assert!(after_first.contains("enabled = false"));

    tray.toggle_state();

    assert!(tray.enabled());
    let after_second = fs::read_to_string(home.config()).expect("the file must still be there");
    assert!(after_second.contains("enabled = true"));

    // The state that was written is the state that comes back.
    let (config, outcome) = settings::read_or_default(&home.config());
    assert!(outcome.is_ok());
    assert!(config.general.enabled);
}

/// ⭐ **Row 9 of the FR-10 table — «Приостановка программы пользователем … полный сброс +
/// обнуление памяти», task Т-13-7.**
///
/// # What was wrong
///
/// The row had no implementation. `Tray::toggle_state` changed the icon and the configuration,
/// `hook::set_active` was a single atomic store, and `buffer::reset` was called from
/// `app::park_buffer` and from nowhere else in the whole product — the audit of 2026-08-24,
/// direction *tests*. The coverage hole hid it: `tests\buffer.rs` claimed this row in a comment
/// and drove `recorder.reset()` by hand.
///
/// ⚠ **What the verdict of that finding narrows, and this test says out loud.** For this row the
/// wire is a **second** guarantee and not the only one: every real road to the menu entry crosses
/// a flush of its own — the tray icon is clicked with a mouse button (Raw Input, FR-13) or
/// reached with `Win+B`, the arrows and `Enter` (boundary keys of rows 1 and 3) — so the ring is
/// in practice already empty when the state moves, and a suspended program records nothing into
/// it either. For row 8, the session lock, nothing of the sort covers it and the wire is the
/// whole of the repair; that half is `tests\watchdog.rs`. This test is about the letter of FR-10
/// being met on purpose rather than by the side effect of somebody else's mechanism.
///
/// # Why the route is driven in two legs
///
/// The reason `the_session_lock_wipes_the_ring_and_the_unlock_does_not` gives at length: the ask
/// travels to the input thread by `PostMessageW`, and a test binary has no input window for it to
/// land at, so the send side is read as a count (`Counters::wipe_requests`) and the receive side
/// is then driven directly at the input window. Both halves of criterion 6 are here — the ring is
/// empty **and** the backing array is zeroed.
#[test]
fn the_pause_from_the_tray_asks_for_the_wipe_of_row_nine() {
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("pause-wipe");
    let mut tray = install(&window, &home);

    // `INPUT_WINDOW` — the cell `watchdog::is_input_window` reads, so that the far leg can be
    // driven. ⚠ No `WM_TIMER` and no `WM_APP_REHOOK` is sent while it is published: either would
    // reach `reinstall_hook` and install a live `WH_KEYBOARD_LL` hook in this process.
    let liveness =
        watchdog::start_liveness_timer(window.handle).expect("the liveness timer must start");

    buffer::install_recorder(Recorder::with_capacity(16));

    for time in [20_000, 20_001, 20_002] {
        press_into_the_buffer(time);
    }

    assert_eq!(buffer::len(), 3, "a half-typed word is in the ring");
    assert_eq!(
        non_zero_slots(),
        Some(3),
        "the positive control of SEC-02: the slots really do hold something now"
    );

    assert!(
        tray.enabled(),
        "section 7 has `general.enabled` default to true"
    );

    // ----- The user suspends the program -----
    let before_pause = watchdog::counters().wipe_requests;

    tray.toggle_state();

    assert!(!tray.enabled(), "FR-90: the program is suspended");
    assert_eq!(
        watchdog::counters().wipe_requests - before_pause,
        1,
        "FR-10 row 9: the pause asks the input thread for the wipe"
    );
    assert_eq!(
        buffer::len(),
        3,
        "the UI thread emptied nothing itself — the ring belongs to another thread"
    );

    // ----- The input thread answers -----
    assert_eq!(
        watchdog::handle_watchdog_message(window.handle, WM_APP_WIPE, WPARAM(0)),
        Some(LRESULT(0))
    );

    assert_eq!(buffer::len(), 0, "полный сброс");
    assert_eq!(
        non_zero_slots(),
        Some(0),
        "обнуление памяти — SEC-02, read off the backing array and not off the length"
    );

    // ----- Resuming is not a row of the table -----
    //
    // The mirror of `hook::set_active`, which asks for a fresh `CapsLock` on the opposite edge
    // (task T-13-4). FR-10 names «приостановка» and not its undoing, and there is nothing left in
    // the ring for a resumption to protect anyway.
    for time in [21_000, 21_001] {
        press_into_the_buffer(time);
    }

    let before_resume = watchdog::counters().wipe_requests;

    tray.toggle_state();

    assert!(tray.enabled(), "FR-90: the program is armed again");
    assert_eq!(
        watchdog::counters().wipe_requests - before_resume,
        0,
        "the resumption is not a flush rule and must not throw a word away"
    );
    assert_eq!(
        buffer::len(),
        2,
        "and what was typed while it was suspended is still there"
    );

    assert!(!hook::is_installed(), "no test here installs a hook");

    buffer::uninstall();
    drop(liveness);
}

/// One keystroke stamped `time` into the typing buffer of this thread — task Т-13-7.
///
/// The shape `tests\watchdog.rs` and `tests\buffer.rs` already use; `0x41` is `A`, an ordinary
/// character key, so the stroke is stored rather than treated as a flush of rows 1, 3 or 4.
fn press_into_the_buffer(time: u32) {
    buffer::record(KeyEvent {
        vk: 0x41,
        edge: Edge::Down,
        extra_info: 0,
        scan: 0x1E,
        flags: 0,
        time,
    });
}

/// How many slots of the backing array of this thread's buffer are not zero — task Т-13-7.
///
/// The SEC-02 reading of `tests\buffer.rs`, «явно перезаписывается нулями, а не просто помечается
/// пустым»: the whole array is examined, the free slots outside the live window included, so a
/// ring that was merely marked empty would fail here.
fn non_zero_slots() -> Option<usize> {
    buffer::with(|recorder| {
        recorder
            .slots()
            .iter()
            .filter(|slot| **slot != Stroke::ZEROED)
            .count()
    })
}

/// **Criterion 5 of task T-13-9, the half that is about the file.** A forged
/// `WM_APP_FAIL_SAFE` creates no configuration.
///
/// The audit's finding was not that the program could be suspended — it was that being
/// suspended by a stranger's message **wrote `general.enabled = false` to the disk**, because
/// the only way the tray offers to the "приостановлена" icon is `Tray::toggle_state` and that
/// method saves. So the measurement here is the directory, not an intention: `Tray::install_at`
/// reads and never writes, so `config.toml` does not exist when the message arrives, and a
/// write of any kind would make it appear.
///
/// ⚠ **What this measures and what it does not.** `tray::with_tray` answers `None` on every
/// thread that has no tray of its own, and a test thread is one of those, so the arm could not
/// have reached *this* tray even without the gate — the gate's own effect is measured in
/// `tests\hook.rs`, on `hook::is_active`, which is the half that was never thread-bound. What
/// this test adds is the fact the audit asked for, measured the way the task asks for it: after
/// a hundred forged messages the folder is still empty. The `toggle_state` at the end is the
/// positive control — it shows that a write really would have been seen.
#[test]
fn a_forged_fail_safe_message_leaves_no_configuration_behind() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("forged-fail-safe");
    let mut tray = install(&window, &home);

    assert!(
        !hook::fail_safe(),
        "no callback of this process has panicked four times running, so the flag is down — \
         which is the state a forgery arrives in"
    );
    assert!(tray.enabled(), "and the icon says «активна»");
    assert_eq!(
        home.entries(),
        Vec::<String>::new(),
        "installing the tray reads the configuration and writes nothing"
    );

    let active_before = hook::is_active();

    for _ in 0..100 {
        let answered = hook::handle_input_message(hook::WM_APP_FAIL_SAFE, WPARAM(0), LPARAM(0));

        assert_eq!(
            answered,
            Some(LRESULT(0)),
            "the message stays one of ours: the list of `handle_input_message` is closed and \
             explicit (SEC-05), and a gate that refused it would hand it to DefWindowProcW"
        );
    }

    println!(
        "after 100 forged WM_APP_FAIL_SAFE: entries={:?} enabled={} hook::is_active={}",
        home.entries(),
        tray.enabled(),
        hook::is_active()
    );

    assert_eq!(
        home.entries(),
        Vec::<String>::new(),
        "SEC-05: a forged message must not write the configuration"
    );
    assert!(tray.enabled(), "and must not move the icon");
    assert_eq!(
        hook::is_active(),
        active_before,
        "and must not disarm the hook"
    );

    // The positive control: the same folder, watched the same way, does see a write.
    tray.toggle_state();

    println!("after one genuine toggle: entries={:?}", home.entries());

    assert_eq!(home.entries(), vec![CONFIG_FILE_NAME.to_owned()]);
    assert!(
        fs::read_to_string(home.config())
            .expect("the file must be readable")
            .contains("enabled = false")
    );
}

// ---------------------------------------------------------------------------------------
// Task T-13-6 — the tray acts on the outcome of the read
//
// The audit of 2026-08-24 found `install_at` writing «let _ = outcome;» and `save_config`
// writing the file unconditionally underneath it, so a configuration this build could not
// read, or one a newer build wrote, was replaced by defaults at the first toggle or at the
// shutdown FR-83 guarantees. These three tests are the three answers.
//
// ⚠ Every one of them installs the tray with `Tray::install_at` on a `TestDir` under `%TEMP%`,
// as every test in this file does. `Tray::install` — the one entry point that calls
// `settings::default_config_path`, and through it `%APPDATA%` — is never called from here.
// ---------------------------------------------------------------------------------------

/// **Acceptance point 5.** A malformed file is kept, byte for byte, before anything is written
/// over it.
///
/// The comparison is of bytes and not of length on purpose. Section 7 leaves the file editable
/// by hand and FR-92 gives `[buffer] capacity` no field in the dialog, so editing it by hand is
/// the only way to set one — which means the file holds a person's typing, comments and line
/// endings included. A copy that re-rendered "the part that parsed" would be the very loss this
/// answers: the sections below the stray bracket are exactly the ones the parser never reached.
#[test]
fn a_malformed_configuration_is_kept_before_anything_is_written_over_it() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("malformed");

    // CRLF, a comment in Russian, a value in Russian, and a whole section the parser stops
    // before ever seeing. Nothing here survives a "write back what we understood".
    let original = "schema_version = 2\r\n\
                    # не трогать\r\n\
                    \r\n\
                    [general\r\n\
                    enabled = = true\r\n\
                    \r\n\
                    [exclusions]\r\n\
                    processes = [\"мой-редактор.exe\"]\r\n";
    fs::write(home.config(), original).expect("the malformed file must be writable");

    let mut tray = install(&window, &home);

    // The unreadable file is still untouched: reading decides nothing, saving does.
    assert_eq!(home.entries(), [CONFIG_FILE_NAME]);
    assert!(
        tray.enabled(),
        "the defaults of section 7 came back, which is what keeps the program running"
    );

    tray.toggle_state();

    let aside = settings::quarantine_path_for(&home.config());
    let kept = fs::read(&aside).expect("the malformed file must have been kept");

    println!(
        "{} bytes written by hand, {} bytes kept in {}",
        original.len(),
        kept.len(),
        aside.display()
    );

    assert_eq!(
        kept,
        original.as_bytes(),
        "the kept copy is not byte for byte what the person had"
    );

    // And a valid new configuration is beside it, carrying the switch that was just made.
    let (config, outcome) = settings::read_or_default(&home.config());
    assert!(
        outcome.is_ok(),
        "the new file must parse: {:?}",
        outcome.err()
    );
    assert!(
        !config.general.enabled,
        "FR-90: the switch reached the file"
    );
    assert_eq!(config.schema_version, settings::CURRENT_SCHEMA_VERSION);

    // Two files and no litter — the temporary of the atomic write is gone.
    assert_eq!(home.entries(), ["config.toml", "config.toml.bad"]);
}

/// **Acceptance point 6.** A file from a newer build is not written to once, and that includes
/// the shutdown of FR-83.
///
/// All four ways to a save are driven here — `toggle_state`, `set_autostart`, `replace_config`
/// and `shut_down` — because a ban that holds on three of them is not a ban. The state still
/// changes; it changes in memory, which is what a session on a file this build must not damage
/// looks like from the inside.
#[test]
fn a_configuration_from_a_newer_build_is_not_written_to_once() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("from_future");

    let original = "schema_version = 99\r\n\
                    \r\n\
                    [general]\r\n\
                    enabled = false\r\n\
                    \r\n\
                    [something_added_in_schema_99]\r\n\
                    setting = \"kept\"\r\n";
    fs::write(home.config(), original).expect("the newer file must be writable");

    let before = fs::read(home.config()).expect("the newer file must be readable");
    let mut tray = install(&window, &home);

    assert!(
        !tray.enabled(),
        "the file was read: `enabled = false` came out of it"
    );

    // 1 of 4 — FR-90, «Приостановить».
    tray.toggle_state();
    assert!(
        tray.enabled(),
        "the state lives in memory and still changes"
    );
    assert_eq!(
        fs::read(home.config()).expect("the file must still be there"),
        before,
        "toggle_state wrote to a file it must not write to"
    );

    // 2 of 4 — FR-93, the check mark of the menu.
    tray.set_autostart(false);
    assert!(!tray.autostart());
    assert_eq!(
        fs::read(home.config()).expect("the file must still be there"),
        before,
        "set_autostart wrote to a file it must not write to"
    );

    // 3 of 4 — FR-92, «Применить» in the settings dialog.
    let mut replacement = tray.config().clone();
    replacement.buffer.capacity = 1024;
    replacement.general.language = settings::Language::En;
    tray.replace_config(replacement);
    assert_eq!(tray.config().buffer.capacity, 1024);
    assert_eq!(
        fs::read(home.config()).expect("the file must still be there"),
        before,
        "replace_config wrote to a file it must not write to"
    );

    // 4 of 4 — FR-83, the one path every exit of the program goes through.
    tray.shut_down();
    let after = fs::read(home.config()).expect("the file must still be there");

    println!(
        "{} bytes before the session, {} after it",
        before.len(),
        after.len()
    );

    assert_eq!(
        after, before,
        "FR-83 saved the configuration over a file it was required not to damage"
    );

    // Nothing was created beside it either: no `.bad`, no temporary of a half-finished write.
    assert_eq!(home.entries(), [CONFIG_FILE_NAME]);
}

/// **Acceptance point 7.** A readable file behaves exactly as it did before.
///
/// The two tests above change what happens to a file this build cannot use. This one is the
/// statement that they changed nothing else: an ordinary configuration is read, written back
/// on a toggle, keeps the fields the dialog never touches, and no copy is made of anything.
#[test]
fn a_readable_configuration_is_written_back_exactly_as_before() {
    // Task Т-13-7: `toggle_state` moves a process-wide counter now. See [`TOGGLE`].
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("readable");

    // `[buffer] capacity` is the field FR-92 gives no control for — the reason a person edits
    // this file by hand at all — so it is the right one to follow through a save.
    let original = "schema_version = 2\n\
                    \n\
                    [general]\n\
                    enabled = true\n\
                    \n\
                    [buffer]\n\
                    capacity = 512\n";
    fs::write(home.config(), original).expect("the configuration file must be writable");

    let mut tray = install(&window, &home);
    assert!(tray.enabled());
    assert_eq!(tray.config().buffer.capacity, 512);

    tray.toggle_state();

    let (config, outcome) = settings::read_or_default(&home.config());
    assert!(outcome.is_ok(), "the file must parse: {:?}", outcome.err());
    assert!(!config.general.enabled, "the switch reached the file");
    assert_eq!(
        config.buffer.capacity, 512,
        "a field the dialog does not show must survive a save"
    );

    assert_eq!(
        home.entries(),
        [CONFIG_FILE_NAME],
        "a readable file is not copied anywhere"
    );
}

/// **Т-29-2, сторож: «ОК» writes the file — always, and not only when something changed.**
///
/// Три сообщения о том, что смена языка «не доходит до диска» (вопросы 83.3 и 94.4, находка
/// м-Э29-1) all had the same shape: the file untouched, byte for byte and to the millisecond.
/// The obvious cause would have been a «write only when it changed» rule somewhere on the road
/// from the dialog to [`settings::write_to`]. **There is no such rule, and this test is what
/// keeps it that way** — the reports turned out to be readings of a broken instrument, and the
/// stand of Э29 measured all 296 «откуда → куда» cases writing the file.
///
/// The measurement is deliberately not «did the timestamp move»: the system clock ticks about
/// every 15.6 ms, so two writes can honestly share one. Instead the file on the disk is
/// **replaced by something else** between the two saves, and the question is whether
/// [`Tray::replace_config`] puts the configuration back. It can only do that by writing, and it
/// is handed a configuration equal to the one it already holds — so nothing in memory changed,
/// and the file changed all the same.
#[test]
fn a_save_writes_the_file_even_when_the_configuration_did_not_change() {
    let _turn = toggle_turn();
    let window = TestWindow::new();
    let home = TestDir::new("save-is-unconditional");

    let mut tray = install(&window, &home);

    // The first save: the language the person picked reaches the file.
    let mut chosen = tray.config().clone();
    chosen.general.language = Language::De;
    tray.replace_config(chosen.clone());

    let written = fs::read_to_string(home.config()).expect("the file must be readable");
    assert!(
        written.contains("language = \"de\""),
        "the language of «ОК» must reach the file: {written}"
    );

    // Now the file says something else, and the tray does not know it. A rule that skipped a
    // save because «nothing changed» would leave this text exactly where it is.
    let meanwhile = "schema_version = 1\n\n[general]\nlanguage = \"en\"\nenabled = false\n";
    fs::write(home.config(), meanwhile).expect("the file must be writable");

    // The same configuration again — not a different one. This is «ОК» pressed on a dialog
    // where the person changed nothing.
    tray.replace_config(chosen.clone());

    let (back, outcome) = settings::read_or_default(&home.config());

    assert!(outcome.is_ok(), "the file must parse: {:?}", outcome.err());
    assert_eq!(
        back.general.language,
        Language::De,
        "an unchanged configuration is still written: the file was put back"
    );
    assert!(
        back.general.enabled,
        "and it was written whole, not patched field by field"
    );
    assert_eq!(
        back.schema_version,
        settings::CURRENT_SCHEMA_VERSION,
        "with the stamp of this build, not the one the intervening text carried"
    );

    assert_eq!(
        home.entries(),
        [CONFIG_FILE_NAME],
        "a readable file is written in place and copied nowhere"
    );
}

// ---------------------------------------------------------------------------------------
// Task T-13-24 — «Применить» does not write an autostart the registry refused
//
// The audit of 2026-08-24 read the comment standing over the FR-93 half of `apply_settings`
// — «FR-93 first: if the registry refuses, the file is not made to claim otherwise» — and
// then the line under it: there was no early return and nothing in its place, so
// `Tray::replace_config` saved the very `general.autostart` the `Run` key had just refused.
// The file, the check mark of FR-91 and the dialog of FR-92 then all claimed an autostart the
// system does not have.
//
// The repair is **one field stepping back, not the operation being given up**: «Применить»
// carries everything the user changed in the dialog, and a refusal from the registry says
// nothing about the hotkey, the layouts or the exclusions. Both halves of that sentence are
// measured below, in one test, by the bytes in the folder.
//
// ⚠ **The registry is a seam here and never the real key.** `settings::set_autostart` writes —
// or deletes — the `HKCU\…\CurrentVersion\Run` value of whoever is running the tests (FR-93),
// which is why `tests\tray.rs` has never fired `CMD_AUTOSTART` with the gate open (task
// T-13-14 says so in its own block above). The refusing branch is nevertheless the whole
// point of this task, so `tray::apply_settings_via` takes the registry write as an argument
// and these tests hand in a closure. Every one of them reads `settings::autostart_value()`
// before and after and asserts the machine's own value never moved.
//
// The `%APPDATA%` rule of this file holds as everywhere: every tray is attached with
// `tray::attach_at` on a `TestDir` under `%TEMP%`.
//
// ⚠ `general.enabled` is deliberately left alone by all four tests. `app::publish_configuration`
// hands it to `hook::set_active`, which is process-wide, and the tests of this binary run on
// parallel threads — `a_forged_fail_safe_message_leaves_no_configuration_behind` reads
// `hook::is_active()` before and after its own work. Changing anything else is free; changing
// that one field would be reaching across into another test.
// ---------------------------------------------------------------------------------------

/// The configuration the user is supposed to have produced in the dialog: the autostart turned
/// the other way round, and four unrelated settings changed with it.
///
/// The four are chosen to land in four different sections of section 7 and to be visible in the
/// file as text, so that "everything else survived" is read out of the bytes rather than
/// inferred.
fn dialog_produced(from: &settings::Config, autostart: bool) -> settings::Config {
    let mut asked = from.clone();

    asked.general.autostart = autostart;
    asked.general.language = settings::Language::En;
    asked.hotkey.key = "F9".to_owned();
    asked.replacement.inter_event_delay_ms = 7;
    asked.exclusions.processes = vec!["мой-редактор.exe".to_owned()];

    asked
}

/// Asserts that the four unrelated changes of [`dialog_produced`] are all in `text`.
fn assert_the_other_changes_are_in(text: &str) {
    for needle in [
        "language = \"en\"",
        "key = \"F9\"",
        "inter_event_delay_ms = 7",
        "мой-редактор.exe",
    ] {
        assert!(
            text.contains(needle),
            "a refusal from the registry says nothing about the rest of the dialog: `{needle}` \
             must be in the file"
        );
    }
}

/// **Criterion 5 of task T-13-24, and both of its facts in one test.** The registry refuses,
/// and the file that is written carries the **previous** `general.autostart` together with
/// **every other** change the user made.
///
/// Measured by the content of `config.toml` — the text, and then the same text read back
/// through the product's own reader — because the finding was about what the file claims and
/// an intention read out of the source would not have caught it.
#[test]
fn a_refused_run_key_keeps_the_autostart_of_the_file_and_stores_every_other_change() {
    // Task Т-31-1: `dialog_produced` asks for `language = "en"`, and «Применить» now publishes
    // the locale — see [`LOCALE`].
    let _locale = locale_turn();
    let window = TestWindow::new();
    let home = TestDir::new("registry-refused");
    let _attached = attach_ui(&window, &home);

    // The `Run` value of whoever is running this, read and never written — FR-93.
    let registry_before = settings::autostart_value();

    let in_force = live(Tray::autostart);

    assert!(
        in_force,
        "section 7 has `general.autostart` default to true, and no file was read"
    );
    assert_eq!(
        home.entries(),
        Vec::<String>::new(),
        "attaching the tray reads the configuration and writes nothing"
    );

    // The user turned the autostart off and changed four other things beside it.
    let asked = dialog_produced(&live(|tray| tray.config().clone()), !in_force);

    assert!(
        asked.general.enabled,
        "`general.enabled` stays where it is — see the note at the head of this block"
    );

    let handed = Cell::new(None);

    tray::apply_settings_via(&asked, |wanted| {
        handed.set(Some(wanted));
        Err(WinError::from(ERROR_ACCESS_DENIED))
    });

    assert_eq!(
        handed.get(),
        Some(!in_force),
        "FR-93 is attempted first and with the value the user asked for — the file steps back \
         only because the registry said no, not instead of asking it"
    );

    let text = fs::read_to_string(home.config()).expect("the configuration must have been saved");

    println!("--- config.toml after a refused Run key ---\n{text}");

    // Fact one: the refused field did not reach the file.
    assert!(
        text.contains("autostart = true"),
        "the file must carry the autostart that is really in force"
    );
    assert!(
        !text.contains("autostart = false"),
        "and must not carry the one the registry refused — this is the finding"
    );

    // Fact two: everything else did.
    assert_the_other_changes_are_in(&text);

    // The same two facts through the product's own reader, so that they are facts about the
    // configuration and not about a substring.
    let (stored, outcome) = settings::read_or_default(&home.config());

    assert!(outcome.is_ok(), "the file must parse: {:?}", outcome.err());
    assert_eq!(
        stored.general.autostart, in_force,
        "the refused field is the one the file already had"
    );
    assert_eq!(stored.general.language, settings::Language::En);
    assert_eq!(stored.hotkey.key, "F9");
    assert_eq!(stored.replacement.inter_event_delay_ms, 7);
    assert_eq!(stored.exclusions.processes, vec!["мой-редактор.exe"]);

    // Memory agrees with the file, so the check mark of FR-91 shows what is true.
    assert_eq!(
        live(Tray::autostart),
        in_force,
        "the tray holds the value the file holds"
    );
    assert_eq!(live(|tray| tray.config().hotkey.key.clone()), "F9");

    // No litter, and the real registry was never asked anything.
    assert_eq!(home.entries(), vec![CONFIG_FILE_NAME.to_owned()]);
    assert_eq!(
        settings::autostart_value(),
        registry_before,
        "FR-93: the `Run` key of the person running this test is untouched"
    );
}

/// **Criterion 6 of task T-13-24.** The registry accepts, and everything is saved exactly as it
/// was before this task — the autostart included.
///
/// The positive control of the test above: it shows that the file really does follow the check
/// box when there is nothing to step back from, so the assertion up there is about the refusal
/// and not about a value that could never have been written anyway.
#[test]
fn an_accepted_run_key_stores_the_autostart_the_user_asked_for() {
    // Task Т-31-1 — see [`LOCALE`].
    let _locale = locale_turn();
    let window = TestWindow::new();
    let home = TestDir::new("registry-accepted");
    let _attached = attach_ui(&window, &home);

    let registry_before = settings::autostart_value();
    let in_force = live(Tray::autostart);

    assert!(in_force);

    let asked = dialog_produced(&live(|tray| tray.config().clone()), !in_force);
    let handed = Cell::new(None);

    tray::apply_settings_via(&asked, |wanted| {
        handed.set(Some(wanted));
        Ok(())
    });

    assert_eq!(handed.get(), Some(!in_force));

    let text = fs::read_to_string(home.config()).expect("the configuration must have been saved");

    println!("--- config.toml after an accepted Run key ---\n{text}");

    assert!(
        text.contains("autostart = false"),
        "an accepted write is the ordinary road, and the file follows the check box down it"
    );
    assert_the_other_changes_are_in(&text);

    assert_eq!(
        live(Tray::autostart),
        !in_force,
        "and the check mark of FR-91 follows it too"
    );

    assert_eq!(home.entries(), vec![CONFIG_FILE_NAME.to_owned()]);
    assert_eq!(
        settings::autostart_value(),
        registry_before,
        "FR-93: still nothing of the real `Run` key was touched — the seam is the whole of it"
    );
}

/// **Criterion 7 of task T-13-24.** The disagreement stays visible: the dialog goes on reading
/// the registry for itself.
///
/// The audit's own caveat, and it is existing behaviour this task must not break. The state
/// line of the dialog is formatted from [`settings::autostart_value`] — the registry — while
/// the check box is drawn from `general.autostart` — the file. They are read from two places on
/// purpose, so that a value somebody removed by hand is visible rather than merely wrong. After
/// a refused apply the two are exactly as independent as they were: the file was corrected, and
/// the registry answer the dialog prints is the machine's own and did not move.
#[test]
fn a_refused_apply_leaves_the_two_readings_of_fr93_independent() {
    // Task Т-31-1 — see [`LOCALE`].
    let _locale = locale_turn();
    let window = TestWindow::new();
    let home = TestDir::new("registry-disagreement");
    let _attached = attach_ui(&window, &home);

    let registry_before = settings::autostart_value();
    let registered_before = settings::autostart_registered();
    let in_force = live(Tray::autostart);

    let asked = dialog_produced(&live(|tray| tray.config().clone()), !in_force);

    tray::apply_settings_via(&asked, |_| Err(WinError::from(ERROR_ACCESS_DENIED)));

    println!(
        "after a refused apply: file/check-box autostart={} registry={:?} registered={}",
        live(Tray::autostart),
        settings::autostart_value(),
        settings::autostart_registered()
    );

    assert_eq!(
        settings::autostart_value(),
        registry_before,
        "the source the dialog's state line is formatted from is the registry, and this task \
         does not write to it"
    );
    assert_eq!(
        settings::autostart_registered(),
        registered_before,
        "nor does it change the answer that line prints"
    );

    // Two readings, two sources. The check box of the dialog and the check mark of FR-91 are
    // drawn from the configuration; the state line is formatted from the registry. This apply
    // corrected the first and asked nothing of the second, and the file the user is left with
    // says what the system says rather than what the check box was clicked to.
    assert_eq!(live(Tray::autostart), in_force);
    assert_eq!(
        fs::read_to_string(home.config())
            .expect("the configuration must have been saved")
            .contains("autostart = true"),
        in_force,
        "the file follows what is in force, and the dialog goes on showing the registry beside \
         it — the disagreement is visible rather than hidden"
    );
}

/// **The two mechanisms decide different questions — task T-13-24 against task T-13-6.**
///
/// This task decides **what** is stored; [`Tray::save_config`] decides **whether** the file may
/// be written at all. Driven together on the one configuration where the second says no: a file
/// from a newer schema, which puts the session into `SavePolicy::Forbidden` for good.
///
/// Neither shadows the other. The file is not touched — that is T-13-6's answer and it is
/// unchanged — and the configuration in memory carries the previous autostart with every other
/// change of the dialog, which is this task's answer and it is reached all the same.
#[test]
fn the_step_back_and_the_save_policy_of_t_13_6_answer_different_questions() {
    // Task Т-31-1 — see [`LOCALE`].
    let _locale = locale_turn();
    let window = TestWindow::new();
    let home = TestDir::new("refused-and-forbidden");

    // Schema 99: `read_or_default` answers `FromNewerSchema`, and nothing may be written this
    // session — not by a toggle, not by an apply, not by the shutdown of FR-83.
    let original = "schema_version = 99\r\n\
                    \r\n\
                    [general]\r\n\
                    enabled = true\r\n\
                    autostart = true\r\n\
                    \r\n\
                    [something_added_in_schema_99]\r\n\
                    setting = \"kept\"\r\n";
    fs::write(home.config(), original).expect("the newer file must be writable");

    let before = fs::read(home.config()).expect("the newer file must be readable");
    let attached = attach_ui(&window, &home);

    let registry_before = settings::autostart_value();
    let in_force = live(Tray::autostart);

    assert!(
        in_force,
        "the file was read: `autostart = true` came out of it"
    );

    let asked = dialog_produced(&live(|tray| tray.config().clone()), !in_force);

    tray::apply_settings_via(&asked, |_| Err(WinError::from(ERROR_ACCESS_DENIED)));

    println!(
        "under SavePolicy::Forbidden: entries={:?} autostart={} hotkey={}",
        home.entries(),
        live(Tray::autostart),
        live(|tray| tray.config().hotkey.key.clone())
    );

    // T-13-6's answer, unchanged.
    assert_eq!(
        fs::read(home.config()).expect("the file must still be there"),
        before,
        "«Применить» wrote to a file this build was told not to write to"
    );
    assert_eq!(
        home.entries(),
        vec![CONFIG_FILE_NAME.to_owned()],
        "and left no `.bad` and no temporary of a half-finished write beside it"
    );

    // This task's answer, reached all the same — in memory, which is where a session on a file
    // this build must not damage keeps its state.
    assert_eq!(
        live(Tray::autostart),
        in_force,
        "the refused field stepped back in memory too — the check mark of FR-91 does not lie \
         merely because the file is off limits"
    );
    assert_eq!(live(|tray| tray.config().hotkey.key.clone()), "F9");
    assert_eq!(
        live(|tray| tray.config().general.language),
        settings::Language::En
    );

    assert_eq!(settings::autostart_value(), registry_before);

    // The shutdown of FR-83 is still forbidden to write, and the tray is taken out here rather
    // than at the end of the scope so that the assertion below is about that and not about the
    // order the locals are dropped in.
    drop(attached);

    assert_eq!(
        fs::read(home.config()).expect("the file must still be there"),
        before,
        "FR-83 saved the configuration over a file it was required not to damage"
    );
}

/// **Criterion 8 of task T-13-24 — the comment and the code say the same thing.**
///
/// Swept over the source for the reason the two neighbouring sweeps of this file state: what a
/// test can drive is the behaviour, and what it cannot drive is the *shape*. The behaviour is
/// measured three tests up; this is the statement that the promise is still written where the
/// code keeps it, that the read of the value that steps back happens before the write that
/// would destroy it, and that exactly one field steps back.
///
/// Insensitive to line endings by construction — `.gitattributes` declares `* text=auto
/// eol=crlf`, so a fresh worktree holds this file in CRLF while the index holds LF.
#[test]
fn the_promise_over_the_registry_write_is_kept_by_the_code_under_it() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("tray.rs"),
    )
    .expect("src\\tray.rs must be readable")
    .replace("\r\n", "\n");

    let at = source
        .find("pub fn apply_settings_via(")
        .expect("apply_settings_via must be in this file");
    let body = &source[at..];
    let end = body.find("\n}").expect("a function closes with its brace");
    let body = &body[..end];

    println!("--- apply_settings_via ---\n{body}");

    // The promise, word for word as the audit quoted it.
    let promise = body
        .find("// FR-93 first: if the registry refuses, the file is not made to claim otherwise.")
        .expect("the promise of FR-93 must still stand over the registry write");
    let read = body
        .find("let in_force = with_tray(")
        .expect("the value that steps back must be read out of the tray");
    let refusal = body
        .find("if let Err(error) = write_run_key(")
        .expect("the registry answer must be examined — NFR-13");
    let step_back = body
        .find("stored.general.autostart = in_force;")
        .expect("and a refusal must put the previous value back");
    let save = body
        .find("tray.replace_config(stored")
        .expect("what is saved must be the corrected configuration and not the argument");

    assert!(
        read < refusal,
        "the previous value is read before anything is written, because `replace_config` is \
         what overwrites it"
    );
    assert!(
        promise < refusal && refusal < step_back && step_back < save,
        "the promise stands over the refusal, the step back is inside it, and the save comes \
         after both"
    );

    // One field, and the operation is not abandoned. This is what the repair is.
    assert_eq!(
        body.matches("stored.general.").count(),
        1,
        "exactly one field of the configuration steps back"
    );
    assert!(
        !body.contains("return"),
        "and the rest of the dialog is not thrown away with it — a bare `return` here would \
         lose the hotkey, the layouts and the exclusions the same «Применить» carries"
    );

    // The twin does return, and that is not a contradiction: its whole operation is the one
    // field, so skipping the field and giving up the operation are the same act.
    let at = source
        .find("fn toggle_autostart() {")
        .expect("toggle_autostart must be in this file");
    let twin = &source[at..];
    let end = twin.find("\n}").expect("a function closes with its brace");
    let twin = &twin[..end];

    assert!(
        twin.contains("app::report_non_critical(\"RegSetValueExW\", &error);")
            && twin.contains("return;"),
        "the twin of FR-93 keeps the rule the way it always has"
    );

    // And the product itself goes through the seam with the real registry write in its hand —
    // the seam is a way in for the tests, never a way round FR-93 for «Применить».
    let at = source
        .find("fn apply_settings(config: &Config) {")
        .expect("apply_settings must be in this file");
    let entry = &source[at..];
    let end = entry.find("\n}").expect("a function closes with its brace");
    let entry = &entry[..end];

    println!("--- apply_settings ---\n{entry}");

    assert!(
        entry.contains("apply_settings_via(config, settings::set_autostart)"),
        "«Применить» must hand in `settings::set_autostart`, which is the one function that \
         writes the `Run` key of FR-93"
    );
}

// ---------------------------------------------------------------------------------------
// FR-94, task Т-31-1 — the interface language of the running process
// ---------------------------------------------------------------------------------------
//
// Решение 99: the language chosen in the settings window takes effect **without a restart**.
// The one place it is published in a running process is [`tray::adopt_ui_language`], and the
// two callers of it are the start-up of `app` and «Применить» — one body, not two.
//
// ⚠ Every test in this block takes the turn of `LOCALE`: publishing a locale is process-wide
// and the tests of one binary run on parallel threads. So do the four tests of task T-13-24
// above, since this task made «Применить» publish the language as well.

/// **Criterion 3 of task Т-31-1.** A configuration accepted by «Применить» publishes its
/// `general.language` into the running process, and the very next menu is built in it.
///
/// The menu is the reader chosen deliberately: посылка П1 of the mandate says `Menu::build`
/// takes its labels out of the string table of the locale in force at the moment of building,
/// and this is the test that says it is so *after an apply* and not only at start-up.
#[test]
fn applying_a_configuration_publishes_its_language_into_the_running_process() {
    let _locale = product_strings(Language::Ru);

    let window = TestWindow::new();
    let home = TestDir::new("live-language");
    let _attached = attach_ui(&window, &home);

    assert_eq!(
        settings::ui_language(),
        Language::Ru,
        "the locale this test starts from"
    );

    let mut asked = live(|tray| tray.config().clone());
    asked.general.language = Language::De;

    // The registry answers yes: FR-93 is not what this test is about, and a real write into
    // `HKCU\…\Run` belongs to whoever is running the tests.
    tray::apply_settings_via(&asked, |_| Ok(()));

    assert_eq!(
        settings::ui_language(),
        Language::De,
        "«Применить» must publish the language of the configuration it accepted — решение 99"
    );

    let menu = Menu::build(true, true, NOT_FAIL_SAFE, NO_DIALOG, &NOTHING_PENDING)
        .expect("the menu must build");
    let labels: Vec<&str> = menu
        .items()
        .iter()
        .map(|item| item.label.as_str())
        .collect();

    println!("--- menu after «Применить» with language = de ---\n{labels:?}");

    assert_eq!(
        labels,
        [
            "Anhalten",
            "Einstellungen…",
            "Beim Anmelden starten",
            // Task Т-32-4: the permanent entry of FR-91's addition follows the locale like
            // every other line of the menu.
            "An den Autor schreiben…",
            "Über",
            "Beenden",
        ],
        "the menu built after the apply carries the labels of the new locale"
    );

    // Criterion 3 of task Т-31-1, third half — the tooltip of the icon follows too. It is a
    // string of the locale only since task Т-31-4, which is why the assertion lives here and
    // not in the commit that published the language: on that tree both words were Russian
    // literals whatever the locale said.
    let tip = tray::icon_tip(live(Tray::enabled));

    println!("--- tooltip after «Применить» with language = de ---\n{tip}");

    assert!(
        tip.starts_with("Lang Switcher — "),
        "the name of the product is not translated — decision on question 7"
    );
    assert!(
        !tip.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
        "the tooltip must have left Russian with the rest of the interface: «{tip}»"
    );
    assert_eq!(tip, "Lang Switcher — aktiv");

    // Back to where the rest of this binary expects to find the process.
    settings::set_ui_language(Language::Ru);
}

/// **Criterion 1 of task Т-31-1 — one body, not two.**
///
/// The mandate allows the start-up publication to stay where it is *or* to go through the same
/// function, and requires that there not be two of them. This sweep says which of the two the
/// repair chose and holds it there: `settings::set_ui_language` is called from exactly one
/// place in `src\`, and that place is [`tray::adopt_ui_language`].
///
/// ⚠ Э22's trap — a sweep that finds its own needle. It reads `src\` only; the needles are
/// written here, in `tests\`, and this file is never among the files read.
#[test]
fn the_language_of_the_running_process_is_published_from_exactly_one_place() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut callers = Vec::new();

    for entry in fs::read_dir(&src).expect("src\\ must be readable") {
        let path = entry.expect("a directory entry").path();

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let text = fs::read_to_string(&path)
            .expect("a source file must be readable")
            .replace("\r\n", "\n");

        // The definition itself lives in `settings.rs` and is not a call.
        for line in text.lines() {
            let line = line.trim();

            if line.starts_with("//") || line.starts_with("///") {
                continue;
            }

            if line.contains("set_ui_language(") && !line.contains("pub fn set_ui_language") {
                callers.push(format!(
                    "{}: {line}",
                    path.file_name().expect("a file name").to_string_lossy()
                ));
            }
        }
    }

    println!("--- calls of set_ui_language in src\\ ---\n{callers:#?}");

    assert_eq!(
        callers.len(),
        1,
        "решение 99 gives the running process one publication of the interface locale; a \
         second call is a second answer to «на каком языке программа»"
    );
    assert!(
        callers[0].starts_with("tray.rs:"),
        "and it lives in `tray::adopt_ui_language`, which is what both the start-up and \
         «Применить» go through"
    );

    let tray_source = fs::read_to_string(src.join("tray.rs"))
        .expect("src\\tray.rs must be readable")
        .replace("\r\n", "\n");

    let at = tray_source
        .find("pub fn adopt_ui_language()")
        .expect("adopt_ui_language must be in this file");
    let body = &tray_source[at..];
    let end = body.find("\n}").expect("a function closes with its brace");
    let body = &body[..end];

    println!("--- adopt_ui_language ---\n{body}");

    assert!(
        body.contains("settings::set_ui_language("),
        "the one body publishes the locale"
    );
    assert!(
        body.contains("refresh_icon()"),
        "…and brings the tooltip of the icon with it — NIM_MODIFY, решение 99.2"
    );

    let app_source = fs::read_to_string(src.join("app.rs"))
        .expect("src\\app.rs must be readable")
        .replace("\r\n", "\n");

    assert!(
        app_source.contains("crate::tray::adopt_ui_language()"),
        "the start-up publication goes through the same body"
    );

    let at = tray_source
        .find("pub fn apply_settings_via(")
        .expect("apply_settings_via must be in this file");
    let apply = &tray_source[at..];
    let end = apply.find("\n}").expect("a function closes with its brace");

    assert!(
        apply[..end].contains("adopt_ui_language()"),
        "and so does «Применить», after the configuration has been stored and written"
    );
}

// ---------------------------------------------------------------------------------------
// FR-90, решение 99.2, task Т-31-4 — the tooltip of the icon
// ---------------------------------------------------------------------------------------

/// The fourteen locales, and the script each of them must be written in.
///
/// The alphabets are ranges rather than names because that is what can be measured: a
/// translation that quietly stayed Russian, or came back as question marks, is caught by asking
/// which block its characters live in. The same instrument Э28 built for the string tables,
/// pointed at the one string those sweeps could not see.
const TIP_LOCALES: [(&str, Language, char, char); 14] = [
    ("ru", Language::Ru, '\u{0400}', '\u{04FF}'),
    ("en", Language::En, '\u{0041}', '\u{024F}'),
    ("uk", Language::Uk, '\u{0400}', '\u{04FF}'),
    ("de", Language::De, '\u{0041}', '\u{024F}'),
    ("fr", Language::Fr, '\u{0041}', '\u{024F}'),
    ("es", Language::Es, '\u{0041}', '\u{024F}'),
    ("pt", Language::Pt, '\u{0041}', '\u{024F}'),
    ("it", Language::It, '\u{0041}', '\u{024F}'),
    ("pl", Language::Pl, '\u{0041}', '\u{024F}'),
    ("cs", Language::Cs, '\u{0041}', '\u{024F}'),
    ("tr", Language::Tr, '\u{0041}', '\u{024F}'),
    ("el", Language::El, '\u{0370}', '\u{03FF}'),
    ("he", Language::He, '\u{0590}', '\u{05FF}'),
    ("ar", Language::Ar, '\u{0600}', '\u{06FF}'),
];

/// **Criteria of task Т-31-4.** The tooltip carries `APP_NAME` untranslated, a state word in the
/// script of the locale, and fits the field `write_tip` copies into.
///
/// Красное «до» this test was written against: the tooltip of `he` — of every locale — held the
/// Russian literals «активна» / «приостановлена», hard-wired in `tray.rs` past the string tables
/// (`scratchpad-Э31\красное-до-4-подсказка.log`).
#[test]
fn the_tooltip_of_the_icon_is_written_in_the_locale_of_the_interface() {
    let _locale = product_strings(Language::Ru);

    // ⚠ The field of `NOTIFYICONDATAW` holds 128 UTF-16 units including the terminator, so
    // `write_tip` copies at most 127 — посылка П5 of the mandate, measured on the drafted
    // strings before they were written (`scratchpad-Э31\посылки-п5.log`, longest 30).
    const LIMIT: usize = 127;

    let mut longest = 0;
    let mut measured = 0;

    for (tag, language, first, last) in TIP_LOCALES {
        settings::set_ui_language(language);

        for enabled in [true, false] {
            let tip = tray::icon_tip(enabled);
            let units = tip.encode_utf16().count();

            measured += 1;
            longest = longest.max(units);

            println!("{tag} / enabled = {enabled}: «{tip}» — {units} UTF-16 units");

            let state = tip.strip_prefix("Lang Switcher — ").unwrap_or_else(|| {
                panic!("{tag}: the tooltip must be «Lang Switcher — состояние»")
            });

            assert!(
                !state.trim().is_empty(),
                "{tag}: the state word must not be empty — an absent string row comes back blank"
            );
            assert!(
                state.chars().any(|c| (first..=last).contains(&c)),
                "{tag}: «{state}» carries no character of the script this locale is written in \
                 — a translation that stayed in another language, or came back as question marks"
            );

            // The красное of the mandate, said in the direction it was said: no locale but the
            // two Cyrillic ones may hold Cyrillic.
            if !matches!(tag, "ru" | "uk") {
                assert!(
                    !state
                        .chars()
                        .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
                    "{tag}: «{state}» holds Cyrillic — this is the defect решение 99.2 names, \
                     the tooltip hard-wired in Russian past the string tables"
                );
            }

            assert!(
                units <= LIMIT,
                "{tag}: {units} UTF-16 units against the {LIMIT} `write_tip` copies — the \
                 tooltip would be cut"
            );
        }
    }

    println!("--- {measured} tooltips measured, longest {longest} of {LIMIT} units ---");

    assert_eq!(
        measured,
        TIP_LOCALES.len() * 2,
        "fourteen locales in two states — «no defects» out of no readings is the cheapest lie"
    );

    // The two states really differ. Without this, one string used for both would pass every
    // assertion above.
    for (tag, language, _, _) in TIP_LOCALES {
        settings::set_ui_language(language);

        assert_ne!(
            tray::icon_tip(true),
            tray::icon_tip(false),
            "{tag}: «активна» and «приостановлена» are two words, not one"
        );
    }

    settings::set_ui_language(Language::Ru);
}

#[test]
fn the_session_may_not_be_held_up_and_ends_in_the_cleanup() {
    let window = TestWindow::new();
    let home = TestDir::new("endsession");
    let mut tray = install(&window, &home);

    // FR-83: `WM_QUERYENDSESSION` answers TRUE. A program has no business refusing.
    let query = tray.handle_message(WM_QUERYENDSESSION, WPARAM(0), LPARAM(0));
    println!("WM_QUERYENDSESSION -> {query:?}");
    assert_eq!(query, Reaction::Handled(LRESULT(1)));

    assert!(
        tray.icon_present(),
        "nothing is cleaned up on the query alone"
    );
    assert!(
        !home.config().exists(),
        "and nothing is written on the query alone either"
    );

    // FR-83: `WM_ENDSESSION` with TRUE really is the end.
    let end = tray.handle_message(WM_ENDSESSION, WPARAM(1), LPARAM(0));
    println!("WM_ENDSESSION(TRUE) -> {end:?}");
    assert_eq!(end, Reaction::Handled(LRESULT(0)));

    assert!(!tray.icon_present(), "FR-83: the icon is removed");
    assert!(home.config().exists(), "FR-83: the configuration is saved");
}

#[test]
fn a_cancelled_session_end_changes_nothing() {
    let window = TestWindow::new();
    let home = TestDir::new("endsession_false");
    let mut tray = install(&window, &home);

    let end = tray.handle_message(WM_ENDSESSION, WPARAM(0), LPARAM(0));

    assert_eq!(end, Reaction::Handled(LRESULT(0)));
    assert!(tray.icon_present(), "the session was not ending after all");
    assert!(!home.config().exists());
}

#[test]
fn dropping_the_tray_runs_the_same_cleanup_as_the_session_end() {
    let home = TestDir::new("drop");

    {
        let window = TestWindow::new();
        let tray = install(&window, &home);
        assert!(tray.icon_present());
        assert!(!home.config().exists());
    }

    // This is acceptance point 16 in one assertion: the exit through the menu and the exit
    // through the FR-97 timeout both end in `Attachment::drop`, which drops the `Tray`,
    // which runs `Tray::shut_down` — the same function `WM_ENDSESSION` calls directly.
    assert!(
        home.config().exists(),
        "the cleanup path of FR-83 runs when the tray is dropped"
    );
}

#[test]
fn the_cleanup_of_fr_83_runs_once_however_often_it_is_asked_for() {
    let window = TestWindow::new();
    let home = TestDir::new("idempotent");
    let mut tray = install(&window, &home);

    tray.shut_down();
    let first = fs::read_to_string(home.config()).expect("the file must exist");

    // Deleting the file and asking again proves the second call did nothing rather than
    // merely doing the same thing twice.
    fs::remove_file(home.config()).expect("the file must be removable");

    tray.shut_down();
    tray.handle_message(WM_ENDSESSION, WPARAM(1), LPARAM(0));

    assert!(
        !home.config().exists(),
        "the cleanup must not run a second time"
    );
    assert!(first.contains("enabled = true"));
}

#[test]
fn messages_the_tray_does_not_know_are_left_to_the_default_procedure() {
    let window = TestWindow::new();
    let home = TestDir::new("sec05");
    let mut tray = install(&window, &home);

    // SEC-05: everything outside the explicit list is `Ignored`, which is what makes
    // `app::window_proc` hand it to `DefWindowProcW` unchanged. `WM_CLOSE`, the one message
    // `app` suppresses itself, must not be claimed here either.
    for message in [0x0000u32, 0x0010, 0x0002, 0x0111, 0x0100, 0x8000] {
        assert_eq!(
            tray.handle_message(message, WPARAM(0), LPARAM(0)),
            Reaction::Ignored,
            "message {message:#06x} is not the tray's"
        );
    }
}

// ---------------------------------------------------------------------------------------
// The about window — a MessageBoxW until task T-11-11 made it the dialog IDD_ABOUT
// ---------------------------------------------------------------------------------------

#[test]
fn the_version_of_the_about_box_comes_out_of_the_version_resource() {
    // The about window shows what `VERSIONINFO` says, not what `Cargo.toml` says — since
    // task T-11-11 the number travels `file_version` → `settings::show_about_dialog` →
    // the version line, but the reading is the same reading. The offset of
    // `VS_FIXEDFILEINFO` inside the resource is the one thing in that path that can be wrong
    // without any Win32 call failing, so it is checked against the bytes `rc.exe` really
    // produced rather than against a buffer built to match the code.
    let product = ProductImage::open();
    let bytes = product.resource(RT_VERSION, 1);

    let version = tray::parse_fixed_file_version(&bytes);

    println!(
        "RT_VERSION #1 is {} bytes; FILEVERSION = {version:?}",
        bytes.len()
    );

    // ⭐ **The number moves with every delivery now.** Решение 8 fixed `0.1.0` as the *starting*
    // version and it stood, unmoved, for thirty-two deliveries. The rule adopted on 2026-09-03
    // is «MINOR is the number of the delivery tag»: the build tagged `e36` is `0.36.0`, `e38` is
    // `0.38.0`, and PATCH exists for a second delivery under one tag. `Cargo.toml` is the single
    // reference and `tools\verify-version.ps1` is what makes the other nine places agree with
    // it — this assertion is the tenth reader, the only one that reads the bytes `rc.exe`
    // really produced, and the only one the instrument cannot see: it is raised **by hand**,
    // and that is what makes it the honest red before a version bump.
    assert_eq!(
        version,
        Some((0, 47, 0, 0)),
        "app.rc must declare FILEVERSION 0,47,0,0 — the delivery is `e47`"
    );

    // The test binary itself carries no resources, so the about window of *this* process
    // has no version to show — `file_version` says so instead of failing, and the dialog
    // shows the absence as a dash.
    assert_eq!(tray::file_version(), None);
}

#[test]
fn the_window_the_about_item_opens_ships_in_the_product_resources() {
    // FR-92а, task T-11-11: «О программе» is no longer a `MessageBoxW` — `show_about`
    // asks `DialogBoxParamW` for template 201 (`IDD_ABOUT`) of the running executable. A
    // missing template would cost nothing at build time — `embed-resource` links whatever
    // `app.rc` produced — and would fail at the moment the menu item is chosen, so the
    // presence of the template is pinned here, in the file that ships. What is *in* the
    // template — the elements, the styles, the strings of both locales — is the business
    // of `tests\settings.rs`.
    let product = ProductImage::open();
    let template = product.resource(RT_DIALOG, 201);

    println!("RT_DIALOG 201 is {} bytes", template.len());

    // A DIALOGEX template opens with version 1, signature 0xFFFF — the same first four
    // bytes the parser of tests\settings.rs demands.
    assert_eq!(
        (
            u16::from_le_bytes([template[0], template[1]]),
            u16::from_le_bytes([template[2], template[3]]),
        ),
        (1, 0xFFFF),
        "RT_DIALOG 201 must be a DIALOGEX template"
    );
}

// ---------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------

/// A hidden top-level window of the same shape as the UI window of section 6.1.
///
/// The class is the system's `STATIC` rather than one of ours: `app` registers its class
/// inside `run()`, which a test cannot call, and the tray does not care what class the
/// window it is installed on has — only that the window is alive and belongs to this thread.
struct TestWindow {
    handle: HWND,
}

impl TestWindow {
    fn new() -> Self {
        // SAFETY: `None` asks for the handle of the running executable, which cannot be
        // unloaded under us.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("the module handle");

        // SAFETY: `STATIC` is a system window class that always exists; the window name is
        // null, which asks for no title. No `lpParam` is passed, so the `WM_CREATE` the call
        // delivers carries no pointer of ours. The handle is destroyed exactly once, in
        // `Drop`, on this same thread — which is what `DestroyWindow` requires.
        let handle = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(HINSTANCE(module.0)),
                None,
            )
        }
        .expect("a hidden window must be creatable");

        Self { handle }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `CreateWindowExW` on this thread and is
        // destroyed exactly once — this type is neither `Copy` nor `Clone`.
        let _ = unsafe { DestroyWindow(self.handle) };
    }
}

/// The shipped `LangSwitcher.exe`, opened as a data file so its resources can be read.
///
/// This exists because of a real property of the build, not for convenience: `embed-resource`
/// links `app.rc` into the **binary** targets of the crate, so `LangSwitcher.exe` carries the
/// icons and the version block while the integration-test executables carry no resource
/// section at all — `LoadImageW` on this process fails with `0x80070714`, "the specified
/// image file does not contain a resource section". The icons under test are the ones that
/// ship, so the tests load them out of the file that ships.
struct ProductImage {
    module: HMODULE,
}

impl ProductImage {
    fn open() -> Self {
        Self {
            module: Self::load(),
        }
    }

    /// Maps the shipped binary for resource reading.
    fn load() -> HMODULE {
        // `cargo test` puts the test executables in `<target>\debug\deps` and the binary
        // target one level up.
        let exe = std::env::current_exe()
            .expect("the test executable must have a path")
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");

        assert!(
            exe.is_file(),
            "the product binary must be built alongside the tests: {}",
            exe.display()
        );

        let path: Vec<u16> = exe
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: `path` is a NUL-terminated UTF-16 buffer owned by this frame and not moved
        // or dropped until the call returns. `None` for the reserved file handle is what the
        // signature demands. `LOAD_LIBRARY_AS_DATAFILE` maps the image for resource reading
        // only: no entry point runs, nothing is relocated and no dependency is loaded, which
        // is what makes it safe to open a binary that would refuse to execute here. The
        // handle is freed exactly once, in `Drop` — or never, for the one mapping
        // `product_strings` keeps alive for the whole run.
        unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
            .expect("the product binary must be openable as a data file")
    }

    /// The module handle as an `HINSTANCE`, for `LoadImageW`.
    fn instance(&self) -> HINSTANCE {
        HINSTANCE(self.module.0)
    }

    /// A copy of one resource of the product binary, by type and numeric identifier.
    fn resource(&self, kind: PCWSTR, id: u16) -> Vec<u8> {
        let name = PCWSTR(std::ptr::without_provenance(usize::from(id)));

        // SAFETY: `self.module` is loaded for the lifetime of this value. Both "strings" are
        // integer identifiers in the `MAKEINTRESOURCE` form — a value below 65536 carried in
        // the pointer — so nothing is dereferenced as a string.
        let found = unsafe { FindResourceW(Some(self.module), name, kind) };
        assert!(!found.0.is_null(), "resource {id} must be present");

        // SAFETY: `self.module` and `found` are the pair just established.
        let size = unsafe { SizeofResource(Some(self.module), found) };

        // SAFETY: the same pair.
        let block = unsafe { LoadResource(Some(self.module), found) }.expect("resource loads");

        // SAFETY: `block` came from the `LoadResource` directly above.
        let start = unsafe { LockResource(block) };
        assert!(
            !start.is_null() && size > 0,
            "resource {id} must be readable"
        );

        // SAFETY: `start` points at `size` bytes of read-only resource data inside the
        // mapping of `self.module`, which is alive for the whole of this call. The bytes are
        // copied out before the borrow ends, so nothing of the module escapes.
        unsafe { std::slice::from_raw_parts(start.cast::<u8>(), size as usize).to_vec() }
    }
}

impl Drop for ProductImage {
    fn drop(&mut self) {
        // SAFETY: `module` came from a successful `LoadLibraryExW` and is freed exactly once
        // — this type is neither `Copy` nor `Clone`. Every slice taken from it has already
        // been copied into a `Vec`, and the icons loaded from it are handles of their own,
        // independent of the mapping.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

/// Installs a tray on `window`, with the icons from the product binary and the configuration
/// in `home`.
///
/// The product image is closed as soon as the icons have been loaded: `LoadImageW` without
/// `LR_SHARED` creates icon objects of their own, which do not refer back to the mapping the
/// bits came from.
fn install(window: &TestWindow, home: &TestDir) -> Tray {
    let product = ProductImage::open();

    Tray::install_at(window.handle, product.instance(), Some(home.config()))
        .expect("the tray must install: the icons are in the product binary's resources")
}

/// The same, put where [`tray::with_tray`] can find it — task T-13-14.
///
/// [`install`] hands back a `Tray` the test owns, which is enough for everything that drives
/// the type directly, and not enough for `tray::dispatch_command`: that function reaches the
/// tray of the calling thread through the module's own thread-local and through nothing else,
/// so a test that means to measure the handler has to put a tray in that slot. The
/// configuration still lives under `%TEMP%`, exactly as everywhere else in this file.
///
/// The returned [`Attachment`] takes the tray back out on drop, running the cleanup of FR-83 —
/// which is also the last write into `home`, so it must be dropped before the directory is.
/// Declaring `window`, then `home`, then this, in that order, is what arranges it.
fn attach_ui(window: &TestWindow, home: &TestDir) -> Attachment {
    let product = ProductImage::open();

    tray::attach_at(window.handle, product.instance(), Some(home.config()))
        .expect("the tray must attach: the icons are in the product binary's resources")
}

/// Reads something off the tray this thread has attached.
fn live<R>(f: impl FnOnce(&Tray) -> R) -> R {
    tray::with_tray(|tray| f(tray)).expect("this thread has a tray attached by `attach_ui`")
}

/// Changes something on it — «Применить» is the only caller, and it is the one step of the
/// audit's scenario that has no menu command behind it.
fn live_mut<R>(f: impl FnOnce(&mut Tray) -> R) -> R {
    tray::with_tray(f).expect("this thread has a tray attached by `attach_ui`")
}

/// A directory under `%TEMP%` that removes itself, panic or no panic.
///
/// Same shape as the one in `tests\settings.rs`, and written out for the same reason: there
/// is no temporary-directory crate in the dependency list of section 3.2 of SPEC.
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lang_switcher_t014_{}_{}_{label}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).expect("the temporary directory must be creatable");
        Self { path }
    }

    /// Path of `config.toml` inside this directory. The file is not created.
    fn config(&self) -> PathBuf {
        self.path.join(CONFIG_FILE_NAME)
    }

    /// Names of everything currently in the directory, sorted — task T-13-6.
    ///
    /// The same helper `tests\settings.rs` has, and it is here for the same reason: what a save
    /// left behind is as much a fact as what it wrote, and a stray `config.toml.bad` or a
    /// temporary of an unfinished atomic write is only visible by listing the folder.
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.path)
            .expect("the temporary directory must be readable")
            .map(|entry| {
                entry
                    .expect("the directory entry must be readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// `GetMenuItemCount` on a menu of ours.
fn item_count(menu: HMENU) -> i32 {
    // SAFETY: `menu` is a live menu owned by the `Menu` value of the calling frame. The call
    // reads no memory of ours and returns -1 for an invalid menu, which the caller asserts
    // against a positive expected count.
    unsafe { GetMenuItemCount(Some(menu)) }
}

/// `GetMenuItemInfoW(MIIM_DATA)` on one entry, by position — what Windows really stored as
/// the entry's `itemData`, which SEC-05 requires to be a command number and nothing else.
fn item_data_at(menu: HMENU, position: u32) -> usize {
    let mut info = MENUITEMINFOW {
        cbSize: u32::try_from(size_of::<MENUITEMINFOW>()).expect("the size fits in a u32"),
        fMask: MIIM_DATA,
        ..Default::default()
    };

    // SAFETY: `menu` is a live menu owned by the calling frame; `info` is a live local
    // whose `cbSize` describes it, and `MIIM_DATA` asks the call to write only
    // `dwItemData`. `true` makes the second argument an index rather than a command.
    unsafe { GetMenuItemInfoW(menu, position, true, &mut info) }
        .expect("the entry must be readable");

    info.dwItemData
}

/// `GetMenuItemID` on one entry, by position.
fn command_at(menu: HMENU, position: u32) -> u32 {
    // SAFETY: `menu` is a live menu owned by the calling frame; the call reads no memory
    // of ours and returns `0xFFFFFFFF` for a position that does not exist, which no
    // command of the crate equals.
    unsafe {
        GetMenuItemID(
            menu,
            i32::try_from(position).expect("a position fits in an i32"),
        )
    }
}

/// The `MF_*` flags of one entry, by position.
fn flags_at(menu: HMENU, position: u32) -> MENU_ITEM_FLAGS {
    // SAFETY: `menu` is a live menu owned by the calling frame; the call reads no memory of
    // ours and returns `0xFFFFFFFF` for an entry that does not exist.
    let flags = unsafe { GetMenuState(menu, position, MF_BYPOSITION) };

    // NFR-13, and since task T-13-14 it earns its keep. `0xFFFFFFFF` is not a set of flags but
    // a refusal, and it has every bit in it — read as flags it would prove an entry greyed,
    // disabled, checked and a separator all at once. The tests of the greying read the
    // **presence** of `MF_GRAYED`, so a refusal that went unexamined would make them pass for
    // an entry the call never looked at.
    assert_ne!(
        flags,
        u32::MAX,
        "GetMenuState refused entry {position} of this menu"
    );

    MENU_ITEM_FLAGS(flags)
}

/// Whether the entry at `position` is one of the two rules of FR-91.
fn is_separator(menu: HMENU, position: u32) -> bool {
    flags_at(menu, position).0 & MF_SEPARATOR.0 != 0
}

/// Whether the entry at `position` carries a check mark.
fn is_checked(menu: HMENU, position: u32) -> bool {
    flags_at(menu, position).0 & MF_CHECKED.0 != 0
}

/// Whether the entry at `position` is owner-drawn — FR-92а, task T-11-10.
fn is_owner_drawn(menu: HMENU, position: u32) -> bool {
    flags_at(menu, position).0 & MF_OWNERDRAW.0 != 0
}

/// Whether the entry at `position` is drawn as unavailable — task T-13-9.
fn is_grayed(menu: HMENU, position: u32) -> bool {
    flags_at(menu, position).0 & MF_GRAYED.0 != 0
}

/// Whether the entry at `position` cannot be chosen — task T-13-9.
///
/// The half that matters for the rule rather than for the look: with `MF_DISABLED` set,
/// `TrackPopupMenuEx` answers zero instead of the command, so no `dispatch_command` ever runs
/// for this entry.
fn is_disabled(menu: HMENU, position: u32) -> bool {
    flags_at(menu, position).0 & MF_DISABLED.0 != 0
}
