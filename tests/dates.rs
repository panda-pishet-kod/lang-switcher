//! The calendar the dates of the letters are printed in — **FR-101**, task Т-33-1,
//! решение 103.1.
//!
//! One rule: a date shown by this program is a **Gregorian** date at every interface locale;
//! the name of the month and the digits are the locale's own. The rule exists because a letter
//! is dated by an event of the program — a version came out, a news item was written — and a
//! person reading it in Arabic was being shown «٢٢ ربيع الأول» for what the release notes call
//! the fourth of September.
//!
//! ⚠ **The baseline is measured here, not written out as literals.** Thirteen of the fourteen
//! locales must print **exactly** what they printed before this task, and the honest way to say
//! that is to ask Windows the old question — `GetDateFormatEx` with the picture the old
//! `format_date` used — and compare. Literals would tie these tests to one machine's NLS data
//! and would say nothing about whether the string *changed*.
//!
//! A file of its own and not a case in `tests\letters.rs`: every test here moves the
//! process-wide interface locale, and `cargo test` runs the tests of one binary on parallel
//! threads. The gate below serialises them; a separate binary keeps them away from the tests
//! that read strings in Russian.

use lang_switcher::letters::{self, Date};
use lang_switcher::settings::{self, Language};

use std::sync::{Mutex, MutexGuard};
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::Globalization::{
    ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx, GetLocaleInfoEx, LOCALE_ICALENDARTYPE,
};
use windows::core::{PCWSTR, w};

/// Serialises the tests that publish an interface locale — the discipline of
/// `tests\settings.rs`, for the same reason: one program, one interface, and one atomic
/// holding which locale that is.
static LOCALE: Mutex<()> = Mutex::new(());

/// Takes the gate. Every test here calls it first and puts `ru` back when it is done.
fn locale_gate() -> MutexGuard<'static, ()> {
    LOCALE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A date this file names by hand: no test here computes a day from the machine's clock.
fn day(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd(year, month, day).expect("the test names a real day")
}

/// A NUL-terminated UTF-16 buffer — the test's own, because `settings::wide` belongs to the
/// crate and a test is not the crate.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// **The old question, asked directly** — `GetDateFormatEx` with the picture `letters::format_date`
/// used before task Т-33-1, and therefore the string this locale printed on `e89db7f`.
fn baseline(tag: &str, date: Date) -> String {
    let system = SYSTEMTIME {
        wYear: u16::try_from(date.year()).unwrap_or(0),
        wMonth: u16::try_from(date.month()).unwrap_or(1),
        wDay: u16::try_from(date.day()).unwrap_or(1),
        ..SYSTEMTIME::default()
    };

    let locale = wide(tag);
    let mut buffer = [0u16; 64];

    // SAFETY: `locale` is a NUL-terminated buffer of this frame, `system` a live local, the
    // picture a static literal, and the buffer a live local whose length the call is told.
    let written = unsafe {
        GetDateFormatEx(
            PCWSTR(locale.as_ptr()),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&system),
            w!("d MMMM"),
            Some(&mut buffer),
            PCWSTR::null(),
        )
    };

    assert!(written > 0, "GetDateFormatEx refused the locale «{tag}»");

    let units = usize::try_from(written).unwrap_or(1).saturating_sub(1);

    String::from_utf16(&buffer[..units]).expect("Windows answers in UTF-16")
}

/// Which calendar this locale uses by default — `LOCALE_ICALENDARTYPE`.
///
/// The number is what decides whether a locale is one of the thirteen that must not change:
/// the Gregorian family is 1, 2 and 9…12 (`winnls.h`), and everything else — 6 Hijri,
/// 7 Thai, 8 Hebrew, 23 Umm al-Qura — is a calendar whose dates are other numbers entirely.
fn default_calendar(tag: &str) -> u32 {
    let locale = wide(tag);
    let mut buffer = [0u16; 16];

    // SAFETY: both buffers are live locals of this frame and the call is told the length of
    // the one it writes into.
    let written = unsafe {
        GetLocaleInfoEx(
            PCWSTR(locale.as_ptr()),
            LOCALE_ICALENDARTYPE,
            Some(&mut buffer),
        )
    };

    assert!(written > 0, "GetLocaleInfoEx refused the locale «{tag}»");

    let units = usize::try_from(written).unwrap_or(1).saturating_sub(1);

    String::from_utf16(&buffer[..units])
        .expect("Windows answers in UTF-16")
        .parse()
        .expect("LOCALE_ICALENDARTYPE is a number")
}

/// Whether a locale's own calendar is a Gregorian one — the thirteen whose strings must not
/// move a byte.
fn is_gregorian(calendar: u32) -> bool {
    matches!(calendar, 1 | 2 | 9..=12)
}

