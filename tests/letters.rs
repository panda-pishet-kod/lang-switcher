//! The schedule of the letters from the author — **FR-101**, task Т-32-1.
//!
//! Every rule of FR-101 is a rule about days, and `letters::due` is a pure function of the
//! state, the day, the version, the feed and the quiet moment. So these tests need no window,
//! no clock and no configuration file: a state is written out by hand, a day is named, and the
//! answer is the letter the requirement names.
//!
//! The dates in here are deliberately absolute — 2026-09-03 is the day this stage was built —
//! rather than «today plus n». A test that computes its own dates from the machine's clock is a
//! test that can pass on a Tuesday and fail on a Wednesday, and this whole module is arithmetic
//! over the calendar.

use lang_switcher::letters::{
    self, Date, FeedItem, FeedView, Letter, REMINDERS_PER_NEWS, Thanks, links,
};
use lang_switcher::settings::{self, Letters};

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
use windows::core::PCWSTR;

/// The built `LangSwitcher.exe`, mapped once so that the string tables of FR-94 can be read.
///
/// Section 4.4 of STATE.md: `embed-resource` links `app.rc` into the **binary** targets of this
/// crate and not into the test executables, so a test that asked this one for a string would be
/// told there is no resource section at all. The strings a letter is made of are the strings
/// that ship, so the test reads the file that ships — the same road `tests\settings.rs` takes.
///
/// The mapping is never freed: it is handed to `settings::set_resource_module`, which keeps no
/// lifetime, and the tests of one binary run on parallel threads.
static PRODUCT: OnceLock<usize> = OnceLock::new();

/// Points the string tables at the product binary. Every test that reads a word of a letter
/// calls this first; calling it twice is calling it once.
fn with_product_strings() {
    let raw = *PRODUCT.get_or_init(|| {
        // `cargo test` puts the test executables in `<target>\debug\deps` and the binary
        // target one level up.
        let exe = std::env::current_exe()
            .expect("the test executable must have a path")
            .parent()
            .and_then(Path::parent)
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
        // or dropped until the call returns. `LOAD_LIBRARY_AS_DATAFILE` maps the image for
        // resource reading only: no entry point runs and no dependency is loaded.
        let module =
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
                .expect("the product binary must be openable as a data file");

        module.0 as usize
    });

    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
    settings::set_ui_language(settings::Language::Ru);
}

/// The day the stage was built, and the «today» of most of the tests below.
fn day(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd(year, month, day).expect("the test names a real day")
}

/// A state that has been running for a while and has nothing due: the welcome shown, the
/// version already seen, «Спасибо» far away.
fn settled(version: &str) -> Letters {
    Letters {
        welcome_shown: true,
        first_run: Some(day(2026, 9, 1)),
        thanks_due: Some(day(2026, 10, 1)),
        last_seen_version: version.to_owned(),
        ..Letters::default()
    }
}

/// One news entry of the feed.
fn news(id: u64) -> FeedItem {
    FeedItem {
        id,
        date: Some(day(2026, 9, 2)),
        title: "У меня вышла новая программа".to_owned(),
        text: "Она тоже делает одну вещь и тоже без сети.".to_owned(),
        link: String::new(),
        version: None,
    }
}

/// The single `update` entry of the feed, announcing `version`.
fn update(version: &str) -> FeedItem {
    FeedItem {
        id: 1,
        date: Some(day(2026, 9, 2)),
        title: "Вышла версия".to_owned(),
        text: "Три изменения.".to_owned(),
        link: String::new(),
        version: Some(version.to_owned()),
    }
}

// =========================================================================================
// The calendar
// =========================================================================================

/// The days-from-civil pair is exact in both directions over the whole range this program can
/// meet, leap days and century rules included.
#[test]
fn the_calendar_counts_days_the_way_the_calendar_does() {
    // The epoch itself, which is what every other number here is counted from.
    assert_eq!(day(1970, 1, 1).to_days(), 0);
    assert_eq!(Date::from_days(0), day(1970, 1, 1));

    // One leap year, one century that is not a leap year, one century that is.
    assert_eq!(day(2024, 2, 28).plus_days(1), day(2024, 2, 29));
    assert_eq!(day(1900, 2, 28).plus_days(1), day(1900, 3, 1));
    assert_eq!(day(2000, 2, 28).plus_days(1), day(2000, 2, 29));

    // Across a year boundary in both directions.
    assert_eq!(day(2026, 12, 31).plus_days(1), day(2027, 1, 1));
    assert_eq!(day(2027, 1, 1).plus_days(-1), day(2026, 12, 31));

    // The cadences of FR-101 and FR-102, each landing where the requirement says.
    assert_eq!(day(2026, 9, 3).plus_days(30), day(2026, 10, 3));
    assert_eq!(day(2026, 9, 3).plus_days(7), day(2026, 9, 10));
    assert_eq!(day(2026, 9, 3).plus_days(15), day(2026, 9, 18));
    assert_eq!(day(2026, 9, 3).plus_days(90), day(2026, 12, 2));

    // The distance is signed and symmetrical.
    assert_eq!(day(2026, 10, 3).days_since(day(2026, 9, 3)), 30);
    assert_eq!(day(2026, 9, 3).days_since(day(2026, 10, 3)), -30);

    // Every day of a hundred years round-trips through the day number. The positive control of
    // the instrument is the count: a loop that walked nothing would assert nothing.
    let mut walked = 0;
    let mut date = day(1980, 1, 1);
    while date < day(2080, 1, 1) {
        assert_eq!(
            Date::from_days(date.to_days()),
            date,
            "{date} did not survive"
        );
        date = date.plus_days(1);
        walked += 1;
    }
    // A hundred years from 1980: 36 500 ordinary days plus twenty-five leap days — 1980, 1984
    // … 2076, and 2000 is one of them because a year divisible by four hundred *is* a leap
    // year. The number is written out because it is the positive control of this loop: a walk
    // that ended early would still pass every assertion inside it.
    assert_eq!(walked, 36_525, "a hundred years from 1980 is 36 525 days");
}

