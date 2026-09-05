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
/// * `version` — this build's version as `VERSIONINFO` spells it, `"0.41.0"`.
/// * `feed` — what the last successful feed read left; [`FeedView::EMPTY`] until one succeeds.
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
/// them: `0.41` and `0.41.0` are the same version. Neither case can arise from a feed the
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
/// The wizard «Написать автору» — FR-104.
pub const IDD_WIZARD: u16 = 205;

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

// Controls of `IDD_WIZARD`, mirrored from `app.rc` — FR-104.
const IDC_WZ_STEP: i32 = 1300;
const IDC_WZ_PROGRESS: i32 = 1301;
const IDC_WZ_TITLE: i32 = 1302;
const IDC_WZ_NOTE: i32 = 1303;
const IDC_WZ_CARD_1: i32 = 1304;
const IDC_WZ_CARD_T1: i32 = 1307;
const IDC_WZ_CARD_S1: i32 = 1310;
const IDC_WZ_PROGRAM_LABEL: i32 = 1313;
const IDC_WZ_PROGRAM: i32 = 1314;
const IDC_WZ_CAPTURE: i32 = 1315;
const IDC_WZ_CAPTURE_NOTE: i32 = 1316;
const IDC_WZ_FIELD_1: i32 = 1317;
const IDC_WZ_FIELD_2_SUB: i32 = 1320;
const IDC_WZ_TYPED_LABEL: i32 = 1321;
const IDC_WZ_LAYOUT: i32 = 1322;
const IDC_WZ_PRESSED_LABEL: i32 = 1323;
const IDC_WZ_CHIP: i32 = 1324;
const IDC_WZ_EXPECTED_LABEL: i32 = 1325;
const IDC_WZ_EXPECTED: i32 = 1326;
const IDC_WZ_GOT_LABEL: i32 = 1327;
const IDC_WZ_GOT: i32 = 1328;
const IDC_WZ_REPEAT_LABEL: i32 = 1329;
const IDC_WZ_REPEAT_1: i32 = 1330;
const IDC_WZ_IDEA_LABEL: i32 = 1333;
const IDC_WZ_IDEA: i32 = 1334;
const IDC_WZ_HELPS_LABEL: i32 = 1335;
const IDC_WZ_HELPS: i32 = 1336;
const IDC_WZ_ATTACH_1: i32 = 1337;
const IDC_WZ_ATTACH_V1: i32 = 1341;
const IDC_WZ_ATTACH_FOOT: i32 = 1345;
const IDC_WZ_PREVIEW: i32 = 1346;
const IDC_WZ_COPY: i32 = 1347;
const IDC_WZ_SAVE: i32 = 1348;
const IDC_WZ_STATUS: i32 = 1349;
const IDC_WZ_CANCEL: i32 = 1350;
const IDC_WZ_BACK: i32 = 1351;
const IDC_WZ_NEXT: i32 = 1352;
const IDC_WZ_THANKS: i32 = 1353;
const IDC_WZ_CHANNEL: i32 = 1354;

/// Every owner-drawn **static** of the wizard — the gate of `WM_DRAWITEM`, and it is written
/// out in full for the reason [`LETTER_LABELS`] carries in its own comment: a run written short
/// is a row that never gets painted, and the screenshot is what finds it.
const WIZARD_LABELS: [i32; 23] = [
    IDC_WZ_STEP,
    IDC_WZ_PROGRESS,
    IDC_WZ_TITLE,
    IDC_WZ_NOTE,
    IDC_WZ_CARD_T1,
    IDC_WZ_CARD_T1 + 1,
    IDC_WZ_CARD_T1 + 2,
    IDC_WZ_CARD_S1,
    IDC_WZ_CARD_S1 + 1,
    IDC_WZ_CARD_S1 + 2,
    IDC_WZ_PROGRAM_LABEL,
    IDC_WZ_CAPTURE_NOTE,
    IDC_WZ_FIELD_2_SUB,
    IDC_WZ_TYPED_LABEL,
    IDC_WZ_PRESSED_LABEL,
    IDC_WZ_CHIP,
    IDC_WZ_EXPECTED_LABEL,
    IDC_WZ_GOT_LABEL,
    IDC_WZ_REPEAT_LABEL,
    IDC_WZ_IDEA_LABEL,
    IDC_WZ_HELPS_LABEL,
    IDC_WZ_STATUS,
    IDC_WZ_THANKS,
];

/// The four labels that stand beside the four boxes of «Что приложить?» — a run, and the
/// footnote under them.
const WIZARD_ATTACH_LABELS: [i32; 5] = [
    IDC_WZ_ATTACH_V1,
    IDC_WZ_ATTACH_V1 + 1,
    IDC_WZ_ATTACH_V1 + 2,
    IDC_WZ_ATTACH_V1 + 3,
    IDC_WZ_ATTACH_FOOT,
];

/// Every owner-drawn **button** of the wizard: the three cards, the two radio runs, the four
/// boxes and the five push buttons.
const WIZARD_BUTTONS: [i32; 20] = [
    IDC_WZ_CHANNEL,
    IDC_WZ_CARD_1,
    IDC_WZ_CARD_1 + 1,
    IDC_WZ_CARD_1 + 2,
    IDC_WZ_CAPTURE,
    IDC_WZ_FIELD_1,
    IDC_WZ_FIELD_1 + 1,
    IDC_WZ_FIELD_1 + 2,
    IDC_WZ_REPEAT_1,
    IDC_WZ_REPEAT_1 + 1,
    IDC_WZ_REPEAT_1 + 2,
    IDC_WZ_ATTACH_1,
    IDC_WZ_ATTACH_1 + 1,
    IDC_WZ_ATTACH_1 + 2,
    IDC_WZ_ATTACH_1 + 3,
    IDC_WZ_COPY,
    IDC_WZ_SAVE,
    IDC_WZ_CANCEL,
    IDC_WZ_BACK,
    IDC_WZ_NEXT,
];

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
        // The two lines of every card of the wizard stand **on** that card, and the card is
        // drawn with the panel brush — see `draw_card`, which never changes its fill for
        // exactly this reason.
        || (IDC_WZ_CARD_T1..IDC_WZ_CARD_T1 + 3).contains(&control)
        || (IDC_WZ_CARD_S1..IDC_WZ_CARD_S1 + 3).contains(&control)
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
        IDS_HELLO_TITLE, IDS_LETTER_CAPTION, IDS_NEWS_DOWNLOAD, IDS_NEWS_FOOT, IDS_NEWS_LATER,
        IDS_NEWS_LETTER_TITLE, IDS_NEWS_OPEN_LINK, IDS_NEWS_READ_BUTTON, IDS_SUPPORT_OPEN,
        IDS_SUPPORT_PANEL, IDS_SUPPORT_TEXT, IDS_THANKS_FOOT, IDS_THANKS_LEAD, IDS_THANKS_PARA,
        IDS_THANKS_SNOOZE, IDS_THANKS_TITLE, IDS_UPDATE_FOOT, IDS_UPDATE_HOW, IDS_UPDATE_STEP_1,
        IDS_UPDATE_STEP_2, IDS_UPDATE_STEP_3, IDS_UPDATE_SUB, IDS_WHATSNEW_1, IDS_WHATSNEW_2,
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

            let released = item.date.map(format_date).unwrap_or_default();

            if is_update {
                // «Обновление» — FR-101 and the mock-up: the author's own words about what
                // changed stand as the paragraph, and the panel under them is the program's:
                // three steps, in words a person who has never installed anything can follow.
                //
                // The version in the heading is **the entry's own**: the author writes the
                // heading, and the number in it is the one being announced. What the program
                // adds underneath is the version running here and the day of the release.
                let announced = item.version.clone().unwrap_or_default();

                return LetterPlan {
                    caption,
                    title: item.title.clone(),
                    subtitle: format_text(IDS_UPDATE_SUB, &[&context.version, &released]),
                    para: item.text.clone(),
                    panel: text(IDS_UPDATE_HOW),
                    rows: vec![
                        Row {
                            marker: "1".to_owned(),
                            text: text(IDS_UPDATE_STEP_1),
                        },
                        Row {
                            marker: "2".to_owned(),
                            text: text(IDS_UPDATE_STEP_2),
                        },
                        Row {
                            marker: "3".to_owned(),
                            text: text(IDS_UPDATE_STEP_3),
                        },
                    ],
                    left: Some(Button::live(text(IDS_CLOSE), Action::Close)),
                    accent: Some(Button::link(
                        text(IDS_NEWS_DOWNLOAD),
                        Action::OpenDownload,
                        &item.link,
                    )),
                    foot: format_text(IDS_UPDATE_FOOT, &[&announced]),
                    ..LetterPlan::default()
                };
            }

            // «Новость» — the window says whose news it is, the panel says which news. The
            // entry's own heading goes **inside** the panel and not into the window's title:
            // the title belongs to the program, and a heading out of the feed in that place
            // would let the author write anything at all where the program speaks.
            LetterPlan {
                caption,
                title: text(IDS_NEWS_LETTER_TITLE),
                subtitle: released,
                panel_title: item.title.clone(),
                panel_text: item.text.clone(),
                panel_buttons: if item.link.is_empty() {
                    Vec::new()
                } else {
                    vec![Button::link(
                        text(IDS_NEWS_OPEN_LINK),
                        Action::OpenLink,
                        &item.link,
                    )]
                },
                // FR-101: closing is postponing, and the button says so. «Закрыть» here would
                // promise an end the letter does not give — it comes back in a week.
                left: Some(Button::live(text(IDS_NEWS_LATER), Action::Close)),
                accent: Some(Button::live(text(IDS_NEWS_READ_BUTTON), Action::MarkRead)),
                foot: text(IDS_NEWS_FOOT),
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
    DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED, ODT_BUTTON,
    ODT_COMBOBOX, ODT_STATIC,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    DM_SETDEFID, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDCANCEL,
    KillTimer, MSG, MapDialogRect, PostMessageW, SET_WINDOW_POS_FLAGS, SW_HIDE, SW_SHOWNOACTIVATE,
    SW_SHOWNORMAL, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOREDRAW, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG,
    WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND,
    WM_INITDIALOG, WM_MEASUREITEM, WM_NCDESTROY, WM_TIMER,
};
use windows::core::{Error as WinError, PCWSTR, w};

use crate::settings::{self, Language};
use crate::theme::{self, StaticColorRole, ThemeSetting};
// Общий слой элементов окон — задача Т-45-2, решение 107.1. Мастер и окна писем берут элементы
// здесь: в этом и состоит принцип пользователя «один механизм на все окна».
use crate::widgets;

/// The identifier of the timer that drives the demonstration of «Привет».
///
/// NFR-10: it is set on `WM_INITDIALOG` of a letter that has a demonstration and killed on
/// `WM_DESTROY`, so **nothing ticks while no such window is open** — which is the whole of the
/// requirement's «в покое ничего не делает».
const DEMO_TIMER: usize = 1;

/// The identifier of the timer that counts the five seconds of «Взять из активного окна».
///
/// NFR-10 again: it exists only between the press of that button and the capture, and is
/// killed on the way out of either. A second timer of this program, and the last.
const CAPTURE_TIMER: usize = 2;

/// How many seconds the wizard waits before it looks at the foreground window — FR-104.
const CAPTURE_SECONDS: u32 = 5;

/// Which of the five windows this is. One of each at a time; asking for one that is already up
/// raises it instead of making a second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A letter — the window of `IDD_LETTER`, whichever of the five it is showing.
    Letter,
    /// «Последние письма» — `IDD_LETTERS_LIST`.
    List,
    /// «От автора» — `IDD_AUTHOR`.
    Author,
    /// The wizard «Написать автору» — `IDD_WIZARD`, FR-104.
    ///
    /// ⛔ There used to be a fourth kind here — `Thanks`, the window the wizard left behind
    /// after «Готово». Задача Т-33-3 (решение 103.3) made the thanks the wizard's own last
    /// step, [`Step::Done`], so there is no second window to be a kind of.
    Wizard,
}

/// One step of the wizard — FR-104.
///
/// Which of them a person walks through depends on the first answer: five for a trouble and
/// three for an idea, and the two roads share the first step and the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// «Что случилось?» — the three cards.
    What,
    /// «Где это случилось?» — the program and the kind of field.
    Where,
    /// «Что вы делали?» — the layout, the key, what was expected and what came out.
    Did,
    /// «Опишите идею» — the short road's only middle step.
    Idea,
    /// «Что приложить?» — the four boxes.
    Attach,
    /// «Проверьте и отправьте» — the whole text, and the two ways out.
    Preview,
    /// «Готово» — the thanks, **in this same window** (задача Т-33-3, решение 103.3).
    ///
    /// Not a member of [`Wizard::road`] and deliberately so: the road is what the progress bar
    /// counts and what «Шаг N из M» says, and the thanks is neither a step of the appeal nor a
    /// number in that count — it is where the wizard ends. [`Wizard::done`] is what says the
    /// window is on it.
    Done,
}

/// **The state of the wizard window** — what the person has filled in and where they are.
///
/// Everything the appeal is built from is in [`Wizard::draft`], and everything the machine
/// contributed is in [`Wizard::facts`], read once when the window opened. The text of the
/// appeal is built from those two by [`report::build`] on the way into the last step, and from
/// that moment the **field** is the truth: a person may edit it, and what is copied or saved is
/// what the field holds.
pub struct Wizard {
    /// What the person filled in.
    draft: report::Draft,
    /// What the machine said about itself when the window opened.
    facts: report::Facts,
    /// Where in [`Wizard::road`] they are.
    step: usize,
    /// Seconds left of the capture count-down; zero means it is not running.
    countdown: u32,
    /// The line under the two buttons of the last step — what the last press did.
    status: String,
    /// The layout names of the combo, in the order the combo holds them.
    layouts: Vec<String>,
    /// The appeal as the field holds it — the truth from the last step onwards.
    text: String,
    /// Whether «Готово» has been pressed and the window is showing the thanks — задача Т-33-3.
    done: bool,
}

impl Wizard {
    /// The road this appeal walks — FR-104: five steps for a trouble, three for an idea.
    fn road(&self) -> &'static [Step] {
        if self.draft.trouble.is_idea() {
            &[Step::What, Step::Idea, Step::Preview]
        } else {
            &[
                Step::What,
                Step::Where,
                Step::Did,
                Step::Attach,
                Step::Preview,
            ]
        }
    }

    /// Which step is on the screen.
    fn current(&self) -> Step {
        if self.done {
            return Step::Done;
        }

        let road = self.road();

        road[self.step.min(road.len() - 1)]
    }

    /// Whether this is the last step — the one whose button says «Готово».
    fn is_last(&self) -> bool {
        self.step + 1 >= self.road().len()
    }
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
    /// What the wizard is holding — `None` for every other kind. Boxed because it is the one
    /// large member and four kinds out of five never fill it.
    wizard: Option<Box<Wizard>>,
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
            // The window of thanks is a letter in everything but its name — the same template
            // and the same accented button.
            Kind::Letter => IDC_LETTER_ACCENT,
            Kind::List => IDC_LETTERS_CLOSE,
            Kind::Author => IDC_AUTHOR_CLOSE,
            // «Далее» — and on the last step it says «Готово». Enter presses it on every step,
            // which is what a wizard is for.
            Kind::Wizard => IDC_WZ_NEXT,
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
    /// **Зазор между текстом подписи и кромкой рамки поля** — решение 107.4, задача Т-45-3.
    ///
    /// ⭐ **Число названо пользователем 2026-09-04: «10px».** Он выбирал его по живому макету с
    /// настоящими цветами и настоящей геометрией продукта при 96 DPI, то есть назвал **десять
    /// ЭКРАННЫХ пикселей**. Сверено с окном настроек, где тот же зазор вышел 9 px у самой
    /// тесной пары («Язык интерфейса:» → комбо) и 15 px у «Клавиши:»; разница там не замысел, а
    /// остаток — подпись стоит в прямоугольнике постоянной ширины из шаблона. Три числа не
    /// сошлись, поэтому выбор был вынесен пользователю, а не сделан исполнителем.
    ///
    /// ⚠ **Четырнадцать, а не десять, и это не описка.** Числа этой программы едут к пикселям
    /// экрана через `theme::scaled`, а та переводит длины **макета**, нарисованного при
    /// 134,4 DPI (`theme::MOCKUP_DPI_TENTHS`). `scaled(10, 96)` дало бы **7 px** — замерено на
    /// живом продукте, — а нужно 10. В мерке макета десять экранных пикселей и есть
    /// 10 × 134,4 / 96 = 14: `scaled(14, 96) = 10`, и при других DPI зазор растёт вместе со
    /// всем остальным.
    ///
    /// ⚠ Единица диалога тут не годится вовсе: по горизонтали она равна 1,75 px, и ни одно
    /// целое их число не даёт десяти — пять дают 9, шесть 11.
    pub(super) const LABEL_TO_FIELD: i32 = 14;
    /// The air between two blocks.
    pub(super) const GAP: i32 = 8;
    /// The inset of everything that stands on a panel.
    pub(super) const PANEL_PAD: i32 = 7;
    /// How far below the top of a panel its contents start — under the caption.
    pub(super) const PANEL_HEAD: i32 = 20;
    /// The height of a push button.
    pub(super) const BUTTON: i32 = 14;
    /// The height of the **box** of a single-line field — задача Т-33а-2.
    ///
    /// ⭐ **Ровно [`BUTTON`], и выведено из него, а не написано числом.** В макете поле и кнопка
    /// одной высоты (30 px при 96 DPI), и поле «Программа:» стоит вровень с кнопкой рядом. Пока
    /// коробку рисовала константа окна настроек — `settings::FIELD_BOX_DLU`, 12 DLU, — поле
    /// выходило на шесть пикселей ниже кнопки: 19 против 25 (`красное-до-2-высоты.log`), и это
    /// то самое «строки ввода как будто стали ниже», что увидел глаз на 0.43.0. Число, выведенное
    /// из соседа, не может с ним разойтись при следующей правке.
    pub(super) const FIELD_BOX: i32 = BUTTON;
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
            SWP_NOZORDER | SWP_NOACTIVATE | quiet_flag(),
        )
    };

    // ⚠ **A panel is `NOT WS_VISIBLE` in the template and must stay that way.** It carries the
    // caption and the rectangle, and the window's own `WM_ERASEBKGND` draws the block from
    // them; shown, it would be an owner-drawn *button* standing over its own caption — which
    // is exactly what the stand photographed before this line named all five panels instead of
    // one (`scratchpad-Э32\снимки`, the first «От автора»).
    if !PANELS.contains(&control) {
        if quiet_layout() {
            // Показать БЕЗ перерисовки — см. [`QUIET_LAYOUT`]. `ShowWindow` такого флага не
            // знает, а `SetWindowPos` знает, и это единственная причина второго вызова.
            //
            // SAFETY: as above.
            let _ = unsafe {
                SetWindowPos(
                    child,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOZORDER
                        | SWP_NOACTIVATE
                        | SWP_NOMOVE
                        | SWP_NOSIZE
                        | SWP_SHOWWINDOW
                        | SWP_NOREDRAW,
                )
            };
        } else {
            // SAFETY: `child` is the live control; the call takes no pointer.
            let _ = unsafe { ShowWindow(child, SW_SHOWNOACTIVATE) };
        }
    }
}

thread_local! {
    /// Идёт ли сейчас **тихая** перестановка — задача Т-33-8.
    ///
    /// ⛔ Переключение шага мастера двигает, показывает и прячет несколько десятков контролов, и
    /// каждый `SetWindowPos`/`ShowWindow` сам просит перерисовать то, что из-под него открылось.
    /// `WM_SETREDRAW` у окна-родителя эти перерисовки НЕ гасит — у ребёнка свой флаг, — и на
    /// экран попадали кадры, которые не были ни прежним шагом, ни новым: замерено 24 и 34 кадра
    /// из 250 при худшем расхождении около 31 000 пикселей из 251 488 уже ПОСЛЕ `WM_SETREDRAW`
    /// и `WS_CLIPCHILDREN` (`scratchpad-Э33\моргание-шагов-после.log`).
    ///
    /// Пока флаг поднят, [`place`] и [`hide`] добавляют `SWP_NOREDRAW`: перестановка идёт
    /// вслепую, и один `RedrawWindow` в конце [`wizard_refresh`] показывает готовый шаг целиком.
    /// Флаг поднимает и опускает **только** `wizard_refresh`, вокруг одного вызова вёрстки; все
    /// прочие вёрстки этих окон идут как прежде.
    static QUIET_LAYOUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Идёт ли тихая перестановка — см. [`QUIET_LAYOUT`].
fn quiet_layout() -> bool {
    QUIET_LAYOUT.with(std::cell::Cell::get)
}

/// `SWP_NOREDRAW` во время тихой перестановки и ничего в остальное время.
fn quiet_flag() -> SET_WINDOW_POS_FLAGS {
    if quiet_layout() {
        SWP_NOREDRAW
    } else {
        SET_WINDOW_POS_FLAGS(0)
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
///
/// ⚠ **A panel is hidden by collapsing its rectangle and not by `ShowWindow`**, because it is
/// never shown to begin with: what draws the block is `WM_ERASEBKGND`, out of the panel's
/// *rectangle*, and a panel left at the size the template gave it goes on being drawn. The
/// window of thanks is where this showed — an empty frame in the middle of four lines of text
/// (`scratchpad-Э32\живое-В\мастер-6-спасибо.png`, the frame before this rule).
fn hide(hwnd: HWND, control: i32) {
    if PANELS.contains(&control) {
        // SAFETY: the control is a live child of this dialog; the five numbers are plain
        // values and no pointer is passed.
        if let Ok(panel) =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) }
        {
            let _ = unsafe {
                SetWindowPos(
                    panel,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOZORDER | SWP_NOACTIVATE | quiet_flag(),
                )
            };
        }
    }

    let Ok(child) =
        (unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) })
    else {
        return;
    };

    if quiet_layout() {
        // Спрятать БЕЗ перерисовки — см. [`QUIET_LAYOUT`]: иначе родитель тут же стирает то,
        // что из-под контрола открылось, и этот кадр видит глаз.
        //
        // SAFETY: `child` is the live control; the call takes no pointer.
        let _ = unsafe {
            SetWindowPos(
                child,
                None,
                0,
                0,
                0,
                0,
                SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_HIDEWINDOW
                    | SWP_NOREDRAW,
            )
        };

        return;
    }

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

    // ⛔ **The one letter whose height its content decided is gone — задача Т-33-3.** It was
    // the window of thanks, and Т-32-10 had to give it a height of its own because four lines
    // of text in a window built for a letter left three hundred pixels of empty ground under
    // them. Решение 103.3 made the thanks the wizard's last step instead, in the wizard's own
    // window and at the wizard's own size, and every letter that is left keeps the height of
    // the template — they fill it.
    let _ = bottom;
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
        // The wizard's quiet lines: the step counter, every explanation and the status line.
        // Everything that asks a question or names a thing stays a plain label.
        IDC_WZ_STEP | IDC_WZ_NOTE | IDC_WZ_CAPTURE_NOTE | IDC_WZ_FIELD_2_SUB
        | IDC_WZ_ATTACH_FOOT | IDC_WZ_STATUS => StaticColorRole::Muted,
        control if (IDC_WZ_CARD_S1..IDC_WZ_CARD_S1 + 3).contains(&control) => {
            StaticColorRole::Muted
        }
        control if (IDC_WZ_ATTACH_V1..IDC_WZ_ATTACH_V1 + 4).contains(&control) => {
            StaticColorRole::Muted
        }
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
        // The wizard: the heading of a step is set like the heading of a letter, and every
        // sentence that wraps is set in the reading face at the reading pitch.
        IDC_WZ_TITLE => (faces.name, None),
        IDC_WZ_NOTE | IDC_WZ_CAPTURE_NOTE | IDC_WZ_FIELD_2_SUB | IDC_WZ_ATTACH_FOOT
        | IDC_WZ_CHIP => (faces.body, Some(faces.body_pitch())),
        control if (IDC_WZ_CARD_S1..IDC_WZ_CARD_S1 + 3).contains(&control) => {
            (faces.body, Some(faces.body_pitch()))
        }
        control if (IDC_WZ_ATTACH_V1..IDC_WZ_ATTACH_V1 + 4).contains(&control) => {
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
                brushes.field_bg(),
            ))
        })
    };

    let Some(Some((ground, line, panel, border, caption, face, kind, field))) = choice else {
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
        // ⚠ The wizard has **no panel**: its steps stand on the window's own ground, and the
        // three cards of the first step are drawn as blocks by the button branch rather than
        // by the background. A panel here would be a block behind the fields.
        Kind::Wizard => &[],
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

        // A panel a letter does not use is collapsed to nothing by `hide`, and nothing is what
        // gets drawn for it — the second half of that rule, kept here so that neither half can
        // be removed on its own.
        if rect.right <= rect.left || rect.bottom <= rect.top {
            continue;
        }

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

    // ⛔ **Рамка поля — задача Т-33-7, находка глазом пользователя.** До неё поля мастера не
    // имели рамки вовсе: коробкой выглядела собственная заливка контрола, а он был высотой в
    // кнопку — и текст стоял по его ВЕРХНЕМУ краю, оставляя под собой одиннадцать пикселей
    // пустоты. Однострочный `EDIT` кладёт текст по верху своей клиентской области, и ни одно
    // документированное сообщение его оттуда не двигает (`EM_SETRECT` — для многострочных,
    // `EM_SETMARGINS` двигает бока). Задача T-12-3 решила это для окна настроек ровно так:
    // контролу — одна кегельная строка, а коробку рисует фон **вокруг него, по центру**, и
    // воздух ложится над текстом и под ним поровну. Здесь — то же самое тело и та же
    // `theme::field_frame_air`.
    if kind == Kind::Wizard {
        // ⛔ **Не `settings::FIELD_BOX_DLU` — задача Т-33а-2.** Т-33-7 взяла коробку у окна
        // настроек (12 DLU), а кнопка мастера — 14, и поле вышло на шесть пикселей ниже соседа.
        // Окно настроек при этом не трогается: там 12 DLU принято глазом в Э23/Э26.
        let box_height = Some(Metrics::of(hwnd).y(air::FIELD_BOX));
        let thickness = theme::scaled(theme::BORDER_THICKNESS, dpi).max(1);

        for (control, rect) in &rects {
            if !WIZARD_FIELDS.contains(control) && !WIZARD_AREAS.contains(control) {
                continue;
            }

            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }

            // ⛔ **Только видимые.** Окно настроек этой проверки не знает — там видно всё, — а у
            // мастера каждый шаг прячет чужие слоты, и прямоугольник из шаблона у них остаётся.
            // Без этой строки фон рисовал рамки полей ЧУЖИХ шагов: пустые белые коробки по
            // всему окну, и это увидел глаз на первом же снимке.
            if !is_shown(hwnd, *control) {
                continue;
            }

            // ⚠ Многострочное поле держит свои строки от верха клиентской области, и центровать
            // вокруг него нечего: воздух ему даёт `EM_SETRECT` внутри, а рамка стоит на одной
            // толщине — та же развилка, что у списков окна настроек (`FRAMED_LISTS`).
            let air = if WIZARD_AREAS.contains(control) {
                (thickness, thickness)
            } else {
                // Задача Т-46-5: параметра `OddPixel` больше нет — коробка равна заказанной
                // высоте у всех окон программы (решение 109.6). У мастера число не менялось:
                // он клал лишний пиксель вниз с задачи Т-33а-2, и это стало стандартом.
                widgets::field::air(box_height, rect.bottom - rect.top, thickness)
            };

            theme::paint_rounded(
                dc,
                &widgets::field::frame(*rect, air, thickness),
                theme::scaled(theme::CORNER_RADIUS, dpi),
                line,
                field,
                dpi,
            );
        }
    }

    // TRUE — the background is drawn; the manager must not erase over it.
    1
}

