//! The window in the corner — tasks T-119-5, T-119-6, T-119-7: the pure parts. The look follows the
//! passport of stage E120 (`PASSPORT.md` §1–§4) through one table; the layout, the hits, the
//! timing of the picture, when «Обновлено» is due and when «Что нового в X…» stands.

use lang_switcher::corner::{
    self, Action, Content, DARK, LIGHT, Measured, Part, ROUND_MS, SIZES, TYPE,
};
use lang_switcher::layouts::LayoutId;
use lang_switcher::letters::Date;

fn content(second: bool, picture: bool, check: bool) -> Content {
    Content {
        about: "обновление".to_owned(),
        title: "Lang Switcher обновлён до 1.8.0".to_owned(),
        text: "Было 1.6.1.".to_owned(),
        link: Some((
            "Все изменения на сайте".to_owned(),
            "https://example.invalid/",
        )),
        main: ("Понятно".to_owned(), Action::Close),
        second: second.then(|| ("Как пользоваться".to_owned(), Action::Close)),
        picture,
        check: check.then(|| "Больше не показывать окна обновлений".to_owned()),
    }
}

fn measured(main: i32, second: i32) -> Measured {
    Measured {
        title: 20,
        text: 60,
        link: 19,
        check: 18,
        buttons: (main, second),
    }
}

/// **The table is the passport's (§1, §3, §4, §2 «Окно в углу»)** — the numbers the shell of E120
/// will carry, under its names.
#[test]
fn the_table_is_the_passport_s() {
    assert_eq!(LIGHT.bg, (0xFA, 0xF9, 0xF6));
    assert_eq!(DARK.bg, (0x23, 0x27, 0x2F));
    assert_eq!(LIGHT.accent, (0x1F, 0x7A, 0x4A));
    assert_eq!(DARK.accent, (0x6C, 0xC8, 0x94));
    assert_eq!(LIGHT.edge, (0x8C, 0x88, 0x80));
    assert_eq!(LIGHT.button, (0x1C, 0x1B, 0x18));
    assert_eq!(DARK.button_fg, (0x23, 0x27, 0x2F));

    assert_eq!(SIZES.width, 364);
    assert_eq!(SIZES.padding, (10, 12, 16, 16));
    assert_eq!(SIZES.gap, 12);
    assert_eq!(SIZES.panda, 44);
    assert_eq!(SIZES.button, 40);
    assert_eq!(SIZES.radius_window, 12);
    assert_eq!(SIZES.radius_button, 8);

    assert!(TYPE.header.0 >= 120, "the header not smaller than 12");
    assert_eq!(TYPE.title.0, 150);
    assert_eq!(TYPE.text.0, 135);
    assert_eq!(corner::palette(true), LIGHT);
    assert_eq!(corner::palette(false), DARK);
}

/// **The layout: 364 wide; the main button on the right and wider than the second (two shares
/// and one); a label that does not fit its share — one under the other, the main one first; the
/// picture and the check box below.**
#[test]
fn the_main_button_is_on_the_right_and_wider_unless_they_do_not_fit() {
    let l = corner::layout(&content(true, true, true), measured(110, 52), 96);

    assert_eq!(l.width, 364);
    assert!(l.main.left > l.second.right, "the main one on the right");
    assert_eq!(l.main.top, l.second.top, "in one row");
    assert!(
        (l.main.right - l.main.left) > (l.second.right - l.second.left),
        "and wider"
    );
    assert!(l.main.bottom - l.main.top >= 40, "not lower than 40");
    assert_eq!(l.main.right, 364 - 12, "to the right padding");
    assert_eq!(l.second.left, 16, "from the left padding");
    assert!(
        l.picture.top > l.main.bottom,
        "the picture under the buttons"
    );
    assert!(
        l.check.top > l.picture.bottom,
        "the check box under the picture"
    );
    assert_eq!(l.height, l.check.bottom + 16, "the bottom padding");

    let wide = corner::layout(&content(true, false, false), measured(60, 200), 96);

    assert_eq!(wide.main.left, 16, "too long: the main one across the row");
    assert!(
        wide.second.top >= wide.main.bottom,
        "and the second under it"
    );

    let one = corner::layout(&content(false, false, false), measured(60, 0), 96);

    assert_eq!(
        (one.main.left, one.main.right),
        (16, 352),
        "one button: the whole row"
    );
}

/// **At 150 % everything is one and a half times — the passport's numbers scale with the monitor.**
#[test]
fn the_layout_scales_with_the_monitor() {
    let l = corner::layout(&content(true, true, true), measured(90, 130), 144);

    assert_eq!(l.width, 546);
    assert!(l.main.bottom - l.main.top >= 60);
    assert_eq!(corner::px(SIZES.width, 120), 455);
    assert_eq!(corner::type_px(TYPE.text.0, 96), 14);
}

/// **A point falls on the ✕, a button, the link, the check box — or on nothing.**
#[test]
fn a_point_falls_on_its_part() {
    let l = corner::layout(&content(true, true, true), measured(110, 52), 96);
    let mid =
        |r: windows::Win32::Foundation::RECT| ((r.left + r.right) / 2, (r.top + r.bottom) / 2);

    for (rect, part) in [
        (l.close, Part::Close),
        (l.main, Part::Main),
        (l.second, Part::Second),
        (l.link, Part::Link),
        (l.check, Part::Check),
    ] {
        let (x, y) = mid(rect);
        assert_eq!(corner::hit(&l, x, y), part);
    }

    let (x, y) = mid(l.panda);
    assert_eq!(corner::hit(&l, x, y), Part::Nothing, "the panda");
}

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
