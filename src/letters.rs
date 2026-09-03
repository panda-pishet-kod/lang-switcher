//! Letters from the author — FR-101, FR-102, FR-103 and FR-104.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: «Письма от автора:
//! расписание и состояние писем, чтение и проверка ленты, окно «От автора», мастер
//! обращения». Stage А of этап Э32 builds the half that never touches the network: the
//! calendar this program counts days in, the state of `[letters]`, the schedule that decides
//! which letter is due, and the three addresses the windows link to.
//!
//! # What this module may not do — SEC-03, вопрос 101 п. 1 and п. 3
//!
//! **Nothing here downloads or runs anything, ever.** The «Обновление» letter of FR-101 tells
//! a person that a newer version exists and walks them to the download page; the fetching and
//! the installing are theirs. The single network operation this program is allowed at all —
//! one signed read of the author's feed, no more often than once in fifteen days — lives in
//! the submodule `feed` of stage Б and nowhere else, so that «which code can open a socket»
//! is a question with a one-line answer.
//!
//! # Why the schedule is a pure function
//!
//! [`due`] takes the state, the day, the version, what the feed last said and whether the
//! moment is a quiet one, and answers with a letter or with nothing. It reads no clock, opens
//! no window and touches no configuration, which is what lets every rule of FR-101 — the
//! priority order, the one letter a day, the two reminders, the single snooze — be checked in
//! a unit test against a state written out by hand, on a machine where nothing is installed.
//! The impure half is the caller's: [`today`] asks the system for the date, and the windows
//! and the tray do the showing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, Serializer};

use crate::settings::Letters;

// =========================================================================================
// 1. The calendar
// =========================================================================================

/// A civil date — year, month and day, and nothing else.
///
/// Every cadence of FR-101 and FR-102 is counted in whole days (thirty to «Спасибо», seven to
/// a snooze, seven and fourteen to the two reminders, fifteen between feed reads, ninety
/// before the feed switch appears), so a date is what the state stores and a day is the unit
/// the arithmetic works in. There is no time of day anywhere in `[letters]`: a letter that
/// arrived at 09:00 and one that arrived at 23:00 are the same day's letter.
///
/// # Why this type exists at all
///
/// The dependency list of section 3.2 is closed (SEC-03) and holds no date crate, and the two
/// `windows` features вопрос 101 п. 3 authorises are the network and the cryptography of the
/// feed — not `Win32_System_SystemInformation`, where `GetLocalTime` lives. So the calendar is
/// arithmetic this file does itself: [`Date::to_days`] and [`Date::from_days`] are the
/// days-from-civil pair, exact for every year the type can hold, and [`today`] asks
/// `GetDateFormatEx` — which is in `Win32_Globalization`, a feature this program has carried
/// since FR-94 — for the **local** date as text and parses it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

impl Date {
    /// A date from its three parts, or `None` for a combination that is not one.
    ///
    /// The month is checked against the length of that month in that year — the leap rule
    /// included — rather than against thirty-one, so that `2026-02-30` is refused where it is
    /// made instead of arriving somewhere far away as an off-by-two.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }

        Some(Self { year, month, day })
    }

    /// The year.
    pub fn year(self) -> i32 {
        self.year
    }

    /// The month, 1 to 12.
    pub fn month(self) -> u32 {
        self.month
    }

    /// The day of the month, 1 to 31.
    pub fn day(self) -> u32 {
        self.day
    }

    /// Days since 1970-01-01, negative before it — the days-from-civil algorithm.
    ///
    /// The shift by four hundred years at the top is what makes the integer division behave
    /// for dates before the epoch as well as after it: the calendar repeats exactly every four
    /// hundred years, so moving the era boundary to a multiple of four hundred and taking it
    /// off again at the end costs one addition and removes every negative-remainder case.
    pub fn to_days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month);
        let day = i64::from(self.day);

        // Day of the year counted from the first of March, which is where the leap day falls
        // at the end and therefore where the pattern of month lengths is regular.
        let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

        era * 146_097 + day_of_era - 719_468
    }

    /// The date `days` days after 1970-01-01 — the exact inverse of [`Date::to_days`].
    pub fn from_days(days: i64) -> Self {
        let shifted = days + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_of_year = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_of_year + 2) / 5 + 1;
        let month = month_of_year + if month_of_year < 10 { 3 } else { -9 };

        Self {
            year: i32::try_from(year + i64::from(month <= 2)).unwrap_or(i32::MAX),
            month: u32::try_from(month).unwrap_or(1),
            day: u32::try_from(day).unwrap_or(1),
        }
    }

    /// This date moved by whole days, forwards for a positive `days` and back for a negative
    /// one.
    pub fn plus_days(self, days: i64) -> Self {
        Self::from_days(self.to_days() + days)
    }

    /// How many days lie between `earlier` and this date — negative when this date is the
    /// earlier of the two.
    pub fn days_since(self, earlier: Self) -> i64 {
        self.to_days() - earlier.to_days()
    }

    /// Reads a date written `YYYY-MM-DD`, and nothing else.
    ///
    /// Deliberately strict about the shape — four digits, a dash, two digits, a dash, two
    /// digits — because the two places it is used are both places where a loose read would be
    /// a defect rather than a kindness: the answer `GetDateFormatEx` gave to a picture this
    /// file wrote itself, and a value out of the configuration file, where a string that is
    /// not a date is exactly the kind of damage section 7 wants noticed.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();

        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }

        let year: i32 = text.get(0..4)?.parse().ok()?;
        let month: u32 = text.get(5..7)?.parse().ok()?;
        let day: u32 = text.get(8..10)?.parse().ok()?;

        Self::from_ymd(year, month, day)
    }
}

impl std::fmt::Display for Date {
    /// `YYYY-MM-DD` — the form section 7 shows and the form [`Date::parse`] reads back.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// How many days that month of that year has — the Gregorian leap rule spelled out.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.rem_euclid(4) == 0
            && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0) =>
        {
            29
        }
        2 => 28,
        _ => 0,
    }
}

impl Serialize for Date {
    /// Writes a **bare** TOML date — `first_run = 2026-09-02`, exactly as section 7 prints it,
    /// and not a quoted string.
    ///
    /// The road to that is `toml::value::Datetime`: the `toml` crate recognises its own type by
    /// a private struct name and emits a date literal for it, and there is no other way to ask
    /// for one through serde. Nothing of the time or the offset is set — a `Datetime` carrying
    /// only a date *is* a TOML local date.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        toml::value::Datetime {
            date: Some(toml::value::Date {
                year: u16::try_from(self.year).unwrap_or(0),
                month: u8::try_from(self.month).unwrap_or(1),
                day: u8::try_from(self.day).unwrap_or(1),
            }),
            time: None,
            offset: None,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Date {
    /// Reads a bare TOML date, and a quoted `"YYYY-MM-DD"` as well.
    ///
    /// The second form is not the one this program writes; it is the one a person writes by
    /// hand. `[letters] feed` is a field section 7 tells people to edit for the first ninety
    /// days (FR-102), so the section is one people open — and refusing a whole configuration
    /// over a pair of quotation marks would move that file to `.bad` and take every other
    /// setting in it with it. `deserialize_any` is what makes both readable: TOML is
    /// self-describing, so the deserialiser says which of the two it has and the visitor below
    /// answers for each.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DateVisitor;

        impl<'de> serde::de::Visitor<'de> for DateVisitor {
            type Value = Date;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a date, written 2026-09-02 or \"2026-09-02\"")
            }

            /// The quoted form.
            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Date, E> {
                Date::parse(text).ok_or_else(|| E::custom("a date is written YYYY-MM-DD"))
            }

            /// The bare form: `toml` hands its own datetime over as a one-entry map whose key
            /// is private to the crate, so the value is taken by asking `toml` itself to read
            /// it — this file never spells that key.
            fn visit_map<M: serde::de::MapAccess<'de>>(self, map: M) -> Result<Date, M::Error> {
                let datetime = toml::value::Datetime::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;

                let date = datetime
                    .date
                    .ok_or_else(|| serde::de::Error::custom("a date, not a time of day"))?;

                Date::from_ymd(
                    i32::from(date.year),
                    u32::from(date.month),
                    u32::from(date.day),
                )
                .ok_or_else(|| serde::de::Error::custom("no such day in that month"))
            }
        }

        deserializer.deserialize_any(DateVisitor)
    }
}

/// Today, by the clock of the machine this is running on, in **local** time.
///
/// `None` when the system refuses to say, which is NFR-13's answer to a call that failed:
/// every caller of this treats it as «no schedule runs this time round», so a refused clock
/// costs one skipped check and never a wrong letter.
///
/// # Why the date arrives as text
///
/// `GetLocalTime` would be the direct route and it is not available: it lives in the
/// `Win32_System_SystemInformation` feature of the `windows` crate, and вопрос 101 п. 3
/// authorises **two** new features, both of them for the feed. `GetDateFormatEx` is in
/// `Win32_Globalization`, which this program has carried since FR-94, and with no `SYSTEMTIME`
/// of its own it formats *the current local date* — that is the documented meaning of a null
/// `lpDate`. The picture is written by this file (`yyyy'-'MM'-'dd`) and the locale is the
/// invariant one, so neither the digits nor the calendar are the user's: an Arabic or a Thai
/// user's own locale would otherwise be entitled to answer in another calendar altogether.
pub fn today() -> Option<Date> {
    use windows::Win32::Globalization::{
        ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx, LOCALE_NAME_INVARIANT,
    };
    use windows::core::{PCWSTR, w};

    // `YYYY-MM-DD` and the terminator: eleven units is the exact answer, and the buffer is
    // asked for a little more so that a longer answer is truncated rather than refused.
    let mut buffer = [0u16; 32];

    // SAFETY: the locale name and the picture are static NUL-terminated literals of this
    // image; the buffer is a live local of this frame and its length is what the call is told;
    // `None` for the date is the documented «the current local date», and `PCWSTR::null()` is
    // the documented value of the reserved calendar argument. The call writes into the buffer
    // and reads nothing else of ours.
    let written = unsafe {
        GetDateFormatEx(
            LOCALE_NAME_INVARIANT,
            ENUM_DATE_FORMATS_FLAGS(0),
            None,
            w!("yyyy'-'MM'-'dd"),
            Some(&mut buffer),
            PCWSTR::null(),
        )
    };

    if written <= 0 {
        crate::app::report_non_critical("GetDateFormatEx", &windows::core::Error::from_thread());
        return None;
    }

    // The count includes the terminator, which is not part of the text.
    let units = usize::try_from(written).ok()?.saturating_sub(1);
    let text = String::from_utf16(buffer.get(..units)?).ok()?;

    Date::parse(&text)
}

// =========================================================================================
// 2. The cadences of FR-101 and FR-102 — вопрос 101, and not one of them is a preference
// =========================================================================================

/// Days from the first run to the «Спасибо» letter — FR-101.
pub const THANKS_AFTER_DAYS: i64 = 30;

/// Days the one snooze of «Спасибо» moves it by — FR-101, «Напомнить через неделю».
pub const SNOOZE_DAYS: i64 = 7;

/// Days between two reads of the feed — FR-102.
pub const FEED_EVERY_DAYS: i64 = 15;

/// When the two reminders of an unread «Новость» fall, counted from the day it was first
/// shown — FR-101.
pub const REMINDER_DAYS: [i64; 2] = [7, 14];

/// How many reminders one news item ever gets. The length of [`REMINDER_DAYS`], named so that
/// the rule reads as a rule where it is applied.
pub const REMINDERS_PER_NEWS: u32 = REMINDER_DAYS.len() as u32;

/// Days from the first run before the feed switch may appear in «От автора» — FR-102. The
/// switch also waits for the first letter the feed produced, and the two conditions are `&&`.
pub const SWITCH_AFTER_DAYS: i64 = 90;

/// How many news items the program stores and shows at once — FR-101.
pub const NEWS_KEPT: usize = 3;

// =========================================================================================
// 3. What a letter is
// =========================================================================================

/// The five letters of FR-101.
///
/// «Новость» carries the identifier of the feed entry it is about, because there can be up to
/// [`NEWS_KEPT`] of them and every rule that follows — read, reminded, expired — is about one
/// of them by number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Letter {
    /// «Привет» — the first run, when there was no configuration file to start from.
    Welcome,
    /// «Что нового» — the build's version differs from the one the state remembers.
    WhatsNew,
    /// «Обновление» — the feed named a version newer than this one.
    Update,
    /// «Новость» — an unread entry of the feed, by identifier.
    News(u64),
    /// «Спасибо» — thirty days after the first run.
    Thanks,
}

/// What the state of `[letters] thanks` can be — FR-101.
///
/// # An open vocabulary, and that is the whole of the downgrade cost
///
/// A word this build does not know reads as [`Thanks::Pending`] instead of refusing the
/// document. That is the shape `theme` already uses for `general.theme` and deliberately not
/// the shape `[replacement] method` uses: a **closed** set costs a schema number every time it
/// grows, because a build that has not been updated meets the new word, refuses the whole
/// file and moves somebody's configuration to `.bad`. Nothing about this field is worth that,
/// and the worst an unknown word can do here is show one letter one more time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Thanks {
    /// Not shown yet.
    #[default]
    Pending,
    /// Shown once and put off by a week — «Напомнить через неделю», which is allowed once.
    Snoozed,
    /// Done with: shown and closed, or snoozed and shown again.
    Done,
}

impl Thanks {
    /// The word section 7 writes for this state.
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Snoozed => "snoozed",
            Self::Done => "done",
        }
    }

    /// The state a word of section 7 means — anything else is [`Thanks::Pending`], see the
    /// type's own note.
    pub fn from_config_str(text: &str) -> Self {
        match text {
            "snoozed" => Self::Snoozed,
            "done" => Self::Done,
            _ => Self::Pending,
        }
    }
}

/// One entry of the author's feed as the schedule sees it — FR-102.
///
/// Stage А never builds one of these: the feed is stage Б, and until then [`FeedView::EMPTY`]
/// is what every caller passes. The type is here now because [`due`] takes it now, and a
/// signature that does not have to change when the feed arrives is a signature the tests of
/// stage А are already written against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedItem {
    /// The `id` of FR-102: unique, ascending, and what «прочитано» is remembered by.
    pub id: u64,
    /// The day the author dated the entry.
    pub date: Option<Date>,
    /// The heading, already chosen for the interface language.
    pub title: String,
    /// The body, already chosen for the interface language.
    pub text: String,
    /// The link the entry points at, if it has one.
    pub link: String,
    /// The version this entry announces — `Some` only for the one `update` entry.
    pub version: Option<String>,
}

/// What the last successful read of the feed left behind — FR-102.
///
/// A borrowed view rather than an owned copy: the schedule reads it and keeps nothing.
#[derive(Debug, Clone, Copy)]
pub struct FeedView<'a> {
    /// The single `update` entry, if the feed carried one.
    pub update: Option<&'a FeedItem>,
    /// The `news` entries the program keeps — at most [`NEWS_KEPT`], oldest first.
    pub news: &'a [FeedItem],
}

impl FeedView<'_> {
    /// The view of a program that has never read the feed — and the whole of what stage А
    /// passes.
    pub const EMPTY: FeedView<'static> = FeedView {
        update: None,
        news: &[],
    };
}

// =========================================================================================
// 4. The schedule — FR-101, one pure function
// =========================================================================================

/// Which letter, if any, is due right now — **the whole of the schedule of FR-101**.
///
/// * `state` — the `[letters]` section as it stands.
/// * `today` — the day, from [`today`].
/// * `version` — this build's version as `VERSIONINFO` spells it, `"0.39.0"`.
/// * `feed` — what the last successful feed read left; [`FeedView::EMPTY`] in stage А.
/// * `quiet` — whether this is a quiet moment: the system accepts notifications and the
///   keyboard has been still for [`crate::letters::QUIET_IDLE_SECONDS`] seconds.
///
/// # The order the rules are in, and why each is where it is
///
/// 1. **«Привет» first and outside every gate.** It is the one letter that is not announced by
///    a notification, does not wait for a quiet moment and does not spend the day's single
///    slot: FR-101 shows it «сразу», because the program has just been installed and the
///    person is looking at the screen they installed it from.
/// 2. **A quiet moment.** Everything else is a knock at somebody's door.
/// 3. **One letter a day.** The four remaining letters share one slot, and the slot is spent
///    by whichever of them is shown.
/// 4. **«Что нового» → «Обновление» → «Новость» → «Спасибо»** — the priority of FR-101, from
///    the letter about the program the person is running to the letter about the author.
pub fn due(
    state: &Letters,
    today: Date,
    version: &str,
    feed: FeedView<'_>,
    quiet: bool,
) -> Option<Letter> {
    if !state.welcome_shown {
        return Some(Letter::Welcome);
    }

    if !quiet || state.last_letter == Some(today) {
        return None;
    }

    if state.last_seen_version != version {
        return Some(Letter::WhatsNew);
    }

    if update_is_due(state, version, feed) {
        return Some(Letter::Update);
    }

    if let Some(id) = news_due(state, today, feed) {
        return Some(Letter::News(id));
    }

    if thanks_is_due(state, today) {
        return Some(Letter::Thanks);
    }

    None
}

/// Whether the feed named a version newer than this one that has not been announced yet —
/// FR-101, «один раз на версию».
pub fn update_is_due(state: &Letters, version: &str, feed: FeedView<'_>) -> bool {
    let Some(update) = feed.update else {
        return false;
    };

    let Some(announced) = update.version.as_deref() else {
        return false;
    };

    version_is_newer(announced, version) && state.latest_known != announced
}

/// Whether an update the feed named is still ahead of this build — the condition the temporary
/// menu entry «Доступна версия X…» of FR-91 stands on, which outlives the letter.
pub fn update_is_pending<'a>(version: &str, feed: FeedView<'a>) -> Option<&'a FeedItem> {
    let update = feed.update?;

    version_is_newer(update.version.as_deref()?, version).then_some(update)
}

/// Which unread news item is due to be shown or reminded of — FR-101.
///
/// Oldest first, so that a person who has been away meets the news in the order it was
/// written. An item that has never been shown is due the moment it arrives; one that has been
/// shown and not acknowledged is due again seven and then fourteen days after **its own first
/// showing**, and after the second reminder never again — from there on it is the dot on the
/// icon (FR-90) and the entry in the menu (FR-91) until «Прочитано» is pressed.
pub fn news_due(state: &Letters, today: Date, feed: FeedView<'_>) -> Option<u64> {
    feed.news
        .iter()
        .filter(|item| !state.is_read(item.id))
        .find(|item| match state.first_shown_on(item.id) {
            None => true,
            Some(first) => {
                let reminders = state.reminders_sent(item.id);

                reminders < REMINDERS_PER_NEWS
                    && REMINDER_DAYS
                        .get(reminders as usize)
                        .is_some_and(|after| today.days_since(first) >= *after)
            }
        })
        .map(|item| item.id)
}

/// Whether «Спасибо» is due — FR-101: thirty days after the first run, and not once the letter
/// is done with.
pub fn thanks_is_due(state: &Letters, today: Date) -> bool {
    state.thanks != Thanks::Done && state.thanks_due.is_some_and(|due| today >= due)
}

/// Whether the letter «Спасибо» may still offer «Напомнить через неделю» — FR-101 allows the
/// snooze **once**, so the second showing has the button disabled.
pub fn snooze_is_offered(state: &Letters) -> bool {
    state.thanks == Thanks::Pending
}

/// Whether the feed switch belongs in the «От автора» window yet — FR-102.
///
/// Both halves, and they are an `&&`: the feed has produced a letter at least once, and the
/// program has been on this machine for [`SWITCH_AFTER_DAYS`] days. Until then the window
/// shows the quiet line that says the setting is edited in the file — which is the same shape
/// the replacement method and the timings of примечание к FR-92 are in.
pub fn switch_is_shown(state: &Letters, today: Date) -> bool {
    state.first_feed_letter.is_some()
        && state
            .first_run
            .is_some_and(|first| today.days_since(first) >= SWITCH_AFTER_DAYS)
}

/// Whether the feed may be read today — FR-102: once in fifteen days, and the first read
/// happens on the first run, when there is nothing to count from.
pub fn feed_read_is_due(state: &Letters, today: Date) -> bool {
    state.feed
        && state
            .feed_last_read
            .is_none_or(|last| today.days_since(last) >= FEED_EVERY_DAYS)
}