/// Whether a child of this window is on the screen right now.
fn is_shown(hwnd: HWND, control: i32) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetDlgItem, IsWindowVisible};

    // SAFETY: `hwnd` is the live window; the crate turns a missing control into an error, and
    // `IsWindowVisible` reads a flag of the window it is handed.
    unsafe { GetDlgItem(Some(hwnd), control) }
        .is_ok_and(|child| unsafe { IsWindowVisible(child) }.as_bool())
}

/// The three **single-line** fields of the wizard — the ones whose box the background draws
/// around a control one em box tall (задача Т-33-7).
const WIZARD_FIELDS: [i32; 3] = [IDC_WZ_PROGRAM, IDC_WZ_EXPECTED, IDC_WZ_GOT];

// ⚠ **`field_box_air` уехала в `widgets::field::air` — задача Т-45-2, решение 107.4.**
//
// Здесь она стояла с задачи Т-33а-2 и была ВТОРОЙ копией арифметики окна настроек: та
// (`theme::field_frame_air`) отвечала одним числом на обе стороны и на нечётном остатке теряла
// пиксель, эта клала его вниз — чтобы коробка вышла ровно вровень с кнопкой рядом. Теперь тело
// одно, и различие названо параметром `widgets::field::OddPixel`, а не двумя телами.

/// Places a single-line field so that its **box** stands centred in a row `row` pixels tall.
///
/// The control itself gets one em box — [`settings::DIALOG_FONT_HEIGHT_DLU`], exactly the line
/// its text needs — and the frame the background draws around it (see [`on_erase`]) puts the
/// leftover air above and below in equal halves. Giving the control the whole row instead is
/// what pinned the text to the top edge, which is what the eye found on 0.43.0.
///
/// The sides are handed to the control itself: `EM_SETMARGINS` is the documented way to inset
/// the text of an `EDIT`, and it is the same inset the settings window uses.
fn place_field(hwnd: HWND, metrics: Metrics, control: i32, x: i32, y: i32, width: i32, row: i32) {
    use windows::Win32::UI::Controls::EM_SETMARGINS;
    use windows::Win32::UI::WindowsAndMessaging::{EC_LEFTMARGIN, EC_RIGHTMARGIN};

    let line = metrics.y(settings::DIALOG_FONT_HEIGHT_DLU);

    place(hwnd, control, x, y + (row - line).max(0) / 2, width, line);

    let inset = metrics.x(settings::FIELD_TEXT_INSET_DLU).max(0);
    let margins = isize::try_from((inset as u32) | ((inset as u32) << 16)).unwrap_or(0);

    settings::send_to(
        hwnd,
        control,
        EM_SETMARGINS,
        usize::try_from(EC_LEFTMARGIN | EC_RIGHTMARGIN).unwrap_or(0),
        margins,
    );
}

/// Gives a **multi-line** field the air its text stands in — `EM_SETRECT`, the one message that
/// moves the formatting rectangle of an edit control, and the one that is documented for
/// multiline controls only.
///
/// Without it the words start in the very corner of the box, against the frame: the mock-up
/// insets them (`textarea.field { padding: 8px 10px }`), and this is that inset.
fn inset_area(hwnd: HWND, metrics: Metrics, control: i32) {
    use windows::Win32::UI::Controls::EM_SETRECT;

    let Ok(child) =
        (unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) })
    else {
        return;
    };

    let mut client = RECT::default();

    // SAFETY: `child` is the live control and `client` a live local the call fills.
    if unsafe { GetClientRect(child, &mut client) }.is_err() {
        return;
    }

    let side = metrics.x(settings::FIELD_TEXT_INSET_DLU).max(0);
    let top = metrics.y(air::TIGHT).max(0);

    let formatting = RECT {
        left: client.left + side,
        top: client.top + top,
        right: (client.right - side).max(client.left + side),
        bottom: (client.bottom - top).max(client.top + top),
    };

    // SAFETY: the pointer names a live local of this frame and the control only reads it for
    // the length of the send — the documented shape of `EM_SETRECT`.
    settings::send_to(
        hwnd,
        control,
        EM_SETRECT,
        0,
        std::ptr::from_ref(&formatting) as isize,
    );
}

/// The three **multi-line** fields of the wizard. Their box is the control's own rectangle plus
/// one thickness, and the air around their text comes from `EM_SETRECT` inside them.
const WIZARD_AREAS: [i32; 3] = [IDC_WZ_IDEA, IDC_WZ_HELPS, IDC_WZ_PREVIEW];

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
                // The six fields of the wizard and the list of its combo — a field's ground and
                // a field's ink, the same pair the settings window gives its own (FR-92а).
                WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => (
                    Some(palette.text),
                    Some(palette.field_bg),
                    brushes.field_bg(),
                ),
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

