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