/// How many days are left until the next feed read — what the «Новости и обновления» panel
/// says out loud. Zero when a read is due now.
pub fn days_until_feed_read(state: &Letters, today: Date) -> i64 {
    state
        .feed_last_read
        .map_or(0, |last| (FEED_EVERY_DAYS - today.days_since(last)).max(0))
}

/// Whether the icon carries the dot of FR-90 — an unread news item among the ones kept.
pub fn has_unread_news(state: &Letters, feed: FeedView<'_>) -> bool {
    feed.news.iter().any(|item| !state.is_read(item.id))
}

/// Compares two version strings the way FR-101 needs them compared — part by part as numbers,
/// so that `0.10.0` is newer than `0.9.0` and a text comparison's answer is not taken.
///
/// A part that is not a number counts as zero, and a version with fewer parts is padded with
/// them: `0.39` and `0.39.0` are the same version. Neither case can arise from a feed the
/// author's own script signed — it checks the shape — and both have to answer something
/// rather than panic, because the string arrives from outside this program (SEC-05).
pub fn version_is_newer(candidate: &str, installed: &str) -> bool {
    let mut theirs = candidate.split('.');
    let mut ours = installed.split('.');

    for _ in 0..4 {
        let left: u32 = theirs.next().unwrap_or("0").trim().parse().unwrap_or(0);
        let right: u32 = ours.next().unwrap_or("0").trim().parse().unwrap_or(0);

        if left != right {
            return left > right;
        }
    }

    false
}

// =========================================================================================
// 5. What showing a letter does to the state — FR-101
// =========================================================================================

/// Fills in what a state that has never been used yet cannot know — the day it started
/// counting from.
///
/// Called once per session, before the first [`due`], and it is the answer to a question the
/// migration of schema 5 → 6 deliberately does not answer: `Config::migrate` is a pure
/// function of the document and reads no clock, so the day of the migration is written here,
/// on the first run after it, which is the same day.
///
/// Two things happen, and only when `first_run` is empty:
///
/// * `first_run` becomes today and `thanks_due` becomes thirty days from now (FR-101);
/// * a machine that has **not** shown «Привет» yet — a fresh install, where
///   `Config::for_a_first_run` left the flag off — is marked as having already seen this
///   version's news. Nothing about a program installed today is «что нового», and without
///   this line the «Что нового» letter would follow «Привет» within the hour.
///
/// Answers whether anything was written, because the caller has to save the file when it was.
pub fn initialise(state: &mut Letters, today: Date, version: &str) -> bool {
    if state.first_run.is_some() {
        return false;
    }

    state.first_run = Some(today);
    state.thanks_due = Some(today.plus_days(THANKS_AFTER_DAYS));

    if !state.welcome_shown {
        state.last_seen_version = version.to_owned();
    }

    true
}

/// Records that a letter has just been put on the screen — FR-101, and the only place the
/// state of the letters moves on its own.
///
/// The day is spent by every letter but «Привет», which FR-101 exempts from the one-a-day
/// rule; and each letter marks the thing that stops it coming back:
///
/// * «Привет» — shown, once and for ever;
/// * «Что нового» — the version it was about;
/// * «Обновление» — the version it announced, so the letter is one per version while the menu
///   entry of FR-91 stays until the person updates;
/// * «Новость» — the day of the **first** showing, and after that the count of reminders;
/// * «Спасибо» — done with, unless the person presses «Напомнить через неделю», which
///   [`snooze_thanks`] answers for.
pub fn after_shown(state: &mut Letters, letter: Letter, today: Date, version: &str) {
    if letter != Letter::Welcome {
        state.last_letter = Some(today);
    }

    match letter {
        Letter::Welcome => state.welcome_shown = true,
        Letter::WhatsNew => state.last_seen_version = version.to_owned(),
        Letter::Update => state.latest_known = version.to_owned(),
        Letter::News(id) => {
            if state.first_shown_on(id).is_none() {
                state.set_first_shown(id, today);
                state.note_feed_letter(today);
            } else {
                state.count_reminder(id);
            }
        }
        Letter::Thanks => {
            state.thanks = Thanks::Done;
        }
    }
}

/// «Напомнить через неделю» — FR-101, and it may be pressed once.
///
/// The letter has already been marked [`Thanks::Done`] by [`after_shown`], because that is
/// what the closing of it means; this puts it back to [`Thanks::Snoozed`] and moves the day.
/// The second showing has the button disabled ([`snooze_is_offered`]), so there is no second
/// snooze to guard against here.
pub fn snooze_thanks(state: &mut Letters, today: Date) {
    state.thanks = Thanks::Snoozed;
    state.thanks_due = Some(today.plus_days(SNOOZE_DAYS));
}

/// «Прочитано» — FR-101, and the only thing that ever makes a news item read.
pub fn mark_read(state: &mut Letters, id: u64) {
    state.mark_read(id);
}

/// Forgets everything about news items that are no longer among the ones the feed keeps —
/// FR-101, «старшие выбывают… вместе с напоминаниями».
///
/// Run after every successful feed read. Without it the three maps of `[letters]` would grow
/// for the life of the installation, and the dot on the icon would go on burning for an item
/// nobody can open any more.
pub fn forget_expired(state: &mut Letters, feed: FeedView<'_>) {
    let kept: Vec<u64> = feed.news.iter().map(|item| item.id).collect();

    state.retain_news(&kept);
}

// =========================================================================================
// 6. The three addresses — вопрос 101 п. 6, полномочия П5, П7 и П8
// =========================================================================================

/// The addresses the program links to, and the one rule that governs all three.
///
/// # Placeholders, and why a button is drawn disabled rather than hidden
///
/// Three addresses are not settled yet: the feed's, the channel's and the support page's. The
/// user's answer (полномочия П5, П7 и П8 of the mandate) was to build the windows now and put
/// a placeholder in each address — «кнопка пусть будет, вместо пути заглушка». So each is a
/// constant of this module holding a `example.invalid` address, [`is_placeholder`] recognises
/// them by their domain, and every button that would open one is drawn in the disabled role
/// the own-drawn buttons already have. The window is the finished window; what is missing is
/// visibly missing, and the day the addresses arrive they are three string literals and no
/// code at all.
///
/// `example.invalid` and not a made-up host: RFC 2606 reserves `.invalid` precisely so that it
/// never resolves. A placeholder that fell out of this rule and reached the network would fail
/// slowly, against somebody else's machine; this one fails at once, against nobody's.
pub mod links {
    /// The reserved domain every placeholder address is under.
    const PLACEHOLDER_HOST: &str = "example.invalid";

    /// Where the feed is read from — FR-102, in order, the first good answer winning.
    ///
    /// ⚠ **The whole of the network surface of this program is this array.** Ворота 3 of
    /// `tools\verify-perimeter.ps1` reads the strings of the Release binary and refuses any
    /// `https://` that is not one of these.
    pub const FEED_URLS: [&str; 1] = ["https://example.invalid/news.toml"];

    /// The author's channel — FR-103, and the «Открыть канал» buttons of three letters.
    pub const CHANNEL_URL: &str = "https://example.invalid/channel";

    /// The support page — FR-103, one link and no payment details of any kind in the program
    /// (вопрос 101 п. 6).
    pub const SUPPORT_URL: &str = "https://example.invalid/support";

    /// Whether this address is one of the placeholders — the one test every button that opens
    /// a link is behind.
    pub fn is_placeholder(url: &str) -> bool {
        url.contains(PLACEHOLDER_HOST)
    }

    /// Whether the program has a real feed address to read from at all.
    pub fn feed_is_configured() -> bool {
        FEED_URLS.iter().any(|url| !is_placeholder(url))
    }
}

// =========================================================================================
// 7. The quiet moment — FR-101
// =========================================================================================

/// How long the keyboard must have been still for the moment to count as a quiet one —
/// FR-101.
pub const QUIET_IDLE_SECONDS: u32 = 30;

/// Whether now is a quiet moment — FR-101: the system accepts notifications **and** nobody has
/// touched the keyboard or the mouse for [`QUIET_IDLE_SECONDS`].
///
/// The first half is `SHQueryUserNotificationState`, which is the documented way to ask
/// Windows whether it is showing a full-screen application, a presentation, a game, or has
/// been put into «do not disturb»: every one of those answers something other than
/// `QUNS_ACCEPTS_NOTIFICATIONS`, and every one of them is a moment this program keeps quiet
/// in. The second half is `GetLastInputInfo`, which answers when the session last saw input;
/// a person in the middle of typing is not interrupted by a letter about the author.
///
/// NFR-13: a refusal from either call answers **false** — not a quiet moment — because the
/// failure mode of this function is asymmetric. Guessing «quiet» opens a window over somebody's
/// game; guessing «busy» delays a letter by an hour.
///
/// # ⚠ Call this from a message handler, and only from one
///
/// «How long ago» needs two numbers in the same clock: the tick `GetLastInputInfo` answers
/// with, and the tick of now. The direct source of the second is `GetTickCount`, which lives in
/// the `Win32_System_SystemInformation` feature of the `windows` crate — and вопрос 101 п. 3
/// authorises two new features, both of them for the feed. `GetMessageTime` is in
/// `Win32_UI_WindowsAndMessaging`, which this program has carried since its first window, and
/// it answers with the tick of **the message this thread is currently handling** — the same
/// clock, to the millisecond, when the handler is the timer that just fired. Both callers of
/// this function are exactly that (the start-up check of NFR-08 and the hourly one of NFR-10),
/// which is what makes the substitution exact rather than approximate.
pub fn moment_is_quiet() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::Shell::{QUNS_ACCEPTS_NOTIFICATIONS, SHQueryUserNotificationState};
    use windows::Win32::UI::WindowsAndMessaging::GetMessageTime;

    // SAFETY: takes no arguments and writes no memory of ours; the crate turns the failure
    // into an error.
    let state = unsafe { SHQueryUserNotificationState() };

    match state {
        Ok(state) if state == QUNS_ACCEPTS_NOTIFICATIONS => {}
        Ok(_) => return false,
        Err(error) => {
            crate::app::report_non_critical("SHQueryUserNotificationState", &error);
            return false;
        }
    }

    let mut info = LASTINPUTINFO {
        cbSize: u32::try_from(size_of::<LASTINPUTINFO>()).unwrap_or(0),
        dwTime: 0,
    };

    // SAFETY: `info` is a live local of this frame whose `cbSize` describes it, which is the
    // only memory the call touches.
    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        crate::app::report_non_critical("GetLastInputInfo", &windows::core::Error::from_thread());
        return false;
    }

    // SAFETY: takes no arguments and touches no memory. The value is the tick of the message
    // being handled — see the note above for why that is the right «now» here.
    let now = unsafe { GetMessageTime() } as u32;

    // `wrapping_sub` and not a subtraction: both values are the same wrapping millisecond
    // counter, and the difference of two of them is right across the wrap that happens every
    // forty-nine days. A machine that has been up that long is exactly the machine this
    // program is meant to survive on (FR-80).
    now.wrapping_sub(info.dwTime) / 1000 >= QUIET_IDLE_SECONDS
}

// =========================================================================================
// 8. The state helpers `settings::Letters` publishes through this module
// =========================================================================================

/// The three maps of `[letters]` keyed the way TOML keys them — by string.
///
/// A news identifier is a number everywhere it is used and a key everywhere it is stored,
/// because a TOML table key is text. The two conversions live here so that no caller ever
/// spells the conversion itself, and so that a key that is not a number — a hand-edited file —
/// is ignored in exactly one place.
pub(crate) fn key_of(id: u64) -> String {
    id.to_string()
}

/// The identifier a stored key names, or `None` for a key that is not one.
pub(crate) fn id_of(key: &str) -> Option<u64> {
    key.parse().ok()
}

/// A map of news identifiers to whatever is remembered about them.
pub type NewsMap<T> = BTreeMap<String, T>;

// =========================================================================================
// 9. What goes into a letter window — FR-101, task Т-32-3
// =========================================================================================

/// Resource identifiers of the three templates, mirrored from `app.rc` by hand — the same rule
/// every identifier of `settings` is kept by, and for the same reason: a `.rc` file and a Rust
/// file share no header, and a mismatch shows up as a missing resource rather than the wrong
/// one.
pub const IDD_LETTER: u16 = 202;
/// The «Последние письма» window — FR-101.
pub const IDD_LETTERS_LIST: u16 = 203;
/// The «От автора» window — FR-103.
pub const IDD_AUTHOR: u16 = 204;

// Controls of `IDD_LETTER`, mirrored from `app.rc`.
const IDC_LETTER_ICON: i32 = 1200;
const IDC_LETTER_TITLE: i32 = 1201;
const IDC_LETTER_SUBTITLE: i32 = 1202;
const IDC_LETTER_LEAD: i32 = 1203;
const IDC_LETTER_PARA: i32 = 1204;
const IDC_LETTER_DEMO: i32 = 1205;
const IDC_LETTER_DEMO_CAP: i32 = 1206;
const IDC_LETTER_PANEL: i32 = 1207;
const IDC_LETTER_PANEL_TITLE: i32 = 1208;
const IDC_LETTER_PANEL_TEXT: i32 = 1209;
const IDC_LETTER_N1: i32 = 1210;
const IDC_LETTER_ROW_1: i32 = 1214;
const IDC_LETTER_PANEL_BTN_1: i32 = 1218;
const IDC_LETTER_PANEL_BTN_2: i32 = 1219;
const IDC_LETTER_LEFT: i32 = 1220;
const IDC_LETTER_RIGHT: i32 = 1221;
const IDC_LETTER_ACCENT: i32 = 1222;
const IDC_LETTER_FOOT: i32 = 1223;

// Controls of `IDD_LETTERS_LIST`, mirrored from `app.rc`. Four entries of five controls, in
// four contiguous runs, so that entry `i` is the first of its run plus `i`.
const IDC_LETTERS_PANEL: i32 = 1230;
const IDC_LETTERS_T1: i32 = 1231;
const IDC_LETTERS_D1: i32 = 1235;
const IDC_LETTERS_X1: i32 = 1239;
const IDC_LETTERS_B1: i32 = 1243;
const IDC_LETTERS_R1: i32 = 1247;
const IDC_LETTERS_CLOSE: i32 = 1251;
const IDC_LETTERS_FOOT: i32 = 1252;

// Controls of `IDD_AUTHOR`, mirrored from `app.rc`.
const IDC_AUTHOR_ICON: i32 = 1260;
const IDC_AUTHOR_NAME: i32 = 1261;
const IDC_AUTHOR_VERSION: i32 = 1262;
const IDC_AUTHOR_PANEL: i32 = 1263;
const IDC_AUTHOR_TEXT: i32 = 1264;
const IDC_AUTHOR_SUPPORT: i32 = 1265;
const IDC_AUTHOR_CHANNEL: i32 = 1266;
const IDC_NEWS_PANEL: i32 = 1267;
const IDC_NEWS_ABOUT: i32 = 1268;
const IDC_NEWS_STATE: i32 = 1269;
const IDC_NEWS_VERSION: i32 = 1270;
const IDC_NEWS_DOWNLOAD: i32 = 1271;
const IDC_NEWS_LETTERS: i32 = 1272;
const IDC_NEWS_FILE_ONLY: i32 = 1273;
const IDC_NEWS_SWITCH: i32 = 1274;
const IDC_NEWS_SWITCH_SUB: i32 = 1275;
const IDC_FEEDBACK_PANEL: i32 = 1276;
const IDC_FEEDBACK_TEXT: i32 = 1277;
const IDC_FEEDBACK_WRITE: i32 = 1278;
const IDC_AUTHOR_CLOSE: i32 = 1279;

/// The four row markers and the four row texts, paired — the panel of a letter holds up to
/// four rows and the identifiers of each run are contiguous (`app.rc`).
const LETTER_ROWS: [(i32, i32); 4] = [
    (IDC_LETTER_N1, IDC_LETTER_ROW_1),
    (IDC_LETTER_N1 + 1, IDC_LETTER_ROW_1 + 1),
    (IDC_LETTER_N1 + 2, IDC_LETTER_ROW_1 + 2),
    (IDC_LETTER_N1 + 3, IDC_LETTER_ROW_1 + 3),
];

/// Every owner-drawn **static** of the letter window, in no particular order — the gate of
/// `WM_DRAWITEM`, exactly as `settings::OWNER_DRAWN_ABOUT_LABELS` is for its window.
const LETTER_LABELS: [i32; 17] = [
    IDC_LETTER_TITLE,
    IDC_LETTER_SUBTITLE,
    IDC_LETTER_LEAD,
    IDC_LETTER_PARA,
    IDC_LETTER_DEMO,
    IDC_LETTER_DEMO_CAP,
    IDC_LETTER_PANEL_TITLE,
    IDC_LETTER_PANEL_TEXT,
    IDC_LETTER_N1,
    IDC_LETTER_N1 + 1,
    IDC_LETTER_N1 + 2,
    IDC_LETTER_N1 + 3,
    // ⚠ **All four rows, and the three after the first arrived only when the stand showed the
    // window**: with `IDC_LETTER_ROW_1` alone on this list the gate of `on_draw_item` refused
    // rows two, three and four, and «Привет» came up with its markers and no sentences beside
    // them. The instrument found it in the first screenshot it took (`scratchpad-Э32\снимки`).
    IDC_LETTER_ROW_1,
    IDC_LETTER_ROW_1 + 1,
    IDC_LETTER_ROW_1 + 2,
    IDC_LETTER_ROW_1 + 3,
    IDC_LETTER_FOOT,
];

/// Whether this label stands **on** a panel, and is therefore filled with the panel's own
/// colour — an owner-drawn static answers for the whole of its rectangle, so a
/// window-coloured fill on a block would cut a hole in it.
///
/// A function over ranges and not a list: the rows of a letter and the entries of the list
/// window are numbered in runs, and a list is a place where a run can be written short — which
/// is exactly the defect the first screenshot of «Привет» showed, in the twin of this rule.
///
/// The three windows are answered by one body because their identifiers are disjoint: every
/// control of every template has a number of its own, so «is this label on a block» is a
/// question about the number and not about the window.
///
/// ⚠ The demonstration of «Привет» is **not** on a panel — it is a field of its own on the
/// window's ground, and it fills its whole rectangle itself.
pub fn label_stands_on_a_panel(control: i32) -> bool {
    matches!(
        control,
        IDC_AUTHOR_TEXT
            | IDC_NEWS_ABOUT
            | IDC_NEWS_SWITCH_SUB
            | IDC_NEWS_STATE
            | IDC_NEWS_VERSION
            | IDC_NEWS_FILE_ONLY
            | IDC_FEEDBACK_TEXT
            | IDC_LETTER_PANEL_TITLE
            | IDC_LETTER_PANEL_TEXT
    ) || (IDC_LETTER_N1..IDC_LETTER_N1 + 4).contains(&control)
        || (IDC_LETTER_ROW_1..IDC_LETTER_ROW_1 + 4).contains(&control)
        || (IDC_LETTERS_T1..IDC_LETTERS_T1 + 4).contains(&control)
        || (IDC_LETTERS_D1..IDC_LETTERS_D1 + 4).contains(&control)
        || (IDC_LETTERS_X1..IDC_LETTERS_X1 + 4).contains(&control)
}

/// Every owner-drawn **button** of the letter window — the list `subclass_buttons` walks so
/// that the cursor gets its answer (task T-12-8), and the gate of the button branch of
/// `WM_DRAWITEM`.
const LETTER_BUTTONS: [i32; 5] = [
    IDC_LETTER_PANEL_BTN_1,
    IDC_LETTER_PANEL_BTN_2,
    IDC_LETTER_LEFT,
    IDC_LETTER_RIGHT,
    IDC_LETTER_ACCENT,
];

/// What a button of a letter does when it is pressed — FR-101 and FR-103.
///
/// A value and not a closure: the plan of a letter is a **pure** description of the window, and
/// a description that carried behaviour could not be compared in a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Close the window and do nothing else.
    Close,
    /// «Напомнить через неделю» — FR-101, allowed once.
    Snooze,
    /// «Прочитано» — the only thing that makes a news item read (FR-101).
    MarkRead,
    /// «Открыть настройки» — the window of FR-92, by the path the tray already uses.
    OpenSettings,
    /// «Открыть канал» — `links::CHANNEL_URL` through the shell.
    OpenChannel,
    /// «Открыть страницу поддержки» — `links::SUPPORT_URL`.
    OpenSupport,
    /// «Открыть страницу загрузки» — the link the `update` entry of the feed carries.
    OpenDownload,
    /// «Открыть ссылку» — the link a `news` entry carries.
    OpenLink,
    /// The «От автора» window — FR-103.
    OpenAuthor,
    /// The «Последние письма» window — FR-101.
    OpenLetters,
    /// The wizard of FR-104. Stage В; until then the button that would ask for it is absent.
    OpenWizard,
}