/// Draws the feed switch of FR-102 — the one check box of these three windows.
///
/// **Assembled from `theme`'s own primitives, not copied from the settings dialog**: the roles
/// come from `theme::glyph_color_roles` (the 2×2×2 table of FR-92а), the figure from
/// `theme::paint_rounded`, the tick from `theme::draw_check_mark` with the settings dialog's own
/// `GLYPH_CHECK_MARK`, and the caption from `theme::paint_label`. What is in this function is
/// **which** element is being drawn — which is exactly the line §6.2 draws between a window's
/// owner and the drawing library.
///
/// ⚠ The tick is drawn through `check_mark_points(…, mirrored)` — a polyline is turned round by
/// a mirrored window and `LAYOUT_BITMAPORIENTATIONPRESERVED` does not fix it (ИТОГ-Э30 §9.5).
///
/// # Safety
///
/// Called from [`on_draw_item`] with values copied out of the `WM_DRAWITEM` message.
unsafe fn draw_switch(
    dc: HDC,
    rect: RECT,
    state: &WindowState,
    disabled: bool,
    label: &str,
) -> isize {
    let (Some(brushes), Some(faces)) = (state.brushes.as_ref(), state.fonts.as_ref()) else {
        return 0;
    };

    let palette = state.palette;
    let dpi = theme::dc_dpi(dc);
    let checked = state.author.switch.unwrap_or(false);
    let colors = theme::glyph_color_roles(theme::GlyphKind::CheckBox, checked, disabled);

    // SAFETY: `dc` is the DC of the message and the brush belongs to this window's state.
    unsafe { FillRect(dc, &rect, brushes.panel_bg()) };

    // ⚠ Задача Т-45-2: та же клетка, что у окна настроек, и теперь одним телом
    // (`widgets::glyph::cell`). Всё остальное здесь у двух окон различается — см. доктекст
    // `widgets::glyph`.
    let cell = widgets::glyph::cell(rect, dpi);

    let fill = match colors.fill {
        theme::GlyphFillRole::FieldBg => brushes.field_bg(),
        theme::GlyphFillRole::AccentBg => brushes.accent_bg(),
    };

    let outline = match colors.frame {
        Some(theme::GlyphFrameRole::BoxBorder) => palette.box_border,
        // The one cell the accent fill covers whole is outlined in its own fill, so that
        // `RoundRect` — which draws frame and fill in one figure — shows no line at all.
        None => match colors.fill {
            theme::GlyphFillRole::FieldBg => palette.field_bg,
            theme::GlyphFillRole::AccentBg => palette.accent_bg,
        },
    };

    theme::paint_rounded(
        dc,
        &cell,
        theme::scaled(settings::GLYPH_CORNER_RADIUS, dpi),
        outline,
        fill,
        dpi,
    );

    if let Some(mark) = colors.mark {
        let ink = match mark {
            theme::GlyphMarkRole::AccentFg => palette.accent_fg,
            theme::GlyphMarkRole::AccentBg => palette.accent_bg,
            theme::GlyphMarkRole::BoxBorder => palette.box_border,
        };

        theme::draw_check_mark(dc, &cell, ink, settings::GLYPH_CHECK_MARK, dpi);
    }

    let text = RECT {
        left: cell.right + theme::scaled(settings::LIST_CHECK_TEXT_GAP, dpi),
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };

    let mut caption: Vec<u16> = label.encode_utf16().collect();

    // SAFETY: `dc` and `text` are live; the face belongs to this window's state.
    unsafe {
        theme::paint_label(
            dc,
            text,
            &mut caption,
            theme::LabelStyle {
                ground: brushes.panel_bg(),
                ink: match colors.text {
                    theme::GlyphTextRole::Text => palette.text,
                    theme::GlyphTextRole::TextMuted => palette.text_muted,
                },
                face: Some(faces.text),
                pitch: None,
                reading: theme::Reading::Native,
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

    // The one combo box of these windows — the layouts of «Что вы делали?». SEC-05: the type
    // **and** the identifier are checked before anything is drawn.
    if ctl_type == ODT_COMBOBOX {
        if control != IDC_WZ_LAYOUT {
            return 0;
        }

        // SAFETY: see the caller — `dc` and `rect` are the values of the message.
        return unsafe {
            with_state(hwnd, |state| {
                draw_combo_row(hwnd, dc, rect, state, item.itemID, item_state.0)
            })
        }
        .unwrap_or(0);
    }

    if ctl_type == ODT_STATIC {
        // SEC-05: a `WM_DRAWITEM` can be forged by any process of our integrity level, so the
        // identifier is checked against the list of statics **this** window really has before
        // anything is drawn — the gate the about window puts in front of its own four labels.
        // SAFETY: see the caller.
        let known = unsafe {
            with_state(hwnd, |state| {
                owner_drawn_labels(state.kind).contains(&control)
                    || owner_drawn_labels_more(state.kind).contains(&control)
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

        // Neither is the progress bar of the wizard: a run of steps behind and a run ahead.
        if control == IDC_WZ_PROGRESS {
            // SAFETY: see the caller.
            return unsafe { with_state(hwnd, |state| draw_progress(dc, rect, state)) }
                .unwrap_or(0);
        }

        // Nor the key chip, which is a figure and not a word.
        if control == IDC_WZ_CHIP {
            let key = settings::get_text(hwnd, IDC_WZ_CHIP);

            // SAFETY: see the caller.
            return unsafe { with_state(hwnd, |state| draw_chip(dc, rect, state, &key)) }
                .unwrap_or(0);
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

    // The three cards of the wizard's first step — blocks with two lines of text on them, and
    // the chosen one wears the accent frame.
    if (IDC_WZ_CARD_1..IDC_WZ_CARD_1 + 3).contains(&control) {
        // SAFETY: see the caller — `dc` and `rect` are the values of the message.
        return unsafe {
            with_state(hwnd, |state| {
                draw_card(hwnd, dc, rect, state, control, hot, focused)
            })
        }
        .unwrap_or(0);
    }

    // The radios and the boxes of the wizard — the same glyph the settings window draws, with
    // the wizard's own record for the answer to «is it checked».
    if let Some(kind) = wizard_glyph(control) {
        let label = settings::get_text(hwnd, control);

        // SAFETY: see the caller.
        return unsafe {
            with_state(hwnd, |state| {
                draw_wizard_glyph(dc, rect, state, kind, control, disabled, &label)
            })
        }
        .unwrap_or(0);
    }

    // The one control of these three windows that is not a push button: the switch of FR-102.
    if control == IDC_NEWS_SWITCH {
        // Read **before** the borrow, the discipline of every drawing of this file.
        let label = settings::get_text(hwnd, IDC_NEWS_SWITCH);

        // SAFETY: see the caller — `dc` and `rect` are the values of the message.
        return unsafe { with_state(hwnd, |state| draw_switch(dc, rect, state, disabled, &label)) }
            .unwrap_or(0);
    }

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

    // ⛔ **Один кадр вместо двух — задача Т-33-8.** Подпись кладёт заливку, потом слова, и, пока
    // это шло прямо в DC сообщения, между ними на экран попадал кадр с пустой землёй. На
    // переключении шага подписей перерисовывается сразу десяток, и это последняя доля того
    // моргания, которое видно глазом: карточки и глифы поверхность получили задачей Т-33-5,
    // кнопки — задачей T-15-1, а подписи оставались.
    //
    // SAFETY: `dc` — DC сообщения, живой на время посылки; буфер освобождает свои DC и растр,
    // когда кончится этот кадр.
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // A row that names the hotkey is laid word by word with a chip around the name — решение
    // 85 п. 1, the same body the help rows of «О программе» are drawn by.
    if caption.contains(theme::KEY_PLACEHOLDER) && !key.is_empty() {
        // SAFETY: `target` is the buffer of this frame or the DC of the message; every handle
        // is an object the state owns for longer than the call.
        let drawn = unsafe {
            theme::paint_chip_row(
                target,
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

        // SAFETY: `dc` is the DC of the message; the buffer is this frame's own.
        if let Some(buffer) = buffer {
            let _ = unsafe { buffer.blit(dc) };
        }

        return drawn;
    }

    let mut wide: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: `target` is the buffer of this frame or the DC of the message; `ground` and
    // `face` are objects the state owns for longer than the call.
    let drawn = unsafe {
        theme::paint_label_at_pitch(
            target,
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
    };

    // SAFETY: as above.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
    }

    drawn
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

            // FR-104: the closed face of the layout combo is drawn by this program and not by
            // the system — the very body the four combo boxes of the settings window are drawn
            // by, asked for through the seam of task Т-32-8 rather than copied (§6.2).
            //
            // ⭐ **Задача Т-45-3: через общий слой, и слой ставит ЗАКРЫТУЮ ВЫСОТУ сам.** До Э45
            // мастер брал у окна настроек подкласс, а высоты — нет, и `CB_SETITEMHEIGHT(−1)` не
            // звал вовсе. Замер на живом 0.44.0: строка выпадающего списка мастера **16 px**
            // против **26 px** у всех четырёх комбо окна настроек — это и есть «нестандартный
            // для моей программы размер» из слов пользователя. `widgets::combo::attach` делает
            // обе половины одним вызовом, и забыть вторую больше негде.
            if kind == Kind::Wizard {
                widgets::combo::attach(hwnd, IDC_WZ_LAYOUT);
            }

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
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLOREDIT
        | WM_CTLCOLORLISTBOX => unsafe { on_ctl_color(hwnd, message, wparam, lparam) },

        // SAFETY: the sender owns the struct `lparam` names for the length of the send, and
        // this procedure is inside that send.
        WM_DRAWITEM => unsafe { on_draw_item(hwnd, lparam) },

        // The height of one row of the layout combo — asked once, before the window is filled.
        //
        // SAFETY: as above.
        WM_MEASUREITEM => unsafe { on_measure_item(hwnd, lparam) },

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
                widgets::repaint::control(hwnd, IDC_LETTER_DEMO);
            }

            0
        }

        // One second of the capture count-down of FR-104 — the wizard's own timer, and the
        // only other one this program sets.
        WM_TIMER if wparam.0 == CAPTURE_TIMER => {
            wizard_tick(hwnd);
            0
        }

        WM_COMMAND => {
            let control = i32::from(settings::low_word(wparam.0));

            // ⭐ **Код уведомления читается — задача Т-46-2, решение 109.1.** До Э46 он не
            // читался вовсе, и глифы этих окон переключались на каждое уведомление очереди
            // настоящего щелчка: `BN_KILLFOCUS` соседке, `BN_SETFOCUS` и `BN_CLICKED` своей.
            let notification = settings::high_word(wparam.0);

            // Esc, which the dialog manager sends whether or not the window has the button.
            if control == IDCANCEL.0 {
                close_window(hwnd);
                return 0;
            }

            // The wizard answers for its own controls — FR-104. SAFETY: the state pointer is
            // live for the length of the window.
            let is_wizard = unsafe { with_state(hwnd, |state| state.kind) } == Some(Kind::Wizard);

            if is_wizard && wizard_command(hwnd, control, notification) {
                return 0;
            }

            // **FR-102: the switch writes the file at once.** Not on «Закрыть» and not on a
            // later save — a person who turns the feed off has turned it off, and a window
            // they close some other way must not undo that.
            //
            // ⛔ **Задача Т-46-2: и здесь фильтр слоя, по той же причине.** Выключатель ленты —
            // `BS_OWNERDRAW | BS_NOTIFY`, то есть шлёт ту же очередь из трёх уведомлений, что и
            // галки мастера. Без фильтра щелчок по нему с переходом фокуса переключал ленту
            // дважды — **и дважды писал файл**, потому что FR-102 пишет немедленно. Пикселей
            // это не меняет; замер находки 3 (`scratchpad-Э46\красное-галки-e45.log`) показал,
            // что болезнь была общей.
            if control == IDC_NEWS_SWITCH
                && widgets::glyph::answer(widgets::glyph::Kind::Check, notification)
                    == widgets::glyph::Answer::Toggle
            {
                let wanted = !state_now().feed;

                update_state(|state| state.feed = wanted);

                settings::send_to(
                    hwnd,
                    IDC_NEWS_SWITCH,
                    windows::Win32::UI::WindowsAndMessaging::BM_SETCHECK,
                    usize::from(wanted),
                    0,
                );
                widgets::repaint::control(hwnd, IDC_NEWS_SWITCH);

                return 0;
            }

            // Кнопки писем: подписанные действия делает щелчок и ничего кроме (Т-46-2).
            if widgets::glyph::answer(widgets::glyph::Kind::Button, notification)
                != widgets::glyph::Answer::Act
            {
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

                // The far half of the pair installed on `WM_INITDIALOG` — while the children
                // are still alive, which is what makes it exact.
                if kind == Kind::Wizard {
                    widgets::combo::detach(hwnd, IDC_WZ_LAYOUT);
                }
            }

            // SAFETY: `hwnd` is the live window; killing a timer that was never set answers
            // false and is harmless (NFR-13).
            let _ = unsafe { KillTimer(Some(hwnd), DEMO_TIMER) };
            // SAFETY: as above — the capture count-down may or may not be running.
            let _ = unsafe { KillTimer(Some(hwnd), CAPTURE_TIMER) };

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

                widgets::repaint::whole(hwnd);
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
        Kind::Wizard => &WIZARD_LABELS,
    }
}

/// The four value lines of «Что приложить?» and the footnote under them — the second half of
/// the wizard's statics, asked separately because the gate takes one slice.
fn owner_drawn_labels_more(kind: Kind) -> &'static [i32] {
    match kind {
        Kind::Wizard => &WIZARD_ATTACH_LABELS,
        _ => &[],
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
        Kind::Wizard => &WIZARD_BUTTONS,
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

/// A date as a person reads it — «14 октября» — in the language of the interface, **and always
/// in the Gregorian calendar** (FR-101, решение 103.1).
///
/// The month is named by the locale of `general.language`, so nothing has to be translated by
/// hand, and the shape is this file's own — the day, a space, the name of the month — so it
/// does not depend on what the user's Windows prefers.
///
/// # Why the name is asked of the calendar and not of `GetDateFormatEx`
///
/// `GetDateFormatEx` prints a date in the locale's **own** calendar, and for `ar` that is Umm
/// al-Qura: 2026-09-04 came out «22 ربيع الأول», a day of the Hijri year for what the release
/// notes call the fourth of September. A letter is dated by an event of this program, so the
/// number and the month have to be the ones the version was released on.
///
/// The two ways out were measured rather than believed (`scratchpad-Э33\календарь.log`):
///
/// * `DATE_USE_ALT_CALENDAR` — **refused**. It does not move `ar` at all (its alternative
///   calendar is Hijri, another lunar one), and it moves `he` into the Hebrew calendar
///   («כ"ב אלול») and `tr` into Umm al-Qura. It fixes nothing and breaks two.
/// * asking the **Gregorian** calendar for the name of the month by number — taken. With
///   `CAL_RETURN_GENITIVE_NAMES` it gives the very word `GetDateFormatEx` used to put after the
///   day: «сентября» and not «Сентябрь», «вересня» and not «вересень». Measured across all
///   fourteen locales on three days: the thirteen whose own calendar is already Gregorian print
///   the same string byte for byte, and `ar` becomes «4 سبتمبر».
///
/// ⚠ **The digits are European.** `GetDateFormatEx` substituted native digits when the locale
/// asked for them; a number written by `to_string` never does. On every one of the fourteen
/// `LOCALE_IDIGITSUBSTITUTION` is 1 («нет») or 0 («по контексту»), so nothing changes today —
/// and this line is here because a machine set otherwise would see Western digits where the old
/// road would have shown Arabic-Indic ones.
///
/// NFR-13: a refusal falls back to the ISO form the configuration file is written in, which is
/// readable everywhere and wrong nowhere.
pub fn format_date(date: Date) -> String {
    let locale = settings::wide(settings::ui_language().tag());

    match gregorian_month_name(&locale, date.month()) {
        Some(month) => format!("{} {month}", date.day()),
        None => date.to_string(),
    }
}

/// The name of a month of the **Gregorian** calendar in the named locale, in the form a date
/// puts it in.
///
/// The genitive form is asked for first and the plain one is the fallback: the modifier is
/// documented as «return the genitive forms of month names», and a locale that has no separate
/// genitive answers with the plain name anyway — but a locale that refuses the modifier
/// outright would otherwise leave the date with no month at all.
///
/// `None` when both refusals happen or the month is not a month, which is what sends
/// [`format_date`] to the ISO form.
fn gregorian_month_name(locale: &[u16], month: u32) -> Option<String> {
    use windows::Win32::Globalization::{
        CAL_GREGORIAN, CAL_RETURN_GENITIVE_NAMES, CAL_SMONTHNAME1, GetCalendarInfoEx,
    };

    if !(1..=12).contains(&month) {
        return None;
    }

    let name = CAL_SMONTHNAME1 + month - 1;

    for kind in [name | CAL_RETURN_GENITIVE_NAMES, name] {
        let mut buffer = [0u16; 64];

        // SAFETY: `locale` is a NUL-terminated buffer owned by the caller's frame, the buffer
        // is a live local whose length the call is told, and `PCWSTR::null()` is the documented
        // value of the reserved argument. `None` for the numeric answer is what asks for the
        // string one.
        let written = unsafe {
            GetCalendarInfoEx(
                PCWSTR(locale.as_ptr()),
                CAL_GREGORIAN,
                PCWSTR::null(),
                kind,
                Some(&mut buffer),
                None,
            )
        };

        if written <= 0 {
            continue;
        }

        let units = usize::try_from(written).unwrap_or(1).saturating_sub(1);

        if let Some(name) = buffer
            .get(..units)
            .and_then(|slice| String::from_utf16(slice).ok())
            && !name.is_empty()
        {
            return Some(name);
        }
    }

    None
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
        Kind::Wizard => fill_wizard(hwnd, state),
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
        Kind::Wizard => unsafe { layout_wizard(hwnd, state) },
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

    // The download button leads where the **feed entry** points, and that address is the
    // author's data rather than a constant of the build — so it is judged by the same rule as
    // the two above and by nothing else. An entry that points at the placeholder host (which
    // is what the site carries until it is published) leaves the button drawn and dead.
    enable(
        hwnd,
        IDC_NEWS_DOWNLOAD,
        view.download
            .as_deref()
            .is_some_and(|url| !links::is_placeholder(url)),
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

/// The entry a letter out of the feed is about, taken from the published feed by its own name.
///
/// The two feed letters carry their entry in the window state, and that copy is enough while
/// the window stands. It is **not** enough when the window has to be built again — a change of
/// interface language reopens it (Э31) — and a letter rebuilt without its entry would come
/// back empty. Everything else answers `None`: their words are in the string tables.
pub fn item_of(letter: Letter) -> Option<FeedItem> {
    match letter {
        Letter::Update => with_feed(|feed| feed.update.cloned()),
        Letter::News(id) => with_feed(|feed| feed.news.iter().find(|item| item.id == id).cloned()),
        Letter::Welcome | Letter::Thanks | Letter::WhatsNew => None,
    }
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

/// This build's version as `VERSIONINFO` spells it — `"0.41.0"`, three parts and not four, the
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

        Action::OpenWizard => open_wizard(owner),
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
        Kind::Wizard => IDD_WIZARD,
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
        wizard: None,
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

    state.author = with_feed(|feed| {
        author_view(
            &stored,
            today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day")),
            &version_string(),
            feed,
        )
    });

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

// -----------------------------------------------------------------------------------------
// The feed thread — SPEC section 6.1, SEC-05
// -----------------------------------------------------------------------------------------

/// The message the feed thread posts to the UI window when it has something — **and it carries
/// nothing at all**.
///
/// SEC-05, the shape SPEC section 6.3 asks for: the message says «look in the box», and what is
/// in the box is checked where it is taken out. A `wParam` carrying a pointer would be a
/// pointer any process of this integrity level could forge.
pub const WM_APP_FEED: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 19;

/// What the feed thread leaves for the UI thread.
///
/// A `Mutex` and not an atomic because what is passed is a whole document; and a mutex is
/// allowed here for the reason NFR-04 gives for forbidding it elsewhere — this is the UI and
/// the feed thread, not the hook path, and neither of them is on anybody's keystroke.
static MAILBOX: std::sync::Mutex<Option<feed::Feed>> = std::sync::Mutex::new(None);

/// Whether a read is in flight. One at a time: a second thread would be a second request.
static READING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// **Starts the one read of FR-102**, on a thread that lives exactly one request.
///
/// SPEC section 6.1: a fourth thread, short-lived, doing no user interface, no input and no
/// hook. It reads, checks the signature, parses, leaves the answer in [`MAILBOX`] and posts
/// [`WM_APP_FEED`] to the UI window. Then it ends.
///
/// Nothing at all happens when the feed is off (`[letters] feed = false`): no thread is
/// started, so there is no timer, no socket and no name to resolve — which is what makes the
/// switch of FR-102 a real switch and not a filter on the answer.
pub fn start_feed_read(owner: HWND, language: String) {
    use std::sync::atomic::Ordering;

    if READING.swap(true, Ordering::AcqRel) {
        return;
    }

    // The window is passed by value across the thread boundary as a number: `HWND` is not
    // `Send`, and what is actually sent is the handle's bits — which `PostMessageW` is
    // documented to take from any thread.
    let target = owner.0 as isize;

    let started = std::thread::Builder::new()
        .name("langsw-feed".to_owned())
        .spawn(move || {
            let answer = feed::read_now(&language);

            if let Ok(mut box_of) = MAILBOX.lock() {
                *box_of = answer;
            }

            READING.store(false, Ordering::Release);

            // SAFETY: `target` is the bits of this program's own UI window, which outlives
            // every thread of the process (it is destroyed as the process ends); the message
            // carries two zeros and no pointer.
            let _ = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(HWND(target as *mut std::ffi::c_void)),
                    WM_APP_FEED,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
        });

    if started.is_err() {
        READING.store(false, Ordering::Release);
        crate::app::report_non_critical("thread body", &WinError::from_thread());
    }
}

/// **Takes what the feed thread left** — the far end of [`WM_APP_FEED`], on the UI thread.
///
/// SEC-05: a forged message finds an empty box and does nothing at all. What is in the box came
/// from this program's own thread and has already been checked against the author's signature —
/// the message is only the nudge that says to look.
///
/// Everything that happens to the state happens here, in one place and in this order: the
/// news is published, the day of the read is written (**only** on a successful read — FR-102),
/// what has fallen out of the feed is forgotten, and the dot on the icon is recomputed.
pub fn take_feed() {
    let Some(fresh) = MAILBOX.lock().ok().and_then(|mut box_of| box_of.take()) else {
        return;
    };

    let Some(today) = today() else {
        return;
    };

    publish_feed(fresh.update.clone(), fresh.news.clone());

    let view = FeedView {
        update: fresh.update.as_ref(),
        news: &fresh.news,
    };

    update_state(|state| {
        state.feed_last_read = Some(today);
        forget_expired(state, view);
    });

    crate::tray::refresh_unread_mark();

    crate::diag::record(
        crate::diag::Operation::from_name("feed read ok"),
        crate::diag::OsCode::NONE,
    );
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

    // FR-102: the one read of the feed, when it is due. Started **before** the letters are
    // looked at, on a thread of its own — what it brings back reaches the next tick, and the
    // one after that if the answer was slow. Nothing waits for it.
    if feed_read_is_due(&stored, today) {
        start_feed_read(owner, settings::ui_language().tag().to_owned());
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

/// Which two strings each letter knocks with — FR-101, задача Т-33а-4.
///
/// `None` for «Привет»: FR-101 shows it at once and never announces it.
///
/// # ⛔ Почему это отдельная таблица, а не `match` внутри `announce`
///
/// До этой задачи «Что нового» стучалось словами «Обновления»: «Вышла версия 0.43.0 · Три
/// изменения **и ссылка на загрузку**». Обе половины были неправдой — версия не «вышла», она
/// уже стоит и работает, а загружать по этому письму нечего. Нашлось это глазом на снимке
/// пользователя, потому что проверить было нечем: строки выбирались внутри функции, которой
/// нужен живой трей. Теперь выбор — чистая функция, и `tests\letters.rs` перебирает по ней все
/// пять писем разом.
///
/// ⭐ Заголовок «Что нового» — тот же [`IDS_WHATSNEW_TITLE`], которым подписано само письмо:
/// слова там те же и уже переведены на четырнадцать языков, а стук и письмо обязаны говорить
/// одно и то же.
pub const fn toast_strings(letter: Letter) -> Option<(u16, u16)> {
    use crate::settings::{
        IDS_TOAST_NEWS, IDS_TOAST_THANKS, IDS_TOAST_TITLE, IDS_TOAST_UPDATE,
        IDS_TOAST_UPDATE_TITLE, IDS_TOAST_WHATSNEW, IDS_WHATSNEW_TITLE,
    };

    match letter {
        Letter::Welcome => None,
        Letter::Thanks => Some((IDS_TOAST_TITLE, IDS_TOAST_THANKS)),
        Letter::News(_) => Some((IDS_TOAST_TITLE, IDS_TOAST_NEWS)),
        Letter::Update => Some((IDS_TOAST_UPDATE_TITLE, IDS_TOAST_UPDATE)),
        Letter::WhatsNew => Some((IDS_WHATSNEW_TITLE, IDS_TOAST_WHATSNEW)),
    }
}

/// Knocks once with the balloon of the icon — FR-101, every letter but «Привет».
fn announce(letter: Letter, version: &str) {
    use crate::settings::{format_text, text};

    // «Привет» is never announced: FR-101 shows it at once, and this function is not reached
    // for it. NFR-13: a letter with no strings knocks with nothing rather than with blanks.
    let Some((title_id, body_id)) = toast_strings(letter) else {
        return;
    };

    let title = match letter {
        // The version in the update's title is the one the **feed** named, not the one running.
        Letter::Update => {
            let newer = with_feed(|feed| {
                update_is_pending(version, feed)
                    .and_then(|item| item.version.clone())
                    .unwrap_or_default()
            });

            format_text(title_id, &[&newer])
        }
        // And in «Что нового» — the one just installed.
        Letter::WhatsNew => format_text(title_id, &[version]),
        _ => text(title_id),
    };

    crate::tray::announce_letter(&title, &text(body_id));
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

                widgets::repaint::whole(hwnd);
            }

            settings::LanguageSwitch::Reopen => {
                // SAFETY: as above.
                let letter = unsafe { with_state(hwnd, |state| (state.letter, state.kind)) };

                close_window(hwnd);

                match letter {
                    Some((Some(letter), Kind::Letter)) => {
                        // Its entry again — the window is being built from nothing, and
                        // `None` here would reopen «Новость» as an empty frame.
                        let item = item_of(letter);

                        show_letter(owner, letter, item.as_ref(), false);
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
                // The entry comes back out of the published feed, not out of thin air: a
                // «Новость» rebuilt in another language must be the same news item.
                let item = item_of(letter);

                state.plan = plan_for(
                    letter,
                    &PlanContext {
                        version,
                        previous_version: stored.last_seen_version.clone(),
                        hotkey: String::new(),
                        snooze_offered: snooze_is_offered(&stored),
                        item: item.as_ref(),
                    },
                );
            }
        }
        Kind::Author => {
            state.author = with_feed(|feed| {
                author_view(
                    &stored,
                    today()
                        .unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day")),
                    &version,
                    feed,
                )
            });
        }
        // Nothing to rebuild: the list is built out of the feed by `fill_list` on every fill,
        // and the window of thanks carries a plan that was made for it once.
        Kind::List => {}
        // The wizard's text is the person's own and is **not** rebuilt: a change of interface
        // language in the middle of an appeal must not throw away what they wrote. What is
        // rebuilt is every label around it, by `fill_wizard`, which the caller runs next.
        Kind::Wizard => {}
    }
}

// =========================================================================================
// 11а. The text of an appeal — FR-104, task Т-32-8
// =========================================================================================

/// **What the wizard of FR-104 writes, and nothing that writes it anywhere.**
///
/// [`report::build`] is a pure function of two records — what the person filled in and what
/// the machine says — and that is the whole point of the split: the text of an appeal can be
/// walked by a test in every one of its branches without a window, without a registry and
/// without a clipboard. The impure half — asking Windows for its build, the scale, the
/// monitors, the layouts — is [`report::facts_now`], and it is called once, by the window.
///
/// ⛔ **Nothing here sends anything.** FR-104 says so and SEC-03 repeats it: the outcome of
/// this module is a `String`. What happens to that string — the clipboard, or a file in the
/// journal folder — is the person's choice, made by pressing a button, and even then the
/// program only *offers* it: it does not open a connection, does not upload and does not run.
pub mod report {
    use crate::settings::{self, Language};

    /// The three cards of the first step — which of them is chosen decides the shape of the
    /// whole wizard: five steps for the two troubles, three for the idea.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Trouble {
        /// «Программа сделала не то».
        WrongResult,
        /// «Ничего не произошло».
        NothingHappened,
        /// «Хочу предложить улучшение» — the short road.
        Idea,
    }

    impl Trouble {
        /// Whether this is the short road of three steps.
        pub fn is_idea(self) -> bool {
            matches!(self, Self::Idea)
        }

        /// How many steps the wizard has for this kind of appeal — FR-104.
        pub fn steps(self) -> usize {
            if self.is_idea() { 3 } else { 5 }
        }
    }

    /// The radio of the «Где» step: what kind of field the text was going into.
    ///
    /// «Поле пароля» is not a curiosity: in a password field this program deliberately keeps
    /// no buffer (SEC-02), so an appeal about one has a known answer before it is read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Field {
        /// «Обычное поле ввода».
        Normal,
        /// «Поле пароля».
        Password,
        /// «Не знаю».
        Unknown,
    }

    /// The radio of the «Что делали» step.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Repeat {
        /// «каждый раз».
        Always,
        /// «иногда».
        Sometimes,
        /// «один раз».
        Once,
    }

    /// Which of the four boxes of the «Что приложить» step are ticked.
    ///
    /// All four start ticked — the mock-up shows them so — and every one of them can be taken
    /// off. What a box adds is written beside it, in full, before it is added to anything.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Attach {
        /// The version of the program, the build of Windows, the scale and the monitors.
        pub machine: bool,
        /// The layouts installed in the system.
        pub layouts: bool,
        /// The settings — the fields of section 7, and **no paths**.
        pub settings: bool,
        /// The journal, out of memory: operation names and codes, never a keystroke.
        pub journal: bool,
    }

    impl Default for Attach {
        fn default() -> Self {
            Self {
                machine: true,
                layouts: true,
                settings: true,
                journal: true,
            }
        }
    }

    /// Everything the person filled in — and **only** what they filled in.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Draft {
        /// Which of the three cards.
        pub trouble: Trouble,
        /// «имя.exe · КлассОкна», as the capture of FR-104 writes it. Empty until captured,
        /// and a person may type into it instead.
        pub program: String,
        /// What kind of field it was.
        pub field: Field,
        /// The layout the word was typed in — a name out of `settings::layout_labels`.
        pub layout: String,
        /// The hotkey as it acts, for the sentence «и нажали …».
        pub hotkey: String,
        /// «Ожидали:».
        pub expected: String,
        /// «Получили:».
        pub got: String,
        /// How often.
        pub repeat: Repeat,
        /// «Что предлагаете?» — the idea road.
        pub idea: String,
        /// «Чем это поможет?» — the idea road.
        pub helps: String,
        /// Which of the four boxes are ticked.
        pub attach: Attach,
    }

    impl Default for Draft {
        /// The wizard as it opens: the first card, everything empty, every box ticked.
        fn default() -> Self {
            Self {
                trouble: Trouble::WrongResult,
                program: String::new(),
                field: Field::Normal,
                layout: String::new(),
                hotkey: String::new(),
                expected: String::new(),
                got: String::new(),
                repeat: Repeat::Always,
                idea: String::new(),
                helps: String::new(),
                attach: Attach::default(),
            }
        }
    }

    /// What the machine says about itself — collected once, outside [`build`].
    ///
    /// Every field is a **finished sentence**, not a number: the collecting is where the Win32
    /// calls live, and a field that could not be read is simply empty. A report never says
    /// «unknown»: it leaves the line out.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Facts {
        /// «0.41.0».
        pub version: String,
        /// «Windows 11 Pro 26100», out of the registry.
        pub windows: String,
        /// «100 %».
        pub scale: String,
        /// «1».
        pub monitors: String,
        /// «English (United States), Русский (Россия)».
        pub layouts: String,
        /// «клавиша Pause · пара 0409 и 0419 · метод auto · исключений: 1».
        pub settings: String,
        /// The journal as `diag::render` writes it.
        pub journal: String,
        /// How many entries that journal holds — the number the check box shows.
        pub journal_entries: usize,
    }

    /// **The whole text of an appeal** — FR-104, and a pure function of its two arguments.
    ///
    /// The labels come out of the string tables of FR-94, so the appeal is written in the
    /// language of the interface: the person writes their half in their own words, and the
    /// program writes its half in the same language rather than in a third one.
    ///
    /// Empty answers are left out entirely. A report with three lines in it is a report; a
    /// report with eleven labels and eight blanks is a form, and a form is harder to read than
    /// the sentence it was made of.
    pub fn build(draft: &Draft, facts: &Facts) -> String {
        use settings::{
            IDS_WIZARD_ATTACH_JOURNAL, IDS_WIZARD_ATTACH_LAYOUTS, IDS_WIZARD_ATTACH_MACHINE,
            IDS_WIZARD_ATTACH_SETTINGS, IDS_WIZARD_CARD_IDEA, IDS_WIZARD_CARD_NOTHING,
            IDS_WIZARD_CARD_WRONG, IDS_WIZARD_EXPECTED, IDS_WIZARD_FIELD_NORMAL,
            IDS_WIZARD_FIELD_PASSWORD, IDS_WIZARD_FIELD_UNKNOWN, IDS_WIZARD_GOT,
            IDS_WIZARD_IDEA_HELPS, IDS_WIZARD_IDEA_WHAT, IDS_WIZARD_PROGRAM, IDS_WIZARD_REPEAT,
            IDS_WIZARD_REPEAT_ALWAYS, IDS_WIZARD_REPEAT_ONCE, IDS_WIZARD_REPEAT_SOMETIMES,
            IDS_WIZARD_TYPED_IN, IDS_WIZARD_WHAT_TITLE, IDS_WIZARD_WHERE_FIELD, text,
        };

        let mut out = String::with_capacity(1024);

        // The heading is the program's name and this build's version, and it is not
        // translated — it is what the author reads first and has to recognise at a glance.
        out.push_str(crate::APP_NAME);

        if !facts.version.is_empty() {
            out.push(' ');
            out.push_str(&facts.version);
        }

        out.push('\n');
        out.push_str(&"=".repeat(32));
        out.push_str("\n\n");

        line(
            &mut out,
            &text(IDS_WIZARD_WHAT_TITLE),
            &text(match draft.trouble {
                Trouble::WrongResult => IDS_WIZARD_CARD_WRONG,
                Trouble::NothingHappened => IDS_WIZARD_CARD_NOTHING,
                Trouble::Idea => IDS_WIZARD_CARD_IDEA,
            }),
        );

        if draft.trouble.is_idea() {
            line(&mut out, &text(IDS_WIZARD_IDEA_WHAT), &draft.idea);
            line(&mut out, &text(IDS_WIZARD_IDEA_HELPS), &draft.helps);
        } else {
            line(&mut out, &text(IDS_WIZARD_PROGRAM), &draft.program);
            line(
                &mut out,
                &text(IDS_WIZARD_WHERE_FIELD),
                &text(match draft.field {
                    Field::Normal => IDS_WIZARD_FIELD_NORMAL,
                    Field::Password => IDS_WIZARD_FIELD_PASSWORD,
                    Field::Unknown => IDS_WIZARD_FIELD_UNKNOWN,
                }),
            );
            line(&mut out, &text(IDS_WIZARD_TYPED_IN), &draft.layout);
            line(&mut out, &text(IDS_WIZARD_EXPECTED), &draft.expected);
            line(&mut out, &text(IDS_WIZARD_GOT), &draft.got);
            line(
                &mut out,
                &text(IDS_WIZARD_REPEAT),
                &text(match draft.repeat {
                    Repeat::Always => IDS_WIZARD_REPEAT_ALWAYS,
                    Repeat::Sometimes => IDS_WIZARD_REPEAT_SOMETIMES,
                    Repeat::Once => IDS_WIZARD_REPEAT_ONCE,
                }),
            );
        }

        // The hotkey travels with a trouble and not with an idea: an idea is not about a key
        // press. It is written under the same label the window used.
        if !draft.trouble.is_idea() && !draft.hotkey.is_empty() {
            line(&mut out, &text(settings::IDS_HOTKEY_LABEL), &draft.hotkey);
        }

        if draft.attach.machine {
            line(
                &mut out,
                &text(IDS_WIZARD_ATTACH_MACHINE),
                &machine_line(facts),
            );
        }

        if draft.attach.layouts {
            line(&mut out, &text(IDS_WIZARD_ATTACH_LAYOUTS), &facts.layouts);
        }

        if draft.attach.settings {
            line(&mut out, &text(IDS_WIZARD_ATTACH_SETTINGS), &facts.settings);
        }

        if draft.attach.journal && !facts.journal.trim().is_empty() {
            out.push('\n');
            out.push_str(&text(IDS_WIZARD_ATTACH_JOURNAL));
            out.push('\n');
            out.push_str(&"-".repeat(32));
            out.push('\n');
            out.push_str(facts.journal.trim_end());
            out.push('\n');
        }

        // ⛔ **CRLF, and it is not a preference.** The appeal is written in one place and read
        // in three, and all three are Windows: a multi-line `EDIT` shows a lone `\n` as **no
        // break at all** — the whole appeal ran together into one line, and the stand's very
        // first screenshot of the last step showed it — the clipboard's `CF_UNICODETEXT` is
        // defined with CRLF, and a file with lone newlines opens in Notepad as one line too.
        // The body above is built with `\n` because that is what a Rust string is comfortable
        // with; the conversion is here, once, at the way out.
        crlf(&out)
    }

    /// Every lone `\n` of a string as `\r\n`, and a `\r\n` left as it is.
    fn crlf(text: &str) -> String {
        let mut out = String::with_capacity(text.len() + text.len() / 16);
        let mut after_cr = false;

        for character in text.chars() {
            if character == '\n' && !after_cr {
                out.push('\r');
            }

            after_cr = character == '\r';
            out.push(character);
        }

        out
    }

    /// The one sentence the first box of «Что приложить?» stands for — the build of Windows,
    /// the scale and the monitors, in the order the mock-up prints them.
    ///
    /// Public and pure: the window shows it beside the box **before** anything is attached,
    /// which is the whole promise of that step («ниже написано, что именно попадёт в текст»),
    /// and [`build`] writes the same sentence into the appeal.
    pub fn machine_line(facts: &Facts) -> String {
        join(
            " · ",
            &[
                facts.windows.as_str(),
                facts.scale.as_str(),
                facts.monitors.as_str(),
            ],
        )
    }

    /// One «label: value» line — and nothing at all when the value is empty.
    fn line(out: &mut String, label: &str, value: &str) {
        let value = value.trim();

        if value.is_empty() {
            return;
        }

        // A label written for a window may already end in a colon («Ожидали:») and may not
        // («Раскладки в системе»). One colon either way, and никогда two.
        let label = label.trim_end_matches([':', '?']);

        out.push_str(label);
        out.push_str(": ");

        // A value of several lines is indented under its label rather than run together: the
        // two free-text fields of the wizard accept newlines and people use them.
        let mut lines = value.lines();

        if let Some(first) = lines.next() {
            out.push_str(first.trim_end());
        }

        for rest in lines {
            out.push_str("\n    ");
            out.push_str(rest.trim_end());
        }

        out.push('\n');
    }

    /// Joins the parts that are not empty — so a fact the machine would not give leaves no
    /// « ·  · » behind it.
    fn join(separator: &str, parts: &[&str]) -> String {
        parts
            .iter()
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// **Asks the machine about itself** — the impure half of this module, called once, by the
    /// window, on the UI thread.
    ///
    /// Every answer is optional and a refusal costs nothing: a field that could not be read
    /// stays empty and [`build`] leaves its line out. NFR-13 — a wizard that could not open a
    /// registry key must still write an appeal.
    ///
    /// ⛔ **Nothing here identifies the machine or the person.** The build of Windows, the
    /// scale, the number of monitors and the layouts are what an author needs to reproduce a
    /// defect; the user name, the computer name, the serial numbers and the paths are not, and
    /// are not read. The settings go in as **values without paths** — §7 fields, never
    /// `%APPDATA%`.
    pub fn facts_now() -> Facts {
        // The configuration comes from the tray for the same reason `state_now` takes it from
        // there: the tray holds the one live copy, and reading the file again could answer
        // with something the person has not applied yet.
        let settings_line =
            crate::tray::with_tray(|tray| settings_now(tray.config())).unwrap_or_default();

        Facts {
            version: super::version_string(),
            windows: windows_build(),
            scale: scale_now(),
            monitors: monitors_now(),
            layouts: layouts_now(),
            settings: settings_line,
            journal: crate::diag::render(),
            journal_entries: crate::diag::snapshot().len(),
        }
    }

    /// «Windows 11 Pro 22H2 26100» out of `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`.
    ///
    /// Read straight from the registry rather than through `reg.exe` — FR-104 says so, and the
    /// reason is that spawning a process to read three strings is a process this program has no
    /// business spawning. An empty answer for a key that will not open: the line goes missing,
    /// nothing else.
    fn windows_build() -> String {
        use windows::Win32::Foundation::ERROR_SUCCESS;
        use windows::Win32::System::Registry::{
            HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, REG_SZ, REG_VALUE_TYPE, RegCloseKey,
            RegOpenKeyExW, RegQueryValueExW,
        };
        use windows::core::{PCWSTR, w};

        let mut key = HKEY::default();

        // SAFETY: the path is a `'static` NUL-terminated literal, `key` a live local the call
        // fills, and the handle is closed on every path below.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
                None,
                KEY_QUERY_VALUE,
                &mut key,
            )
        };

        if status != ERROR_SUCCESS {
            return String::new();
        }

        let read = |name: PCWSTR| -> String {
            let mut kind = REG_VALUE_TYPE::default();
            let mut bytes: u32 = 0;

            // SAFETY: the two-call pattern of the registry API — the first call asks for the
            // size alone, both out parameters are live locals, and the name is a literal.
            let status = unsafe {
                RegQueryValueExW(key, name, None, Some(&mut kind), None, Some(&mut bytes))
            };

            if status != ERROR_SUCCESS || kind != REG_SZ || bytes == 0 {
                return String::new();
            }

            let units = bytes as usize / size_of::<u16>();
            let mut buffer = vec![0u16; units + 1];
            let mut capacity = (buffer.len() * size_of::<u16>()) as u32;

            // SAFETY: `buffer` is a live allocation of `capacity` bytes owned by this frame and
            // `capacity` describes it exactly, so the call cannot write past its end.
            let status = unsafe {
                RegQueryValueExW(
                    key,
                    name,
                    None,
                    None,
                    Some(buffer.as_mut_ptr().cast::<u8>()),
                    Some(&mut capacity),
                )
            };

            if status != ERROR_SUCCESS {
                return String::new();
            }

            let written = capacity as usize / size_of::<u16>();
            let text = &buffer[..written.min(buffer.len())];
            let end = text
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(text.len());

            String::from_utf16_lossy(&text[..end])
        };

        let product = read(w!("ProductName"));
        let display = read(w!("DisplayVersion"));
        let build = read(w!("CurrentBuild"));

        // SAFETY: `key` came from a successful `RegOpenKeyExW` above and is closed exactly once.
        let _ = unsafe { RegCloseKey(key) };

        join(
            " ",
            &[
                eleven(&product, &build).as_str(),
                display.as_str(),
                build.as_str(),
            ],
        )
    }

    /// ⛔ **`ProductName` says «Windows 10 Pro» on Windows 11**, and the live acceptance of Э32
    /// wrote exactly that into an appeal from a machine running build 26100. Microsoft left the
    /// value behind at the change of name, and every reader of that key has to correct it the
    /// way `winver` does — by the build number. Twenty-two thousand is where 11 begins.
    ///
    /// Pure, and separate for that reason: the rule is a rule about two strings, and a test can
    /// walk it without a registry.
    pub fn eleven(product: &str, build: &str) -> String {
        let is_eleven = build
            .trim()
            .parse::<u32>()
            .is_ok_and(|number| number >= 22000);

        if is_eleven && product.contains("Windows 10") {
            product.replacen("Windows 10", "Windows 11", 1)
        } else {
            product.to_owned()
        }
    }

    /// «масштаб 100 %» — the scale of the screen this program's window is on.
    fn scale_now() -> String {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, LOGPIXELSY, ReleaseDC};

        // SAFETY: `None` asks for the screen's own device context, which is released below.
        let dc = unsafe { GetDC(None) };

        if dc.is_invalid() {
            return String::new();
        }

        // SAFETY: `dc` is the live context just taken.
        let dpi = unsafe { GetDeviceCaps(Some(dc), LOGPIXELSY) };

        // SAFETY: releases exactly the context taken above, once.
        let _ = unsafe { ReleaseDC(None::<HWND>, dc) };

        if dpi <= 0 {
            return String::new();
        }

        format!("{} %", dpi * 100 / 96)
    }

    /// «мониторов: 2».
    fn monitors_now() -> String {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};

        // SAFETY: a plain query with no arguments to keep alive.
        let count = unsafe { GetSystemMetrics(SM_CMONITORS) };

        if count <= 0 {
            String::new()
        } else {
            count.to_string()
        }
    }

    /// The layouts installed in the system, by the names the settings window shows.
    fn layouts_now() -> String {
        let Ok(installed) = crate::layouts::enumerate_all() else {
            return String::new();
        };

        settings::layout_labels(&installed, &installed).join(", ")
    }

    /// The settings, as **values without paths** — §7 and nothing about this disk.
    fn settings_now(config: &settings::Config) -> String {
        let hotkey = config.hotkey.key.clone();
        let layouts = match config.layouts.mode {
            settings::LayoutMode::Pair => format!(
                "{} → {}",
                config.layouts.pair_source, config.layouts.pair_target
            ),
            settings::LayoutMode::Cycle => config.layouts.cycle.join(", "),
        };

        let method = match config.replacement.method {
            settings::ReplacementMethod::Auto => "auto",
            settings::ReplacementMethod::Backspace => "backspace",
            settings::ReplacementMethod::Selection => "selection",
        };

        format!(
            "{hotkey} · {layouts} · {method} · selection {} · exclusions {}",
            if config.selection.enabled {
                "on"
            } else {
                "off"
            },
            config.exclusions.processes.len(),
        )
    }

    /// The name of the file «Сохранить в папку журнала» writes — `обращение-<дата>.txt`.
    ///
    /// Pure, and separate from the writing for the same reason `build` is: a test can check the
    /// shape of the name without touching a disk. The date is the one the caller passes, and
    /// it is the ISO form the configuration file uses rather than a localised one — a file
    /// name that changed shape with the interface language would sort differently on the same
    /// machine on two different days.
    pub fn file_name(today: super::Date) -> String {
        let stem = match settings::ui_language() {
            Language::Ru => "обращение",
            _ => "report",
        };

        format!("{stem}-{today}.txt")
    }
}

// =========================================================================================
// 11б. The wizard «Написать автору» — FR-104, task Т-32-8
// =========================================================================================

/// **Opens the wizard** — FR-104, the one window of this program a person walks through.
///
/// The facts are read here, once: a machine that changed its scale between the first step and
/// the last would otherwise write two different things into one appeal.
pub fn open_wizard(owner: HWND) {
    let mut state = fresh_state(owner, Kind::Wizard);

    let layouts = crate::layouts::enumerate_all()
        .map(|installed| settings::layout_labels(&installed, &installed))
        .unwrap_or_default();

    let mut draft = report::Draft {
        hotkey: state.hotkey.clone(),
        ..report::Draft::default()
    };

    // The layout a person is most likely to have typed in is the first one they have; it is a
    // guess, and the combo beside it is how they say otherwise.
    if let Some(first) = layouts.first() {
        draft.layout = first.clone();
    }

    state.wizard = Some(Box::new(Wizard {
        draft,
        facts: report::facts_now(),
        step: 0,
        countdown: 0,
        status: String::new(),
        layouts,
        text: String::new(),
        done: false,
    }));

    open_window(owner, state, true);
}

// ⛔ **`open_thanks` is gone — задача Т-33-3, решение 103.3.** «Готово» used to close the
// wizard and open a window of its own on `IDD_LETTER` with four lines of text in it. The user
// asked for the thanks to be the wizard's last step, in the same window and at the same size:
// «Окно спасибо в конце было бы логично сделать того же размера, что и мастер, и сделать
// финальным шагом мастера, а не отдельным всплывающим окном». What that window said is said by
// [`Step::Done`] now, out of the very same four strings, and `Kind::Thanks` went with it.

/// Which controls one step shows — every other slot of the pool is hidden.
///
/// A table and not a chain of conditions: the six steps of FR-104 differ in **which slots they
/// fill** and in nothing else, and a table is what a test can walk.
pub fn controls_of(step: Step) -> &'static [i32] {
    match step {
        Step::What => &[
            IDC_WZ_CARD_1,
            IDC_WZ_CARD_1 + 1,
            IDC_WZ_CARD_1 + 2,
            IDC_WZ_CARD_T1,
            IDC_WZ_CARD_T1 + 1,
            IDC_WZ_CARD_T1 + 2,
            IDC_WZ_CARD_S1,
            IDC_WZ_CARD_S1 + 1,
            IDC_WZ_CARD_S1 + 2,
        ],
        Step::Where => &[
            IDC_WZ_PROGRAM_LABEL,
            IDC_WZ_PROGRAM,
            IDC_WZ_CAPTURE,
            IDC_WZ_CAPTURE_NOTE,
            IDC_WZ_FIELD_1,
            IDC_WZ_FIELD_1 + 1,
            IDC_WZ_FIELD_2_SUB,
            IDC_WZ_FIELD_1 + 2,
        ],
        Step::Did => &[
            IDC_WZ_TYPED_LABEL,
            IDC_WZ_LAYOUT,
            IDC_WZ_PRESSED_LABEL,
            IDC_WZ_CHIP,
            IDC_WZ_EXPECTED_LABEL,
            IDC_WZ_EXPECTED,
            IDC_WZ_GOT_LABEL,
            IDC_WZ_GOT,
            IDC_WZ_REPEAT_LABEL,
            IDC_WZ_REPEAT_1,
            IDC_WZ_REPEAT_1 + 1,
            IDC_WZ_REPEAT_1 + 2,
        ],
        Step::Idea => &[
            IDC_WZ_IDEA_LABEL,
            IDC_WZ_IDEA,
            IDC_WZ_HELPS_LABEL,
            IDC_WZ_HELPS,
        ],
        Step::Attach => &[
            IDC_WZ_ATTACH_1,
            IDC_WZ_ATTACH_V1,
            IDC_WZ_ATTACH_1 + 1,
            IDC_WZ_ATTACH_V1 + 1,
            IDC_WZ_ATTACH_1 + 2,
            IDC_WZ_ATTACH_V1 + 2,
            IDC_WZ_ATTACH_1 + 3,
            IDC_WZ_ATTACH_V1 + 3,
            IDC_WZ_ATTACH_FOOT,
        ],
        Step::Preview => &[IDC_WZ_PREVIEW, IDC_WZ_COPY, IDC_WZ_SAVE, IDC_WZ_STATUS],
        Step::Done => &[IDC_WZ_THANKS, IDC_WZ_CHANNEL],
    }
}

/// Every slot of the pool — what «hide everything, then place this step's own» is written
/// against. Two arrays because the pool is longer than one line of a `const` is comfortable
/// with; a slot missing from **both** is a slot that never gets hidden, and that is a defect
/// only a screenshot would find.
const WIZARD_SLOTS: [i32; 22] = [
    IDC_WZ_CARD_1,
    IDC_WZ_CARD_1 + 1,
    IDC_WZ_CARD_1 + 2,
    IDC_WZ_CARD_T1,
    IDC_WZ_CARD_T1 + 1,
    IDC_WZ_CARD_T1 + 2,
    IDC_WZ_CARD_S1,
    IDC_WZ_CARD_S1 + 1,
    IDC_WZ_CARD_S1 + 2,
    IDC_WZ_PROGRAM_LABEL,
    IDC_WZ_PROGRAM,
    IDC_WZ_CAPTURE,
    IDC_WZ_CAPTURE_NOTE,
    IDC_WZ_FIELD_1,
    IDC_WZ_FIELD_1 + 1,
    IDC_WZ_FIELD_1 + 2,
    IDC_WZ_FIELD_2_SUB,
    IDC_WZ_TYPED_LABEL,
    IDC_WZ_LAYOUT,
    IDC_WZ_PRESSED_LABEL,
    IDC_WZ_CHIP,
    IDC_WZ_EXPECTED_LABEL,
];

/// The rest of the pool — see [`WIZARD_SLOTS`].
const WIZARD_SLOTS_TAIL: [i32; 18] = [
    IDC_WZ_EXPECTED,
    IDC_WZ_GOT_LABEL,
    IDC_WZ_GOT,
    IDC_WZ_REPEAT_LABEL,
    IDC_WZ_REPEAT_1,
    IDC_WZ_REPEAT_1 + 1,
    IDC_WZ_REPEAT_1 + 2,
    IDC_WZ_IDEA_LABEL,
    IDC_WZ_IDEA,
    IDC_WZ_HELPS_LABEL,
    IDC_WZ_HELPS,
    IDC_WZ_ATTACH_1,
    IDC_WZ_ATTACH_1 + 1,
    IDC_WZ_ATTACH_1 + 2,
    IDC_WZ_ATTACH_1 + 3,
    IDC_WZ_ATTACH_V1,
    IDC_WZ_ATTACH_FOOT,
    IDC_WZ_PREVIEW,
];

/// The rest again — the value lines two, three and four of «Что приложить?», the status line,
/// and the two buttons of the last step.
///
/// ⚠ **The two buttons belong here and it took a test to notice.** They are push buttons like
/// the three at the bottom, and the eye reads them as chrome — but they are chrome of **one
/// step**, and a slot missing from the pool is a control that never gets hidden: «Скопировать
/// и открыть канал» would have stood on every step of the wizard.
const WIZARD_SLOTS_VALUES: [i32; 8] = [
    IDC_WZ_ATTACH_V1 + 1,
    IDC_WZ_ATTACH_V1 + 2,
    IDC_WZ_ATTACH_V1 + 3,
    IDC_WZ_STATUS,
    IDC_WZ_COPY,
    IDC_WZ_SAVE,
    // The two slots of «Готово» — задача Т-33-3. They belong to the pool for the same reason
    // the two buttons above it do: a slot missing from the pool is a control that never gets
    // hidden, and the thanks would then stand on every step of the wizard.
    IDC_WZ_THANKS,
    IDC_WZ_CHANNEL,
];

/// How many slots the wizard's pool holds — what a test compares the sum of the six steps
/// against, so that a slot belonging to no step at all shows up as a number rather than as a
/// control nobody ever sees.
pub fn wizard_slot_count() -> usize {
    WIZARD_SLOTS.len() + WIZARD_SLOTS_TAIL.len() + WIZARD_SLOTS_VALUES.len()
}

/// Whether this control of the wizard is checked right now — the wizard's own record and never
/// the control, which keeps no state of its own once it is `BS_OWNERDRAW`.
fn wizard_is_checked(wizard: &Wizard, control: i32) -> bool {
    use report::{Field, Repeat, Trouble};

    match control {
        IDC_WZ_CARD_1 => wizard.draft.trouble == Trouble::WrongResult,
        c if c == IDC_WZ_CARD_1 + 1 => wizard.draft.trouble == Trouble::NothingHappened,
        c if c == IDC_WZ_CARD_1 + 2 => wizard.draft.trouble == Trouble::Idea,
        IDC_WZ_FIELD_1 => wizard.draft.field == Field::Normal,
        c if c == IDC_WZ_FIELD_1 + 1 => wizard.draft.field == Field::Password,
        c if c == IDC_WZ_FIELD_1 + 2 => wizard.draft.field == Field::Unknown,
        IDC_WZ_REPEAT_1 => wizard.draft.repeat == Repeat::Always,
        c if c == IDC_WZ_REPEAT_1 + 1 => wizard.draft.repeat == Repeat::Sometimes,
        c if c == IDC_WZ_REPEAT_1 + 2 => wizard.draft.repeat == Repeat::Once,
        IDC_WZ_ATTACH_1 => wizard.draft.attach.machine,
        c if c == IDC_WZ_ATTACH_1 + 1 => wizard.draft.attach.layouts,
        c if c == IDC_WZ_ATTACH_1 + 2 => wizard.draft.attach.settings,
        c if c == IDC_WZ_ATTACH_1 + 3 => wizard.draft.attach.journal,
        _ => false,
    }
}

/// **Fills the wizard** — every slot's text, for the step now on the screen.
fn fill_wizard(hwnd: HWND, state: &WindowState) {
    use crate::settings::{
        IDS_WIZARD_AND_PRESSED, IDS_WIZARD_ATTACH_FOOT, IDS_WIZARD_ATTACH_JOURNAL,
        IDS_WIZARD_ATTACH_JOURNAL_SUB, IDS_WIZARD_ATTACH_LAYOUTS, IDS_WIZARD_ATTACH_MACHINE,
        IDS_WIZARD_ATTACH_NOTE, IDS_WIZARD_ATTACH_SETTINGS, IDS_WIZARD_ATTACH_TITLE,
        IDS_WIZARD_BACK, IDS_WIZARD_CANCEL, IDS_WIZARD_CAPTION, IDS_WIZARD_CAPTURE,
        IDS_WIZARD_CAPTURE_COUNT, IDS_WIZARD_CAPTURE_NOTE, IDS_WIZARD_CARD_IDEA,
        IDS_WIZARD_CARD_IDEA_SUB, IDS_WIZARD_CARD_NOTHING, IDS_WIZARD_CARD_NOTHING_SUB,
        IDS_WIZARD_CARD_WRONG, IDS_WIZARD_CARD_WRONG_SUB, IDS_WIZARD_COPY, IDS_WIZARD_DID_NOTE,
        IDS_WIZARD_DID_TITLE, IDS_WIZARD_DONE, IDS_WIZARD_EXPECTED, IDS_WIZARD_FIELD_NORMAL,
        IDS_WIZARD_FIELD_PASSWORD, IDS_WIZARD_FIELD_PASSWORD_SUB, IDS_WIZARD_FIELD_UNKNOWN,
        IDS_WIZARD_GOT, IDS_WIZARD_IDEA_HELPS, IDS_WIZARD_IDEA_NOTE, IDS_WIZARD_IDEA_TITLE,
        IDS_WIZARD_IDEA_WHAT, IDS_WIZARD_NEXT, IDS_WIZARD_PREVIEW_NOTE, IDS_WIZARD_PREVIEW_TITLE,
        IDS_WIZARD_PROGRAM, IDS_WIZARD_REPEAT, IDS_WIZARD_REPEAT_ALWAYS, IDS_WIZARD_REPEAT_ONCE,
        IDS_WIZARD_REPEAT_SOMETIMES, IDS_WIZARD_SAVE, IDS_WIZARD_STEP, IDS_WIZARD_TYPED_IN,
        IDS_WIZARD_WHAT_NOTE, IDS_WIZARD_WHAT_TITLE, IDS_WIZARD_WHERE_NOTE, IDS_WIZARD_WHERE_TITLE,
        format_text, text,
    };
    // The words of «Готово» — every one of them a string the abolished window of thanks already
    // had (задача Т-33-3): not one row was added to the tables of FR-94.
    use crate::settings::{
        IDS_CHANNEL_OPEN, IDS_CLOSE, IDS_THANKYOU_BUG_TEXT, IDS_THANKYOU_BUG_TITLE,
        IDS_THANKYOU_IDEA_TEXT, IDS_THANKYOU_IDEA_TITLE,
    };

    let Some(wizard) = state.wizard.as_deref() else {
        return;
    };

    let caption = settings::wide(&text(IDS_WIZARD_CAPTION));

    // SAFETY: `hwnd` is the live window and `caption` a NUL-terminated buffer of this frame.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    let step = wizard.current();
    let road = wizard.road();

    let idea = wizard.draft.trouble.is_idea();

    let (title, note) = match step {
        Step::What => (IDS_WIZARD_WHAT_TITLE, Some(IDS_WIZARD_WHAT_NOTE)),
        Step::Where => (IDS_WIZARD_WHERE_TITLE, Some(IDS_WIZARD_WHERE_NOTE)),
        Step::Did => (IDS_WIZARD_DID_TITLE, Some(IDS_WIZARD_DID_NOTE)),
        Step::Idea => (IDS_WIZARD_IDEA_TITLE, Some(IDS_WIZARD_IDEA_NOTE)),
        Step::Attach => (IDS_WIZARD_ATTACH_TITLE, Some(IDS_WIZARD_ATTACH_NOTE)),
        Step::Preview => (IDS_WIZARD_PREVIEW_TITLE, Some(IDS_WIZARD_PREVIEW_NOTE)),
        // «Готово» — задача Т-33-3: the heading is the one the abolished window carried, and
        // there is no explanation line, because the thanks is a paragraph of its own and a
        // muted sentence above it would be a second voice saying the same thing.
        Step::Done => (
            if idea {
                IDS_THANKYOU_IDEA_TITLE
            } else {
                IDS_THANKYOU_BUG_TITLE
            },
            None,
        ),
    };

    let entries = wizard.facts.journal_entries.to_string();

    for (control, caption) in [
        (
            IDC_WZ_STEP,
            // ⚠ **The last step is not numbered.** «Готово» is where the wizard ends, not the
            // sixth of five — the mock-up puts that word in the place of «Шаг N из M», and it
            // is the very string the button carried up to it.
            if step == Step::Done {
                text(IDS_WIZARD_DONE)
            } else {
                format_text(
                    IDS_WIZARD_STEP,
                    &[&(wizard.step + 1).to_string(), &road.len().to_string()],
                )
            },
        ),
        (
            IDC_WZ_THANKS,
            text(if idea {
                IDS_THANKYOU_IDEA_TEXT
            } else {
                IDS_THANKYOU_BUG_TEXT
            }),
        ),
        (IDC_WZ_CHANNEL, text(IDS_CHANNEL_OPEN)),
        (IDC_WZ_TITLE, text(title)),
        (IDC_WZ_NOTE, note.map(text).unwrap_or_default()),
        (IDC_WZ_CARD_T1, text(IDS_WIZARD_CARD_WRONG)),
        (IDC_WZ_CARD_T1 + 1, text(IDS_WIZARD_CARD_NOTHING)),
        (IDC_WZ_CARD_T1 + 2, text(IDS_WIZARD_CARD_IDEA)),
        (IDC_WZ_CARD_S1, text(IDS_WIZARD_CARD_WRONG_SUB)),
        (IDC_WZ_CARD_S1 + 1, text(IDS_WIZARD_CARD_NOTHING_SUB)),
        (IDC_WZ_CARD_S1 + 2, text(IDS_WIZARD_CARD_IDEA_SUB)),
        (IDC_WZ_PROGRAM_LABEL, text(IDS_WIZARD_PROGRAM)),
        (
            IDC_WZ_CAPTURE,
            if wizard.countdown > 0 {
                format_text(IDS_WIZARD_CAPTURE_COUNT, &[&wizard.countdown.to_string()])
            } else {
                text(IDS_WIZARD_CAPTURE)
            },
        ),
        (IDC_WZ_CAPTURE_NOTE, text(IDS_WIZARD_CAPTURE_NOTE)),
        (IDC_WZ_FIELD_1, text(IDS_WIZARD_FIELD_NORMAL)),
        (IDC_WZ_FIELD_1 + 1, text(IDS_WIZARD_FIELD_PASSWORD)),
        (IDC_WZ_FIELD_2_SUB, text(IDS_WIZARD_FIELD_PASSWORD_SUB)),
        (IDC_WZ_FIELD_1 + 2, text(IDS_WIZARD_FIELD_UNKNOWN)),
        (IDC_WZ_TYPED_LABEL, text(IDS_WIZARD_TYPED_IN)),
        (IDC_WZ_PRESSED_LABEL, text(IDS_WIZARD_AND_PRESSED)),
        (IDC_WZ_CHIP, state.hotkey.clone()),
        (IDC_WZ_EXPECTED_LABEL, text(IDS_WIZARD_EXPECTED)),
        (IDC_WZ_GOT_LABEL, text(IDS_WIZARD_GOT)),
        (IDC_WZ_REPEAT_LABEL, text(IDS_WIZARD_REPEAT)),
        (IDC_WZ_REPEAT_1, text(IDS_WIZARD_REPEAT_ALWAYS)),
        (IDC_WZ_REPEAT_1 + 1, text(IDS_WIZARD_REPEAT_SOMETIMES)),
        (IDC_WZ_REPEAT_1 + 2, text(IDS_WIZARD_REPEAT_ONCE)),
        (IDC_WZ_IDEA_LABEL, text(IDS_WIZARD_IDEA_WHAT)),
        (IDC_WZ_HELPS_LABEL, text(IDS_WIZARD_IDEA_HELPS)),
        (IDC_WZ_ATTACH_1, text(IDS_WIZARD_ATTACH_MACHINE)),
        (IDC_WZ_ATTACH_1 + 1, text(IDS_WIZARD_ATTACH_LAYOUTS)),
        (IDC_WZ_ATTACH_1 + 2, text(IDS_WIZARD_ATTACH_SETTINGS)),
        (IDC_WZ_ATTACH_1 + 3, text(IDS_WIZARD_ATTACH_JOURNAL)),
        (IDC_WZ_ATTACH_V1, report::machine_line(&wizard.facts)),
        (IDC_WZ_ATTACH_V1 + 1, wizard.facts.layouts.clone()),
        (IDC_WZ_ATTACH_V1 + 2, wizard.facts.settings.clone()),
        (
            IDC_WZ_ATTACH_V1 + 3,
            format_text(IDS_WIZARD_ATTACH_JOURNAL_SUB, &[&entries]),
        ),
        (IDC_WZ_ATTACH_FOOT, text(IDS_WIZARD_ATTACH_FOOT)),
        (IDC_WZ_COPY, text(IDS_WIZARD_COPY)),
        (IDC_WZ_SAVE, text(IDS_WIZARD_SAVE)),
        (IDC_WZ_STATUS, wizard.status.clone()),
        (IDC_WZ_CANCEL, text(IDS_WIZARD_CANCEL)),
        (IDC_WZ_BACK, text(IDS_WIZARD_BACK)),
        (
            IDC_WZ_NEXT,
            // Three faces of one button: «Далее», «Готово» on the last step of the road, and
            // «Закрыть» on the thanks — where it is the only button left (задача Т-33-3).
            text(match step {
                Step::Done => IDS_CLOSE,
                _ if wizard.is_last() => IDS_WIZARD_DONE,
                _ => IDS_WIZARD_NEXT,
            }),
        ),
    ] {
        settings::set_text(hwnd, control, &caption);
    }

    // ⚠ The fields are written **only** when what they hold differs from the record. Writing
    // them unconditionally would put the caret back to the start on every repaint, which is
    // what a person typing into one would feel as the window fighting them.
    for (control, value) in [
        (IDC_WZ_PROGRAM, wizard.draft.program.as_str()),
        (IDC_WZ_EXPECTED, wizard.draft.expected.as_str()),
        (IDC_WZ_GOT, wizard.draft.got.as_str()),
        (IDC_WZ_IDEA, wizard.draft.idea.as_str()),
        (IDC_WZ_HELPS, wizard.draft.helps.as_str()),
        (IDC_WZ_PREVIEW, wizard.text.as_str()),
    ] {
        if settings::get_text(hwnd, control) != value {
            settings::set_text(hwnd, control, value);
        }
    }

    if step == Step::Did {
        fill_layout_combo(hwnd, wizard);
    }
}

/// Fills the combo of «Что вы делали?» with the layouts of this machine and selects the one the
/// draft names.
///
/// The list is rebuilt only when its length disagrees with the record: `CB_RESETCONTENT` while
/// the list is dropped down closes it under the pointer, and this function runs on every fill.
fn fill_layout_combo(hwnd: HWND, wizard: &Wizard) {
    use windows::Win32::UI::WindowsAndMessaging::{
        CB_ADDSTRING, CB_GETCOUNT, CB_RESETCONTENT, CB_SETCURSEL,
    };

    let held = settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_GETCOUNT, 0, 0);

    if held != isize::try_from(wizard.layouts.len()).unwrap_or(0) {
        settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_RESETCONTENT, 0, 0);

        for name in &wizard.layouts {
            let wide = settings::wide(name);

            // SAFETY: `wide` is a NUL-terminated UTF-16 buffer of this frame and the control
            // copies it — `CBS_HASSTRINGS` is what makes that true.
            settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_ADDSTRING, 0, wide.as_ptr() as isize);
        }
    }

    let chosen = wizard
        .layouts
        .iter()
        .position(|name| *name == wizard.draft.layout)
        .unwrap_or(0);

    settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_SETCURSEL, chosen, 0);
}