/// A day that is not a day is refused where it is made, and the refusal knows the leap rule.
#[test]
fn a_day_that_does_not_exist_is_refused() {
    assert!(
        Date::from_ymd(2026, 2, 29).is_none(),
        "2026 is not a leap year"
    );
    assert!(Date::from_ymd(2024, 2, 29).is_some(), "2024 is");
    assert!(
        Date::from_ymd(2026, 4, 31).is_none(),
        "April has thirty days"
    );
    assert!(
        Date::from_ymd(2026, 13, 1).is_none(),
        "there is no month thirteen"
    );
    assert!(Date::from_ymd(2026, 0, 1).is_none(), "or a month zero");
    assert!(Date::from_ymd(2026, 1, 0).is_none(), "or a day zero");
}

/// The two spellings a date can arrive in, and everything that is not one.
#[test]
fn a_date_is_read_and_written_the_way_section_7_prints_it() {
    assert_eq!(Date::parse("2026-09-03"), Date::from_ymd(2026, 9, 3));
    assert_eq!(day(2026, 9, 3).to_string(), "2026-09-03");
    assert_eq!(day(2026, 12, 31).to_string(), "2026-12-31");

    for refused in [
        "2026-9-3",
        "26-09-03",
        "2026/09/03",
        "2026-09-03T10:00:00Z",
        "2026-13-01",
        "2026-02-30",
        "",
        "tomorrow",
    ] {
        assert!(Date::parse(refused).is_none(), "«{refused}» is not a date");
    }
}

/// The clock the program actually asks — the one impure function of the calendar.
///
/// Not «what day is it»: that is the machine's business and would make the test a copy of the
/// thing under test. What is asserted is that the answer is *a* day, in the range a running
/// machine can be in, and that two calls a moment apart agree — which is what fails if the
/// picture, the locale or the parsing is wrong, and the failure `GetDateFormatEx` would give
/// for a wrong picture is an empty answer rather than a wrong date.
#[test]
fn today_is_a_day_this_program_can_be_running_on() {
    let today = letters::today().expect("the system must be able to say what day it is");

    assert!(
        today > day(2020, 1, 1),
        "{today} is before this program existed"
    );
    assert!(
        today < day(2100, 1, 1),
        "{today} is beyond any plausible clock"
    );
    assert_eq!(
        letters::today(),
        Some(today),
        "two readings of one day agree"
    );
}

// =========================================================================================
// «Привет» — FR-101
// =========================================================================================

/// «Привет» is shown at once, and it is the only letter that is not gated by anything.
#[test]
fn the_welcome_letter_is_shown_at_once_and_waits_for_nothing() {
    let fresh = Letters::default();
    let today = day(2026, 9, 3);

    // Not quiet, and it does not matter: FR-101 shows this one «сразу».
    assert_eq!(
        letters::due(&fresh, today, "0.39.0", FeedView::EMPTY, false),
        Some(Letter::Welcome)
    );
    assert_eq!(
        letters::due(&fresh, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Welcome)
    );

    // A letter was already shown today, and it still does not matter.
    let mut busy = fresh.clone();
    busy.last_letter = Some(today);
    assert_eq!(
        letters::due(&busy, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Welcome)
    );
}

/// Shown once, and the mark is what stops it — and it does **not** spend the day's one slot,
/// which is the exemption FR-101 grants it by name.
#[test]
fn the_welcome_letter_is_shown_once_and_does_not_spend_the_day() {
    let mut state = Letters::default();
    let today = day(2026, 9, 3);

    letters::initialise(&mut state, today, "0.39.0");
    let letter = letters::due(&state, today, "0.39.0", FeedView::EMPTY, true).expect("welcome");
    letters::after_shown(&mut state, letter, today, "0.39.0");

    assert!(state.welcome_shown);
    assert_eq!(state.last_letter, None, "«Привет» does not spend the day");
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        None,
        "and nothing follows it on a fresh install"
    );
}

/// **A fresh installation is not «обновившийся»**: the version it was installed at counts as
/// already seen, so «Что нового» does not follow «Привет» within the hour.
#[test]
fn a_fresh_installation_has_nothing_new_to_be_told_about() {
    let mut state = Letters::default();
    let today = day(2026, 9, 3);

    assert!(letters::initialise(&mut state, today, "0.39.0"));

    assert_eq!(state.first_run, Some(today));
    assert_eq!(state.thanks_due, Some(day(2026, 10, 3)));
    assert_eq!(
        state.last_seen_version, "0.39.0",
        "the version it was installed at is already seen"
    );

    // And the second call does nothing at all: the day is counted from once.
    assert!(!letters::initialise(&mut state, day(2026, 9, 4), "0.39.0"));
    assert_eq!(state.first_run, Some(today));
}

/// A machine that came through the migration **is** «обновившийся»: it has seen «Привет» and it
/// has not seen this version, so the first thing it meets is «Что нового».
#[test]
fn a_migrated_machine_is_told_what_is_new_and_not_hello() {
    let mut state = Letters {
        welcome_shown: true,
        last_seen_version: String::new(),
        ..Letters::default()
    };
    let today = day(2026, 9, 3);

    assert!(letters::initialise(&mut state, today, "0.39.0"));
    assert_eq!(
        state.last_seen_version, "",
        "initialisation must not silence «Что нового» for somebody who updated"
    );
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::WhatsNew)
    );
}

// =========================================================================================
// The gates: the quiet moment and the one letter a day — FR-101
// =========================================================================================

