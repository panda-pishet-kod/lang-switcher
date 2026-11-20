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

            let released = item
                .date
                .map(|day| format_date_in(language, day))
                .unwrap_or_default();

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

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, MSG, SW_SHOWNOACTIVATE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{Error as WinError, PCWSTR};

use crate::settings::{self, Language};
use crate::theme::ThemeSetting;
// Общий слой элементов окон — задача Т-45-2, решение 107.1. Мастер и окна писем берут элементы
// здесь: в этом и состоит принцип пользователя «один механизм на все окна».
use crate::widgets;

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
    pub fn road(&self) -> &'static [Step] {
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
    pub fn current(&self) -> Step {
        if self.done {
            return Step::Done;
        }

        let road = self.road();

        road[self.step.min(road.len() - 1)]
    }

    /// Whether this is the last step — the one whose button says «Готово».
    pub fn is_last(&self) -> bool {
        self.step + 1 >= self.road().len()
    }
}

/// What «Далее» did — [`Wizard::forward`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forward {
    /// The thanks was on the screen: the window closes.
    Close,
    /// The last step: the appeal is to go to the site — [`Wizard::send`].
    Send,
    /// The next step is on the screen.
    Moved,
}

/// The answers a person types into the wizard — [`Wizard::answer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// «Программа» of «Где это случилось?».
    Program,
    /// «Ожидали» of «Что вы делали?».
    Expected,
    /// «Получили».
    Got,
    /// «Что предлагаете?» of «Опишите идею».
    Idea,
    /// «Чем это поможет?».
    Helps,
}

/// How often the window of the wizard asks whether the site answered, ms — only while the appeal
/// is on its way (T-120-19: the window of the shell has no message of its own to be posted, so it
/// asks; NFR-10 — no timer at all at rest).
pub const WIZARD_POLL_MS: u32 = 250;

/// One second of the count-down of «Взять из активного окна», ms.
pub const WIZARD_SECOND_MS: u32 = 1000;

impl Wizard {
    /// **The wizard as the menu opens it** — FR-104: the layouts of this machine, the first of them
    /// chosen, the hotkey as it acts, and what the machine says about itself now. Until T-120-19
    /// this was `wizard_state`, the state of the dialog `IDD_WIZARD`.
    pub fn now() -> Self {
        let (_, hotkey) = look();
        let layouts = crate::layouts::enumerate_all()
            .map(|installed| settings::layout_labels(&installed, &installed))
            .unwrap_or_default();
        let mut draft = report::Draft {
            hotkey,
            ..report::Draft::default()
        };
        if let Some(first) = layouts.first() {
            draft.layout = first.clone();
        }
        Self::with(draft, report::facts_now(), layouts)
    }

    /// A wizard of the given answers and facts on its first step — [`Wizard::now`], and the
    /// descriptions of the instrument of fitting and of the pictures (`ui::windows::wizard`).
    pub fn with(draft: report::Draft, facts: report::Facts, layouts: Vec<String>) -> Self {
        Self {
            draft,
            facts,
            step: 0,
            countdown: 0,
            status: String::new(),
            layouts,
            text: String::new(),
            contact: String::new(),
            post: Post::Ready,
            nonce: None,
            done: false,
        }
    }