/// Which glyph a control of the wizard wears — a circle for a choice of one out of several, a
/// box for a thing that is on or off by itself, and nothing for the three cards, which are
/// drawn as blocks rather than as radio buttons.
fn wizard_glyph(control: i32) -> Option<theme::GlyphKind> {
    if (IDC_WZ_FIELD_1..IDC_WZ_FIELD_1 + 3).contains(&control)
        || (IDC_WZ_REPEAT_1..IDC_WZ_REPEAT_1 + 3).contains(&control)
    {
        return Some(theme::GlyphKind::RadioButton);
    }

    if (IDC_WZ_ATTACH_1..IDC_WZ_ATTACH_1 + 4).contains(&control) {
        return Some(theme::GlyphKind::CheckBox);
    }

    None
}

/// **Lays out the wizard** — one step at a time, everything else hidden.
///
/// The window's height does **not** change with the step (the mock-up asks for that), so the
/// three buttons at the bottom are placed from the bottom edge upwards and the step's own
/// controls from the top downwards. What is between them is air.
///
/// # Safety
///
/// As [`layout_letter`].
unsafe fn layout_wizard(hwnd: HWND, state: &WindowState) {
    let (Some(faces), Some(wizard)) = (state.fonts.as_ref(), state.wizard.as_deref()) else {
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
    let pad = metrics.x(air::PAD);
    let width = client.right - pad * 2;
    let tight = metrics.y(air::TIGHT);
    let gap = metrics.y(air::GAP);
    let button = metrics.y(air::BUTTON);
    let pitch = Some(faces.body_pitch());

    // Everything that does not belong to this step goes out of the way first: a slot of
    // another step left on the screen is the one way this window can show two things at once.
    // The table [`controls_of`] is what decides, and it is therefore load-bearing rather than
    // documentation — a slot missing from it is a slot that vanishes.
    let mine = controls_of(wizard.current());

    for slot in WIZARD_SLOTS
        .into_iter()
        .chain(WIZARD_SLOTS_TAIL)
        .chain(WIZARD_SLOTS_VALUES)
    {
        if !mine.contains(&slot) {
            hide(hwnd, slot);
        }
    }

    // --- the head: the step line, the progress bar, the heading and the note ---------------
    let mut y = metrics.y(air::TOP);

    // SAFETY: `dc` is the live DC and the faces belong to this window's state — the contract
    // of `measure`, held for every call in this function.
    let step_height = unsafe {
        measure(
            dc,
            faces.text,
            width,
            &settings::get_text(hwnd, IDC_WZ_STEP),
            None,
        )
    };

    place(hwnd, IDC_WZ_STEP, pad, y, width, step_height);
    y += step_height + tight;

    // The progress bar is a static this window paints: one filled run for the steps behind and
    // one quiet run for those ahead. Three dialog units tall, which is a line and not a bar.
    place(hwnd, IDC_WZ_PROGRESS, pad, y, width, metrics.y(3));
    y += metrics.y(3) + gap;

    // SAFETY: as above.
    let title_height = unsafe {
        measure(
            dc,
            faces.name,
            width,
            &settings::get_text(hwnd, IDC_WZ_TITLE),
            None,
        )
    };

    place(hwnd, IDC_WZ_TITLE, pad, y, width, title_height);
    y += title_height + tight;

    // SAFETY: as above.
    let note_height = unsafe {
        measure(
            dc,
            faces.body,
            width,
            &settings::get_text(hwnd, IDC_WZ_NOTE),
            pitch,
        )
    };

    place(hwnd, IDC_WZ_NOTE, pad, y, width, note_height);
    y += note_height + gap;

    // --- the step's own controls ------------------------------------------------------------
    let top_of_body = y;

    match wizard.current() {
        Step::What => {
            for index in 0..3 {
                // ⚠ **The two lines are not placed here and never were controls on the
                // screen** — see `card_lines` and `draw_card`. The statics that carry their
                // words are `NOT WS_VISIBLE`; what this loop needs of them is the height they
                // make the card, and that is the third answer of the shared arithmetic.
                //
                // SAFETY: as above.
                let (_, _, card_height) = unsafe {
                    card_lines(
                        dc,
                        faces,
                        metrics,
                        width,
                        &settings::get_text(hwnd, IDC_WZ_CARD_T1 + index),
                        &settings::get_text(hwnd, IDC_WZ_CARD_S1 + index),
                    )
                };

                place(hwnd, IDC_WZ_CARD_1 + index, pad, y, width, card_height);

                y += card_height + gap;
            }
        }

        Step::Where => {
            let label = settings::get_text(hwnd, IDC_WZ_PROGRAM_LABEL);
            // SAFETY: as above.
            let label_width = unsafe { text_width(dc, faces.text, &label) };
            // SAFETY: as above.
            let capture_width = unsafe {
                button_width(
                    dc,
                    faces.text,
                    metrics,
                    &settings::get_text(hwnd, IDC_WZ_CAPTURE),
                )
            };

            // ⭐ **МЕСТО ПОД КРОМКУ И ЗАЗОР — задача Т-45-3, решение 107.4.** Слово
            // пользователя: «„Поле ввода программы:“ слишком прижато к окну ввода, и часть окна
            // ввода не дорисована со стороны текста». Обе половины жалобы — одна причина.
            //
            // Прежде подпись занимала прямоугольник `текст + 3 единицы диалога`, а поле
            // начиналось СРАЗУ за ним. Коробку поля рисует фон окна на `left − толщина`, то
            // есть **внутри прямоугольника подписи**, а подпись — owner-draw статик: она
            // грунтует свой прямоугольник целиком и кромку стирает. Замер на живом 0.44.0
            // (`красное-рамка-мастер-e44.log`): пикселей кромки слева **4 из 19 = 21 %** при
            // 100 % справа и 100 % у обоих полей окна настроек; зазор «текст → кромка» — 4 px.
            //
            // Теперь прямоугольник подписи равен её тексту, а между текстом и полем
            // резервируется зазор макета плюс толщина кромки. Число зазора — [`LABEL_TO_FIELD`],
            // и оно названо пользователем.
            let dpi = theme::dc_dpi(dc);
            let thickness = theme::scaled(theme::BORDER_THICKNESS, dpi).max(1);
            let field_x = pad + label_width + theme::scaled(air::LABEL_TO_FIELD, dpi) + thickness;

            place(
                hwnd,
                IDC_WZ_PROGRAM_LABEL,
                pad,
                y + tight,
                label_width,
                step_height,
            );
            place_field(
                hwnd,
                metrics,
                IDC_WZ_PROGRAM,
                field_x,
                y,
                pad + width - capture_width - metrics.x(air::BUTTON_GAP) - field_x,
                button,
            );
            place(
                hwnd,
                IDC_WZ_CAPTURE,
                pad + width - capture_width,
                y,
                capture_width,
                button,
            );

            y += button + tight;

            // SAFETY: as above.
            let note = unsafe {
                measure(
                    dc,
                    faces.body,
                    width,
                    &settings::get_text(hwnd, IDC_WZ_CAPTURE_NOTE),
                    pitch,
                )
            };

            place(hwnd, IDC_WZ_CAPTURE_NOTE, pad, y, width, note);
            y += note + gap;

            for index in 0..3 {
                let control = IDC_WZ_FIELD_1 + index;

                place(hwnd, control, pad, y, width, glyph_height(metrics, faces));
                y += glyph_height(metrics, faces);

                // The second radio carries a sentence of its own under it.
                if index == 1 {
                    let inset = metrics.x(settings::GLYPH_TEXT_INSET_DLU);
                    // SAFETY: as above.
                    let sub = unsafe {
                        measure(
                            dc,
                            faces.body,
                            width - inset,
                            &settings::get_text(hwnd, IDC_WZ_FIELD_2_SUB),
                            pitch,
                        )
                    };

                    place(hwnd, IDC_WZ_FIELD_2_SUB, pad + inset, y, width - inset, sub);
                    y += sub;
                }

                y += tight;
            }
        }

        Step::Did => {
            place(hwnd, IDC_WZ_TYPED_LABEL, pad, y, width, step_height);
            y += step_height + tight;

            let combo = width / 2;

            place(hwnd, IDC_WZ_LAYOUT, pad, y, combo, metrics.y(12));

            let pressed = settings::get_text(hwnd, IDC_WZ_PRESSED_LABEL);
            // SAFETY: as above.
            let pressed_width = unsafe { text_width(dc, faces.text, &pressed) } + tight;
            let chip_x = pad + combo + metrics.x(air::BUTTON_GAP);

            place(
                hwnd,
                IDC_WZ_PRESSED_LABEL,
                chip_x,
                y + tight,
                pressed_width,
                step_height,
            );
            place(
                hwnd,
                IDC_WZ_CHIP,
                chip_x + pressed_width + tight,
                y,
                pad + width - (chip_x + pressed_width + tight),
                metrics.y(12),
            );

            y += metrics.y(12) + gap;

            for (label, field) in [
                (IDC_WZ_EXPECTED_LABEL, IDC_WZ_EXPECTED),
                (IDC_WZ_GOT_LABEL, IDC_WZ_GOT),
            ] {
                place(hwnd, label, pad, y, width, step_height);
                y += step_height + tight;
                place_field(hwnd, metrics, field, pad, y, width, button);
                y += button + gap;
            }

            place(hwnd, IDC_WZ_REPEAT_LABEL, pad, y, width, step_height);
            y += step_height + tight;

            let column = (width - metrics.x(air::BUTTON_GAP) * 2) / 3;

            for index in 0..3 {
                place(
                    hwnd,
                    IDC_WZ_REPEAT_1 + index,
                    pad + (column + metrics.x(air::BUTTON_GAP)) * index,
                    y,
                    column,
                    glyph_height(metrics, faces),
                );
            }
        }

        Step::Idea => {
            let room = (bottom_of_body(client, metrics) - y - step_height * 2 - tight * 2 - gap)
                .max(metrics.y(30));
            let each = room / 2;

            for (label, field) in [
                (IDC_WZ_IDEA_LABEL, IDC_WZ_IDEA),
                (IDC_WZ_HELPS_LABEL, IDC_WZ_HELPS),
            ] {
                place(hwnd, label, pad, y, width, step_height);
                y += step_height + tight;
                place(hwnd, field, pad, y, width, each);
                inset_area(hwnd, metrics, field);
                y += each + gap;
            }
        }

        Step::Attach => {
            let inset = metrics.x(settings::GLYPH_TEXT_INSET_DLU);

            for index in 0..4 {
                place(
                    hwnd,
                    IDC_WZ_ATTACH_1 + index,
                    pad,
                    y,
                    width,
                    glyph_height(metrics, faces),
                );
                y += glyph_height(metrics, faces);

                // SAFETY: as above.
                let value = unsafe {
                    measure(
                        dc,
                        faces.body,
                        width - inset,
                        &settings::get_text(hwnd, IDC_WZ_ATTACH_V1 + index),
                        pitch,
                    )
                };

                place(
                    hwnd,
                    IDC_WZ_ATTACH_V1 + index,
                    pad + inset,
                    y,
                    width - inset,
                    value,
                );
                y += value + tight;
            }

            y += gap - tight;

            // SAFETY: as above.
            let foot = unsafe {
                measure(
                    dc,
                    faces.body,
                    width,
                    &settings::get_text(hwnd, IDC_WZ_ATTACH_FOOT),
                    pitch,
                )
            };

            // ⚠ **Clamped to the body, and the instrument is what asked for it.** In Greek —
            // the longest of the fourteen — the four values and this footnote together stand
            // taller than the window, and the sentence ran straight through the button row:
            // `влезание-В.log`, three overlaps of 1345 with 1350…1352. The room that is left
            // is what it gets; a sentence cut short is visible, a sentence over the buttons is
            // a window nobody can press.
            let room = (bottom_of_body(client, metrics) - y).max(0);

            place(hwnd, IDC_WZ_ATTACH_FOOT, pad, y, width, foot.min(room));
        }

        Step::Preview => {
            // SAFETY: as above.
            let copy_width = unsafe {
                button_width(
                    dc,
                    faces.text,
                    metrics,
                    &settings::get_text(hwnd, IDC_WZ_COPY),
                )
            };
            // SAFETY: as above.
            let save_width = unsafe {
                button_width(
                    dc,
                    faces.text,
                    metrics,
                    &settings::get_text(hwnd, IDC_WZ_SAVE),
                )
            };

            // ⚠ **The two buttons wrap when they do not fit side by side**, and that is not a
            // precaution: in Greek «Αποθήκευση στον φάκελο του ημερολογίου» took the second
            // button 86 px past the right edge of the window (`влезание-В.log`, control 1348
            // «вне клиента»). The two rows cost the preview one button of height and nothing
            // else.
            let side_by_side = copy_width + metrics.x(air::BUTTON_GAP) + save_width <= width;
            let rows = if side_by_side { 1 } else { 2 };

            let bottom = bottom_of_body(client, metrics);
            let room =
                (bottom - y - button * rows - step_height - tight * (rows + 1)).max(metrics.y(40));

            place(hwnd, IDC_WZ_PREVIEW, pad, y, width, room);
            inset_area(hwnd, metrics, IDC_WZ_PREVIEW);
            y += room + tight;

            place(hwnd, IDC_WZ_COPY, pad, y, copy_width, button);

            if side_by_side {
                place(
                    hwnd,
                    IDC_WZ_SAVE,
                    pad + copy_width + metrics.x(air::BUTTON_GAP),
                    y,
                    save_width,
                    button,
                );
            } else {
                y += button + tight;
                place(hwnd, IDC_WZ_SAVE, pad, y, save_width.min(width), button);
            }

            y += button + tight;
            place(hwnd, IDC_WZ_STATUS, pad, y, width, step_height);
        }

        // «Готово» — задача Т-33-3. The paragraph gets the height its own words need, and the
        // one button stands a block under it: the window keeps the size every other step has,
        // and the air below is air.
        Step::Done => {
            // SAFETY: as above.
            let thanks = unsafe {
                measure(
                    dc,
                    faces.body,
                    width,
                    &settings::get_text(hwnd, IDC_WZ_THANKS),
                    pitch,
                )
            };

            let room = (bottom_of_body(client, metrics) - y - button - gap).max(0);

            place(hwnd, IDC_WZ_THANKS, pad, y, width, thanks.min(room));
            y += thanks.min(room) + gap;

            // SAFETY: as above.
            let channel_width = unsafe {
                button_width(
                    dc,
                    faces.text,
                    metrics,
                    &settings::get_text(hwnd, IDC_WZ_CHANNEL),
                )
            };

            place(
                hwnd,
                IDC_WZ_CHANNEL,
                pad,
                y,
                channel_width.min(width),
                button,
            );

            // ⚠ **The address is still a placeholder (П7), so the button is drawn and dead** —
            // the same rule «От автора» keeps for the same address.
            enable(
                hwnd,
                IDC_WZ_CHANNEL,
                !links::is_placeholder(links::CHANNEL_URL),
            );
        }
    }

    let _ = top_of_body;

    // --- the three buttons, from the bottom edge upwards --------------------------------
    let row_y = client.bottom - metrics.y(air::PAD) - button;

    // SAFETY: as above.
    let cancel_width = unsafe {
        button_width(
            dc,
            faces.text,
            metrics,
            &settings::get_text(hwnd, IDC_WZ_CANCEL),
        )
    };
    // SAFETY: as above.
    let back_width = unsafe {
        button_width(
            dc,
            faces.text,
            metrics,
            &settings::get_text(hwnd, IDC_WZ_BACK),
        )
    };
    // SAFETY: as above.
    let next_width = unsafe {
        button_width(
            dc,
            faces.text,
            metrics,
            &settings::get_text(hwnd, IDC_WZ_NEXT),
        )
    };

    place(
        hwnd,
        IDC_WZ_NEXT,
        pad + width - next_width,
        row_y,
        next_width,
        button,
    );

    // ⛔ **Решается ДО размещения, а не после — задача Т-33-8, находка глазом пользователя.**
    // «Назад» на первом шаге нет вовсе, и раньше её сперва ставили (а `place` показывает
    // контрол), и только следующей строкой прятали: кнопка успевала мелькнуть. Замерено —
    // семь кадров из восьмидесяти на первом шаге показывали её прямоугольник непустым
    // (`scratchpad-Э33\моргание-шагов-до.log`). Показанный и тут же спрятанный контрол — это
    // всегда мелькание, сколько бы миллисекунд ни прошло между двумя вызовами.
    //
    // ⚠ Место «Назад» держит по-прежнему: она не двигается, она отсутствует — макет просит
    // именно этого, и кнопка, которая переезжала бы, двигала соседнюю.
    let back_shown = wizard.step > 0 && !wizard.done;
    let cancel_shown = !wizard.done;

    if back_shown {
        place(
            hwnd,
            IDC_WZ_BACK,
            pad + width - next_width - metrics.x(air::BUTTON_GAP) - back_width,
            row_y,
            back_width,
            button,
        );
    } else {
        hide(hwnd, IDC_WZ_BACK);
    }

    // On «Готово» there is nothing to cancel and nowhere to go back to: both leave, «Закрыть»
    // stays where «Далее» stood, and no button moves a pixel (задача Т-33-3, решение 103.3).
    if cancel_shown {
        place(hwnd, IDC_WZ_CANCEL, pad, row_y, cancel_width, button);
    } else {
        hide(hwnd, IDC_WZ_CANCEL);
    }

    // SAFETY: releases exactly the DC taken above, once.
    unsafe { ReleaseDC(Some(hwnd), dc) };
}

/// Answers `WM_MEASUREITEM` for the one owner-drawn combo of these windows.
///
/// # SEC-05
///
/// The type and the identifier are read out of the message and checked before anything is
/// written, and the only field written back is the height. A forged message names a control
/// that is not the combo and is refused.
///
/// # Safety
///
/// Called from [`letter_proc`] with the `lparam` of the message: the sender owns the struct it
/// names for the length of the send.
unsafe fn on_measure_item(hwnd: HWND, lparam: LPARAM) -> isize {
    use windows::Win32::UI::Controls::MEASUREITEMSTRUCT;

    if lparam.0 == 0 {
        return 0;
    }

    // SAFETY: the dialog manager owns the struct for the length of the send.
    let item = unsafe { &mut *(lparam.0 as *mut MEASUREITEMSTRUCT) };

    if item.CtlType != ODT_COMBOBOX || i32::try_from(item.CtlID).unwrap_or(-1) != IDC_WZ_LAYOUT {
        return 0;
    }

    // ⭐ **Задача Т-45-3: высоту строки списка считает общий слой.** Здесь стояло
    // `faces.body_height.abs() + 8` — своё число, без масштаба DPI и без связи с окном
    // настроек. Замер на живом 0.44.0 (`красное-высоты-комбо-e44.log`): строка списка мастера
    // **16 px**, строка списка каждого из четырёх комбо окна настроек — **26 px**. Теперь
    // формула одна на все окна: высота шрифта диалога плюс `LIST_ITEM_EXTRA` через масштаб.
    //
    // ⚠ Закрытую часть это сообщение не задаёт и задавать не должно: одно `WM_MEASUREITEM`
    // красит обе высоты сразу, поэтому закрытую двигает `CB_SETITEMHEIGHT(−1)` из
    // `widgets::combo::attach`.
    let height = widgets::combo::row_height(hwnd, widgets::combo::LIST_ITEM_EXTRA);

    let Some(height) = height else {
        return 0;
    };

    let Ok(height) = u32::try_from(height) else {
        return 0;
    };

    item.itemHeight = height;

    // TRUE — measured.
    1
}

/// Draws one row of the layout combo — the closed face and every item of the dropped list.
///
/// A body of its own and not the settings window's: that one reads its colours out of a
/// `DialogState` this module has no access to. What is shared is the vocabulary — the same
/// palette fields, the same `paint_label`.
///
/// # Safety
///
/// As [`draw_progress`].
unsafe fn draw_combo_row(
    hwnd: HWND,
    dc: HDC,
    rect: RECT,
    state: &WindowState,
    item: u32,
    item_state: u32,
) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::{CB_GETLBTEXT, CB_GETLBTEXTLEN};

    let (Some(brushes), Some(faces)) = (state.brushes.as_ref(), state.fonts.as_ref()) else {
        return 0;
    };

    let palette = state.palette;
    let selected = item_state & ODS_SELECTED.0 != 0;

    // ⚠ An item identifier of `u32::MAX` means «the list is empty» — the documented value, and
    // the one moment this can be asked before anything has been added.
    if item == u32::MAX {
        // SAFETY: `dc` is the DC of the message and the brush belongs to the state.
        unsafe { FillRect(dc, &rect, brushes.field_bg()) };
        return 1;
    }

    let index = usize::try_from(item).unwrap_or(0);
    let length = settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_GETLBTEXTLEN, index, 0);
    let mut buffer = vec![0u16; usize::try_from(length).unwrap_or(0) + 1];

    // SAFETY: `buffer` is a live allocation of this frame long enough for the string the
    // control has just reported, plus its terminator — `CBS_HASSTRINGS` is what keeps the
    // strings in the control at all.
    settings::send_to(
        hwnd,
        IDC_WZ_LAYOUT,
        CB_GETLBTEXT,
        index,
        buffer.as_mut_ptr() as isize,
    );

    let end = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    let mut caption = buffer[..end].to_vec();

    // ⭐ **Строка рисуется телом окна настроек — задача Т-46-4, решение 109.4.**
    //
    // Здесь стоял `theme::paint_label`, и он верен для подписей: его формат — `DT_TOP` с
    // переносом, потому что подпись бывает многострочной. Строке списка это не подходит, и
    // замер Т-46-1 сказал числом: в строке 24 px слово стояло с воздухом **3 сверху и 9
    // снизу**, а в окне настроек — **8 и 5**, то есть отцентровано (`DT_VCENTER`). Глазу это
    // видно, и пользователь это увидел. Эталон механики — окно настроек (решение 109.1).
    //
    // Земля остаётся за окном: у выбранной строки она своя, и роли красок этих двух окон
    // разные. Слой красок не знает — они приходят разрешёнными.
    let (ground, ink) = if selected {
        (brushes.sel_bg(), palette.sel_fg)
    } else {
        (brushes.field_bg(), palette.text)
    };

    // SAFETY: `dc` is the DC of the message and `ground` is a live brush of this window's state.
    unsafe { FillRect(dc, &rect, ground) };

    // SAFETY: `dc` is live for the length of the message; `faces.text` is a font this window's
    // state owns for longer than the call; `caption` is a live local of this frame.
    unsafe {
        widgets::combo::draw_row(
            dc,
            rect,
            &mut caption,
            ink,
            Some(faces.text),
            // Тот же отступ, что был: три единицы диалога окна настроек — это ≈5 px, и шесть
            // пикселей макета через масштаб дают столько же. Число не трогается (решение
            // 109.9: палитра и строки списка раскладок — вне этапа).
            theme::scaled(6, theme::dc_dpi(dc)),
        );
    }

    1
}

