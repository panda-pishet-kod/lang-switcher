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
//! **Nothing here downloads or runs anything, ever** — not in the program. The «Обновление»
//! letter of FR-101 tells a person that a newer version exists and walks them to the download
//! page; since stage Э104 (вопрос 189) the fetching and the installing are the updater's, a
//! separate file without a window and without `uiAccess` ([`crate::update`]), and the letter is
//! its fallback. The single network operation this program is allowed at all — one signed read
//! of the author's feed, no more often than once a day (решение 189.2) — lives in the submodule
//! `feed` of stage Б and nowhere else, so that «which code can open a socket» is a question with
//! a one-line answer; the updater reads through the same submodule.
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
/// a snooze, seven and fourteen to the two reminders, one between feed reads, ninety
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

/// Days between two reads of the feed — FR-102: **one**, решение 189.2 (task T-104-3); fifteen
/// until `e104`. The feed carries the versions the updater installs by itself, and a version
/// learnt of a fortnight late was a fortnight of the old one.
pub const FEED_EVERY_DAYS: i64 = 1;

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

/// The four letters of FR-101.
///
/// «Новость» carries the identifier of the feed entry it is about, because there can be up to
/// [`NEWS_KEPT`] of them and every rule that follows — read, reminded, expired — is about one
/// of them by number.
///
/// ⛔ **«Что нового» больше нет — решение владельца 208.94 п. 4, задача T-119-8.** Письмо было
/// закрыто воротами с решения 118.5 и хранилось на случай возврата; возврата не будет: после
/// важного обновления говорит окно «Обновлено» в углу (208.90–208.94), а не письмо.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Letter {
    /// «Привет» — the first run, when there was no configuration file to start from.
    Welcome,
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

/// ⭐ **The mark of the first run — stage E116, task T-116-12, решение 208.59** (`[letters]
/// first_mark`): the one read of the feed after an installation carries `?first=1`, once.
///
/// * **written as [`FirstMark::Due`] only when this run creates the file** — no `config.toml` at
///   all (`settings::read_from_via`, the road of вопрос 95). An update keeps the file, and a file
///   without the key is a file of an older build: the mark is never sent for it;
/// * **[`FirstMark::Sent`] after any HTTP answer to the marked request**, whatever its status — so
///   the mark goes out at most once. No answer at all (no network, a host that is silent) leaves it
///   due, and the read of FR-102 asks again on its own schedule;
/// * the updater never sends it — it reads the feed by `feed::fetch`, unmarked.
///
/// No schema of its own, as `feed_last_try` and `update_noticed` before it: there is no
/// `deny_unknown_fields`, and an older build reads past the key. A word this build does not know
/// is no mark at all (`None`): never sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstMark {
    /// This run created the file; the next read of the feed carries `?first=1`.
    Due,
    /// The marked read was answered: the mark has gone out and never goes again.
    Sent,
}

impl FirstMark {
    /// The word of the file for this state.
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Due => "due",
            Self::Sent => "sent",
        }
    }

    /// The state a word of the file means; anything else is no mark, which is never sent.
    pub fn from_config_str(text: &str) -> Option<Self> {
        match text {
            "due" => Some(Self::Due),
            "sent" => Some(Self::Sent),
            _ => None,
        }
    }
}

/// ⭐ **Whether the next read of the feed carries `?first=1`** — stage E116, task T-116-12: only
/// while the mark is [`FirstMark::Due`]. No mark (a file of an older build, an unknown word) and a
/// mark already sent answer `false`.
pub const fn first_query(mark: Option<FirstMark>) -> bool {
    matches!(mark, Some(FirstMark::Due))
}

/// ⭐ **The mark after a read of the feed** — stage E116, task T-116-12. `answered` is whether an
/// HTTP answer came back to the marked request, of any status. A due mark answered is sent; every
/// other mark stays what it was — a due one with no answer stays due, and no mark never becomes one.
pub const fn first_mark_after(mark: Option<FirstMark>, answered: bool) -> Option<FirstMark> {
    match (mark, answered) {
        (Some(FirstMark::Due), true) => Some(FirstMark::Sent),
        (other, _) => other,
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
/// 3. **One letter a day.** The two remaining letters share one slot, and the slot is spent
///    by whichever of them is shown.
/// 4. **«Новость» → «Спасибо»** — the priority of FR-101. «Обновление» left the schedule with
///    task T-119-9: it opens only when the person asks for it (see the body).
///
/// ⛔ **«Что нового» в этом порядке нет — решение 118.5 закрыло его ворота, решение 208.94 п. 4
/// (задача T-119-8) убрало само письмо.**
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

    // ⚠ **`last_seen_version` движется без письма** — её ведёт [`initialise`] на каждом старте;
    // по ней [`was_updated`] узнаёт первый старт после обновления.

    // ⛔ **«Обновления» в расписании нет — задача T-119-9, решения 208.98, 208.99.** Since stage
    // Э104 the updater installs a new version by itself, and «Обновление» is its fallback; since
    // 1.8.0 it never comes by itself — no balloon, no dot, no window of its own accord. It opens
    // only from the menu entry «Доступна версия X…» ([`pending`]) or from its row of «Последние
    // письма» ([`list_entries`]), and both stand from the seventh day ([`update_fallback_is_open`]).
    let _ = version;

    if let Some(id) = news_due(state, today, feed) {
        return Some(Letter::News(id));
    }

    if thanks_is_due(state, today) {
        return Some(Letter::Thanks);
    }

    None
}

/// Days the feed must have named a version newer than this build before the entry «Доступна
/// версия X…» of FR-91 and the row of «Последние письма» come — the two doors of the letter
/// «Обновление». **Seven since task T-119-9** (решение 208.98): three until then (TZ E104 §0 п. 20,
/// решение 189.18).
///
/// The updater reads the same feed once a day and installs in the next idle moment (§0 п. 14),
/// and about 95 % of copies have the new version within two days (решение 208.96); a week of a
/// newer version that is still not here is a week of an updater that could not, and only then
/// does the program offer the way by hand — quietly, in the menu, never on its own (208.98).
pub const UPDATE_FALLBACK_DAYS: i64 = 7;

/// **Whether the fallback of an update is open** — §0 п. 20: the feed has named a version newer
/// than this build ([`notice_update`]) for at least [`UPDATE_FALLBACK_DAYS`] days.
pub fn update_fallback_is_open(state: &Letters, today: Date) -> bool {
    state
        .update_noticed
        .is_some_and(|noticed| today.days_since(noticed) >= UPDATE_FALLBACK_DAYS)
}

/// Keeps `[letters] update_noticed` — the day the feed **first** named a version newer than this
/// build — on every good read of the feed (§0 п. 20): set on the first such read, kept while the
/// feed goes on naming a newer one, cleared by a read that names nothing newer. Answers whether it
/// moved.
pub fn notice_update(state: &mut Letters, version: &str, feed: FeedView<'_>, today: Date) -> bool {
    let before = state.update_noticed;

    state.update_noticed = if update_is_pending(version, feed).is_some() {
        before.or(Some(today))
    } else {
        None
    };

    state.update_noticed != before
}

/// **Whether this start is the first after an update** — TZ E104 §0 п. 21: the version last seen
/// on this machine is not empty and is older than this build's. Not on the first run (nothing was
/// seen), not after going back to an older build, and once per version — [`initialise`] moves the
/// version seen on the same tick.
pub fn was_updated(state: &Letters, version: &str) -> bool {
    !state.last_seen_version.is_empty() && version_is_newer(version, &state.last_seen_version)
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

/// Whether the feed may be read today — FR-102: once a day (решение 189.2), the first read
/// happens on the first run, when there is nothing to count from, and **one attempt a day** at
/// most.
///
/// ⭐ Task T-88-1, решение 148.8: a read that fails leaves `feed_last_read` where it was, so
/// the feed stays due — and until `e88` the next tick of the letters started it again, once an
/// hour, for as long as the host did not answer. `feed_last_try` is the day the feed was last
/// asked, whatever came of it; a feed already asked today waits for tomorrow.
pub fn feed_read_is_due(state: &Letters, today: Date) -> bool {
    state.feed
        && state.feed_last_try != Some(today)
        && state
            .feed_last_read
            .is_none_or(|last| today.days_since(last) >= FEED_EVERY_DAYS)
}

/// How many days are left until the next feed read — what the «Новости и обновления» panel
/// says out loud. Zero when a read is due now; one when it is due but the feed was already
/// asked today, because the next attempt is tomorrow (решение 148.8).
pub fn days_until_feed_read(state: &Letters, today: Date) -> i64 {
    let left = state
        .feed_last_read
        .map_or(0, |last| (FEED_EVERY_DAYS - today.days_since(last)).max(0));

    if left == 0 && state.feed_last_try == Some(today) {
        1
    } else {
        left
    }
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
/// Две вещи, и у каждой своё условие:
///
/// * **на всяком старте** — `last_seen_version` подводится к версии сборки. С решения
///   **118.5** (задача T-53a-5) это единственное место, которое её двигает на живой машине:
///   ворота письма «Что нового» закрыты, `after_shown` для него больше не вызывается, и без
///   этой строки отметка застыла бы на версии закрытия ворот. Тогда человек, которому письмо
///   однажды вернут, получил бы его за все пропущенные версии разом. ⚠ Отметка идёт **до**
///   раннего возврата: у машины, считающей дни давно, `first_run` заполнен;
/// * **только когда `first_run` пуст** — `first_run` становится сегодняшним днём, а
///   `thanks_due` — тридцатым от него (FR-101).
///
/// ⛔ **Прежняя третья вещь ушла вместе с воротами.** До T-53a-5 версия помечалась виденной
/// только на свежей установке (`welcome_shown` ещё не поднят), нарочно: обновившийся должен
/// был получить «Что нового», а поставивший программу сегодня — нет, иначе письмо шло бы
/// следом за «Привет» в тот же час. Теперь письма нет ни у кого, и различать эти два случая
/// стало нечем — но условие «свежая установка молчит» соблюдено по-прежнему, и даже строже.
///
/// Answers whether anything was written, because the caller has to save the file when it was.
pub fn initialise(state: &mut Letters, today: Date, version: &str) -> bool {
    // Решение 118.5: отметка виденной версии ведётся ЗДЕСЬ и на каждом старте — см. доктекст
    // выше. Стоит первой, до раннего возврата: у машины, считающей дни давно, `first_run`
    // заполнен, и всё, что ниже, её бы миновало.
    let version_moved = state.last_seen_version != version;

    if version_moved {
        state.last_seen_version = version.to_owned();
    }

    if state.first_run.is_some() {
        return version_moved;
    }

    state.first_run = Some(today);
    state.thanks_due = Some(today.plus_days(THANKS_AFTER_DAYS));

    true
}

/// Records that a letter has just been put on the screen — FR-101, and the only place the
/// state of the letters moves on its own.
///
/// The day is spent by every letter but «Привет», which FR-101 exempts from the one-a-day
/// rule; and each letter marks the thing that stops it coming back:
///
/// * «Привет» — shown, once and for ever;
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

/// The addresses the program links to, and the one rule that governs all of them.
///
/// # What is settled — решения 138, 139.2 и 147.3
///
/// All five addresses are the author's own and permanent. The channel, the support page, the
/// download page and the program's page arrived in one word of the user's (2026-09-17); the
/// feed's followed on 2026-09-25 (147.3, stage Э88). They are string literals here and nothing
/// else — which is what the paragraph below promised the day the placeholders went in.
///
/// No placeholder address is left in this module, and [`is_placeholder`] stays all the same: it
/// is the one question every button with a link stands behind, and the link of a feed entry is
/// written by the author's hand, not by this module. For the five constants it answers «no».
///
/// # Placeholders, and why a button is drawn disabled rather than hidden
///
/// The user's answer (полномочия П5, П7 и П8 of the mandate) was to build the windows now and
/// put a placeholder in each unsettled address — «кнопка пусть будет, вместо пути заглушка». So
/// each is a constant of this module, [`is_placeholder`] recognises a placeholder by its domain,
/// and every button that would open one is drawn in the disabled role the own-drawn buttons
/// already have. The window is the finished window; what is missing is visibly missing.
///
/// `example.invalid` and not a made-up host: RFC 2606 reserves `.invalid` precisely so that it
/// never resolves. A placeholder that fell out of this rule and reached the network would fail
/// slowly, against somebody else's machine; this one fails at once, against nobody's.
///
/// # ⚠ Two different things, and the difference matters
///
/// **The network surface of this program is [`FEED_URLS`] and nothing else** — that is the one
/// address the program itself goes out and *fetches*, unasked, on a timer. The six below are
/// never fetched by this program at all: they are handed to the shell, one at a time, because a
/// person clicked a button, and it is the browser that goes to them. Ворота 3 of
/// `tools\verify-perimeter.ps1` reads the strings of the Release binary and refuses any
/// `https://` that is not one of the seven, which is why every one of them lives here.
pub mod links {
    use crate::settings::Language;

    /// The reserved domain every placeholder address is under.
    const PLACEHOLDER_HOST: &str = "example.invalid";

    /// Where the feed is read from — FR-102, in order, the first good answer winning.
    ///
    /// ⚠ **The whole of the network surface of this program is this array** — see the module's
    /// own doc for what that does and does not mean about the six addresses below.
    ///
    /// The author's own address since `e88` — word of the user, 2026-09-25 (решения 147.3 и
    /// 148.1): the host of the other four, and **one** address, with no second road behind it.
    /// Until then this was a placeholder by decision (138.4), and the guard that said so is now
    /// the test `the_hard_wired_addresses_are_real_and_so_is_the_feed`.
    pub const FEED_URLS: [&str; 1] = ["https://panda-pishet-kod.dev/langswitcher/news.toml"];

    /// The author's channel — FR-103, «Что нового» and the thanks of the wizard: the «Открыть
    /// канал» buttons, the road to the news.
    ///
    /// ⚠ **A channel is read, not written to** — there is nowhere in it for a reader to write
    /// (вопрос 181). So the two places that ask a person to write go to [`feedback_url`] since
    /// решение 181.1: «Скопировать и открыть…» of the wizard of FR-104 and the letter «Спасибо».
    pub const CHANNEL_URL: &str = "https://t.me/panda_pishet_kod";

    /// The topic of Lang Switcher in the author's group — where a person who reads this program
    /// in Russian or Ukrainian writes to the author (решение 181.1). The topic the site sends the
    /// questions about the program to.
    pub const GROUP_URL: &str = "https://t.me/panda_koding/45";

    /// The issues of the program on GitHub — where a person who reads this program in any of the
    /// other twelve languages writes to the author (решение 181.1), as the English pages of the
    /// site send them.
    ///
    /// The list of the issues and not the form of a new one: the address the site and README name,
    /// and one that does not depend on the templates of the repository.
    pub const ISSUES_URL: &str = "https://github.com/panda-pishet-kod/lang-switcher/issues";

    /// The box of the appeals about this program — **the way back when the site did not answer**,
    /// решение 199.3: «отдельная почта … (создать отдельный ящик для обращений по программе)».
    /// The wizard sends an appeal to [`APPEAL_URL`]; when that fails, its last step shows this
    /// address and copies it, and the person writes the letter themselves. Any appeal, a trouble
    /// or an idea, in any language — and not the author's own address (`send.me@…`, решение 181.6,
    /// which the wizard named until 1.2.0).
    ///
    /// ⚠ **Not a link, and not in [`allowed_hosts`].** Nothing opens it: there is no `mailto:` and
    /// no mail program is started. It is shown on the last step of the wizard and copied to the
    /// clipboard by a button. The tables of FR-94 carry it only as `{0}`, so the fourteen
    /// translations hold no copy of it.
    pub const MAIL_ADDRESS: &str = "lang.switcher@panda-pishet-kod.dev";

    /// **Where the wizard sends an appeal** — решения 199.1, 199.7 и 199.9: one `POST` of
    /// [`super::appeal::body`] over HTTPS, when the person presses «Отправить», and at no other
    /// moment.
    ///
    /// ⚠ Not a page and not opened by anything, so not in [`allowed_hosts`] — the shell is never
    /// handed it. It is in this module for the reason [`UPDATE_FILES_URL`] is: ворота 3 of
    /// `tools\verify-perimeter.ps1` judge every `https://` of an image against the addresses
    /// written here, and the image carries this one.
    pub const APPEAL_URL: &str = "https://panda-pishet-kod.dev/langswitcher/feedback";

    /// **Which side of the author's places a person reads, by the language they read the program
    /// in** — решения 181.1, 181.33 и 202.6: the Russian side (the topic of the group, the pages of
    /// the site without `/en/`) for the six languages below, the English one (GitHub Issues, the pages
    /// under `/en/`) for every other. ⚠ Until stage E114 this sentence said «Russian and Ukrainian …
    /// the other twelve», which was true of fourteen languages and stopped being true at sixty-six.
    ///
    /// The one place the line is drawn — [`feedback_url`], [`download_url`] and [`program_url`]
    /// all ask here, so a person is never sent to write on one side and to read on the other. A
    /// `match` without a wildcard on purpose: a language added to [`Language`] does not compile
    /// until somebody has said which side its readers are on.
    ///
    /// ⚠ **Since stage E112 the pages ask [`OWN_PAGES`] first** (Q-206-1): a language of that table
    /// reads the site in its own language and goes on writing to the author on its side here —
    /// Ukrainian to the group, Arabic, Persian and Thai to GitHub Issues.
    ///
    /// ⭐ **Stage E108, решение 202.6** — дословно: «Делаем так, пока нет адаптированных страниц
    /// сайта под каждый из языков»: Kazakh, Belarusian, Tatar and Uzbek in Cyrillic read the
    /// Russian side too; the other twenty-six of group A — the English one.
    /// Stage E109: all twenty-two of group B — the English one (TZ E109 §0 п. 3).
    fn reads_the_russian_side(language: Language) -> bool {
        match language {
            Language::Ru
            | Language::Uk
            | Language::Kk
            | Language::Be
            | Language::Tt
            | Language::UzCyrl => true,
            Language::En
            | Language::De
            | Language::Fr
            | Language::Es
            | Language::Pt
            | Language::It
            | Language::Pl
            | Language::Cs
            | Language::Tr
            | Language::El
            | Language::He
            | Language::Ar
            | Language::Hi
            | Language::Bn
            | Language::Ur
            | Language::Pa
            | Language::Mr
            | Language::Te
            | Language::Ta
            | Language::Fa
            | Language::Gu
            | Language::Th
            | Language::Kn
            | Language::Or
            | Language::Ml
            | Language::Uz
            | Language::Ne
            | Language::As
            | Language::Az
            | Language::Km
            | Language::Sr
            | Language::SrLatn
            | Language::Ug
            | Language::Bg
            | Language::Hy
            | Language::Lo
            | Language::Ka
            | Language::Mk
            | Language::Vi
            | Language::Nl
            | Language::Ro
            | Language::Hu
            | Language::Sv
            | Language::Sq
            | Language::Da
            | Language::Fi
            | Language::Hr
            | Language::Sk
            | Language::Nb
            | Language::Lt
            | Language::Sl
            | Language::Bs
            | Language::Lv
            | Language::Et
            | Language::Cy
            | Language::Mt
            | Language::Is
            | Language::Lb
            | Language::Mi
            | Language::Nn => false,
        }
    }

    /// **Which pages of the site a person without pages of their own reads** — the side of
    /// [`reads_the_russian_side`], and, since stage E114 (task T-114-3, решение 208.6, 1а; Q-208-5),
    /// the Russian pages for Armenian and Uzbek in the Latin script too: a reader of either script of
    /// Uzbek and a reader of Armenian read Russian more often than English, while their channel stays
    /// where it was ([`feedback_url`]). The pages and the channel are two decisions, as in task
    /// T-112-1.
    fn reads_the_russian_pages(language: Language) -> bool {
        reads_the_russian_side(language) || matches!(language, Language::Hy | Language::Uz)
    }

    /// **Where a person writes to the author, by the language they read the program in** —
    /// решение 181.1: Russian and Ukrainian go to the topic of the group, the other twelve to the
    /// issues on GitHub ([`reads_the_russian_side`]).
    ///
    /// The one place that choice is made — the wizard of FR-104 and the letter «Спасибо» both ask
    /// here, with the language of their own window.
    ///
    /// ⚠ **Only the channel.** The pages are another decision ([`reads_the_russian_pages`], stage
    /// E114): Armenian and Uzbek in the Latin script read the pages of the Russian side and still
    /// write to the issues on GitHub.
    pub fn feedback_url(language: Language) -> &'static str {
        if reads_the_russian_side(language) {
            GROUP_URL
        } else {
            ISSUES_URL
        }
    }

    /// **The pages the site has in the language of a person** — Q-206-1, решение 207.3:
    /// `(language, download page, program's page)`. One row is one language, and a language with a
    /// row leaves the pages of its side ([`reads_the_russian_side`]) for these.
    ///
    /// ⛔ **A row only on a line from the controller of the languages of the site** that the pages
    /// are out: the mark outranks the browser and the site remembers it, so `?lang=` of a language
    /// the site does not have would strand the reader. `he` comes next.
    ///
    /// ⚠ The addresses are written out whole and not put together by `concat!`: ворота 3 of
    /// `tools\verify-perimeter.ps1` read the declared addresses as string literals of this file, and
    /// one built by a macro would stand in the image undeclared.
    #[rustfmt::skip]
    ///
    /// ⭐ **Stage E114, task T-114-3** (Q-208-1…4, Q-208-6, Q-208-8, решения 208.6 и 208.8) — fourteen
    /// more, each on the line of the controller that its pages are out (2026-11-10: every address
    /// 200 and in its own `lang`). Both Serbian scripts read `/sr/` under the mark `sr` (the page is
    /// `lang="sr-Latn"`), Portuguese reads `/pt/` under `pt` (the page is `lang="pt-BR"`). In the
    /// order of [`Language::ALL`]. Hungarian after them, on the line of the controller of the same
    /// day (208.1: both addresses 200, `lang=hu`). ⭐ Stage E116, task T-116-6 (Q-208-13 п. 1):
    /// Swedish and Slovak, on the line of the controller of the languages of the site of 2026-11-12;
    /// all four addresses checked in Chrome before the rows were written (2026-11-14: 200,
    /// `lang=sv` and `lang=sk`).
    const OWN_PAGES: [(Language, &str, &str); 21] = [
        (Language::Uk, "https://panda-pishet-kod.dev/uk/langswitcher/download/?lang=uk", "https://panda-pishet-kod.dev/uk/langswitcher/?lang=uk"),
        (Language::De, "https://panda-pishet-kod.dev/de/langswitcher/download/?lang=de", "https://panda-pishet-kod.dev/de/langswitcher/?lang=de"),
        (Language::Fr, "https://panda-pishet-kod.dev/fr/langswitcher/download/?lang=fr", "https://panda-pishet-kod.dev/fr/langswitcher/?lang=fr"),
        (Language::Pt, "https://panda-pishet-kod.dev/pt/langswitcher/download/?lang=pt", "https://panda-pishet-kod.dev/pt/langswitcher/?lang=pt"),
        (Language::It, "https://panda-pishet-kod.dev/it/langswitcher/download/?lang=it", "https://panda-pishet-kod.dev/it/langswitcher/?lang=it"),
        (Language::Pl, "https://panda-pishet-kod.dev/pl/langswitcher/download/?lang=pl", "https://panda-pishet-kod.dev/pl/langswitcher/?lang=pl"),
        (Language::Cs, "https://panda-pishet-kod.dev/cs/langswitcher/download/?lang=cs", "https://panda-pishet-kod.dev/cs/langswitcher/?lang=cs"),
        (Language::Tr, "https://panda-pishet-kod.dev/tr/langswitcher/download/?lang=tr", "https://panda-pishet-kod.dev/tr/langswitcher/?lang=tr"),
        (Language::El, "https://panda-pishet-kod.dev/el/langswitcher/download/?lang=el", "https://panda-pishet-kod.dev/el/langswitcher/?lang=el"),
        (Language::He, "https://panda-pishet-kod.dev/he/langswitcher/download/?lang=he", "https://panda-pishet-kod.dev/he/langswitcher/?lang=he"),
        (Language::Ar, "https://panda-pishet-kod.dev/ar/langswitcher/download/?lang=ar", "https://panda-pishet-kod.dev/ar/langswitcher/?lang=ar"),
        (Language::Fa, "https://panda-pishet-kod.dev/fa/langswitcher/download/?lang=fa", "https://panda-pishet-kod.dev/fa/langswitcher/?lang=fa"),
        (Language::Th, "https://panda-pishet-kod.dev/th/langswitcher/download/?lang=th", "https://panda-pishet-kod.dev/th/langswitcher/?lang=th"),
        (Language::Az, "https://panda-pishet-kod.dev/az/langswitcher/download/?lang=az", "https://panda-pishet-kod.dev/az/langswitcher/?lang=az"),
        (Language::Sr, "https://panda-pishet-kod.dev/sr/langswitcher/download/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/?lang=sr"),
        (Language::SrLatn, "https://panda-pishet-kod.dev/sr/langswitcher/download/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/?lang=sr"),
        (Language::Bg, "https://panda-pishet-kod.dev/bg/langswitcher/download/?lang=bg", "https://panda-pishet-kod.dev/bg/langswitcher/?lang=bg"),
        (Language::Nl, "https://panda-pishet-kod.dev/nl/langswitcher/download/?lang=nl", "https://panda-pishet-kod.dev/nl/langswitcher/?lang=nl"),
        (Language::Hu, "https://panda-pishet-kod.dev/hu/langswitcher/download/?lang=hu", "https://panda-pishet-kod.dev/hu/langswitcher/?lang=hu"),
        (Language::Sv, "https://panda-pishet-kod.dev/sv/langswitcher/download/?lang=sv", "https://panda-pishet-kod.dev/sv/langswitcher/?lang=sv"),
        (Language::Sk, "https://panda-pishet-kod.dev/sk/langswitcher/download/?lang=sk", "https://panda-pishet-kod.dev/sk/langswitcher/?lang=sk"),
    ];

    /// The row of [`OWN_PAGES`] for this language, if it has one.
    fn own_pages(language: Language) -> Option<(Language, &'static str, &'static str)> {
        OWN_PAGES
            .iter()
            .copied()
            .find(|(own, _, _)| *own == language)
    }

    /// The support page — FR-103, one link and no payment details of any kind in the program
    /// (вопрос 101 п. 6).
    pub const SUPPORT_URL: &str = "https://panda-pishet-kod.dev/donate/";

    /// The download page — FR-103, the target of «Открыть страницу загрузки».
    ///
    /// # Why the build's own address beats the one in the feed entry
    ///
    /// An `update` entry of the feed carries a `link` of its own, and until task T-78-2 that
    /// link was what the button opened. It loses to this constant on one argument: a feed entry
    /// is **signed and dated**, so its address is as old as the entry and goes stale, whereas
    /// this page is permanent and always shows the current version. Word of the user, 2026-09-17:
    /// «если новость об обновлении пришла, значит оно уже лежит на сайте, и перейдя по ссылке
    /// загрузки программы, мы уже попадаем на актуальную версию».
    ///
    /// The `link` field stays in the feed format — it is the format's, not ours — and this
    /// program simply no longer reads it for this button.
    ///
    /// ⭐ **The Russian page** — since решение 181.33 the button opens [`download_url`] of the
    /// language of its window, and this is the page of the Russian side, marked `?lang=ru` for the
    /// site (решение 181.35, see [`download_url`]).
    pub const DOWNLOAD_URL: &str = "https://panda-pishet-kod.dev/langswitcher/download/?lang=ru";

    /// The English download page — the pair of [`DOWNLOAD_URL`] under `/en/`, решение 181.33, marked
    /// `?lang=en` (решение 181.35): what «Open the download page» opens for a person who reads the
    /// program in any of the twelve languages of the English side.
    pub const DOWNLOAD_URL_EN: &str =
        "https://panda-pishet-kod.dev/en/langswitcher/download/?lang=en";

    /// **The download page in the language a person reads the program in** — [`OWN_PAGES`], or else
    /// решение 181.33 ([`reads_the_russian_side`]): «Открыть страницу загрузки» of «От автора» and
    /// of the letter about an update.
    ///
    /// ⚠ **Why each address carries `?lang=`** — решение 181.35. The page alone does not decide the
    /// language a person sees: the script of the language of a page on the site (П-179 of the site)
    /// moves a person to the choice they made with the site's own language button, or else to the
    /// language of the browser (поправка 181.34). The mark tells the site the language of the
    /// program; the name `lang` and its values are the ones the site already gives that word. A
    /// site that does not read the mark yet simply ignores it.
    pub fn download_url(language: Language) -> &'static str {
        match own_pages(language) {
            Some((_, download, _)) => download,
            None if reads_the_russian_pages(language) => DOWNLOAD_URL,
            None => DOWNLOAD_URL_EN,
        }
    }

    /// The program's own page — the target of a click on the name «Lang Switcher» in «О
    /// программе», task T-78-3.
    ///
    /// The only address of the five with no button of its own: it is opened by a click on the
    /// name itself, which is a push button without a border and looks like the label it used to
    /// be until the pointer is over it.
    ///
    /// ⭐ **The Russian page** — since решение 181.33 the name opens [`program_url`] of the
    /// language of its window; marked `?lang=ru` for the site (решение 181.35).
    pub const PROGRAM_URL: &str = "https://panda-pishet-kod.dev/langswitcher/?lang=ru";

    /// The English page of the program — the pair of [`PROGRAM_URL`] under `/en/`, решение 181.33,
    /// marked `?lang=en` (решение 181.35).
    pub const PROGRAM_URL_EN: &str = "https://panda-pishet-kod.dev/en/langswitcher/?lang=en";

    /// **The program's page in the language a person reads the program in** — [`OWN_PAGES`], or else
    /// решение 181.33 ([`reads_the_russian_side`]): the name in «О программе» and in «От автора». The
    /// mark of the language is there for the reason [`download_url`] gives.
    pub fn program_url(language: Language) -> &'static str {
        match own_pages(language) {
            Some((_, _, program)) => program,
            None if reads_the_russian_pages(language) => PROGRAM_URL,
            None => PROGRAM_URL_EN,
        }
    }

    /// The page «Что нового» — the target of a click on the notification «Lang Switcher обновлён
    /// до X» (TZ E104 §0 п. 5, task T-104-7): the Russian side, marked `?lang=ru` for the site
    /// (решение 181.35).
    pub const CHANGELOG_URL: &str = "https://panda-pishet-kod.dev/langswitcher/changelog/?lang=ru";

    /// The English page «What's new» — the pair of [`CHANGELOG_URL`] under `/en/`, marked
    /// `?lang=en`.
    pub const CHANGELOG_URL_EN: &str =
        "https://panda-pishet-kod.dev/en/langswitcher/changelog/?lang=en";

    /// The English page «What's new» **with no mark** — for a language of [`OWN_PAGES`], whose page
    /// «Что нового» the site does not have yet (Q-206-1). ⛔ `?lang=en` would take the reader off
    /// their language: the mark outranks the browser and the site remembers it.
    pub const CHANGELOG_URL_UNMARKED: &str =
        "https://panda-pishet-kod.dev/en/langswitcher/changelog/";

    /// **The page «Что нового» in the language a person reads the program in** — the side of
    /// [`reads_the_russian_side`], as for the two pages above; a language of [`OWN_PAGES`] has no
    /// such page of its own and reads [`CHANGELOG_URL_UNMARKED`].
    pub fn changelog_url(language: Language) -> &'static str {
        match own_pages(language) {
            Some(_) => CHANGELOG_URL_UNMARKED,
            None if reads_the_russian_pages(language) => CHANGELOG_URL,
            None => CHANGELOG_URL_EN,
        }
    }

    /// **The one folder the updater takes an installer from** — TZ E104 §0 п. 7 и 15: the
    /// author's site, `/langswitcher/files/`, and nowhere else; no second road (решение 189.10).
    ///
    /// ⚠ Not a page and not opened by anything: the program never touches it. It is here because
    /// ворота 3 of `tools\verify-perimeter.ps1` judge every `https://` of an image against this
    /// module, and the updater carries this one ([`crate::update::decide`] holds the address of
    /// a `[download]` against it).
    pub const UPDATE_FILES_URL: &str = "https://panda-pishet-kod.dev/langswitcher/files/";

    /// Whether this address is one of the placeholders — the one test every button that opens
    /// a link is behind.
    pub fn is_placeholder(url: &str) -> bool {
        url.contains(PLACEHOLDER_HOST)
    }

    /// Whether the program has a real feed address to read from at all.
    pub fn feed_is_configured() -> bool {
        FEED_URLS.iter().any(|url| !is_placeholder(url))
    }

    /// The one scheme this program opens anything under — finding С52, task T-41-1.
    const HTTPS_SCHEME: &str = "https://";

    /// The host of an `https://` address, or `None` when the string is not one.
    ///
    /// The scheme is compared **without regard to case**, because `HTTPS://` is the same scheme
    /// to every browser and a check that missed it would be a check anybody could walk past.
    ///
    /// The host ends at the first `/`, `\`, `?` or `#`. The backslash is not decoration: every
    /// browser reads `https://evil.example\ours.invalid/` as a page at `evil.example`, so a
    /// reader that did not stop there would compare a host no browser will ever visit.
    ///
    /// `:` and `@` in what is left make the answer `None` rather than a shorter host — a port
    /// and a set of credentials are both ways of writing a host that is not the host it looks
    /// like, and none of the seven addresses of this program has either. The same two
    /// characters are refused for the same reason by [`super::feed::split_https`].
    fn host_of(url: &str) -> Option<&str> {
        if !url
            .get(..HTTPS_SCHEME.len())
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case(HTTPS_SCHEME))
        {
            return None;
        }

        let host = url[HTTPS_SCHEME.len()..]
            .split(['/', '\\', '?', '#'])
            .next()
            .unwrap_or_default();

        (!host.is_empty() && !host.contains(':') && !host.contains('@')).then_some(host)
    }

    /// Every host this program is allowed to reach — the constants above **and nothing else**.
    ///
    /// Read out of the constants rather than written down a second time: a separate list is a
    /// second place to forget, and when the author's real addresses replaced the placeholders
    /// (task T-78-1) this gate followed them with no edit but the two new names below.
    ///
    /// ⚠ Every **address** of this module belongs in this chain — [`MAIL_ADDRESS`] is not one: it
    /// is shown and copied, never opened — and being on a host some other
    /// constant already names is **not** a reason to leave one out: that is an accident of where
    /// the author happens to host the pages today, and the day one of them moves, the address
    /// would stop opening for a reason nobody could see from the line that changed.
    fn allowed_hosts() -> impl Iterator<Item = &'static str> {
        FEED_URLS
            .iter()
            .copied()
            .chain([
                CHANNEL_URL,
                GROUP_URL,
                ISSUES_URL,
                SUPPORT_URL,
                DOWNLOAD_URL,
                DOWNLOAD_URL_EN,
                PROGRAM_URL,
                PROGRAM_URL_EN,
                CHANGELOG_URL,
                CHANGELOG_URL_EN,
                CHANGELOG_URL_UNMARKED,
            ])
            .chain(
                OWN_PAGES
                    .iter()
                    .flat_map(|(_, download, program)| [*download, *program]),
            )
            .filter_map(host_of)
    }

    /// Whether this program may hand `url` to the shell — finding С52, task T-41-1.
    ///
    /// # Why a white list, and why here
    ///
    /// The process carries `uiAccess`, so **whatever `ShellExecuteW` starts inherits that pass**
    /// — the raised rights and the right to reach across the window boundary Windows otherwise
    /// keeps. The address comes out of the feed; the feed is signed; and a signature attests the
    /// author, not the safety of a string. `file:///…`, `ms-settings:`, a UNC path, a path to an
    /// `.exe` and a foreign host would every one of them be opened by the shell, and one typo in
    /// the author's TOML is enough to do it.
    ///
    /// Pure, allocation-free, and **the answer does not depend on the placeholder check**: that
    /// one is a separate fact about a separate question (is there a real address yet), and
    /// [`is_placeholder`] goes on answering it beside this.
    pub fn is_allowed(url: &str) -> bool {
        host_of(url)
            .is_some_and(|host| allowed_hosts().any(|allowed| allowed.eq_ignore_ascii_case(host)))
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
    moment_is_quiet_for(QUIET_IDLE_SECONDS)
}

