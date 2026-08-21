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

use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use lang_switcher::settings::{self, CONFIG_FILE_NAME};
use lang_switcher::tray::{self, Menu, Reaction, Tray};

use windows::Win32::Foundation::{FreeLibrary, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, LoadResource,
    LockResource, SizeofResource,
};
use windows::Win32::UI::Controls::{MEASUREITEMSTRUCT, ODT_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetMenuItemCount, GetMenuItemID, GetMenuItemInfoW,
    GetMenuState, GetSystemMetrics, HMENU, MENU_ITEM_FLAGS, MENUITEMINFOW, MF_BYPOSITION,
    MF_CHECKED, MF_OWNERDRAW, MF_SEPARATOR, MIIM_DATA, RT_VERSION, SM_CXSMICON, SM_CYSMICON,
    WM_DRAWITEM, WM_ENDSESSION, WM_MEASUREITEM, WM_QUERYENDSESSION, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

// ---------------------------------------------------------------------------------------
// FR-91, read back from Windows
// ---------------------------------------------------------------------------------------

/// The menu of FR-91 as its block prints, top to bottom. `None` is one of the two rules.
///
/// Written out here on purpose. These seven lines are the requirement; the module's own
/// resources are the implementation, and a test must not check one against itself.
const FR_91: [Option<&str>; 7] = [
    Some("Приостановить"),
    None,
    Some("Настройки…"),
    Some("Запускать при входе в систему"),
    None,
    Some("О программе"),
    Some("Выход"),
];

/// The same seven lines in the other locale of FR-94 — task T-08-2.
///
/// ⚠ The **composition** is identical and only the words move: five commands and the same two
/// rules in the same places. A translation that quietly dropped or added an entry would be a
/// different menu, and FR-91 fixes the menu.
const FR_91_ENGLISH: [Option<&str>; 7] = [
    Some("Suspend"),
    None,
    Some("Settings…"),
    Some("Start when I sign in"),
    None,
    Some("About"),
    Some("Exit"),
];

/// Serialises the tests that publish an interface locale — it is process-wide, and the tests of
/// one binary run on parallel threads.
static LOCALE: Mutex<()> = Mutex::new(());

/// The mapping [`product_strings`] hands out, loaded once and never unmapped.
static SHARED_IMAGE: OnceLock<usize> = OnceLock::new();

/// Points `settings` at the string tables of the built binary and publishes `language`.
///
/// ⚠ Without the redirection every label would come back empty: `embed-resource` links `app.rc`
/// into the **binary** targets of the crate, so this test executable has no resource section of
/// its own — section 4.4 of STATE.md, the same barrier the icons above are loaded around. What
/// the menu is checked against is therefore the table that ships.
fn product_strings(language: settings::Language) -> MutexGuard<'static, ()> {
    let guard = LOCALE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let raw = *SHARED_IMAGE.get_or_init(|| ProductImage::load().0 as usize);

    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
    settings::set_ui_language(language);

    guard
}

/// Checks one built menu against one seven-line block of FR-91.
///
/// Since task T-11-10 the entries are `MF_OWNERDRAW`, so the check is in two halves. Out
/// of Windows, as an outside observer would read them: the entry count, which positions
/// are rules, that every command entry carries the owner-draw flag, and that its
/// `itemData` is exactly its command identifier (SEC-05 — a number, not a pointer). Out of
/// the builder's record, because Windows no longer holds a string for an owner-drawn
/// entry: the labels, in order, against the literals of the block.
fn assert_menu_is(menu: &Menu, block: &[Option<&str>; 7]) {
    assert_eq!(
        item_count(menu.handle()),
        7,
        "FR-91 lists five commands and two rules, which is seven entries"
    );

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
                println!("{position}: separator={separator}");

                assert!(separator, "entry {position} of FR-91 is a rule");
                assert!(
                    !is_owner_drawn(menu.handle(), index),
                    "a rule stays system-drawn — the accepted cost of section 10"
                );
            }
        }
    }

    assert!(
        items.next().is_none(),
        "the builder recorded exactly the five commands of FR-91"
    );
}

#[test]
fn the_menu_is_the_block_of_fr_91_entry_for_entry() {
    let _locale = product_strings(settings::Language::Ru);
    let menu = Menu::build(true, true).expect("the menu of FR-91 must be creatable");

    assert_menu_is(&menu, &FR_91);
}

#[test]
fn the_menu_of_fr_91_is_the_same_menu_in_english() {
    // **Criterion 12 of T-08-2.** FR-94 translates the labels; FR-91 fixes what the menu
    // *is*, and the second must survive the first — same seven entries, same two rules,
    // same places.
    let _locale = product_strings(settings::Language::En);
    let menu = Menu::build(true, true).expect("the menu of FR-91 must be creatable");

    assert_menu_is(&menu, &FR_91_ENGLISH);

    // The suspended state moves the same entry in this locale as in the other one.
    let suspended = Menu::build(false, true).expect("the menu must be creatable");
    assert_eq!(suspended.items()[0].label, "Resume");

    settings::set_ui_language(settings::Language::Ru);
}

#[test]
fn the_first_entry_follows_the_state() {
    let _locale = product_strings(settings::Language::Ru);
    let active = Menu::build(true, true).expect("the menu must be creatable");
    let suspended = Menu::build(false, true).expect("the menu must be creatable");

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

#[test]
fn the_autostart_entry_shows_the_check_mark_of_the_configuration() {
    let _locale = product_strings(settings::Language::Ru);
    let on = Menu::build(true, true).expect("the menu must be creatable");
    let off = Menu::build(true, false).expect("the menu must be creatable");

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
    let menu = Menu::build(true, true).expect("the menu must be creatable");

    // The five command positions of the FR-91 block — everything but the two rules.
    let command_positions = [0u32, 2, 3, 5, 6];

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
// The about box
// ---------------------------------------------------------------------------------------

#[test]
fn the_version_of_the_about_box_comes_out_of_the_version_resource() {
    // The about box shows what `VERSIONINFO` says, not what `Cargo.toml` says. The offset of
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

    assert_eq!(
        version,
        Some((0, 1, 0, 0)),
        "app.rc declares FILEVERSION 0,1,0,0 (decision 8)"
    );

    // The test binary itself carries no resources, so the about box of *this* process has no
    // version to show — and says so instead of failing.
    assert_eq!(tray::file_version(), None);
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
    // ours and returns `0xFFFFFFFF` for an entry that does not exist, which cannot be
    // mistaken for either flag tested below.
    MENU_ITEM_FLAGS(unsafe { GetMenuState(menu, position, MF_BYPOSITION) })
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