/// Draws the progress line of the wizard — the steps behind in the accent, the steps ahead in
/// the quiet border colour.
///
/// # Safety
///
/// As [`draw_demo`]: `dc` and `rect` are the values of the message.
unsafe fn draw_progress(dc: HDC, rect: RECT, state: &WindowState) -> isize {
    let (Some(brushes), Some(wizard)) = (state.brushes.as_ref(), state.wizard.as_deref()) else {
        return 0;
    };

    // SAFETY: `dc` is the DC of the message and the brush belongs to this window's state.
    unsafe { FillRect(dc, &rect, brushes.window_bg()) };

    let steps = i32::try_from(wizard.road().len()).unwrap_or(1).max(1);

    // On «Готово» the line is full and keeps the number of cells the road had: the thanks is
    // where the road ends, not a sixth cell in a row of five (задача Т-33-3).
    let done = if wizard.done {
        steps
    } else {
        i32::try_from(wizard.step + 1).unwrap_or(1).clamp(1, steps)
    };
    let width = rect.right - rect.left;
    let gap = theme::scaled(4, theme::dc_dpi(dc));
    let each = (width - gap * (steps - 1)) / steps;

    for index in 0..steps {
        let left = rect.left + (each + gap) * index;
        let cell = RECT {
            left,
            top: rect.top,
            right: left + each,
            bottom: rect.bottom,
        };

        // SAFETY: as above — the two brushes belong to the state.
        unsafe {
            FillRect(
                dc,
                &cell,
                if index < done {
                    brushes.accent_bg()
                } else {
                    brushes.box_border()
                },
            )
        };
    }

    1
}

/// Draws the key chip of «Что вы делали?» — the same figure the help rows wear, alone in its
/// own rectangle.
///
/// # Safety
///
/// As [`draw_progress`].
unsafe fn draw_chip(dc: HDC, rect: RECT, state: &WindowState, key: &str) -> isize {
    let (Some(brushes), Some(faces)) = (state.brushes.as_ref(), state.fonts.as_ref()) else {
        return 0;
    };

    let palette = state.palette;
    let dpi = theme::dc_dpi(dc);

    // SAFETY: `dc` is the DC of the message and the brush belongs to this window's state.
    unsafe { FillRect(dc, &rect, brushes.window_bg()) };

    // SAFETY: as above; every handle of the style is an object the state owns for longer.
    unsafe {
        theme::paint_chip_row(
            dc,
            rect,
            theme::chip_row(theme::KEY_PLACEHOLDER, key),
            theme::ChipRowStyle {
                ground: brushes.window_bg(),
                ink: palette.text,
                body: Some(faces.body),
                chip_face: Some((faces.chip, faces.chip_height)),
                chip: theme::ChipColors {
                    outline: palette.field_border,
                    fill: brushes.field_bg(),
                    ink: palette.text,
                },
                pitch: faces.body_pitch(),
                dpi,
            },
        )
    }
}

