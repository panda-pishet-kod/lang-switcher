//! The window in the corner — tasks T-119-5, T-119-6, T-119-7: the pure parts of its behaviour —
//! the timing of the picture, when «Обновлено» is due and when «Что нового в X…» stands. Its look is
//! the shell's since T-120-14 (the tests of `ui_corner`).

use lang_switcher::corner::{self, ROUND_MS};
use lang_switcher::layouts::LayoutId;
use lang_switcher::letters::Date;

/// **The picture follows the mock-up's timing over its 5,6 seconds** — the flag of the first
/// layout, the plate of «Alt + Shift» and the second flag, the cursor, the list and its hover, the
/// first layout again.
#[test]
fn the_picture_follows_the_mock_up_s_timing() {
    assert_eq!(ROUND_MS, 5600);

    let at = corner::frame;
    assert_eq!(at(500).flag, 0);
    assert!(!at(500).plate);
    assert!(at(1200).plate, "the plate from 1,0 s");
    assert_eq!(at(1500).flag, 1, "the second flag from 1,4 s");
    assert!(at(2500).cursor.is_some(), "the cursor from 2,3 s");
    assert_eq!(at(2300).cursor, Some(0));
    assert_eq!(
        at(3000).cursor,
        Some(1000),
        "at the flag after half a second"
    );
    assert!(at(3000).list, "the list from 2,8 s");
    assert!(at(3600).hover, "the first row hovered from 3,5 s");
    assert_eq!(at(4300).flag, 0, "the first layout again at 4,2 s");
    assert!(!at(4300).list);
    assert_eq!(at(5600 + 1500), at(1500), "round after round");
}

/// **The picture shows the person's own layouts — the one in force and the next.**
#[test]
fn the_picture_shows_the_person_s_layouts() {
    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let de = LayoutId::from_raw(0x0407_0407);

    assert_eq!(corner::picture_layouts(&[us, ru], ru), (ru, us));
    assert_eq!(corner::picture_layouts(&[us, ru, de], us), (us, ru));
    assert_eq!(corner::picture_layouts(&[us], us), (us, us), "one layout");
    assert_eq!(
        corner::picture_layouts(&[us, ru], de),
        (us, ru),
        "a layout out of the session"
    );
}

/// **«Обновлено» is owed after an update from a version older than the block, never on the first
/// install, never for a block newer than the build** (T-119-6).
#[test]
fn the_window_of_an_update_is_owed_once_by_the_rule() {
    assert!(corner::release_is_due("1.6.1", "1.8.0", "1.8.0"));
    assert!(
        corner::release_is_due("1.6.1", "1.8.1", "1.8.0"),
        "a minor release on top"
    );
    assert!(
        !corner::release_is_due("", "1.8.0", "1.8.0"),
        "the first install has «Привет»"
    );
    assert!(
        !corner::release_is_due("1.8.0", "1.8.1", "1.8.0"),
        "from the important release itself"
    );
    assert!(
        !corner::release_is_due("1.6.1", "1.7.0", "1.8.0"),
        "a block newer than the build"
    );
    assert_eq!(corner::LAST_IMPORTANT.version, "1.8.0");
    assert!(
        !lang_switcher::letters::version_is_newer(
            corner::LAST_IMPORTANT.version,
            env!("CARGO_PKG_VERSION")
        ),
        "the check of the build: the block is not newer than the build"
    );
}

/// **«Что нового в X…» stands for a week after any update** (T-119-7).
#[test]
fn what_is_new_stands_a_week() {
    let updated = Date::from_ymd(2026, 11, 20).expect("a date");
    let on = |days: i64| corner::whats_new_stands(Some(updated), Some(updated.plus_days(days)));

    assert!(on(0), "the day of the update");
    assert!(on(6), "the sixth day");
    assert!(!on(7), "a week later — gone");
    assert!(!on(-1), "a clock turned back");
    assert!(!corner::whats_new_stands(None, Some(updated)), "no update");
}

/// **The hint and «Обновлено» say what the mock-up says, by the passport's buttons** — the hint's
/// main button opens the page of the taskbar; «Обновлено» closes with «Понятно», its second opens
/// a page of the program's own.
#[test]
fn the_two_windows_say_what_they_are_for() {
    assert_eq!(corner::TASKBAR_SETTINGS, "ms-settings:taskbar");
    assert_eq!(corner::QUIET_SECONDS, 5, "five seconds without a key");
    assert_ne!(corner::QUIET_TIMER, corner::HINT_TIMER);
}