/// Everything but «Привет» waits for a quiet moment.
#[test]
fn nothing_but_hello_arrives_while_the_moment_is_not_quiet() {
    let state = settled("0.38.0");
    let today = day(2026, 9, 3);

    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, false),
        None,
        "«Что нового» is due and the moment is not quiet"
    );
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::WhatsNew)
    );
}

/// One letter a day, and the day survives a restart because it is written in the file.
#[test]
fn only_one_letter_a_day_reaches_the_screen() {
    let mut state = settled("0.38.0");
    state.thanks_due = Some(day(2026, 9, 1));
    let today = day(2026, 9, 3);

    // Two letters are due at once: «Что нового» and «Спасибо». The priority of FR-101 picks
    // the first.
    let first = letters::due(&state, today, "0.39.0", FeedView::EMPTY, true);
    assert_eq!(first, Some(Letter::WhatsNew));

    letters::after_shown(&mut state, Letter::WhatsNew, today, "0.39.0");
    assert_eq!(state.last_letter, Some(today));

    // The second one waits for tomorrow — and this is the assertion that needs the day to be in
    // the file: the state has been written and read back in between on a real machine.
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        None,
        "«Спасибо» must not follow «Что нового» on the same day"
    );
    assert_eq!(
        letters::due(&state, today.plus_days(1), "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Thanks)
    );
}

/// The priority of FR-101, all four letters due at the same moment.
#[test]
fn the_priority_of_fr_101_is_whats_new_update_news_thanks() {
    let mut state = settled("0.38.0");
    state.thanks_due = Some(day(2026, 9, 1));
    let today = day(2026, 9, 3);
    let items = [news(12)];
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &items,
    };

    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::WhatsNew),
        "first the letter about the program the person is running"
    );

    state.last_seen_version = "0.39.0".to_owned();
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::Update),
        "then the one about the version they could be running"
    );

    state.latest_known = "0.40.0".to_owned();
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::News(12)),
        "then the news"
    );

    state.mark_read(12);
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::Thanks),
        "and last the letter about the author"
    );

    state.thanks = Thanks::Done;
    assert_eq!(letters::due(&state, today, "0.39.0", feed, true), None);
}

// =========================================================================================
// «Что нового» — FR-101
// =========================================================================================

/// Shown once per version, and the mark is the version itself.
#[test]
fn whats_new_is_shown_once_for_each_version() {
    let mut state = settled("0.38.0");
    let today = day(2026, 9, 3);

    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::WhatsNew)
    );

    letters::after_shown(&mut state, Letter::WhatsNew, today, "0.39.0");
    assert_eq!(state.last_seen_version, "0.39.0");

    assert_eq!(
        letters::due(&state, today.plus_days(1), "0.39.0", FeedView::EMPTY, true),
        None,
        "the same version is not new twice"
    );
    assert_eq!(
        letters::due(&state, today.plus_days(2), "0.40.0", FeedView::EMPTY, true),
        Some(Letter::WhatsNew),
        "and the next one is"
    );
}

// =========================================================================================
// «Спасибо» — FR-101
// =========================================================================================

/// Thirty days, one snooze of seven, and then done for good.
#[test]
fn thanks_arrives_after_thirty_days_and_snoozes_once() {
    let mut state = settled("0.39.0");
    let first_run = day(2026, 9, 1);
    letters::initialise(&mut state, first_run, "0.39.0");

    assert_eq!(state.thanks_due, Some(day(2026, 10, 1)));

    // The day before is not the day.
    assert_eq!(
        letters::due(&state, day(2026, 9, 30), "0.39.0", FeedView::EMPTY, true),
        None
    );
    assert_eq!(
        letters::due(&state, day(2026, 10, 1), "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Thanks)
    );

    // Shown, and the snooze offered — this is the first showing.
    assert!(letters::snooze_is_offered(&state));
    letters::after_shown(&mut state, Letter::Thanks, day(2026, 10, 1), "0.39.0");
    letters::snooze_thanks(&mut state, day(2026, 10, 1));

    assert_eq!(state.thanks, Thanks::Snoozed);
    assert_eq!(state.thanks_due, Some(day(2026, 10, 8)));
    assert!(
        !letters::snooze_is_offered(&state),
        "the snooze of FR-101 is offered once"
    );

    // A week later it comes back once more, and the showing ends it.
    assert_eq!(
        letters::due(&state, day(2026, 10, 7), "0.39.0", FeedView::EMPTY, true),
        None
    );
    assert_eq!(
        letters::due(&state, day(2026, 10, 8), "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Thanks)
    );

    letters::after_shown(&mut state, Letter::Thanks, day(2026, 10, 8), "0.39.0");
    assert_eq!(state.thanks, Thanks::Done);
    assert_eq!(
        letters::due(&state, day(2027, 1, 1), "0.39.0", FeedView::EMPTY, true),
        None,
        "«done» is for ever"
    );
}

// =========================================================================================
// «Обновление» — FR-101
// =========================================================================================

/// Versions are compared part by part as numbers, which is the only way `0.10.0` is newer than
/// `0.9.0`.
#[test]
fn a_newer_version_is_newer_by_number_and_not_by_spelling() {
    assert!(letters::version_is_newer("0.39.0", "0.38.0"));
    assert!(letters::version_is_newer("0.10.0", "0.9.0"));
    assert!(letters::version_is_newer("1.0.0", "0.99.99"));
    assert!(!letters::version_is_newer("0.38.0", "0.38.0"));
    assert!(!letters::version_is_newer("0.38.0", "0.39.0"));
    assert!(!letters::version_is_newer("0.9.0", "0.10.0"));

    // A shorter version is the same version with zeros, and rubbish is a zero rather than a
    // panic: the string came from outside this program (SEC-05).
    assert!(!letters::version_is_newer("0.38", "0.38.0"));
    assert!(letters::version_is_newer("0.39", "0.38.0"));
    assert!(!letters::version_is_newer("не версия", "0.38.0"));
    assert!(!letters::version_is_newer("", "0.0.0"));
}

/// One letter per announced version, and the menu entry outlives the letter.
#[test]
fn the_update_letter_is_shown_once_for_each_announced_version() {
    let mut state = settled("0.39.0");
    let today = day(2026, 9, 3);
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::Update)
    );

    letters::after_shown(&mut state, Letter::Update, today, "0.40.0");
    assert_eq!(state.latest_known, "0.40.0");

    assert_eq!(
        letters::due(&state, today.plus_days(1), "0.39.0", feed, true),
        None,
        "one letter per version"
    );
    assert!(
        letters::update_is_pending("0.39.0", feed).is_some(),
        "and the menu entry of FR-91 stays until the person updates"
    );
    assert!(
        letters::update_is_pending("0.40.0", feed).is_none(),
        "and goes when they have"
    );
}