/// Where the two lines of a card stand **inside it**, and how tall the card therefore is.
///
/// **The one place this arithmetic lives.** [`layout_wizard`] asks it for the height and
/// [`draw_card`] asks it for the two rectangles; a second copy of the formula would be a card
/// whose words drift out of the block the moment either half is touched.
///
/// The rectangles are in the card's own coordinates — origin at its top-left corner — which is
/// exactly the space `WM_DRAWITEM` hands the drawing, and which is the same space in a mirrored
/// window as in an ordinary one (ИТОГ-Э30 §9.7: an offset is safe where an absolute corner is
/// not).
///
/// # Safety
///
/// `dc` is a live DC of this window and `faces` the fonts it owns — the contract of
/// [`measure`].
unsafe fn card_lines(
    dc: HDC,
    faces: &Faces,
    metrics: Metrics,
    width: i32,
    title: &str,
    sub: &str,
) -> (RECT, RECT, i32) {
    let pad = metrics.x(air::PANEL_PAD);
    let tight = metrics.y(air::TIGHT);
    let inner = width - pad * 2;

    // SAFETY: see the contract.
    let title_height = unsafe { measure(dc, faces.text, inner, title, None) };
    // SAFETY: see the contract.
    let sub_height = unsafe { measure(dc, faces.body, inner, sub, Some(faces.body_pitch())) };

    let title_rect = RECT {
        left: pad,
        top: tight,
        right: pad + inner,
        bottom: tight + title_height,
    };
    let sub_rect = RECT {
        left: pad,
        top: tight + title_height + tight,
        right: pad + inner,
        bottom: tight + title_height + tight + sub_height,
    };

    (title_rect, sub_rect, tight * 3 + title_height + sub_height)
}

/// Draws one card of «Что случилось?» — a block with a heading and a line under it, and the
/// chosen one framed in the accent.
///
/// # ⛔ Why the card draws its own words — задача Т-33-2, решение 103.2
///
/// The two lines used to be **visible statics standing on the card**, and hovering the card
/// wiped them off the screen. Measured rather than reasoned
/// (`scratchpad-Э33\причина-2-наведение.log`):
///
/// * in the z-order of the dialog the three cards stand **above** their own six labels — the
///   template lists the buttons first, and the order of a template *is* the z-order;
/// * **not one control of this window carries `WS_CLIPSIBLINGS`** (every style read off the
///   live window is `0x5001400B` or `0x5002000D`);
/// * so at a full repaint the children paint from the top of the z-order downwards: the card
///   fills its rectangle and the labels, being **below** it, paint afterwards and land on top.
///   The words were visible only because they were painted **last**;
/// * on hover `settings::invalidate_hot` invalidates **the button alone**. It fills its whole
///   rectangle again and nothing ever asks the labels to repaint. Ink inside a card measured
///   3030 → **0**.
///
/// So the card is one body of drawing now: the statics are `NOT WS_VISIBLE` carriers of the
/// two strings — the arrangement the panels of a letter and «Как пользоваться» have always
/// used — and everything inside this rectangle is painted here, in one pass, by one owner.
///
/// # Safety
///
/// As [`draw_progress`]; `hwnd` is the live window the item belongs to.
unsafe fn draw_card(
    hwnd: HWND,
    dc: HDC,
    rect: RECT,
    state: &WindowState,
    control: i32,
    hot: bool,
    focused: bool,
) -> isize {
    let (Some(brushes), Some(faces), Some(wizard)) = (
        state.brushes.as_ref(),
        state.fonts.as_ref(),
        state.wizard.as_deref(),
    ) else {
        return 0;
    };

    let palette = state.palette;
    let dpi = theme::dc_dpi(dc);
    let chosen = wizard_is_checked(wizard, control);

    // ⚠ **Один кадр вместо череды — задача Т-33-5, находка глазом пользователя.** Тело ниже
    // кладёт заливку, потом рамку, потом две строки, и, пока оно писало прямо в DC сообщения,
    // DWM успевал снять окно между заливкой и текстом: карточка **моргала** при наведении.
    // Замерено на живом продукте, а не выведено — очередь кадров с качающимся указателем,
    // чернила текста 1852 → **0** → 1852 в **21 кадре из 160** (`scratchpad-Э33\моргание-до.log`;
    // положительный контроль, указатель вне окна, — ровный ряд 1852). Это тот же дефект, ради
    // которого задача T-15-1 сделала [`theme::PaintBuffer`] для кнопок настроек, и лечится он
    // теми же тремя строками: поверхность своего размера, тело пишет в неё, готовый элемент
    // переходит на экран одним `BitBlt`.
    //
    // `None` — GDI отказал (NFR-13): `map_or` тогда отдаёт телу DC сообщения, и карточка
    // рисуется ровно как до этой задачи — с морганием, то есть ухудшенно, но живо.
    //
    // ⚠ DPI берётся **у DC сообщения**, а не у поверхности: `GetDeviceCaps` буфера отвечает про
    // память, и радиус, посчитанный по нему, был бы неверен на любом масштабе кроме 100 %.
    //
    // SAFETY: `dc` — DC сообщения, живой на время посылки; буфер читает его глубину цвета и
    // освобождает свои DC и растр, когда кончится этот кадр.
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // ⛔⛔ **Заливка ГРУНТУЕТ буфер, и без неё углы карточки становятся чёрными.** Находка
    // глазом пользователя на светлой палитре 0.43.0: `paint_rounded` держит четыре угловых
    // плитки вне отсечения и потом смешивает дугу **с той подложкой, которую читает из того же
    // DC**, — а пиксели свежего `CreateCompatibleBitmap` не определены. На тёмной палитре
    // неопределённое чёрное совпало с фоном и никто не заметил; на светлой углы вышли чёрными.
    // Ровно об этом предупреждает `theme::PaintBuffer`: «эта заливка — то, чем грунтуется
    // буфер», и ровно эту строку носит `settings::paint_push_button` с задачи T-15-1. Здесь она
    // была пропущена, потому что `paint_rounded` заливает **середину** сама и выглядела
    // самодостаточной.
    //
    // ⚠ Грунтуется **фоном окна, а не лицом карточки**: за срезанными углами стоит то, на чём
    // карточка лежит, и до буфера там оставалось ровно это — то, что нарисовал `WM_ERASEBKGND`
    // окна. Заливка панельной кистью закрасила бы углы лицом карточки, и скругление в крайнем
    // пикселе пропало бы: замерено, угол читался 0xFFFFFF при подложке 0xEDEFF2. Это та же
    // кисть, которой `settings::paint_push_button` грунтует свой буфер, — «та самая, которой
    // отвечает сообщение».
    //
    // SAFETY: `target` — буфер этого кадра или, при отказе GDI, DC сообщения; `rect` — значение
    // сообщения, кисть принадлежит состоянию окна дольше, чем длится вызов.
    unsafe { FillRect(target, &rect, brushes.window_bg()) };

    // ⚠ The **fill** never changes: the ground of a card is the panel brush whatever the
    // pointer is doing, because the words above it are drawn on that ground. What answers to
    // the pointer and to the focus is the frame.
    theme::paint_rounded(
        target,
        &rect,
        theme::scaled(theme::CORNER_RADIUS, dpi),
        if chosen {
            palette.accent_bg
        } else if hot || focused {
            palette.box_border
        } else {
            palette.panel_border
        },
        brushes.panel_bg(),
        dpi,
    );

    let index = control - IDC_WZ_CARD_1;
    let title = settings::get_text(hwnd, IDC_WZ_CARD_T1 + index);
    let sub = settings::get_text(hwnd, IDC_WZ_CARD_S1 + index);

    // ⚠ Мерится по **DC сообщения**, а не по буферу: `measure` выбирает начертание в тот DC,
    // которому его дали, и обе поверхности несут одно и то же начертание — но правило «у окна
    // спрашивают про окно» стоит держать в одном месте, а не вспоминать.
    //
    // SAFETY: `dc` is the DC of the message and `faces` the fonts of this window's state.
    let (title_rect, sub_rect, _) = unsafe {
        card_lines(
            dc,
            faces,
            Metrics::of(hwnd),
            rect.right - rect.left,
            &title,
            &sub,
        )
    };

    for (line, box_of, role, face, pitch) in [
        (
            &title,
            title_rect,
            label_color_role(IDC_WZ_CARD_T1),
            faces.text,
            None,
        ),
        (
            &sub,
            sub_rect,
            label_color_role(IDC_WZ_CARD_S1),
            faces.body,
            Some(faces.body_pitch()),
        ),
    ] {
        let mut wide: Vec<u16> = line.encode_utf16().collect();

        // The rectangle comes back in the card's own coordinates; `rcItem` is (0,0)-based for
        // a button, and the shift is written out anyway so that the two spaces are never
        // silently assumed to be the same one.
        let placed = RECT {
            left: rect.left + box_of.left,
            top: rect.top + box_of.top,
            right: rect.left + box_of.right,
            bottom: rect.top + box_of.bottom,
        };

        // SAFETY: `target` is the buffer of this frame or, on a refusal, the DC of the message;
        // `placed` is live, and the brush and the face belong to this window's state for longer
        // than the call.
        unsafe {
            theme::paint_label_at_pitch(
                target,
                placed,
                &mut wide,
                theme::LabelStyle {
                    ground: brushes.panel_bg(),
                    ink: theme::label_ink(role, palette),
                    face: Some(face),
                    pitch,
                    reading: theme::Reading::Native,
                },
            );
        }
    }

    // The one moment any of this becomes visible. A refused blit leaves the card showing the
    // picture the window already had there — its previous state and never a hole; painting the
    // body a second time into the DC of the message would be the very flicker this removes.
    //
    // SAFETY: `dc` is the DC of the message; `buffer` is this frame's own and is freed as it
    // goes out of scope on this line.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
    }

    1
}

/// Draws one radio or one check box of the wizard — the glyph and the words beside it.
///
/// The twin of [`draw_switch`], and it is a second body rather than a parameter of the first
/// for one reason: the switch reads its state out of [`AuthorView`] and this reads it out of
/// the wizard's draft. Everything below the answer to «checked?» is the same table.
///
/// # Safety
///
/// As [`draw_progress`].
unsafe fn draw_wizard_glyph(
    dc: HDC,
    rect: RECT,
    state: &WindowState,
    kind: theme::GlyphKind,
    control: i32,
    disabled: bool,
    label: &str,
) -> isize {
    let (Some(brushes), Some(faces), Some(wizard)) = (
        state.brushes.as_ref(),
        state.fonts.as_ref(),
        state.wizard.as_deref(),
    ) else {
        return 0;
    };

    let palette = state.palette;
    let dpi = theme::dc_dpi(dc);
    let checked = wizard_is_checked(wizard, control);
    let colors = theme::glyph_color_roles(kind, checked, disabled);

    // ⚠ **Один кадр вместо череды — задача Т-33-5.** То же, что у [`draw_card`], и по той же
    // измеренной причине: заливка, глиф и подпись писались прямо в DC сообщения, и радиокнопка
    // **моргала** под указателем — чернила 738 → **0** → 738 в 13 кадрах из 120 и 474 → **0** →
    // 474 в 25 из 120 (`scratchpad-Э33\моргание-до.log`). Кнопки этого окна не моргали никогда
    // именно потому, что `settings::paint_push_button` носит эту поверхность с задачи T-15-1.
    //
    // SAFETY: как в [`draw_card`].
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // SAFETY: `target` is the buffer of this frame or the DC of the message, and the brush
    // belongs to this window's state.
    unsafe { FillRect(target, &rect, brushes.window_bg()) };

    // ⚠ Задача Т-45-2: та же клетка, что у окна настроек, и теперь одним телом
    // (`widgets::glyph::cell`). Всё остальное здесь у двух окон различается — см. доктекст
    // `widgets::glyph`.
    let cell = widgets::glyph::cell(rect, dpi);

    let fill = match colors.fill {
        theme::GlyphFillRole::FieldBg => brushes.field_bg(),
        theme::GlyphFillRole::AccentBg => brushes.accent_bg(),
    };

    let outline = match colors.frame {
        Some(theme::GlyphFrameRole::BoxBorder) => palette.box_border,
        None => match colors.fill {
            theme::GlyphFillRole::FieldBg => palette.field_bg,
            theme::GlyphFillRole::AccentBg => palette.accent_bg,
        },
    };

    match kind {
        theme::GlyphKind::CheckBox => {
            theme::paint_rounded(
                target,
                &cell,
                theme::scaled(settings::GLYPH_CORNER_RADIUS, dpi),
                outline,
                fill,
                dpi,
            );

            if let Some(mark) = colors.mark {
                theme::draw_check_mark(
                    target,
                    &cell,
                    glyph_ink(mark, palette),
                    settings::GLYPH_CHECK_MARK,
                    dpi,
                );
            }
        }
        theme::GlyphKind::RadioButton => {
            theme::paint_ellipse(target, &cell, Some(outline), fill, dpi);

            if colors.mark.is_some() {
                let inset = theme::scaled_tenths_offset(settings::GLYPH_DOT_INSET_TENTHS, dpi);
                let dot = RECT {
                    left: cell.left + inset,
                    top: cell.top + inset,
                    right: cell.right - inset,
                    bottom: cell.bottom - inset,
                };

                theme::paint_ellipse(target, &dot, None, brushes.accent_bg(), dpi);
            }
        }
    }

    let text = RECT {
        left: cell.right + theme::scaled(settings::LIST_CHECK_TEXT_GAP, dpi),
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };

    let mut caption: Vec<u16> = label.encode_utf16().collect();

    // SAFETY: `target` and `text` are live; the face belongs to this window's state.
    let drawn = unsafe {
        theme::paint_label(
            target,
            text,
            &mut caption,
            theme::LabelStyle {
                ground: brushes.window_bg(),
                ink: match colors.text {
                    theme::GlyphTextRole::Text => palette.text,
                    theme::GlyphTextRole::TextMuted => palette.text_muted,
                },
                face: Some(faces.text),
                pitch: None,
                reading: theme::Reading::Native,
            },
        )
    };

    // The one moment any of this becomes visible — see the head of this body.
    //
    // SAFETY: `dc` is the DC of the message; `buffer` is this frame's own and is freed as it
    // goes out of scope on this line.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
    }

    drawn
}

/// **What a press does in the wizard** — FR-104, and the answer is whether it was handled here.
///
/// # ⛔ Задача Т-46-2, решение 109.1: КОД УВЕДОМЛЕНИЯ ЧИТАЕТСЯ, и это ремонт находки пользователя
///
/// До Э46 эта функция брала из `WM_COMMAND` только идентификатор, а код уведомления не читала
/// вовсе. Все глифы шаблона `IDD_WIZARD` — `BS_OWNERDRAW | BS_NOTIFY`, поэтому на один
/// настоящий щелчок мышью приходит **очередь из трёх уведомлений**, и замер Т-46-1 её напечатал
/// (`scratchpad-Э46\красное-галки-e45.log`): `1337/BN_KILLFOCUS`, `1338/BN_SETFOCUS`,
/// `1338/BN_CLICKED`. Отвечая на каждое, окно переключало соседку и переключало свою **дважды**
/// — 36 несработавших щелчков и 35 сдвинутых соседок из 44. Слова пользователя: «то не ставятся
/// с первого раза, то не снимаются с первого раза».
///
/// Теперь решает [`widgets::glyph::answer`] — **то же тело, что у окна настроек** (эталон
/// механики, решение 109.1). Галка отвечает на щелчок, радио — ещё на приход фокуса от стрелки,
/// карточка и кнопка — только на щелчок, комбобокс — только на `CBN_SELCHANGE`.
fn wizard_command(hwnd: HWND, control: i32, notification: u16) -> bool {
    use crate::widgets::glyph::{Answer, Kind, answer};
    use report::{Field, Repeat, Trouble};
    use windows::Win32::UI::WindowsAndMessaging::{CB_GETCURSEL, CBN_SELCHANGE};

    // Кнопки шаблона `BS_NOTIFY` не несут и шлют один `BN_CLICKED`, но правило одно на все
    // элементы окна и здесь: то, что за кнопкой написано, делает щелчок и ничего кроме.
    let pressed = answer(Kind::Button, notification) == Answer::Act;

    // The three that do not touch the draft.
    match control {
        IDC_WZ_CANCEL if pressed => {
            close_window(hwnd);
            return true;
        }
        IDC_WZ_NEXT if pressed => {
            wizard_forward(hwnd);
            return true;
        }
        IDC_WZ_BACK if pressed => {
            // ⭐ **Черновик собирается и на шаге НАЗАД — задача Т-46-3, решение 109.3.** Поля
            // есть правда, пока шаг на экране; уходя с него в любую сторону, эту правду надо
            // забрать, иначе набранное потеряется. Вперёд это делал `wizard_forward` с самого
            // начала, назад — не делал никто.
            wizard_harvest(hwnd);

            // SAFETY: the state pointer is live for the length of the window.
            unsafe {
                with_state(hwnd, |state| {
                    if let Some(wizard) = state.wizard.as_deref_mut() {
                        wizard.step = wizard.step.saturating_sub(1);
                        wizard.status.clear();
                    }
                })
            };

            wizard_refresh(hwnd);
            return true;
        }
        IDC_WZ_CAPTURE => {
            wizard_start_capture(hwnd);
            return true;
        }
        IDC_WZ_COPY => {
            wizard_copy(hwnd);
            return true;
        }
        IDC_WZ_SAVE => {
            wizard_save(hwnd);
            return true;
        }
        // «Открыть канал» on the thanks — задача Т-33-3. Dead while the address is a
        // placeholder, and the button is disabled to say so (П7).
        IDC_WZ_CHANNEL => {
            open_link(hwnd, links::CHANNEL_URL);
            return true;
        }
        _ => {}
    }

    // Выбор внутри шага. **Что именно считать нажатием, решает слой** (`widgets::glyph::answer`,
    // решение 109.1) — и по роду элемента: карточка отвечает щелчку, радио ещё и приходу фокуса
    // от стрелки, галка щелчку, список — только `CBN_SELCHANGE`.
    let chosen = settings::send_to(hwnd, IDC_WZ_LAYOUT, CB_GETCURSEL, 0, 0);

    let card_pressed = pressed;
    let radio_chosen = answer(Kind::Radio, notification) == Answer::Choose;
    let check_toggled = answer(Kind::Check, notification) == Answer::Toggle;
    let list_changed = u32::from(notification) == CBN_SELCHANGE;

    // SAFETY: as above.
    let touched = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref_mut()?;

            let touched = match control {
                c if (IDC_WZ_CARD_1..=IDC_WZ_CARD_1 + 2).contains(&c) && card_pressed => {
                    // Прежняя карточка нужна, чтобы перерисовать ровно две: ту, что теряет
                    // выбор, и ту, что его получает.
                    let was = IDC_WZ_CARD_1
                        + match wizard.draft.trouble {
                            Trouble::WrongResult => 0,
                            Trouble::NothingHappened => 1,
                            Trouble::Idea => 2,
                        };

                    wizard.draft.trouble = match control - IDC_WZ_CARD_1 {
                        0 => Trouble::WrongResult,
                        1 => Trouble::NothingHappened,
                        _ => Trouble::Idea,
                    };

                    Touched::Cards(was, control)
                }

                c if (IDC_WZ_FIELD_1..=IDC_WZ_FIELD_1 + 2).contains(&c) && radio_chosen => {
                    wizard.draft.field = match control - IDC_WZ_FIELD_1 {
                        0 => Field::Normal,
                        1 => Field::Password,
                        _ => Field::Unknown,
                    };

                    Touched::Range(IDC_WZ_FIELD_1, IDC_WZ_FIELD_1 + 2)
                }

                c if (IDC_WZ_REPEAT_1..=IDC_WZ_REPEAT_1 + 2).contains(&c) && radio_chosen => {
                    wizard.draft.repeat = match control - IDC_WZ_REPEAT_1 {
                        0 => Repeat::Always,
                        1 => Repeat::Sometimes,
                        _ => Repeat::Once,
                    };

                    Touched::Range(IDC_WZ_REPEAT_1, IDC_WZ_REPEAT_1 + 2)
                }

                c if (IDC_WZ_ATTACH_1..=IDC_WZ_ATTACH_1 + 3).contains(&c) && check_toggled => {
                    let attach = &mut wizard.draft.attach;

                    match control - IDC_WZ_ATTACH_1 {
                        0 => attach.machine = !attach.machine,
                        1 => attach.layouts = !attach.layouts,
                        2 => attach.settings = !attach.settings,
                        _ => attach.journal = !attach.journal,
                    }

                    Touched::Glyph(control)
                }

                IDC_WZ_LAYOUT if list_changed => {
                    let index = usize::try_from(chosen).ok()?;
                    wizard.draft.layout = wizard.layouts.get(index)?.clone();

                    // Комбобокс перерисовывает закрытую часть сам, выбирая строку.
                    Touched::Nothing
                }

                _ => return None,
            };

            Some(touched)
        })
    }
    .flatten();

    let Some(touched) = touched else {
        return false;
    };

    // ⭐ **ЗДЕСЬ И БЫЛО МОРГАНИЕ — задача Т-45-3, решение 107.5.** До Э45 эта строка звала
    // `wizard_refresh`, то есть на КАЖДЫЙ щелчок по карточке, радио или галке гасила окно
    // `WM_SETREDRAW`, переставляла все контролы шага и стирала окно целиком
    // `RedrawWindow(RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW)`. Комментарий рядом честно
    // говорил, почему так: «Rebuilding always is the cheaper rule to hold».
    //
    // Цена правила измерена на живом продукте 0.44.0 (`красное-моргание-e44.log`): при щелчках
    // по карточкам **5 кадров из 1087** не равны ни одному устоявшемуся виду, два из них
    // расходятся больше чем на процент окна, худший — на **27 700 пикселей**: на сохранённом
    // кадре первой карточки в окне НЕТ ВОВСЕ.
    //
    // ⭐ **Задача Т-46-3, решение 109.2:** и `wizard_refresh_controls`, который перерисовывал
    // ВЕСЬ шаг и ещё семь контролов на каждый щелчок, тоже ушёл. Перерисовывается только
    // затронутое, и **по роду элемента**, как в окне настроек.
    //
    // Смена шага по-прежнему идёт полной дорогой: там контролы вправду переезжают.
    wizard_after_choice(hwnd, touched);
    true
}

/// Что именно затронул выбор внутри шага — задача Т-46-3, решение 109.2.
///
/// Перерисовка идёт **по роду элемента, как в окне настроек**: глиф — дорогой `set_check`
/// (со стиранием), карточка — дорогой `invalidate_hot` (без стирания). Список перерисовывает
/// себя сам.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Touched {
    /// Одна галка.
    Glyph(i32),
    /// Ряд радио: диапазон идентификаторов, тот же, что ходит `check_radio`.
    Range(i32, i32),
    /// Две карточки: прежняя и новая.
    Cards(i32, i32),
    /// Ничего своего.
    Nothing,
}

/// **Состояние элемента внутри шага сменилось** — перерисовать затронутое и ничего больше.
///
/// Решение 109.2, задача Т-46-3. Пришло на смену `wizard_refresh_controls`, который звал
/// [`fill_wizard`] (а тот **переписывал поля устаревшим черновиком** — находка пользователя) и
/// инвалидировал все контролы шага плюс ещё семь на каждый щелчок.
///
/// # Что делается
///
/// 1. **Затронутый элемент** — по роду: галка и ряд радио через `widgets::repaint::control` и
///    `range` (со стиранием, ровно как `set_check`/`check_radio` окна настроек); две карточки
///    через `control_no_erase` (без стирания, ровно как `invalidate_hot`).
/// 2. **Подписи, чей текст вправду изменился.** Выбор карточки «идея» укорачивает дорогу с пяти
///    шагов до трёх, и «Шаг N из M», заголовок, пояснение и подпись кнопки обязаны это
///    показать. Пишутся только те, у кого текст стал другим: `SetDlgItemTextW` сам просит
///    перерисовку, и запись того же слова была бы лишним кадром.
/// 3. **Полоса хода** — только если строка шага изменилась: полоса рисует длину дороги, и
///    другой у неё причины меняться нет.
///
/// # Чего НЕ делается
///
/// [`fill_wizard`] — ни целиком, ни частью: **поля есть правда, пока шаг на экране**
/// (решение 109.3), и смена состояния их не касается. [`layout_wizard`] — внутри одного шага ни
/// один контрол не переезжает.
fn wizard_after_choice(hwnd: HWND, touched: Touched) {
    match touched {
        Touched::Glyph(control) => widgets::repaint::control(hwnd, control),
        Touched::Range(first, last) => widgets::repaint::range(hwnd, first, last),
        Touched::Cards(was, now) => {
            widgets::repaint::control_no_erase(hwnd, was);

            if now != was {
                widgets::repaint::control_no_erase(hwnd, now);
            }
        }
        Touched::Nothing => {}
    }

    // Подписи, зависящие от черновика. `road()` и `is_last()` читаются здесь, чтобы дорога
    // «идеи» короче пяти шагов стала видна сразу.
    //
    // SAFETY: the state pointer is live for the length of the window.
    let words = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref()?;
            Some(wizard_choice_words(wizard))
        })
    }
    .flatten();

    let Some(words) = words else {
        return;
    };

    let mut step_changed = false;

    for (control, text) in words {
        let changed = set_text_if_changed(hwnd, control, &text);

        if control == IDC_WZ_STEP {
            step_changed = changed;
        }
    }

    if step_changed {
        widgets::repaint::control_no_erase(hwnd, IDC_WZ_PROGRESS);
    }
}