/// One button of a letter: what it says, what it does, and whether it can be pressed.
///
/// `enabled` is false for exactly one reason in stage А — the address it would open is still a
/// placeholder (полномочия П7 and П8). The button is **drawn and disabled** rather than hidden,
/// because that is what the user asked for: «кнопка пусть будет, вместо пути заглушка».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// The caption, out of the string table of the locale in force (FR-94).
    pub label: String,
    /// What pressing it does.
    pub action: Action,
    /// Whether it can be pressed at all.
    pub enabled: bool,
}

impl Button {
    /// A button that can be pressed.
    fn live(label: String, action: Action) -> Self {
        Self {
            label,
            action,
            enabled: true,
        }
    }

    /// A button that opens `url` — live if the address is real, drawn and disabled while it is
    /// a placeholder (полномочия П5, П7 и П8).
    fn link(label: String, action: Action, url: &str) -> Self {
        Self {
            label,
            action,
            enabled: !links::is_placeholder(url),
        }
    }
}

/// One row of a letter's panel: the marker in the narrow column and the sentence beside it.
///
/// The marker is «1», «2», «3» for a numbered list and «·» for a bulleted one — a template
/// literal in both cases, the same in every locale, exactly as the five numerals of the about
/// window are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The marker.
    pub marker: String,
    /// The sentence. May carry [`theme::KEY_PLACEHOLDER`], and then it is drawn as a chip row.
    pub text: String,
}

/// **What a letter looks like** — the pure description one window is filled from.
///
/// Every field is a slot of `IDD_LETTER`; an empty one is a slot this letter does not use, and
/// the layout hides it. This is the whole of «which letter is this»: the window procedure
/// itself knows about panels and buttons and nothing about «Привет» or «Спасибо».
///
/// Pure, and that is the point — [`plan_for`] reads the string tables and the state and answers
/// this, so every rule about what a letter says is checked in a test with no window at all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LetterPlan {
    /// The window's own caption.
    pub caption: String,
    /// The heading, in the large semibold face.
    pub title: String,
    /// The quiet line under the heading — a version, a date.
    pub subtitle: String,
    /// The paragraph in the text column beside the icon.
    pub lead: String,
    /// A paragraph the full width of the window, under the head block.
    pub para: String,
    /// Whether the demonstration of «Привет» is shown.
    pub demo: bool,
    /// The caption under the demonstration.
    pub demo_caption: String,
    /// The caption of the panel; empty means the letter has no panel.
    pub panel: String,
    /// A heading inside the panel — the title of a news item.
    pub panel_title: String,
    /// A paragraph inside the panel.
    pub panel_text: String,
    /// Up to four rows inside the panel.
    pub rows: Vec<Row>,
    /// Up to two buttons inside the panel.
    pub panel_buttons: Vec<Button>,
    /// The button at the bottom left.
    pub left: Option<Button>,
    /// The plain button at the bottom right.
    pub right: Option<Button>,
    /// The accented button at the bottom right — the one Enter presses.
    pub accent: Option<Button>,
    /// The quiet line under the buttons.
    pub foot: String,
}

/// Everything [`plan_for`] needs that is not in the string tables.
#[derive(Debug, Clone)]
pub struct PlanContext<'a> {
    /// This build's version, as `VERSIONINFO` spells it.
    pub version: String,
    /// The version the state remembers — empty on a machine raised from schema 5.
    pub previous_version: String,
    /// The hotkey as it acts, for the chip of «Привет».
    pub hotkey: String,
    /// Whether «Спасибо» may still offer its one snooze.
    pub snooze_offered: bool,
    /// The feed entry a letter out of the feed is about.
    pub item: Option<&'a FeedItem>,
}

/// What one letter says — **the whole of FR-101's text side**, and a pure function of the
/// string tables, the state and the feed.
pub fn plan_for(letter: Letter, context: &PlanContext<'_>) -> LetterPlan {
    use crate::settings::{
        IDS_CHANNEL_OPEN, IDS_CLOSE, IDS_HELLO_DEMO_CAP, IDS_HELLO_LEAD, IDS_HELLO_OK,
        IDS_HELLO_PANEL, IDS_HELLO_ROW_1, IDS_HELLO_ROW_2, IDS_HELLO_ROW_3, IDS_HELLO_SETTINGS,
        IDS_HELLO_TITLE, IDS_LETTER_CAPTION, IDS_NEWS_DOWNLOAD, IDS_NEWS_READ_BUTTON,
        IDS_SUPPORT_OPEN, IDS_SUPPORT_PANEL, IDS_SUPPORT_TEXT, IDS_THANKS_FOOT, IDS_THANKS_LEAD,
        IDS_THANKS_PARA, IDS_THANKS_SNOOZE, IDS_THANKS_TITLE, IDS_WHATSNEW_1, IDS_WHATSNEW_2,
        IDS_WHATSNEW_3, IDS_WHATSNEW_FROM, IDS_WHATSNEW_FULL, IDS_WHATSNEW_PANEL,
        IDS_WHATSNEW_TITLE, IDS_WHATSNEW_TODAY, format_text, text,
    };

    let caption = text(IDS_LETTER_CAPTION);

    match letter {
        Letter::Welcome => LetterPlan {
            caption,
            title: text(IDS_HELLO_TITLE),
            lead: text(IDS_HELLO_LEAD),
            demo: true,
            demo_caption: text(IDS_HELLO_DEMO_CAP),
            panel: text(IDS_HELLO_PANEL),
            rows: vec![
                Row {
                    marker: "1".to_owned(),
                    text: text(IDS_HELLO_ROW_1),
                },
                Row {
                    marker: "2".to_owned(),
                    text: text(IDS_HELLO_ROW_2),
                },
                Row {
                    marker: "3".to_owned(),
                    text: text(IDS_HELLO_ROW_3),
                },
            ],
            left: Some(Button::live(text(IDS_HELLO_SETTINGS), Action::OpenSettings)),
            accent: Some(Button::live(text(IDS_HELLO_OK), Action::Close)),
            ..LetterPlan::default()
        },

        Letter::Thanks => LetterPlan {
            caption,
            title: text(IDS_THANKS_TITLE),
            lead: text(IDS_THANKS_LEAD),
            para: text(IDS_THANKS_PARA),
            panel: text(IDS_SUPPORT_PANEL),
            panel_text: text(IDS_SUPPORT_TEXT),
            panel_buttons: vec![
                Button::link(
                    text(IDS_SUPPORT_OPEN),
                    Action::OpenSupport,
                    links::SUPPORT_URL,
                ),
                Button::link(
                    text(IDS_CHANNEL_OPEN),
                    Action::OpenChannel,
                    links::CHANNEL_URL,
                ),
            ],
            // FR-101: the snooze is offered once, and the second showing has no button where
            // it stood — not a disabled one. A disabled «Напомнить через неделю» would say the
            // program is refusing; an absent one says the letter is on its last showing, which
            // is what the line under it says too.
            left: context
                .snooze_offered
                .then(|| Button::live(text(IDS_THANKS_SNOOZE), Action::Snooze)),
            accent: Some(Button::live(text(IDS_CLOSE), Action::Close)),
            foot: text(IDS_THANKS_FOOT),
            ..LetterPlan::default()
        },

        Letter::WhatsNew => LetterPlan {
            caption,
            title: format_text(IDS_WHATSNEW_TITLE, &[&context.version]),
            // A machine raised from schema 5 has no previous version to name — the rung left
            // the field empty on purpose — so it is told only that it was updated today.
            subtitle: if context.previous_version.is_empty() {
                text(IDS_WHATSNEW_TODAY)
            } else {
                format_text(IDS_WHATSNEW_FROM, &[&context.previous_version])
            },
            panel: text(IDS_WHATSNEW_PANEL),
            rows: vec![
                Row {
                    marker: "·".to_owned(),
                    text: text(IDS_WHATSNEW_1),
                },
                Row {
                    marker: "·".to_owned(),
                    text: text(IDS_WHATSNEW_2),
                },
                Row {
                    marker: "·".to_owned(),
                    text: text(IDS_WHATSNEW_3),
                },
            ],
            para: text(IDS_WHATSNEW_FULL),
            left: Some(Button::link(
                text(IDS_CHANNEL_OPEN),
                Action::OpenChannel,
                links::CHANNEL_URL,
            )),
            accent: Some(Button::live(text(IDS_CLOSE), Action::Close)),
            ..LetterPlan::default()
        },

        // The two letters out of the feed — стадия Б fills them from `context.item`. Until
        // there is a feed there is nothing to say, and a letter with no entry behind it is a
        // window that must not open: the plan comes back empty and the caller refuses.
        Letter::Update | Letter::News(_) => {
            let Some(item) = context.item else {
                return LetterPlan::default();
            };

            let is_update = matches!(letter, Letter::Update);

            LetterPlan {
                caption,
                title: item.title.clone(),
                panel_text: item.text.clone(),
                panel: if is_update {
                    text(IDS_WHATSNEW_PANEL)
                } else {
                    String::new()
                },
                panel_buttons: if item.link.is_empty() {
                    Vec::new()
                } else {
                    vec![Button::link(
                        text(IDS_NEWS_DOWNLOAD),
                        if is_update {
                            Action::OpenDownload
                        } else {
                            Action::OpenLink
                        },
                        &item.link,
                    )]
                },
                left: Some(Button::live(text(IDS_CLOSE), Action::Close)),
                accent: (!is_update)
                    .then(|| Button::live(text(IDS_NEWS_READ_BUTTON), Action::MarkRead)),
                ..LetterPlan::default()
            }
        }
    }
}

// =========================================================================================
// 10. The demonstration of «Привет» — FR-101, and it is a pure function of one number
// =========================================================================================

/// How long one frame of the demonstration lasts, in milliseconds.
const DEMO_TICK_MS: u32 = 130;

/// How many frames one turn of the demonstration takes.
pub const DEMO_FRAMES: u32 = 48;

/// The word typed in the wrong layout, and the word it becomes. Latin and Cyrillic literals
/// rather than strings of the tables: they are **the example itself** — the very keys a person
/// presses — and translating them would destroy what the picture shows.
const DEMO_TYPED: &str = "ghbdtn";
/// What [`DEMO_TYPED`] becomes.
const DEMO_FIXED: &str = "привет";

/// One frame of the demonstration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemoFrame {
    /// The text on the little screen at this moment.
    pub text: &'static str,
    /// How many characters of it are typed so far.
    pub shown: usize,
    /// Whether the key chip is lit at this moment.
    pub pressed: bool,
    /// Whether the layout indicator says the target layout.
    pub converted: bool,
}

/// What the demonstration shows at frame `tick` — **pure**, so the whole animation is a table a
/// test can walk without a window, a timer or a clock.
///
/// One turn: the word is typed letter by letter, the key flashes, the word is fixed and the
/// indicator flips, the key flashes again, the word comes back, and it starts over. That is
/// FR-01 to FR-06 in eight seconds and no words at all.
pub fn demo_frame(tick: u32) -> DemoFrame {
    match tick % DEMO_FRAMES {
        // Typing, one character a frame.
        frame @ 0..=5 => DemoFrame {
            text: DEMO_TYPED,
            shown: frame as usize + 1,
            pressed: false,
            converted: false,
        },
        // The typed word, waiting.
        6..=13 => DemoFrame {
            text: DEMO_TYPED,
            shown: DEMO_TYPED.chars().count(),
            pressed: false,
            converted: false,
        },
        // The key goes down.
        14..=15 => DemoFrame {
            text: DEMO_TYPED,
            shown: DEMO_TYPED.chars().count(),
            pressed: true,
            converted: false,
        },
        // The word is fixed and the layout has switched.
        16..=29 => DemoFrame {
            text: DEMO_FIXED,
            shown: DEMO_FIXED.chars().count(),
            pressed: false,
            converted: true,
        },
        // The key goes down again.
        30..=31 => DemoFrame {
            text: DEMO_FIXED,
            shown: DEMO_FIXED.chars().count(),
            pressed: true,
            converted: true,
        },
        // And the word is back, letter for letter — FR-05.
        _ => DemoFrame {
            text: DEMO_TYPED,
            shown: DEMO_TYPED.chars().count(),
            pressed: false,
            converted: false,
        },
    }
}

/// The two words of the layout indicator. Not translated, for the reason [`DEMO_TYPED`] is not:
/// they are the names of the layouts in the picture, and a layout is named in its own language.
const DEMO_BADGE: [&str; 2] = ["EN", "RU"];

// =========================================================================================
// 11. The window itself — FR-101 and FR-103, task Т-32-3
// =========================================================================================

use std::cell::RefCell;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, FillRect, GetDC, HDC, HFONT, ReleaseDC, SelectObject,
};
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED, ODT_BUTTON, ODT_STATIC,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    DM_SETDEFID, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDCANCEL,
    KillTimer, MSG, MapDialogRect, PostMessageW, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
    SWP_NOACTIVATE, SWP_NOZORDER, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_INITDIALOG, WM_NCDESTROY,
    WM_TIMER,
};
use windows::core::{Error as WinError, PCWSTR, w};

use crate::settings::{self, Language};
use crate::theme::{self, StaticColorRole, ThemeSetting};

/// The identifier of the timer that drives the demonstration of «Привет».
///
/// NFR-10: it is set on `WM_INITDIALOG` of a letter that has a demonstration and killed on
/// `WM_DESTROY`, so **nothing ticks while no such window is open** — which is the whole of the
/// requirement's «в покое ничего не делает».
const DEMO_TIMER: usize = 1;

/// Which of the three windows this is. One of each at a time; asking for one that is already up
/// raises it instead of making a second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A letter — the window of `IDD_LETTER`, whichever of the five it is showing.
    Letter,
    /// «Последние письма» — `IDD_LETTERS_LIST`.
    List,
    /// «От автора» — `IDD_AUTHOR`.
    Author,
}

/// The six faces one of these windows draws in.
///
/// The **derivations** are shared with the about window — `settings::caption_logfont`,
/// `about_name_logfont`, `about_body_logfont`, `about_number_logfont`, `about_chip_logfont`,
/// `theme::smoothed_logfont` — so a letter and «О программе» cannot come out set differently.
/// What is not shared is the ownership: these handles belong to **this** window and are freed
/// when it is destroyed.
struct Faces {
    text: HFONT,
    caption: HFONT,
    name: HFONT,
    body: HFONT,
    number: HFONT,
    chip: HFONT,
    body_height: i32,
    chip_height: i32,
}

impl Faces {
    /// All six out of the font the dialog manager gave the window, every handle examined.
    ///
    /// `None` on any refusal, and a half-built set is unwound rather than leaked — the
    /// discipline `settings::DialogFonts::new` keeps, for the same reason: a constructor that
    /// answers `None` leaves no value for `Drop` to run on. The window then draws in the
    /// manager's own face (NFR-13).
    fn new(hwnd: HWND, control: i32) -> Option<Self> {
        let base = settings::dialog_logfont(hwnd, control)?;
        let body = settings::about_body_logfont(base);
        let emphasis = settings::resolve_emphasis(hwnd, base);
        let chip = settings::about_chip_logfont(base, emphasis);

        let wanted = [
            theme::smoothed_logfont(base),
            settings::caption_logfont(base),
            settings::about_name_logfont(base, emphasis),
            body,
            settings::about_number_logfont(base, emphasis),
            chip,
        ];

        let mut made: Vec<HFONT> = Vec::with_capacity(wanted.len());

        for logical in wanted {
            let Some(face) = theme::create_font(logical) else {
                for face in made {
                    // SAFETY: each was created by this loop, handed to nobody, and is freed
                    // exactly once here.
                    let _ = unsafe { DeleteObject(face.into()) };
                }

                return None;
            };

            made.push(face);
        }

        Some(Self {
            text: made[0],
            caption: made[1],
            name: made[2],
            body: made[3],
            number: made[4],
            chip: made[5],
            body_height: body.lfHeight,
            chip_height: chip.lfHeight,
        })
    }

    /// The step from one wrapped line of the body to the next, in pixels of this window.
    fn body_pitch(&self) -> i32 {
        settings::about_body_line_pitch(self.body_height)
    }
}

impl Drop for Faces {
    fn drop(&mut self) {
        for face in [
            self.text,
            self.caption,
            self.name,
            self.body,
            self.number,
            self.chip,
        ] {
            // SAFETY: each handle was made by `Faces::new` and is owned by this value; nothing
            // else frees them, and a font still selected into a DC is not: every drawing puts
            // the previous face back before it returns.
            let _ = unsafe { DeleteObject(face.into()) };
        }
    }
}

/// Everything one of these windows knows about itself.
///
/// Lives in a `Box` the window owns: unlike the two modal windows of `settings`, a modeless one
/// outlives the call that made it, so a state on the caller's frame would be gone before the
/// first `WM_PAINT`. The pointer goes into `GWLP_USERDATA` and the box is freed on
/// `WM_NCDESTROY` — the last message a window ever gets.
struct WindowState {
    kind: Kind,
    /// Which letter this is, for a window of [`Kind::Letter`].
    letter: Option<Letter>,
    /// The identifier of the feed entry a «Новость» is about — what «Прочитано» marks.
    news: Option<u64>,
    /// The link the entry carried, for the two buttons that open one.
    link: String,
    setting: ThemeSetting,
    palette: &'static theme::Palette,
    brushes: Option<theme::Brushes>,
    fonts: Option<Faces>,
    /// What this window says — the pure description it is filled and laid out from.
    plan: LetterPlan,
    /// What «От автора» says. Empty for the other two kinds.
    author: AuthorView,
    /// What «Последние письма» shows. Empty for the other two kinds, and empty for the whole of
    /// stage А — the feed arrives in stage Б.
    entries: Vec<Entry>,
    /// The hotkey as it acts, for the chips of «Привет».
    hotkey: String,
    icons: Option<settings::CaptionIcons>,
    logo: Option<settings::AboutLogo>,
    /// The frame of the demonstration, counted by the timer.
    tick: u32,
    /// The locale the window was built in — what `language_switch` is asked about.
    language: Language,
    /// The window that owns this one — this program's hidden UI window, the same owner the two
    /// windows of `settings` take. Kept because the actions of a letter open other windows.
    owner: HWND,
}

impl WindowState {
    /// Which control of this window is the accented one — the button Enter presses and the one
    /// drawn in the accent colour.
    ///
    /// One place, because two ask: the drawing, which colours it, and `WM_INITDIALOG`, which
    /// tells the dialog manager about it with `DM_SETDEFID`.
    fn accent_control(&self) -> i32 {
        match self.kind {
            Kind::Letter => IDC_LETTER_ACCENT,
            Kind::List => IDC_LETTERS_CLOSE,
            Kind::Author => IDC_AUTHOR_CLOSE,
        }
    }

    /// The `SS_ICON` static this window's logo goes into.
    fn icon_control(&self) -> i32 {
        match self.kind {
            Kind::Author => IDC_AUTHOR_ICON,
            _ => IDC_LETTER_ICON,
        }
    }

    /// What pressing this control does, out of the plan — `None` for a control that is not a
    /// button of this window.
    fn action_of(&self, control: i32) -> Option<Action> {
        let plan = &self.plan;

        // The «От автора» window's buttons are fixed — its shape does not change with a
        // letter — so they answer by identifier and not out of a plan.
        if self.kind == Kind::Author {
            return match control {
                IDC_AUTHOR_SUPPORT => Some(Action::OpenSupport),
                IDC_AUTHOR_CHANNEL => Some(Action::OpenChannel),
                IDC_NEWS_DOWNLOAD => Some(Action::OpenDownload),
                IDC_NEWS_LETTERS => Some(Action::OpenLetters),
                IDC_FEEDBACK_WRITE => Some(Action::OpenWizard),
                IDC_AUTHOR_CLOSE => Some(Action::Close),
                _ => None,
            };
        }

        if self.kind == Kind::List {
            return (control == IDC_LETTERS_CLOSE).then_some(Action::Close);
        }

        let button = match control {
            IDC_LETTER_LEFT => plan.left.as_ref(),
            IDC_LETTER_RIGHT => plan.right.as_ref(),
            IDC_LETTER_ACCENT => plan.accent.as_ref(),
            IDC_LETTER_PANEL_BTN_1 => plan.panel_buttons.first(),
            IDC_LETTER_PANEL_BTN_2 => plan.panel_buttons.get(1),
            _ => None,
        }?;

        button.enabled.then_some(button.action)
    }
}