/// A feed that announces a version nobody needs says nothing at all.
#[test]
fn an_update_that_is_not_newer_is_not_a_letter() {
    let state = settled("0.39.0");
    let today = day(2026, 9, 3);
    let announced = update("0.39.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    assert_eq!(letters::due(&state, today, "0.39.0", feed, true), None);
    assert!(!letters::update_is_due(&state, "0.39.0", feed));
}

// =========================================================================================
// «Новость» — FR-101
// =========================================================================================

/// The round of an unread news item: the arrival, seven days, fourteen days, and then the dot
/// on the icon for ever.
#[test]
fn an_unread_news_item_is_reminded_of_twice_and_then_never_again() {
    let mut state = settled("0.39.0");
    // This test walks more than a year forward, and «Спасибо» would fall due inside that walk
    // and answer where the news is expected. It is not the subject here, so it is put to rest —
    // the letter's own rules are the subject of `thanks_arrives_after_thirty_days_and_snoozes_once`.
    state.thanks = Thanks::Done;
    let items = [news(12)];
    let feed = FeedView {
        update: None,
        news: &items,
    };
    let arrived = day(2026, 9, 3);

    // The arrival.
    assert_eq!(
        letters::due(&state, arrived, "0.39.0", feed, true),
        Some(Letter::News(12))
    );
    letters::after_shown(&mut state, Letter::News(12), arrived, "0.39.0");
    assert_eq!(state.first_shown_on(12), Some(arrived));
    assert_eq!(state.reminders_sent(12), 0);
    assert_eq!(
        state.first_feed_letter,
        Some(arrived),
        "the first letter out of the feed is remembered — FR-102 hangs the switch on it"
    );

    // Six days later: nothing. Seven: the first reminder.
    assert_eq!(
        letters::due(&state, arrived.plus_days(6), "0.39.0", feed, true),
        None
    );
    assert_eq!(
        letters::due(&state, arrived.plus_days(7), "0.39.0", feed, true),
        Some(Letter::News(12))
    );
    letters::after_shown(&mut state, Letter::News(12), arrived.plus_days(7), "0.39.0");
    assert_eq!(state.reminders_sent(12), 1);
    assert_eq!(
        state.first_shown_on(12),
        Some(arrived),
        "the reminders are counted from the first showing, which does not move"
    );

    // Thirteen days: nothing. Fourteen: the second and last reminder.
    assert_eq!(
        letters::due(&state, arrived.plus_days(13), "0.39.0", feed, true),
        None
    );
    assert_eq!(
        letters::due(&state, arrived.plus_days(14), "0.39.0", feed, true),
        Some(Letter::News(12))
    );
    letters::after_shown(
        &mut state,
        Letter::News(12),
        arrived.plus_days(14),
        "0.39.0",
    );
    assert_eq!(state.reminders_sent(12), REMINDERS_PER_NEWS);

    // And never again — but the dot stays, because the item is still unread.
    for after in [21, 28, 100, 400] {
        assert_eq!(
            letters::due(&state, arrived.plus_days(after), "0.39.0", feed, true),
            None,
            "no third reminder, {after} days on"
        );
    }
    assert!(letters::has_unread_news(&state, feed));
}

/// «Прочитано» is the only thing that makes a news item read — and it puts the dot out.
#[test]
fn only_the_read_button_makes_a_news_item_read() {
    let mut state = settled("0.39.0");
    let items = [news(12)];
    let feed = FeedView {
        update: None,
        news: &items,
    };
    let arrived = day(2026, 9, 3);

    letters::after_shown(&mut state, Letter::News(12), arrived, "0.39.0");
    assert!(
        letters::has_unread_news(&state, feed),
        "«Позже» left it unread"
    );

    letters::mark_read(&mut state, 12);

    assert!(state.is_read(12));
    assert!(!letters::has_unread_news(&state, feed), "the dot goes out");
    assert_eq!(
        letters::due(&state, arrived.plus_days(7), "0.39.0", feed, true),
        None,
        "and the reminders stop"
    );
}

/// The oldest unread item is the one shown, so somebody who has been away meets the news in the
/// order it was written.
#[test]
fn the_oldest_unread_news_is_the_one_shown() {
    let mut state = settled("0.39.0");
    let items = [news(11), news(12), news(13)];
    let feed = FeedView {
        update: None,
        news: &items,
    };
    let today = day(2026, 9, 3);

    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::News(11))
    );

    state.mark_read(11);
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::News(12))
    );
}