/// ⭐ **A second start before the first tick of the letters owes no second window** — the stand of
/// 2026-11-20 (Windows 11, `lab-f77b50d`): the program was started on a configuration that last saw
/// 1.6.1, showed «Обновлено», was stopped and started again 32 s later — before the first tick of the
/// letters (90 s), which until then was what moved the version seen — and showed it again. The start
/// that owes the window moves the version seen itself.
#[test]
fn a_second_start_before_the_first_tick_owes_no_second_window() {
    let build = env!("CARGO_PKG_VERSION");
    let mut state = lang_switcher::settings::Letters {
        last_seen_version: "1.6.1".to_owned(),
        ..lang_switcher::settings::Letters::default()
    };

    assert!(
        state.release_window,
        "the window of updates is on by default"
    );
    assert!(
        corner::owe_release_window(&mut state, build),
        "the first start after the update from 1.6.1 owes the window"
    );
    assert_eq!(
        state.release_due, "1.6.1",
        "written as due, with the version updated from"
    );

    // The window was shown in its quiet moment: no longer due (`quiet_tick`).
    state.release_due.clear();

    // The program is stopped and started again before the first tick of the letters.
    assert!(
        !corner::owe_release_window(&mut state, build),
        "a second start owes no second window"
    );
    assert!(state.release_due.is_empty(), "and writes none");
    assert_eq!(
        state.last_seen_version, build,
        "because the first start moved the version seen to this build at once"
    );

    // The control: a window owed and not yet shown (stopped before its quiet moment) stays owed.
    let mut stopped = lang_switcher::settings::Letters {
        last_seen_version: build.to_owned(),
        release_due: "1.6.1".to_owned(),
        ..lang_switcher::settings::Letters::default()
    };

    assert!(
        corner::owe_release_window(&mut stopped, build),
        "a window not shown yet is still owed"
    );

    // And «Больше не показывать окна обновлений» owes nothing.
    let mut off = lang_switcher::settings::Letters {
        last_seen_version: "1.6.1".to_owned(),
        release_window: false,
        ..lang_switcher::settings::Letters::default()
    };

    assert!(!corner::owe_release_window(&mut off, build));
}

/// ⭐ **The path Windows 11 writes in `NotifyIconSettings` is opened before it is compared** — defect 2
/// of the stand (2026-11-20): the head of `ExecutablePath` is a known folder,
/// `{6D809377-6AF0-444B-8957-A3773F02200E}` for Program Files, and the path compared letter for letter
/// found no key, so the hint of the flag (T-119-5) never came.
#[test]
fn the_path_of_notify_icon_settings_is_opened_from_its_known_folder() {
    let program_files = std::env::var("ProgramW6432")
        .or_else(|_| std::env::var("ProgramFiles"))
        .expect("Program Files is named in the environment");

    assert_eq!(
        corner::executable_path_of(
            r"{6D809377-6AF0-444B-8957-A3773F02200E}\Lang_Switcher\LangSwitcher.exe"
        )
        .to_lowercase(),
        format!(r"{program_files}\Lang_Switcher\LangSwitcher.exe").to_lowercase(),
        "the head is Program Files, the rest as written"
    );
    assert_eq!(
        corner::executable_path_of("C:\\lab\\ls\\LangSwitcher.exe\0"),
        r"C:\lab\ls\LangSwitcher.exe",
        "a path with no folder of its own is the path"
    );
    assert_eq!(
        corner::executable_path_of(r"{00000000-0000-0000-0000-000000000001}\x\LangSwitcher.exe"),
        r"{00000000-0000-0000-0000-000000000001}\x\LangSwitcher.exe",
        "a folder the system does not know leaves it as written"
    );
    assert_eq!(
        corner::executable_path_of(r"{not a guid}\LangSwitcher.exe"),
        r"{not a guid}\LangSwitcher.exe"
    );
}

/// ⭐ **Z10 (решение 208.209): «Открыть Параметры Windows» leaves the hint of the flag up** — the
/// owner's word: «должна дождаться пока я нажму понятно или пункт будет выставлен». Until 1.8.0 every
/// button of a window in the corner closed it, the hint's main button too (Б2 of AUDIT-180). Now the
/// press of the hint's main button opens the page of the taskbar and the window stays; «Понятно» and
/// the ✕ end it (the flag on the taskbar ends it too — `hint_tick`); every other window in the corner
/// still ends on its main button.
#[test]
fn the_hint_of_the_flag_stays_after_its_main_button() {
    use lang_switcher::corner::{Action, EndBy, Press, on_press};
    use lang_switcher::letters::{self, Letter};
    use lang_switcher::settings::Language;
    use lang_switcher::ui::windows::corner::Id;

    for language in [Language::Ru, Language::En, Language::He] {
        let hint = corner::hint_content_in(language);
        assert_eq!(
            on_press(&hint, Id::Main),
            Press::Open(corner::TASKBAR_SETTINGS),
            "{language:?}: the main button of the hint opens the page of the taskbar and stays"
        );
        assert_eq!(
            on_press(&hint, Id::Second),
            Press::End(Action::Close, EndBy::Second),
            "{language:?}: «Понятно» ends the hint"
        );
        assert_eq!(
            on_press(&hint, Id::Close),
            Press::End(Action::Close, EndBy::Close),
            "{language:?}: the ✕ ends the hint"
        );

        let updated = corner::release_content_in(language, "1.7.0");
        assert!(
            matches!(on_press(&updated, Id::Main), Press::End(_, EndBy::Main)),
            "{language:?}: «Обновлено» still ends on its main button"
        );
        let notice =
            letters::notice_content_in(language, Letter::Thanks).expect("«Спасибо» knocks");
        assert!(
            matches!(on_press(&notice, Id::Main), Press::End(_, EndBy::Main)),
            "{language:?}: the knock of a letter still ends on «Открыть письмо»"
        );
    }
}