    /// **Whether the appeal reads right to left** — the direction of the language of the text
    /// itself, not of the window (T-120-19, the C3 of the controller): the appeal is data for the
    /// author. The first line is the name of the program and its version, Latin in every language;
    /// the first letter after it is a label of the report in the language it was built in — the
    /// English stand-ins until the wave included — or what the person wrote over it.
    pub fn appeal_reads_right_to_left(&self) -> bool {
        self.text
            .lines()
            .skip(1)
            .flat_map(str::chars)
            .find(|letter| letter.is_alphabetic())
            .is_some_and(|letter| {
                // Hebrew, Syriac, Arabic, Thaana, N'Ko and the presentation forms of Hebrew and
                // Arabic — the letters of the right-to-left languages of the program.
                matches!(u32::from(letter), 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
            })
    }

    /// Where on [`Wizard::road`] the person is: 0 is the first step.
    pub fn number(&self) -> usize {
        self.step.min(self.road().len() - 1)
    }

    /// Where the appeal is on its way to the site.
    pub fn post(&self) -> Post {
        self.post
    }

    /// What the person filled in.
    pub fn draft(&self) -> &report::Draft {
        &self.draft
    }

    /// What the machine said about itself when the window opened.
    pub fn facts(&self) -> &report::Facts {
        &self.facts
    }

    /// The layout names of the list «Раскладка».
    pub fn layouts(&self) -> &[String] {
        &self.layouts
    }

    /// Seconds left of the capture count-down; zero — it is not running.
    pub fn countdown(&self) -> u32 {
        self.countdown
    }

    /// The line of the last press: «Текст скопирован…», «Сохранено…», a failure.
    pub fn said(&self) -> &str {
        &self.status
    }

    /// The appeal as its field holds it — the truth from the last step on.
    pub fn appeal(&self) -> &str {
        &self.text
    }

    /// What the field «Как с вами связаться» holds.
    pub fn contact(&self) -> &str {
        &self.contact
    }

    /// The period of the window's timer: the count-down's second while it runs, the asking while
    /// the appeal is on its way, none at rest (NFR-10).
    pub fn timer(&self) -> Option<u32> {
        if matches!(self.post, Post::Sending(_)) {
            Some(WIZARD_POLL_MS)
        } else if self.countdown > 0 {
            Some(WIZARD_SECOND_MS)
        } else {
            None
        }
    }

    /// **«Далее»** — the next step; on the way into the last one the appeal is built from the
    /// answers and the facts, with the contact already typed (until T-120-19 `wizard_forward`).
    pub fn forward(&mut self) -> Forward {
        if self.done {
            return Forward::Close;
        }
        if self.is_last() {
            return Forward::Send;
        }
        self.step += 1;
        self.status.clear();
        if self.current() == Step::Preview {
            let built = report::build(&self.draft, &self.facts);
            self.text = report::with_contact(&built, "", &self.contact);
        }
        Forward::Moved
    }

    /// **«Назад»** — the step before; a failure of the site is forgotten with it.
    pub fn back(&mut self) {
        self.step = self.step.saturating_sub(1);
        self.status.clear();
        self.post = Post::Ready;
    }

    /// The card of «Что случилось?» — and with it the road.
    pub fn choose(&mut self, trouble: report::Trouble) {
        self.draft.trouble = trouble;
    }

    /// The kind of field of «Где это случилось?».
    pub fn set_field(&mut self, field: report::Field) {
        self.draft.field = field;
    }

    /// «Повторяется».
    pub fn set_repeat(&mut self, repeat: report::Repeat) {
        self.draft.repeat = repeat;
    }

    /// One box of «Что приложить?», 0…3: the machine, the layouts, the settings, the journal.
    pub fn attach(&mut self, index: usize, on: bool) {
        let attach = &mut self.draft.attach;
        match index {
            0 => attach.machine = on,
            1 => attach.layouts = on,
            2 => attach.settings = on,
            _ => attach.journal = on,
        }
    }

    /// The layout of «Раскладка» by its place in [`Wizard::layouts`].
    pub fn choose_layout(&mut self, index: usize) {
        if let Some(name) = self.layouts.get(index) {
            self.draft.layout.clone_from(name);
        }
    }

    /// What a person typed into one of the answers.
    pub fn answer(&mut self, which: Answer, value: String) {
        let draft = &mut self.draft;
        match which {
            Answer::Program => draft.program = value,
            Answer::Expected => draft.expected = value,
            Answer::Got => draft.got = value,
            Answer::Idea => draft.idea = value,
            Answer::Helps => draft.helps = value,
        }
    }

    /// The appeal as the person edited it in its field — from now on the truth.
    pub fn edit_appeal(&mut self, value: String) {
        if !value.is_empty() {
            self.text = value;
        }
    }

    /// **«Как с вами связаться»** — задача T-107-3: the line «Для связи: …» of the appeal follows
    /// the field, put in, changed or taken out (until T-120-19 `wizard_contact_changed`).
    pub fn edit_contact(&mut self, typed: String) {
        if self.current() != Step::Preview || self.text.is_empty() {
            self.contact = typed;
            return;
        }
        self.text = report::with_contact(&self.text, &self.contact, &typed);
        self.contact = typed;
    }

    /// **«Отправить» or «Повторить»** — задача T-107-3: the appeal goes to the site on a thread of
    /// its own, with the nonce of the first press; nothing while it is on its way or empty. The
    /// answer is taken by [`Wizard::take_answer`] on the window's timer.
    pub fn send(&mut self, language: Language) -> bool {
        static TICKETS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

        if matches!(self.post, Post::Sending(_)) || self.text.trim().is_empty() {
            return false;
        }
        let ticket = TICKETS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Some(nonce) = appeal::keep_or_draw(&mut self.nonce, appeal::nonce) else {
            self.post = Post::Failed;
            return true;
        };
        let body = appeal::body(&appeal::Appeal {
            kind: appeal::kind_of(self.draft.trouble),
            lang: language.tag(),
            version: env!("CARGO_PKG_VERSION"),
            text: &self.text,
            contact: &self.contact,
            nonce,
        });
        self.post = Post::Sending(ticket);
        self.status.clear();
        if !start_appeal(ticket, body) {
            self.post = Post::Failed;
        }
        true
    }

    /// The answer of the site, if it came: the thanks, or «Не удалось отправить».
    pub fn take_answer(&mut self) -> bool {
        let Post::Sending(waiting) = self.post else {
            return false;
        };
        let answers = APPEAL_ANSWERS
            .lock()
            .map(|mut answers| std::mem::take(&mut *answers))
            .unwrap_or_default();
        let Some((_, sent)) = answers.iter().find(|(ticket, _)| *ticket == waiting) else {
            return false;
        };
        if *sent {
            self.post = Post::Ready;
            self.done = true;
        } else {
            self.post = Post::Failed;
        }
        self.status.clear();
        true
    }

    /// **«Взять из активного окна»** — the count-down starts; a second press while it runs is
    /// nothing.
    pub fn start_capture(&mut self) -> bool {
        if self.countdown > 0 {
            return false;
        }
        self.countdown = crate::guard::CAPTURE_SECONDS;
        true
    }

    /// One second of the count-down; at zero the window in front — not `mine` — is read into
    /// «Программа» (until T-120-19 `wizard_tick`).
    pub fn second(&mut self, mine: HWND) {
        if self.countdown == 0 {
            return;
        }
        self.countdown -= 1;
        if self.countdown > 0 {
            return;
        }
        let caught = capture_foreground(mine);
        if !caught.is_empty() {
            self.draft.program = caught;
        }
    }

    /// **«Скопировать текст»** — the appeal to the clipboard, and the line says where to send it.
    pub fn copy_appeal(&mut self, owner: HWND) {
        use crate::settings::{IDS_WIZARD_FAILED, IDS_WIZARD_TEXT_COPIED, format_text, text};

        if self.text.is_empty() {
            return;
        }
        self.status = if crate::selection::write_unicode_text(owner, &self.text).is_ok() {
            format_text(IDS_WIZARD_TEXT_COPIED, &[links::MAIL_ADDRESS])
        } else {
            text(IDS_WIZARD_FAILED)
        };
    }

    /// **«Скопировать адрес»**.
    pub fn copy_address(&mut self, owner: HWND) {
        use crate::settings::{IDS_WIZARD_ADDRESS_COPIED, IDS_WIZARD_FAILED, format_text, text};

        self.status = if crate::selection::write_unicode_text(owner, links::MAIL_ADDRESS).is_ok() {
            format_text(IDS_WIZARD_ADDRESS_COPIED, &[links::MAIL_ADDRESS])
        } else {
            text(IDS_WIZARD_FAILED)
        };
    }

    /// **«Сохранить в папку журнала»** — the appeal into a file of the day in the folder of the
    /// journal, and the folder opened.
    pub fn save(&mut self) {
        use crate::settings::{IDS_WIZARD_FAILED, IDS_WIZARD_SAVED, format_text, text};

        if self.text.is_empty() {
            return;
        }
        let Some(folder) = crate::diag::log_dir() else {
            self.status = text(IDS_WIZARD_FAILED);
            return;
        };
        let today =
            today().unwrap_or_else(|| Date::from_ymd(1970, 1, 1).expect("the epoch is a day"));
        let path = folder.join(report::file_name(today));
        crate::sandbox::guard_path(&path);
        if std::fs::create_dir_all(&folder).is_err() || std::fs::write(&path, &self.text).is_err() {
            self.status = text(IDS_WIZARD_FAILED);
            return;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.status = format_text(IDS_WIZARD_SAVED, &[&name]);
        open_folder(&folder);
    }
}

/// **The eight pages of the wizard** for the instrument of fitting and the pictures of C3
/// (T-120-19): the five steps of a trouble, the step of an idea, the thanks and «Не удалось
/// отправить» — with answers near the longest a person types and the facts of a machine whose
/// journal is full. The name of each page is what the instrument prints.
pub fn wizard_pages() -> Vec<(&'static str, Wizard)> {
    use report::{Attach, Draft, Facts, Field, Repeat, Trouble};

    let layouts = vec![
        "Русская (Россия)".to_owned(),
        "English (United States)".to_owned(),
    ];
    let draft = |trouble| Draft {
        trouble,
        program: "notepad.exe · Notepad".to_owned(),
        field: Field::Normal,
        layout: layouts[0].clone(),
        hotkey: "Pause".to_owned(),
        expected: "привет, как дела".to_owned(),
        got: "ghbdtn? rfr ltkf".to_owned(),
        repeat: Repeat::Always,
        idea: "Флаг текущей раскладки в трее, как у Punto Switcher".to_owned(),
        helps: "Привычка: видно раскладку, не глядя на РУС/ENG".to_owned(),
        attach: Attach::default(),
    };
    let facts = Facts {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        windows: "Windows 11 Pro 26100".to_owned(),
        scale: "125 %".to_owned(),
        monitors: "1".to_owned(),
        layouts: "ru-RU, en-US".to_owned(),
        settings: "Pause · auto · выделение вкл · исключений 0".to_owned(),
        journal: "12:04:11 hook.install ok\n12:04:11 layouts.cache 2".to_owned(),
        journal_entries: 37,
    };
    let page = |trouble, step, post, done| {
        let mut wizard = Wizard::with(draft(trouble), facts.clone(), layouts.clone());
        wizard.step = step;
        wizard.post = post;
        wizard.done = done;
        if wizard.current() == Step::Preview || done {
            wizard.text = report::build(&wizard.draft, &wizard.facts);
        }
        wizard
    };

    vec![
        (
            "wizard what",
            page(Trouble::WrongResult, 0, Post::Ready, false),
        ),
        (
            "wizard where",
            page(Trouble::WrongResult, 1, Post::Ready, false),
        ),
        (
            "wizard did",
            page(Trouble::WrongResult, 2, Post::Ready, false),
        ),
        (
            "wizard attach",
            page(Trouble::WrongResult, 3, Post::Ready, false),
        ),
        (
            "wizard preview",
            page(Trouble::WrongResult, 4, Post::Ready, false),
        ),
        ("wizard idea", page(Trouble::Idea, 1, Post::Ready, false)),
        (
            "wizard sent",
            page(Trouble::WrongResult, 4, Post::Ready, true),
        ),
        (
            "wizard failed",
            page(Trouble::WrongResult, 4, Post::Failed, false),
        ),
    ]
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

/// Whether a window of this kind is open on this thread.
pub fn is_open(kind: Kind) -> bool {
    window_of(kind).is_some()
}

/// The handle of the open window of this kind, if there is one.
fn window_of(kind: Kind) -> Option<HWND> {
    SHELL.with_borrow(|open| {
        open.iter()
            .find(|window| window.kind == kind)
            .map(|window| window.hwnd)
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
    // Задача T-118-2: Ctrl+A в поле — тело общее, `widgets::keys`. Диалогов у `letters` нет с
    // T-120-19: мастер был последним; окна оболочки ведут клавиатуру сами.
    widgets::keys::select_all_on_ctrl_a(message)
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

// -----------------------------------------------------------------------------------------
// The window procedure
// -----------------------------------------------------------------------------------------

// ⚠ **`field_box_air` уехала в `widgets::field::air` — задача Т-45-2, решение 107.4.**
//
// Здесь она стояла с задачи Т-33а-2 и была ВТОРОЙ копией арифметики окна настроек: та
// (`theme::field_frame_air`) отвечала одним числом на обе стороны и на нечётном остатке теряла
// пиксель, эта клала его вниз — чтобы коробка вышла ровно вровень с кнопкой рядом. Теперь тело
// одно, и различие названо параметром `widgets::field::OddPixel`, а не двумя телами.

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
    /// The title of the head — `IDS_ABOUT_LINE_1`, задача T-81-3. The second line of the head
    /// of «О программе», `IDS_ABOUT_LINE_2`, is not in «От автора» since C6 of T-120-17
    /// (решение контролёра по слову 208.178: in the mock-up of the window there is none).
    pub line_1: String,
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
    /// The feed has been read — the dot of its line is the accent; else the quiet one.
    pub feed_read: bool,
    /// A newer version is announced — the dot of the line of the version warns.
    pub update_waiting: bool,
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
        IDS_ABOUT_LINE_1, IDS_AUTHOR_TEXT, IDS_FEEDBACK_TEXT, IDS_NEWS_ABOUT_FEED,
        IDS_NEWS_AVAILABLE, IDS_NEWS_FILE_ONLY, IDS_NEWS_LATEST, IDS_NEWS_NEVER_READ,
        IDS_NEWS_OFF_IN_FILE, IDS_NEWS_READ_ON,
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

    // Строки версии под именем больше нет (T-120-17): до задачи T-81-3 она несла
    // «версия {0} · Панда, он же Panda_Pishet_Kod» (снята решением 142.2 п. 5), потом —
    // `IDS_ABOUT_VERSION`; у окна оболочки нет имени, под которым ей стоять, и номер версии
    // стоит в строке состояния (`IDS_NEWS_LATEST`, `IDS_NEWS_AVAILABLE`).
    AuthorView {
        // Заголовок общей шапки — те же слова, что в «О программе», из той же строки; подписи
        // под ним у «От автора» нет (C6 T-120-17).
        line_1: text(IDS_ABOUT_LINE_1),
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
                    &format_date_in(language, day),
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
        feed_read: state.feed_last_read.is_some(),
        update_waiting: update.and_then(|item| item.version.as_deref()).is_some(),
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
    format_date_in(settings::ui_language(), date)
}

/// [`format_date`] in `language` — the language of the window that shows the date, which is the
/// program's in the running program and any language in a description built for it (T-120-17:
/// «От автора» in Hebrew printed the day with the Russian month of the program).
pub fn format_date_in(language: Language, date: Date) -> String {
    let locale = settings::wide(language.tag());

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
        AuthorView, Entry, FeedItem, HWND, Kind, Letter, ThemeSetting, show_letter, window_of,
    };
    use std::cell::Cell;
    use windows::Win32::Foundation::HMODULE;

    thread_local! {
        /// Whether [`super::raise`] must leave the window hidden.
        static HIDDEN: Cell<bool> = const { Cell::new(false) };
    }

    /// Whether the window being opened is to stay hidden.
    pub(super) fn hidden() -> bool {
        HIDDEN.with(Cell::get)
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

    /// «От автора» open on this thread now — a new window after a change of language, which builds
    /// it again (T-120-17).
    pub fn author_window() -> Option<HWND> {
        window_of(Kind::Author)
    }

    /// [`super::language_changed`] with the windows of this module left hidden: the letter and the
    /// list it opens again in the new language are not shown on the screen of a test.
    pub fn language_changed_hidden(owner: HWND) {
        hidden_while(|| super::language_changed(owner));
    }

    /// **The door of task T-103-2** — opens the wizard of FR-104 on `owner`, a window of the shell
    /// since T-120-19, and never shows it.
    ///
    /// The window answered is the caller's to destroy.
    pub fn open_wizard_hidden(owner: HWND, _module: HMODULE) -> Option<HWND> {
        hidden_while(|| super::open_wizard_with(owner, super::settings::ui_language(), None, false))
    }

    /// **The door of task T-103-2** — where the button of the open window `hwnd` that sends a
    /// person to write would take them: the address «Скопировать и открыть…» of the wizard and
    /// «Открыть группу» of «Спасибо» hand to the shell, read by the very call they read it by
    /// (решение 181.1). Nothing is opened.
    ///
    /// `hwnd` is a window of this module on the caller's thread.
    pub fn feedback_url(hwnd: HWND) -> Option<&'static str> {
        shell_language(hwnd).map(super::links::feedback_url)
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
        shell_language(hwnd).map(|language| {
            (
                super::links::download_url(language),
                super::links::program_url(language),
            )
        })
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
    let shell: Vec<ShellWindow> = SHELL.with_borrow(|open| open.clone());

    for window in shell {
        if settings::language_switch(window.language, settings::ui_language())
            == settings::LanguageSwitch::Unchanged
        {
            continue;
        }

        if window.kind == Kind::Wizard {
            KEEP_WIZARD.set(true);
            close_window(window.hwnd);
            KEEP_WIZARD.set(false);
            let held = HELD_WIZARD.take();
            open_wizard_with(owner, settings::ui_language(), held, false);
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
    open_wizard_with(owner, settings::ui_language(), None, true);
}

/// The wizard on `owner` in `language` — a window of the shell since T-120-19
/// (`ui::windows::wizard`); `held` — the answers of a wizard that closed for a change of the
/// language ([`language_changed`]), else a fresh one ([`Wizard::now`]). The one that is open is
/// raised instead.
fn open_wizard_with(
    owner: HWND,
    language: Language,
    held: Option<Wizard>,
    activate: bool,
) -> Option<HWND> {
    if let Some(hwnd) = window_of(Kind::Wizard) {
        raise(hwnd, activate);
        return Some(hwnd);
    }
    let (theme, _) = look();
    let product = crate::ui::windows::wizard::Product {
        page: crate::ui::windows::wizard::Page {
            wizard: held.unwrap_or_else(Wizard::now),
            language,
            theme,
        },
        owner,
        _record: ShellRecord(Kind::Wizard),
    };

    open_shell(Kind::Wizard, None, language, owner, product, activate)
}

/// The open wizard — the window its capture of the active window must not take for the active
/// one.
pub fn wizard_window() -> Option<HWND> {
    window_of(Kind::Wizard)
}

thread_local! {
    /// Whether the wizard that closes now closes for a change of the language — its answers are
    /// kept for the window that opens in its place ([`wizard_closed`]).
    static KEEP_WIZARD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The answers on their way from that window to the new one.
    static HELD_WIZARD: RefCell<Option<Wizard>> = const { RefCell::new(None) };
}

/// **The window of the wizard closed** with `wizard` in it — `ui::windows::wizard::Product` says so
/// as it goes. Kept only when [`language_changed`] closed it, to open it again in the new language
/// with every answer where the person left it (the dialog until T-120-19 relabelled itself).
pub fn wizard_closed(wizard: Wizard) {
    if KEEP_WIZARD.get() {
        HELD_WIZARD.set(Some(wizard));
    }
}

/// An empty wizard — what a closing window leaves behind it in place of the one it hands over.
pub fn wizard_left() -> Wizard {
    Wizard::with(
        report::Draft::default(),
        report::Facts::default(),
        Vec::new(),
    )
}

// ⛔ **`open_thanks` is gone — задача Т-33-3, решение 103.3.** «Готово» used to close the
// wizard and open a window of its own on `IDD_LETTER` with four lines of text in it. The user
// asked for the thanks to be the wizard's last step, in the same window and at the same size:
// «Окно спасибо в конце было бы логично сделать того же размера, что и мастер, и сделать
// финальным шагом мастера, а не отдельным всплывающим окном». What that window said is said by
// [`Step::Done`] now, out of the very same four strings, and `Kind::Thanks` went with it.

// ⚠ **`wizard_refresh_controls` убран — задача Т-46-3, решение 109.2.** Он звал `fill_wizard`
// на КАЖДЫЙ щелчок, и тот переписывал поля устаревшим черновиком (находка пользователя: «текст
// в полях сбрасывается при нажатии любого переключателя»), а затем инвалидировал все контролы
// шага и ещё семь. Дорога щелчка теперь — `wizard_after_choice` с перерисовкой затронутого по
// роду элемента; отсчёт, состояние и захват — три подписи ниже.

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
/// start, and then nothing was sent. The answer waits in `APPEAL_ANSWERS` for the window of the
/// wizard to take it on its timer ([`Wizard::take_answer`]): a window of the shell has no message
/// of its own to be posted (T-120-19).
fn start_appeal(ticket: u32, body: String) -> bool {
    let started = std::thread::Builder::new()
        .name("langsw-appeal".to_owned())
        .spawn(move || {
            let sent = appeal::send(&body);

            if let Ok(mut answers) = APPEAL_ANSWERS.lock() {
                answers.push((ticket, sent));
            }
        });

    if started.is_err() {
        crate::app::report_non_critical("thread body", &WinError::from_thread());
        return false;
    }

    true
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

/// Opens a folder in the shell — the road the journal folder of the settings window takes.
fn open_folder(folder: &std::path::Path) {
    // ⭐ **Finding С26, task T-41-4 — this is the door the finding warned about.** It opens the
    // *same* journal folder as the button «Открыть папку журнала» of the settings dialog, by
    // different code: «вторая дверь через полгода» had already happened by the time the repair
    // arrived. Both now go through [`settings::open_in_the_shell`].
    settings::open_in_the_shell(&folder.display().to_string());
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