/// News the feed no longer carries is forgotten — the marks, the reminders and the dot with it.
#[test]
fn news_that_has_fallen_out_of_the_feed_is_forgotten_entirely() {
    let mut state = settled("0.39.0");
    let today = day(2026, 9, 3);

    for id in [11, 12, 13] {
        letters::after_shown(&mut state, Letter::News(id), today, "0.39.0");
        state.count_reminder(id);
    }
    state.mark_read(11);

    // The feed moved on: 11 fell out, 14 arrived.
    let items = [news(12), news(13), news(14)];
    let feed = FeedView {
        update: None,
        news: &items,
    };

    letters::forget_expired(&mut state, feed);

    assert!(!state.is_read(11), "the read mark of a departed item goes");
    assert_eq!(state.reminders_sent(11), 0, "and its reminders");
    assert_eq!(state.first_shown_on(11), None, "and the day it was shown");
    assert_eq!(state.reminders_sent(12), 1, "the ones still carried stay");
    assert_eq!(state.first_shown_on(13), Some(today));

    // A hand-written key that is not a number is swept away by the same pass rather than kept
    // for ever.
    state.reminders.insert("не число".to_owned(), 1);
    letters::forget_expired(&mut state, feed);
    assert!(!state.reminders.contains_key("не число"));
}

/// The count of reminders is read out of a file anybody may edit, so it saturates rather than
/// runs away.
#[test]
fn the_count_of_reminders_never_passes_two() {
    let mut state = Letters::default();

    for _ in 0..10 {
        state.count_reminder(12);
    }

    assert_eq!(state.reminders_sent(12), REMINDERS_PER_NEWS);
}

// =========================================================================================
// The feed's own cadences — FR-102
// =========================================================================================

/// Once in fifteen days, the first read on the first run, and nothing at all when the feed is
/// off.
#[test]
fn the_feed_is_read_once_in_fifteen_days_and_not_at_all_when_it_is_off() {
    let mut state = settled("0.39.0");
    let today = day(2026, 9, 3);

    assert!(
        letters::feed_read_is_due(&state, today),
        "a machine that has never read the feed reads it now"
    );

    state.feed_last_read = Some(today);
    assert!(!letters::feed_read_is_due(&state, today));
    assert!(!letters::feed_read_is_due(&state, today.plus_days(14)));
    assert!(letters::feed_read_is_due(&state, today.plus_days(15)));

    assert_eq!(letters::days_until_feed_read(&state, today), 15);
    assert_eq!(
        letters::days_until_feed_read(&state, today.plus_days(3)),
        12
    );
    assert_eq!(
        letters::days_until_feed_read(&state, today.plus_days(20)),
        0
    );

    state.feed = false;
    assert!(
        !letters::feed_read_is_due(&state, today.plus_days(100)),
        "a feed that is off is never due — no thread, no request"
    );
}

/// The switch in «От автора» appears only after both conditions of FR-102, and it is an `&&`.
#[test]
fn the_feed_switch_waits_for_a_letter_and_for_ninety_days() {
    let mut state = settled("0.39.0");
    state.first_run = Some(day(2026, 9, 1));

    assert!(
        !letters::switch_is_shown(&state, day(2027, 1, 1)),
        "ninety days on their own are not enough — the feed has shown nothing"
    );

    state.first_feed_letter = Some(day(2026, 9, 10));

    assert!(
        !letters::switch_is_shown(&state, day(2026, 11, 29)),
        "and a letter on its own is not enough — that is day eighty-nine"
    );
    assert!(
        letters::switch_is_shown(&state, day(2026, 11, 30)),
        "day ninety, with a letter behind it"
    );
}

// =========================================================================================
// The three addresses — полномочия П5, П7 и П8
// =========================================================================================

/// While an address is a placeholder, the button that would open it is disabled — and the
/// program has no feed address at all.
///
/// ⚠ This test is written so that it goes on being true when the real addresses arrive: it
/// asserts the *rule*, not the placeholder. The one assertion about the current state of the
/// constants is the last one, and it is the one that will be edited on the day the user's own
/// addresses land — which is exactly the moment somebody should be made to look at this file.
#[test]
fn a_placeholder_address_is_recognised_by_its_reserved_domain() {
    assert!(links::is_placeholder("https://example.invalid/news.toml"));
    assert!(links::is_placeholder("https://example.invalid/channel"));
    assert!(!links::is_placeholder("https://t.me/lang_switcher"));
    assert!(!links::is_placeholder("https://example.com/support"));

    // The state of the three constants today — полномочия П5, П7 и П8 of the mandate of Э32.
    assert!(links::is_placeholder(links::CHANNEL_URL));
    assert!(links::is_placeholder(links::SUPPORT_URL));
    assert!(
        links::FEED_URLS
            .iter()
            .all(|url| links::is_placeholder(url))
    );
    assert!(!links::feed_is_configured());
}

/// **SEC-03, ворота 3.** Every address this program can open is `https://`, and there are only
/// three of them plus the feed.
#[test]
fn every_address_in_the_program_is_https_and_named_here() {
    for url in links::FEED_URLS {
        assert!(url.starts_with("https://"), "{url}");
    }

    assert!(links::CHANNEL_URL.starts_with("https://"));
    assert!(links::SUPPORT_URL.starts_with("https://"));
}

// =========================================================================================
// The demonstration of «Привет» — FR-101
// =========================================================================================