/// **Т-33-1** — the thirteen locales whose own calendar is Gregorian print exactly what they
/// printed before, on a one-digit day and on a two-digit one.
///
/// The two dates are not decoration: `d` has no leading zero, and a picture rebuilt by hand
/// could easily grow one. September and March are two different genitive forms in the Slavic
/// locales — «сентября» and «марта» — so a month name taken in the nominative case would be
/// caught here rather than by the eye.
#[test]
fn every_locale_with_a_gregorian_calendar_prints_exactly_what_it_printed_before() {
    let _gate = locale_gate();

    for date in [day(2026, 9, 4), day(2026, 3, 22), day(2026, 12, 31)] {
        for language in Language::ALL {
            let tag = language.tag();
            let calendar = default_calendar(tag);

            if !is_gregorian(calendar) {
                println!("{tag}: календарь {calendar} — не григорианский, строка меняется");
                continue;
            }

            settings::set_ui_language(language);

            let now = letters::format_date(date);
            let before = baseline(tag, date);

            println!("{tag} {date}: «{now}» (было «{before}»)");

            assert_eq!(
                now, before,
                "{tag} on {date} must print what it printed before task Т-33-1"
            );
        }
    }

    settings::set_ui_language(Language::Ru);
}

/// **Т-33-1, the red one** — Arabic prints a Gregorian month and no month of the Hijri year.
///
/// On `e89db7f` this test fails: `GetDateFormatEx` takes the locale's own calendar, which for
/// `ar` is Umm al-Qura (`LOCALE_ICALENDARTYPE` = 23), and 2026-09-04 reads «22 ربيع الأول».
///
/// The positive control is the second half of the test: the baseline for the same day **must**
/// be the Hijri month, or this file is comparing a locale that never had the defect and the
/// green tick means nothing.
#[test]
fn the_arabic_locale_prints_a_gregorian_month_and_not_a_hijri_one() {
    let _gate = locale_gate();

    settings::set_ui_language(Language::Ar);

    for (date, gregorian, hijri) in [
        (day(2026, 9, 4), "سبتمبر", "ربيع"),
        (day(2026, 3, 22), "مارس", "شوال"),
    ] {
        let now = letters::format_date(date);
        let before = baseline("ar", date);

        println!("ar {date}: «{now}» (было «{before}»)");

        // The positive control first: the old road really did go to the Hijri calendar.
        assert!(
            before.contains(hijri),
            "the baseline for {date} is «{before}» and carries no «{hijri}» — this test would \
             pass on a machine that never had the defect"
        );

        assert!(
            now.contains(gregorian),
            "ar on {date} must name the Gregorian month «{gregorian}»: «{now}»"
        );
        assert!(
            !now.contains(hijri),
            "ar on {date} still carries the Hijri month «{hijri}»: «{now}»"
        );
        assert!(
            now.starts_with(&date.day().to_string()),
            "ar on {date} must start with the day of the Gregorian month: «{now}»"
        );
    }

    settings::set_ui_language(Language::Ru);
}

/// **Т-33-1** — Hebrew names the Gregorian month too.
///
/// ⚠ **`he` never had the defect, and saying so is the point of this test.** The mandate
/// expected the Hebrew locale to be printing dates of the Hebrew year; it was not — the
/// default calendar of `he` is Gregorian (1) and the Hebrew calendar is only its *optional*
/// one, which nothing here asks for. The test stands as a guard: a repair that reached for
/// `DATE_USE_ALT_CALENDAR` would move this locale to «כ"ב אלול» and be caught here.
#[test]
fn the_hebrew_locale_names_the_gregorian_month() {
    let _gate = locale_gate();

    settings::set_ui_language(Language::He);

    let now = letters::format_date(day(2026, 9, 4));

    println!("he: «{now}»");

    assert!(
        now.contains("ספטמבר"),
        "he must name the Gregorian September: «{now}»"
    );
    assert!(
        now.starts_with('4'),
        "he must start with the day of the Gregorian month: «{now}»"
    );

    settings::set_ui_language(Language::Ru);
}

/// **Т-33-1** — every one of the fourteen answers something, and none of them falls back to
/// `YYYY-MM-DD`.
///
/// The fallback exists (NFR-13: a refused call must leave a date rather than a panic), and a
/// repair that silently took it everywhere would look tidy in the code and print ISO dates in
/// every letter. This is the test that would notice.
#[test]
fn no_locale_falls_back_to_the_iso_form() {
    let _gate = locale_gate();

    let date = day(2026, 9, 4);
    let iso = date.to_string();

    for language in Language::ALL {
        settings::set_ui_language(language);

        let now = letters::format_date(date);

        println!("{}: «{now}»", language.tag());

        assert_ne!(
            now,
            iso,
            "{} fell back to the ISO form — the calendar was refused",
            language.tag()
        );
        assert!(!now.is_empty(), "{} printed nothing at all", language.tag());
    }

    settings::set_ui_language(Language::Ru);
}
