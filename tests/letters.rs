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
use lang_switcher::settings::Letters;

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