/// The whole animation is a table: typed letter by letter, the key flashes, the word is fixed,
/// the key flashes again, the word comes back — and it repeats exactly.
#[test]
fn the_demonstration_types_a_word_fixes_it_and_starts_over() {
    // The typing: one more character every frame, and nothing else moves.
    for frame in 0..6 {
        let shown = letters::demo_frame(frame);

        assert_eq!(shown.shown, frame as usize + 1, "frame {frame}");
        assert!(!shown.pressed, "the key is not down while a word is typed");
        assert!(!shown.converted, "and the layout has not switched yet");
    }

    // The key goes down, and only then does the word change.
    assert!(letters::demo_frame(14).pressed);
    assert!(!letters::demo_frame(14).converted, "the press comes first");
    assert!(letters::demo_frame(16).converted, "and the word after it");
    assert_ne!(
        letters::demo_frame(16).text,
        letters::demo_frame(14).text,
        "the word really changes — that is the whole picture"
    );

    // The second press puts it back, letter for letter (FR-05).
    assert!(letters::demo_frame(30).pressed);
    assert_eq!(
        letters::demo_frame(40).text,
        letters::demo_frame(0).text,
        "and the word comes back exactly as it was"
    );

    // And the turn repeats: frame `n` and frame `n + DEMO_FRAMES` are the same picture.
    for frame in 0..letters::DEMO_FRAMES {
        assert_eq!(
            letters::demo_frame(frame),
            letters::demo_frame(frame + letters::DEMO_FRAMES),
            "the turn must repeat exactly at frame {frame}"
        );
    }
}

// =========================================================================================
// What a letter says — FR-101, and what «От автора» says — FR-103
// =========================================================================================

/// Every letter of FR-101 fills the slots it is supposed to and leaves the rest empty.
///
/// The **structure** and not the words: the words are the string tables, and
/// `tests\settings.rs` checks all hundred and thirty-four of them against both locales. What
/// this test is about is that «Привет» has a demonstration and «Спасибо» does not, that the
/// panel of «Что нового» holds three rows, that the letters out of the feed refuse to be built
/// without an entry behind them.
#[test]
fn each_letter_fills_the_slots_of_its_own_shape() {
    with_product_strings();

    let context = letters::PlanContext {
        version: "0.39.0".to_owned(),
        previous_version: String::new(),
        hotkey: "Pause".to_owned(),
        snooze_offered: true,
        item: None,
    };

    let hello = letters::plan_for(Letter::Welcome, &context);
    assert!(
        hello.demo,
        "«Привет» is the one letter with a demonstration"
    );
    assert_eq!(hello.rows.len(), 3, "and three things for the first day");
    assert!(hello.left.is_some(), "«Открыть настройки»");
    assert!(hello.accent.is_some(), "«Понятно»");
    assert!(hello.panel_buttons.is_empty(), "and nothing on the panel");
    assert!(hello.foot.is_empty(), "and no line under the buttons");

    let thanks = letters::plan_for(Letter::Thanks, &context);
    assert!(!thanks.demo);
    assert_eq!(
        thanks.panel_buttons.len(),
        2,
        "«Спасибо» carries the support page and the channel"
    );
    assert!(
        thanks.panel_buttons.iter().all(|button| !button.enabled),
        "and both are disabled while their addresses are placeholders (П7, П8)"
    );
    assert!(thanks.left.is_some(), "«Напомнить через неделю» is offered");
    assert!(
        !thanks.foot.is_empty(),
        "and the letter says it is shown once"
    );

    // FR-101: the snooze is offered once. The second showing has no button in its place.
    let second = letters::plan_for(
        Letter::Thanks,
        &letters::PlanContext {
            snooze_offered: false,
            ..context.clone()
        },
    );
    assert!(second.left.is_none(), "the snooze is offered once");

    let news = letters::plan_for(Letter::WhatsNew, &context);
    assert_eq!(news.rows.len(), 3, "«Что нового» names three changes");
    assert!(!news.demo);
    assert!(
        news.left.as_ref().is_some_and(|button| !button.enabled),
        "«Открыть канал» is drawn and disabled while the address is a placeholder"
    );

    // The two letters out of the feed have nothing to say without an entry behind them, and a
    // plan with no heading is what `show_letter` refuses to open a window for.
    for letter in [Letter::Update, Letter::News(12)] {
        assert_eq!(
            letters::plan_for(letter, &context),
            letters::LetterPlan::default(),
            "{letter:?} without a feed entry must produce no window at all"
        );
    }
}

/// A letter out of the feed says what the feed said, and its buttons follow the link the entry
/// carried.
#[test]
fn a_letter_out_of_the_feed_says_what_the_entry_said() {
    with_product_strings();

    let mut item = news(12);
    item.link = "https://example.com/post".to_owned();

    let context = letters::PlanContext {
        version: "0.39.0".to_owned(),
        previous_version: "0.38.0".to_owned(),
        hotkey: "Pause".to_owned(),
        snooze_offered: false,
        item: Some(&item),
    };

    let plan = letters::plan_for(Letter::News(12), &context);

    // ⚠ The window's own heading is the **program's** and the entry's heading stands inside
    // the panel. Task Т-32-6 moved it there deliberately: the title line is where the program
    // speaks, and a heading out of the feed in that place would let whoever holds the signing
    // key write anything at all in the program's own voice.
    assert_ne!(plan.title, item.title, "the title line is the program's");
    assert!(!plan.title.is_empty(), "and it says whose news this is");
    assert_eq!(plan.panel_title, item.title, "the heading is the author's");
    assert_eq!(plan.panel_text, item.text, "and so is the text");
    assert_eq!(plan.panel_buttons.len(), 1, "and the link it carried");
    assert!(
        plan.panel_buttons[0].enabled,
        "a real address is a live button — the rule is about placeholders, not about links"
    );
    assert!(
        plan.accent
            .as_ref()
            .is_some_and(|button| button.action == letters::Action::MarkRead),
        "«Прочитано» is the accented button of a news letter — FR-101"
    );
    assert!(
        !plan.foot.is_empty(),
        "the line under it says what «Позже» promises — FR-101, word for word from the mock-up"
    );

    // An update has nothing to mark as read: its accented button is the download page, and the
    // three steps of FR-102 stand in the panel under the author's own words.
    let update = update("0.41.0");
    let plan = letters::plan_for(
        Letter::Update,
        &letters::PlanContext {
            item: Some(&update),
            ..context.clone()
        },
    );

    assert!(
        plan.accent
            .as_ref()
            .is_some_and(|button| button.action == letters::Action::OpenDownload),
        "«Открыть страницу загрузки» is the accented button of an update — the mock-up"
    );
    assert!(plan.left.is_some(), "and it closes with the plain button");
    assert_eq!(plan.para, update.text, "the author says what changed");
    assert_eq!(plan.rows.len(), 3, "and the program says how to update");
    assert_eq!(
        plan.rows
            .iter()
            .map(|row| row.marker.as_str())
            .collect::<Vec<_>>(),
        ["1", "2", "3"],
        "the steps are numbered, not bulleted"
    );
    assert!(
        plan.rows.iter().all(|row| !row.text.trim().is_empty()),
        "every step is written in all fourteen tables"
    );
    assert!(
        plan.subtitle.contains("0.39.0"),
        "the quiet line names the version installed here, not the one announced"
    );
    assert!(
        plan.foot.contains("0.41.0"),
        "and the line under the buttons names the one in the menu"
    );
}