thread_local! {
    /// The windows of this module that are open **on this thread**, in the order they were
    /// opened.
    ///
    /// Thread-local and not a global: every one of these windows is created on the UI thread
    /// and an `HWND` of another thread has no business here. The message pump asks this list
    /// what to hand `IsDialogMessageW`, the tray asks it whether a window is already up, and
    /// FR-94 asks it which windows to relabel when the interface language changes.
    static OPEN: RefCell<Vec<(Kind, HWND)>> = const { RefCell::new(Vec::new()) };
}

/// Whether a window of this kind is open on this thread.
pub fn is_open(kind: Kind) -> bool {
    OPEN.with_borrow(|open| open.iter().any(|(other, _)| *other == kind))
}

/// The handle of the open window of this kind, if there is one.
fn window_of(kind: Kind) -> Option<HWND> {
    OPEN.with_borrow(|open| {
        open.iter()
            .find(|(other, _)| *other == kind)
            .map(|(_, hwnd)| *hwnd)
    })
}

/// **The message filter of the modeless windows** — the half of them that lives in the message
/// pump of the UI thread.
///
/// A modal dialog runs its own loop and gets `IsDialogMessageW` for free; a modeless one does
/// not, and without this the Tab key would not move between its buttons, Enter would not press
/// the default one and Esc would not close it. Answers `true` when the message was a dialog's
/// and has been dealt with — the pump must then not dispatch it.
///
/// Costs one comparison per message on a thread with no letters open, which is the ordinary
/// state of this program.
///
/// # Safety
///
/// `message` is the caller's live `MSG`, filled by `GetMessageW`.
pub unsafe fn filter_message(message: &MSG) -> bool {
    let open: Vec<HWND> = OPEN.with_borrow(|open| open.iter().map(|(_, hwnd)| *hwnd).collect());

    open.into_iter().any(|hwnd| {
        // SAFETY: `hwnd` is a window this module made and has not yet seen destroyed — the
        // record is removed on `WM_NCDESTROY` — and `message` is the caller's live message.
        unsafe { windows::Win32::UI::WindowsAndMessaging::IsDialogMessageW(hwnd, message) }
            .as_bool()
    })
}

// -----------------------------------------------------------------------------------------
// The layout — measured, not written down
// -----------------------------------------------------------------------------------------

/// The two base units of a dialog and the DPI of its window: everything the layout counts in.
///
/// A dialog unit is a share of the font the template names, so the only honest way to get one
/// is to ask the window — `MapDialogRect` over a rectangle of four by eight units answers the
/// pair exactly. Spelling the numbers as constants would tie the layout to one font and one
/// DPI, which is what a program with fourteen locales and four icon sizes must not do.
#[derive(Debug, Clone, Copy)]
struct Metrics {
    base_x: i32,
    base_y: i32,
}

impl Metrics {
    /// The base units of this window.
    fn of(hwnd: HWND) -> Self {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 4,
            bottom: 8,
        };

        // SAFETY: `hwnd` is the live dialog and `rect` is a live local the call fills.
        if unsafe { MapDialogRect(hwnd, &mut rect) }.is_err() {
            // NFR-13: the units of a 9 pt Segoe UI dialog at 96 DPI. A window laid out with
            // them is a window laid out for the wrong DPI, which is a blemish; a window not
            // laid out at all is a blank rectangle.
            return Self {
                base_x: 7,
                base_y: 15,
            };
        }

        Self {
            base_x: rect.right.max(1),
            base_y: rect.bottom.max(1),
        }
    }

    /// Horizontal dialog units in pixels of this window.
    fn x(self, dlu: i32) -> i32 {
        dlu * self.base_x / 4
    }

    /// Vertical dialog units in pixels of this window.
    fn y(self, dlu: i32) -> i32 {
        dlu * self.base_y / 8
    }
}

/// The air and the columns of these windows, in dialog units — the mock-up's paddings, and the
/// about window's where the two agree.
mod air {
    /// The margin at the left and the right of the window.
    pub(super) const PAD: i32 = 12;
    /// The top of the head block.
    pub(super) const TOP: i32 = 12;
    /// Where the text column beside the icon starts.
    pub(super) const TEXT_X: i32 = 42;
    /// The air between two things that belong together.
    pub(super) const TIGHT: i32 = 3;
    /// The air between two blocks.
    pub(super) const GAP: i32 = 8;
    /// The inset of everything that stands on a panel.
    pub(super) const PANEL_PAD: i32 = 7;
    /// How far below the top of a panel its contents start — under the caption.
    pub(super) const PANEL_HEAD: i32 = 20;
    /// The height of a push button.
    pub(super) const BUTTON: i32 = 14;
    /// The air inside a button, at each end of its caption.
    pub(super) const BUTTON_PAD: i32 = 10;
    /// The narrowest a button may be.
    pub(super) const BUTTON_MIN: i32 = 40;
    /// The air between two buttons of a row.
    pub(super) const BUTTON_GAP: i32 = 6;
    /// The width of the marker column of a panel row.
    pub(super) const MARKER: i32 = 7;
    /// Where the text of a panel row starts, from the panel's own inset.
    pub(super) const ROW_TEXT: i32 = 11;
    /// The height of the demonstration of «Привет».
    pub(super) const DEMO: i32 = 34;
}

/// Measures the height one caption needs at one width, in the face it will be drawn in.
///
/// The face is selected into the DC first and put back afterwards, because
/// [`theme::measure_label`] measures **whatever the DC holds** — that is what makes its answer
/// the painter's answer and not a second opinion.
///
/// # Safety
///
/// `dc` is a live DC of this window; `face` is a live font this window owns.
unsafe fn measure(dc: HDC, face: HFONT, width: i32, caption: &str, pitch: Option<i32>) -> i32 {
    if caption.is_empty() {
        return 0;
    }

    let mut wide: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: `dc` is the caller's live DC and `face` a live font; the previous face is put
    // back below.
    let previous = unsafe { SelectObject(dc, face.into()) };
    // SAFETY: the face is selected into `dc`, which is the contract of `measure_label`.
    let height = unsafe { theme::measure_label(dc, width, &mut wide, pitch) };
    // SAFETY: puts back exactly the handle the call above answered with.
    let _ = unsafe { SelectObject(dc, previous) };

    height
}

/// How tall a **row that may carry a key chip** has to be — [`measure`] with the width narrowed
/// by what the chip adds.
///
/// ⚠ **A row with a chip in it is wider than its words, and measuring it without the chip is a
/// measurement of a different row.** The German «Привет» came up one line short because of
/// exactly this: the last two words of its caption were drawn past the bottom of the label
/// (`scratchpad-Э32\снимки\de-dark-hello.png`, before this function existed). The figure round
/// the key name takes `chip_box().width` where the words take the width of the name itself, so
/// the honest measurement is of the same sentence in a **narrower** column — the shape the
/// fitting stand of Э30 already found for the help rows of «О программе».
///
/// A caption with no placeholder, or a window with no key name, is measured exactly as
/// [`measure`] measures it: the overhang is zero and the call is the same call.
///
/// # Safety
///
/// As [`measure`], with `chip` a live font of this window as well.
unsafe fn measure_row(
    dc: HDC,
    face: HFONT,
    chip: Chip<'_>,
    width: i32,
    caption: &str,
    pitch: Option<i32>,
) -> i32 {
    // SAFETY: see the contract.
    let overhang = unsafe { chip_overhang(dc, face, chip, caption) };

    // SAFETY: as above.
    unsafe { measure(dc, face, width - overhang, caption, pitch) }
}

/// The chip of a row: the face its key name is set in, that face's `lfHeight`, and the name.
///
/// A record rather than three more parameters — the shape `theme::ChipRowStyle` already has,
/// and for the same reason: this project's `clippy` denies a seventh argument.
#[derive(Clone, Copy)]
struct Chip<'a> {
    face: HFONT,
    height: i32,
    key: &'a str,
}

/// How much wider than its own words a chip row is — the figure minus the placeholder it
/// replaces. Zero when there is no chip in this caption.
///
/// # Safety
///
/// As [`measure`].
unsafe fn chip_overhang(dc: HDC, face: HFONT, chip: Chip<'_>, caption: &str) -> i32 {
    if chip.key.is_empty() || !caption.contains(theme::KEY_PLACEHOLDER) {
        return 0;
    }

    // SAFETY: see the contract — the two faces are live fonts of this window.
    let key_width = unsafe { text_width(dc, chip.face, chip.key) };
    // SAFETY: as above.
    let placeholder = unsafe { text_width(dc, face, theme::KEY_PLACEHOLDER) };

    let mut metrics = windows::Win32::Graphics::Gdi::TEXTMETRICW::default();

    // SAFETY: the chip face is selected into the DC for the measurement and put back.
    let previous = unsafe { SelectObject(dc, chip.face.into()) };
    // SAFETY: `metrics` is a live local the call fills.
    let _ = unsafe { windows::Win32::Graphics::Gdi::GetTextMetricsW(dc, &mut metrics) };
    // SAFETY: puts back exactly the handle the call above answered with.
    let _ = unsafe { SelectObject(dc, previous) };

    let box_of_chip = theme::chip_box(
        key_width,
        chip.height,
        metrics.tmHeight,
        theme::scaled(theme::BORDER_THICKNESS, theme::dc_dpi(dc)),
    );

    (box_of_chip.width - placeholder).max(0)
}

/// The width of one run of text in one face, on one line.
///
/// # Safety
///
/// As [`measure`].
unsafe fn text_width(dc: HDC, face: HFONT, text: &str) -> i32 {
    let mut wide: Vec<u16> = text.encode_utf16().collect();

    if wide.is_empty() {
        return 0;
    }

    // SAFETY: see the contract.
    let previous = unsafe { SelectObject(dc, face.into()) };

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    // SAFETY: `DT_CALCRECT | DT_SINGLELINE` measures and paints nothing.
    let _ = unsafe {
        windows::Win32::Graphics::Gdi::DrawTextW(
            dc,
            &mut wide,
            &mut rect,
            windows::Win32::Graphics::Gdi::DT_CALCRECT
                | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
        )
    };

    // SAFETY: puts back exactly the handle the call above answered with.
    let _ = unsafe { SelectObject(dc, previous) };

    rect.right - rect.left
}

/// How wide a button has to be to hold its caption — the caption plus the air of the mock-up,
/// and never narrower than [`air::BUTTON_MIN`].
///
/// This is what makes the row of buttons fit in **fourteen** locales without a table of widths:
/// German writes «Unterstützungsseite öffnen» where Russian writes «Открыть страницу
/// поддержки», and the button is as wide as the words it holds.
///
/// # Safety
///
/// As [`measure`].
unsafe fn button_width(dc: HDC, face: HFONT, metrics: Metrics, caption: &str) -> i32 {
    let mut wide: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: see the contract.
    let previous = unsafe { SelectObject(dc, face.into()) };

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    // SAFETY: `DT_CALCRECT | DT_SINGLELINE` measures and paints nothing; the buffer and the
    // rectangle are live locals of this frame.
    let _ = unsafe {
        windows::Win32::Graphics::Gdi::DrawTextW(
            dc,
            &mut wide,
            &mut rect,
            windows::Win32::Graphics::Gdi::DT_CALCRECT
                | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
        )
    };

    // SAFETY: puts back exactly the handle the call above answered with.
    let _ = unsafe { SelectObject(dc, previous) };

    (rect.right - rect.left + metrics.x(air::BUTTON_PAD) * 2).max(metrics.x(air::BUTTON_MIN))
}

/// Puts one child where the layout decided, and shows it.
fn place(hwnd: HWND, control: i32, x: i32, y: i32, width: i32, height: i32) {
    let Ok(child) =
        (unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) })
    else {
        return;
    };

    // SAFETY: `child` is the live control just answered for this dialog; the five numbers are
    // plain values and no pointer is passed.
    let _ = unsafe {
        SetWindowPos(
            child,
            None,
            x,
            y,
            width.max(0),
            height.max(0),
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };

    // ⚠ **A panel is `NOT WS_VISIBLE` in the template and must stay that way.** It carries the
    // caption and the rectangle, and the window's own `WM_ERASEBKGND` draws the block from
    // them; shown, it would be an owner-drawn *button* standing over its own caption — which
    // is exactly what the stand photographed before this line named all five panels instead of
    // one (`scratchpad-Э32\снимки`, the first «От автора»).
    if !PANELS.contains(&control) {
        // SAFETY: `child` is the live control; the call takes no pointer.
        let _ = unsafe { ShowWindow(child, SW_SHOWNOACTIVATE) };
    }
}

/// Every panel of every one of the three windows — the controls that carry a caption and a
/// rectangle and are never shown.
const PANELS: [i32; 5] = [
    IDC_LETTER_PANEL,
    IDC_LETTERS_PANEL,
    IDC_AUTHOR_PANEL,
    IDC_NEWS_PANEL,
    IDC_FEEDBACK_PANEL,
];

/// Hides one child — a slot this letter does not use.
fn hide(hwnd: HWND, control: i32) {
    let Ok(child) =
        (unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) })
    else {
        return;
    };

    // SAFETY: `child` is the live control; the call takes no pointer.
    let _ = unsafe { ShowWindow(child, SW_HIDE) };
}

/// Enables or disables one child — the disabled state of a button whose address is still a
/// placeholder (полномочия П7 и П8).
fn enable(hwnd: HWND, control: i32, enabled: bool) {
    let Ok(child) =
        (unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) })
    else {
        return;
    };

    // SAFETY: `child` is the live control; the call takes no pointer. The previous state is
    // deliberately dropped — this is a statement of what the state must be, not a toggle.
    let _ = unsafe { EnableWindow(child, enabled) };
}

/// Lays a row of buttons against the bottom of the window: the left one at the left margin, the
/// accented one at the right margin, the plain one beside it.
///
/// ⚠ **«Left» and «right» are the window's own**, and in a mirrored window the window manager
/// makes them the other way round for us: a child's coordinates in a `WS_EX_LAYOUTRTL` parent
/// are counted from the **right** edge. That is the whole of what ИТОГ-Э30 §9.2 asks of a new
/// window — be born mirrored and then lay out as usual.
///
/// # Safety
///
/// As [`measure`].
unsafe fn place_button_row(
    hwnd: HWND,
    dc: HDC,
    face: HFONT,
    metrics: Metrics,
    width: i32,
    y: i32,
    row: [(i32, Option<&Button>); 3],
) {
    let pad = metrics.x(air::PAD);
    let height = metrics.y(air::BUTTON);
    let mut right = width - pad;

    // The two right-hand buttons, from the right edge inwards: the accent first, then the
    // plain one beside it.
    for (control, button) in row.iter().skip(1) {
        let Some(button) = button else {
            hide(hwnd, *control);
            continue;
        };

        // SAFETY: see the contract.
        let wanted = unsafe { button_width(dc, face, metrics, &button.label) };

        place(hwnd, *control, right - wanted, y, wanted, height);
        enable(hwnd, *control, button.enabled);

        right -= wanted + metrics.x(air::BUTTON_GAP);
    }

    let (control, button) = row[0];

    match button {
        Some(button) => {
            // SAFETY: see the contract.
            let wanted = unsafe { button_width(dc, face, metrics, &button.label) };

            place(hwnd, control, pad, y, wanted.min(right - pad), height);
            enable(hwnd, control, button.enabled);
        }
        None => hide(hwnd, control),
    }
}

/// **Lays out one letter** — the whole of «where everything goes», measured on this window with
/// this window's faces at this window's DPI.
///
/// Top to bottom for the head, the paragraph and the demonstration; bottom to top for the line
/// under the buttons and the buttons themselves; and the panel takes everything left between
/// the two, which is the mock-up's «панель тянется на свободную высоту» exactly.
///
/// Every slot the plan leaves empty is hidden, and a hidden owner-drawn static paints nothing —
/// which is what keeps the rule of `app.rc` true here: **no two of these rectangles ever
/// overlap**, because the ones in use are handed disjoint bands and the rest are not on the
/// screen at all.
///
/// # Safety
///
/// `hwnd` is the live letter window and `state` is its own state, borrowed for this call only.
unsafe fn layout_letter(hwnd: HWND, state: &WindowState) {
    let Some(faces) = state.fonts.as_ref() else {
        return;
    };

    let mut client = RECT::default();

    // SAFETY: `hwnd` is the live window and `client` a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return;
    }

    // SAFETY: `hwnd` is the live window; the DC is released on every path below.
    let dc = unsafe { GetDC(Some(hwnd)) };

    if dc.is_invalid() {
        return;
    }

    let metrics = Metrics::of(hwnd);
    let plan = &state.plan;
    let pad = metrics.x(air::PAD);
    let full_width = client.right - pad * 2;
    let text_x = metrics.x(air::TEXT_X);
    let text_width = client.right - text_x - pad;
    let pitch = Some(faces.body_pitch());
    let tight = metrics.y(air::TIGHT);
    let gap = metrics.y(air::GAP);

    let mut top = metrics.y(air::TOP);

    // --- the head: the icon, the heading, the quiet line, the paragraph beside them --------
    for (control, caption, face, width, own_pitch) in [
        (
            IDC_LETTER_TITLE,
            &plan.title,
            faces.name,
            text_width,
            None::<i32>,
        ),
        (
            IDC_LETTER_SUBTITLE,
            &plan.subtitle,
            faces.text,
            text_width,
            None,
        ),
        (IDC_LETTER_LEAD, &plan.lead, faces.body, text_width, pitch),
    ] {
        if caption.is_empty() {
            hide(hwnd, control);
            continue;
        }

        // SAFETY: `dc` is this window's DC and the face is one it owns.
        let height = unsafe { measure(dc, face, width, caption, own_pitch) };

        place(hwnd, control, text_x, top, width, height);
        top += height + tight;
    }

    // The head block never ends above the icon: the icon is 22 units tall from 13, and a
    // one-line heading with no paragraph would otherwise let the panel run up beside it.
    top = top.max(metrics.y(13 + 22) + gap);

    // --- the paragraph the full width of the window ---------------------------------------
    if plan.para.is_empty() {
        hide(hwnd, IDC_LETTER_PARA);
    } else {
        // SAFETY: as above.
        let height = unsafe { measure(dc, faces.body, full_width, &plan.para, pitch) };

        place(hwnd, IDC_LETTER_PARA, pad, top, full_width, height);
        top += height + gap;
    }

    // --- the demonstration of «Привет» ------------------------------------------------------
    if plan.demo {
        let height = metrics.y(air::DEMO);

        place(hwnd, IDC_LETTER_DEMO, pad, top, full_width, height);
        top += height + tight;

        if plan.demo_caption.is_empty() {
            hide(hwnd, IDC_LETTER_DEMO_CAP);
        } else {
            // SAFETY: as above.
            // The caption of the demonstration names the key, so it is measured as a chip row.
            let height = unsafe {
                measure_row(
                    dc,
                    faces.body,
                    Chip {
                        face: faces.chip,
                        height: faces.chip_height,
                        key: &state.hotkey,
                    },
                    full_width,
                    &plan.demo_caption,
                    pitch,
                )
            };

            place(hwnd, IDC_LETTER_DEMO_CAP, pad, top, full_width, height);
            top += height;
        }

        top += gap;
    } else {
        hide(hwnd, IDC_LETTER_DEMO);
        hide(hwnd, IDC_LETTER_DEMO_CAP);
    }

    // --- from the bottom: the quiet line, then the row of buttons --------------------------
    let mut bottom = client.bottom - metrics.y(air::PAD);

    if plan.foot.is_empty() {
        hide(hwnd, IDC_LETTER_FOOT);
    } else {
        // SAFETY: as above.
        let height = unsafe { measure(dc, faces.text, full_width, &plan.foot, None) };

        bottom -= height;
        place(hwnd, IDC_LETTER_FOOT, pad, bottom, full_width, height);
        bottom -= tight;
    }

    if plan.left.is_some() || plan.right.is_some() || plan.accent.is_some() {
        bottom -= metrics.y(air::BUTTON);

        // SAFETY: as above.
        unsafe {
            place_button_row(
                hwnd,
                dc,
                faces.text,
                metrics,
                client.right,
                bottom,
                [
                    (IDC_LETTER_LEFT, plan.left.as_ref()),
                    (IDC_LETTER_ACCENT, plan.accent.as_ref()),
                    (IDC_LETTER_RIGHT, plan.right.as_ref()),
                ],
            );
        }

        bottom -= gap;
    } else {
        for control in [IDC_LETTER_LEFT, IDC_LETTER_RIGHT, IDC_LETTER_ACCENT] {
            hide(hwnd, control);
        }
    }

    // --- and the panel takes what is left --------------------------------------------------
    // SAFETY: `dc` is this window's DC; the borrow of the state ends with this call.
    unsafe { layout_panel(hwnd, dc, state, metrics, pad, full_width, top, bottom) };

    // SAFETY: releases exactly the DC taken at the top of this function, once.
    unsafe { ReleaseDC(Some(hwnd), dc) };
}