/// Четыре подписи, которые зависят от черновика и потому могут смениться внутри шага.
///
/// Слова берутся теми же строками, что и в [`fill_wizard`], — второй таблицы не заводится:
/// одна и та же строка ресурса читается там и здесь.
fn wizard_choice_words(wizard: &Wizard) -> [(i32, String); 4] {
    use crate::settings::{
        IDS_CLOSE, IDS_THANKYOU_BUG_TITLE, IDS_THANKYOU_IDEA_TITLE, IDS_WIZARD_ATTACH_NOTE,
        IDS_WIZARD_ATTACH_TITLE, IDS_WIZARD_DID_NOTE, IDS_WIZARD_DID_TITLE, IDS_WIZARD_DONE,
        IDS_WIZARD_IDEA_NOTE, IDS_WIZARD_IDEA_TITLE, IDS_WIZARD_NEXT, IDS_WIZARD_PREVIEW_NOTE,
        IDS_WIZARD_PREVIEW_TITLE, IDS_WIZARD_STEP, IDS_WIZARD_WHAT_NOTE, IDS_WIZARD_WHAT_TITLE,
        IDS_WIZARD_WHERE_NOTE, IDS_WIZARD_WHERE_TITLE, format_text, text,
    };

    let step = wizard.current();
    let road = wizard.road();
    let idea = wizard.draft.trouble.is_idea();

    let (title, note) = match step {
        Step::What => (IDS_WIZARD_WHAT_TITLE, Some(IDS_WIZARD_WHAT_NOTE)),
        Step::Where => (IDS_WIZARD_WHERE_TITLE, Some(IDS_WIZARD_WHERE_NOTE)),
        Step::Did => (IDS_WIZARD_DID_TITLE, Some(IDS_WIZARD_DID_NOTE)),
        Step::Idea => (IDS_WIZARD_IDEA_TITLE, Some(IDS_WIZARD_IDEA_NOTE)),
        Step::Attach => (IDS_WIZARD_ATTACH_TITLE, Some(IDS_WIZARD_ATTACH_NOTE)),
        Step::Preview => (IDS_WIZARD_PREVIEW_TITLE, Some(IDS_WIZARD_PREVIEW_NOTE)),
        Step::Done => (
            if idea {
                IDS_THANKYOU_IDEA_TITLE
            } else {
                IDS_THANKYOU_BUG_TITLE
            },
            None,
        ),
    };

    [
        (
            IDC_WZ_STEP,
            if step == Step::Done {
                text(IDS_WIZARD_DONE)
            } else {
                format_text(
                    IDS_WIZARD_STEP,
                    &[&(wizard.step + 1).to_string(), &road.len().to_string()],
                )
            },
        ),
        (IDC_WZ_TITLE, text(title)),
        (IDC_WZ_NOTE, note.map(text).unwrap_or_default()),
        (
            IDC_WZ_NEXT,
            text(match step {
                Step::Done => IDS_CLOSE,
                _ if wizard.is_last() => IDS_WIZARD_DONE,
                _ => IDS_WIZARD_NEXT,
            }),
        ),
    ]
}

/// Пишет подпись, **только если она вправду другая**, и отвечает, писала ли.
///
/// Решение 109.2: `SetDlgItemTextW` сам просит перерисовку контрола, поэтому запись того же
/// слова — это лишний кадр и ничего больше.
fn set_text_if_changed(hwnd: HWND, control: i32, text: &str) -> bool {
    if settings::get_text(hwnd, control) == text {
        return false;
    }

    settings::set_text(hwnd, control, text);
    true
}

/// Reads what the person typed out of the fields of this step into the draft.
///
/// Called before every move: the fields are the truth while a step is on the screen, and the
/// draft is the truth between steps.
fn wizard_harvest(hwnd: HWND) {
    let program = settings::get_text(hwnd, IDC_WZ_PROGRAM);
    let expected = settings::get_text(hwnd, IDC_WZ_EXPECTED);
    let got = settings::get_text(hwnd, IDC_WZ_GOT);
    let idea = settings::get_text(hwnd, IDC_WZ_IDEA);
    let helps = settings::get_text(hwnd, IDC_WZ_HELPS);
    let preview = settings::get_text(hwnd, IDC_WZ_PREVIEW);

    // SAFETY: the state pointer is live for the length of the window.
    unsafe {
        with_state(hwnd, |state| {
            let Some(wizard) = state.wizard.as_deref_mut() else {
                return;
            };

            wizard.draft.program = program;
            wizard.draft.expected = expected;
            wizard.draft.got = got;
            wizard.draft.idea = idea;
            wizard.draft.helps = helps;

            // The appeal is only taken back from the field once it has one — before the last
            // step the control is empty and would erase what `build` is about to write.
            if !preview.is_empty() {
                wizard.text = preview;
            }
        })
    };
}

// ⚠ **`wizard_refresh_controls` убран — задача Т-46-3, решение 109.2.** Он звал `fill_wizard`
// на КАЖДЫЙ щелчок, и тот переписывал поля устаревшим черновиком (находка пользователя: «текст
// в полях сбрасывается при нажатии любого переключателя»), а затем инвалидировал все контролы
// шага и ещё семь. Дорога щелчка теперь — `wizard_after_choice` с перерисовкой затронутого по
// роду элемента; отсчёт, состояние и захват — три подписи ниже.

/// Подпись кнопки «Взять из активного окна» — она одна и меняется отсчётом, задача Т-46-3.
///
/// ⛔ **Пришло на смену `wizard_refresh_controls`, который звал [`fill_wizard`]** — а тот
/// переписывал ПОЛЯ устаревшим черновиком (решение 109.3, находка пользователя). Отсчёт меняет
/// одну надпись, и трогать он обязан одну надпись.
fn wizard_show_capture(hwnd: HWND) {
    use crate::settings::{IDS_WIZARD_CAPTURE, IDS_WIZARD_CAPTURE_COUNT, format_text, text};

    // SAFETY: the state pointer is live for the length of the window.
    let caption = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref()?;

            Some(if wizard.countdown > 0 {
                format_text(IDS_WIZARD_CAPTURE_COUNT, &[&wizard.countdown.to_string()])
            } else {
                text(IDS_WIZARD_CAPTURE)
            })
        })
    }
    .flatten();

    if let Some(caption) = caption {
        set_text_if_changed(hwnd, IDC_WZ_CAPTURE, &caption);
    }
}

/// Строка состояния под кнопками «Скопировать» и «Сохранить» — одна подпись, задача Т-46-3.
fn wizard_show_status(hwnd: HWND) {
    // SAFETY: the state pointer is live for the length of the window.
    let said = unsafe {
        with_state(hwnd, |state| {
            state.wizard.as_deref().map(|w| w.status.clone())
        })
    }
    .flatten();

    if let Some(said) = said {
        set_text_if_changed(hwnd, IDC_WZ_STATUS, &said);
    }
}

/// Имя пойманной программы — в поле «Программа:», задача Т-46-3.
///
/// ⚠ **Единственное место, где программа пишет в поле, пока шаг на экране,** и это не
/// нарушение решения 109.3, а его смысл: захват для того и нажат, чтобы поле заполнилось.
/// Пишется, только если имя вправду другое, — иначе каретка человека уехала бы ни за чем.
fn wizard_show_program(hwnd: HWND) {
    // SAFETY: the state pointer is live for the length of the window.
    let name = unsafe {
        with_state(hwnd, |state| {
            state.wizard.as_deref().map(|w| w.draft.program.clone())
        })
    }
    .flatten();

    if let Some(name) = name {
        set_text_if_changed(hwnd, IDC_WZ_PROGRAM, &name);
    }
}

/// Fills and lays out the wizard again, and repaints it.
///
/// # ⛔ Один кадр вместо череды — задача Т-33-8, находка глазом пользователя
///
/// Переключение шага двигает, показывает и прячет **несколько десятков** контролов, и каждый
/// `SetWindowPos`/`ShowWindow` сам просит перерисовать то, что из-под него открылось. Пока это
/// шло с включённой перерисовкой, на экран попадали кадры, которые не были ни прежним шагом,
/// ни новым: замерено — **19 кадров из 100**, худший расходился с обеими картинками на
/// 73 230 пикселей из 251 488 (`scratchpad-Э33\моргание-шагов-до.log`). Это и есть то
/// моргание, которое видно глазом.
///
/// `WM_SETREDRAW` — документированный способ сказать окну «пока не рисуй»: вся перестановка
/// проходит вслепую, и один `RedrawWindow` в конце показывает готовый шаг целиком.
/// `RDW_ALLCHILDREN` обязателен — без него перерисуется фон, а дети останутся прежними.
///
/// # ⛔ Почему здесь `SendMessageTimeoutW`, а не `SendMessage` — задача Т-33а-0
///
/// Первая редакция этой функции слала `WM_SETREDRAW` голым `SendMessageW`, и сторож FR-72
/// (`tests\guard.rs`) от этого падал: запрет голого `SendMessage` в этой программе стоит **на
/// всей** `src\`, а не на одном модуле `guard`, — ровно чтобы будущая правка не могла завести
/// его тихо. Красный уехал в поставку e43 вместе с правкой. Окно здесь своё и поток свой,
/// повиснуть не на чем, но правило от этого не перестаёт быть правилом: сторож, который падает,
/// не сторожит уже ничего.
fn wizard_refresh(hwnd: HWND) {
    use windows::Win32::Graphics::Gdi::{
        RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETREDRAW,
    };

    /// Сколько ждать ответа на `WM_SETREDRAW` — величина номинальная и никогда не тратится:
    /// адресат этих двух сообщений живёт в **этом же** потоке, а такому адресату система зовёт
    /// оконную процедуру прямо внутри вызова, не заводя ожидания вовсе. Число существует затем,
    /// чтобы в программе не осталось ни одного вызова без верхней границы (FR-72).
    const REDRAW_TIMEOUT_MS: u32 = 50;

    // SAFETY: `hwnd` is the live dialog of this very thread; the message carries two numbers and
    // no pointer, and the out-parameter is `None`, so nothing of ours is dereferenced. NFR-13: a
    // refused switch leaves the window drawing as it did, which is the state before the call.
    let _ = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_SETREDRAW,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            REDRAW_TIMEOUT_MS,
            None,
        )
    };

    // ⚠ `WM_SETREDRAW` у родителя гасит только его собственное рисование: у каждого ребёнка
    // свой флаг, и `SetWindowPos`/`ShowWindow` по ним перерисовываются сразу. Флаг ниже — вторая
    // половина тишины, без неё моргание оставалось (см. [`QUIET_LAYOUT`]).
    QUIET_LAYOUT.with(|quiet| quiet.set(true));

    // SAFETY: the state pointer is live for the length of the window.
    unsafe {
        with_state(hwnd, |state| {
            fill_wizard(hwnd, state);
            layout_wizard(hwnd, state);
        })
    };

    QUIET_LAYOUT.with(|quiet| quiet.set(false));

    // SAFETY: as above.
    let _ = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_SETREDRAW,
            WPARAM(1),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            REDRAW_TIMEOUT_MS,
            None,
        )
    };

    // NFR-13: a refused redraw leaves the window to the next `WM_PAINT`, which arrives anyway.
    //
    // SAFETY: `hwnd` is the live dialog; both region arguments are `None`, which is the
    // documented «the whole window», and the call keeps no pointer.
    let _ = unsafe {
        RedrawWindow(
            Some(hwnd),
            None,
            None,
            // `RDW_UPDATENOW` — чтобы всё нарисовалось ЗДЕСЬ, одним заходом, а не отдельными
            // `WM_PAINT` фона и каждого ребёнка: между такими проходами и виден чужой кадр.
            RDW_ERASE | RDW_FRAME | RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        )
    };
}

/// «Далее», «Готово» on the last step of the road, and «Закрыть» on the thanks — FR-104.
///
/// ⚠ **The thanks is a step and not a second window** (задача Т-33-3, решение 103.3). «Готово»
/// used to close this window and open one on `IDD_LETTER`; the user asked for the same window
/// at the same size, and what changes now is a flag.
fn wizard_forward(hwnd: HWND) {
    wizard_harvest(hwnd);

    // SAFETY: the state pointer is live for the length of the window.
    let close = unsafe {
        with_state(hwnd, |state| {
            let Some(wizard) = state.wizard.as_deref_mut() else {
                return false;
            };

            // The thanks is the end of the road: its one button closes the window.
            if wizard.done {
                return true;
            }

            if wizard.is_last() {
                wizard.done = true;
                wizard.status.clear();
                return false;
            }

            wizard.step += 1;
            wizard.status.clear();

            // The text of the appeal is built on the way **into** the last step, out of what
            // the person has filled in by then — and from that moment the field owns it.
            if wizard.current() == Step::Preview {
                wizard.text = report::build(&wizard.draft, &wizard.facts);
            }

            false
        })
    };

    if close == Some(true) {
        close_window(hwnd);
        return;
    }

    wizard_refresh(hwnd);
}

/// «Взять из активного окна» — starts the five-second count-down of FR-104.
fn wizard_start_capture(hwnd: HWND) {
    // SAFETY: the state pointer is live for the length of the window.
    let started = unsafe {
        with_state(hwnd, |state| {
            let Some(wizard) = state.wizard.as_deref_mut() else {
                return false;
            };

            if wizard.countdown > 0 {
                return false;
            }

            wizard.countdown = CAPTURE_SECONDS;
            true
        })
    };

    if started != Some(true) {
        return;
    }

    // SAFETY: `hwnd` is the live window; `None` for the callback asks for `WM_TIMER` at this
    // window, which is what the procedure answers.
    if unsafe { SetTimer(Some(hwnd), CAPTURE_TIMER, 1000, None) } == 0 {
        crate::app::report_non_critical("SetTimer", &WinError::from_thread());
    }

    // Задачи Т-45-3 и Т-46-3: меняется одна подпись — перерисовывается она одна.
    wizard_show_capture(hwnd);
}

/// One second of the count-down; at zero it looks at the foreground window and stops.
fn wizard_tick(hwnd: HWND) {
    // SAFETY: the state pointer is live for the length of the window.
    let now = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref_mut()?;

            wizard.countdown = wizard.countdown.saturating_sub(1);
            Some(wizard.countdown)
        })
    }
    .flatten();

    if now != Some(0) {
        // ⭐ Тик отсчёта каждую секунду перерисовывал ОКНО ЦЕЛИКОМ. Задача Т-45-3: только
        // затронутое — на экране меняется одна цифра (Т-46-3: одна подпись).
        wizard_show_capture(hwnd);
        return;
    }

    // SAFETY: `hwnd` is the live window; killing a timer that is not set is harmless.
    let _ = unsafe { KillTimer(Some(hwnd), CAPTURE_TIMER) };

    let caught = capture_foreground(hwnd);

    // SAFETY: as above.
    unsafe {
        with_state(hwnd, |state| {
            if let Some(wizard) = state.wizard.as_deref_mut()
                && !caught.is_empty()
            {
                wizard.draft.program = caught;
            }
        })
    };

    // Задачи Т-45-3 и Т-46-3: шаг тот же, меняются имя в поле и подпись кнопки — точечно.
    wizard_show_program(hwnd);
    wizard_show_capture(hwnd);
}

/// **What the capture takes** — «имя.exe · КлассОкна», and nothing else.
///
/// ⛔ **The window title is never read.** FR-104 says so in as many words and the reason is in
/// the note beside the button: a title carries the name of a document, of a message, of a page,
/// and none of that is any of this program's business. `GetWindowTextW` is not called here.
///
/// The wizard's own window is skipped: a person who presses the button and does not switch
/// anywhere would otherwise be told that the program they have trouble with is this one.
fn capture_foreground(mine: HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: a plain query; the handle it answers is borrowed and not freed.
    let front = unsafe { GetForegroundWindow() };

    if front.is_invalid() || front == mine {
        return String::new();
    }

    // ⚠ **The name of a process is read by module `guard` and by nothing else** — the rule
    // `tests\guard.rs` holds the whole program to, and it caught this function the first time
    // it was written: the call that reads an image name had been spelled out here, in a module
    // that has no business making it. The chain below is the one FR-84 already uses, borrowed
    // rather than copied (§6.2). ⚠ The sweep reads **prose as well as code**, so the name of
    // that call is deliberately not written in this comment either.
    let class = crate::guard::class_name(front);

    let mut pid: u32 = 0;

    // SAFETY: `front` is the live window and `pid` a live local the call fills.
    unsafe { GetWindowThreadProcessId(front, Some(&mut pid)) };

    let name = if pid == 0 {
        String::new()
    } else {
        crate::guard::process_file_name(pid).unwrap_or_default()
    };

    match (name.is_empty(), class.is_empty()) {
        (true, true) => String::new(),
        (false, true) => name,
        (true, false) => class,
        (false, false) => format!("{name} · {class}"),
    }
}

/// «Скопировать и открыть канал» — FR-104.
///
/// The clipboard, and then the shell, and **only** in that order: a person whose channel does
/// not open still has the text. While the address is a placeholder (П7) the second half does
/// not happen at all, and the status line says so.
fn wizard_copy(hwnd: HWND) {
    use crate::settings::{IDS_WIZARD_COPIED, IDS_WIZARD_COPIED_ONLY, IDS_WIZARD_FAILED, text};

    wizard_harvest(hwnd);

    // SAFETY: the state pointer is live for the length of the window.
    let appeal = unsafe {
        with_state(hwnd, |state| {
            state.wizard.as_deref().map(|wizard| wizard.text.clone())
        })
    }
    .flatten()
    .unwrap_or_default();

    if appeal.is_empty() {
        return;
    }

    let written = crate::selection::write_unicode_text(hwnd, &appeal).is_ok();
    let live = !links::is_placeholder(links::CHANNEL_URL);

    if written && live {
        open_link(hwnd, links::CHANNEL_URL);
    }

    let said = match (written, live) {
        (false, _) => text(IDS_WIZARD_FAILED),
        (true, false) => text(IDS_WIZARD_COPIED_ONLY),
        (true, true) => text(IDS_WIZARD_COPIED),
    };

    wizard_say(hwnd, said);
}

/// «Сохранить в папку журнала» — FR-104: a file beside the journal, and the folder opened after
/// it so that the person can see what was written.
fn wizard_save(hwnd: HWND) {
    use crate::settings::{IDS_WIZARD_FAILED, IDS_WIZARD_SAVED, format_text, text};

    wizard_harvest(hwnd);

    // SAFETY: the state pointer is live for the length of the window.
    let appeal = unsafe {
        with_state(hwnd, |state| {
            state.wizard.as_deref().map(|wizard| wizard.text.clone())
        })
    }
    .flatten()
    .unwrap_or_default();

    if appeal.is_empty() {
        return;
    }

    let Some(folder) = crate::diag::log_dir() else {
        wizard_say(hwnd, text(IDS_WIZARD_FAILED));
        return;
    };

    let today = today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day"));
    let path = folder.join(report::file_name(today));

    if std::fs::create_dir_all(&folder).is_err() || std::fs::write(&path, &appeal).is_err() {
        wizard_say(hwnd, text(IDS_WIZARD_FAILED));
        return;
    }

    // ⚠ The **name** of the file and not its path: the live acceptance showed the whole path
    // running off the one line the status is (`живое-В\мастер-5-сохранено.png` — «Сохранено:»
    // and nothing after it). The folder opens a moment later, so the name is what a person
    // needs to find it there.
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    wizard_say(hwnd, format_text(IDS_WIZARD_SAVED, &[&name]));

    open_folder(&folder);
}

/// Opens a folder in the shell — the road the journal folder of the settings window takes.
fn open_folder(folder: &std::path::Path) {
    let wide = settings::wide(&folder.display().to_string());

    // SAFETY: both buffers are NUL-terminated and live for the length of the call; `SW_SHOW`
    // is a plain value. ⛔ Nothing is run: «open» on a folder hands it to the file manager.
    let answer = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    if answer.0 as isize <= 32 {
        crate::app::report_non_critical("ShellExecuteW", &WinError::from_thread());
    }
}

/// Writes one line into the wizard's status line and shows it.
fn wizard_say(hwnd: HWND, said: String) {
    // SAFETY: the state pointer is live for the length of the window.
    unsafe {
        with_state(hwnd, |state| {
            if let Some(wizard) = state.wizard.as_deref_mut() {
                wizard.status = said;
            }
        })
    };

    // Задачи Т-45-3 и Т-46-3: строка состояния — одна подпись, и пишется она одна.
    wizard_show_status(hwnd);
}

/// The ink of a glyph's mark, by the role the table names.
fn glyph_ink(
    mark: theme::GlyphMarkRole,
    palette: &theme::Palette,
) -> windows::Win32::Foundation::COLORREF {
    match mark {
        theme::GlyphMarkRole::AccentFg => palette.accent_fg,
        theme::GlyphMarkRole::AccentBg => palette.accent_bg,
        theme::GlyphMarkRole::BoxBorder => palette.box_border,
    }
}

/// The height of one line with a glyph beside it — the box or the circle, or the text, which
/// ever is taller.
fn glyph_height(metrics: Metrics, faces: &Faces) -> i32 {
    metrics.y(10).max(faces.body_height.abs() + 2)
}

/// Where the body of the wizard has to stop — the top of the button row, less the air above it.
fn bottom_of_body(client: RECT, metrics: Metrics) -> i32 {
    client.bottom - metrics.y(air::PAD) - metrics.y(air::BUTTON) - metrics.y(air::GAP)
}

// =========================================================================================
// 12. The author's feed — FR-102, SEC-03, task Т-32-6
// =========================================================================================

/// **The one network operation this program has**, and everything that decides what to do with
/// what it brings back.
///
/// # What is in here and what is deliberately not
///
/// In: one `GET` over HTTPS from a fixed list of addresses, no more often than once in fifteen
/// days; the ECDSA P-256 signature of the answer, checked by the system's own cryptography; the
/// parsing of the document; the choice of language. **Nothing else in this program opens a
/// socket** — that is what makes «which code can reach the network» a question with a one-line
/// answer, and what ворота 3 of `tools\verify-perimeter.ps1` checks from the outside by reading
/// the strings of the shipped binary.
///
/// Not in, and never: sending anything, downloading anything, running anything, an identifier,
/// a version, a language, a cookie, a conditional request. The request carries **no parameters
/// of any kind** (вопрос 101 п. 5): the file holds every language and the choice is made on
/// this machine.
///
/// # The trust is in the signature and not in the host
///
/// The addresses are ordinary web hosting and may be anybody's tomorrow. What the program
/// believes is the author's signature over the body, checked against **two** public keys
/// compiled into it — a working one and a reserve. A file whose signature does not check out is
/// dropped with one line in the journal and no trace in the interface at all: a person must
/// never be shown a letter this program cannot prove is the author's.
pub mod feed {
    use super::{Date, FeedItem, NEWS_KEPT};

    /// The two public keys a signature is accepted from — the working one and the reserve.
    ///
    /// `X` then `Y`, thirty-two bytes each, which is the tail of a `BCRYPT_ECCPUBLIC_BLOB`; the
    /// eight-byte header is a constant of the format and is put on at import time.
    ///
    /// Made by `tools\make-news-key.ps1`. The private half of the first lives in this machine's
    /// key store and in an encrypted file; the private half of the second lives **only** in an
    /// encrypted file, off this machine — so that a stolen working key can be answered with a
    /// letter the thief cannot forge.
    pub const FEED_KEYS: [[u8; 64]; 2] = [
        // The working key.
        [
            0x44, 0xCF, 0xB8, 0xB2, 0x7F, 0x43, 0x40, 0xF0, 0x58, 0x5E, 0x82, 0x4A, 0x46, 0xEE,
            0xA1, 0x8B, 0xD9, 0x84, 0x48, 0x47, 0x45, 0x50, 0xB4, 0x12, 0xA8, 0x3D, 0x45, 0x71,
            0xB3, 0x9B, 0xC0, 0x8E, 0x25, 0xF3, 0x49, 0xD0, 0x99, 0xC8, 0x6D, 0xA6, 0x9F, 0x96,
            0x58, 0x41, 0x86, 0x91, 0x86, 0xA4, 0x33, 0x81, 0xB3, 0xAD, 0x71, 0xC9, 0xB5, 0x84,
            0x74, 0xDB, 0x90, 0x38, 0x34, 0x7B, 0x3D, 0x66,
        ],
        // The reserve key — it does not live on this machine.
        [
            0x94, 0xFC, 0x7F, 0x20, 0x1A, 0x69, 0x5E, 0x35, 0xD2, 0x09, 0x46, 0xE1, 0xF9, 0x49,
            0x4E, 0x45, 0x88, 0x90, 0xF6, 0xF7, 0x18, 0x23, 0x46, 0x56, 0x02, 0x19, 0xA1, 0x9F,
            0xC8, 0xBC, 0x4D, 0x27, 0xF1, 0x25, 0x37, 0x4A, 0x7F, 0x7E, 0x0A, 0xDA, 0x2D, 0xAD,
            0x30, 0x41, 0xE8, 0x67, 0x98, 0xC9, 0x56, 0x26, 0xA8, 0x0D, 0x9D, 0xD9, 0x9C, 0xCA,
            0x04, 0xB6, 0x7C, 0x86, 0x62, 0xFC, 0xFF, 0x9C,
        ],
    ];