/// «Что нового» names the version it came from when it knows it, and does not invent one when
/// it does not — the case a machine raised from schema 5 is in.
#[test]
fn whats_new_names_the_previous_version_only_when_there_is_one() {
    with_product_strings();

    let context = letters::PlanContext {
        version: "0.39.0".to_owned(),
        previous_version: "0.38.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: false,
        item: None,
    };

    let known = letters::plan_for(Letter::WhatsNew, &context);
    let unknown = letters::plan_for(
        Letter::WhatsNew,
        &letters::PlanContext {
            previous_version: String::new(),
            ..context.clone()
        },
    );

    assert!(
        known.subtitle.contains("0.38.0"),
        "the version it came from is named: {}",
        known.subtitle
    );
    assert!(
        !unknown.subtitle.contains("0.38.0") && !unknown.subtitle.is_empty(),
        "and a machine that does not know still says something: {}",
        unknown.subtitle
    );
    assert!(
        known.title.contains("0.39.0"),
        "and the heading names the version now running: {}",
        known.title
    );
}

/// **FR-102, the two states of the «Новости и обновления» panel.** Before the switch may be
/// shown the panel says what the feed is and that the setting lives in the file; after, the
/// switch stands in that line's place and the sentence moves under it.
#[test]
fn the_author_window_has_two_states_and_the_switch_is_the_difference() {
    with_product_strings();

    let today = day(2027, 1, 1);
    let mut state = settled("0.39.0");
    state.first_run = Some(day(2026, 9, 1));

    let before = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);

    assert!(before.switch.is_none(), "no letter from the feed yet");
    assert!(
        !before.file_only.is_empty(),
        "so the file is the way to say no"
    );
    assert!(
        !before.feed_about.is_empty(),
        "and the panel says what the feed is"
    );
    assert!(before.switch_note.is_empty());
    assert!(
        !before.feed_state.is_empty(),
        "and whether it has been read"
    );
    assert!(before.download.is_none(), "no update to lead to");
    assert!(!before.letters, "and nothing in «Последние письма»");

    state.first_feed_letter = Some(day(2026, 9, 10));

    let after = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);

    assert_eq!(after.switch, Some(true), "the switch appears, and it is on");
    assert!(after.file_only.is_empty(), "and the quiet line goes");
    assert!(
        after.feed_about.is_empty(),
        "the sentence is not said twice"
    );
    assert!(!after.switch_note.is_empty(), "it moved under the switch");

    // And it follows the setting rather than being decoration.
    state.feed = false;
    assert_eq!(
        letters::author_view(&state, today, "0.39.0", FeedView::EMPTY).switch,
        Some(false)
    );
}

/// The version line of «От автора» says what is installed, and what is available when the feed
/// names something newer.
#[test]
fn the_author_window_names_the_version_and_the_one_that_is_waiting() {
    with_product_strings();

    let state = settled("0.39.0");
    let today = day(2026, 9, 3);
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    let latest = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);
    assert!(latest.version_state.contains("0.39.0"));
    assert!(latest.download.is_none());

    let waiting = letters::author_view(&state, today, "0.39.0", feed);
    assert!(waiting.version_state.contains("0.39.0"));
    assert!(
        waiting.version_state.contains("0.40.0"),
        "and the one that is waiting: {}",
        waiting.version_state
    );
    assert!(waiting.download.is_some(), "with a page to open");
    assert!(waiting.letters, "and something for «Последние письма»");
}

/// **«Последние письма»** — the update first, then the news newest first, each marked read or
/// not.
#[test]
fn the_list_shows_the_update_and_the_three_news_newest_first() {
    with_product_strings();

    let mut state = settled("0.39.0");
    let announced = update("0.40.0");
    let items = [news(11), news(12), news(13)];
    let feed = FeedView {
        update: Some(&announced),
        news: &items,
    };

    state.mark_read(12);

    let entries = letters::list_entries(&state, "0.39.0", feed);

    assert_eq!(entries.len(), 4, "one update and three news");
    assert_eq!(entries[0].news, None, "the update is first and is not news");
    assert!(!entries[0].unread, "and it is never «unread»");
    assert_eq!(entries[1].news, Some(13), "then the newest news");
    assert_eq!(entries[2].news, Some(12));
    assert_eq!(entries[3].news, Some(11));
    assert!(entries[1].unread, "13 has not been read");
    assert!(!entries[2].unread, "12 has");
    assert!(entries[3].unread, "11 has not");

    for entry in &entries {
        assert!(!entry.title.is_empty(), "every entry has a heading");
        assert!(!entry.meta.is_empty(), "and a line saying when and whether");
    }

    // An installation with no feed behind it has nothing to show, which is the whole of stage А.
    assert!(letters::list_entries(&state, "0.39.0", FeedView::EMPTY).is_empty());
}