/// Lays out the panel of a letter and everything standing on it — the second half of
/// [`layout_letter`], split out because a panel is a window of its own inside the window.
///
/// # Safety
///
/// As [`layout_letter`].
#[allow(clippy::too_many_arguments)]
unsafe fn layout_panel(
    hwnd: HWND,
    dc: HDC,
    state: &WindowState,
    metrics: Metrics,
    pad: i32,
    full_width: i32,
    top: i32,
    bottom: i32,
) {
    let Some(faces) = state.fonts.as_ref() else {
        return;
    };

    let plan = &state.plan;

    let wanted = !plan.panel.is_empty()
        || !plan.panel_title.is_empty()
        || !plan.panel_text.is_empty()
        || !plan.rows.is_empty()
        || !plan.panel_buttons.is_empty();

    if !wanted || bottom <= top {
        for control in [
            IDC_LETTER_PANEL,
            IDC_LETTER_PANEL_TITLE,
            IDC_LETTER_PANEL_TEXT,
            IDC_LETTER_PANEL_BTN_1,
            IDC_LETTER_PANEL_BTN_2,
        ] {
            hide(hwnd, control);
        }

        for (marker, row) in LETTER_ROWS {
            hide(hwnd, marker);
            hide(hwnd, row);
        }

        return;
    }

    place(hwnd, IDC_LETTER_PANEL, pad, top, full_width, bottom - top);

    let inset = metrics.x(air::PANEL_PAD);
    let inner_x = pad + inset;
    let inner_width = full_width - inset * 2;
    let pitch = Some(faces.body_pitch());
    let tight = metrics.y(air::TIGHT);

    // Under the caption the window's background draws into the top of the block. A panel with
    // no caption of its own still keeps the same inset: what stands on a block never touches
    // its edge.
    let mut y = top + metrics.y(air::PANEL_HEAD);

    for (control, caption, face, own_pitch) in [
        (
            IDC_LETTER_PANEL_TITLE,
            &plan.panel_title,
            faces.number,
            None::<Option<i32>>.flatten(),
        ),
        (IDC_LETTER_PANEL_TEXT, &plan.panel_text, faces.body, pitch),
    ] {
        if caption.is_empty() {
            hide(hwnd, control);
            continue;
        }

        // SAFETY: see the contract.
        let height = unsafe { measure(dc, face, inner_width, caption, own_pitch) };

        place(hwnd, control, inner_x, y, inner_width, height);
        y += height + tight;
    }

    // --- the rows -------------------------------------------------------------------------
    let marker_width = metrics.x(air::MARKER);
    let row_x = inner_x + metrics.x(air::ROW_TEXT);
    let row_width = inner_width - metrics.x(air::ROW_TEXT);

    for (index, (marker_control, text_control)) in LETTER_ROWS.into_iter().enumerate() {
        let Some(row) = plan.rows.get(index) else {
            hide(hwnd, marker_control);
            hide(hwnd, text_control);
            continue;
        };

        // SAFETY: see the contract. A row that names the key is measured **as a chip row**:
        // the figure is wider than the words it replaces, and measuring without it is what put
        // the German «Привет» one line short.
        let height = unsafe {
            measure_row(
                dc,
                faces.body,
                Chip {
                    face: faces.chip,
                    height: faces.chip_height,
                    key: &state.hotkey,
                },
                row_width,
                &row.text,
                pitch,
            )
        };
        // SAFETY: as above — one line of the marker's own face.
        let marker_height = unsafe { measure(dc, faces.number, marker_width, &row.marker, None) };

        place(
            hwnd,
            marker_control,
            inner_x,
            y,
            marker_width,
            marker_height,
        );
        place(hwnd, text_control, row_x, y, row_width, height);

        y += height.max(marker_height) + metrics.y(air::TIGHT + 1);
    }

    // --- the buttons inside the panel -------------------------------------------------------
    let controls = [IDC_LETTER_PANEL_BTN_1, IDC_LETTER_PANEL_BTN_2];

    // SAFETY: see the contract. The wrap is what keeps the second button on the block in a
    // locale that writes the first one long.
    unsafe {
        place_panel_buttons(
            hwnd,
            dc,
            faces.text,
            metrics,
            inner_x,
            y,
            inner_width,
            &[
                (controls[0], !plan.panel_buttons.is_empty()),
                (controls[1], plan.panel_buttons.len() > 1),
            ],
        );
    }

    for (control, button) in controls.into_iter().zip(plan.panel_buttons.iter()) {
        enable(hwnd, control, button.enabled);
    }
}

/// Puts the words of the plan into the controls — FR-94, and the one body that fills a letter
/// window whether it has just been created or has just been handed a new locale.
fn fill_letter(hwnd: HWND, state: &WindowState) {
    let plan = &state.plan;
    let caption = settings::wide(&plan.caption);

    // SAFETY: `hwnd` is the live window and `caption` a NUL-terminated UTF-16 buffer owned by
    // this frame, neither moved nor dropped until the call returns; the call copies it.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    for (control, caption) in [
        (IDC_LETTER_TITLE, &plan.title),
        (IDC_LETTER_SUBTITLE, &plan.subtitle),
        (IDC_LETTER_LEAD, &plan.lead),
        (IDC_LETTER_PARA, &plan.para),
        (IDC_LETTER_DEMO_CAP, &plan.demo_caption),
        (IDC_LETTER_PANEL, &plan.panel),
        (IDC_LETTER_PANEL_TITLE, &plan.panel_title),
        (IDC_LETTER_PANEL_TEXT, &plan.panel_text),
        (IDC_LETTER_FOOT, &plan.foot),
    ] {
        settings::set_text(hwnd, control, caption);
    }

    for (index, (marker_control, text_control)) in LETTER_ROWS.into_iter().enumerate() {
        let (marker, text) = plan
            .rows
            .get(index)
            .map_or((String::new(), String::new()), |row| {
                (row.marker.clone(), row.text.clone())
            });

        settings::set_text(hwnd, marker_control, &marker);
        settings::set_text(hwnd, text_control, &text);
    }

    for (control, button) in [
        (IDC_LETTER_LEFT, plan.left.as_ref()),
        (IDC_LETTER_RIGHT, plan.right.as_ref()),
        (IDC_LETTER_ACCENT, plan.accent.as_ref()),
    ] {
        settings::set_text(hwnd, control, button.map_or("", |button| &button.label));
    }

    for (control, button) in [IDC_LETTER_PANEL_BTN_1, IDC_LETTER_PANEL_BTN_2]
        .into_iter()
        .zip(
            plan.panel_buttons
                .iter()
                .map(Some)
                .chain(std::iter::repeat(None)),
        )
    {
        settings::set_text(hwnd, control, button.map_or("", |button| &button.label));
    }

    // Решение 97.2, ИТОГ-Э30 §9.4: the demonstration is an **island** — Latin letters and two
    // Latin layout names — and an island in a mirrored window keeps its own reading order.
    // The control is owner-drawn, so what the island needs is not this call but the reading
    // order its painting asks for; the call is here for the same reason the about window makes
    // it for its logo — the *place* stays mirrored and only the inside is straightened.
    settings::unmirror_control(hwnd, IDC_LETTER_DEMO);
}

// -----------------------------------------------------------------------------------------
// The window procedure
// -----------------------------------------------------------------------------------------

/// Borrows the state one of these windows keeps, by the pointer in its `GWLP_USERDATA`.
///
/// `None` before `WM_INITDIALOG` has stored the pointer, after `WM_NCDESTROY` has cleared it,
/// and while the state is already borrowed — the last is what makes re-entry safe: a handler
/// that paints while another is painting simply does nothing rather than panicking.
///
/// # Safety
///
/// `hwnd` is one of this module's windows.
unsafe fn with_state<R>(hwnd: HWND, body: impl FnOnce(&mut WindowState) -> R) -> Option<R> {
    // SAFETY: `GWLP_USERDATA` is a field every window has and the dialog manager does not use
    // for itself; this module is the only writer of it on these windows.
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if pointer == 0 {
        return None;
    }

    // SAFETY: the value is the pointer `open_window` boxed and `WM_INITDIALOG` stored, and it
    // is cleared before the box is freed, so a non-zero value names a live `RefCell`.
    let state = unsafe { &*(pointer as *const RefCell<WindowState>) };

    state
        .try_borrow_mut()
        .ok()
        .map(|mut state| body(&mut state))
}

/// The colour role of one static of these windows — the closed vocabulary
/// [`theme::StaticColorRole`] already answers with, reused rather than widened (§6.2).
///
/// The quiet ones are the two lines that are quiet in the mock-up — the subtitle under the
/// heading and the line under the buttons — and the markers of the panel rows, which are the
/// mock-up's `.key` against its row text. Everything else is the full-strength ink.
///
/// Public so that a test can call the very function the window calls.
pub fn label_color_role(control: i32) -> StaticColorRole {
    match control {
        IDC_LETTER_SUBTITLE | IDC_LETTER_FOOT | IDC_LETTER_DEMO_CAP => StaticColorRole::Muted,
        control if (IDC_LETTER_N1..IDC_LETTER_N1 + 4).contains(&control) => StaticColorRole::Muted,
        // The demonstration is a field of its own — it is drawn as one, and its ground is the
        // field colour rather than the window's.
        IDC_LETTER_DEMO => StaticColorRole::Field,
        _ => StaticColorRole::Label,
    }
}

/// The face and the line pitch one label of these windows is set in — решение 85's role table,
/// one pure place, exactly as `settings::about_label_face` is for the about window.
fn label_face(control: i32, faces: &Faces) -> (HFONT, Option<i32>) {
    match control {
        IDC_LETTER_TITLE => (faces.name, None),
        IDC_LETTER_PANEL_TITLE => (faces.number, None),
        control if (IDC_LETTER_N1..IDC_LETTER_N1 + 4).contains(&control) => (faces.number, None),
        IDC_LETTER_LEAD
        | IDC_LETTER_PARA
        | IDC_LETTER_PANEL_TEXT
        | IDC_LETTER_DEMO_CAP
        | IDC_LETTER_DEMO => (faces.body, Some(faces.body_pitch())),
        control if (IDC_LETTER_ROW_1..IDC_LETTER_ROW_1 + 4).contains(&control) => {
            (faces.body, Some(faces.body_pitch()))
        }
        _ => (faces.text, None),
    }
}

/// The background of one of these windows: the ground, the line under the title bar and the
/// panel — the same three the about window draws, in the same order and for the same reason (a
/// block is the background of what stands on it, so it is laid before the children paint).
///
/// # Safety
///
/// Called from [`letter_proc`] with the `wparam` of `WM_ERASEBKGND`.
unsafe fn on_erase(hwnd: HWND, wparam: WPARAM) -> isize {
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);

    let mut client = RECT::default();

    // SAFETY: `hwnd` is the live window and `client` a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return 0;
    }

    // The colour choice, split from the painting: the borrow ends before the DC is touched —
    // the discipline every handler of `settings` keeps, and for the same reason.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            let brushes = state.brushes.as_ref()?;

            Some((
                brushes.window_bg(),
                state.palette.field_border,
                brushes.panel_bg(),
                state.palette.panel_border,
                state.palette.cap,
                state.fonts.as_ref().map(|faces| faces.caption),
                state.kind,
            ))
        })
    };

    let Some(Some((ground, line, panel, border, caption, face, kind))) = choice else {
        return 0;
    };

    // SAFETY: `dc` is the DC of the message, painted into for the length of this send;
    // `ground` is a live brush this window's state owns for longer than the call.
    unsafe { FillRect(dc, &client, ground) };

    theme::paint_caption_underline(dc, &client, line);

    let dpi = theme::dc_dpi(dc);
    let panels: &[i32] = match kind {
        Kind::Letter => &[IDC_LETTER_PANEL],
        Kind::List => &[IDC_LETTERS_PANEL],
        Kind::Author => &[IDC_AUTHOR_PANEL, IDC_NEWS_PANEL, IDC_FEEDBACK_PANEL],
    };

    let rects = settings::child_rects_in_client(hwnd);

    for wanted in panels {
        let Some(rect) = rects
            .iter()
            .find(|(control, _)| control == wanted)
            .map(|(_, rect)| *rect)
        else {
            continue;
        };

        theme::paint_rounded(
            dc,
            &rect,
            theme::scaled(theme::CORNER_RADIUS, dpi),
            border,
            panel,
            dpi,
        );

        if let Some(face) = face {
            // SAFETY: `dc` is the caller's; `face` is a live font this window's state owns for
            // longer than the call, and `draw_panel_caption` puts the previous one back.
            unsafe { settings::draw_panel_caption(hwnd, *wanted, dc, rect, caption, dpi, face) };
        }
    }

    // TRUE — the background is drawn; the manager must not erase over it.
    1
}

/// The `WM_CTLCOLOR*` answers of these windows — the choosing half; the applying half is the
/// shared `settings::apply_ctl_color` (§6.2).
///
/// # Safety
///
/// Called from [`letter_proc`] with the arguments of the message.
unsafe fn on_ctl_color(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);
    let control = HWND(lparam.0 as *mut std::ffi::c_void);

    // SAFETY: `control` is a window handle out of the message; asking for its identifier reads
    // a field of that window and no memory of ours.
    let control_id = unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgCtrlID(control) };

    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            Some(match message {
                WM_CTLCOLORDLG | WM_CTLCOLORBTN => (None, None, brushes.window_bg()),
                WM_CTLCOLORSTATIC => match label_color_role(control_id) {
                    StaticColorRole::Label => (Some(palette.text), None, brushes.window_bg()),
                    StaticColorRole::Muted => (Some(palette.text_muted), None, brushes.window_bg()),
                    StaticColorRole::Field => (
                        Some(palette.text),
                        Some(palette.field_bg),
                        brushes.field_bg(),
                    ),
                },
                _ => return None,
            })
        })
    };

    settings::apply_ctl_color(dc, choice.flatten())
}

/// Draws the demonstration of «Привет» — FR-101, the one picture in this program that moves.
///
/// A field with the word being typed, the key chip that lights up when it is pressed, and the
/// layout indicator that flips when the word is fixed. Everything in it is an **island**
/// (решение 97.2): Latin letters, a Latin key name and two Latin layout names, so the whole
/// rectangle is laid left to right even in a mirrored window.
///
/// # Safety
///
/// Called from [`on_draw_item`] with values copied out of the `WM_DRAWITEM` message.
unsafe fn draw_demo(dc: HDC, rect: RECT, state: &WindowState) -> isize {
    let (Some(brushes), Some(faces)) = (state.brushes.as_ref(), state.fonts.as_ref()) else {
        return 0;
    };

    let dpi = theme::dc_dpi(dc);
    let palette = state.palette;
    let frame = demo_frame(state.tick);

    // The ground first: an owner-drawn static answers for the whole of its rectangle, and the
    // figure below is rounded — without this the four corners would keep whatever was there.
    //
    // SAFETY: `dc` is the DC of the message and `rect` a live local of the caller's; the brush
    // belongs to this window's state.
    unsafe { FillRect(dc, &rect, brushes.window_bg()) };

    // The field the word is typed into.
    theme::paint_rounded(
        dc,
        &rect,
        theme::scaled(theme::CORNER_RADIUS, dpi),
        palette.field_border,
        brushes.field_bg(),
        dpi,
    );

    let inset = theme::scaled(10, dpi);
    let typed: String = frame.text.chars().take(frame.shown).collect();

    // The word, and the two islands beside it — the key chip and the layout badge — are laid
    // out as one chip row: `theme::paint_chip_row` already knows how to put a figure with a
    // word in it beside a sentence, and it is the very body the help rows of «О программе»
    // are drawn by (§6.2).
    let row = RECT {
        left: rect.left + inset,
        top: rect.top + inset / 2,
        right: rect.right - inset,
        bottom: rect.bottom - inset / 2,
    };

    let template = format!(
        "{typed}   {}   {}",
        theme::KEY_PLACEHOLDER,
        DEMO_BADGE[usize::from(frame.converted)]
    );

    // SAFETY: `dc` is the DC of the message and every handle of the style is an object this
    // window's state owns for longer than the call.
    unsafe {
        theme::paint_chip_row(
            dc,
            row,
            theme::chip_row(&template, &state.hotkey),
            theme::ChipRowStyle {
                ground: brushes.field_bg(),
                ink: palette.text,
                body: Some(faces.body),
                chip_face: Some((faces.chip, faces.chip_height)),
                chip: theme::ChipColors {
                    outline: palette.field_border,
                    // The key lights up in the accent while it is pressed — the mock-up's
                    // flash, and the one place this window uses the accent at all.
                    fill: if frame.pressed {
                        brushes.accent_bg()
                    } else {
                        brushes.window_bg()
                    },
                    ink: if frame.pressed {
                        palette.accent_fg
                    } else {
                        palette.text
                    },
                },
                pitch: faces.body_pitch(),
                dpi,
            },
        )
    }
}

/// The `WM_DRAWITEM` of these windows: the owner-drawn statics and the owner-drawn buttons.
///
/// SEC-05: the control type and the identifier are checked before any work, the same copied
/// fields and nothing else are taken out of the message, and `itemData` is never read.
///
/// # Safety
///
/// Called from [`letter_proc`] with the `lparam` of the message: the sender owns the struct it
/// names for the length of the send, and this procedure is inside that send.
unsafe fn on_draw_item(hwnd: HWND, lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return 0;
    }

    // SAFETY: the dialog manager owns the struct for the length of the send, and this
    // procedure is inside that send.
    let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
    let (ctl_type, ctl_id, item_state, dc, rect, item_window) = (
        item.CtlType,
        item.CtlID,
        item.itemState,
        item.hDC,
        item.rcItem,
        item.hwndItem,
    );

    let control = i32::try_from(ctl_id).unwrap_or(-1);

    if ctl_type == ODT_STATIC {
        // SEC-05: a `WM_DRAWITEM` can be forged by any process of our integrity level, so the
        // identifier is checked against the list of statics **this** window really has before
        // anything is drawn — the gate the about window puts in front of its own four labels.
        // SAFETY: see the caller.
        let known = unsafe {
            with_state(hwnd, |state| {
                owner_drawn_labels(state.kind).contains(&control)
            })
        };

        if known != Some(true) {
            return 0;
        }

        // The demonstration is not a caption and is drawn by its own hand.
        if control == IDC_LETTER_DEMO {
            // SAFETY: see the caller.
            return unsafe { with_state(hwnd, |state| draw_demo(dc, rect, state)) }.unwrap_or(0);
        }

        // SAFETY: see the caller.
        return unsafe { draw_label(hwnd, control, dc, rect) };
    }

    if ctl_type != ODT_BUTTON {
        return 0;
    }

    let pressed = item_state.0 & ODS_SELECTED.0 != 0;
    let disabled = item_state.0 & ODS_DISABLED.0 != 0;
    let focused = item_state.0 & ODS_FOCUS.0 != 0 && item_state.0 & ODS_NOFOCUSRECT.0 == 0;
    let hot = settings::is_hot(item_window);

    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            let brushes = state.brushes.as_ref()?;
            let accent = state.accent_control();

            let (colors, hot_brush) = theme::resolve_button_colors(
                // The accented button of a letter is the accent of this window, and every
                // other button is a plain one — the same table the about window's «ОК» is
                // drawn by, asked about the identifier that is accented **here**.
                settings::about_button_colors(
                    if control == accent {
                        settings::OK_COMMAND
                    } else {
                        control
                    },
                    hot,
                    pressed,
                    disabled,
                ),
                brushes.window_bg(),
                brushes,
                state.palette,
            );

            Some((
                colors,
                hot_brush,
                state.fonts.as_ref().map(|faces| faces.text),
            ))
        })
    };

    let Some(Some((colors, _hot_brush, face))) = choice else {
        return 0;
    };

    // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only to
    // paint into for the length of this send; `face` is a face the state owns for longer.
    unsafe { settings::paint_push_button(hwnd, control, dc, rect, colors, focused, face) }
}