/// [`moment_is_quiet`] with a quiet of `seconds` — the window «Обновлено» waits five without a key
/// (task T-119-6), a letter thirty. The same two calls, the same clock, the same refusals.
pub fn moment_is_quiet_for(seconds: u32) -> bool {
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
    now.wrapping_sub(info.dwTime) / 1000 >= seconds
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

/// Resource identifiers of the two templates, mirrored from `app.rc` by hand — the same rule
/// every identifier of `settings` is kept by, and for the same reason: a `.rc` file and a Rust
/// file share no header, and a mismatch shows up as a missing resource rather than the wrong
/// one. The letters and «Последние письма» are windows of the shell (T-120-16) and have none.
///
/// The «От автора» window — FR-103.
pub const IDD_AUTHOR: u16 = 204;
/// The wizard «Написать автору» — FR-104.
pub const IDD_WIZARD: u16 = 205;

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
/// «Адрес для письма: …» — the last step when the site did not answer, задача T-107-3 (решение
/// 199.3; until 1.2.0 — a trouble's last step, задача T-103-7).
const IDC_WZ_ADDRESS: i32 = 1355;
/// «Скопировать адрес» — the same step.
const IDC_WZ_COPY_ADDRESS: i32 = 1356;
/// «Как с вами связаться…» — the label of the contact, задача T-107-3 (решения 199.10, 199.11).
const IDC_WZ_CONTACT_LABEL: i32 = 1357;
/// The contact itself — one line, at most [`appeal::CONTACT_MAX`] characters.
const IDC_WZ_CONTACT: i32 = 1358;

/// Every owner-drawn **static** of the wizard — the gate of `WM_DRAWITEM`, and it is written
/// out in full for the reason a list of runs needs: a run written short
/// is a row that never gets painted, and the screenshot is what finds it.
const WIZARD_LABELS: [i32; 25] = [
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
    IDC_WZ_ADDRESS,
    IDC_WZ_CONTACT_LABEL,
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
/// boxes and the six push buttons.
const WIZARD_BUTTONS: [i32; 21] = [
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
    IDC_WZ_COPY_ADDRESS,
    IDC_WZ_SAVE,
    IDC_WZ_CANCEL,
    IDC_WZ_BACK,
    IDC_WZ_NEXT,
];

/// Whether this label stands **on** a panel, and is therefore filled with the panel's own
/// colour — an owner-drawn static answers for the whole of its rectangle, so a
/// window-coloured fill on a block would cut a hole in it.
///
/// A function over ranges and not a list: the cards of the wizard are numbered in runs, and a
/// list is a place where a run can be written short — which is exactly the defect the first
/// screenshot of «Привет» showed, in the twin of this rule.
///
/// The two windows are answered by one body because their identifiers are disjoint: every
/// control of every template has a number of its own, so «is this label on a block» is a
/// question about the number and not about the window.
///
/// ⭐⭐ **Labels only until task T-80-1, and that was the defect.** A button is owner-drawn too,
/// and its drawing cuts a rounded figure out of the rectangle and fills what it cuts away with
/// the ground — so a button standing on a panel but told «window» came out with four corners in
/// the wrong colour: 18/16/13 off the panel in «Туман», 7/8/9 in «Графит», measured. The user saw
/// exactly that on `e79` and named the cure himself — one mechanism for all of them, not a second
/// one beside it. Hence this function is no longer about labels, and its name says so.
///
pub fn stands_on_a_panel(control: i32) -> bool {
    // «От автора» stood on three panels until it became a window of the shell (T-120-17). The two
    // lines of every card of the wizard stand **on** that card, and the card is drawn with the panel
    // brush — see `draw_card`, which never changes its fill for exactly this reason.
    (IDC_WZ_CARD_T1..IDC_WZ_CARD_T1 + 3).contains(&control)
        || (IDC_WZ_CARD_S1..IDC_WZ_CARD_S1 + 3).contains(&control)
}

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
    /// «Открыть группу» / «Open GitHub Issues» of «Спасибо» — [`links::feedback_url`] of the
    /// language the window is built in, where a person can write (решение 181.1).
    OpenFeedback,
    /// «Открыть страницу поддержки» — `links::SUPPORT_URL`.
    OpenSupport,
    /// «Открыть страницу загрузки» — the link the `update` entry of the feed carries.
    OpenDownload,
    /// The page of the application — the click on the program's name in the head of «От автора»,
    /// задача T-81-3. The same address the same name opens in «О программе».
    OpenProgram,
    /// «Открыть ссылку» — the link a `news` entry carries.
    OpenLink,
    /// The «От автора» window — FR-103.
    OpenAuthor,
    /// The «Последние письма» window — FR-101.
    OpenLetters,
    /// The wizard of FR-104. Stage В; until then the button that would ask for it is absent.
    OpenWizard,
}

impl Action {
    /// Whether the action hands an address to the browser — its button carries the sign ↗
    /// (passport §6); a button that opens a window of the program or acts on the letter does not.
    pub const fn opens_browser(self) -> bool {
        match self {
            Action::OpenChannel
            | Action::OpenFeedback
            | Action::OpenSupport
            | Action::OpenDownload
            | Action::OpenProgram
            | Action::OpenLink => true,
            Action::Close
            | Action::Snooze
            | Action::MarkRead
            | Action::OpenSettings
            | Action::OpenAuthor
            | Action::OpenLetters
            | Action::OpenWizard => false,
        }
    }
}

/// One button of a letter: what it says, what it does, and whether it can be pressed.
///
/// `enabled` is false for one of two reasons: the address it would open is still a placeholder
/// (полномочия П7 and П8), or its host is not one of the program's own — the gate of
/// [`links::is_allowed`], task T-41-1, finding С52 (see [`Button::link`]). The button is **drawn
/// and disabled** rather than hidden, because that is what the user asked for: «кнопка пусть
/// будет, вместо пути заглушка».
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
    ///
    /// **Task T-41-1, finding С52:** and disabled just the same for an address the gate of
    /// [`links::is_allowed`] turns away. The layout does not move — the button keeps its place
    /// and its word, exactly as it does for a placeholder — because the appearance is frozen and
    /// because a button that vanishes says less than one that is plainly not available. The
    /// address is refused at the door as well; this is the half the person can see.
    fn link(label: String, action: Action, url: &str) -> Self {
        Self {
            label,
            action,
            enabled: links::is_allowed(url) && !links::is_placeholder(url),
        }
    }
}

/// One row of a numbered list of a letter: its number and the sentence beside it.
///
/// The marker is «1», «2», «3» — a literal, the same in every locale; an empty marker is a line
/// that follows the list without a number (the last line of «Как обновиться»).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The marker.
    pub marker: String,
    /// The sentence. May carry [`theme::KEY_PLACEHOLDER`] — the name of the key goes there.
    pub text: String,
}

impl Row {
    /// The sentence — for the description of the shell, which names no `text` (the guard of the
    /// window descriptions, rule 2).
    pub fn words(&self) -> &str {
        &self.text
    }
}

/// One of the pair of action cards of «Спасибо»: a heading, a sentence and the button under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub title: String,
    /// The sentence under the heading.
    pub words: String,
    pub button: Button,
}

/// **What a letter says** — the pure description the window of the shell is built from
/// (`ui::windows::letter`), part by part in the order of the window: the head, the notice, the
/// lead, the example with the key, the paragraph, the entry of the feed, the numbered list, the
/// pair of cards, the link, the line under them and the row of buttons. An empty part is a part
/// this letter does not have.
///
/// Pure, and that is the point — [`plan_for`] reads the string tables and the state and answers
/// this, so every rule about what a letter says is checked in a test with no window at all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LetterPlan {
    /// The window's own caption.
    pub caption: String,
    /// The line over the heading — whose letter this is; never a version (T-120-16 п. 4).
    pub eyebrow: String,
    /// The heading.
    pub title: String,
    /// The quiet line under the heading — the versions, a date.
    pub subtitle: String,
    /// The notice in its own block — what happened, before anything else is said.
    pub note: String,
    /// The opening paragraph.
    pub lead: String,
    /// Whether the example of «Привет» is shown: the word typed, the key, the word fixed.
    pub demo: bool,
    /// The pair the example shows — [`demo_pair`] of the language the plan is built in.
    pub demo_pair: DemoPair,
    /// The sentence under the example. May carry [`theme::KEY_PLACEHOLDER`].
    pub demo_caption: String,
    /// A paragraph of the body.
    pub para: String,
    /// The heading of the entry of the feed a «Новость» is about — the author's words.
    pub panel_title: String,
    /// The label over the author's words about a release — «Что нового в X».
    pub whats_new: String,
    /// The author's text: of the news entry, or of the release.
    pub panel_text: String,
    /// The label over the numbered list; empty means the letter has no list.
    pub panel: String,
    /// The numbered list.
    pub rows: Vec<Row>,
    /// The pair of action cards.
    pub cards: Vec<Card>,
    /// A link to the site under the body.
    pub link: Option<Button>,
    /// The line under the body — what the buttons do to the letter.
    pub foot: String,
    /// The first button of the row.
    pub left: Option<Button>,
    /// The second plain button of the row.
    pub right: Option<Button>,
    /// The main button — the rightmost, the one Enter presses.
    pub accent: Option<Button>,
}

impl LetterPlan {
    /// The buttons of the row at the bottom, in their order: the main one last.
    pub fn buttons(&self) -> impl Iterator<Item = &Button> {
        [&self.left, &self.right, &self.accent]
            .into_iter()
            .filter_map(Option::as_ref)
    }

    /// Every button of the letter: the cards', the link and the row at the bottom.
    pub fn all_buttons(&self) -> impl Iterator<Item = &Button> {
        self.cards
            .iter()
            .map(|card| &card.button)
            .chain(self.link.as_ref())
            .chain(self.buttons())
    }
}

/// Everything [`plan_for`] needs that is not in the string tables.
#[derive(Debug, Clone)]
pub struct PlanContext<'a> {
    /// This build's version, as `VERSIONINFO` spells it.
    pub version: String,
    /// The hotkey as it acts. Unused by the plan: the name of the key goes into the window when it
    /// is built (`ui::windows::letter`), in place of [`theme::KEY_PLACEHOLDER`].
    pub hotkey: String,
    /// Whether «Спасибо» may still offer its one snooze.
    pub snooze_offered: bool,
    /// The feed entry a letter out of the feed is about.
    pub item: Option<&'a FeedItem>,
}

/// What one letter says in the language of the program — [`plan_in`] of
/// `settings::ui_language()`.
pub fn plan_for(letter: Letter, context: &PlanContext<'_>) -> LetterPlan {
    plan_in(settings::ui_language(), letter, context)
}