/// A letter out of the feed can find its own entry again — task Т-32-6.
///
/// Why this exists: a change of interface language **destroys and rebuilds** the window (Э31),
/// and the rebuild starts from the letter and nothing else. Before this, the rebuilt window
/// asked `plan_for` with no entry at all and «Новость» came back as an empty frame. The test
/// walks the same road the rebuild does: publish a feed, then ask for the entry by letter.
#[test]
fn a_letter_out_of_the_feed_finds_its_entry_again_by_name() {
    with_product_strings();

    let announced = update("0.41.0");
    let items = [news(11), news(12), news(13)];

    letters::publish_feed(Some(announced.clone()), items.to_vec());

    assert_eq!(
        letters::item_of(Letter::Update).map(|item| item.version),
        Some(announced.version.clone()),
        "the update entry comes back"
    );
    assert_eq!(
        letters::item_of(Letter::News(12)).map(|item| item.id),
        Some(12),
        "and a news entry comes back by its own identifier"
    );
    assert!(
        letters::item_of(Letter::News(99)).is_none(),
        "an identifier that has fallen out of the feed finds nothing"
    );

    for letter in [Letter::Welcome, Letter::Thanks, Letter::WhatsNew] {
        assert!(
            letters::item_of(letter).is_none(),
            "{letter:?} says its own words and has no entry"
        );
    }

    // The window this rebuilds is not empty: the entry that came back fills it.
    let item = letters::item_of(Letter::News(13)).expect("13 is in the feed");
    let plan = letters::plan_for(
        Letter::News(13),
        &letters::PlanContext {
            version: "0.40.0".to_owned(),
            previous_version: String::new(),
            hotkey: String::new(),
            snooze_offered: false,
            item: Some(&item),
        },
    );

    assert!(!plan.panel_title.is_empty(), "with the entry's own heading");
    assert!(!plan.panel_text.is_empty(), "and the entry's own text");

    letters::publish_feed(None, Vec::new());
}

/// The download button obeys the placeholder rule even though its address comes out of the
/// feed rather than out of the build — полномочия П5.
#[test]
fn a_download_address_on_the_reserved_domain_is_a_dead_button() {
    with_product_strings();

    let mut announced = update("0.41.0");
    announced.link = "https://example.invalid/download".to_owned();

    let plan = letters::plan_for(
        Letter::Update,
        &letters::PlanContext {
            version: "0.40.0".to_owned(),
            previous_version: String::new(),
            hotkey: String::new(),
            snooze_offered: false,
            item: Some(&announced),
        },
    );

    assert!(
        plan.accent.as_ref().is_some_and(|button| !button.enabled),
        "a placeholder address leaves the button drawn and dead — П5"
    );

    // And a real one does not.
    announced.link = "https://example.com/download".to_owned();

    let plan = letters::plan_for(
        Letter::Update,
        &letters::PlanContext {
            version: "0.40.0".to_owned(),
            previous_version: String::new(),
            hotkey: String::new(),
            snooze_offered: false,
            item: Some(&announced),
        },
    );

    assert!(
        plan.accent.as_ref().is_some_and(|button| button.enabled),
        "an address that is not on the reserved domain is a live button"
    );
}

// =========================================================================================
// The colours and the grounds of the windows — FR-92а's vocabulary, reused
// =========================================================================================

/// The quiet labels are quiet and the rest are not — the closed vocabulary of
/// `theme::StaticColorRole`, reused rather than widened.
#[test]
fn the_colour_roles_of_the_letter_windows_follow_their_table() {
    use lang_switcher::theme::StaticColorRole;

    // The two quiet lines of the mock-up and the markers of the panel rows.
    for control in [1202, 1223, 1206, 1210, 1211, 1212, 1213] {
        assert_eq!(
            letters::label_color_role(control),
            StaticColorRole::Muted,
            "control {control} is one of the quiet ones"
        );
    }

    // The demonstration is a field of its own.
    assert_eq!(letters::label_color_role(1205), StaticColorRole::Field);

    // And everything else is the full-strength ink.
    for control in [1201, 1203, 1204, 1208, 1209, 1214, 1215, 1216, 1217] {
        assert_eq!(
            letters::label_color_role(control),
            StaticColorRole::Label,
            "control {control} is ordinary text"
        );
    }
}

/// **Every label that stands on a block is filled with the block's colour**, and the runs are
/// whole.
///
/// ⚠ The test exists because the twin of this rule was written short once: the gate of
/// `WM_DRAWITEM` named `IDC_LETTER_ROW_1` and not the four rows, and «Привет» came up with
/// three markers and one sentence. A run written short is the defect this file is watching for.
#[test]
fn every_row_of_every_run_stands_on_its_panel() {
    // The four rows of a letter and their four markers.
    for control in 1210..1218 {
        assert!(
            letters::label_stands_on_a_panel(control),
            "control {control} is a row of a letter's panel"
        );
    }

    // The three labels of each of the four entries of «Последние письма».
    for control in (1231..1235).chain(1235..1239).chain(1239..1243) {
        assert!(
            letters::label_stands_on_a_panel(control),
            "control {control} is a label of an entry"
        );
    }

    // The paragraphs of the three panels of «От автора».
    for control in [1264, 1268, 1275, 1269, 1270, 1273, 1277] {
        assert!(
            letters::label_stands_on_a_panel(control),
            "control {control} stands on a panel of «От автора»"
        );
    }

    // And what does **not**: the head of a letter, the demonstration, the line under the
    // buttons, the name and version of «От автора», the footnote of the list.
    for control in [1201, 1202, 1203, 1204, 1205, 1206, 1223, 1261, 1262, 1252] {
        assert!(
            !letters::label_stands_on_a_panel(control),
            "control {control} stands on the window's own ground"
        );
    }
}