/// Draws one owner-drawn label of these windows — the choosing half; the painting half is the
/// shared `theme::paint_label_at_pitch`, or `theme::paint_chip_row` for a row that carries a
/// key name (§6.2: the about window's path, not a copy of it).
///
/// # Safety
///
/// Called from [`on_draw_item`] with values copied out of the message.
unsafe fn draw_label(hwnd: HWND, control: i32, dc: HDC, rect: RECT) -> isize {
    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            let brushes = state.brushes.as_ref()?;

            Some((
                if label_stands_on_a_panel(control) {
                    brushes.panel_bg()
                } else {
                    brushes.window_bg()
                },
                theme::label_ink(label_color_role(control), state.palette),
                state.fonts.as_ref().map(|faces| label_face(control, faces)),
                state.hotkey.clone(),
                state
                    .fonts
                    .as_ref()
                    .map(|faces| (faces.chip, faces.chip_height)),
                state.palette,
            ))
        })
    };

    let Some(Some((ground, ink, face, key, chip_face, palette))) = choice else {
        return 0;
    };

    let (face, pitch) = match face {
        Some((face, pitch)) => (Some(face), pitch),
        None => (None, None),
    };

    // Read after the borrow ends — the discipline of every drawing of `settings`.
    let caption = settings::get_text(hwnd, control);

    // A row that names the hotkey is laid word by word with a chip around the name — решение
    // 85 п. 1, the same body the help rows of «О программе» are drawn by.
    if caption.contains(theme::KEY_PLACEHOLDER) && !key.is_empty() {
        // SAFETY: `dc` and `rect` are the values of the message; every handle is an object the
        // state owns for longer than the call.
        return unsafe {
            theme::paint_chip_row(
                dc,
                rect,
                theme::chip_row(&caption, &key),
                theme::ChipRowStyle {
                    ground,
                    ink,
                    body: face,
                    chip_face,
                    chip: theme::ChipColors {
                        outline: palette.field_border,
                        // The chip stands **on the row's own ground**, which on a panel is the
                        // panel's colour — the same choice `draw_about_help_row` makes for the
                        // rows of «Как пользоваться», and for the same reason: a chip filled
                        // with the window colour would be a hole in the block.
                        fill: ground,
                        ink: palette.text,
                    },
                    pitch: pitch.unwrap_or(0),
                    dpi: theme::dc_dpi(dc),
                },
            )
        };
    }

    let mut wide: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: `dc` and `rect` are the values of the message; `ground` and `face` are objects
    // the state owns for longer than the call.
    unsafe {
        theme::paint_label_at_pitch(
            dc,
            rect,
            &mut wide,
            theme::LabelStyle {
                ground,
                ink,
                face,
                pitch,
                reading: theme::Reading::Native,
            },
        )
    }
}

/// The dialog procedure of the three windows of FR-101 and FR-103 — **one procedure, three
/// windows**, because what differs between them is which slots the plan fills and not how a
/// window of this program behaves.
///
/// # Safety
///
/// Called by the dialog manager with the arguments of a window message. `hwnd` names the live
/// window, and on `WM_INITDIALOG` `lparam` is the pointer [`open_window`] boxed and nothing
/// else — the manager forwards it unchanged and no other sender can reach this procedure
/// (SEC-05).
unsafe extern "system" fn letter_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match message {
        WM_INITDIALOG => {
            // SAFETY: `GWLP_USERDATA` is a field every window has, which the dialog manager
            // does not use for itself. The value stored is the pointer the manager forwarded;
            // it is only ever read back by `with_state` and freed on `WM_NCDESTROY`.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0) };

            // SAFETY: the pointer has just been stored and names the box `open_window` made,
            // which lives until this window's `WM_NCDESTROY`.
            let prepared = unsafe {
                with_state(hwnd, |state| {
                    state.fonts = Faces::new(hwnd, state.accent_control());
                    state.logo = settings::AboutLogo::load(hwnd);

                    OPEN.with_borrow_mut(|open| open.push((state.kind, hwnd)));

                    settings::apply_title_bar_theme(hwnd, state.palette);

                    (
                        state.kind,
                        state.accent_control(),
                        state.plan.demo,
                        state.icon_control(),
                        state.logo.as_ref().map(settings::AboutLogo::handle),
                        state.icons.as_ref().map(settings::CaptionIcons::frames),
                    )
                })
            };

            let Some((kind, accent, demo, icon, logo, frames)) = prepared else {
                return 0;
            };

            settings::subclass_buttons(hwnd, buttons_of(kind));

            if let Some(frames) = frames {
                // SAFETY: `hwnd` is the live window and the handles belong to the value the
                // state keeps, which is dropped only when the window is destroyed.
                unsafe { settings::CaptionIcons::show_on(hwnd, frames) };
            }

            if let Some(logo) = logo {
                // `STM_SETICON`, the documented way to give an `SS_ICON` static a picture —
                // the road the about window already takes for the same 40 px logo. The shared
                // frame it answers with is deliberately dropped: it was never ours to free.
                let _ = settings::send_to(
                    hwnd,
                    icon,
                    windows::Win32::UI::WindowsAndMessaging::STM_SETICON,
                    logo.0 as usize,
                    0,
                );

                // Решение 97.1: a picture carries a meaning and not a direction.
                settings::unmirror_control(hwnd, icon);
            }

            // SAFETY: as above — the pointer names the live box.
            unsafe {
                with_state(hwnd, |state| {
                    fill_window(hwnd, state);
                    layout_window(hwnd, state);
                })
            };

            // The accented button is the default one, handed to the manager by the documented
            // replacement — *posted*, not sent, for the reason `dialog_proc` gives at its own
            // (FR-72).
            //
            // SAFETY: `hwnd` is the live window; the message carries two plain integers and no
            // pointer.
            if let Err(error) = unsafe {
                PostMessageW(
                    Some(hwnd),
                    DM_SETDEFID,
                    WPARAM(usize::try_from(accent).unwrap_or(0)),
                    LPARAM(0),
                )
            } {
                crate::app::report_non_critical("PostMessageW", &error);
            }

            if demo {
                // NFR-10: the one timer of this program that is not the watchdog's, and it
                // exists only while a window with a demonstration is open.
                //
                // SAFETY: `hwnd` is the live window; `None` for the callback asks for
                // `WM_TIMER` at this window, which is what the procedure answers.
                let started = unsafe { SetTimer(Some(hwnd), DEMO_TIMER, DEMO_TICK_MS, None) };

                if started == 0 {
                    crate::app::report_non_critical("SetTimer", &WinError::from_thread());
                }
            }

            // TRUE: let the dialog manager choose the focus.
            1
        }

        // SAFETY: the pointer was stored on `WM_INITDIALOG` and the box lives until
        // `WM_NCDESTROY`.
        WM_ERASEBKGND => unsafe { on_erase(hwnd, wparam) },

        // SAFETY: as above.
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => unsafe {
            on_ctl_color(hwnd, message, wparam, lparam)
        },

        // SAFETY: the sender owns the struct `lparam` names for the length of the send, and
        // this procedure is inside that send.
        WM_DRAWITEM => unsafe { on_draw_item(hwnd, lparam) },

        // The demonstration of «Привет» — one frame. SEC-05: a forged `WM_TIMER` buys the
        // sender one frame of an animation in our own window.
        WM_TIMER if wparam.0 == DEMO_TIMER => {
            // SAFETY: as above.
            let ticked = unsafe {
                with_state(hwnd, |state| {
                    state.tick = state.tick.wrapping_add(1);
                    state.plan.demo
                })
            };

            if ticked == Some(true) {
                settings::repaint_control(hwnd, IDC_LETTER_DEMO);
            }

            0
        }

        WM_COMMAND => {
            let control = i32::from(settings::low_word(wparam.0));

            // Esc, which the dialog manager sends whether or not the window has the button.
            if control == IDCANCEL.0 {
                close_window(hwnd);
                return 0;
            }

            // SAFETY: as above.
            let action = unsafe { with_state(hwnd, |state| state.action_of(control)) }.flatten();

            if let Some(action) = action {
                perform(hwnd, action);
            }

            0
        }

        WM_CLOSE => {
            close_window(hwnd);
            0
        }

        WM_DESTROY => {
            // SAFETY: as above. The window is still alive here and its children with it, so
            // the buttons are still there to be handed back their own procedures.
            let kind = unsafe { with_state(hwnd, |state| state.kind) };

            if let Some(kind) = kind {
                settings::unsubclass_buttons(hwnd, buttons_of(kind));
            }

            // SAFETY: `hwnd` is the live window; killing a timer that was never set answers
            // false and is harmless (NFR-13).
            let _ = unsafe { KillTimer(Some(hwnd), DEMO_TIMER) };

            OPEN.with_borrow_mut(|open| open.retain(|(_, other)| *other != hwnd));

            // «Not handled»: the dialog manager still needs its own `WM_DESTROY`.
            0
        }

        // The last message a window ever gets, and the one place the state is freed.
        WM_NCDESTROY => {
            // SAFETY: the field holds the pointer stored on `WM_INITDIALOG` or zero.
            let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

            // Cleared **before** the box is freed, so that a message arriving in between finds
            // nothing rather than a dangling pointer.
            //
            // SAFETY: `hwnd` is the live window and the value written is zero.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };

            if pointer != 0 {
                // SAFETY: the pointer came from `Box::into_raw` in `open_window` and is turned
                // back into a box exactly once — this message arrives once per window.
                drop(unsafe { Box::from_raw(pointer as *mut RefCell<WindowState>) });
            }

            0
        }

        // The system theme moved while this window is up — the far end of the nudge
        // `settings::on_system_theme_message` posts. SEC-05: the message carries nothing and
        // decides nothing; the palette is resolved afresh out of this program's own setting.
        settings::WM_APP_SYSTEM_THEME => {
            // SAFETY: as above.
            let moved = unsafe {
                with_state(hwnd, |state| {
                    let fresh = theme::resolve(state.setting, theme::system_is_light());

                    if std::ptr::eq(fresh, state.palette) {
                        return false;
                    }

                    let Some(brushes) = theme::Brushes::new(fresh) else {
                        return false;
                    };

                    state.palette = fresh;
                    state.brushes = Some(brushes);
                    true
                })
            };

            if moved == Some(true) {
                // SAFETY: as above.
                unsafe {
                    with_state(hwnd, |state| {
                        settings::apply_title_bar_theme(hwnd, state.palette);
                    })
                };

                settings::repaint_whole_window(hwnd);
            }

            0
        }

        _ => 0,
    }
}

/// Every owner-drawn static of one kind of window — the gate of the static branch of
/// `WM_DRAWITEM`, and the answer to «is this identifier one of ours» (SEC-05).
fn owner_drawn_labels(kind: Kind) -> &'static [i32] {
    match kind {
        Kind::Letter => &LETTER_LABELS,
        Kind::List => &LIST_LABELS,
        Kind::Author => &AUTHOR_LABELS,
    }
}

/// The owner-drawn statics of «Последние письма»: three per entry and the line under the
/// window.
const LIST_LABELS: [i32; 13] = [
    IDC_LETTERS_T1,
    IDC_LETTERS_T1 + 1,
    IDC_LETTERS_T1 + 2,
    IDC_LETTERS_T1 + 3,
    IDC_LETTERS_D1,
    IDC_LETTERS_D1 + 1,
    IDC_LETTERS_D1 + 2,
    IDC_LETTERS_D1 + 3,
    IDC_LETTERS_X1,
    IDC_LETTERS_X1 + 1,
    IDC_LETTERS_X1 + 2,
    IDC_LETTERS_X1 + 3,
    IDC_LETTERS_FOOT,
];

/// The owner-drawn statics of «От автора».
const AUTHOR_LABELS: [i32; 9] = [
    IDC_AUTHOR_NAME,
    IDC_AUTHOR_VERSION,
    IDC_AUTHOR_TEXT,
    IDC_NEWS_ABOUT,
    IDC_NEWS_SWITCH_SUB,
    IDC_NEWS_STATE,
    IDC_NEWS_VERSION,
    IDC_NEWS_FILE_ONLY,
    IDC_FEEDBACK_TEXT,
];

/// The owner-drawn buttons of one kind of window — the list `subclass_buttons` walks.
fn buttons_of(kind: Kind) -> &'static [i32] {
    match kind {
        Kind::Letter => &LETTER_BUTTONS,
        Kind::List => &LIST_BUTTONS,
        Kind::Author => &AUTHOR_BUTTONS,
    }
}

/// The owner-drawn buttons of «Последние письма»: «Открыть письмо» and «Прочитано» for each of
/// the four entries, and the one that closes the window.
const LIST_BUTTONS: [i32; 9] = [
    IDC_LETTERS_B1,
    IDC_LETTERS_B1 + 1,
    IDC_LETTERS_B1 + 2,
    IDC_LETTERS_B1 + 3,
    IDC_LETTERS_R1,
    IDC_LETTERS_R1 + 1,
    IDC_LETTERS_R1 + 2,
    IDC_LETTERS_R1 + 3,
    IDC_LETTERS_CLOSE,
];

/// The owner-drawn buttons of «От автора».
const AUTHOR_BUTTONS: [i32; 6] = [
    IDC_AUTHOR_SUPPORT,
    IDC_AUTHOR_CHANNEL,
    IDC_NEWS_DOWNLOAD,
    IDC_NEWS_LETTERS,
    IDC_FEEDBACK_WRITE,
    IDC_AUTHOR_CLOSE,
];

// -----------------------------------------------------------------------------------------
// «От автора» — FR-103
// -----------------------------------------------------------------------------------------

/// **What the «От автора» window says** — a pure function of the state, the day, the version
/// and the feed, exactly as [`LetterPlan`] is for a letter.
///
/// The window has two states and the difference is one line: before the feed has shown a letter
/// (and before ninety days) the panel carries the quiet line that says the setting is edited in
/// the file; after both, a switch stands in that line's place. FR-102 puts it that way round on
/// purpose — a switch offered on the first day would be a question nobody has the answer to
/// yet, and a feed that has never said anything is not something to turn off.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthorView {
    /// The line under the program's name: the version and the author's alias.
    pub version_line: String,
    /// The paragraph of the «Автор» panel.
    pub author_text: String,
    /// What the feed is and how often it is read — said from the first day (FR-102).
    pub feed_about: String,
    /// When it was last read and when the next reading is, or that it has never been read.
    pub feed_state: String,
    /// The installed version, and the available one when there is a newer.
    pub version_state: String,
    /// The quiet line about the settings file — empty once the switch is shown.
    pub file_only: String,
    /// The switch and its state, or `None` while FR-102 keeps it away.
    pub switch: Option<bool>,
    /// The sentence under the switch.
    pub switch_note: String,
    /// The address «Открыть страницу загрузки» leads to, when there is an update to lead to.
    pub download: Option<String>,
    /// Whether «Последние письма» has anything to show.
    pub letters: bool,
    /// The paragraph of the «Обратная связь» panel.
    pub feedback_text: String,
}

/// The whole of what «От автора» says — FR-103.
pub fn author_view(state: &Letters, today: Date, version: &str, feed: FeedView<'_>) -> AuthorView {
    use crate::settings::{
        IDS_AUTHOR_TEXT, IDS_AUTHOR_VERSION, IDS_FEEDBACK_TEXT, IDS_NEWS_ABOUT_FEED,
        IDS_NEWS_AVAILABLE, IDS_NEWS_FILE_ONLY, IDS_NEWS_LATEST, IDS_NEWS_NEVER_READ,
        IDS_NEWS_READ_ON, IDS_NEWS_SWITCH_SUB, format_text, text,
    };

    let switch = switch_is_shown(state, today).then_some(state.feed);
    let update = update_is_pending(version, feed);

    AuthorView {
        version_line: format_text(IDS_AUTHOR_VERSION, &[version]),
        author_text: text(IDS_AUTHOR_TEXT),
        // The sentence about the feed moves under the switch once there is one — it is the
        // same sentence, and two copies of it on one panel would be a stutter.
        feed_about: if switch.is_some() {
            String::new()
        } else {
            text(IDS_NEWS_ABOUT_FEED)
        },
        switch_note: if switch.is_some() {
            text(IDS_NEWS_SWITCH_SUB)
        } else {
            String::new()
        },
        feed_state: match state.feed_last_read {
            None => text(IDS_NEWS_NEVER_READ),
            Some(day) => format_text(
                IDS_NEWS_READ_ON,
                &[
                    &format_date(day),
                    &days_until_feed_read(state, today).to_string(),
                ],
            ),
        },
        version_state: match update.and_then(|item| item.version.as_deref()) {
            Some(newer) => format_text(IDS_NEWS_AVAILABLE, &[version, newer]),
            None => format_text(IDS_NEWS_LATEST, &[version]),
        },
        file_only: if switch.is_some() {
            String::new()
        } else {
            text(IDS_NEWS_FILE_ONLY)
        },
        switch,
        download: update.map(|item| item.link.clone()),
        letters: !feed.news.is_empty() || update.is_some(),
        feedback_text: text(IDS_FEEDBACK_TEXT),
    }
}

/// A date as a person reads it — «14 октября» — in the language of the interface.
///
/// `GetDateFormatEx` with the locale of `general.language`, so the month is named in the
/// language the window is written in and nothing has to be translated by hand. The picture is
/// this file's own (`d MMMM`), so the shape does not depend on what the user's Windows prefers.
///
/// NFR-13: a refusal falls back to the ISO form the configuration file is written in, which is
/// readable everywhere and wrong nowhere.
pub fn format_date(date: Date) -> String {
    use windows::Win32::Foundation::SYSTEMTIME;
    use windows::Win32::Globalization::{ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx};

    let system = SYSTEMTIME {
        wYear: u16::try_from(date.year()).unwrap_or(0),
        wMonth: u16::try_from(date.month()).unwrap_or(1),
        wDay: u16::try_from(date.day()).unwrap_or(1),
        ..SYSTEMTIME::default()
    };

    let locale = settings::wide(settings::ui_language().tag());
    let mut buffer = [0u16; 64];

    // SAFETY: `locale` is a NUL-terminated buffer of this frame, `system` a live local, the
    // picture a static literal of this image, and the buffer a live local whose length the call
    // is told. `PCWSTR::null()` is the documented value of the reserved calendar argument.
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

    if written <= 0 {
        return date.to_string();
    }

    let units = usize::try_from(written).unwrap_or(1).saturating_sub(1);

    buffer
        .get(..units)
        .and_then(|slice| String::from_utf16(slice).ok())
        .unwrap_or_else(|| date.to_string())
}

// -----------------------------------------------------------------------------------------
// Filling and laying out, by kind
// -----------------------------------------------------------------------------------------

/// Puts the words into one of the three windows — the one body that fills a window whether it
/// has just been created or has just been handed a new locale (FR-94, решение 99.1).
fn fill_window(hwnd: HWND, state: &WindowState) {
    match state.kind {
        Kind::Letter => fill_letter(hwnd, state),
        Kind::List => fill_list(hwnd, state),
        Kind::Author => fill_author(hwnd, state),
    }
}

/// Lays out one of the three windows.
///
/// # Safety
///
/// `hwnd` is the live window and `state` its own state.
unsafe fn layout_window(hwnd: HWND, state: &WindowState) {
    match state.kind {
        Kind::Letter => unsafe { layout_letter(hwnd, state) },
        Kind::List => unsafe { layout_list(hwnd, state) },
        Kind::Author => unsafe { layout_author(hwnd, state) },
    }
}

/// One entry of «Последние письма» — FR-101: what it says, when it arrived, whether it has been
/// read, and whether it is still worth a «Прочитано».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The heading of the letter.
    pub title: String,
    /// «14 октября · обновление · у вас 0.38.0», or «2 ноября · не прочитано».
    pub meta: String,
    /// The first line of what it said.
    pub excerpt: String,
    /// The identifier of the news item, for the «Прочитано» button; `None` for the update.
    pub news: Option<u64>,
    /// Whether the entry still has something to mark as read.
    pub unread: bool,
}

