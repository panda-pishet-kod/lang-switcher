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