    /// The largest answer this program will read — FR-102.
    ///
    /// A news item in fourteen languages is about 21 KB and the whole file about 84 KB; the
    /// author's own signing script refuses to sign anything over 192 KB. This is the ceiling on
    /// what is **read**, and it is deliberately above that one: the script's limit protects the
    /// reader from a mistake, and this one protects it from a host that is not the author's.
    pub const RESPONSE_CAP: usize = 256 * 1024;

    /// The schema of the document this build understands.
    pub const FEED_SCHEMA: u32 = 1;

    /// Why a feed was refused — one word for the journal and nothing else.
    ///
    /// **Never shown to anybody.** SEC-07: a refusal is a line in the journal with the name of
    /// the operation, and what the interface does about it is nothing at all.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Refusal {
        /// The first line is not a signature.
        NoSignature,
        /// The signature is not sixty-four bytes of base64.
        BadSignature,
        /// The body is not TOML, or not the shape FR-102 describes.
        Malformed,
        /// The document names a schema this build does not know.
        Schema(u32),
        /// The signature did not check out against either key.
        Unsigned,
        /// The answer was longer than [`RESPONSE_CAP`].
        TooBig,
    }

    impl Refusal {
        /// The name this refusal goes into the journal under — one of a closed set, and no
        /// value of the document travels with it (SEC-07).
        pub fn journal_name(self) -> &'static str {
            match self {
                Self::NoSignature | Self::BadSignature => "feed signature line refused",
                Self::Malformed => "feed document refused",
                Self::Schema(_) => "feed from a newer schema",
                Self::Unsigned => "feed signature did not verify",
                Self::TooBig => "feed answer oversized",
            }
        }
    }

    /// What one read of the feed produced: the single `update` entry and the news items kept.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Feed {
        /// The `update` entry with the greatest identifier, if there is one.
        pub update: Option<FeedItem>,
        /// At most [`NEWS_KEPT`] `news` entries — the ones with the greatest identifiers,
        /// **oldest first**, which is the order they are shown in.
        pub news: Vec<FeedItem>,
    }

    /// The signature line and the body it covers — the split every read makes first.
    ///
    /// The body is **the bytes after the first newline**, verbatim. Not «the document without
    /// the first line» as a parsed thing: what is signed is bytes, and re-serialising them
    /// would let a difference of formatting change what was checked.
    fn split(text: &str) -> Result<(Vec<u8>, &str), Refusal> {
        let (first, body) = text.split_once('\n').ok_or(Refusal::NoSignature)?;

        let quoted = first
            .trim()
            .strip_prefix("signature")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::trim)
            .ok_or(Refusal::NoSignature)?;

        let encoded = quoted
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .ok_or(Refusal::NoSignature)?;

        let signature = base64(encoded).ok_or(Refusal::BadSignature)?;

        if signature.len() != 64 {
            return Err(Refusal::BadSignature);
        }

        Ok((signature, body))
    }

    /// Decodes base64 — the sixty-four bytes of a signature and nothing else.
    ///
    /// Written here because the dependency list of section 3.2 is closed (SEC-03) and holds no
    /// base64 crate. Deliberately strict: the standard alphabet, padding required, and any
    /// character outside it refuses the whole string. It decodes **one field of one file**, and
    /// a lenient decoder is a decoder that accepts two spellings of one signature.
    fn base64(text: &str) -> Option<Vec<u8>> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let bytes = text.as_bytes();

        if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
            return None;
        }

        let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();

        if padding > 2 {
            return None;
        }

        let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
        let mut accumulator = 0u32;
        let mut held = 0u32;

        for byte in &bytes[..bytes.len() - padding] {
            let value = ALPHABET.iter().position(|letter| letter == byte)? as u32;

            accumulator = (accumulator << 6) | value;
            held += 6;

            if held >= 8 {
                held -= 8;
                out.push(u8::try_from((accumulator >> held) & 0xFF).ok()?);
            }
        }

        // What is left over must be zero bits: anything else is a string that decodes to
        // something its own padding says it does not.
        if accumulator & ((1 << held) - 1) != 0 {
            return None;
        }

        Some(out)
    }

    /// Reads one feed document: the signature, the signature check, the parse and the choice of
    /// what to keep — **the whole of what a read of the feed means**.
    ///
    /// `verify` is passed in rather than called: the signature check is the one part of this
    /// that needs the system, and a test that could not replace it could not check a single
    /// rule of the parsing against a document it wrote itself. The product passes
    /// [`verify_signature`]; the tests pass their own.
    pub fn read_document(
        text: &str,
        language: &str,
        verify: impl Fn(&[u8], &[u8]) -> bool,
    ) -> Result<Feed, Refusal> {
        if text.len() > RESPONSE_CAP {
            return Err(Refusal::TooBig);
        }

        let (signature, body) = split(text)?;

        if !verify(body.as_bytes(), &signature) {
            return Err(Refusal::Unsigned);
        }

        parse_body(body, language)
    }

    /// The parsed body — the half that needs no cryptography and no network.
    pub fn parse_body(body: &str, language: &str) -> Result<Feed, Refusal> {
        let document: RawFeed = toml::from_str(body).map_err(|_| Refusal::Malformed)?;

        if document.schema != FEED_SCHEMA {
            return Err(Refusal::Schema(document.schema));
        }

        let mut news: Vec<FeedItem> = Vec::new();
        let mut update: Option<FeedItem> = None;

        for raw in document.item {
            let Some(item) = raw.into_item(language) else {
                // FR-102: a record with no Russian and no English is skipped, and the file is
                // **not** refused over it. One bad entry must not silence the others.
                continue;
            };

            match item.version {
                // An `update` entry: the one with the greatest identifier wins.
                Some(_) => {
                    if update.as_ref().is_none_or(|kept| kept.id < item.id) {
                        update = Some(item);
                    }
                }
                None => news.push(item),
            }
        }

        // The three greatest identifiers, then back into ascending order — the order they are
        // shown and reminded of in.
        news.sort_unstable_by_key(|item| item.id);

        if news.len() > NEWS_KEPT {
            news.drain(..news.len() - NEWS_KEPT);
        }

        Ok(Feed { update, news })
    }

    /// **Checks the author's signature over the body** — FR-102, the system's own cryptography
    /// and no crate of anybody's (SEC-03).
    ///
    /// ECDSA P-256 over SHA-256, and the signature is the raw `r ‖ s` pair of sixty-four bytes
    /// that `BCryptVerifySignature` wants — not a DER structure. It is accepted from **either**
    /// of [`FEED_KEYS`]: that is the whole of the recovery plan, and the check costs one more
    /// import of thirty-two bytes.
    ///
    /// `false` for every refusal along the way (NFR-13, SEC-05): a provider that will not open,
    /// a key that will not import, a hash that will not finish, a signature that does not
    /// verify — all mean the same thing to the caller, «this is not the author's file», and the
    /// journal is told which call refused. **A document that fails here is dropped without a
    /// trace in the interface**: a person must never be shown a letter this program cannot
    /// prove is the author's.
    pub fn verify_signature(body: &[u8], signature: &[u8]) -> bool {
        verify_with(body, signature, &FEED_KEYS)
    }

    /// [`verify_signature`] against a named set of keys — the same body, and the only thing the
    /// product ever passes is [`FEED_KEYS`].
    ///
    /// The keys are a parameter for one reason and it is not flexibility: **a test must sign
    /// with keys of its own** (the mandate: «тестовые ключи — отдельные от рабочих»), and a
    /// check that could only be run against the author's own key could only be run by the
    /// author. What the test then exercises is this very body — the import, the hash, the
    /// verify and the two-key loop — and not a copy of it.
    pub fn verify_with(body: &[u8], signature: &[u8], keys: &[[u8; 64]]) -> bool {
        use windows::Win32::Security::Cryptography::{
            BCRYPT_ALG_HANDLE, BCRYPT_ECCPUBLIC_BLOB, BCRYPT_ECDSA_P256_ALGORITHM,
            BCRYPT_ECDSA_PUBLIC_P256_MAGIC, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE,
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS, BCryptCloseAlgorithmProvider, BCryptDestroyKey,
            BCryptImportKeyPair, BCryptOpenAlgorithmProvider, BCryptVerifySignature,
        };
        use windows::core::PCWSTR;

        if signature.len() != 64 {
            return false;
        }

        let Some(hash) = sha256(body) else {
            return false;
        };

        let mut algorithm = BCRYPT_ALG_HANDLE::default();

        // SAFETY: `algorithm` is a live local the call fills; the algorithm name is a static
        // NUL-terminated literal of this image and the implementation is the default.
        let opened = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut algorithm,
                BCRYPT_ECDSA_P256_ALGORITHM,
                PCWSTR::null(),
                BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
            )
        };

        if opened.is_err() {
            crate::app::report_non_critical(
                "BCryptOpenAlgorithmProvider",
                &windows::core::Error::from_hresult(opened.to_hresult()),
            );
            return false;
        }

        let mut accepted = false;

        for key in keys {
            // `BCRYPT_ECCPUBLIC_BLOB`: the magic, the size of one coordinate, then X and Y.
            let mut blob = Vec::with_capacity(8 + key.len());
            blob.extend_from_slice(&BCRYPT_ECDSA_PUBLIC_P256_MAGIC.to_le_bytes());
            blob.extend_from_slice(&32u32.to_le_bytes());
            blob.extend_from_slice(key);

            let mut handle = BCRYPT_KEY_HANDLE::default();

            // SAFETY: `algorithm` is the provider just opened, `blob` a live buffer of this
            // frame holding a whole public blob, and `handle` a live local the call fills.
            let imported = unsafe {
                BCryptImportKeyPair(
                    algorithm,
                    None,
                    BCRYPT_ECCPUBLIC_BLOB,
                    &mut handle,
                    &blob,
                    0,
                )
            };

            if imported.is_err() {
                crate::app::report_non_critical(
                    "BCryptImportKeyPair",
                    &windows::core::Error::from_hresult(imported.to_hresult()),
                );
                continue;
            }

            // SAFETY: `handle` is the key just imported; both slices are live buffers of this
            // frame and are read, not written.
            let verified =
                unsafe { BCryptVerifySignature(handle, None, &hash, signature, BCRYPT_FLAGS(0)) };

            // SAFETY: frees exactly the key imported above, once.
            let _ = unsafe { BCryptDestroyKey(handle) };

            if verified.is_ok() {
                accepted = true;
                break;
            }
        }

        // SAFETY: closes exactly the provider opened above, once.
        let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };

        accepted
    }

    /// The SHA-256 of a buffer, by the system's own hash — the half of the check that is not
    /// about keys.
    fn sha256(body: &[u8]) -> Option<[u8; 32]> {
        use windows::Win32::Security::Cryptography::{
            BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
            BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptCreateHash,
            BCryptDestroyHash, BCryptFinishHash, BCryptHashData, BCryptOpenAlgorithmProvider,
        };
        use windows::core::PCWSTR;

        let mut algorithm = BCRYPT_ALG_HANDLE::default();

        // SAFETY: as in `verify_signature` — a live local and a static literal.
        let opened = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut algorithm,
                BCRYPT_SHA256_ALGORITHM,
                PCWSTR::null(),
                BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
            )
        };

        if opened.is_err() {
            crate::app::report_non_critical(
                "BCryptOpenAlgorithmProvider",
                &windows::core::Error::from_hresult(opened.to_hresult()),
            );
            return None;
        }

        let mut hash = BCRYPT_HASH_HANDLE::default();

        // SAFETY: `algorithm` is the provider just opened; `None` for the object buffer asks
        // the provider to allocate its own, which is the documented modern form.
        let created = unsafe { BCryptCreateHash(algorithm, &mut hash, None, None, 0) };

        let mut digest = [0u8; 32];
        let mut ok = created.is_ok();

        if ok {
            // SAFETY: `hash` is the object just created and `body` a buffer the caller owns for
            // the length of this call; it is read and not written.
            ok = unsafe { BCryptHashData(hash, body, 0) }.is_ok();
        }

        if ok {
            // SAFETY: as above; `digest` is a live local the call fills, and its length is what
            // SHA-256 produces.
            ok = unsafe { BCryptFinishHash(hash, &mut digest, 0) }.is_ok();
        }

        if !created.is_err() {
            // SAFETY: frees exactly the hash object created above, once.
            let _ = unsafe { BCryptDestroyHash(hash) };
        }

        // SAFETY: closes exactly the provider opened above, once.
        let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };

        ok.then_some(digest)
    }

    /// The name this program calls itself on the network — FR-102, and **it carries no
    /// version**.
    ///
    /// A user agent is the one field of the request the author could be tempted to put
    /// something in, and вопрос 101 п. 5 says the request carries no information about the
    /// machine at all. A version would tell the host how many people are running which build —
    /// which is telemetry with a different name. The test
    /// `the_user_agent_names_the_program_and_nothing_about_the_machine` fails if a digit ever
    /// appears here.
    pub const USER_AGENT: &str = "LangSwitcher";

    /// How long the program waits for the name to resolve and the connection to open, in
    /// milliseconds — FR-102.
    pub const CONNECT_TIMEOUT_MS: i32 = 10_000;

    /// And how long for the answer — FR-102.
    pub const RECEIVE_TIMEOUT_MS: i32 = 20_000;

    /// **Reads one address** — the whole of the network surface of this program.
    ///
    /// One `GET` over HTTPS with no parameters, no headers of ours beyond the agent, no
    /// conditional request and no cookie. The answer is read to [`RESPONSE_CAP`] and no
    /// further: a host that answers for ever gets to fill one buffer.
    ///
    /// ⛔ **Nothing is written to disk, nothing is executed, nothing is sent.** What comes back
    /// is bytes in memory, and the only thing done with them is a signature check.
    ///
    /// `None` for every refusal — a name that will not resolve, a connection that will not
    /// open, a status that is not 200, a body over the ceiling. FR-102 says a refusal is
    /// silent: the journal is told the name of the call, the interface is told nothing, and
    /// `feed_last_read` is **not** moved, so a broken host is tried again tomorrow rather than
    /// in fifteen days.
    ///
    /// # Safety
    ///
    /// None: every unsafe block inside is local and documented. The function is safe to call
    /// from any thread, and the product calls it from a thread of its own that lives exactly
    /// one request (SPEC section 6.1).
    pub fn fetch(url: &str) -> Option<String> {
        use windows::Win32::Networking::WinHttp::{
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WinHttpCloseHandle, WinHttpOpen,
            WinHttpSetTimeouts,
        };
        use windows::core::PCWSTR;

        let (host, path) = split_https(url)?;

        // The line that makes «no request was made» measurable. It is written **before** the
        // session exists, carries no address and no status (SEC-07), and is the positive half
        // of the instrument that checks `[letters] feed = false`: with the feed on, the journal
        // of a due read holds this line; with it off, the journal never holds it.
        crate::diag::record(
            crate::diag::Operation::from_name("feed request"),
            crate::diag::OsCode::NONE,
        );

        let agent = crate::settings::wide(USER_AGENT);
        let host_wide = crate::settings::wide(&host);
        let path_wide = crate::settings::wide(&path);
        let verb = crate::settings::wide("GET");

        // SAFETY: the agent is a NUL-terminated buffer of this frame; the two null pointers are
        // the documented «no proxy, no bypass», and the access type asks Windows for the
        // machine's own automatic proxy configuration.
        let session = unsafe {
            WinHttpOpen(
                PCWSTR(agent.as_ptr()),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            )
        };

        if session.is_null() {
            crate::app::report_non_critical("WinHttpOpen", &windows::core::Error::from_thread());
            return None;
        }

        // The four timeouts of FR-102: resolve, connect, send, receive. A feed that does not
        // answer must not hold a thread of this program for minutes.
        //
        // SAFETY: `session` is the handle just opened; the four numbers are plain values.
        let _ = unsafe {
            WinHttpSetTimeouts(
                session,
                CONNECT_TIMEOUT_MS,
                CONNECT_TIMEOUT_MS,
                RECEIVE_TIMEOUT_MS,
                RECEIVE_TIMEOUT_MS,
            )
        };

        let answer = read_over(session, &host_wide, &path_wide, &verb);

        // SAFETY: closes exactly the session opened above, once. Every handle below it is
        // closed by `read_over` before it returns.
        let _ = unsafe { WinHttpCloseHandle(session) };

        answer
    }

    /// The connection, the request and the reading — the half of [`fetch`] that owns three
    /// handles, split out so that the session above is closed on every path.
    fn read_over(
        session: *mut std::ffi::c_void,
        host: &[u16],
        path: &[u16],
        verb: &[u16],
    ) -> Option<String> {
        use windows::Win32::Networking::WinHttp::{
            WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS, WinHttpCloseHandle, WinHttpConnect,
            WinHttpOpenRequest, WinHttpReceiveResponse, WinHttpSendRequest,
        };
        use windows::core::PCWSTR;

        const HTTPS_PORT: u16 = 443;

        // SAFETY: `session` is the live session and `host` a NUL-terminated buffer of the
        // caller's frame; the reserved argument is zero as documented.
        let connection = unsafe { WinHttpConnect(session, PCWSTR(host.as_ptr()), HTTPS_PORT, 0) };

        if connection.is_null() {
            crate::app::report_non_critical("WinHttpConnect", &windows::core::Error::from_thread());
            return None;
        }

        // ⚠ **`WINHTTP_FLAG_SECURE` is what makes this HTTPS and it is not optional.** Without
        // it the same call would fetch the same path over plain HTTP from port 443 and fail in
        // a way that looks like a network problem.
        //
        // SAFETY: `connection` is the handle just opened; the four string arguments are
        // NUL-terminated buffers (two of the caller's frame, two null for «the default version»
        // and «no referrer»), and the accept-types pointer is null for «anything».
        let request = unsafe {
            WinHttpOpenRequest(
                connection,
                PCWSTR(verb.as_ptr()),
                PCWSTR(path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_OPEN_REQUEST_FLAGS(WINHTTP_FLAG_SECURE.0),
            )
        };

        if request.is_null() {
            crate::app::report_non_critical(
                "WinHttpOpenRequest",
                &windows::core::Error::from_thread(),
            );
            // SAFETY: closes exactly the connection opened above, once.
            let _ = unsafe { WinHttpCloseHandle(connection) };
            return None;
        }

        // **No headers of ours and no body**: `None` for the headers, null and zero for the
        // optional data, and zero for the context. That is the whole request — вопрос 101 п. 5.
        //
        // SAFETY: `request` is the handle just opened; every argument is a plain value or a
        // documented «nothing».
        let sent = unsafe { WinHttpSendRequest(request, None, None, 0, 0, 0) };

        let answer = if sent.is_err() {
            crate::app::report_non_critical(
                "WinHttpSendRequest",
                &windows::core::Error::from_thread(),
            );
            None
        } else {
            // SAFETY: `request` is the live request; the reserved argument is null.
            let received = unsafe { WinHttpReceiveResponse(request, std::ptr::null_mut()) };

            if received.is_err() {
                crate::app::report_non_critical(
                    "WinHttpReceiveResponse",
                    &windows::core::Error::from_thread(),
                );
                None
            } else if status_of(request) != 200 {
                // A status that is not 200 is a refusal like any other and is not journalled
                // with its number: SEC-07, and the number is the host's business.
                None
            } else {
                drain(request)
            }
        };

        // SAFETY: closes exactly the two handles opened above, once each.
        unsafe {
            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connection);
        }

        answer
    }

    /// The status code of an answer, or zero when it cannot be read.
    fn status_of(request: *mut std::ffi::c_void) -> u32 {
        use windows::Win32::Networking::WinHttp::{
            WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpQueryHeaders,
        };
        use windows::core::PCWSTR;

        let mut status = 0u32;
        let mut length = u32::try_from(size_of::<u32>()).unwrap_or(4);
        let mut index = 0u32;

        // SAFETY: `request` is the live request; `status`, `length` and `index` are live locals
        // the call fills, and the length says how big the buffer is.
        let read = unsafe {
            WinHttpQueryHeaders(
                request,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(std::ptr::from_mut(&mut status).cast()),
                &mut length,
                &mut index,
            )
        };

        if read.is_err() { 0 } else { status }
    }

    /// Reads the body to [`RESPONSE_CAP`] and no further.
    fn drain(request: *mut std::ffi::c_void) -> Option<String> {
        use windows::Win32::Networking::WinHttp::WinHttpReadData;

        let mut body: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 8192];

        loop {
            let mut read = 0u32;

            // SAFETY: `request` is the live request and `chunk` a live local of this frame;
            // the call writes at most as many bytes as it is told the buffer holds.
            let ok = unsafe {
                WinHttpReadData(
                    request,
                    chunk.as_mut_ptr().cast(),
                    u32::try_from(chunk.len()).unwrap_or(0),
                    &mut read,
                )
            };

            if ok.is_err() {
                crate::app::report_non_critical(
                    "WinHttpReadData",
                    &windows::core::Error::from_thread(),
                );
                return None;
            }

            if read == 0 {
                break;
            }

            let taken = usize::try_from(read).unwrap_or(0).min(chunk.len());

            // The ceiling of FR-102. A host that answers for ever fills one buffer and is then
            // dropped: the read stops and the file is refused, because a truncated document is
            // a document whose signature will not check out anyway.
            if body.len() + taken > RESPONSE_CAP {
                return None;
            }

            body.extend_from_slice(&chunk[..taken]);
        }

        // The document is UTF-8 by construction — the author's own script writes it. Anything
        // else is not the author's file, and `from_utf8` saying so is the cheapest of the
        // checks that say it.
        String::from_utf8(body).ok()
    }

    /// **One read of the feed**: every address in order, the first good answer winning.
    ///
    /// «Good» means all of it — the answer arrived, the signature checked out against one of the
    /// two keys, and the document parsed. An address that answers with something that is not
    /// the author's file is not «a bad host» to be retried differently; it is simply not an
    /// answer, and the next address is tried.
    ///
    /// `None` when no address gave one. The caller then leaves `feed_last_read` where it is, so
    /// the next attempt is tomorrow rather than in fifteen days.
    pub fn read_now(language: &str) -> Option<Feed> {
        for url in super::links::FEED_URLS {
            // A placeholder address is not read at all: `.invalid` never resolves, and asking
            // costs a DNS timeout for nothing (полномочие П5).
            if super::links::is_placeholder(url) {
                continue;
            }

            let Some(text) = fetch(url) else {
                continue;
            };

            match read_document(&text, language, verify_signature) {
                Ok(feed) => return Some(feed),
                Err(refusal) => {
                    // SEC-07: the name of what went wrong and not a word of the document.
                    crate::diag::record(
                        crate::diag::Operation::from_name(refusal.journal_name()),
                        crate::diag::OsCode::NONE,
                    );
                }
            }
        }

        None
    }

    /// Splits an `https://host/path` into its two halves — `None` for anything that is not one.
    ///
    /// ⛔ **Only `https://`.** Not a preference: SEC-03 says the one network operation is over
    /// HTTPS, and a plain-HTTP address in the list would be a signature check over a document
    /// anybody on the way could have replaced — the signature would catch the replacement, and
    /// the address would still have leaked to whoever was listening.
    pub fn split_https(url: &str) -> Option<(String, String)> {
        let rest = url.strip_prefix("https://")?;
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));

        if host.is_empty() || host.contains(':') || host.contains('@') {
            return None;
        }

        Some((host.to_owned(), format!("/{path}")))
    }

    /// The document as the author writes it — FR-102.
    #[derive(serde::Deserialize)]
    struct RawFeed {
        schema: u32,
        #[serde(default)]
        item: Vec<RawItem>,
    }

    /// One entry, before the language is chosen.
    #[derive(serde::Deserialize)]
    struct RawItem {
        id: u64,
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        date: Option<Date>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        link: String,
        /// The per-language sub-tables, `ru`, `en`, `uk`… — everything else in the entry.
        #[serde(flatten)]
        languages: std::collections::BTreeMap<String, RawText>,
    }

    /// The two strings of one language.
    #[derive(serde::Deserialize)]
    struct RawText {
        title: String,
        text: String,
    }

    impl RawItem {
        /// The entry with the language chosen — `None` for one this program cannot show.
        ///
        /// **The choice of language is made here and on this machine** (вопрос 101 п. 5): the
        /// interface language, then English, then Russian. A record that has none of the three
        /// is skipped: FR-102 makes `ru` and `en` obligatory, and one the author forgot is one
        /// nobody can read.
        fn into_item(mut self, language: &str) -> Option<FeedItem> {
            let is_update = self.kind == "update";

            if !is_update && self.kind != "news" {
                return None;
            }

            // An update with no version says nothing at all — there is no version to compare.
            if is_update && self.version.is_none() {
                return None;
            }

            let chosen = [language, "en", "ru"]
                .into_iter()
                .find_map(|tag| self.languages.remove(tag))?;

            Some(FeedItem {
                id: self.id,
                date: self.date,
                title: chosen.title,
                text: chosen.text,
                link: self.link,
                version: if is_update { self.version } else { None },
            })
        }
    }
}