/// What one letter says in `language` — **the whole of FR-101's text side**, and a pure function
/// of the string tables, the state and the feed. In any language, for the instrument of fitting,
/// which goes over all of them.
pub fn plan_in(language: Language, letter: Letter, context: &PlanContext<'_>) -> LetterPlan {
    use crate::settings::{
        IDS_CLOSE, IDS_FALLBACK_NOTICE, IDS_FALLBACK_REPEATS, IDS_FALLBACK_SUB, IDS_FALLBACK_TITLE,
        IDS_FALLBACK_WHATS_NEW, IDS_FEEDBACK_OPEN, IDS_FEEDBACK_PANEL, IDS_FEEDBACK_TEXT,
        IDS_HELLO_DEMO_CAP, IDS_HELLO_FOOT, IDS_HELLO_LEAD, IDS_HELLO_OK, IDS_HELLO_PANEL,
        IDS_HELLO_ROW_1, IDS_HELLO_ROW_2, IDS_HELLO_ROW_3, IDS_HELLO_SETTINGS, IDS_HELLO_TITLE,
        IDS_LETTER_CAPTION, IDS_NEWS_DOWNLOAD, IDS_NEWS_FOOT, IDS_NEWS_LATER,
        IDS_NEWS_LETTER_TITLE, IDS_NEWS_OPEN_LINK, IDS_NEWS_READ_BUTTON, IDS_SUPPORT_OPEN,
        IDS_SUPPORT_PANEL, IDS_SUPPORT_TEXT, IDS_THANKS_FOOT, IDS_THANKS_LEAD, IDS_THANKS_SNOOZE,
        IDS_THANKS_TITLE, IDS_UPDATE_FOOT, IDS_UPDATE_HOW, IDS_UPDATE_STEP_1, IDS_UPDATE_STEP_2,
        IDS_UPDATE_STEP_3, IDS_WRITE_TO_AUTHOR,
    };

    let text = |id| settings::text_in(language, id);
    let format_text = |id, arguments: &[&str]| {
        let mut filled = text(id);
        for (index, argument) in arguments.iter().enumerate() {
            filled = filled.replace(&format!("{{{index}}}"), argument);
        }
        filled
    };
    let numbered = |ids: [u16; 3]| -> Vec<Row> {
        ids.iter()
            .zip(1..)
            .map(|(id, number)| Row {
                marker: number.to_string(),
                text: text(*id),
            })
            .collect()
    };

    let caption = text(IDS_LETTER_CAPTION);
    // Whose letter it is, over the heading of each of the four: the caption's own words. No version
    // stands there (T-120-16 п. 4) — the only window with a number over its title is «О программе».
    let eyebrow = caption.clone();

    match letter {
        Letter::Welcome => LetterPlan {
            caption,
            eyebrow,
            title: text(IDS_HELLO_TITLE),
            lead: text(IDS_HELLO_LEAD),
            demo: true,
            demo_pair: demo_pair(language),
            demo_caption: text(IDS_HELLO_DEMO_CAP),
            panel: text(IDS_HELLO_PANEL),
            rows: numbered([IDS_HELLO_ROW_1, IDS_HELLO_ROW_2, IDS_HELLO_ROW_3]),
            left: Some(Button::live(text(IDS_HELLO_SETTINGS), Action::OpenSettings)),
            accent: Some(Button::live(text(IDS_HELLO_OK), Action::Close)),
            // 208.106 п. 3: pause and settings by a right-click on the icon, the flag in the settings —
            // what the third row of the old letter said of the icon, said for the icon of 1.8.0.
            foot: text(IDS_HELLO_FOOT),
            ..LetterPlan::default()
        },

        Letter::Thanks => LetterPlan {
            caption,
            eyebrow,
            title: text(IDS_THANKS_TITLE),
            lead: text(IDS_THANKS_LEAD),
            // No paragraph after the lead (C6 of T-120-16, ◆ word of the controller): it said again what
            // the pair of cards and the link under them say, and the window did not fit 620 × 500.
            // The pair of action cards (T-120-16 п. 1): to support the author — the page of
            // support, in the browser; to write to him — the wizard of the program, which says
            // more than a page could.
            cards: vec![
                Card {
                    title: text(IDS_SUPPORT_PANEL),
                    words: text(IDS_SUPPORT_TEXT),
                    button: Button::link(
                        text(IDS_SUPPORT_OPEN),
                        Action::OpenSupport,
                        links::SUPPORT_URL,
                    ),
                },
                Card {
                    title: text(IDS_FEEDBACK_PANEL),
                    words: text(IDS_FEEDBACK_TEXT),
                    button: Button::live(text(IDS_WRITE_TO_AUTHOR), Action::OpenWizard),
                },
            ],
            // Решение 181.1: «Если понравилось, расскажите об этом» — where a person can write,
            // by language: the group, or GitHub Issues. A link, because it only goes to the site.
            link: Some(Button::link(
                text(IDS_FEEDBACK_OPEN),
                Action::OpenFeedback,
                links::feedback_url(language),
            )),
            // FR-101: the snooze is offered once, and the second showing has no button where
            // it stood — not a disabled one. A disabled «Напомнить позже» would say the program
            // is refusing; an absent one says the letter is on its last showing, which is what
            // the line under it says too.
            left: context
                .snooze_offered
                .then(|| Button::live(text(IDS_THANKS_SNOOZE), Action::Snooze)),
            accent: Some(Button::live(text(IDS_CLOSE), Action::Close)),
            foot: text(IDS_THANKS_FOOT),
            ..LetterPlan::default()
        },

        // The two letters out of the feed. A letter with no entry behind it is a window that must
        // not open: the plan comes back empty and the caller refuses.
        Letter::Update | Letter::News(_) => {
            let Some(item) = context.item else {
                return LetterPlan::default();
            };

            let released = item.date.map(format_date).unwrap_or_default();

            if matches!(letter, Letter::Update) {
                // ⭐ «Обновление» — the honest letter of task T-119-9 (решения 208.95–208.99). It
                // opens only when the person asks — the menu entry or the row of «Последние
                // письма», from the seventh day — so it says what happened, in the notice: the
                // program could not put the version in by itself. The release date stands as a
                // date and not as a count of days; the author's own words about the release come
                // under «Что нового в X»; the three steps, and the line that sends to the author
                // when it repeats, with the wizard among the buttons (208.97 п. 2).
                let announced = item.version.clone().unwrap_or_default();

                return LetterPlan {
                    caption,
                    eyebrow,
                    title: text(IDS_FALLBACK_TITLE),
                    subtitle: format_text(
                        IDS_FALLBACK_SUB,
                        &[&announced, &context.version, &released],
                    ),
                    note: format_text(IDS_FALLBACK_NOTICE, &[&announced, &released]),
                    whats_new: format_text(IDS_FALLBACK_WHATS_NEW, &[&announced]),
                    panel_text: item.text.clone(),
                    panel: text(IDS_UPDATE_HOW),
                    rows: numbered([IDS_UPDATE_STEP_1, IDS_UPDATE_STEP_2, IDS_UPDATE_STEP_3])
                        .into_iter()
                        .chain([Row {
                            marker: String::new(),
                            text: text(IDS_FALLBACK_REPEATS),
                        }])
                        .collect(),
                    left: Some(Button::live(text(IDS_WRITE_TO_AUTHOR), Action::OpenWizard)),
                    right: Some(Button::live(text(IDS_CLOSE), Action::Close)),
                    accent: Some(Button::link(
                        text(IDS_NEWS_DOWNLOAD),
                        Action::OpenDownload,
                        &item.link,
                    )),
                    foot: format_text(IDS_UPDATE_FOOT, &[&announced]),
                    ..LetterPlan::default()
                };
            }

            // «Новость» — the window says whose news it is, the body says which news. The entry's
            // own heading goes **into the body** and not into the window's title: the title
            // belongs to the program, and a heading out of the feed in that place would let the
            // author write anything at all where the program speaks.
            LetterPlan {
                caption,
                eyebrow,
                title: text(IDS_NEWS_LETTER_TITLE),
                subtitle: released,
                panel_title: item.title.clone(),
                panel_text: item.text.clone(),
                // FR-101: closing is postponing, and the button says so. «Закрыть» here would
                // promise an end the letter does not give — it comes back in a week.
                left: Some(Button::live(text(IDS_NEWS_LATER), Action::Close)),
                // Task T-41-1, finding С52: an address the gate refuses gets no button at all —
                // and since `e88` a placeholder takes that road too (task T-88-1, решение 148.7).
                right: links::is_allowed(&item.link)
                    .then(|| Button::link(text(IDS_NEWS_OPEN_LINK), Action::OpenLink, &item.link)),
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

/// How long one frame of the demonstration lasts, in milliseconds — the tick the window of
/// «Привет» is given (T-120-25).
pub const DEMO_TICK_MS: u32 = 130;

/// How many frames one turn of the demonstration takes.
pub const DEMO_FRAMES: u32 = 48;

/// The frame the demonstration stands at while the window has no tick — Windows moves nothing
/// (its animation is off) or the window is not shown: the word fixed, the key up, what the example
/// is for (T-120-25: «анимация отключена — стоит итоговый кадр»).
pub const DEMO_STILL: u32 = 20;

/// One pair of the demonstration — the word as the wrong layout typed it, the word it becomes, and
/// the two names of the layout indicator.
///
/// Literals of the code rather than strings of the tables: they are **the example itself** — the
/// very keys a person presses — and translating them would destroy what the picture shows. So are
/// the names of the indicator: they are the names of the layouts in the picture, and a layout is
/// named in its own language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DemoPair {
    /// The word typed in the wrong layout — what stands in the field before the press.
    pub typed: &'static str,
    /// What [`Self::typed`] becomes after the press.
    pub fixed: &'static str,
    /// The layout indicator: `[before the press, after it]`.
    pub badge: [&'static str; 2],
}

/// **The pair of the demonstration in one language of the interface — решение 178.6** (вопрос 178,
/// the mock-up of version 2, задача T-102-2): a word in the reader's own language typed with the
/// English layout on, and the word it becomes. Russian keeps the pair it always had. English has
/// no layout of its own to be typed in by mistake, and the Polish layout of the programmers is the
/// English one with `AltGr` (178.5), so both show the pair the other way round, «руддщ» → «hello».
///
/// ⛔ Every word was taken off the keys of the Windows layouts — the tables of their own DLLs,
/// character → key → the same key in the other layout, and back (178.4, 178.5) — and not out of
/// memory: the lesson of Э87, «цреф» is not «what». A word changed here goes through that
/// instrument again, and through the canon of `tests\letters.rs`, written apart from this table.
///
/// A `match` without a wildcard on purpose: a language added to [`Language`] does not compile
/// until it has a pair. The typed words are four to eight letters long — [`demo_frame`] types one
/// a frame and must be done before the press at frame 14. Hebrew and Arabic: the field is not
/// mirrored (`unmirror_control` below), so their row reads left to right as everyone's does — the
/// Latin letters typed, then the word itself after the press (§0 п. 6 of TZ-E102).
pub fn demo_pair(language: Language) -> DemoPair {
    match language {
        Language::Ru => DemoPair {
            typed: "ghbdtn",
            fixed: "привет",
            badge: ["EN", "RU"],
        },
        Language::En => DemoPair {
            typed: "руддщ",
            fixed: "hello",
            badge: ["RU", "EN"],
        },
        Language::Uk => DemoPair {
            typed: "ghbdsn",
            fixed: "привіт",
            badge: ["EN", "UK"],
        },
        Language::De => DemoPair {
            typed: "Gr[-e",
            fixed: "Grüße",
            badge: ["EN", "DE"],
        },
        Language::Fr => DemoPair {
            typed: "sqlut",
            fixed: "salut",
            badge: ["EN", "FR"],
        },
        Language::Es => DemoPair {
            typed: "ma;ana",
            fixed: "mañana",
            badge: ["EN", "ES"],
        },
        Language::Pt => DemoPair {
            typed: "for;a",
            fixed: "força",
            badge: ["EN", "PT"],
        },
        Language::It => DemoPair {
            typed: "caff[",
            fixed: "caffè",
            badge: ["EN", "IT"],
        },
        Language::Pl => DemoPair {
            typed: "руддщ",
            fixed: "hello",
            badge: ["RU", "EN"],
        },
        Language::Cs => DemoPair {
            typed: "d2kuji",
            fixed: "děkuji",
            badge: ["EN", "CS"],
        },
        Language::Tr => DemoPair {
            typed: "g]naydin",
            fixed: "günaydın",
            badge: ["EN", "TR"],
        },
        Language::El => DemoPair {
            typed: "geia",
            fixed: "γεια",
            badge: ["EN", "EL"],
        },
        Language::He => DemoPair {
            typed: "akuo",
            fixed: "שלום",
            badge: ["EN", "HE"],
        },
        Language::Ar => DemoPair {
            typed: "lvpfh",
            fixed: "مرحبا",
            badge: ["EN", "AR"],
        },
        // Stage E108, the trial of решение 202.1 — the words of the translators, the keys taken
        // off the tables of KBDINHIN.DLL (and KBDINDEV.DLL, the same keys) and KBDURDU.DLL against
        // KBDUS.DLL (`scratchpad-E108\pairs-e108.out.txt`).
        Language::Hi => DemoPair {
            typed: "vcmdls",
            fixed: "नमस्ते",
            badge: ["EN", "HI"],
        },
        Language::Ur => DemoPair {
            typed: "vgja",
            fixed: "سلام",
            badge: ["EN", "UR"],
        },
        // Wave 1 — KBDTH0.DLL (Kedmanee) and KBDLAO.DLL against KBDUS.DLL, with the way back
        // (`scratchpad-E108\pairs-th-lo.out.txt`); the words of the translators.
        Language::Th => DemoPair {
            typed: "l;ylfu",
            fixed: "สวัสดี",
            badge: ["EN", "TH"],
        },
        Language::Lo => DemoPair {
            typed: "lt[kpfu",
            fixed: "ສະບາຍດີ",
            badge: ["EN", "LO"],
        },
        // KBDINBEN, KBDINBE1 and KBDINBE2 (the same keys), KBDINMAR, KBDINASA, KBDYCC and KBDYCL
        // against KBDUS (`scratchpad-E108\pairs-wave1a.out.txt`).
        Language::Bn => DemoPair {
            typed: "vcmdkej",
            fixed: "নমস্কার",
            badge: ["EN", "BN"],
        },
        Language::Mr => DemoPair {
            typed: "vcmdkej",
            fixed: "नमस्कार",
            badge: ["EN", "MR"],
        },
        // ⚠ The layout Windows gives Nepali is «Nepali» (KBDNEPR.DLL), not the INSCRIPT of Hindi:
        // the virama is `Shift`+`\`. The translator wrote the INSCRIPT keys «vcmdls» from memory.
        Language::Ne => DemoPair {
            typed: "gd;|t]",
            fixed: "नमस्ते",
            badge: ["EN", "NE"],
        },
        Language::As => DemoPair {
            typed: "vcmdkej",
            fixed: "নমস্কাৰ",
            badge: ["EN", "AS"],
        },
        // Serbian is QWERTZ in both scripts: «з»/«z» stands on the key of the US «y».
        Language::Sr => DemoPair {
            typed: "ydravo",
            fixed: "здраво",
            badge: ["EN", "SR"],
        },
        Language::SrLatn => DemoPair {
            typed: "ydravo",
            fixed: "zdravo",
            badge: ["EN", "SR"],
        },
        // KBDFA and KBDFAR, KBDUGHR and KBDUGHR1 — the same keys in both layouts of each
        // (`scratchpad-E108\pairs-fa-ug.out.txt`).
        Language::Fa => DemoPair {
            typed: "sghl",
            fixed: "سلام",
            badge: ["EN", "FA"],
        },
        Language::Ug => DemoPair {
            typed: "sflfm",
            fixed: "سالام",
            badge: ["EN", "UG"],
        },
        // KBDINTAM (Tamil, INSCRIPT — as the other Indic languages of the program; Tamil 99 would
        // be «vphfhkf»: the stand looks), KBDINMAL, KBDUZB (`scratchpad-E108\pairs-uz-ta-ml.out.txt`).
        // The translators' keys were from memory and two were wrong (ta «bCkdkcd», uz-Cyrl «Cj]…»).
        Language::Ta => DemoPair {
            typed: "bCidicd",
            fixed: "வணக்கம்",
            badge: ["EN", "TA"],
        },
        Language::Ml => DemoPair {
            typed: "vcmdkejx",
            fixed: "നമസ്കാരം",
            badge: ["EN", "ML"],
        },
        Language::UzCyrl => DemoPair {
            typed: "Cj-vbcbp",
            fixed: "Соғмисиз",
            badge: ["EN", "UZ"],
        },
        // Uzbek in Latin has no layout of its own in Windows — its readers type with the English
        // one, as the Polish programmers do (решение 178.5) — so its pair is the other way round:
        // «Salom» typed while the Russian layout was on (KBDRU, `pairs-uz-reverse.out.txt`).
        Language::Uz => DemoPair {
            typed: "Ыфдщь",
            fixed: "Salom",
            badge: ["RU", "UZ"],
        },
        // Wave 2 — KBDINTEL and KBDINKAN against KBDUS (`scratchpad-E108\pairs-te-kn.out.txt`); the
        // translators' captions held a stub «ghbdtn», and KBDKAN.DLL, which the report named, is
        // not a file of Windows.
        Language::Te => DemoPair {
            typed: "vcmdkejx",
            fixed: "నమస్కారం",
            badge: ["EN", "TE"],
        },
        Language::Kn => DemoPair {
            typed: "vcmdkej",
            fixed: "ನಮಸ್ಕಾರ",
            badge: ["EN", "KN"],
        },
        // Wave 2 — KBDINPUN, KBDINGUJ, KBDAZEL, KBDBLR, KBDKAZ, KBDTAT, KBDBULG (and KBDBU — the same
        // keys), KBDMAC (and KBDMACST) against KBDUS (`scratchpad-E108\pairs-pa-gu.out.txt`,
        // `pairs-wave2a.out.txt`); the captions of the translators held stubs.
        Language::Pa => DemoPair {
            typed: "vcmls",
            fixed: "ਨਮਸਤੇ",
            badge: ["EN", "PA"],
        },
        Language::Gu => DemoPair {
            typed: "vcmdls",
            fixed: "નમસ્તે",
            badge: ["EN", "GU"],
        },
        Language::Az => DemoPair {
            typed: "Nec's'n",
            fixed: "Necəsən",
            badge: ["EN", "AZ"],
        },
        Language::Be => DemoPair {
            typed: "ghsdbnfyyt",
            fixed: "прывітанне",
            badge: ["EN", "BE"],
        },
        Language::Kk => DemoPair {
            typed: "c2ktv",
            fixed: "сәлем",
            badge: ["EN", "KK"],
        },
        Language::Tt => DemoPair {
            typed: "cokfv",
            fixed: "сәлам",
            badge: ["EN", "TT"],
        },
        // Bulgarian: Phonetic Traditional (`00040402`, 208.6) — «здравей» in the keys of KBDUS is
        // «zdrawej» (stage E114, the tables of the guest by ToUnicodeEx, `out\q114\c-probe.txt`).
        Language::Bg => DemoPair {
            typed: "zdrawej",
            fixed: "здравей",
            badge: ["EN", "BG"],
        },
        Language::Mk => DemoPair {
            typed: "zdravo",
            fixed: "здраво",
            badge: ["EN", "MK"],
        },
        // KBDINORI, KBDKHMR (Khmer: «សួស្តី» takes a space for its coeng on this layout, and the
        // program converts the last word — so «ជំរាបសួរ», no coeng), KBDGEOQW (Georgian QWERTY;
        // the traditional KBDGEO gives «ugbgjwk.g»), KBDARMPH (Armenian Phonetic, the layout of
        // hy-AM; the Eastern one gives «bar&») against KBDUS (`pairs-wave2b.out.txt`).
        Language::Or => DemoPair {
            typed: "vcmdkej",
            fixed: "ନମସ୍କାର",
            badge: ["EN", "OR"],
        },
        Language::Km => DemoPair {
            typed: "CMrabsYr",
            fixed: "ជំរាបសួរ",
            badge: ["EN", "KM"],
        },
        Language::Ka => DemoPair {
            typed: "gamarjoba",
            fixed: "გამარჯობა",
            badge: ["EN", "KA"],
        },
        Language::Hy => DemoPair {
            typed: "ba87",
            fixed: "բարև",
            badge: ["EN", "HY"],
        },
        // Stage E108: every value of the closed set has its pair since wave 2. A language of a later
        // group joins with its own arm — the word of its translator, the keys off its layout by the
        // instrument of 178.4; until then it may stand in an arm of `PAIR_TO_COME`, which the build
        // never draws (`Language::is_shipped`), and `tests\letters.rs` refuses a shipped language
        // that is still there.
        //
        // Stage E109, group B: the word of the translator, the keys off KBDCR (hr, bs, sl) and KBDSL
        // (sk) against KBDUS, with the way back (`scratchpad-E109\pairs-*.out.txt`).
        Language::Hr => DemoPair {
            typed: "ydravo",
            fixed: "zdravo",
            badge: ["EN", "HR"],
        },
        Language::Sk => DemoPair {
            typed: "4au",
            fixed: "čau",
            badge: ["EN", "SK"],
        },
        Language::Sl => DemoPair {
            typed: "\\ivjo",
            fixed: "živjo",
            badge: ["EN", "SL"],
        },
        Language::Bs => DemoPair {
            typed: "'ao",
            fixed: "ćao",
            badge: ["EN", "BS"],
        },
        // KBDLT1 (Lithuanian, the layout of lt-LT: the letters on the row of digits) against KBDUS.
        // Latvian: on the layout of lv-LV, KBDLV1 (QWERTY), the `č` of «čau» and the `ī` of «labrīt»
        // stand on no key and on no modifier — only behind a dead key (`pairs-w2.out.txt`); so the pair
        // is the other way of 178.5, as Uzbek's — «Sveiki» typed while the Russian layout was on
        // (KBDRU, `pairs-lv-reverse.out.txt`).
        Language::Lt => DemoPair {
            typed: "a2i8",
            fixed: "ačiū",
            badge: ["EN", "LT"],
        },
        Language::Lv => DemoPair {
            typed: "Ымушлш",
            fixed: "Sveiki",
            badge: ["RU", "LV"],
        },
        // KBDSW (Swedish: `å` on the key of `[`) and KBDDA (Danish: `ø` on the key of `'`) against
        // KBDUS.
        Language::Sv => DemoPair {
            typed: "hall[",
            fixed: "hallå",
            badge: ["EN", "SV"],
        },
        Language::Da => DemoPair {
            typed: "hall'",
            fixed: "hallø",
            badge: ["EN", "DA"],
        },
        // KBDFI and KBDEST (`ä` on the key of `'`), KBDNO for both forms of Norwegian (`å` on the key
        // of `[`) against KBDUS. Nynorsk takes «hallå» of the reviewer, not «skål» — a toast (C7-5).
        Language::Fi => DemoPair {
            typed: "p'iv''",
            fixed: "päivää",
            badge: ["EN", "FI"],
        },
        Language::Nb => DemoPair {
            typed: "hall[",
            fixed: "hallå",
            badge: ["EN", "NB"],
        },
        Language::Et => DemoPair {
            typed: "t'nan",
            fixed: "tänan",
            badge: ["EN", "ET"],
        },
        Language::Nn => DemoPair {
            typed: "hall[",
            fixed: "hallå",
            badge: ["EN", "NN"],
        },
        // Dutch: since stage E114 (208.8) the Belgian AZERTY of nl-BE, «Belgian (Period)» `00000813` —
        // «zaterdag» in the keys of KBDUS is «wqterdqg» (the tables of the guest by ToUnicodeEx,
        // `out\q114\c-probe.txt`); the US-International of nl-NL types «hallo» with the keys of
        // KBDUS, which is why 178.5 had taken the other way («рфддщ» → «hallo») until then.
        // Luxembourgish: KBDSF, the layout of lb-LU (`0000046E`), `ä` with `Shift` on the key of `'`.
        Language::Nl => DemoPair {
            typed: "wqterdqg",
            fixed: "zaterdag",
            badge: ["EN", "NL"],
        },
        Language::Lb => DemoPair {
            typed: "\"ddi",
            fixed: "äddi",
            badge: ["EN", "LB"],
        },
        // Welsh: the layout of cy-GB is United Kingdom Extended (KBDUKX), «helo» in the keys of KBDUS —
        // the other way of 178.5, as Dutch (`pairs-cy-mi-reverse.out.txt`). Icelandic: KBDIC, `æ` on
        // the key of `;` («halló» of the translator: `ó` is behind a dead key).
        Language::Cy => DemoPair {
            typed: "рудщ",
            fixed: "helo",
            badge: ["RU", "CY"],
        },
        Language::Is => DemoPair {
            typed: "h;",
            fixed: "hæ",
            badge: ["EN", "IS"],
        },
        // Vietnamese: KBDVNTC, the layout of vi-VN; its tones are combining marks, so «chào» of the
        // translator would not come out as one precomposed letter (NFC, TZ E109 §0 п. 5) — «alô», the
        // «hello» of the telephone, `ô` on the key of `4`. KBDROST and KBDRO (Romanian, `ă` on the
        // key of `[`), KBDHU (`ü` on the key of `-`), KBDAL (Albanian, `ë` on the key of `;`),
        // KBDMLT47 (Maltese, `ġ` on the key of `[`). Maori: KBDMAORI writes `ō` of «mōrena» only
        // behind a dead key — the other way of 178.5, «aroha» typed while the Russian layout was on.
        Language::Vi => DemoPair {
            typed: "al4",
            fixed: "alô",
            badge: ["EN", "VI"],
        },
        Language::Ro => DemoPair {
            typed: "bun[",
            fixed: "bună",
            badge: ["EN", "RO"],
        },
        Language::Hu => DemoPair {
            typed: "-dv",
            fixed: "üdv",
            badge: ["EN", "HU"],
        },
        Language::Sq => DemoPair {
            typed: "mir;dita",
            fixed: "mirëdita",
            badge: ["EN", "SQ"],
        },
        Language::Mt => DemoPair {
            typed: "bon[u",
            fixed: "bonġu",
            badge: ["EN", "MT"],
        },
        Language::Mi => DemoPair {
            typed: "фкщрф",
            fixed: "aroha",
            badge: ["RU", "MI"],
        },
    }
}

/// The pair of a language whose translation has not come yet — the English pair, so a
/// stray drawing shows a real example and not an empty field. Never the pair of a shipped language
/// but English and Polish (which show it by решение 178.5): `tests\letters.rs` holds that.
pub const PAIR_TO_COME: DemoPair = DemoPair {
    typed: "руддщ",
    fixed: "hello",
    badge: ["RU", "EN"],
};

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

impl DemoFrame {
    /// The word on the little screen as much of it as is typed at this frame — what the picture of
    /// «Привет» shows (T-120-25), and a name of the frame a description may use: the descriptions of
    /// the shell name no `text` (the guard of the window descriptions, rule 2).
    pub fn word_shown(&self) -> String {
        self.text.chars().take(self.shown).collect()
    }
}

/// What the demonstration of `pair` shows at frame `tick` — **pure**, so the whole animation is a
/// table a test can walk without a window, a timer or a clock.
///
/// One turn: the word is typed letter by letter, the key flashes, the word is fixed and the
/// indicator flips, the key flashes again, the word comes back, and it starts over. That is
/// FR-01 to FR-06 in eight seconds and no words at all.
///
/// ⭐ **Задача T-102-2, решение 178.6: the typing takes as many frames as the word has letters**,
/// one a frame — four to eight since every language has a pair of its own; until `e101` the one
/// word was «ghbdtn» and the typing was frames 0…5. The rest of the turn is what it was: the word
/// waits until frame 13, the key is down at 14–15, the fixed word stands 16–29, the key is down
/// again at 30–31, and the typed word is back for 32–47.
pub fn demo_frame(tick: u32, pair: DemoPair) -> DemoFrame {
    let typed = pair.typed.chars().count();

    match tick % DEMO_FRAMES {
        // Typing, one character a frame.
        frame if (frame as usize) < typed => DemoFrame {
            text: pair.typed,
            shown: frame as usize + 1,
            pressed: false,
            converted: false,
        },
        // The typed word, waiting.
        0..=13 => DemoFrame {
            text: pair.typed,
            shown: typed,
            pressed: false,
            converted: false,
        },
        // The key goes down.
        14..=15 => DemoFrame {
            text: pair.typed,
            shown: typed,
            pressed: true,
            converted: false,
        },
        // The word is fixed and the layout has switched.
        16..=29 => DemoFrame {
            text: pair.fixed,
            shown: pair.fixed.chars().count(),
            pressed: false,
            converted: true,
        },
        // The key goes down again.
        30..=31 => DemoFrame {
            text: pair.fixed,
            shown: pair.fixed.chars().count(),
            pressed: true,
            converted: true,
        },
        // And the word is back, letter for letter — FR-05.
        _ => DemoFrame {
            text: pair.typed,
            shown: typed,
            pressed: false,
            converted: false,
        },
    }
}

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
use windows::Win32::UI::WindowsAndMessaging::{
    DM_SETDEFID, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDCANCEL,
    KillTimer, MSG, MapDialogRect, PostMessageW, SET_WINDOW_POS_FLAGS, SW_HIDE, SW_SHOWNOACTIVATE,
    SW_SHOWNORMAL, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOREDRAW, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG,
    WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND,
    WM_INITDIALOG, WM_MEASUREITEM, WM_NCDESTROY, WM_TIMER,
};
use windows::core::{Error as WinError, PCWSTR};

use crate::settings::{self, Language};
use crate::theme::{self, StaticColorRole, ThemeSetting};
// Общий слой элементов окон — задача Т-45-2, решение 107.1. Мастер и окна писем берут элементы
// здесь: в этом и состоит принцип пользователя «один механизм на все окна».
use crate::widgets;

/// The identifier of the timer that counts the five seconds of «Взять из активного окна».
///
/// NFR-10 again: it exists only between the press of that button and the capture, and is
/// killed on the way out of either. A second timer of this program, and the last.
const CAPTURE_TIMER: usize = 2;

// ⚠ `CAPTURE_SECONDS` stood here until task T-100-2. The five seconds are one number for the wizard
// and for «Выбрать окно программы» of the settings window, and that number is
// `guard::CAPTURE_SECONDS`, beside the one body of the capture.

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
    /// What the field «Как с вами связаться» holds, as typed — задача T-107-3. Kept across the
    /// steps, so a person who goes back and forward finds it where they left it, and the line
    /// [`report::with_contact`] wrote into [`Wizard::text`] is found by it to be taken out again.
    contact: String,
    /// Where the appeal is on its way to the site — задача T-107-3.
    post: Post,
    /// The nonce of this appeal, drawn at the first «Отправить» and sent again by every
    /// «Повторить» (199.7 (б)): the site answers a repeat with the same 201 and no second record.
    nonce: Option<String>,
    /// Whether the site took the appeal and the window is showing the thanks — задача Т-33-3, and
    /// since T-107-3 set by the answer of the site and not by a press.
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

/// **Where the appeal is on its way to the site** — задача T-107-3, решения 199.3 и 199.11 (макет
/// «Шаг «Отправить» мастера», версия 2, вариант Б).
///
/// ⛔ **`Destination` stood here until 1.2.0** — «a trouble by letter, an idea to the group or to
/// GitHub Issues» (решения 181.1 и 181.6). Решение 199.3 took both ways out of the wizard: every
/// appeal, in every language, goes to the site by «Отправить», and the letter to
/// [`links::MAIL_ADDRESS`] is the way back when the site did not answer. The group and the issues
/// stay where they were outside the wizard — «От автора», the letters, README and the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Post {
    /// Not sent yet — the main button says «Отправить».
    Ready,
    /// On its way, on the thread with this ticket — «Отправляется…», and every button and field of
    /// the window is grey, so the appeal cannot be sent twice or changed under the request.
    Sending(u32),
    /// The site did not take it — «Не удалось отправить»: the text stays, the main button says
    /// «Повторить», and the letter is offered — the text, the address, the file.
    Failed,
}

/// The words of one step — its heading, its explanation and its main button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepWords {
    pub title: u16,
    pub note: Option<u16>,
    pub next: u16,
}

/// **The words of a step, by the step and by where the appeal is** — one pure place, asked by the
/// fill of the window and by the repaint after a choice, so the two cannot say different things
/// (until 1.2.0 each of them had its own copy of this table).
///
/// The last step of either road is «Проверьте и отправьте» while the appeal is ready or on its
/// way, and «Не удалось отправить» once the site did not take it; the thanks is «Обращение
/// отправлено» for a trouble and for an idea alike (решение 199.10).
pub fn step_words(step: Step, post: Post) -> StepWords {
    use crate::settings::{
        IDS_CLOSE, IDS_WIZARD_ATTACH_NOTE, IDS_WIZARD_ATTACH_TITLE, IDS_WIZARD_DID_NOTE,
        IDS_WIZARD_DID_TITLE, IDS_WIZARD_FAILED_NOTE, IDS_WIZARD_FAILED_TITLE,
        IDS_WIZARD_IDEA_NOTE, IDS_WIZARD_IDEA_TITLE, IDS_WIZARD_NEXT, IDS_WIZARD_PREVIEW_NOTE,
        IDS_WIZARD_PREVIEW_TITLE, IDS_WIZARD_RETRY, IDS_WIZARD_SEND, IDS_WIZARD_SENDING,
        IDS_WIZARD_SENT_TITLE, IDS_WIZARD_WHAT_NOTE, IDS_WIZARD_WHAT_TITLE, IDS_WIZARD_WHERE_NOTE,
        IDS_WIZARD_WHERE_TITLE,
    };

    let middle = |title, note| StepWords {
        title,
        note: Some(note),
        next: IDS_WIZARD_NEXT,
    };

    match step {
        Step::What => middle(IDS_WIZARD_WHAT_TITLE, IDS_WIZARD_WHAT_NOTE),
        Step::Where => middle(IDS_WIZARD_WHERE_TITLE, IDS_WIZARD_WHERE_NOTE),
        Step::Did => middle(IDS_WIZARD_DID_TITLE, IDS_WIZARD_DID_NOTE),
        Step::Idea => middle(IDS_WIZARD_IDEA_TITLE, IDS_WIZARD_IDEA_NOTE),
        Step::Attach => middle(IDS_WIZARD_ATTACH_TITLE, IDS_WIZARD_ATTACH_NOTE),
        Step::Preview => match post {
            Post::Ready => StepWords {
                title: IDS_WIZARD_PREVIEW_TITLE,
                note: Some(IDS_WIZARD_PREVIEW_NOTE),
                next: IDS_WIZARD_SEND,
            },
            Post::Sending(_) => StepWords {
                title: IDS_WIZARD_PREVIEW_TITLE,
                note: Some(IDS_WIZARD_PREVIEW_NOTE),
                next: IDS_WIZARD_SENDING,
            },
            Post::Failed => StepWords {
                title: IDS_WIZARD_FAILED_TITLE,
                note: Some(IDS_WIZARD_FAILED_NOTE),
                next: IDS_WIZARD_RETRY,
            },
        },
        // «Готово» — задача Т-33-3: there is no explanation line, because the thanks is a
        // paragraph of its own and a muted sentence above it would be a second voice saying the
        // same thing.
        Step::Done => StepWords {
            title: IDS_WIZARD_SENT_TITLE,
            note: None,
            next: IDS_CLOSE,
        },
    }
}

/// **Which slots of the last step stand on the screen, by where the appeal is** — задача T-107-3.
///
/// Ready and on its way: the text, the contact under it (вариант Б, 199.11) and «Сохранить в папку
/// журнала». Not taken: the text, the address of the letter, «Скопировать текст», «Скопировать
/// адрес» and «Сохранить в папку журнала» — the contact is in the text by then («Для связи: …»),
/// and the request will carry it again. The status line under the buttons is in both.
pub fn preview_controls(post: Post) -> &'static [i32] {
    match post {
        Post::Ready | Post::Sending(_) => &[
            IDC_WZ_PREVIEW,
            IDC_WZ_CONTACT_LABEL,
            IDC_WZ_CONTACT,
            IDC_WZ_SAVE,
            IDC_WZ_STATUS,
        ],
        Post::Failed => &[
            IDC_WZ_PREVIEW,
            IDC_WZ_ADDRESS,
            IDC_WZ_COPY,
            IDC_WZ_COPY_ADDRESS,
            IDC_WZ_SAVE,
            IDC_WZ_STATUS,
        ],
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
    /// [`Kind::Wizard`]: the letters, «Последние письма» and «От автора» are windows of
    /// the shell (`ui::windows::letter`, `ui::windows::letters_list`) and have no state here.
    kind: Kind,
    setting: ThemeSetting,
    palette: &'static theme::Palette,
    brushes: Option<theme::Brushes>,
    fonts: Option<Faces>,
    /// What the wizard is holding — `None` for «От автора». Boxed because it is the one large
    /// member.
    wizard: Option<Box<Wizard>>,
    /// The hotkey as it acts, for the chip of the wizard.
    hotkey: String,
    icons: Option<settings::CaptionIcons>,
    /// The locale the window was built in — what `language_switch` is asked about.
    language: Language,
}

impl WindowState {
    /// Which control of this window is the accented one — the button Enter presses and the one
    /// drawn in the accent colour.
    ///
    /// One place, because two ask: the drawing, which colours it, and `WM_INITDIALOG`, which
    /// tells the dialog manager about it with `DM_SETDEFID`.
    fn accent_control(&self) -> i32 {
        // «Далее» — and on the last step it says «Готово». Enter presses it on every step,
        // which is what a wizard is for. The wizard is the one window of this state left.
        IDC_WZ_NEXT
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
    window_of(kind).is_some()
}

/// The handle of the open window of this kind, if there is one.
fn window_of(kind: Kind) -> Option<HWND> {
    OPEN.with_borrow(|open| {
        open.iter()
            .find(|(other, _)| *other == kind)
            .map(|(_, hwnd)| *hwnd)
    })
    .or_else(|| {
        SHELL.with_borrow(|open| {
            open.iter()
                .find(|window| window.kind == kind)
                .map(|window| window.hwnd)
        })
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

    // Задача T-118-2: Ctrl+A — до менеджера диалога, который этой клавиши не знает. С задачи T-118-6
    // тело общее — `widgets::keys`: его же зовёт перехватчик модального цикла для окна настроек.
    if widgets::keys::select_all_on_ctrl_a(message) {
        return true;
    }

    open.into_iter().any(|hwnd| {
        // SAFETY: `hwnd` is a window this module made and has not yet seen destroyed — the
        // record is removed on `WM_NCDESTROY` — and `message` is the caller's live message.
        unsafe { windows::Win32::UI::WindowsAndMessaging::IsDialogMessageW(hwnd, message) }
            .as_bool()
    })
}

thread_local! {
    /// Whether [`modal_loop_filter_proc`] is already running on this thread — задача T-118-2.
    ///
    /// `IsDialogMessageW` begins by calling the message filter itself (`CallMsgFilter`,
    /// `MSGF_DIALOGBOX`), so the filter's own call of [`filter_message`] comes straight back into
    /// the filter; the flag answers that second arrival with the next hook and nothing else.
    static IN_MODAL_LOOP_FILTER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// **Окна писем внутри модального цикла этого потока** — задача T-118-2, Q-208-23 п. 3: «не
/// везде возможен переход по текстовым полям нажатием Tab».
///
/// # Почему не работал Tab
///
/// Окна писем немодальные, и Tab, Enter и Esc им даёт [`filter_message`] из цикла сообщений
/// потока (`app::pump`). Но окно настроек модальное: пока оно открыто, сообщения потока выбирает
/// **его** цикл (`DialogBoxIndirectParamW`), и `IsDialogMessageW` там зовётся только для окна
/// настроек. Мастер «Написать автору…», открытый кнопкой окна настроек, этого цикла не видел:
/// замер на стенде с 1.6.0 — из трея Tab и Shift+Tab ходят по всем семи полям мастера, из окна
/// настроек — ни в одном, а в трёх многострочных полях Tab ещё и вписывает символ табуляции.
///
/// # Что теперь
///
/// На время модального цикла — поточный перехватчик `WH_MSGFILTER`. Менеджер диалога отдаёт ему
/// каждое сообщение цикла (`CallMsgFilter` с `MSGF_DIALOGBOX` в начале `IsDialogMessageW`), и
/// перехватчик отдаёт его общему телу [`filter_message`]: сообщение окна писем там и
/// обрабатывается, а цикл окна настроек его пропускает. Сообщения самого окна настроек
/// `filter_message` не берёт — `IsDialogMessageW` окна писем отвечает «не моё» на всё, что не
/// его, — и они идут дальше, как шли. **Кроме Ctrl+A в поле ввода — задача T-118-6** (слово
/// владельца 208.80): первым делом `filter_message` зовёт общее тело `widgets::keys`, и оно берёт
/// Ctrl+A в поле любого окна этого потока — так поле «Вручную» окна настроек выделяет весь текст
/// тем же телом, что поля мастера. Механизм тот же, что у меню трея (`WH_CALLWNDPROC` на
/// время меню, задача T-12-9): поточный, с `Drop`, который снимает его на любом пути.
///
/// `None` внутри — система перехватчик не дала (NFR-13); тогда мастер, открытый из окна настроек,
/// и поле «Вручную» ведут себя как в 1.6.0. Журналу нечего записать: в закрытом словаре `diag`
/// строки для этого вызова нет.
pub struct ModalLoopFilter(Option<windows::Win32::UI::WindowsAndMessaging::HHOOK>);

impl ModalLoopFilter {
    /// Puts the filter on this thread until the value is dropped. Called by
    /// `settings::show_modal_dialog` around the modal loop of a dialog of this program.
    pub fn install() -> Self {
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_MSGFILTER};

        // SAFETY: the procedure is a `'static` function of this module and outlives the hook,
        // which `Drop` removes. `None` for the module handle is what the documentation requires
        // when the thread named is a thread of the current process, and it is this one.
        let hook = unsafe {
            SetWindowsHookExW(
                WH_MSGFILTER,
                Some(modal_loop_filter_proc),
                None,
                GetCurrentThreadId(),
            )
        };

        Self(hook.ok())
    }
}

impl Drop for ModalLoopFilter {
    fn drop(&mut self) {
        let Some(hook) = self.0.take() else {
            return;
        };

        // NFR-13: examined here and deliberately dropped, as for the hook of the tray menu — a
        // refused removal cannot be repaired by a second attempt with the same handle.
        //
        // SAFETY: `hook` came from the successful `SetWindowsHookExW` of `install`, is taken out
        // of the field so it cannot be removed twice, and its procedure is in this image.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook) };
    }
}

/// The `WH_MSGFILTER` callback of [`ModalLoopFilter`] — задача T-118-2.
///
/// The whole of its work when no window of this module is open is one list read by
/// [`filter_message`]: the ordinary state of a settings window.
unsafe extern "system" fn modal_loop_filter_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, MSGF_DIALOGBOX};

    if code == MSGF_DIALOGBOX as i32 && lparam.0 != 0 && !IN_MODAL_LOOP_FILTER.with(|f| f.get()) {
        // SAFETY: for `WH_MSGFILTER` at a non-negative code `lParam` is the `MSG` being filtered,
        // owned by the caller of `CallMsgFilter` for the length of this call; read only.
        let message = unsafe { &*(lparam.0 as *const MSG) };

        IN_MODAL_LOOP_FILTER.with(|f| f.set(true));
        // SAFETY: `message` is the live message of the loop.
        let handled = unsafe { filter_message(message) };
        IN_MODAL_LOOP_FILTER.with(|f| f.set(false));

        if handled {
            // Non-zero: the message is dealt with, and the loop that asked must not dispatch it.
            return windows::Win32::Foundation::LRESULT(1);
        }
    }

    // SAFETY: forwards the call unchanged.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
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
    ///
    /// ⚠ **6 → 4 задачей T-81-2, решение 142.2 п. 7.** Четыре — это число окна настроек, у
    /// которого ряд стоял по правилу и до этого этапа; на приёмке Э80 владелец смотрел на то
    /// окно как на образец (решение 141а), и «единая нижняя строка во всех окнах» означает
    /// прийти к его зазору, а не завести седьмой.
    pub(super) const BUTTON_GAP: i32 = 4;
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

/// The panels of the templates of this module — the controls that carry a caption and a
/// rectangle and are never shown. None since «От автора» became a window of the shell (T-120-17);
/// the code that keeps them hidden goes with the wizard's template (T-120-19).
const PANELS: [i32; 0] = [];

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
        // The wizard's quiet lines: the step counter, every explanation and the status line.
        // Everything that asks a question or names a thing stays a plain label.
        IDC_WZ_STEP | IDC_WZ_NOTE | IDC_WZ_CAPTURE_NOTE | IDC_WZ_FIELD_2_SUB
        | IDC_WZ_ATTACH_FOOT | IDC_WZ_STATUS => StaticColorRole::Muted,
        // Задача T-107-3: the two fields of the last step, grey while the appeal is on its way. A
        // disabled `EDIT` asks `WM_CTLCOLORSTATIC` and not `WM_CTLCOLOREDIT`, and without this row
        // its ground would turn the window's colour inside the field's own frame.
        IDC_WZ_PREVIEW | IDC_WZ_CONTACT => StaticColorRole::Field,
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
        // The wizard: the heading of a step is set in the face of a name, and every
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

    let dpi = theme::drawing_dpi(dc);
    let panels: &[i32] = match kind {
        // ⚠ The wizard has **no panel**: its steps stand on the window's own ground, and the
        // three cards of the first step are drawn as blocks by the button branch rather than
        // by the background. A panel here would be a block behind the fields.
        Kind::Wizard | Kind::Letter | Kind::List | Kind::Author => &[],
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

/// The four **single-line** fields of the wizard — the ones whose box the background draws
/// around a control one em box tall (задача Т-33-7). The contact since задача T-107-3.
const WIZARD_FIELDS: [i32; 4] = [IDC_WZ_PROGRAM, IDC_WZ_EXPECTED, IDC_WZ_GOT, IDC_WZ_CONTACT];

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

        // The progress bar of the wizard: a run of steps behind and a run ahead.
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

    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            let brushes = state.brushes.as_ref()?;
            let accent = state.accent_control();

            let (colors, hot_brush) = theme::resolve_button_colors(
                // The accented button of a letter is the accent of this window, and every
                // other button is a plain one — the same table the about window's «ОК» is
                // drawn by, asked about the identifier that is accented **here**.
                settings::button_color_roles(
                    if control == accent {
                        settings::OK_COMMAND
                    } else {
                        control
                    },
                    hot,
                    pressed,
                    disabled,
                ),
                // ⭐⭐ **The ground the button actually stands on** — task T-80-1. This was
                // `window_bg` for every button of these three windows, and five of the seven
                // buttons of «От автора» stand on a block. The drawing cuts a rounded figure out
                // of the rectangle and fills what it cuts away with this brush, so the wrong one
                // does not tint the button — it paints four square corners of another colour
                // around it, which is precisely what the user saw on `e79` in both palettes.
                //
                // The question is [`stands_on_a_panel`], the one the labels of this file have
                // always asked. Not a second answer beside theirs — the same one.
                if stands_on_a_panel(control) {
                    brushes.panel_bg()
                } else {
                    brushes.window_bg()
                },
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
                if stands_on_a_panel(control) {
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
                    dpi: theme::drawing_dpi(dc),
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

                    OPEN.with_borrow_mut(|open| open.push((state.kind, hwnd)));

                    settings::apply_title_bar_theme(hwnd, state.palette);

                    (
                        state.kind,
                        state.accent_control(),
                        state.icons.as_ref().map(settings::CaptionIcons::frames),
                    )
                })
            };

            let Some((kind, accent, frames)) = prepared else {
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

                // Задача T-107-3: the field does not take a contact longer than the site does
                // (199.7: 200 characters). The answer of `EM_SETLIMITTEXT` is documented as none.
                let _ = settings::send_to(
                    hwnd,
                    IDC_WZ_CONTACT,
                    windows::Win32::UI::Controls::EM_SETLIMITTEXT,
                    appeal::CONTACT_MAX,
                    0,
                );
            }

            if let Some(frames) = frames {
                // SAFETY: `hwnd` is the live window and the handles belong to the value the
                // state keeps, which is dropped only when the window is destroyed.
                unsafe { settings::CaptionIcons::show_on(hwnd, frames) };
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

        // One second of the capture count-down of FR-104 — the wizard's own timer, and the
        // only other one this program sets.
        WM_TIMER if wparam.0 == CAPTURE_TIMER => {
            wizard_tick(hwnd);
            0
        }

        // The site answered an appeal — задача T-107-3. The message is a nudge and nothing more;
        // the answer is in the box, under the ticket of the press (SEC-05).
        WM_APP_APPEAL => {
            wizard_answer(hwnd);
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

            // The wizard answers its own commands (`wizard_command`, above); «От автора», whose
            // buttons were answered here, is a window of the shell (T-120-17).

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

            // SAFETY: `hwnd` is the live window; the capture count-down may or may not be running,
            // and killing a timer that was never set answers false and is harmless (NFR-13).
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
        Kind::Wizard => &WIZARD_LABELS,
        Kind::Letter | Kind::List | Kind::Author => &[],
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

/// The owner-drawn buttons of one kind of window — the list `subclass_buttons` walks.
fn buttons_of(kind: Kind) -> &'static [i32] {
    match kind {
        Kind::Wizard => &WIZARD_BUTTONS,
        Kind::Letter | Kind::List | Kind::Author => &[],
    }
}

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
    /// The line under the program's name: the version.
    ///
    /// ⚠ **«and the author's alias» no more — задача T-81-3, решение 142.2 п. 5.** The line is
    /// `IDS_ABOUT_VERSION` now, the very one «О программе» writes, and the signature the owner
    /// asked to remove («не надо вставлять панда, он же панда») went with the string that
    /// carried it.
    pub version_line: String,
    /// The first line of the head's description — `IDS_ABOUT_LINE_1`, задача T-81-3.
    pub line_1: String,
    /// The second — `IDS_ABOUT_LINE_2`.
    pub line_2: String,
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
    /// The switch and its state, or `None` while FR-102 keeps it away. The state is the setting's
    /// when the window was built, and after a click the setting's as written —
    /// [`AuthorView::follow_feed`].
    pub switch: Option<bool>,
    /// The sentence under the switch.
    pub switch_note: String,
    /// Whether «Последние письма» has anything to show.
    pub letters: bool,
    /// The paragraph of the «Обратная связь» panel.
    pub feedback_text: String,
}

impl AuthorView {
    /// **The switch follows the setting a click has written — FR-102, task T-70-1.**
    ///
    /// `draw_switch` paints the tick out of [`AuthorView::switch`], and [`author_view`] fills
    /// that in twice in a window's life: when «От автора» opens and when the interface language
    /// changes. A click writes `Letters::feed` through the tray and repaints the switch — and
    /// before this method the repaint drew the copy of the opening: the file said the feed was
    /// off while the tick said it was on, and a second click turned the feed back on under a
    /// tick that had never moved (finding Э69-Б-1, read out of the code in stage Э69).
    ///
    /// Only the state follows. Whether there is a switch at all stays the verdict of
    /// [`switch_is_shown`]: a hidden switch is not there to be clicked, and a setting that has
    /// changed must not make one appear.
    pub fn follow_feed(&mut self, state: &Letters) {
        if let Some(on) = self.switch.as_mut() {
            *on = state.release_window;
        }
    }
}

/// The whole of what «От автора» says — FR-103, in the language of the program.
pub fn author_view(state: &Letters, today: Date, version: &str, feed: FeedView<'_>) -> AuthorView {
    author_view_in(settings::ui_language(), state, today, version, feed)
}

/// [`author_view`] in `language` — for the instrument of fitting, which goes over all of them.
pub fn author_view_in(
    language: Language,
    state: &Letters,
    today: Date,
    version: &str,
    feed: FeedView<'_>,
) -> AuthorView {
    use crate::settings::{
        IDS_ABOUT_LINE_1, IDS_ABOUT_LINE_2, IDS_ABOUT_VERSION, IDS_AUTHOR_TEXT, IDS_FEEDBACK_TEXT,
        IDS_NEWS_ABOUT_FEED, IDS_NEWS_AVAILABLE, IDS_NEWS_FILE_ONLY, IDS_NEWS_LATEST,
        IDS_NEWS_NEVER_READ, IDS_NEWS_OFF_IN_FILE, IDS_NEWS_READ_ON,
    };

    let text = |id| settings::text_in(language, id);
    let format_text = |id, arguments: &[&str]| {
        let mut filled = text(id);
        for (index, argument) in arguments.iter().enumerate() {
            filled = filled.replace(&format!("{{{index}}}"), argument);
        }
        filled
    };

    // ⭐ Task T-119-10 (решение 208.119): the switch of the panel is «Сообщать об обновлениях
    // программы» — `[letters] release_window`, the window «Обновлено» — and it is always there; the
    // switch of the feed and its rule of ninety days (T-69-5) are gone. `[letters] feed` stays in the
    // file, and a feed turned off there is said under the switch.
    let _ = today;
    let switch = Some(state.release_window);
    let update = update_is_pending(version, feed);

    AuthorView {
        // ⭐⭐ **`IDS_ABOUT_VERSION` и не `IDS_AUTHOR_VERSION` — задача T-81-3, решение 142.2
        // п. 5.** Вторая строка несла «версия {0} · Панда, он же Panda_Pishet_Kod» и снята со
        // всеми четырнадцатью переводами; шапка у двух окон одна, и строка версии у них одна.
        version_line: format_text(IDS_ABOUT_VERSION, &[version]),
        // Две строки описания общей шапки — те же слова, что в «О программе», из тех же строк.
        line_1: text(IDS_ABOUT_LINE_1),
        line_2: text(IDS_ABOUT_LINE_2),
        author_text: text(IDS_AUTHOR_TEXT),
        // The sentence about the feed moves under the switch once there is one — it is the
        // same sentence, and two copies of it on one panel would be a stutter.
        feed_about: if switch.is_some() {
            String::new()
        } else {
            text(IDS_NEWS_ABOUT_FEED)
        },
        switch_note: if state.feed {
            String::new()
        } else {
            text(IDS_NEWS_OFF_IN_FILE)
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
        // ⛔ No `download` here any more — task T-78-2. «Открыть страницу загрузки» opens
        // [`links::DOWNLOAD_URL`], a constant of this build, so there is nothing about it for a
        // view of the feed to carry. Keeping the field would have left the entry's address one
        // assignment away from creeping back into that button.
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
        Kind::Wizard => fill_wizard(hwnd, state),
        Kind::Letter | Kind::List | Kind::Author => {}
    }
}

/// Lays out one of the three windows.
///
/// # Safety
///
/// `hwnd` is the live window and `state` its own state.
unsafe fn layout_window(hwnd: HWND, state: &WindowState) {
    match state.kind {
        Kind::Wizard => unsafe { layout_wizard(hwnd, state) },
        Kind::Letter | Kind::List | Kind::Author => {}
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
///
/// ⭐ **The update only from the seventh day — task T-119-9** (решение 208.99 п. 2): its row is
/// the second door of the letter «Обновление», and it stands exactly when the first one, the menu
/// entry of [`pending`], does — [`update_fallback_is_open`] on `today`; no day (`None`), no row.
/// Its words are the letter's own: «Обновиться автоматически не получилось», «доступна X · у вас Y».
pub fn list_entries(
    state: &Letters,
    today: Option<Date>,
    version: &str,
    feed: FeedView<'_>,
) -> Vec<Entry> {
    use crate::settings::{
        IDS_ENTRY_FALLBACK_MARK, IDS_ENTRY_READ_MARK, IDS_ENTRY_UNREAD_MARK, IDS_FALLBACK_TITLE,
        format_text, text,
    };

    let mut entries = Vec::with_capacity(NEWS_KEPT + 1);
    let fallback = today.is_some_and(|today| update_fallback_is_open(state, today));

    if let Some(update) = update_is_pending(version, feed).filter(|_| fallback) {
        let announced = update.version.as_deref().unwrap_or_default();

        entries.push(Entry {
            title: text(IDS_FALLBACK_TITLE),
            meta: format_text(IDS_ENTRY_FALLBACK_MARK, &[announced, version]),
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
        Letter::Welcome | Letter::Thanks => None,
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
    // TZ E104 §0 п. 20: the entry «Доступна версия X…» is the fallback of the updater, as the
    // letter is — there only once the feed has named the newer version for three days.
    let fallback = today().is_some_and(|today| update_fallback_is_open(&stored, today));

    with_feed(|feed| {
        (
            has_unread_news(&stored, feed),
            update_is_pending(&version, feed)
                .filter(|_| fallback)
                .and_then(|item| item.version.clone()),
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

/// What a window hands [`act`]: whose window it is, which news entry it is about and the link
/// that entry carried, and the language of its words.
pub(crate) struct Deed<'a> {
    /// The program's window — the owner of every window an action opens.
    pub owner: HWND,
    /// The identifier of the feed entry a «Новость» is about — what «Прочитано» marks.
    pub news: Option<u64>,
    /// The link the entry carried, for «Читать на сайте».
    pub link: &'a str,
    /// The language of the window: the pages it opens are the pages in that language
    /// (решения 181.1, 181.33) — the language its words are in, and not the one asked of
    /// `settings` at the click.
    pub language: Language,
}

/// Does what a button of a letter, of «Последние письма» or of «От автора» says it does —
/// FR-101, FR-103 — and answers whether the window it stands in closes. One body for the windows
/// of the shell and the old ones alike.
pub(crate) fn act(deed: &Deed<'_>, action: Action) -> bool {
    match action {
        Action::Close => return true,

        Action::Snooze => {
            if let Some(today) = today() {
                update_state(|state| snooze_thanks(state, today));
            }

            return true;
        }

        Action::MarkRead => {
            if let Some(id) = deed.news {
                update_state(|state| mark_read(state, id));
                crate::tray::refresh_unread_mark();
            }

            return true;
        }

        // FR-92 through the one door the tray already opens it by — a second `show_dialog`
        // call site would be a second place that can forget the guard of `dialog_is_open`.
        Action::OpenSettings => {
            crate::tray::dispatch_command(deed.owner, crate::tray::CMD_SETTINGS);
        }

        Action::OpenChannel => open_link(links::CHANNEL_URL),
        // Решение 181.1: where this window's reader can write — the group or GitHub Issues.
        Action::OpenFeedback => open_link(links::feedback_url(deed.language)),
        Action::OpenSupport => open_link(links::SUPPORT_URL),

        // ⭐ Task T-78-2: the download page of **this build**, never the `link` of the feed
        // entry that announced the update. See [`links::DOWNLOAD_URL`] for why the constant
        // wins — in short, the entry is signed and dated and its address goes stale.
        // Решение 181.33: the page in the language of this window — Russian for the Russian side,
        // under `/en/` for the twelve of the English one.
        Action::OpenDownload => open_link(links::download_url(deed.language)),

        // Задача T-81-3: щелчок по имени программы в шапке «От автора». Адрес — тот же, что
        // открывает имя в «О программе», и той же дверью `open_link`, которая **и есть** ворота
        // С52: только https и только на хосте, объявленном этой сборкой. С решения 181.33 —
        // страница на языке окна.
        Action::OpenProgram => open_link(links::program_url(deed.language)),

        Action::OpenLink => open_link(deed.link),

        Action::OpenAuthor => open_author(deed.owner),
        Action::OpenLetters => open_list(deed.owner),

        Action::OpenWizard => open_wizard(deed.owner),
    }

    false
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
///
/// # The gate of С52 — task T-41-1
///
/// [`links::is_allowed`] is asked **here**, on the near side of the one door of the program,
/// and not in the reading of the feed. A check in the parser would be a check one future caller
/// can walk round; a check on the door is the door. The button of an address that fails it is
/// not drawn either, and that is a second courtesy rather than the protection.
///
/// ⚠ The owner window went with task T-41-4: [`settings::open_in_the_shell`] is shared by three
/// callers and has no one window to give the shell's error box to.
///
/// ⭐ `pub(crate)` since task T-78-3: the name «Lang Switcher» of «О программе» opens the
/// program's page, and that window lives in `settings`. It opens it **by this function** and not
/// by a second road of its own — the gate of С52 above is the whole reason this function exists,
/// and a caller that reached the shared door past it would be the future caller the paragraph
/// above says a check in the parser could not stop.
pub(crate) fn open_link(url: &str) {
    if url.is_empty() || links::is_placeholder(url) {
        return;
    }

    if !links::is_allowed(url) {
        note_link_refused();
        return;
    }

    // Finding С26, task T-41-4: through the one door of the program. What that door does
    // inside itself is an open question — see [`settings::open_in_the_shell`] — but there is
    // one of it, and a sweep keeps it so.
    settings::open_in_the_shell(url);
}

/// Writes down that an address was turned away by the gate of С52 — task T-41-1.
///
/// **Nothing of the address is kept** — not the host, not the scheme, not a character of it.
/// SEC-07 says what may reach the journal is a name chosen at compile time, and «a link was
/// refused» is that name; which link it was is exactly the text that must not travel.
///
/// It exists so that the refusal is not a silence. A button that is not drawn and a click that
/// does nothing look, from where the person stands, like the program being broken; one line in
/// the journal is the difference between «не работает» and «программа отказалась и сказала об
/// этом».
fn note_link_refused() {
    crate::diag::record(
        crate::diag::Operation::from_name("link refused"),
        crate::diag::OsCode::NONE,
    );
}

/// Raises a window that is already open — the answer to «open it again».
fn raise(hwnd: HWND, activate: bool) {
    // The measuring door of the tests keeps its windows off the screen — see [`testing`].
    #[cfg(feature = "testing")]
    if testing::hidden() {
        return;
    }

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
        // person pressed «Открыть письмо» of the knock or asked for the window from the menu, and a window that
        // did not come to the front then would look broken.
        //
        // SAFETY: `hwnd` is the live window; the call takes no pointer.
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}

/// **Hides the focus cue of a window that opens by itself** — FR-101, задача T-94-1, решение
/// 158.3.
///
/// Находка 158: the Welcome letter came up on a clean machine with the dotted focus frame on
/// «Открыть настройки», as if the button had been chosen in advance. Nothing had chosen it: the
/// dialog manager gives the focus to the first tab stop of every dialog, and whether that focus is
/// *drawn* is up to the keyboard cues, which Windows decides by the last input — and the last input
/// of a person typing in another program is a key. A window the person did not ask for has no
/// business pointing at a button, so its focus cue stays hidden until the person presses Tab in
/// it: the dialog manager lifts the ban itself on the first Tab, and from then on the frame moves
/// with the focus as in a window opened by the mouse. The focus stays where the manager put it
/// (решение 158.3: not on «Понятно»), so Enter still presses the default button and Space the
/// focused one.
///
/// ⛔ **Not in `WM_INITDIALOG`.** Windows decides the cues of a dialog — `WM_CHANGEUISTATE` with
/// `UIS_INITIALIZE` — *after* `WM_INITDIALOG` and before the window is shown (замер 158.2, eight
/// openings of eight), and that decision would undo a ban set any earlier. The moment after the
/// dialog manager has returned the window and before [`raise`] shows it is the one that comes both
/// after the decision and before the first paint.
///
/// `SendMessageTimeoutW` and not a bare send, for the reason [`wizard_refresh`] gives (FR-72, the
/// sweep of `tests\guard.rs`, урок Т-33а-0): the window lives on this very thread, so the call goes
/// straight to its procedure and the timeout is never spent. A refused ban leaves the cues as
/// Windows decided them — the state before this function (NFR-13).
fn hide_focus_cue(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SMTO_ABORTIFHUNG, SendMessageTimeoutW, UIS_SET, UISF_HIDEFOCUS, WM_CHANGEUISTATE,
    };

    /// How long to wait for the answer — nominal, as in [`wizard_refresh`]: the window is of this
    /// thread, and its procedure is called inside the call.
    const CUE_TIMEOUT_MS: u32 = 50;

    // MAKEWPARAM(UIS_SET, UISF_HIDEFOCUS): the action in the low word, the cue in the high one.
    let change = (UIS_SET | (UISF_HIDEFOCUS << 16)) as usize;

    // SAFETY: `hwnd` is the live window this thread has just made; the message carries two plain
    // numbers and no pointer, and the out-parameter is `None`, so nothing of ours is dereferenced.
    // NFR-13: a refused ban leaves the cues as Windows decided them, which is the state before the
    // call — and, as in [`wizard_refresh`], nothing goes to the journal: the journal names every
    // operation it carries (`tests\diag.rs`), and a ban that cannot fail on this thread has no
    // name there.
    let _ = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_CHANGEUISTATE,
            WPARAM(change),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            CUE_TIMEOUT_MS,
            None,
        )
    };
}

/// Builds one of these windows, or raises the one that is already up.
///
/// `activate` is FR-101's whole rule about focus: a letter that opens **by itself** is shown
/// with `SW_SHOWNOACTIVATE` and never calls `SetForegroundWindow`, so it cannot take the
/// keyboard from what somebody is typing into. A window the person asked for — from the menu,
/// from «О программе», by «Открыть письмо» of the knock — is shown the ordinary way.
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

    // The measuring door of the tests names the module the templates come out of — see
    // [`testing`].
    #[cfg(feature = "testing")]
    let module = testing::templates().unwrap_or(module);

    let template = match kind {
        Kind::Wizard => IDD_WIZARD,
        // The letters, the list and «От автора» are windows of the shell.
        Kind::Letter | Kind::List | Kind::Author => return None,
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

    // Решение 158.3: a window that opens by itself comes up without the focus cue. Here — after
    // the creation, where Windows has already decided the cues, and before the window is shown.
    // A window raised again above keeps the cues it has: they belong to the person, who may have
    // pressed Tab in it already.
    if !activate {
        hide_focus_cue(hwnd);
    }

    raise(hwnd, activate);

    Some(hwnd)
}

/// The state one of these windows starts with — the palette out of the configuration, the
/// hotkey as it acts, and the plan.
fn fresh_state(kind: Kind) -> WindowState {
    let (setting, hotkey) = look();

    let palette = theme::resolve(setting, theme::system_is_light());

    WindowState {
        kind,
        setting,
        palette,
        brushes: theme::Brushes::new(palette),
        fonts: None,
        wizard: None,
        hotkey,
        icons: settings::CaptionIcons::load(),
        language: settings::ui_language(),
    }
}

/// The setting «Оформление» and the hotkey as it acts, out of the configuration — what every
/// window of this module starts with.
fn look() -> (ThemeSetting, String) {
    let (setting, key) = crate::tray::with_tray(|tray| {
        (
            tray.config().general.theme,
            settings::effective_hotkey_name(&tray.config().hotkey),
        )
    })
    .unwrap_or_else(|| {
        (
            ThemeSetting::System,
            settings::effective_hotkey_name(&settings::Hotkey::default()),
        )
    });
    // The stand of the letters names the theme itself: it has no tray (`testing::run_probe`).
    #[cfg(feature = "testing")]
    let setting = testing::theme().unwrap_or(setting);
    (setting, key)
}

/// One open window of the shell this module made — a letter or «Последние письма».
#[derive(Clone, Copy, Debug)]
struct ShellWindow {
    kind: Kind,
    hwnd: HWND,
    /// Which letter, for a window of [`Kind::Letter`] — what a new language opens again.
    letter: Option<Letter>,
    /// The locale the window was built in — what `language_switch` is asked about.
    language: Language,
}

thread_local! {
    /// The windows of the shell this module opened **on this thread**: the letters and
    /// «Последние письма» (T-120-16). Apart from [`OPEN`], because the message pump hands the
    /// windows of `OPEN` to `IsDialogMessageW`, and a window of the shell is not a dialog — its
    /// keys are the host's.
    static SHELL: RefCell<Vec<ShellWindow>> = const { RefCell::new(Vec::new()) };
}

/// The place of a window of the shell in [`SHELL`]: the content holds it, and when the host drops
/// the content — the window is destroyed, however it was closed — the place goes with it.
pub struct ShellRecord(Kind);

impl Drop for ShellRecord {
    fn drop(&mut self) {
        let kind = self.0;
        let _ = SHELL.try_with(|open| {
            if let Ok(mut open) = open.try_borrow_mut() {
                open.retain(|window| window.kind != kind);
            }
        });
    }
}

/// Opens `content` as a window of the shell of `kind` on `owner` and writes it into [`SHELL`];
/// answers the window, or none — a window Windows refused is journaled, and the person is told by
/// its absence.
///
/// `activate` is FR-101's whole rule about focus: a letter that opens **by itself** is shown with
/// `SW_SHOWNOACTIVATE` and never takes the keyboard from what somebody is typing into; a window the
/// person asked for is shown the ordinary way ([`raise`]).
fn open_shell<C: crate::ui::host::Content + 'static>(
    kind: Kind,
    letter: Option<Letter>,
    language: Language,
    owner: HWND,
    content: C,
    activate: bool,
) -> Option<HWND> {
    use crate::ui::host;

    // The door of the tests keeps the window hidden (`testing`): made, never shown.
    #[cfg(feature = "testing")]
    let made = if testing::hidden() {
        host::create(content, Some(owner))
    } else if activate {
        host::open(content, Some(owner))
    } else {
        host::open_inactive(content, Some(owner))
    };
    #[cfg(not(feature = "testing"))]
    let made = if activate {
        host::open(content, Some(owner))
    } else {
        host::open_inactive(content, Some(owner))
    };

    match made {
        Ok(hwnd) => {
            SHELL.with_borrow_mut(|open| {
                open.push(ShellWindow {
                    kind,
                    hwnd,
                    letter,
                    language,
                });
            });
            // A window the person asked for comes to the front, as the old one did.
            if activate {
                raise(hwnd, true);
            }
            Some(hwnd)
        }
        Err(error) => {
            crate::app::report_non_critical("CreateWindowExW", &error);
            None
        }
    }
}

/// **Shows one letter** — FR-101, a window of the shell (`ui::windows::letter`, T-120-16).
///
/// Answers whether a window is on the screen. `false` means the letter had nothing to say (a
/// letter out of the feed with no entry behind it) or the window could not be made — and the
/// caller must then not record the letter as shown. One letter at a time: while one is open it is
/// raised instead.
pub fn show_letter(owner: HWND, letter: Letter, item: Option<&FeedItem>, activate: bool) -> bool {
    show_letter_in(settings::ui_language(), owner, letter, item, activate)
}

/// [`show_letter`] in `language` — the language of the program, or the one the stand of the letters
/// names (`testing::run_probe`): the window is given its language, and the language of the process
/// is published in one place only (решение 99, `tests\tray.rs`).
fn show_letter_in(
    language: Language,
    owner: HWND,
    letter: Letter,
    item: Option<&FeedItem>,
    activate: bool,
) -> bool {
    let stored = state_now();

    let plan = plan_in(
        language,
        letter,
        &PlanContext {
            version: version_string(),
            hotkey: String::new(),
            snooze_offered: snooze_is_offered(&stored),
            item,
        },
    );

    if plan.title.is_empty() {
        return false;
    }

    if let Some(hwnd) = window_of(Kind::Letter) {
        raise(hwnd, activate);
        return true;
    }

    let (theme, key) = look();
    let product = crate::ui::windows::letter::Product {
        view: crate::ui::windows::letter::LetterView {
            letter,
            plan,
            key,
            tick: DEMO_STILL,
            language,
            theme,
        },
        owner,
        news: match letter {
            Letter::News(id) => Some(id),
            _ => None,
        },
        link: item.map(|item| item.link.clone()).unwrap_or_default(),
        _record: ShellRecord(Kind::Letter),
    };

    open_shell(
        Kind::Letter,
        Some(letter),
        language,
        owner,
        product,
        activate,
    )
    .is_some()
}

/// **Shows «От автора»** — FR-103.
pub fn open_author(owner: HWND) {
    open_author_with(owner, &version_string(), true);
}

/// [`open_author`] for `version` — the product's road and the door of the tests (a test binary
/// carries no version resource of its own). A window of the shell (T-120-17); the one that is open
/// is raised instead.
fn open_author_with(owner: HWND, version: &str, activate: bool) -> Option<HWND> {
    if let Some(hwnd) = window_of(Kind::Author) {
        raise(hwnd, activate);
        return Some(hwnd);
    }

    let stored = state_now();
    let language = settings::ui_language();
    let view = with_feed(|feed| {
        author_view_in(
            language,
            &stored,
            today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day")),
            version,
            feed,
        )
    });
    open_author_view(owner, language, view, activate)
}

/// «От автора» saying `view` in `language` — [`open_author_with`] and the door of the tests that
/// hands the window the words of a machine whose feed is live.
fn open_author_view(
    owner: HWND,
    language: Language,
    view: AuthorView,
    activate: bool,
) -> Option<HWND> {
    if let Some(hwnd) = window_of(Kind::Author) {
        raise(hwnd, activate);
        return Some(hwnd);
    }
    let (theme, _) = look();
    let product = crate::ui::windows::author::Product {
        author: crate::ui::windows::author::Author {
            view,
            language,
            theme,
        },
        owner,
        _record: ShellRecord(Kind::Author),
    };

    open_shell(Kind::Author, None, language, owner, product, activate)
}

/// **The measuring door of stage Э83** — hidden windows of this module for the tests, compiled
/// only under the `testing` feature (SEC-04a; criterion 8 finds no trace of it in the Release
/// build).
///
/// A test binary carries no templates — `embed-resource` links `app.rc` into the product only —
/// and a window of this module shown on the owner's screen by a test would be a window on the
/// owner's screen. So the door takes the templates out of a module the test names (the product
/// image it maps as a data file) and keeps the window **hidden**: it is built, laid out and
/// measured exactly as the product builds it, and never shown.
#[cfg(feature = "testing")]
pub mod testing {
    use super::{
        AuthorView, Entry, FeedItem, HWND, Kind, Letter, ThemeSetting, open_window, show_letter,
        window_of, with_state, wizard_state,
    };
    use std::cell::Cell;
    use windows::Win32::Foundation::HMODULE;

    thread_local! {
        /// Whether [`super::raise`] must leave the window hidden.
        static HIDDEN: Cell<bool> = const { Cell::new(false) };
        /// The module the templates come out of, or zero for the running program's own.
        static TEMPLATES: Cell<usize> = const { Cell::new(0) };
    }

    /// Whether the window being opened is to stay hidden.
    pub(super) fn hidden() -> bool {
        HIDDEN.with(Cell::get)
    }

    /// The module the templates come out of while the door is open.
    pub(super) fn templates() -> Option<HMODULE> {
        let raw = TEMPLATES.with(Cell::get);

        (raw != 0).then(|| HMODULE(std::ptr::without_provenance_mut(raw)))
    }

    /// Opens «От автора» on `owner` out of the templates of `module`, for `version`, and never
    /// shows it. The window answered is the caller's to destroy.
    pub fn open_author_hidden(owner: HWND, _module: HMODULE, version: &str) -> Option<HWND> {
        hidden_while(|| super::open_author_with(owner, version, false))
    }

    /// **The door of task T-98-1** — opens «От автора» on `owner` out of the templates of `module`
    /// with the words of `view`, and never shows it.
    ///
    /// [`open_author_hidden`] builds the words out of the configuration and the feed of the running
    /// program, and a test binary has neither: its window is always the one of a first day, with
    /// no switch of FR-102 and no «Последние письма». The window of a machine whose feed is live
    /// is a view of [`super::author_view`] the caller builds, and the window lays it out exactly
    /// as the product does (Э83-Б-1, решение 166.2).
    ///
    /// The window answered is the caller's to destroy.
    pub fn open_author_hidden_with(
        owner: HWND,
        _module: HMODULE,
        view: AuthorView,
    ) -> Option<HWND> {
        hidden_while(|| super::open_author_view(owner, super::settings::ui_language(), view, false))
    }

    /// Runs `f` with the windows of this module left hidden on this thread.
    fn hidden_while<R>(f: impl FnOnce() -> R) -> R {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                HIDDEN.with(|hidden| hidden.set(false));
            }
        }

        HIDDEN.with(|hidden| hidden.set(true));
        let _reset = Reset;
        f()
    }

    /// **The door of the tests of «Последние письма»** — opens the window of the shell on `owner`
    /// with `entries` by the product's own road, and never shows it. A test binary reads no feed,
    /// so the entries are those of [`super::list_entries`] the caller builds.
    ///
    /// The window answered is the caller's to destroy.
    pub fn open_list_hidden(owner: HWND, entries: Vec<Entry>) -> Option<HWND> {
        hidden_while(|| {
            super::open_list_with(owner, super::settings::ui_language(), entries, false)
        })
    }

    /// **The door of the tests of the letters** — opens `letter` on `owner`, about `item` when it
    /// is a letter out of the feed, by the product's own [`show_letter`] **with the activation** —
    /// the road of a window the person asked for — and keeps it **hidden**: a shown window opened
    /// with the activation calls `SetForegroundWindow`, which no test may call.
    ///
    /// The window answered is the caller's to destroy.
    pub fn open_letter_hidden(
        owner: HWND,
        letter: Letter,
        item: Option<&FeedItem>,
    ) -> Option<HWND> {
        hidden_while(|| show_letter(owner, letter, item, true))
            .then(|| window_of(Kind::Letter))
            .flatten()
    }

    /// The letter open on this thread now — a new window after a change of language, which builds
    /// the letter again.
    pub fn letter_window() -> Option<HWND> {
        window_of(Kind::Letter)
    }

    /// [`super::language_changed`] with the windows of this module left hidden: the letter and the
    /// list it opens again in the new language are not shown on the screen of a test.
    pub fn language_changed_hidden(owner: HWND) {
        hidden_while(|| super::language_changed(owner));
    }

    /// **The door of task T-102-3** — gives the open window `hwnd` the theme setting `setting`, the
    /// way `[general].theme` of the configuration gives it to a window of the running program.
    ///
    /// A test binary has no tray to read the configuration from, so its windows follow the theme of
    /// the system. The snapshots of «Привет» in both palettes (вопрос 178) set the setting here and
    /// then post `settings::WM_APP_SYSTEM_THEME`: the window resolves its palette afresh and repaints
    /// by its own answer to a moved theme — nothing of the drawing is the door's.
    ///
    /// `hwnd` is a window of this module on the caller's thread.
    pub fn set_theme(hwnd: HWND, setting: ThemeSetting) {
        // SAFETY: `hwnd` is a live window of this module on the caller's thread — the contract of
        // this door — and the borrow of its state ends with the call.
        let _ = unsafe { with_state(hwnd, |state| state.setting = setting) };
    }

    /// **The door of task T-103-2** — opens the wizard of FR-104 on `owner` out of the templates of
    /// `module` by the product's own [`wizard_state`], and never shows it.
    ///
    /// The window answered is the caller's to destroy. A caller that stands on a desktop of its
    /// own may show it itself, for the snapshots.
    pub fn open_wizard_hidden(owner: HWND, module: HMODULE) -> Option<HWND> {
        HIDDEN.with(|hidden| hidden.set(true));
        TEMPLATES.with(|templates| templates.set(module.0 as usize));

        let window = open_window(owner, wizard_state(), false);

        HIDDEN.with(|hidden| hidden.set(false));
        TEMPLATES.with(|templates| templates.set(0));

        window
    }

    /// **The door of task T-103-2** — where the button of the open window `hwnd` that sends a
    /// person to write would take them: the address «Скопировать и открыть…» of the wizard and
    /// «Открыть группу» of «Спасибо» hand to the shell, read by the very call they read it by
    /// (решение 181.1). Nothing is opened.
    ///
    /// `hwnd` is a window of this module on the caller's thread.
    pub fn feedback_url(hwnd: HWND) -> Option<&'static str> {
        // ⚠ A window of the shell first: its `GWLP_USERDATA` is the host's, and `with_state` would
        // read it as the state of a dialog of this module.
        if let Some(language) = shell_language(hwnd) {
            return Some(super::links::feedback_url(language));
        }
        // SAFETY: `hwnd` is a live dialog of this module on the caller's thread — the contract of
        // this door — and the borrow of its state ends with the call.
        unsafe { with_state(hwnd, |state| super::links::feedback_url(state.language)) }
    }

    /// The language a window of the shell of this module was built in — what its pages are
    /// chosen by (`letters::act`).
    fn shell_language(hwnd: HWND) -> Option<super::Language> {
        super::SHELL.with_borrow(|open| {
            open.iter()
                .find(|window| window.hwnd == hwnd)
                .map(|window| window.language)
        })
    }

    /// **The door of task T-103-9** — the two pages of the site the open window `hwnd` would take
    /// a person to: `(download, program)`, what «Открыть страницу загрузки» and the name of the
    /// program hand to the shell, read by the very calls `perform` reads them by
    /// (решение 181.33). Nothing is opened.
    ///
    /// `hwnd` is a window of this module on the caller's thread.
    pub fn site_pages(hwnd: HWND) -> Option<(&'static str, &'static str)> {
        if let Some(language) = shell_language(hwnd) {
            return Some((
                super::links::download_url(language),
                super::links::program_url(language),
            ));
        }
        // SAFETY: as for `feedback_url` above — a dialog of this module, not a window of the shell.
        unsafe {
            with_state(hwnd, |state| {
                (
                    super::links::download_url(state.language),
                    super::links::program_url(state.language),
                )
            })
        }
    }

    thread_local! {
        /// The setting «Оформление» the windows of this module take instead of the configuration's
        /// — the stand has no tray to read it from ([`run_probe`]).
        static THEME: Cell<Option<ThemeSetting>> = const { Cell::new(None) };
    }

    /// The theme the stand asked for, if it asked.
    pub(super) fn theme() -> Option<ThemeSetting> {
        THEME.with(Cell::get)
    }

    /// **The entry of the stand of the letters — `--ui-probe-letters` (T-120-16)**: the testing
    /// build opens one window of this module alone, by the product's own road, and pumps its
    /// messages until it is gone. What it opens:
    ///
    /// * `LANGSW_TESTING_UI_PROBE_LETTER` — `hello` (the default), `thanks`, `news`, `update` (the
    ///   letters out of the feed about the sample entry), `list` (full), `knock-thanks`,
    ///   `knock-news` (the window in the corner);
    /// * `LANGSW_TESTING_UI_PROBE_LANG` — the tag of the language of the interface;
    /// * `LANGSW_TESTING_UI_PROBE_THEME` — `light` or `dark`; else the theme of the system;
    /// * `LANGSW_TESTING_UI_PROBE_BY_ITSELF` — the letter opens as one that came by itself, without
    ///   the activation (FR-101); else as one the person asked for.
    ///
    /// The marks go to the file `LANGSW_TESTING_UI_PROBE` names, and the window closes itself after
    /// `LANGSW_TESTING_UI_PROBE_CLOSE_MS` — the conventions of the probes of the shell.
    pub fn run_probe() -> std::process::ExitCode {
        use windows::Win32::Foundation::{LPARAM, RECT, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DispatchMessageW, FindWindowW, GetForegroundWindow, GetMessageW,
            GetWindowRect, IsWindow, MSG, PostMessageW, TranslateMessage, WM_CLOSE,
            WS_EX_TOOLWINDOW, WS_POPUP,
        };
        use windows::core::{PCWSTR, w};

        let report = std::env::var_os("LANGSW_TESTING_UI_PROBE");
        let mut lines = String::new();
        let mut mark = |line: String| {
            lines.push_str(&line);
            lines.push_str("\r\n");
            if let Some(path) = &report {
                let _ = std::fs::write(path, &lines);
            }
        };
        let var = |name: &str| std::env::var(name).unwrap_or_default();

        let language = super::Language::ALL
            .into_iter()
            .find(|language| language.tag() == var("LANGSW_TESTING_UI_PROBE_LANG"))
            .unwrap_or_else(super::settings::ui_language);
        let theme = match var("LANGSW_TESTING_UI_PROBE_THEME").as_str() {
            "light" => Some(ThemeSetting::Light),
            "dark" => Some(ThemeSetting::Dark),
            _ => None,
        };
        THEME.with(|cell| cell.set(theme));
        let what = var("LANGSW_TESTING_UI_PROBE_LETTER");
        let by_itself = std::env::var_os("LANGSW_TESTING_UI_PROBE_BY_ITSELF").is_some();
        mark(format!(
            "letter={what} language={} theme={theme:?} by_itself={by_itself}",
            language.tag()
        ));

        // SAFETY: a system class, no parent, no pointer of ours; the owner of the window, alive to
        // the end of the process.
        let Ok(owner) = (unsafe {
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
                None,
                None,
            )
        }) else {
            return std::process::ExitCode::FAILURE;
        };
        // SAFETY: reads the foreground window.
        let before = unsafe { GetForegroundWindow() };

        let item = super::sample_entry();
        let window = match what.as_str() {
            "list" => {
                super::open_list_with(owner, language, super::sample_entries(language), !by_itself)
            }
            "knock-thanks" | "knock-news" => {
                let letter = if what == "knock-thanks" {
                    Letter::Thanks
                } else {
                    Letter::News(item.id)
                };
                let shown = super::notice_content_in(language, letter)
                    .is_some_and(|content| crate::corner::show(content, false, |_| {}));
                mark(format!("shown={shown}"));
                // SAFETY: the class of the popups of the shell, a static name.
                unsafe { FindWindowW(w!("LangSwitcher.Shell.Popup"), None) }.ok()
            }
            other => {
                let letter = match other {
                    "thanks" => Letter::Thanks,
                    "news" => Letter::News(item.id),
                    "update" => Letter::Update,
                    _ => Letter::Welcome,
                };
                super::show_letter_in(language, owner, letter, Some(&item), !by_itself)
                    .then(|| window_of(Kind::Letter))
                    .flatten()
            }
        };
        let Some(window) = window else {
            mark("window=none".to_owned());
            return std::process::ExitCode::FAILURE;
        };

        let mut rect = RECT::default();
        // SAFETY: a live window of this thread; the out parameter is a local.
        let after = unsafe {
            let _ = GetWindowRect(window, &mut rect);
            GetForegroundWindow()
        };
        mark(format!("window={}", window.0 as usize));
        mark(format!(
            "rect={},{},{},{}",
            rect.left, rect.top, rect.right, rect.bottom
        ));
        mark(format!("foreground_is_window={}", after == window));
        mark(format!("foreground_kept={}", after == before));

        if let Some(close) = std::env::var("LANGSW_TESTING_UI_PROBE_CLOSE_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            let raw = window.0 as usize;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(close));
                // SAFETY: a posted message to the probe's window; a window already gone ignores it.
                let _ = unsafe {
                    PostMessageW(Some(HWND(raw as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0))
                };
            });
        }
        let mut message = MSG::default();
        // SAFETY: the message loop of this thread; it ends when the window is gone.
        while unsafe { IsWindow(Some(window)) }.as_bool()
            && unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool()
        {
            // SAFETY: a message just taken from the queue.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        mark("closed=true".to_owned());
        std::process::ExitCode::SUCCESS
    }
}

/// What «Последние письма» shows now — [`list_entries`] of the stored state and the published
/// feed, today.
pub fn list_entries_now() -> Vec<Entry> {
    let stored = state_now();
    let version = version_string();
    with_feed(|feed| list_entries(&stored, today(), &version, feed))
}

/// The entry of the feed the instrument of fitting (`ui::windows::catalogue`) puts into
/// «Новость» and «Обновление»: near the longest the order of the feed allows — the first line of
/// the text 145 characters of the 150 (`sign-news.ps1`), and the line «Подробности…» under it.
/// In the words of the feed, which come in the language of the site whatever the language of the
/// program.
pub fn sample_entry() -> FeedItem {
    FeedItem {
        id: 7,
        date: Date::from_ymd(2026, 11, 16),
        title: "Программа теперь обновляется сама".to_owned(),
        text: format!(
            "{}\nПодробности — на странице программы.",
            "С версии 0.104.0 Lang Switcher сам скачивает новые версии с сайта автора и ставит \
             их, проверив подпись. Установите новую версию один раз — дальше"
        ),
        link: links::PROGRAM_URL.to_owned(),
        version: Some("1.7.1".to_owned()),
    }
}

/// «Последние письма» full, for the instrument of fitting: the row of the update in the words of
/// its letter and three news items, two of them unread — of [`sample_entry`], in `language`.
pub fn sample_entries(language: Language) -> Vec<Entry> {
    let item = sample_entry();
    let date = item.date.map(format_date).unwrap_or_default();
    let mark = |id| settings::text_in(language, id).replace("{0}", &date);
    let entry = |news: Option<u64>, unread: bool| Entry {
        title: item.title.clone(),
        meta: mark(if unread {
            settings::IDS_ENTRY_UNREAD_MARK
        } else {
            settings::IDS_ENTRY_READ_MARK
        }),
        excerpt: first_line(&item.text),
        news,
        unread,
    };
    // The row of the update has the words of its letter, as `list_entries` gives them.
    let update = Entry {
        title: settings::text_in(language, settings::IDS_FALLBACK_TITLE),
        meta: settings::text_in(language, settings::IDS_ENTRY_FALLBACK_MARK)
            .replace("{0}", item.version.as_deref().unwrap_or_default())
            .replace("{1}", "1.8.0"),
        excerpt: first_line(&item.text),
        news: None,
        unread: false,
    };
    vec![
        update,
        entry(Some(9), true),
        entry(Some(8), true),
        entry(Some(7), false),
    ]
}

/// «От автора» for the instrument of fitting, in `language`: the fullest the window gets — the feed
/// read, a newer version announced by [`sample_entry`], news to show in «Последние письма», the switch
/// on, and `[letters] feed = false` said under it (the longest of its two states).
pub fn author_view_sample(language: Language) -> AuthorView {
    let item = sample_entry();
    let news = [sample_entry()];
    let today = item
        .date
        .unwrap_or_else(|| Date::from_ymd(2026, 11, 16).expect("a day"));
    let state = Letters {
        feed: false,
        feed_last_read: Some(today),
        release_window: true,
        ..Letters::default()
    };
    let feed = FeedView {
        update: Some(&item),
        news: &news,
    };
    author_view_in(language, &state, today, "1.7.0", feed)
}

/// **Shows «Последние письма»** — FR-101, a window of the shell (`ui::windows::letters_list`,
/// T-120-16); the one that is open is raised instead.
pub fn open_list(owner: HWND) {
    open_list_with(owner, settings::ui_language(), list_entries_now(), true);
}

/// [`open_list`] with the entries named — the product's road and the door of the tests.
fn open_list_with(
    owner: HWND,
    language: Language,
    entries: Vec<Entry>,
    activate: bool,
) -> Option<HWND> {
    if let Some(hwnd) = window_of(Kind::List) {
        raise(hwnd, activate);
        return Some(hwnd);
    }

    let (theme, _) = look();
    let product = crate::ui::windows::letters_list::Product {
        list: crate::ui::windows::letters_list::LettersList {
            entries,
            language,
            theme,
        },
        owner,
        _record: ShellRecord(Kind::List),
    };

    open_shell(Kind::List, None, language, owner, product, activate)
}

thread_local! {
    /// The letter the knock in the corner announced and that nobody has opened yet — FR-101.
    ///
    /// Cleared when the letter is shown, however it is shown: by «Открыть письмо» of the knock, or
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

/// ⭐ **The marked read was answered — stage E116, task T-116-12.** Set by the feed thread when the
/// read carried `?first=1` and an HTTP answer came back, of any status; taken by [`take_feed`] on the
/// UI thread, which writes the mark as sent. An atomic and not the mailbox: the mailbox holds only a
/// good feed, and a refused document is an answer all the same (решение 208.59).
static FIRST_ANSWERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// **Starts the one read of FR-102**, on a thread that lives exactly one request.
///
/// SPEC section 6.1: a fourth thread, short-lived, doing no user interface, no input and no
/// hook. It reads, checks the signature, parses, leaves the answer in [`MAILBOX`] and posts
/// [`WM_APP_FEED`] to the UI window. Then it ends.
///
/// Nothing at all happens when the feed is off (`[letters] feed = false`): no thread is
/// started, so there is no timer, no socket and no name to resolve — which is what makes the
/// switch of FR-102 a real switch and not a filter on the answer.
///
/// ⭐ Stage E116, task T-116-12: `first` — the read carries `?first=1` ([`feed::read_now_marked`]).
pub fn start_feed_read(owner: HWND, language: String, first: bool) {
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
            let (answer, answered) = feed::read_now_marked(&language, first);

            // Stage E116, task T-116-12: the mark goes out with the first answer, of any status.
            if first && answered {
                FIRST_ANSWERED.store(true, Ordering::Release);
            }

            // ⭐ Task T-119-9, решение 208.98: the good document is kept on disk as it came, so
            // that a restart on the same day still has the feed ([`restore_feed`]). Written here,
            // on the feed's own thread, and before the nudge — the UI thread never waits on a disk.
            let answer = answer.map(|(fresh, document)| {
                keep_feed_document(&document);
                fresh
            });

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
    // ⭐ Stage E116, task T-116-12: the marked read was answered — the mark is sent, whether or not the
    // document was good, and before the mailbox is asked (it holds only a good one).
    if FIRST_ANSWERED.swap(false, std::sync::atomic::Ordering::AcqRel) {
        update_state(|state| state.first_mark = first_mark_after(state.first_mark, true));
    }

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

    let version = version_string();

    update_state(|state| {
        state.feed_last_read = Some(today);
        // TZ E104 §0 п. 20: the day the feed first named a newer version, for the fallback.
        notice_update(state, &version, view, today);
        forget_expired(state, view);
    });

    crate::tray::refresh_unread_mark();

    crate::diag::record(
        crate::diag::Operation::from_name("feed read ok"),
        crate::diag::OsCode::NONE,
    );

    // Task T-41-1, finding С52. The entry is **kept** — news with an address this program will
    // not open is still news — and the button that would open it is simply not drawn. One line
    // per such entry, written here and not in `plan_for`: this runs once per read of the feed,
    // where the planner runs every time a letter is built and is a pure function besides.
    for _ in fresh
        .news
        .iter()
        .chain(fresh.update.as_ref())
        .filter(|item| !item.link.is_empty() && !links::is_allowed(&item.link))
    {
        note_link_refused();
    }
}

/// **Keeps the good feed on disk** — task T-119-9, решение 208.98 (п. 2 of 208.97): the signed
/// document as it came, [`settings::kept_feed_path`], written atomically by the body that writes
/// the configuration. Called on the feed's own thread. A write that fails leaves the old copy and
/// a line in the journal; nothing else depends on it.
fn keep_feed_document(document: &str) {
    let Some(path) = settings::kept_feed_path() else {
        return;
    };

    if settings::write_bytes_to(&path, document.as_bytes()).is_err() {
        crate::diag::record(
            crate::diag::Operation::from_name("feed keep"),
            crate::diag::OsCode::NONE,
        );
    }
}

/// **The kept feed, read again** — task T-119-9, решение 208.98: the document of
/// [`keep_feed_document`] goes through [`feed::read_document`] with the author's keys **every
/// time**, exactly as an answer of the site does. A file that is spoilt, cut, foreign or signed by
/// anybody else is **no feed at all** (`None`), never half of one.
pub fn kept_feed(document: &str, language: &str) -> Option<feed::Feed> {
    let (_, keys) = feed::sources();

    feed::read_document(document, language, |body, signature| {
        feed::verify_with(body, signature, &keys)
    })
    .ok()
}

/// **Puts the kept feed back after a start** — task T-119-9, решение 208.98: a restart on the
/// same day, after the day's read, no longer loses the entry «Доступна версия X…», the letter and
/// the row of «Последние письма» until tomorrow. On the UI thread, once, before the first tick.
///
/// Nothing when the feed is off (`[letters] feed = false`): a feed turned off shows nothing, kept
/// or fresh. Only the published feed and the day the newer version was first named
/// ([`notice_update`]) move; the day of the read does not — this is not a read.
pub fn restore_feed() {
    let stored = state_now();

    if !stored.feed {
        return;
    }

    let Some(document) =
        settings::kept_feed_path().and_then(|path| std::fs::read_to_string(path).ok())
    else {
        return;
    };

    let Some(kept) = kept_feed(&document, settings::ui_language().tag()) else {
        crate::diag::record(
            crate::diag::Operation::from_name("kept feed refused"),
            crate::diag::OsCode::NONE,
        );
        return;
    };

    publish_feed(kept.update.clone(), kept.news.clone());

    if let Some(today) = today() {
        let mut moved = stored;
        let view = FeedView {
            update: kept.update.as_ref(),
            news: &kept.news,
        };

        if notice_update(&mut moved, &version_string(), view, today) {
            update_state(move |state| *state = moved);
        }
    }

    crate::tray::refresh_unread_mark();
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
///    at once), and the window itself opens at the next tick if the knock went unnoticed.
pub fn tick(owner: HWND) {
    let Some(today) = today() else {
        return;
    };

    let version = version_string();
    let mut stored = state_now();

    // TZ E104 §0 п. 21: asked BEFORE `initialise`, which moves the version seen to this build's
    // on the same tick — and that move is what makes the notification one per version.
    let updated = was_updated(&stored, &version);

    if initialise(&mut stored, today, &version) {
        let fresh = stored.clone();

        update_state(move |state| *state = fresh);
    }

    // FR-102: the one read of the feed, when it is due — started **before** the letters are
    // looked at, on a thread of its own. Nothing waits for it: the answer is taken the moment
    // the thread posts `WM_APP_FEED`, and the letters it brings are looked at on the next tick.
    //
    // ⭐ Task T-88-1, решение 148.8: the day of the attempt is written **first**, whatever the
    // read comes to. A host that does not answer is asked again tomorrow and not at every tick,
    // and the day is in the file, so a restart does not buy another attempt either.
    if feed_read_is_due(&stored, today) {
        update_state(|state| state.feed_last_try = Some(today));
        start_feed_read(
            owner,
            settings::ui_language().tag().to_owned(),
            first_query(stored.first_mark),
        );
    }

    // TZ E104 §0 п. 21: the first tick after an update belongs to the update and to nothing else;
    // the letters wait for the next tick. ⛔ Its balloon «Lang Switcher обновлён до X» (`NIF_INFO`,
    // T-104-7) is gone — решение 208.94 п. 4, задача T-119-8: the window «Обновлено» replaces it.
    if updated {
        return;
    }

    let announced = ANNOUNCED.with_borrow(|announced| *announced);
    let quiet = moment_is_quiet();

    let due = with_feed(|feed| due(&stored, today, &version, feed, quiet));

    let Some(letter) = due else {
        return;
    };

    // A letter that was announced at an earlier tick and not opened is **shown** now: the
    // knock in the corner went unanswered, and FR-101 says the window then opens by
    // itself in a quiet moment, without the focus.
    if announced == Some(letter) || letter == Letter::Welcome {
        if show(owner, letter, today, &version, false) {
            ANNOUNCED.with_borrow_mut(|announced| *announced = None);
        }

        return;
    }

    // The first knock: the window in the corner. Nothing is recorded as shown — the letter is due until it
    // is actually on the screen.
    announce(owner, letter);
    ANNOUNCED.with_borrow_mut(|slot| *slot = Some(letter));
}

/// Opens the letter the knock announced — FR-101, «Открыть письмо» of the window in the corner.
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
/// письма разом. Письма «Что нового» с её стуком больше нет (решение 208.94 п. 4, T-119-8).
pub const fn toast_strings(letter: Letter) -> Option<(u16, u16)> {
    use crate::settings::{IDS_TOAST_NEWS, IDS_TOAST_THANKS, IDS_TOAST_TITLE};

    match letter {
        // ⛔ «Обновление» не стучит — задача T-119-9, решение 208.99 п. 1: уведомления Windows о нём
        // нет, письмо открывает только сам человек (пункт меню, строка «Последних писем»).
        Letter::Welcome | Letter::Update => None,
        Letter::Thanks => Some((IDS_TOAST_TITLE, IDS_TOAST_THANKS)),
        Letter::News(_) => Some((IDS_TOAST_TITLE, IDS_TOAST_NEWS)),
    }
}

/// What the window in the corner says about a letter that has come, in `language` — the knock of
/// FR-101 on the shell (T-120-14, T-120-16): «LANG SWITCHER · ПИСЬМО», the heading of the letter,
/// the words of [`toast_strings`], and one button, «Открыть письмо». `None` for the
/// letters that do not knock.
///
/// One button and the ✕, not two: the ✕ does what an unnoticed knock always did — nothing, and
/// the letter opens by itself at the next quiet moment (FR-101). A second button «позже» would
/// promise a delay the schedule does not give.
pub fn notice_content_in(language: Language, letter: Letter) -> Option<crate::corner::Content> {
    use crate::settings::{
        IDS_CORNER_LETTER, IDS_LETTERS_OPEN, IDS_NEWS_LETTER_TITLE, IDS_THANKS_TITLE,
    };

    // The words of the old balloon; its title is not shown — the header of the window in the corner
    // is one short word, as the hint's and «Обновлено»'s are (C6 of T-120-16).
    let (_, words) = toast_strings(letter)?;
    let title = match letter {
        Letter::Thanks => IDS_THANKS_TITLE,
        _ => IDS_NEWS_LETTER_TITLE,
    };
    let text = |id| settings::text_in(language, id);

    Some(crate::corner::Content {
        about: text(IDS_CORNER_LETTER),
        title: text(title),
        text: text(words),
        link: None,
        main: (text(IDS_LETTERS_OPEN), crate::corner::Action::Close),
        second: None,
        picture: false,
        check: None,
    })
}

thread_local! {
    /// The program's window the last knock was made for — the owner of the letter «Открыть
    /// письмо» opens; the window in the corner tells its end through a plain `fn`.
    static NOTICE_OWNER: std::cell::Cell<Option<HWND>> = const { std::cell::Cell::new(None) };
}

/// Knocks once with the window in the corner — FR-101, every letter but «Привет» and
/// «Обновление»; with the sound of a notification, as the balloon of the icon had. A window in
/// the corner already up (the hint of the flag, «Обновлено») takes the knock's place: the letter
/// opens by itself at the next quiet moment, as after a knock nobody noticed.
fn announce(owner: HWND, letter: Letter) {
    // «Привет» is never announced: FR-101 shows it at once, and this function is not reached
    // for it. NFR-13: a letter with no strings knocks with nothing rather than with blanks —
    // «Обновление» too, since task T-119-9 (it is not in the schedule at all).
    let Some(content) = notice_content_in(settings::ui_language(), letter) else {
        return;
    };

    NOTICE_OWNER.with(|slot| slot.set(Some(owner)));
    crate::corner::show(content, true, notice_ended);
}

/// The knock ended: «Открыть письмо» opens the letter, the person asked for it; the ✕ and `Esc`
/// leave it to the schedule.
fn notice_ended(end: crate::corner::End) {
    if end.by != crate::corner::EndBy::Main {
        return;
    }
    if let Some(owner) = NOTICE_OWNER.with(std::cell::Cell::get) {
        open_announced(owner);
    }
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
                        fill_window(hwnd, state);
                        layout_window(hwnd, state);
                    })
                };

                widgets::repaint::whole(hwnd);
            }

            settings::LanguageSwitch::Reopen => {
                close_window(hwnd);
            }
        }
    }

    // The windows of the shell are built again in the new language, wherever it moved: their
    // words, their direction and their mirror are the description's, and a description is built
    // from nothing. Shown without the activation — the person is at the settings window.
    let shell: Vec<ShellWindow> = SHELL.with_borrow(|open| open.clone());

    for window in shell {
        if settings::language_switch(window.language, settings::ui_language())
            == settings::LanguageSwitch::Unchanged
        {
            continue;
        }

        close_window(window.hwnd);

        match (window.kind, window.letter) {
            (Kind::Letter, Some(letter)) => {
                // Its entry again — the window is being built from nothing, and `None` here
                // would reopen «Новость» as an empty frame.
                let item = item_of(letter);

                show_letter(owner, letter, item.as_ref(), false);
            }
            (Kind::List, _) => {
                open_list_with(owner, settings::ui_language(), list_entries_now(), false);
            }
            (Kind::Author, _) => {
                open_author_with(owner, &version_string(), false);
            }
            _ => {}
        }
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

        // ⭐ **Решение 181.28: an idea carries what the person wrote and the version in the heading,
        // and nothing of the machine.** It goes to an open discussion (решение 181.1), and its road
        // has no step «Что приложить?» — the four boxes it never showed must not fill it. A trouble
        // goes by letter (решение 181.6) and carries what its boxes say.
        let attach = if draft.trouble.is_idea() {
            Attach {
                machine: false,
                layouts: false,
                settings: false,
                journal: false,
            }
        } else {
            draft.attach
        };

        if attach.machine {
            line(
                &mut out,
                &text(IDS_WIZARD_ATTACH_MACHINE),
                &machine_line(facts),
            );
        }

        if attach.layouts {
            line(&mut out, &text(IDS_WIZARD_ATTACH_LAYOUTS), &facts.layouts);
        }

        if attach.settings {
            line(&mut out, &text(IDS_WIZARD_ATTACH_SETTINGS), &facts.settings);
        }

        if attach.journal && !facts.journal.trim().is_empty() {
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

    /// **The text of the appeal with the line of the contact** — задача T-107-3, решение 199.10:
    /// «Для связи: …» at the end, after an empty line, and only when there is a contact — so the
    /// person sees in the text what goes with it (199.1).
    ///
    /// `was` is the contact whose line the text may carry now: when the text ends with that line,
    /// it is taken off first, so that typing in the field moves one line and does not grow a tail
    /// of them. A text the person changed at its very end keeps what they wrote, and the line goes
    /// after it. A pure function, as [`build`] is.
    pub fn with_contact(text: &str, was: &str, now: &str) -> String {
        let mut out = text
            .strip_suffix(contact_line(was).as_str())
            .unwrap_or(text)
            .to_owned();

        out.push_str(&contact_line(now));
        out
    }

    /// The line [`with_contact`] writes for one contact, with the empty line before it — and
    /// nothing at all for an empty one. The contact as it goes ([`super::appeal::contact_of`]).
    pub fn contact_line(contact: &str) -> String {
        let contact = super::appeal::contact_of(contact);

        if contact.is_empty() {
            return String::new();
        }

        let mut out = String::from("\n");

        line(
            &mut out,
            &settings::text(settings::IDS_WIZARD_CONTACT_LINE),
            &contact,
        );

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
        let hotkey = settings::effective_hotkey_name(&config.hotkey);
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
    open_window(owner, wizard_state(), true);
}

/// The state the wizard opens with — the facts of the machine and an empty draft.
///
/// Split out of [`open_wizard`] by задача T-103-2 so that the measuring door of the tests opens the
/// very same wizard ([`testing::open_wizard_hidden`]) and not a copy of it.
fn wizard_state() -> WindowState {
    let mut state = fresh_state(Kind::Wizard);

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
        contact: String::new(),
        post: Post::Ready,
        nonce: None,
        done: false,
    }));

    state
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
        // Задача T-107-3: one step and two shapes of it — the contact while the appeal is ready
        // or on its way, the letter once the site did not take it. All of them are slots of this
        // step; `layout_wizard` hides what [`preview_controls`] does not name.
        Step::Preview => &[
            IDC_WZ_PREVIEW,
            IDC_WZ_CONTACT_LABEL,
            IDC_WZ_CONTACT,
            IDC_WZ_ADDRESS,
            IDC_WZ_COPY,
            IDC_WZ_COPY_ADDRESS,
            IDC_WZ_SAVE,
            IDC_WZ_STATUS,
        ],
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
const WIZARD_SLOTS_VALUES: [i32; 12] = [
    IDC_WZ_ATTACH_V1 + 1,
    IDC_WZ_ATTACH_V1 + 2,
    IDC_WZ_ATTACH_V1 + 3,
    IDC_WZ_STATUS,
    IDC_WZ_COPY,
    IDC_WZ_SAVE,
    // The address of the letter and its button — задача T-103-7.
    IDC_WZ_ADDRESS,
    IDC_WZ_COPY_ADDRESS,
    // The contact — задача T-107-3.
    IDC_WZ_CONTACT_LABEL,
    IDC_WZ_CONTACT,
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
        IDS_WIZARD_ATTACH_SETTINGS, IDS_WIZARD_BACK, IDS_WIZARD_CANCEL, IDS_WIZARD_CAPTION,
        IDS_WIZARD_CAPTURE, IDS_WIZARD_CAPTURE_COUNT, IDS_WIZARD_CAPTURE_NOTE,
        IDS_WIZARD_CARD_IDEA, IDS_WIZARD_CARD_IDEA_SUB, IDS_WIZARD_CARD_NOTHING,
        IDS_WIZARD_CARD_NOTHING_SUB, IDS_WIZARD_CARD_WRONG, IDS_WIZARD_CARD_WRONG_SUB,
        IDS_WIZARD_DONE, IDS_WIZARD_EXPECTED, IDS_WIZARD_FIELD_NORMAL, IDS_WIZARD_FIELD_PASSWORD,
        IDS_WIZARD_FIELD_PASSWORD_SUB, IDS_WIZARD_FIELD_UNKNOWN, IDS_WIZARD_GOT,
        IDS_WIZARD_IDEA_HELPS, IDS_WIZARD_IDEA_WHAT, IDS_WIZARD_PROGRAM, IDS_WIZARD_REPEAT,
        IDS_WIZARD_REPEAT_ALWAYS, IDS_WIZARD_REPEAT_ONCE, IDS_WIZARD_REPEAT_SOMETIMES,
        IDS_WIZARD_SAVE, IDS_WIZARD_STEP, IDS_WIZARD_TYPED_IN, format_text, text,
    };
    // The words of the letter to the box of the appeals — задача T-103-7, and since T-107-3 the
    // way back of every appeal the site did not take (решение 199.3).
    use crate::settings::{IDS_WIZARD_COPY_ADDRESS, IDS_WIZARD_COPY_TEXT, IDS_WIZARD_MAIL_ADDRESS};
    // The contact and «Готово» — задача T-107-3, решения 199.10 и 199.11.
    use crate::settings::{IDS_CHANNEL_OPEN, IDS_WIZARD_CONTACT, IDS_WIZARD_SENT_TEXT};

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
    let words = step_words(step, wizard.post);

    let entries = wizard.facts.journal_entries.to_string();

    for (control, caption) in [
        (
            IDC_WZ_STEP,
            // ⚠ **The last step is not numbered.** «Готово» is where the wizard ends, not the
            // sixth of five — the mock-up puts that word in the place of «Шаг N из M».
            if step == Step::Done {
                text(IDS_WIZARD_DONE)
            } else {
                format_text(
                    IDS_WIZARD_STEP,
                    &[&(wizard.step + 1).to_string(), &road.len().to_string()],
                )
            },
        ),
        // Решение 199.10: the owner's words, one paragraph for a trouble and for an idea alike.
        (IDC_WZ_THANKS, text(IDS_WIZARD_SENT_TEXT)),
        (IDC_WZ_CHANNEL, text(IDS_CHANNEL_OPEN)),
        (IDC_WZ_TITLE, text(words.title)),
        (IDC_WZ_NOTE, words.note.map(text).unwrap_or_default()),
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
        (IDC_WZ_COPY, text(IDS_WIZARD_COPY_TEXT)),
        (
            IDC_WZ_ADDRESS,
            format_text(IDS_WIZARD_MAIL_ADDRESS, &[links::MAIL_ADDRESS]),
        ),
        (IDC_WZ_COPY_ADDRESS, text(IDS_WIZARD_COPY_ADDRESS)),
        (IDC_WZ_CONTACT_LABEL, text(IDS_WIZARD_CONTACT)),
        (IDC_WZ_SAVE, text(IDS_WIZARD_SAVE)),
        (IDC_WZ_STATUS, wizard.status.clone()),
        (IDC_WZ_CANCEL, text(IDS_WIZARD_CANCEL)),
        (IDC_WZ_BACK, text(IDS_WIZARD_BACK)),
        // The faces of one button — «Далее»; «Отправить», «Отправляется…» and «Повторить» on the
        // last step (задача T-107-3); «Закрыть» on the thanks, where it is the only button left
        // (задача Т-33-3). [`step_words`] says which.
        (IDC_WZ_NEXT, text(words.next)),
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
        (IDC_WZ_CONTACT, wizard.contact.as_str()),
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

        // Правило 118.1, задача T-53a-1 — ЗДЕСЬ, внутри ветви перестроения, и это ровно то
        // «после заполнения и заново при всяком изменении числа строк», которого правило
        // требует: ветвь срабатывает тогда и только тогда, когда число строк разошлось с
        // записанным. Высота считается слоем `widgets::combo`, своей арифметики у мастера нет.
        let _taken = widgets::combo::set_dropped_height(hwnd, IDC_WZ_LAYOUT);
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
            let dpi = theme::drawing_dpi(dc);
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

            // ⛔ **Здесь стояла `metrics.y(12)`, и в ней был дефект Э53а** — задача T-53a-1,
            // правило 118.1. У выпадающего комбобокса высота ОКНА есть высота РАСКРЫТОГО
            // состояния вместе с полем, а не высота ряда вёрстки. Пока 12 единиц диалога были
            // меньше закрытой части, Windows запрос отбрасывала и держала высоту шаблона —
            // и всё работало; Э51 подняла шрифт мастера 9 → 10 pt, `base_y` 15 → 17, и те же
            // 12 единиц стали 25 пикселями, ровно закрытой частью: запрос приняли, на строки
            // не осталось ничего. Замер: окно списка 203 × **2** px, ноль строк.
            //
            // Поэтому вёрстка назначает место и ширину, а ВЫСОТУ — общий слой, вызовом сразу
            // следом: `fill_wizard` идёт до `layout_wizard`, и без этого вызова разметка
            // затирала бы высоту, поставленную при заполнении списка.
            //
            // NFR-13: отказ слоя разобран — комбобокс остаётся при высоте ряда, то есть при
            // прежнем поведении, а не при нулевой высоте.
            place(hwnd, IDC_WZ_LAYOUT, pad, y, combo, step_height);
            widgets::combo::set_dropped_height(hwnd, IDC_WZ_LAYOUT);

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
            use crate::settings::{
                IDS_WIZARD_ADDRESS_COPIED, IDS_WIZARD_FAILED, IDS_WIZARD_TEXT_COPIED, format_text,
                text,
            };

            // Задача T-107-3, решения 199.3 и 199.11: while the appeal is ready or on its way — the
            // contact under the text and the one button «Сохранить в папку журнала»; once the
            // site did not take it — the letter: the line of the address and three buttons.
            // [`preview_controls`] names the shape; what it does not name is hidden here.
            let post = state
                .wizard
                .as_deref()
                .map_or(Post::Ready, |wizard| wizard.post);
            let shown = preview_controls(post);
            let mail = shown.contains(&IDC_WZ_ADDRESS);
            let row: &[i32] = if mail {
                &[IDC_WZ_COPY, IDC_WZ_COPY_ADDRESS, IDC_WZ_SAVE]
            } else {
                &[IDC_WZ_SAVE]
            };

            for &control in controls_of(Step::Preview) {
                if !shown.contains(&control) {
                    hide(hwnd, control);
                }
            }

            let widths: Vec<i32> = row
                .iter()
                .map(|&control| {
                    // SAFETY: as above.
                    unsafe {
                        button_width(dc, faces.text, metrics, &settings::get_text(hwnd, control))
                    }
                })
                .collect();

            // ⚠ **The buttons wrap when they do not fit side by side**, and that is not a
            // precaution: in Greek «Αποθήκευση στον φάκελο του ημερολογίου» took the second
            // button 86 px past the right edge of the window (`влезание-В.log`, control 1348
            // «вне клиента»). Every row costs the preview one button of height and nothing else.
            let at = wrap_row(&widths, metrics.x(air::BUTTON_GAP), width);
            let rows = at
                .last()
                .map_or(1, |&(_, line)| i32::try_from(line).unwrap_or(0) + 1);

            let address = if mail {
                // SAFETY: as above.
                unsafe {
                    measure(
                        dc,
                        faces.text,
                        width,
                        &settings::get_text(hwnd, IDC_WZ_ADDRESS),
                        None,
                    )
                }
            } else {
                0
            };

            // The contact: its label as tall as its words need — two lines in a long language are
            // allowed (201.11а) — and the field one button high, as the fields of «Что вы делали?».
            let contact_label = if mail {
                0
            } else {
                // SAFETY: as above.
                unsafe {
                    measure(
                        dc,
                        faces.text,
                        width,
                        &settings::get_text(hwnd, IDC_WZ_CONTACT_LABEL),
                        None,
                    )
                }
                .max(step_height)
            };

            // The line under the buttons, as tall as the longest thing it can say: «Сохранено: …»
            // or «Не получилось…» in both shapes, and in the letter's two more that carry the
            // address.
            let mut said = vec![text(IDS_WIZARD_FAILED)];

            if mail {
                said.push(format_text(IDS_WIZARD_TEXT_COPIED, &[links::MAIL_ADDRESS]));
                said.push(format_text(
                    IDS_WIZARD_ADDRESS_COPIED,
                    &[links::MAIL_ADDRESS],
                ));
            }

            let status = said
                .iter()
                .map(|said| {
                    // SAFETY: as above.
                    unsafe { measure(dc, faces.text, width, said, None) }
                })
                .fold(step_height, i32::max);

            let between = if mail {
                address + tight
            } else {
                contact_label + tight + button + tight
            };
            let bottom = bottom_of_body(client, metrics);
            let room = (bottom - y - between - button * rows - status - tight * (rows + 1))
                .max(metrics.y(40));

            place(hwnd, IDC_WZ_PREVIEW, pad, y, width, room);
            inset_area(hwnd, metrics, IDC_WZ_PREVIEW);
            y += room + tight;

            if mail {
                place(hwnd, IDC_WZ_ADDRESS, pad, y, width, address);
                y += address + tight;
            } else {
                place(hwnd, IDC_WZ_CONTACT_LABEL, pad, y, width, contact_label);
                y += contact_label + tight;
                place_field(hwnd, metrics, IDC_WZ_CONTACT, pad, y, width, button);
                y += button + tight;
            }

            for (&control, (&wide, &(x, line))) in row.iter().zip(widths.iter().zip(at.iter())) {
                let line = i32::try_from(line).unwrap_or(0);

                place(
                    hwnd,
                    control,
                    pad + x,
                    y + line * (button + tight),
                    wide.min(width),
                    button,
                );
            }

            y += rows * (button + tight);
            place(hwnd, IDC_WZ_STATUS, pad, y, width, status);
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

    // Задача T-107-3 (макет, «Отправляется»): while the appeal is on its way every button and
    // field of the window is grey — it cannot be sent twice, nor changed under the request, nor
    // the step left. Stated on every layout, so the answer of the site gives them back by the
    // same line. The cross of the title bar and Esc still close the window; what the thread
    // answers then finds no window and is dropped ([`wizard_answer`]).
    let sending = matches!(wizard.post, Post::Sending(_));

    for control in [
        IDC_WZ_PREVIEW,
        IDC_WZ_CONTACT,
        IDC_WZ_SAVE,
        IDC_WZ_CANCEL,
        IDC_WZ_BACK,
        IDC_WZ_NEXT,
    ] {
        enable(hwnd, control, !sending);
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
    //
    // ⭐ **Задача T-50-2, решение 115.2.** «Формула одна на все окна» было правдой, а результат
    // всё равно расходился: формула несла высоту шрифта ОКНА, а мастер несёт 9 pt против 10 pt
    // окна настроек — 24 px против 26. Теперь спрашивается бо́льшая из макетной мерки и прежней
    // шрифтовой, и это окно поднимается до 26; окно настроек — эталон и не двигается.
    let height = widgets::combo::list_row_height(hwnd);

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
            theme::scaled(6, theme::drawing_dpi(dc)),
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
    let gap = theme::scaled(4, theme::drawing_dpi(dc));
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
    let dpi = theme::drawing_dpi(dc);

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
    let dpi = theme::drawing_dpi(dc);
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
/// Since decision 132.3 (task T-71-5) a door into [`draw_glyph`], the body the feed switch of «От
/// автора» goes through as well: this one reads «checked?» out of the wizard's draft and lays the
/// element on the ground of the window.
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
    let Some(wizard) = state.wizard.as_deref() else {
        return 0;
    };

    let glyph = Glyph {
        kind,
        checked: wizard_is_checked(wizard, control),
        disabled,
        ground: Ground::Window,
    };

    // SAFETY: see the caller — the values of the message, handed on unchanged.
    unsafe { draw_glyph(dc, rect, state, glyph, label) }
}

/// One glyph element of these windows — the four answers the two doors of [`draw_glyph`] differ
/// by (decision 132.3).
#[derive(Clone, Copy)]
struct Glyph {
    /// A check box or a radio button.
    kind: theme::GlyphKind,
    /// The answer to «checked?», read by the door out of its own record.
    checked: bool,
    /// `ODS_DISABLED` of the message.
    disabled: bool,
    /// What the element lies on.
    ground: Ground,
}

/// What a glyph element lies on — the brush its whole rectangle is grounded with.
#[derive(Clone, Copy)]
enum Ground {
    /// The window itself — the radios and the check boxes of the wizard.
    Window,
}

/// Draws one glyph element — the glyph and the words beside it, **in one frame**.
///
/// ⭐ **The one body of the glyphs of these windows since decision 132.3 (task T-71-5)**: the
/// wizard's radios and check boxes ([`draw_wizard_glyph`]) and the feed switch of «От автора»
/// ([`draw_switch`]). Everything below the four answers of [`Glyph`] was the same table
/// (`theme::glyph_color_roles`), the same cell (`widgets::glyph::cell`) and the same caption in
/// both copies; what the switch's copy lacked was the buffer below, and the owner saw that as a
/// blink.
///
/// ⚠ Not the body of the settings window (`settings::draw_glyph_element`): that one differs by the
/// rule of the ground, the caption, the focus frame and the pen of the circle, and merging them
/// would move pixels of an accepted window — decision 107.2, the documentation of
/// `widgets::glyph`.
///
/// # Safety
///
/// As [`draw_progress`].
unsafe fn draw_glyph(dc: HDC, rect: RECT, state: &WindowState, glyph: Glyph, label: &str) -> isize {
    let (Some(brushes), Some(faces)) = (state.brushes.as_ref(), state.fonts.as_ref()) else {
        return 0;
    };

    let palette = state.palette;
    let dpi = theme::drawing_dpi(dc);
    let colors = theme::glyph_color_roles(glyph.kind, glyph.checked, glyph.disabled);

    // The ground of the whole rectangle. It is also what grounds the buffer below: the corners of
    // the rounded cell are smoothed into what the buffer holds under them.
    let ground = match glyph.ground {
        Ground::Window => brushes.window_bg(),
    };

    // ⚠ **Один кадр вместо череды — задача Т-33-5.** То же, что у [`draw_card`], и по той же
    // измеренной причине: заливка, глиф и подпись писались прямо в DC сообщения, и радиокнопка
    // **моргала** под указателем — чернила 738 → **0** → 738 в 13 кадрах из 120 и 474 → **0** →
    // 474 в 25 из 120 (`scratchpad-Э33\моргание-до.log`). Кнопки этого окна не моргали никогда
    // именно потому, что `settings::paint_push_button` носит эту поверхность с задачи T-15-1.
    // ⚠ Задача T-71-5: выключатель ленты этой поверхности не носил и моргал по той же причине.
    //
    // SAFETY: как в [`draw_card`].
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // SAFETY: `target` is the buffer of this frame or the DC of the message, and the brush
    // belongs to this window's state.
    unsafe { FillRect(target, &rect, ground) };

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

    match glyph.kind {
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
                ground,
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
    use windows::Win32::UI::WindowsAndMessaging::{CB_GETCURSEL, CBN_SELCHANGE, EN_CHANGE};

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
                        // Задача T-107-3: a step back from «Не удалось отправить» is a step back
                        // into the appeal; coming forward again, it is ready to be sent. The
                        // nonce stays — it is still the same appeal (199.7 (б)).
                        wizard.post = Post::Ready;
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
        // «Скопировать текст» — the letter the site did not take, задача T-107-3 (решение 199.3):
        // the clipboard and nothing else, for every appeal. Until 1.2.0 the same button of an
        // idea opened the group or GitHub Issues as well; решение 199.3 took that way out.
        IDC_WZ_COPY => {
            wizard_copy_text(hwnd, links::MAIL_ADDRESS);
            return true;
        }
        // The contact, as it is typed — задача T-107-3: its line in the text follows it at once,
        // so what the person sees in the field above is what goes (решение 199.1).
        IDC_WZ_CONTACT if u32::from(notification) == EN_CHANGE => {
            wizard_contact_changed(hwnd);
            return true;
        }
        IDC_WZ_COPY_ADDRESS => {
            wizard_copy_address(hwnd);
            return true;
        }
        IDC_WZ_SAVE => {
            wizard_save(hwnd);
            return true;
        }
        // «Открыть канал» on the thanks — задача Т-33-3. Dead while the address is a
        // placeholder, and the button is disabled to say so (П7).
        IDC_WZ_CHANNEL => {
            open_link(links::CHANNEL_URL);
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
    use crate::settings::{IDS_WIZARD_DONE, IDS_WIZARD_STEP, format_text, text};

    let step = wizard.current();
    let road = wizard.road();
    // Задача T-107-3: the same table `fill_wizard` asks — one, and not a copy of it.
    let words = step_words(step, wizard.post);

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
        (IDC_WZ_TITLE, text(words.title)),
        (IDC_WZ_NOTE, words.note.map(text).unwrap_or_default()),
        (IDC_WZ_NEXT, text(words.next)),
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
    let contact = settings::get_text(hwnd, IDC_WZ_CONTACT);

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
            // The contact is the field's from the moment the last step is first on the screen —
            // before that the field is empty and holds nothing to take (задача T-107-3).
            if wizard.current() == Step::Preview {
                wizard.contact = contact;
            }

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

/// «Далее»; «Отправить» and «Повторить» on the last step of the road; «Закрыть» on the thanks —
/// FR-104.
///
/// ⚠ **The thanks is a step and not a second window** (задача Т-33-3, решение 103.3), and since
/// задача T-107-3 the step it comes to is decided by the **site**: the last step's button sends
/// the appeal ([`wizard_send`]), and the thanks is shown when the site answered 201
/// ([`wizard_answer`]) — never on the press itself.
fn wizard_forward(hwnd: HWND) {
    wizard_harvest(hwnd);

    /// What the press does, decided under the borrow and done after it.
    enum Next {
        Close,
        Send,
        Refresh,
    }

    // SAFETY: the state pointer is live for the length of the window.
    let next = unsafe {
        with_state(hwnd, |state| {
            let Some(wizard) = state.wizard.as_deref_mut() else {
                return Next::Refresh;
            };

            // The thanks is the end of the road: its one button closes the window.
            if wizard.done {
                return Next::Close;
            }

            if wizard.is_last() {
                return Next::Send;
            }

            wizard.step += 1;
            wizard.status.clear();

            // The text of the appeal is built on the way **into** the last step, out of what
            // the person has filled in by then — and from that moment the field owns it. The
            // contact a person typed before going back is written into it again (T-107-3).
            if wizard.current() == Step::Preview {
                let built = report::build(&wizard.draft, &wizard.facts);

                wizard.text = report::with_contact(&built, "", &wizard.contact);
            }

            Next::Refresh
        })
    };

    match next {
        Some(Next::Close) => close_window(hwnd),
        Some(Next::Send) => wizard_send(hwnd),
        Some(Next::Refresh) | None => wizard_refresh(hwnd),
    }
}

/// **The contact changed** — its line in the text of the appeal follows it, задача T-107-3
/// (решение 199.10: «Для связи: …», only when there is a contact).
///
/// The line written for the contact before this keystroke is taken off the end of the text and
/// the line of the new one put there ([`report::with_contact`]); whatever else the person wrote
/// in the text stays as they wrote it. Written into the field only when it changed, as every
/// write of this window is.
fn wizard_contact_changed(hwnd: HWND) {
    let typed = settings::get_text(hwnd, IDC_WZ_CONTACT);
    let preview = settings::get_text(hwnd, IDC_WZ_PREVIEW);

    // SAFETY: the state pointer is live for the length of the window.
    let text = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref_mut()?;

            if wizard.current() != Step::Preview || preview.is_empty() {
                return None;
            }

            let text = report::with_contact(&preview, &wizard.contact, &typed);

            wizard.contact = typed;
            wizard.text.clone_from(&text);

            Some(text)
        })
    }
    .flatten();

    if let Some(text) = text {
        set_text_if_changed(hwnd, IDC_WZ_PREVIEW, &text);
    }
}

/// **«Отправить» and «Повторить»** — the appeal goes to the site, задача T-107-3 (решения 199.1,
/// 199.3, 199.7).
///
/// What goes is the text the field holds — the very text on the screen — with the language of
/// the window, the kind of the first step, the version of this build, the contact and the nonce
/// of this appeal, drawn at the first press and the same at every «Повторить». The window goes
/// grey under «Отправляется…» ([`Post::Sending`]) and the request runs on a thread of its own
/// ([`start_appeal`]); its answer comes back as [`WM_APP_APPEAL`].
///
/// Nothing is sent for a text that is empty or only spaces (the site would answer 400, 199.9),
/// and nothing a second time while one request is on its way. A nonce the system would not draw,
/// or a thread that would not start, is a «Не удалось отправить» at once — the letter is there.
fn wizard_send(hwnd: HWND) {
    /// The ticket of the next request — so an answer is taken only by the press it answers.
    static TICKETS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

    // SAFETY: the state pointer is live for the length of the window.
    let body = unsafe {
        with_state(hwnd, |state| {
            let language = state.language;
            let wizard = state.wizard.as_deref_mut()?;

            if matches!(wizard.post, Post::Sending(_)) || wizard.text.trim().is_empty() {
                return None;
            }

            let ticket = TICKETS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let Some(nonce) = appeal::keep_or_draw(&mut wizard.nonce, appeal::nonce) else {
                wizard.post = Post::Failed;
                return Some(None);
            };

            let body = appeal::body(&appeal::Appeal {
                kind: appeal::kind_of(wizard.draft.trouble),
                lang: language.tag(),
                version: env!("CARGO_PKG_VERSION"),
                text: &wizard.text,
                contact: &wizard.contact,
                nonce,
            });

            wizard.post = Post::Sending(ticket);
            wizard.status.clear();

            Some(Some((ticket, body)))
        })
    }
    .flatten();

    let pressed = body.is_some();

    if let Some(Some((ticket, body))) = body
        && !start_appeal(hwnd, ticket, body)
    {
        // SAFETY: as above.
        unsafe {
            with_state(hwnd, |state| {
                if let Some(wizard) = state.wizard.as_deref_mut() {
                    wizard.post = Post::Failed;
                }
            })
        };
    }

    if pressed {
        wizard_refresh(hwnd);
    }
}

/// The nudge the thread of an appeal posts to its wizard when the site has answered — задача
/// T-107-3.
///
/// `WM_APP + 21`, the first number free above [`crate::app::WM_APP_SOUND_REFUSED`] (`+ 20`); the
/// whole map is written down in the doc comment of every constant of the family. Posted to the
/// **wizard** and not to the UI window: the answer is for the window that asked, and a wizard
/// that was closed meanwhile is simply not there to be told.
///
/// SEC-05: the message carries nothing. What the site said is in [`APPEAL_ANSWERS`], put there by
/// this program's own thread under the ticket of the press, so a forged message finds no answer
/// with the ticket of this window and changes nothing.
pub const WM_APP_APPEAL: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 21;

/// What the threads of the appeals left: the ticket of the press and whether the site took it.
/// A list and not one slot, so an answer to a wizard closed and opened again cannot cover the
/// answer the new one waits for.
static APPEAL_ANSWERS: std::sync::Mutex<Vec<(u32, bool)>> = std::sync::Mutex::new(Vec::new());

/// **Starts the request of one appeal** on a thread that lives exactly that request — the shape
/// of [`start_feed_read`]: no user interface, no input, no hook. `false` when the thread would not
/// start, and then nothing was sent.
fn start_appeal(hwnd: HWND, ticket: u32, body: String) -> bool {
    // `HWND` is not `Send`; what crosses is its bits, which `PostMessageW` takes from any thread.
    let target = hwnd.0 as isize;

    let started = std::thread::Builder::new()
        .name("langsw-appeal".to_owned())
        .spawn(move || {
            let sent = appeal::send(&body);

            if let Ok(mut answers) = APPEAL_ANSWERS.lock() {
                answers.push((ticket, sent));
            }

            // SAFETY: `target` is the bits of the wizard that asked; if it was closed meanwhile,
            // the post fails and is dropped. The message carries two zeros and no pointer.
            let _ = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(HWND(target as *mut std::ffi::c_void)),
                    WM_APP_APPEAL,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
        });

    if started.is_err() {
        crate::app::report_non_critical("thread body", &WinError::from_thread());
        return false;
    }

    true
}

/// **Takes the answer of the site** — the far end of [`WM_APP_APPEAL`], on the UI thread.
///
/// The answer with the ticket this wizard is waiting for: 201 — the thanks, «Обращение
/// отправлено» ([`Step::Done`]); anything else — «Не удалось отправить», the text where it was,
/// «Повторить» and the letter. Answers of other tickets belong to windows that are gone, and are
/// dropped with it.
fn wizard_answer(hwnd: HWND) {
    let answers = APPEAL_ANSWERS
        .lock()
        .map(|mut answers| std::mem::take(&mut *answers))
        .unwrap_or_default();

    // SAFETY: the state pointer is live for the length of the window.
    let changed = unsafe {
        with_state(hwnd, |state| {
            let wizard = state.wizard.as_deref_mut()?;

            let Post::Sending(waiting) = wizard.post else {
                return None;
            };

            let (_, sent) = answers.iter().find(|(ticket, _)| *ticket == waiting)?;

            if *sent {
                wizard.post = Post::Ready;
                wizard.done = true;
            } else {
                wizard.post = Post::Failed;
            }

            wizard.status.clear();

            Some(())
        })
    }
    .flatten();

    if changed.is_some() {
        wizard_refresh(hwnd);
    }
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

            wizard.countdown = crate::guard::CAPTURE_SECONDS;
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
/// and none of that is any of this program's business. The body that reads the window holds no
/// call that could read a title, and a sweep of `tests\guard.rs` keeps it so.
///
/// The wizard's own window is skipped: a person who presses the button and does not switch
/// anywhere would otherwise be told that the program they have trouble with is this one.
///
/// ⭐ **The reading is not here since task T-100-2.** It is `guard::capture_window`, the one body
/// the wizard and «Выбрать окно программы» of the settings window share; what stays here is the
/// wizard's own half — which window is its own, and how the two names are written in the field.
/// Решение 172.21: a Store program is named by the program inside its frame, as the settings window
/// names it (`CalculatorApp.exe`, not `ApplicationFrameHost.exe`); every other window, as before.
fn capture_foreground(mine: HWND) -> String {
    let Some(found) = crate::guard::capture_foreground(crate::guard::Asker::Window(mine)) else {
        return String::new();
    };

    let name = found.process.unwrap_or_default();
    let class = found.class;

    match (name.is_empty(), class.is_empty()) {
        (true, true) => String::new(),
        (false, true) => name,
        (true, false) => class,
        (false, false) => format!("{name} · {class}"),
    }
}

// ⛔ **`wizard_copy` is gone — задача T-107-3, решение 199.3.** «Скопировать и открыть группу» /
// «Copy and open GitHub Issues» copied the text and opened the topic of the group or the issues
// on GitHub (решение 181.1). Решение 199.3 took the group and the issues out of the wizard: an
// appeal goes to the site by «Отправить», and the way back is the letter below.

/// «Скопировать текст» — the letter of an appeal the site did not take, задача T-107-3 (решение
/// 199.3; until 1.2.0 — the letter of a trouble, задача T-103-7, решение 181.6).
///
/// **The clipboard and nothing else**: no page is opened, no mail program is started, nothing is
/// sent. The line under the buttons says where to paste it.
fn wizard_copy_text(hwnd: HWND, address: &str) {
    use crate::settings::{IDS_WIZARD_FAILED, IDS_WIZARD_TEXT_COPIED, format_text, text};

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

    let said = if crate::selection::write_unicode_text(hwnd, &appeal).is_ok() {
        format_text(IDS_WIZARD_TEXT_COPIED, &[address])
    } else {
        text(IDS_WIZARD_FAILED)
    };

    wizard_say(hwnd, said);
}

/// «Скопировать адрес» — the author's email to the clipboard, задача T-103-7 (решение 181.6).
///
/// The address stands on the screen as well; the button is there because an owner-drawn line
/// cannot be selected, and a typed address is an address with a typo in it.
fn wizard_copy_address(hwnd: HWND) {
    use crate::settings::{IDS_WIZARD_ADDRESS_COPIED, IDS_WIZARD_FAILED, format_text, text};

    let said = if crate::selection::write_unicode_text(hwnd, links::MAIL_ADDRESS).is_ok() {
        format_text(IDS_WIZARD_ADDRESS_COPIED, &[links::MAIL_ADDRESS])
    } else {
        text(IDS_WIZARD_FAILED)
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
    crate::sandbox::guard_path(&path);

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
    // ⭐ **Finding С26, task T-41-4 — this is the door the finding warned about.** It opens the
    // *same* journal folder as the button «Открыть папку журнала» of the settings dialog, by
    // different code: «вторая дверь через полгода» had already happened by the time the repair
    // arrived. Both now go through [`settings::open_in_the_shell`].
    settings::open_in_the_shell(&folder.display().to_string());
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

/// Where each button of a row stands when the row wraps — its offset from the left edge of the
/// row and the line it is on, for buttons `widths` wide and `gap` apart in a row `width` wide.
///
/// A button that is not the first on its line and would run past the edge starts the next line;
/// nothing is narrowed here. Two buttons answer exactly what the last step of the wizard answered
/// before задача T-103-7 gave it a third: side by side when the two and the gap fit, else two lines.
fn wrap_row(widths: &[i32], gap: i32, width: i32) -> Vec<(i32, usize)> {
    let mut at = Vec::with_capacity(widths.len());
    let (mut x, mut line) = (0, 0);

    for &wide in widths {
        if x > 0 && x + wide > width {
            x = 0;
            line += 1;
        }

        at.push((x, line));
        x += wide + gap;
    }

    at
}

// =========================================================================================
// 12. The author's feed — FR-102, SEC-03, task Т-32-6
// =========================================================================================

/// **The one network operation this program has**, and everything that decides what to do with
/// what it brings back.
///
/// # What is in here and what is deliberately not
///
/// In: one `GET` over HTTPS from a fixed list of addresses, no more often than once a day
/// (решение 189.2); the ECDSA P-256 signature of the answer, checked by the system's own cryptography; the
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
/// # ⚠ And the one thing the program sends — the appeal, since 1.2.0
///
/// Задача T-107-2, решения 199.1 и 199.7: an appeal of the wizard goes to the site by one `POST`
/// ([`post`]) — **only when the person presses «Отправить»**, and exactly the text they saw. It
/// runs on the same body as the read ([`exchange`]), so «which code can reach the network» is
/// still this module and nothing else; what it sends is built by [`super::appeal::body`], where
/// the rule of what goes and what never does is written down. The read of the feed is untouched by
/// it: the same verb, the same headers (none), the same timeouts, the same redirections.
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
        /// The `[download]` table — where the installer of the version the `update` entry
        /// names lies, its SHA-256 and its size (TZ E104 §0 п. 16). **Only the updater reads
        /// it** ([`crate::update`]); the program shows letters and nothing else of it.
        pub download: Table<Download>,
        /// The `[stop]` table — the versions no updater may install, and the word that the
        /// working key is revoked for good. Honoured only in a feed signed with the **reserve**
        /// key (TZ E104 §0 п. 16), and that is the updater's rule, not the parser's.
        pub stop: Table<Stop>,
    }

    /// A top-level table of the feed that one reader needs and another does not — what was
    /// there, read whole, or that it was there and could not be read.
    ///
    /// # Why «malformed» is its own answer and not a refused file
    ///
    /// The tables `[download]` and `[stop]` arrived with stage Э104 **under `schema = 1`** (§0
    /// п. 16): every installed version since `e88` reads the file, skips a top-level table it
    /// does not know (`RawFeed` has no `deny_unknown_fields` — measured on the code of `e103`,
    /// посылка П3, 189.30) and goes on showing letters. A table this build cannot read must
    /// cost **this table** and not the letters: a mistake in a line about the installer must
    /// not silence the news of the same file — and the updater, for which the table is the
    /// whole point, does nothing about a table it cannot read.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub enum Table<T> {
        /// The file has no such table.
        #[default]
        Absent,
        /// The table, read and checked.
        Read(T),
        /// The table is there and is not of the shape §0 п. 16 describes.
        Malformed,
    }

    impl<T> Table<T> {
        /// The table, when it was there and could be read.
        pub fn read(&self) -> Option<&T> {
            match self {
                Self::Read(value) => Some(value),
                Self::Absent | Self::Malformed => None,
            }
        }
    }

    /// The `[download]` table, checked: the shape is the parser's business, whether to believe
    /// it is the updater's ([`crate::update::decide`]).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Download {
        /// The version the installer installs — the same as the `update` entry's, or the
        /// updater installs nothing (§0 п. 16).
        pub version: String,
        /// Where the installer lies — checked against the one host and folder by the updater,
        /// not here.
        pub url: String,
        /// The SHA-256 of the installer, from 64 hexadecimal digits.
        pub sha256: [u8; 32],
        /// The size of the installer, in bytes; never zero.
        pub size: u64,
    }

    /// The `[stop]` table, checked.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Stop {
        /// Versions no updater installs, whatever the rest of the file says.
        pub versions: Vec<String>,
        /// The working key is revoked for good — the updater remembers it and accepts only the
        /// reserve key from then on.
        pub revoke_working: bool,
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
        read_document_by(text, language, |body, signature| {
            verify(body, signature).then_some(0)
        })
        .map(|(feed, _)| feed)
    }

    /// [`read_document`] that also answers **which key** the signature checked out against —
    /// the index into the keys the verifier holds, [`verify_which`] for the product.
    ///
    /// The program does not care which of the two keys signed a letter; the updater does (TZ
    /// E104 §0 п. 15–16): `[stop]` counts only in a file signed with the reserve key, and a
    /// working key the reserve has revoked is refused for good. One body for both, so that the
    /// file the updater believes is the file the program shows.
    pub fn read_document_by(
        text: &str,
        language: &str,
        verify: impl Fn(&[u8], &[u8]) -> Option<usize>,
    ) -> Result<(Feed, usize), Refusal> {
        if text.len() > RESPONSE_CAP {
            return Err(Refusal::TooBig);
        }

        let (signature, body) = split(text)?;

        let Some(key) = verify(body.as_bytes(), &signature) else {
            return Err(Refusal::Unsigned);
        };

        parse_body(body, language).map(|feed| (feed, key))
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

        Ok(Feed {
            update,
            news,
            download: table_of(document.download, download_of),
            stop: table_of(document.stop, stop_of),
        })
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
        verify_which(body, signature, keys).is_some()
    }

    /// [`verify_with`] that answers **which** of the keys the signature checked out against —
    /// its index in `keys`, or `None` (task T-104-4). The same import, hash and verify; the
    /// program asks only whether, the updater asks which (TZ E104 §0 п. 15).
    pub fn verify_which(body: &[u8], signature: &[u8], keys: &[[u8; 64]]) -> Option<usize> {
        use windows::Win32::Security::Cryptography::{
            BCRYPT_ALG_HANDLE, BCRYPT_ECCPUBLIC_BLOB, BCRYPT_ECDSA_P256_ALGORITHM,
            BCRYPT_ECDSA_PUBLIC_P256_MAGIC, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE,
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS, BCryptCloseAlgorithmProvider, BCryptDestroyKey,
            BCryptImportKeyPair, BCryptOpenAlgorithmProvider, BCryptVerifySignature,
        };
        use windows::core::PCWSTR;

        if signature.len() != 64 {
            return None;
        }

        let hash = sha256(body)?;

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
            return None;
        }

        let mut accepted = None;

        for (index, key) in keys.iter().enumerate() {
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
                accepted = Some(index);
                break;
            }
        }

        // SAFETY: closes exactly the provider opened above, once.
        let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };

        accepted
    }

    /// The SHA-256 of a buffer, by the system's own hash — the half of the check that is not
    /// about keys. [`Sha256`] over one piece.
    fn sha256(body: &[u8]) -> Option<[u8; 32]> {
        let mut hasher = Sha256::new()?;

        if !hasher.update(body) {
            return None;
        }

        hasher.finish()
    }

    /// **The system's SHA-256 fed in pieces** — task T-104-5: the installer the updater
    /// downloads is hashed as it arrives, and the signature of the feed is hashed in one piece
    /// ([`sha256`]); one body for both, so there is one way this crate hashes anything.
    ///
    /// The provider and the hash object are this value's and are freed in `Drop`, once, on
    /// every path — a download that fails halfway drops the hasher like any other value.
    pub struct Sha256 {
        algorithm: windows::Win32::Security::Cryptography::BCRYPT_ALG_HANDLE,
        hash: windows::Win32::Security::Cryptography::BCRYPT_HASH_HANDLE,
    }

    impl Sha256 {
        /// A fresh hash — `None`, with the refusal in the journal, when the system will not
        /// give one.
        pub fn new() -> Option<Self> {
            use windows::Win32::Security::Cryptography::{
                BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
                BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptCreateHash,
                BCryptOpenAlgorithmProvider,
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

            if created.is_err() {
                // SAFETY: closes exactly the provider opened above, once.
                let _ = unsafe { BCryptCloseAlgorithmProvider(algorithm, 0) };
                return None;
            }

            Some(Self { algorithm, hash })
        }

        /// Feeds one piece; `false` when the system refused it, and then the sum is worthless.
        pub fn update(&mut self, data: &[u8]) -> bool {
            // SAFETY: `self.hash` is the live object this value owns and `data` a buffer the
            // caller owns for the length of this call; it is read and not written.
            unsafe { windows::Win32::Security::Cryptography::BCryptHashData(self.hash, data, 0) }
                .is_ok()
        }

        /// The sum of everything fed.
        pub fn finish(self) -> Option<[u8; 32]> {
            let mut digest = [0u8; 32];

            // SAFETY: as above; `digest` is a live local the call fills, and its length is what
            // SHA-256 produces. The object is freed by `Drop` right after.
            let ok = unsafe {
                windows::Win32::Security::Cryptography::BCryptFinishHash(self.hash, &mut digest, 0)
            }
            .is_ok();

            ok.then_some(digest)
        }
    }

    impl Drop for Sha256 {
        fn drop(&mut self) {
            use windows::Win32::Security::Cryptography::{
                BCryptCloseAlgorithmProvider, BCryptDestroyHash,
            };

            // SAFETY: frees exactly the hash object and closes exactly the provider this value
            // was made with, once each — `Drop` runs once.
            unsafe {
                let _ = BCryptDestroyHash(self.hash);
                let _ = BCryptCloseAlgorithmProvider(self.algorithm, 0);
            }
        }
    }

    /// The name this program calls itself on the network — FR-102: **the name and the version of
    /// the program, `LangSwitcher/X.Y.Z`, and nothing else** (stage E116, task T-116-11, решение
    /// 208.58).
    ///
    /// Until 1.6.0 it was the name alone: вопрос 101 п. 5 kept the version out, because it tells
    /// the host how many people run which build. The owner decided for 1.6.0 to send it — «В
    /// 1.6.0», the updater «Да, тот же» — and to leave the promise of SEC-03 as it is: nothing about
    /// the machine, no Windows, no build number, no language. The three numbers come from
    /// `Cargo.toml` through `CARGO_PKG_VERSION`, the one source `tools\verify-version.ps1` holds the
    /// version places to, so the header cannot drift from the file version; `concat!` makes it a
    /// constant. The program and the updater are one package and send it through the one
    /// `WinHttpOpen` below ([`get`] and [`fetch`]). The test
    /// `the_user_agent_is_the_name_and_the_version_of_the_program_and_nothing_about_the_machine`
    /// holds the form.
    pub const USER_AGENT: &str = concat!("LangSwitcher/", env!("CARGO_PKG_VERSION"));

    /// How long the program waits for the name to resolve and the connection to open, in
    /// milliseconds — FR-102.
    pub const CONNECT_TIMEOUT_MS: i32 = 10_000;

    /// And how long for the answer — FR-102.
    pub const RECEIVE_TIMEOUT_MS: i32 = 20_000;

    /// The four timeouts of one exchange, in milliseconds, in the order `WinHttpSetTimeouts`
    /// takes them: the name, the connection, the sending, the answer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Timeouts {
        pub resolve: i32,
        pub connect: i32,
        pub send: i32,
        pub receive: i32,
    }

    impl Timeouts {
        /// The longest one exchange can wait in all — the four one after another.
        pub const fn total(self) -> i32 {
            self.resolve + self.connect + self.send + self.receive
        }
    }

    /// FR-102's four, as they have been since the feed was first read: the read of the feed and
    /// the download of the updater wait this long.
    pub const READ_TIMEOUTS: Timeouts = Timeouts {
        resolve: CONNECT_TIMEOUT_MS,
        connect: CONNECT_TIMEOUT_MS,
        send: RECEIVE_TIMEOUT_MS,
        receive: RECEIVE_TIMEOUT_MS,
    };

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
    /// `feed_last_read` is **not** moved, so a broken host is tried again tomorrow — as a good
    /// one is, since решение 189.2 — and not sooner: the tick wrote the day of this attempt
    /// before it started it (`feed_last_try`, решение 148.8).
    ///
    /// # Safety
    ///
    /// None: every unsafe block inside is local and documented. The function is safe to call
    /// from any thread, and the product calls it from a thread of its own that lives exactly
    /// one request (SPEC section 6.1).
    pub fn fetch(url: &str) -> Option<String> {
        fetch_answering(url).0
    }

    /// [`fetch`], and whether an HTTP answer came back at all, of any status — stage E116, task
    /// T-116-12: the mark of the first run is sent once an answer comes back (решение 208.59), and
    /// stays due when none does (no network, a silent host).
    pub fn fetch_answering(url: &str) -> (Option<String>, bool) {
        let answered = std::cell::Cell::new(false);
        let text = fetch_into(url, &answered);

        (text, answered.get())
    }

    /// The body of [`fetch_answering`].
    fn fetch_into(url: &str, answered: &std::cell::Cell<bool>) -> Option<String> {
        let target = target_of(url)?;

        // The line that makes «no request was made» measurable. It is written **before** the
        // session exists, carries no address and no status (SEC-07), and is the positive half
        // of the instrument that checks `[letters] feed = false`: with the feed on, the journal
        // of a due read holds this line; with it off, the journal never holds it.
        crate::diag::record(
            crate::diag::Operation::from_name("feed request"),
            crate::diag::OsCode::NONE,
        );

        let mut body: Vec<u8> = Vec::new();

        get_answering(
            &target,
            RESPONSE_CAP as u64,
            &mut |chunk| {
                body.extend_from_slice(chunk);
                true
            },
            Some(answered),
        )?;

        // The document is UTF-8 by construction — the author's own script writes it. Anything
        // else is not the author's file, and `from_utf8` saying so is the cheapest of the
        // checks that say it.
        String::from_utf8(body).ok()
    }

    /// Where one `GET` goes — task T-104-5: the host, the port, the path and whether the request
    /// is over TLS. The feed of the program and the installer of the updater go through one body,
    /// [`get`], and this is what it is told.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Target {
        /// The name of the host, as it was written in the address.
        pub host: String,
        /// 443 for every address of a Release build.
        pub port: u16,
        /// The path, from its first `/`.
        pub path: String,
        /// `WINHTTP_FLAG_SECURE` — always, outside the doors of the stand.
        pub secure: bool,
    }

    /// The target of an address: `https://host/path` only ([`split_https`]), port 443.
    ///
    /// ⚠ **Under the `testing` feature, and only there**, also `http://` and the loopback of the
    /// machine with a port of its own — the feed and the installer of the stand of TZ E104 §0
    /// п. 22, served inside the guest. The door is not in a Release image: ворота 6 of
    /// `tools\verify-perimeter.ps1` refuse an image that carries the name of the loopback.
    pub fn target_of(url: &str) -> Option<Target> {
        if let Some((host, path)) = split_https(url) {
            return Some(Target {
                host,
                port: 443,
                path,
                secure: true,
            });
        }

        #[cfg(feature = "testing")]
        if let Some(target) = loopback_of(url) {
            return Some(target);
        }

        None
    }

    /// The door of the stand: `http://127.0.0.1:<port>/<path>` — TZ E104 §0 п. 22.
    #[cfg(feature = "testing")]
    fn loopback_of(url: &str) -> Option<Target> {
        let rest = url.strip_prefix("http://127.0.0.1:")?;
        let (port, path) = rest.split_once('/')?;

        Some(Target {
            host: "127.0.0.1".to_owned(),
            port: port.parse().ok()?,
            path: format!("/{path}"),
            secure: false,
        })
    }

    /// **One `GET`, its body handed to `sink` as it arrives** — at most `cap` bytes, and the
    /// number of bytes there were. The one body under [`fetch`] (the feed, into memory) and the
    /// download of the updater (the installer, into a file as it is hashed — task T-104-5).
    ///
    /// The request is the request of FR-102 and nothing more: no parameters but what the address
    /// carries (since 1.6.0 the one `?first=1` of the first run, [`marked`]), no headers of ours
    /// beyond the agent, no cookie. `None` for every refusal — the name, the connection, a
    /// status that is not 200, a body over `cap`, a `sink` that answered `false`.
    pub fn get(target: &Target, cap: u64, sink: &mut dyn FnMut(&[u8]) -> bool) -> Option<u64> {
        get_answering(target, cap, sink, None)
    }

    /// [`get`], and `answered` set to `true` once an HTTP answer has come back, whatever its
    /// status — stage E116, task T-116-12 (решение 208.59), for the mark of the first run.
    fn get_answering(
        target: &Target,
        cap: u64,
        sink: &mut dyn FnMut(&[u8]) -> bool,
        answered: Option<&std::cell::Cell<bool>>,
    ) -> Option<u64> {
        let call = Call {
            verb: "GET",
            headers: None,
            body: &[],
            expect: 200,
            redirects: true,
            timeouts: READ_TIMEOUTS,
            answered,
        };

        exchange(target, &call, Some((cap, sink)))
    }

    /// **One `POST` of `body`, and whether the host answered `201 Created`** — the appeal of the
    /// wizard, решения 199.7 и 199.9, задача T-107-2.
    ///
    /// The request of [`get`] with three differences and no fourth: the verb, one header of ours
    /// (`Content-Type`), and a body whose length goes in `Content-Length` — WinHTTP writes it out
    /// of the total length handed to `WinHttpSendRequest`, so the body is never chunked (the site
    /// answers a chunked one with 411, 199.9). **A redirection is not followed**: a `POST` turned
    /// into a `GET` somewhere else would answer something that is not the site's «принято».
    ///
    /// **Only 201 is a yes.** Every other status, a name that will not resolve, a connection that
    /// will not open, a timeout — `false`, which the wizard shows as «Не удалось отправить». The
    /// body of the answer is **not read**: the program shows no number of the appeal (решение
    /// 199.10), so there is nothing in it to take, and a host that answered for ever holds this
    /// thread for not one byte.
    pub fn post(target: &Target, content_type: &str, body: &[u8], timeouts: Timeouts) -> bool {
        let headers = format!("Content-Type: {content_type}\r\n");
        let call = Call {
            verb: "POST",
            headers: Some(&headers),
            body,
            expect: 201,
            redirects: false,
            timeouts,
            answered: None,
        };

        exchange(target, &call, None).is_some()
    }

    /// What one exchange is besides its address — what [`get`] and [`post`] differ in.
    struct Call<'a> {
        verb: &'a str,
        /// Headers of ours beyond the agent, each ending in CRLF — `None` for the read.
        headers: Option<&'a str>,
        /// The body — empty for the read.
        body: &'a [u8],
        /// The one status that is an answer: 200 for the read, 201 for an appeal.
        expect: u32,
        /// Whether WinHTTP may follow a redirection — the read as it always has, an appeal never.
        redirects: bool,
        timeouts: Timeouts,
        /// ⭐ Stage E116, task T-116-12: set to `true` the moment an HTTP answer has come back,
        /// whatever its status — the mark of the first run is sent then (решение 208.59). `None`
        /// for every call that does not ask.
        answered: Option<&'a std::cell::Cell<bool>>,
    }

    /// How the body of an answer is read: at most this many bytes, each piece handed to the sink —
    /// or, for `None`, not read at all.
    type Read<'a> = Option<(u64, &'a mut dyn FnMut(&[u8]) -> bool)>;

    /// The one body under [`get`] and [`post`]: the session, the four timeouts of `call`, the
    /// connection, the request, the status — and the body of the answer to `read` (at most its
    /// cap) when there is a `read`, or nothing read at all when there is not.
    fn exchange(target: &Target, call: &Call<'_>, read: Read<'_>) -> Option<u64> {
        use windows::Win32::Networking::WinHttp::{
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WinHttpCloseHandle, WinHttpOpen,
            WinHttpSetTimeouts,
        };
        use windows::core::PCWSTR;

        // The length is a `u32` to WinHTTP; a body that does not fit is refused before any handle
        // exists, so no early return below has a handle to forget.
        let length = u32::try_from(call.body.len()).ok()?;

        let agent = crate::settings::wide(USER_AGENT);
        let host_wide = crate::settings::wide(&target.host);
        let path_wide = crate::settings::wide(&target.path);
        let verb = crate::settings::wide(call.verb);

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

        // The four timeouts: resolve, connect, send, receive — FR-102's for the read, the
        // appeal's own for an appeal. A host that does not answer must not hold a thread of this
        // program for minutes.
        //
        // SAFETY: `session` is the handle just opened; the four numbers are plain values.
        let _ = unsafe {
            WinHttpSetTimeouts(
                session,
                call.timeouts.resolve,
                call.timeouts.connect,
                call.timeouts.send,
                call.timeouts.receive,
            )
        };

        let request = Request {
            host: &host_wide,
            port: target.port,
            path: &path_wide,
            verb: &verb,
            secure: target.secure,
            call,
            length,
        };

        let answer = read_over(session, &request, read);

        // SAFETY: closes exactly the session opened above, once. Every handle below it is
        // closed by `read_over` before it returns.
        let _ = unsafe { WinHttpCloseHandle(session) };

        answer
    }

    /// The wide strings and values one request is made of — what [`exchange`] hands down.
    struct Request<'a> {
        host: &'a [u16],
        port: u16,
        path: &'a [u16],
        verb: &'a [u16],
        secure: bool,
        call: &'a Call<'a>,
        /// The length of `call.body`, already known to fit WinHTTP's `u32`.
        length: u32,
    }

    /// The connection, the request and the reading — the half of [`exchange`] that owns three
    /// handles, split out so that the session above is closed on every path.
    fn read_over(
        session: *mut std::ffi::c_void,
        request_of: &Request<'_>,
        read: Read<'_>,
    ) -> Option<u64> {
        use windows::Win32::Networking::WinHttp::{
            WINHTTP_DISABLE_REDIRECTS, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
            WINHTTP_OPTION_DISABLE_FEATURE, WinHttpCloseHandle, WinHttpConnect, WinHttpOpenRequest,
            WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption,
        };
        use windows::core::PCWSTR;

        let (host, path, verb) = (request_of.host, request_of.path, request_of.verb);

        // SAFETY: `session` is the live session and `host` a NUL-terminated buffer of the
        // caller's frame; the reserved argument is zero as documented.
        let connection =
            unsafe { WinHttpConnect(session, PCWSTR(host.as_ptr()), request_of.port, 0) };

        if connection.is_null() {
            crate::app::report_non_critical("WinHttpConnect", &windows::core::Error::from_thread());
            return None;
        }

        // ⚠ **`WINHTTP_FLAG_SECURE` is what makes this HTTPS and it is not optional.** Without
        // it the same call would fetch the same path over plain HTTP from port 443 and fail in
        // a way that looks like a network problem. Only the door of the stand ([`target_of`],
        // `testing`) asks for a target that is not secure.
        let flags = if request_of.secure {
            WINHTTP_OPEN_REQUEST_FLAGS(WINHTTP_FLAG_SECURE.0)
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };

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
                flags,
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

        // ⛔ **An appeal is not redirected** (TZ E107 §0 п. 9). The default of WinHTTP follows a
        // redirection and turns a `POST` into a `GET` on the way; the feature is switched off on
        // this request, and a request on which it would not switch off is not sent at all.
        //
        // SAFETY: `request` is the handle just opened; the buffer is a live local of four bytes,
        // the size the option is documented to take.
        let redirects_off = request_of.call.redirects
            || unsafe {
                WinHttpSetOption(
                    Some(request.cast_const()),
                    WINHTTP_OPTION_DISABLE_FEATURE,
                    Some(&WINHTTP_DISABLE_REDIRECTS.to_ne_bytes()),
                )
            }
            .map_err(|error| crate::app::report_non_critical("WinHttpSetOption", &error))
            .is_ok();

        if !redirects_off {
            // SAFETY: closes exactly the two handles opened above, once each.
            unsafe {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connection);
            }
            return None;
        }

        // **The read: no headers of ours and no body** — `None` for the headers, null and zero for
        // the optional data. That is the whole request of FR-102 (вопрос 101 п. 5). An appeal hands
        // its one header and its body; the total length is the body's own, so WinHTTP writes
        // `Content-Length` and sends the body in one piece.
        let headers: Option<Vec<u16>> = request_of
            .call
            .headers
            .map(|headers| headers.encode_utf16().collect());
        let body = request_of.call.body;
        let data = (!body.is_empty()).then(|| body.as_ptr().cast::<std::ffi::c_void>());

        // SAFETY: `request` is the handle just opened; the headers are a live buffer of this frame
        // whose length the crate passes beside it, and `data` points into `body`, which the caller
        // owns for the length of this call and which the call only reads — `request_of.length`
        // bytes of it, its whole length.
        let sent = unsafe {
            WinHttpSendRequest(
                request,
                headers.as_deref(),
                data,
                request_of.length,
                request_of.length,
                0,
            )
        };

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
            } else {
                // ⭐ Stage E116, task T-116-12: an answer came back — of any status (решение
                // 208.59: the mark of the first run is sent then, and goes no more).
                if let Some(answered) = request_of.call.answered {
                    answered.set(true);
                }

                if status_of(request) == request_of.call.expect {
                    match read {
                        Some((cap, sink)) => drain(request, cap, sink),
                        None => Some(0),
                    }
                } else {
                    // A status that is not the one expected — 200 for the read, 201 for an appeal —
                    // is a refusal like any other and is not journalled with its number: SEC-07,
                    // and the number is the host's business.
                    None
                }
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

    /// Reads the body to `cap` and no further — [`RESPONSE_CAP`] for the feed — handing every
    /// piece to `sink`, and answers how many bytes there were.
    fn drain(
        request: *mut std::ffi::c_void,
        cap: u64,
        sink: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Option<u64> {
        use windows::Win32::Networking::WinHttp::WinHttpReadData;

        let mut total: u64 = 0;
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
            // a document whose signature will not check out anyway. The updater's ceiling is the
            // size the signed feed named, so one byte more is a refusal too (TZ E104 §0 п. 15).
            let taken_bytes = u64::try_from(taken).unwrap_or(u64::MAX);

            if total.saturating_add(taken_bytes) > cap {
                return None;
            }

            if !sink(&chunk[..taken]) {
                return None;
            }

            total += taken_bytes;
        }

        Some(total)
    }

    /// **One read of the feed** — FR-102: [`read_from`] over the addresses this build carries,
    /// [`super::links::FEED_URLS`], and over nothing else. The product reads the feed through
    /// this function and through no other.
    pub fn read_now(language: &str) -> Option<Feed> {
        read_from(&super::links::FEED_URLS, language)
    }

    /// ⭐ **[`read_now`] with the mark of the first run** — stage E116, task T-116-12, решение
    /// 208.59. `first`: every address is asked with `?first=1` ([`marked`]). Answers the feed, as
    /// [`read_now`] does, and whether **any** address gave an HTTP answer, of any status — the
    /// moment the mark counts as sent. The program reads the feed through this function; the
    /// updater does not, and never sends the mark.
    ///
    /// ⭐ Task T-119-9: the good feed comes **with its document**, the signed text as the site gave
    /// it — the program keeps it on disk ([`super::keep_feed_document`]).
    pub fn read_now_marked(language: &str, first: bool) -> (Option<(Feed, String)>, bool) {
        let (urls, keys) = sources();
        let urls: Vec<&str> = urls.iter().map(String::as_str).collect();

        read_document_from_marked_with(&urls, language, first, &keys)
    }

    /// **Where the program reads the feed from, and with which keys** — [`super::links::FEED_URLS`]
    /// and [`FEED_KEYS`], and nothing else in a Release image. ⭐ Task T-119-13 (слово контролёра
    /// 2026-11-20): under the `testing` feature only, the doors of the stand ([`stand_doors`]) may
    /// name another address and other keys. The kept copy of the feed is read again with the same
    /// keys (`super::kept_feed`).
    pub fn sources() -> (Vec<String>, [[u8; 64]; 2]) {
        #[cfg(feature = "testing")]
        if let Some(stand) = stand_doors() {
            return stand;
        }

        (
            super::links::FEED_URLS
                .iter()
                .map(|url| (*url).to_owned())
                .collect(),
            FEED_KEYS,
        )
    }

    /// ⭐ **The doors of the stand for the program's own feed** — task T-119-13, слово контролёра
    /// 2026-11-20: **the very file and the very keys of the updater's doors** (TZ E104 §0 п. 22,
    /// `update-doors.toml` beside the program, laid over by
    /// [`crate::update::Places::of_this_machine`]), so that the guest without a network can show
    /// the seventh day and the kept feed. Only under the `testing` feature: a Release image has no
    /// trace of it — gate 6 of `tools\verify-perimeter.ps1` reads both images for the doors, and
    /// `tests\update.rs` holds every door behind its `cfg`. `None` when the doors name no feed.
    #[cfg(feature = "testing")]
    fn stand_doors() -> Option<(Vec<String>, [[u8; 64]; 2])> {
        let places = crate::update::Places::of_this_machine()?;

        (places.feed_url != super::links::FEED_URLS[0])
            .then(|| (vec![places.feed_url], places.keys))
    }

    /// ⭐ **The address of the feed with the mark of the first run** — stage E116, task T-116-12:
    /// `?first=1` added to the path, and nothing else — no number, nothing of the machine. The
    /// addresses of the feed carry no query of their own.
    pub fn marked(url: &str) -> String {
        format!("{url}?first=1")
    }

    /// [`read_from`] with the mark of the first run — stage E116, task T-116-12. See
    /// [`read_now_marked`].
    pub fn read_from_marked(urls: &[&str], language: &str, first: bool) -> (Option<Feed>, bool) {
        let (good, answered) = read_document_from_marked(urls, language, first);

        (good.map(|(feed, _)| feed), answered)
    }

    /// [`read_from_marked`] that also answers **the document** the good feed was read out of —
    /// task T-119-9. One body for both: the text kept on disk is the text that was just checked.
    pub fn read_document_from_marked(
        urls: &[&str],
        language: &str,
        first: bool,
    ) -> (Option<(Feed, String)>, bool) {
        read_document_from_marked_with(urls, language, first, &FEED_KEYS)
    }

    /// [`read_document_from_marked`] against named keys — the product passes [`FEED_KEYS`], or,
    /// under the `testing` feature, the keys of the doors of the stand ([`sources`]).
    fn read_document_from_marked_with(
        urls: &[&str],
        language: &str,
        first: bool,
        keys: &[[u8; 64]; 2],
    ) -> (Option<(Feed, String)>, bool) {
        let mut answered = false;

        for &url in urls {
            // A placeholder address is not read at all: `.invalid` never resolves, and asking
            // costs a DNS timeout for nothing (полномочие П5).
            if super::links::is_placeholder(url) {
                continue;
            }

            let address = if first { marked(url) } else { url.to_owned() };
            let (text, answer) = fetch_answering(&address);

            answered |= answer;

            let Some(text) = text else {
                continue;
            };

            match read_document(&text, language, |body, signature| {
                verify_with(body, signature, keys)
            }) {
                Ok(feed) => return (Some((feed, text)), answered),
                Err(refusal) => {
                    // SEC-07: the name of what went wrong and not a word of the document.
                    crate::diag::record(
                        crate::diag::Operation::from_name(refusal.journal_name()),
                        crate::diag::OsCode::NONE,
                    );
                }
            }
        }

        (None, answered)
    }

    /// **One read of the feed from these addresses**: every address in order, the first good
    /// answer winning.
    ///
    /// «Good» means all of it — the answer arrived, the signature checked out against one of the
    /// two keys, and the document parsed. An address that answers with something that is not
    /// the author's file is not «a bad host» to be retried differently; it is simply not an
    /// answer, and the next address is tried.
    ///
    /// `None` when no address gave one. The caller then leaves `feed_last_read` where it is, so
    /// the read stays due and is tried again tomorrow — the day of this attempt is already in
    /// `feed_last_try` (решение 148.8) — and not again today.
    ///
    /// # Why the addresses are a parameter — task T-88-1
    ///
    /// Since `e88` the address in the build is the author's real one, so a test can no longer
    /// prove «a placeholder is not read at all» by asking [`read_now`]: that would go out to the
    /// author's site. This is the same read over the addresses it is handed, and the test hands
    /// it a placeholder of its own — the battery stays without a socket, and the check it makes
    /// is the check the product makes. [`read_now`] is the only caller in the program.
    pub fn read_from(urls: &[&str], language: &str) -> Option<Feed> {
        // Stage E116, task T-116-12: one body with the marked read, unmarked here.
        read_from_marked(urls, language, false).0
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
    ///
    /// ⚠ **No `deny_unknown_fields`, and that is load-bearing** (посылка П3 of TZ E104, measured
    /// on the code of `e103`, 189.30): every installed build skips a top-level table it does not
    /// know, which is the only reason `[download]` and `[stop]` could arrive under `schema = 1`.
    /// The two tables are taken as raw TOML here and read by [`table_of`], so that a table of
    /// the wrong shape costs itself and not the whole file.
    #[derive(serde::Deserialize)]
    struct RawFeed {
        schema: u32,
        #[serde(default)]
        item: Vec<RawItem>,
        #[serde(default)]
        download: Option<toml::Value>,
        #[serde(default)]
        stop: Option<toml::Value>,
    }

    /// `[download]` as written: four fields, all of them obligatory.
    #[derive(serde::Deserialize)]
    struct RawDownload {
        version: String,
        url: String,
        sha256: String,
        size: u64,
    }

    /// `[stop]` as written: both fields may be left out, and then nothing is stopped.
    #[derive(serde::Deserialize)]
    struct RawStop {
        #[serde(default)]
        versions: Vec<String>,
        #[serde(default)]
        revoke_working: bool,
    }

    /// One of the two tables of the updater, read whole or not at all.
    fn table_of<R: serde::de::DeserializeOwned, T>(
        raw: Option<toml::Value>,
        check: impl FnOnce(R) -> Option<T>,
    ) -> Table<T> {
        let Some(value) = raw else {
            return Table::Absent;
        };

        // A table, and not an array of tables or a value: `[[download]]` is a mistake of the
        // author, and two installers for one version is not a thing this format can say.
        if !value.is_table() {
            return Table::Malformed;
        }

        value
            .try_into::<R>()
            .ok()
            .and_then(check)
            .map_or(Table::Malformed, Table::Read)
    }

    /// `[download]` checked for its shape: sixty-four hexadecimal digits of the sum, in either
    /// case, and a size that is not zero. Whether the address is the author's and the version
    /// newer is the updater's question ([`crate::update::decide`]), not the shape's.
    fn download_of(raw: RawDownload) -> Option<Download> {
        let digits = raw.sha256.as_bytes();

        if digits.len() != 64 || raw.size == 0 || raw.version.is_empty() || raw.url.is_empty() {
            return None;
        }

        let mut sha256 = [0u8; 32];

        for (byte, pair) in sha256.iter_mut().zip(digits.chunks_exact(2)) {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;

            *byte = u8::try_from(high * 16 + low).ok()?;
        }

        Some(Download {
            version: raw.version,
            url: raw.url,
            sha256,
            size: raw.size,
        })
    }

    /// `[stop]` has no shape beyond its types.
    fn stop_of(raw: RawStop) -> Option<Stop> {
        Some(Stop {
            versions: raw.versions,
            revoke_working: raw.revoke_working,
        })
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

// =========================================================================================
// 13. The appeal of the wizard — FR-104, решения 199.1…199.12, задача T-107-2
// =========================================================================================

/// **What the wizard sends to the site, and how** — решения 199.1, 199.7, 199.9 и 199.12.
///
/// # What goes, and what never does
///
/// Exactly the text the last step of the wizard showed the person (199.1: for a trouble what the
/// boxes of FR-104 let in, for an idea the person's words and the version), the language of the
/// interface, the kind of the appeal, the version of the program, the contact **if** the person
/// wrote one, and a nonce that makes a repeated press one appeal and not two. Nothing else: no
/// identifier of the machine, no name of the person, nothing the person did not see on the screen.
/// The address the site sees is the site's business (199.4: kept in memory against the floods
/// and never written beside the appeal).
///
/// # When
///
/// Only when the person presses «Отправить» (199.5, 199.12: «уходит на сайт, только когда вы сами
/// нажмёте «Отправить»»), and again only when they press «Повторить» — on a thread of its own
/// that lives one request ([`super::start_appeal`]). Never on its own, never in the background,
/// never a second time behind the person's back.
///
/// # The test folder of the site
///
/// `"test": true` sends an appeal into the folder «Тест» of the site, with no notice to the author
/// and cleared after seven days (199.7, 199.9). **Only a build with the `testing` feature writes
/// it** — the stand of TZ E107; the code that writes the field does not exist in a Release image,
/// and `the_shipped_body_carries_no_test_field` holds that.
pub mod appeal {
    use super::{feed, links, report};

    /// The longest contact the site takes, in characters — 199.7. The field of the wizard does
    /// not let more be typed, and [`contact_of`] cuts it here a second time.
    pub const CONTACT_MAX: usize = 200;

    /// `Content-Type` of the body — 199.7.
    pub const CONTENT_TYPE: &str = "application/json; charset=utf-8";

    /// **How long one appeal waits — 15 seconds at the very most** (TZ E107 §0 п. 10: «не больше
    /// 15 с всего»): 3 for the name, 4 for the connection, 4 for the sending, 4 for the answer.
    ///
    /// Why so much shorter than the feed's 60: a person is looking at «Отправляется…» and waiting,
    /// and a press that hangs for a minute reads as a program that hung. The site answers in well
    /// under a second (201.1: three appeals of premise П2 in one second of the clock), so four
    /// seconds a stage is room for a slow network and not for a dead one; a dead one ends in
    /// «Не удалось отправить», where «Повторить» and the letter wait.
    pub const TIMEOUTS: feed::Timeouts = feed::Timeouts {
        resolve: 3_000,
        connect: 4_000,
        send: 4_000,
        receive: 4_000,
    };

    /// `type` of the body — the two kinds the site knows (199.7). A `match` without a wildcard on
    /// purpose: a fourth card of the first step does not compile until somebody has said which of
    /// the two it is.
    pub fn kind_of(trouble: report::Trouble) -> &'static str {
        match trouble {
            report::Trouble::WrongResult | report::Trouble::NothingHappened => "error",
            report::Trouble::Idea => "wish",
        }
    }

    /// One appeal, as it goes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Appeal<'a> {
        /// [`kind_of`] the first step.
        pub kind: &'static str,
        /// The tag of the language of the interface — `settings::Language::tag`.
        pub lang: &'a str,
        /// The version of this build, `X.Y.Z`.
        pub version: &'a str,
        /// The text the field of the last step holds — what the person saw.
        pub text: &'a str,
        /// What the person typed into «Как с вами связаться», as typed; empty when nothing.
        pub contact: &'a str,
        /// [`nonce`] of this appeal — one for every press of this appeal (199.7 (б)).
        pub nonce: &'a str,
    }

    /// The contact as it goes: without the spaces around it and at most [`CONTACT_MAX`]
    /// characters. Empty means **no field at all** in the body (199.9: `null` is a 400).
    pub fn contact_of(typed: &str) -> String {
        let cut: String = typed.trim().chars().take(CONTACT_MAX).collect();

        cut.trim_end().to_owned()
    }

    /// **The body of the `POST`** — 199.7 and 199.9 word for word: `schema`, `type`, `lang`,
    /// `version`, `text`, `contact` only when there is one, `nonce` — and `test` only in a build of
    /// the stand. A pure function of its argument, so the tests read exactly what goes.
    ///
    /// Written by hand rather than through a crate: the list of crates is closed (C2), the body
    /// is seven fields, and [`string`] is the whole of JSON a string needs.
    pub fn body(appeal: &Appeal<'_>) -> String {
        let mut out = String::with_capacity(appeal.text.len() + appeal.contact.len() + 192);

        out.push_str("{\"schema\":1,\"type\":");
        string(&mut out, appeal.kind);
        out.push_str(",\"lang\":");
        string(&mut out, appeal.lang);
        out.push_str(",\"version\":");
        string(&mut out, appeal.version);
        out.push_str(",\"text\":");
        string(&mut out, appeal.text);

        let contact = contact_of(appeal.contact);

        // ⚠ **Absent, not `null` and not `""`** — 199.9: the site answers `null` with 400, and an
        // empty string would be a contact that says nothing.
        if !contact.is_empty() {
            out.push_str(",\"contact\":");
            string(&mut out, &contact);
        }

        out.push_str(",\"nonce\":");
        string(&mut out, appeal.nonce);

        // ⛔ **The stand's folder, and only the stand's.** Under `testing` the site files the appeal
        // under «Тест» (199.9); without the feature this line is not compiled, so no Release build
        // can send it — the field is not switched off there, it does not exist.
        #[cfg(feature = "testing")]
        out.push_str(",\"test\":true");

        out.push('}');
        out
    }

    /// One JSON string — RFC 8259 § 7: the quotation mark, the reverse solidus and the control
    /// characters escaped, everything else as it is (the body is UTF-8, 199.7).
    pub fn string(out: &mut String, value: &str) {
        out.push('"');

        for character in value.chars() {
            match character {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                control if u32::from(control) < 0x20 => {
                    out.push_str(&format!("\\u{:04x}", u32::from(control)));
                }
                other => out.push(other),
            }
        }

        out.push('"');
    }

    /// **A fresh nonce** — 32 lowercase hexadecimal digits of 16 bytes from the system's own
    /// generator (`BCryptGenRandom`, the preferred one of the system), 199.7 (б) and 199.8 (2).
    /// `None` when the system refused, and then the appeal is not sent: a nonce that is not random
    /// would make two people's appeals one.
    pub fn nonce() -> Option<String> {
        use windows::Win32::Security::Cryptography::{
            BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
        };

        let mut bytes = [0u8; 16];

        // SAFETY: `bytes` is a live local the call fills, and the crate passes its length; no
        // algorithm handle is needed with the flag of the system's preferred generator.
        let drawn = unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };

        if drawn.is_err() {
            crate::app::report_non_critical(
                "BCryptGenRandom",
                &windows::core::Error::from_hresult(drawn.to_hresult()),
            );
            return None;
        }

        Some(hex(&bytes))
    }

    /// **The nonce of an appeal: the one it has, or a fresh one the first time** — 199.7 (б):
    /// «Повторить» sends the nonce of the first press, so the site answers a repeat with the same
    /// 201 and records the appeal once. `draw` is asked only when there is none yet; `None` when it
    /// is asked and refuses.
    pub fn keep_or_draw(
        kept: &mut Option<String>,
        draw: impl FnOnce() -> Option<String>,
    ) -> Option<&str> {
        if kept.is_none() {
            *kept = draw();
        }

        kept.as_deref()
    }

    /// Bytes as lowercase hexadecimal digits, two to a byte.
    pub fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";

        let mut out = String::with_capacity(bytes.len() * 2);

        for &byte in bytes {
            out.push(char::from(DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(DIGITS[usize::from(byte & 0x0F)]));
        }

        out
    }

    /// **Sends one body to [`links::APPEAL_URL`], and whether the site took it** — `true` only for
    /// `201`. Blocks for at most [`TIMEOUTS`]; called on the thread of [`super::start_appeal`].
    ///
    /// The journal gets the fact and never a byte of the appeal (SEC-01, SEC-07): «appeal request»
    /// before anything leaves, «appeal sent» or «appeal refused» after.
    pub fn send(body: &str) -> bool {
        let Some(target) = feed::target_of(links::APPEAL_URL) else {
            return false;
        };

        crate::diag::record(
            crate::diag::Operation::from_name("appeal request"),
            crate::diag::OsCode::NONE,
        );

        let sent = feed::post(&target, CONTENT_TYPE, body.as_bytes(), TIMEOUTS);

        crate::diag::record(
            crate::diag::Operation::from_name(if sent {
                "appeal sent"
            } else {
                "appeal refused"
            }),
            crate::diag::OsCode::NONE,
        );

        sent
    }
}