/// **What «Последние письма» shows** — FR-101, a pure function of the state and the feed.
///
/// The update first, when there is one worth showing, then the news the program keeps, newest
/// first. Stage А never has anything to put here: the feed is stage Б, and an empty list is a
/// window with its own line under it saying what it holds.
pub fn list_entries(state: &Letters, version: &str, feed: FeedView<'_>) -> Vec<Entry> {
    use crate::settings::{
        IDS_ENTRY_READ_MARK, IDS_ENTRY_UNREAD_MARK, IDS_ENTRY_UPDATE_MARK, format_text,
    };

    let mut entries = Vec::with_capacity(NEWS_KEPT + 1);

    if let Some(update) = update_is_pending(version, feed) {
        entries.push(Entry {
            title: update.title.clone(),
            meta: format_text(
                IDS_ENTRY_UPDATE_MARK,
                &[&update.date.map_or_else(String::new, format_date), version],
            ),
            excerpt: first_line(&update.text),
            news: None,
            unread: false,
        });
    }

    for item in feed.news.iter().rev() {
        let read = state.is_read(item.id);

        entries.push(Entry {
            title: item.title.clone(),
            meta: format_text(
                if read {
                    IDS_ENTRY_READ_MARK
                } else {
                    IDS_ENTRY_UNREAD_MARK
                },
                &[&item.date.map_or_else(String::new, format_date)],
            ),
            excerpt: first_line(&item.text),
            news: Some(item.id),
            unread: !read,
        });
    }

    entries
}

/// The first line of a letter's text — what an entry of the list shows of it.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// Puts the entries into «Последние письма» — FR-101.
fn fill_list(hwnd: HWND, state: &WindowState) {
    use crate::settings::{
        IDS_CLOSE, IDS_LETTERS_CAPTION, IDS_LETTERS_FOOT, IDS_LETTERS_OPEN, IDS_NEWS_READ_BUTTON,
        text,
    };

    let caption = settings::wide(&text(IDS_LETTERS_CAPTION));

    // SAFETY: `hwnd` is the live window and `caption` a NUL-terminated buffer of this frame.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    settings::set_text(hwnd, IDC_LETTERS_FOOT, &text(IDS_LETTERS_FOOT));
    settings::set_text(hwnd, IDC_LETTERS_CLOSE, &text(IDS_CLOSE));

    // Read once rather than once per entry: four entries would otherwise be eight walks of the
    // string tables for two sentences.
    let open = text(IDS_LETTERS_OPEN);
    let read = text(IDS_NEWS_READ_BUTTON);

    for index in 0..NEWS_KEPT + 1 {
        let entry = state.entries.get(index);
        let offset = i32::try_from(index).unwrap_or(0);

        settings::set_text(
            hwnd,
            IDC_LETTERS_T1 + offset,
            entry.map_or("", |entry| &entry.title),
        );
        settings::set_text(
            hwnd,
            IDC_LETTERS_D1 + offset,
            entry.map_or("", |entry| &entry.meta),
        );
        settings::set_text(
            hwnd,
            IDC_LETTERS_X1 + offset,
            entry.map_or("", |entry| &entry.excerpt),
        );
        settings::set_text(
            hwnd,
            IDC_LETTERS_B1 + offset,
            if entry.is_some() { &open } else { "" },
        );
        settings::set_text(
            hwnd,
            IDC_LETTERS_R1 + offset,
            if entry.is_some_and(|entry| entry.unread) {
                &read
            } else {
                ""
            },
        );
    }
}

/// Lays out «Последние письма»: one panel and up to four entries in it, each as tall as what it
/// says.
///
/// # Safety
///
/// As [`layout_letter`].
unsafe fn layout_list(hwnd: HWND, state: &WindowState) {
    let Some(faces) = state.fonts.as_ref() else {
        return;
    };

    let mut client = RECT::default();

    // SAFETY: `hwnd` is the live window and `client` a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return;
    }

    // SAFETY: `hwnd` is the live window; the DC is released before every return below.
    let dc = unsafe { GetDC(Some(hwnd)) };

    if dc.is_invalid() {
        return;
    }

    let metrics = Metrics::of(hwnd);
    let pad = metrics.x(air::PAD);
    let full_width = client.right - pad * 2;
    let inset = metrics.x(air::PANEL_PAD);
    let inner_x = pad + inset;
    let inner_width = full_width - inset * 2;
    let pitch = Some(faces.body_pitch());
    let tight = metrics.y(air::TIGHT);

    // The line under the window and the button that closes it, from the bottom.
    let mut bottom = client.bottom - metrics.y(air::PAD);

    // SAFETY: `dc` is this window's DC and the faces are its own.
    let foot = unsafe {
        measure(
            dc,
            faces.text,
            full_width,
            &settings::get_text(hwnd, IDC_LETTERS_FOOT),
            None,
        )
    };

    bottom -= foot;
    place(hwnd, IDC_LETTERS_FOOT, pad, bottom, full_width, foot);
    bottom -= tight + metrics.y(air::BUTTON);

    // SAFETY: as above.
    let width = unsafe {
        button_width(
            dc,
            faces.text,
            metrics,
            &settings::get_text(hwnd, IDC_LETTERS_CLOSE),
        )
    };

    place(
        hwnd,
        IDC_LETTERS_CLOSE,
        client.right - pad - width,
        bottom,
        width,
        metrics.y(air::BUTTON),
    );

    let top = metrics.y(air::TOP);

    place(
        hwnd,
        IDC_LETTERS_PANEL,
        pad,
        top,
        full_width,
        bottom - metrics.y(air::GAP) - top,
    );

    let mut y = top + inset;

    for index in 0..NEWS_KEPT + 1 {
        let offset = i32::try_from(index).unwrap_or(0);
        let controls = [
            IDC_LETTERS_T1 + offset,
            IDC_LETTERS_D1 + offset,
            IDC_LETTERS_X1 + offset,
            IDC_LETTERS_B1 + offset,
            IDC_LETTERS_R1 + offset,
        ];

        let Some(entry) = state.entries.get(index) else {
            for control in controls {
                hide(hwnd, control);
            }

            continue;
        };

        for (control, caption, face, own_pitch) in [
            (controls[0], &entry.title, faces.number, None),
            (controls[1], &entry.meta, faces.text, None),
            (controls[2], &entry.excerpt, faces.body, pitch),
        ] {
            // SAFETY: as above.
            let height = unsafe { measure(dc, face, inner_width, caption, own_pitch) };

            place(hwnd, control, inner_x, y, inner_width, height);
            y += height + tight;
        }

        // SAFETY: as above.
        unsafe {
            y = place_panel_buttons(
                hwnd,
                dc,
                faces.text,
                metrics,
                inner_x,
                y,
                inner_width,
                &[(controls[3], true), (controls[4], entry.unread)],
            );
        }

        y += metrics.y(air::GAP);
    }

    // SAFETY: releases exactly the DC taken at the top of this function, once.
    unsafe { ReleaseDC(Some(hwnd), dc) };
}

/// Puts the words of [`AuthorView`] into «От автора» — FR-103.
fn fill_author(hwnd: HWND, state: &WindowState) {
    use crate::settings::{
        IDS_AUTHOR_CAPTION, IDS_AUTHOR_PANEL, IDS_CHANNEL_OPEN, IDS_CLOSE, IDS_FEEDBACK_PANEL,
        IDS_NEWS_DOWNLOAD, IDS_NEWS_LETTERS, IDS_NEWS_PANEL, IDS_NEWS_SWITCH, IDS_SUPPORT_OPEN,
        IDS_WRITE_TO_AUTHOR, text,
    };

    let view = &state.author;
    let caption = settings::wide(&text(IDS_AUTHOR_CAPTION));

    // SAFETY: `hwnd` is the live window and `caption` a NUL-terminated buffer of this frame.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    for (control, caption) in [
        (IDC_AUTHOR_VERSION, view.version_line.clone()),
        (IDC_AUTHOR_PANEL, text(IDS_AUTHOR_PANEL)),
        (IDC_AUTHOR_TEXT, view.author_text.clone()),
        (IDC_AUTHOR_SUPPORT, text(IDS_SUPPORT_OPEN)),
        (IDC_AUTHOR_CHANNEL, text(IDS_CHANNEL_OPEN)),
        (IDC_NEWS_PANEL, text(IDS_NEWS_PANEL)),
        (IDC_NEWS_ABOUT, view.feed_about.clone()),
        (IDC_NEWS_SWITCH, text(IDS_NEWS_SWITCH)),
        (IDC_NEWS_SWITCH_SUB, view.switch_note.clone()),
        (IDC_NEWS_STATE, view.feed_state.clone()),
        (IDC_NEWS_VERSION, view.version_state.clone()),
        (IDC_NEWS_DOWNLOAD, text(IDS_NEWS_DOWNLOAD)),
        (IDC_NEWS_LETTERS, text(IDS_NEWS_LETTERS)),
        (IDC_NEWS_FILE_ONLY, view.file_only.clone()),
        (IDC_FEEDBACK_PANEL, text(IDS_FEEDBACK_PANEL)),
        (IDC_FEEDBACK_TEXT, view.feedback_text.clone()),
        (IDC_FEEDBACK_WRITE, text(IDS_WRITE_TO_AUTHOR)),
        (IDC_AUTHOR_CLOSE, text(IDS_CLOSE)),
    ] {
        settings::set_text(hwnd, control, &caption);
    }

    // The two buttons that lead to an address that is still a placeholder are drawn and
    // disabled — полномочия П7 и П8.
    enable(
        hwnd,
        IDC_AUTHOR_SUPPORT,
        !links::is_placeholder(links::SUPPORT_URL),
    );
    enable(
        hwnd,
        IDC_AUTHOR_CHANNEL,
        !links::is_placeholder(links::CHANNEL_URL),
    );
}

/// Lays out «От автора»: three panels, each as tall as what stands on it, and the window as
/// tall as the three together — the one window of FR-103 whose height the content decides.
///
/// # Safety
///
/// As [`layout_letter`].
unsafe fn layout_author(hwnd: HWND, state: &WindowState) {
    let Some(faces) = state.fonts.as_ref() else {
        return;
    };

    let mut client = RECT::default();

    // SAFETY: `hwnd` is the live window and `client` a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return;
    }

    // SAFETY: `hwnd` is the live window; the DC is released on every path below.
    let dc = unsafe { GetDC(Some(hwnd)) };

    if dc.is_invalid() {
        return;
    }

    let metrics = Metrics::of(hwnd);
    let view = &state.author;
    let pad = metrics.x(air::PAD);
    let full_width = client.right - pad * 2;
    let inset = metrics.x(air::PANEL_PAD);
    let inner_x = pad + inset;
    let inner_width = full_width - inset * 2;
    let pitch = Some(faces.body_pitch());
    let tight = metrics.y(air::TIGHT);
    let gap = metrics.y(air::GAP);
    let button = metrics.y(air::BUTTON);

    // The head: the name is a template literal and the version line is the view's.
    let mut top = metrics.y(air::TOP);

    // SAFETY: `dc` is this window's DC and the faces are its own.
    let name_height = unsafe {
        measure(
            dc,
            faces.name,
            full_width - metrics.x(air::TEXT_X - air::PAD),
            "Lang Switcher",
            None,
        )
    };

    place(
        hwnd,
        IDC_AUTHOR_NAME,
        metrics.x(air::TEXT_X),
        top,
        client.right - metrics.x(air::TEXT_X) - pad,
        name_height,
    );
    top += name_height + tight;

    // SAFETY: as above.
    let version_height = unsafe {
        measure(
            dc,
            faces.text,
            client.right - metrics.x(air::TEXT_X) - pad,
            &view.version_line,
            None,
        )
    };

    place(
        hwnd,
        IDC_AUTHOR_VERSION,
        metrics.x(air::TEXT_X),
        top,
        client.right - metrics.x(air::TEXT_X) - pad,
        version_height,
    );

    top = (top + version_height).max(metrics.y(13 + 22)) + gap;

    // --- panel «Автор» ----------------------------------------------------------------------
    let mut panel_top = top;
    let mut y = panel_top + metrics.y(air::PANEL_HEAD);

    // SAFETY: as above.
    let height = unsafe { measure(dc, faces.body, inner_width, &view.author_text, pitch) };
    place(hwnd, IDC_AUTHOR_TEXT, inner_x, y, inner_width, height);
    y += height + tight;

    // SAFETY: as above.
    unsafe {
        y = place_panel_buttons(
            hwnd,
            dc,
            faces.text,
            metrics,
            inner_x,
            y,
            inner_width,
            &[(IDC_AUTHOR_SUPPORT, true), (IDC_AUTHOR_CHANNEL, true)],
        );
    }

    place(
        hwnd,
        IDC_AUTHOR_PANEL,
        pad,
        panel_top,
        full_width,
        y + inset - panel_top,
    );
    top = y + inset + gap;

    // --- panel «Новости и обновления» --------------------------------------------------------
    panel_top = top;
    y = panel_top + metrics.y(air::PANEL_HEAD);

    match view.switch {
        // FR-102: before the switch may be shown, the panel says what the feed is and that the
        // setting lives in the file.
        None => {
            hide(hwnd, IDC_NEWS_SWITCH);
            hide(hwnd, IDC_NEWS_SWITCH_SUB);

            // SAFETY: as above.
            let height = unsafe { measure(dc, faces.body, inner_width, &view.feed_about, pitch) };
            place(hwnd, IDC_NEWS_ABOUT, inner_x, y, inner_width, height);
            y += height + tight;
        }
        Some(checked) => {
            hide(hwnd, IDC_NEWS_ABOUT);

            place(
                hwnd,
                IDC_NEWS_SWITCH,
                inner_x,
                y,
                inner_width,
                metrics.y(10),
            );
            settings::send_to(
                hwnd,
                IDC_NEWS_SWITCH,
                windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                usize::from(checked),
                0,
            );
            y += metrics.y(11);

            // SAFETY: as above.
            let height = unsafe {
                measure(
                    dc,
                    faces.body,
                    inner_width - metrics.x(air::ROW_TEXT),
                    &view.switch_note,
                    pitch,
                )
            };
            place(
                hwnd,
                IDC_NEWS_SWITCH_SUB,
                inner_x + metrics.x(air::ROW_TEXT),
                y,
                inner_width - metrics.x(air::ROW_TEXT),
                height,
            );
            y += height + tight;
        }
    }

    for (control, caption) in [
        (IDC_NEWS_STATE, &view.feed_state),
        (IDC_NEWS_VERSION, &view.version_state),
    ] {
        // SAFETY: as above.
        let height = unsafe { measure(dc, faces.body, inner_width, caption, pitch) };
        place(hwnd, control, inner_x, y, inner_width, height);
        y += height + tight;
    }

    // SAFETY: as above.
    unsafe {
        y = place_panel_buttons(
            hwnd,
            dc,
            faces.text,
            metrics,
            inner_x,
            y,
            inner_width,
            &[
                (IDC_NEWS_DOWNLOAD, view.download.is_some()),
                (IDC_NEWS_LETTERS, view.letters),
            ],
        );
    }

    if view.file_only.is_empty() {
        hide(hwnd, IDC_NEWS_FILE_ONLY);
    } else {
        // SAFETY: as above.
        let height = unsafe { measure(dc, faces.text, inner_width, &view.file_only, None) };
        place(hwnd, IDC_NEWS_FILE_ONLY, inner_x, y, inner_width, height);
        y += height + tight;
    }

    place(
        hwnd,
        IDC_NEWS_PANEL,
        pad,
        panel_top,
        full_width,
        y + inset - panel_top,
    );
    top = y + inset + gap;

    // --- panel «Обратная связь» --------------------------------------------------------------
    panel_top = top;
    y = panel_top + metrics.y(air::PANEL_HEAD);

    // SAFETY: as above.
    let height = unsafe { measure(dc, faces.body, inner_width, &view.feedback_text, pitch) };
    place(hwnd, IDC_FEEDBACK_TEXT, inner_x, y, inner_width, height);
    y += height + tight;

    // SAFETY: as above.
    unsafe {
        y = place_panel_buttons(
            hwnd,
            dc,
            faces.text,
            metrics,
            inner_x,
            y,
            inner_width,
            &[(IDC_FEEDBACK_WRITE, true)],
        );
    }

    place(
        hwnd,
        IDC_FEEDBACK_PANEL,
        pad,
        panel_top,
        full_width,
        y + inset - panel_top,
    );
    top = y + inset + gap;

    // --- the button that closes the window, and then the window's own height -----------------
    // SAFETY: as above.
    let width = unsafe {
        button_width(
            dc,
            faces.text,
            metrics,
            &settings::get_text(hwnd, IDC_AUTHOR_CLOSE),
        )
    };

    place(
        hwnd,
        IDC_AUTHOR_CLOSE,
        client.right - pad - width,
        top,
        width,
        button,
    );

    let wanted = top + button + metrics.y(air::PAD);

    // SAFETY: releases exactly the DC taken at the top of this function, once.
    unsafe { ReleaseDC(Some(hwnd), dc) };

    resize_client(hwnd, client.bottom, wanted);
}

/// Puts a row of buttons on a panel, left to right, **wrapping to the next line when the row
/// would run off the block**, and answers where the panel's content now ends. A button that is
/// not wanted is hidden and takes no room.
///
/// ⚠ **The wrap is not decoration.** The first screenshot of «Спасибо» showed «Открыть канал»
/// cut off at the right edge: Russian writes «Открыть страницу поддержки» beside it, and the
/// two together are wider than the panel. A table of widths per locale would be a table that is
/// wrong in the fifteenth language; measuring the caption and starting a new line when it does
/// not fit is right in every one of them (`scratchpad-Э32\снимки`, `ru-dark-thanks`).
///
/// # Safety
///
/// As [`measure`].
#[allow(clippy::too_many_arguments)]
unsafe fn place_panel_buttons(
    hwnd: HWND,
    dc: HDC,
    face: HFONT,
    metrics: Metrics,
    x: i32,
    y: i32,
    available: i32,
    buttons: &[(i32, bool)],
) -> i32 {
    let height = metrics.y(air::BUTTON);
    let gap = metrics.x(air::BUTTON_GAP);
    let mut left = x;
    let mut top = y;
    let mut any = false;

    for (control, wanted) in buttons {
        if !wanted {
            hide(hwnd, *control);
            continue;
        }

        // SAFETY: see the contract.
        let width = unsafe { button_width(dc, face, metrics, &settings::get_text(hwnd, *control)) };

        // Not the first on its line, and it would run off the block: it starts a new one.
        if left > x && left + width > x + available {
            left = x;
            top += height + metrics.y(air::TIGHT);
        }

        place(hwnd, *control, left, top, width.min(available), height);
        left += width + gap;
        any = true;
    }

    if any {
        top + height + metrics.y(air::TIGHT)
    } else {
        y
    }
}

/// Makes the window as tall as its content — the one window of the three that does this
/// (FR-103; the letters keep the fixed height решение 101 п. 7 gives them).
///
/// The client area is what is measured and the window is what is moved, so the difference
/// between the two — the title bar and the frame — is taken from the window itself rather than
/// guessed at.
fn resize_client(hwnd: HWND, current: i32, wanted: i32) {
    if wanted <= 0 || wanted == current {
        return;
    }

    let mut window = RECT::default();

    // SAFETY: `hwnd` is the live window and `window` a live local the call fills.
    if unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut window) }.is_err()
    {
        return;
    }

    let chrome = (window.bottom - window.top) - current;

    // SAFETY: `hwnd` is the live window; the numbers are plain values and no pointer is passed.
    // `SWP_NOMOVE` is not asked for — the window is centred by the manager and moving it would
    // put it somewhere nobody chose.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            None,
            window.left,
            window.top,
            window.right - window.left,
            wanted + chrome,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

// -----------------------------------------------------------------------------------------
// Opening, closing and acting
// -----------------------------------------------------------------------------------------

thread_local! {
    /// What the last successful read of the feed left behind — the one update entry and the
    /// news items the program keeps (FR-102).
    ///
    /// Thread-local on the **UI thread**, which is the only thread that shows a letter or
    /// builds a menu. The feed thread of stage Б does not write here: it publishes to the UI
    /// thread by message (SEC-05 — the message says «look in the box», the data is checked in
    /// the box), and the UI thread is what calls [`publish_feed`].
    ///
    /// Empty for the whole of stage А: there is no feed yet, and every rule that reads it
    /// answers «nothing due» for an empty one.
    static FEED: RefCell<(Option<FeedItem>, Vec<FeedItem>)> =
        const { RefCell::new((None, Vec::new())) };
}

/// Lends the feed to whoever asks — a borrow and not a copy, because a letter reads it and
/// keeps nothing.
pub fn with_feed<R>(body: impl FnOnce(FeedView<'_>) -> R) -> R {
    FEED.with_borrow(|(update, news)| {
        body(FeedView {
            update: update.as_ref(),
            news,
        })
    })
}

/// Publishes what a read of the feed produced — stage Б, called on the UI thread.
pub fn publish_feed(update: Option<FeedItem>, news: Vec<FeedItem>) {
    FEED.with_borrow_mut(|stored| *stored = (update, news));
}

/// What the letters have to say in the menu right now — FR-91's two temporary entries.
///
/// Answers whether a news item is unread and, if the feed names a newer version than the one
/// running, which one.
pub fn pending() -> (bool, Option<String>) {
    let stored = state_now();
    let version = version_string();

    with_feed(|feed| {
        (
            has_unread_news(&stored, feed),
            update_is_pending(&version, feed).and_then(|item| item.version.clone()),
        )
    })
}

/// Shows the letter one of the two temporary menu entries is about — FR-91, FR-101.
///
/// The person asked for it, so the window comes to the front.
pub fn show_pending(owner: HWND, update: bool) {
    let stored = state_now();

    let letter = with_feed(|feed| {
        if update {
            update_is_pending(&version_string(), feed).map(|item| (Letter::Update, item.clone()))
        } else {
            news_due_or_unread(&stored, feed).map(|item| (Letter::News(item.id), item.clone()))
        }
    });

    if let Some((letter, item)) = letter {
        show_letter(owner, letter, Some(&item), true);
    }
}

/// The unread news item the menu entry is about — the first unread of the ones kept, whether or
/// not a reminder is due for it. The entry stands **until «Прочитано»** (FR-101), long after the
/// two reminders have run out, so it cannot ask [`news_due`], which stops with them.
fn news_due_or_unread<'a>(state: &Letters, feed: FeedView<'a>) -> Option<&'a FeedItem> {
    feed.news.iter().find(|item| !state.is_read(item.id))
}

/// This build's version as `VERSIONINFO` spells it — `"0.39.0"`, three parts and not four, the
/// same three the about window shows.
///
/// Empty when the resource cannot be read, which is next to impossible for a window created
/// from a template in that very resource section (NFR-13).
pub fn version_string() -> String {
    crate::tray::file_version().map_or_else(String::new, |(major, minor, patch, _)| {
        format!("{major}.{minor}.{patch}")
    })
}

/// Reads `[letters]` out of the running configuration.
pub fn state_now() -> Letters {
    crate::tray::with_tray(|tray| tray.config().letters.clone()).unwrap_or_default()
}

/// Changes `[letters]` and writes the file — the one road every button of a letter takes to the
/// configuration.
///
/// Through the tray, because the tray is the single writer of `config.toml` in this program
/// (`Tray::save_config`, and the `SavePolicy` that stands in front of it). A letter that wrote
/// the file itself would be a second writer, and a second writer is how a file from a newer
/// schema gets overwritten by an older one.
pub fn update_state(change: impl FnOnce(&mut Letters)) {
    crate::tray::with_tray(|tray| {
        let mut config = tray.config().clone();

        change(&mut config.letters);
        tray.replace_config(config);
    });
}

/// Closes one of these windows.
fn close_window(hwnd: HWND) {
    // SAFETY: `hwnd` is the live window this module made. `DestroyWindow` and not `EndDialog`:
    // the window is modeless, so there is no modal loop to end — and `EndDialog` on a modeless
    // dialog is documented to do nothing at all.
    if let Err(error) = unsafe { DestroyWindow(hwnd) } {
        crate::app::report_non_critical("DestroyWindow", &error);
    }
}

/// Closes the open window of this kind, if there is one — what the tray calls when the program
/// is coming down and what `Reopen` calls before it builds the window again.
pub fn close(kind: Kind) {
    if let Some(hwnd) = window_of(kind) {
        close_window(hwnd);
    }
}

/// Does what a button of a letter says it does — FR-101, FR-103.
fn perform(hwnd: HWND, action: Action) {
    // SAFETY: the pointer was stored on `WM_INITDIALOG` and the box lives until
    // `WM_NCDESTROY`. Everything needed is taken out **before** anything is done, so no action
    // runs while the state is borrowed.
    let taken = unsafe {
        with_state(hwnd, |state| {
            (state.owner, state.news, state.link.clone(), state.kind)
        })
    };

    let Some((owner, news, link, _kind)) = taken else {
        return;
    };

    match action {
        Action::Close => close_window(hwnd),

        Action::Snooze => {
            if let Some(today) = today() {
                update_state(|state| snooze_thanks(state, today));
            }

            close_window(hwnd);
        }

        Action::MarkRead => {
            if let Some(id) = news {
                update_state(|state| mark_read(state, id));
                crate::tray::refresh_unread_mark();
            }

            close_window(hwnd);
        }

        // FR-92 through the one door the tray already opens it by — a second `show_dialog`
        // call site would be a second place that can forget the guard of `dialog_is_open`.
        Action::OpenSettings => crate::tray::dispatch_command(owner, crate::tray::CMD_SETTINGS),

        Action::OpenChannel => open_link(hwnd, links::CHANNEL_URL),
        Action::OpenSupport => open_link(hwnd, links::SUPPORT_URL),
        Action::OpenDownload | Action::OpenLink => open_link(hwnd, &link),

        Action::OpenAuthor => open_author(owner),
        Action::OpenLetters => open_list(owner),

        // FR-104 arrives in stage В. Until then the button that would ask for it leads to the
        // window that will hold it, which is where a person looking for it will look next.
        Action::OpenWizard => open_author(owner),
    }
}

/// Opens one address in the shell — the road the journal folder already takes.
///
/// ⛔ **Nothing is downloaded and nothing is run**: `ShellExecuteW` with `"open"` on an
/// `https://` address hands it to whatever the person's system opens links with, and that is
/// the whole of what this program does with a link (вопрос 101 п. 1, SEC-03).
///
/// A placeholder never reaches here — the button that would open it is drawn disabled — and the
/// check is made again all the same: a disabled button is a fact about the screen, and this is
/// a fact about the program.
fn open_link(hwnd: HWND, url: &str) {
    if url.is_empty() || links::is_placeholder(url) {
        return;
    }

    let wide = settings::wide(url);

    // SAFETY: `wide` is a NUL-terminated UTF-16 buffer owned by this frame and neither moved
    // nor dropped until the call returns; the two null pointers are the documented way to pass
    // no arguments and no working directory. `hwnd` is the live window and becomes the owner of
    // any error box the shell decides to show.
    let result = unsafe {
        ShellExecuteW(
            Some(hwnd),
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // NFR-13. `ShellExecuteW` reports failure as a value of 32 or below in what is nominally a
    // module handle — the one Win32 return value that is neither a `BOOL` nor an error code.
    if result.0 as usize <= 32 {
        crate::app::report_non_critical("ShellExecuteW", &WinError::from_thread());
    }
}

/// Raises a window that is already open — the answer to «open it again».
fn raise(hwnd: HWND, activate: bool) {
    // SAFETY: `hwnd` is a live window of this module.
    let _ = unsafe {
        ShowWindow(
            hwnd,
            if activate {
                SW_SHOWNORMAL
            } else {
                SW_SHOWNOACTIVATE
            },
        )
    };

    if activate {
        // FR-101: a letter never takes the focus **by itself**. This is the other case — the
        // person clicked the balloon or asked for the window from the menu, and a window that
        // did not come to the front then would look broken.
        //
        // SAFETY: `hwnd` is the live window; the call takes no pointer.
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}

/// Builds one of these windows, or raises the one that is already up.
///
/// `activate` is FR-101's whole rule about focus: a letter that opens **by itself** is shown
/// with `SW_SHOWNOACTIVATE` and never calls `SetForegroundWindow`, so it cannot take the
/// keyboard from what somebody is typing into. A window the person asked for — from the menu,
/// from «О программе», by clicking the balloon — is shown the ordinary way.
fn open_window(owner: HWND, state: WindowState, activate: bool) -> Option<HWND> {
    let kind = state.kind;

    if let Some(hwnd) = window_of(kind) {
        raise(hwnd, activate);
        return Some(hwnd);
    }

    // SAFETY: `None` asks for the handle of the file used to create the calling process, which
    // is the running executable — the module `app.rc` was linked into, and therefore the one
    // holding the templates. The handle is borrowed and must not be freed.
    let module =
        match unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(PCWSTR::null()) } {
            Ok(module) => module,
            Err(error) => {
                crate::app::report_non_critical("GetModuleHandleW", &error);
                return None;
            }
        };

    let template = match kind {
        Kind::Letter => IDD_LETTER,
        Kind::List => IDD_LETTERS_LIST,
        Kind::Author => IDD_AUTHOR,
    };

    // The state outlives this call, so it goes on the heap and the window owns it — see
    // `settings::show_modeless_dialog`, which says so in as many words.
    let boxed = Box::into_raw(Box::new(RefCell::new(state)));

    // SAFETY: `module` is this program's own image, `owner` a live window of this thread, and
    // `boxed` a pointer to a value that lives until the window's `WM_NCDESTROY` frees it.
    let created = unsafe {
        settings::show_modeless_dialog(
            HINSTANCE(module.0),
            template,
            owner,
            Some(letter_proc),
            LPARAM(boxed as isize),
        )
    };

    let Some(hwnd) = created else {
        // The window was never made, so nothing will ever free the box: this is the one path
        // that has to.
        //
        // SAFETY: the pointer came from `Box::into_raw` a moment ago and no window took it.
        drop(unsafe { Box::from_raw(boxed) });
        return None;
    };

    raise(hwnd, activate);

    Some(hwnd)
}

/// The state one of these windows starts with — the palette out of the configuration, the
/// hotkey as it acts, and the plan.
fn fresh_state(owner: HWND, kind: Kind) -> WindowState {
    let (setting, hotkey) = crate::tray::with_tray(|tray| {
        (
            tray.config().general.theme,
            settings::effective_hotkey_name(&tray.config().hotkey.key),
        )
    })
    .unwrap_or_else(|| (ThemeSetting::System, settings::effective_hotkey_name("")));

    let palette = theme::resolve(setting, theme::system_is_light());

    WindowState {
        kind,
        letter: None,
        news: None,
        link: String::new(),
        setting,
        palette,
        brushes: theme::Brushes::new(palette),
        fonts: None,
        plan: LetterPlan::default(),
        author: AuthorView::default(),
        entries: Vec::new(),
        hotkey,
        icons: settings::CaptionIcons::load(),
        logo: None,
        tick: 0,
        language: settings::ui_language(),
        owner,
    }
}

/// **Shows one letter** — FR-101.
///
/// Answers whether a window is on the screen. `false` means the letter had nothing to say (a
/// letter out of the feed with no entry behind it) or the window could not be made — and the
/// caller must then not record the letter as shown.
pub fn show_letter(owner: HWND, letter: Letter, item: Option<&FeedItem>, activate: bool) -> bool {
    let stored = state_now();
    let version = version_string();

    let plan = plan_for(
        letter,
        &PlanContext {
            version: version.clone(),
            previous_version: stored.last_seen_version.clone(),
            hotkey: String::new(),
            snooze_offered: snooze_is_offered(&stored),
            item,
        },
    );

    if plan.title.is_empty() {
        return false;
    }

    let mut state = fresh_state(owner, Kind::Letter);

    state.letter = Some(letter);
    state.news = match letter {
        Letter::News(id) => Some(id),
        _ => None,
    };
    state.link = item.map(|item| item.link.clone()).unwrap_or_default();
    state.plan = plan;

    open_window(owner, state, activate).is_some()
}

/// **Shows «От автора»** — FR-103.
pub fn open_author(owner: HWND) {
    let stored = state_now();
    let mut state = fresh_state(owner, Kind::Author);

    state.author = author_view(
        &stored,
        today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day")),
        &version_string(),
        FeedView::EMPTY,
    );

    open_window(owner, state, true);
}

/// **Shows «Последние письма»** — FR-101. Built in stage А and filled in stage Б.
pub fn open_list(owner: HWND) {
    let stored = state_now();
    let version = version_string();
    let mut state = fresh_state(owner, Kind::List);

    state.entries = with_feed(|feed| list_entries(&stored, &version, feed));

    open_window(owner, state, true);
}

thread_local! {
    /// The letter the balloon announced and that nobody has opened yet — FR-101.
    ///
    /// Cleared when the letter is shown, however it is shown: by the click on the balloon, or
    /// by itself at the next quiet moment.
    static ANNOUNCED: RefCell<Option<Letter>> = const { RefCell::new(None) };
}

/// **One tick of the schedule of FR-101** — the whole of what the letters do on their own.
///
/// Called by the tray's clock: ninety seconds after the start (NFR-08) and once an hour after
/// that (NFR-10). Everything it needs is a pure function away; what is impure here is the day,
/// the quiet moment and the window.
///
/// The order matters and is FR-101's:
///
/// 1. the day this installation started counting is written if it never was;
/// 2. `due` is asked what is due — and it is asked with **the quiet moment**, so a person in
///    the middle of typing is not interrupted;
/// 3. a letter that is due is **announced** first (every one but «Привет», which FR-101 shows
///    at once), and the window itself opens at the next tick if the balloon went unnoticed.
pub fn tick(owner: HWND) {
    let Some(today) = today() else {
        return;
    };

    let version = version_string();
    let mut stored = state_now();

    if initialise(&mut stored, today, &version) {
        let fresh = stored.clone();

        update_state(move |state| *state = fresh);
    }

    let announced = ANNOUNCED.with_borrow(|announced| *announced);
    let quiet = moment_is_quiet();

    let due = with_feed(|feed| due(&stored, today, &version, feed, quiet));

    let Some(letter) = due else {
        return;
    };

    // A letter that was announced at an earlier tick and not opened is **shown** now: the
    // balloon was its knock and it went unanswered, and FR-101 says the window then opens by
    // itself in a quiet moment, without the focus.
    if announced == Some(letter) || letter == Letter::Welcome {
        if show(owner, letter, today, &version, false) {
            ANNOUNCED.with_borrow_mut(|announced| *announced = None);
        }

        return;
    }

    // The first knock: the balloon. Nothing is recorded as shown — the letter is due until it
    // is actually on the screen.
    announce(letter, &version);
    ANNOUNCED.with_borrow_mut(|slot| *slot = Some(letter));
}

/// Opens the letter the balloon announced — FR-101, the click on the balloon.
///
/// The person asked for it, so the window comes up with the focus.
pub fn open_announced(owner: HWND) {
    let Some(letter) = ANNOUNCED.with_borrow(|announced| *announced) else {
        return;
    };

    let Some(today) = today() else {
        return;
    };

    if show(owner, letter, today, &version_string(), true) {
        ANNOUNCED.with_borrow_mut(|announced| *announced = None);
    }
}

/// Shows one letter and records that it was shown — the one place [`after_shown`] is called
/// from, so that «shown» and «on the screen» cannot come apart.
fn show(owner: HWND, letter: Letter, today: Date, version: &str, activate: bool) -> bool {
    let item = with_feed(|feed| match letter {
        Letter::Update => feed.update.cloned(),
        Letter::News(id) => feed.news.iter().find(|item| item.id == id).cloned(),
        _ => None,
    });

    // The version an «Обновление» records is the one it announced, not the one running.
    let recorded = match letter {
        Letter::Update => item
            .as_ref()
            .and_then(|item| item.version.clone())
            .unwrap_or_else(|| version.to_owned()),
        _ => version.to_owned(),
    };

    if !show_letter(owner, letter, item.as_ref(), activate) {
        return false;
    }

    update_state(|state| after_shown(state, letter, today, &recorded));
    crate::tray::refresh_unread_mark();

    true
}

/// Knocks once with the balloon of the icon — FR-101, every letter but «Привет».
fn announce(letter: Letter, version: &str) {
    use crate::settings::{
        IDS_TOAST_NEWS, IDS_TOAST_THANKS, IDS_TOAST_TITLE, IDS_TOAST_UPDATE,
        IDS_TOAST_UPDATE_TITLE, format_text, text,
    };

    let (title, body) = match letter {
        // «Привет» is never announced: FR-101 shows it at once, and this function is not
        // reached for it.
        Letter::Welcome => return,
        Letter::Thanks => (text(IDS_TOAST_TITLE), text(IDS_TOAST_THANKS)),
        Letter::News(_) => (text(IDS_TOAST_TITLE), text(IDS_TOAST_NEWS)),
        Letter::Update => {
            let newer = with_feed(|feed| {
                update_is_pending(version, feed)
                    .and_then(|item| item.version.clone())
                    .unwrap_or_default()
            });

            (
                format_text(IDS_TOAST_UPDATE_TITLE, &[&newer]),
                text(IDS_TOAST_UPDATE),
            )
        }
        // «Что нового» is about the version the person has just installed, and the balloon
        // says so with the same two lines the update uses — the letter behind it differs, the
        // knock does not.
        Letter::WhatsNew => (
            format_text(IDS_TOAST_UPDATE_TITLE, &[version]),
            text(IDS_TOAST_UPDATE),
        ),
    };

    crate::tray::announce_letter(&title, &body);
}

/// FR-94, решение 99.1 — the interface language changed while one of these windows was open.
///
/// The two mechanisms of Э31, and the choice between them is `settings::language_switch`: the
/// direction of the script is the same, so the window is **refilled in place** with the very
/// body that fills it at creation; the direction changed, so it is **rebuilt** — a window is
/// born mirrored and cannot become mirrored (ИТОГ-Э30 §9.2).
///
/// ⚠ **These windows have no background cache**, which is the one thing that made the settings
/// window's refill hard (ИТОГ-Э31 §8.1): `on_erase` draws the panels afresh out of the controls
/// every time it is called, so an invalidation is the whole of what a refill needs here. The
/// lesson is not thereby ignored — it is the reason this window was built without such a cache.
pub fn language_changed(owner: HWND) {
    let open: Vec<(Kind, HWND)> = OPEN.with_borrow(|open| open.clone());

    for (_, hwnd) in open {
        // SAFETY: `hwnd` is a window of this module that has not been destroyed.
        let previous = unsafe { with_state(hwnd, |state| state.language) };

        let Some(previous) = previous else {
            continue;
        };

        match settings::language_switch(previous, settings::ui_language()) {
            settings::LanguageSwitch::Unchanged => {}

            settings::LanguageSwitch::Relabel => {
                // SAFETY: as above.
                unsafe {
                    with_state(hwnd, |state| {
                        state.language = settings::ui_language();
                        refresh_plan(state);
                        fill_window(hwnd, state);
                        layout_window(hwnd, state);
                    })
                };

                settings::repaint_whole_window(hwnd);
            }

            settings::LanguageSwitch::Reopen => {
                // SAFETY: as above.
                let letter = unsafe { with_state(hwnd, |state| (state.letter, state.kind)) };

                close_window(hwnd);

                match letter {
                    Some((Some(letter), Kind::Letter)) => {
                        show_letter(owner, letter, None, false);
                    }
                    Some((_, Kind::Author)) => open_author(owner),
                    Some((_, Kind::List)) => open_list(owner),
                    _ => {}
                }
            }
        }
    }
}

/// Builds this window's description again in the locale now in force — the half of a refill
/// that is not about pixels.
fn refresh_plan(state: &mut WindowState) {
    let stored = state_now();
    let version = version_string();

    match state.kind {
        Kind::Letter => {
            if let Some(letter) = state.letter {
                state.plan = plan_for(
                    letter,
                    &PlanContext {
                        version,
                        previous_version: stored.last_seen_version.clone(),
                        hotkey: String::new(),
                        snooze_offered: snooze_is_offered(&stored),
                        item: None,
                    },
                );
            }
        }
        Kind::Author => {
            state.author = author_view(
                &stored,
                today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day")),
                &version,
                FeedView::EMPTY,
            );
        }
        Kind::List => {}
    }
}
