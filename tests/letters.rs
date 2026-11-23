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

#[path = "support/excused.rs"]
mod excused;
use lang_switcher::letters::{
    self, Date, FeedItem, FeedView, Letter, REMINDERS_PER_NEWS, Thanks, links,
};
use lang_switcher::settings::{self, Language, Letters};

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};

/// The resource script with its language files — the one reader of the string tables (T-L10N-2).
#[path = "support/resource_script.rs"]
mod resource_script;
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

/// The language of the interface is one for the whole process, and a test that moves it must not
/// move it under another: every test that reads a word of a letter holds this lock.
static LOCALE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Points the string tables at the product binary and the language at Russian, and answers the
/// lock of the language. Every test that reads a word of a letter calls this first and keeps the
/// guard to its end.
fn with_product_strings() -> std::sync::MutexGuard<'static, ()> {
    let guard = LOCALE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    guard
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
// Чем стучится каждое письмо — задача Т-33а-4, решение 104.4
// =========================================================================================

/// **Находка контролёра на снимке пользователя, а не теста.** Уведомление письма «Что нового»
/// несло слова уведомления «Обновления»: «Вышла версия 0.43.0 · Три изменения **и ссылка на
/// загрузку**». Версия не «вышла» — она уже стоит и работает; загружать по этому письму нечего.
///
/// Письма «Что нового» с его стуком больше нет (решение 208.94 п. 4, T-119-8); сторож остался на
/// том, ради чего он был: ни одно письмо не стучится чужими словами.
///
/// ⛔ **«Обновление» не стучится с задачи T-119-9** (решение 208.99 п. 1): уведомления Windows о нём
/// нет, его слова 3137 и 3138 сняты, письмо открывает только сам человек.
#[test]
fn every_letter_knocks_with_its_own_words() {
    assert_eq!(
        letters::toast_strings(Letter::Update),
        None,
        "решение 208.99 п. 1: «Обновление» уведомлением не объявляется"
    );

    // Остальные два письма — на месте, и «Привет» по-прежнему не объявляется вовсе.
    assert_eq!(
        letters::toast_strings(Letter::Thanks),
        Some((settings::IDS_TOAST_TITLE, settings::IDS_TOAST_THANKS))
    );
    assert_eq!(
        letters::toast_strings(Letter::News(7)),
        Some((settings::IDS_TOAST_TITLE, settings::IDS_TOAST_NEWS))
    );
    assert_eq!(
        letters::toast_strings(Letter::Welcome),
        None,
        "«Привет» показывается сразу и уведомлением не объявляется — FR-101"
    );

    // И ни одно письмо не делит тело с другим: одинаковые слова на разные поводы — это ровно
    // тот дефект, который здесь чинится, только в другом месте таблицы.
    let bodies: Vec<u16> = [Letter::Update, Letter::News(1), Letter::Thanks]
        .into_iter()
        .filter_map(|letter| letters::toast_strings(letter).map(|(_, body)| body))
        .collect();

    let mut unique = bodies.clone();
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(
        bodies.len(),
        unique.len(),
        "два письма стучатся одним телом: {bodies:?}"
    );
}

// ⚠ Влезание этих строк в пределы оболочки (`szInfoTitle` 63 единицы, `szInfo` 255) мерится по
// всем четырнадцати языкам в `tests\settings.rs`, а не здесь: чтение строк требует ЗАМКА —
// `with_product_strings` там отдаёт `MutexGuard`, — потому что язык интерфейса величина
// ПРОЦЕССА, и перебор четырнадцати языков без замка сбивал бы соседние тесты, читающие слова.
// Здешний тест замка не просит и не может: он не читает ни одной строки, только их номера.

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

/// A machine that came through the migration **is** «обновившийся»: it has seen «Привет», so
/// «Привет» is not what it meets.
///
/// ⚠⚠ **ПЕРЕВЁРНУТ задачей T-53a-5, решение владельца 118.5.** До неё этот тест утверждал
/// вторую половину: обновившийся встречает **«Что нового»**, и `initialise` не смеет эту
/// версию молча пометить виденной («initialisation must not silence «Что нового» for somebody
/// who updated»). Ворота письма закрыты — описание изменений приходит с сигналом ленты **до**
/// установки, — и вместе с ними перевернулась ровно эта половина: теперь `initialise`
/// **обязана** пометить версию виденной, потому что она единственная, кто отметку ещё двигает.
///
/// ⭐ **Первая половина правды цела и проверяется по-прежнему:** «Привет» обновившемуся не
/// приходит. Ради неё тест и держится — без неё исчезновение письма скрыло бы и её.
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
        state.last_seen_version, "0.39.0",
        "решение 118.5: отметку виденной версии ведёт `initialise`, и только она — иначе, \
         когда письмо однажды вернут, оно придёт за все пропущенные версии разом"
    );

    // Половина, ради которой тест жив: «Привет» тому, кто его уже видел, не приходит.
    let due = letters::due(&state, today, "0.39.0", FeedView::EMPTY, true);
    assert_ne!(
        due,
        Some(Letter::Welcome),
        "«Привет» показывается один раз и обновившемуся не приходит"
    );
    assert_eq!(
        due, None,
        "и «Что нового» с решения 118.5 не приходит тоже — ворота закрыты, лента расскажет \
         об изменениях до установки"
    );
}

// =========================================================================================
// The gates: the quiet moment and the one letter a day — FR-101
// =========================================================================================

/// Everything but «Привет» waits for a quiet moment.
///
/// ⚠ **Повозку сменила задача T-53a-5** (решение 118.5): гейт тишины проверялся письмом
/// «Что нового», а его ворота закрыты. Проверяемое правило то же — оно про **гейт**, а не про
/// то письмо, — и теперь его везёт «Спасибо». ⛔ Тест не выброшен именно потому, что его
/// правда ко второй задаче отношения не имеет.
#[test]
fn nothing_but_hello_arrives_while_the_moment_is_not_quiet() {
    let mut state = settled("0.39.0");
    state.thanks_due = Some(day(2026, 9, 1));
    let today = day(2026, 9, 3);

    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, false),
        None,
        "«Спасибо» причитается, но минута не спокойная"
    );
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Thanks),
        "и приходит, как только она спокойна, — гейт один на все письма, кроме «Привет»"
    );
}

/// One letter a day, and the day survives a restart because it is written in the file.
///
/// ⚠ **Повозку сменила задача T-53a-5** (решение 118.5): парой «Что нового» + «Спасибо»
/// правило больше не покажешь. Пара теперь «Новость» + «Спасибо» — обе причитаются в один
/// день, и приоритет FR-101 берёт первую. Правда теста прежняя: **день тратится одним
/// письмом, и на другой день приходит следующее**.
#[test]
fn only_one_letter_a_day_reaches_the_screen() {
    let mut state = settled("0.39.0");
    state.thanks_due = Some(day(2026, 9, 1));
    let today = day(2026, 9, 3);
    let items = [news(12)];
    let feed = FeedView {
        update: None,
        news: &items,
    };

    // Два письма причитаются разом: «Новость» и «Спасибо». Приоритет FR-101 берёт первое.
    let first = letters::due(&state, today, "0.39.0", feed, true);
    assert_eq!(first, Some(Letter::News(12)));

    letters::after_shown(&mut state, Letter::News(12), today, "0.39.0");
    assert_eq!(state.last_letter, Some(today));

    // The second one waits for tomorrow — and this is the assertion that needs the day to be in
    // the file: the state has been written and read back in between on a real machine.
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        None,
        "«Спасибо» не идёт следом за «Новостью» в тот же день"
    );
    assert_eq!(
        letters::due(&state, today.plus_days(1), "0.39.0", FeedView::EMPTY, true),
        Some(Letter::Thanks)
    );
}

/// The priority of FR-101, all the letters that can be due at the same moment.
///
/// ⚠⚠ **ПЕРЕВЁРНУТ задачей T-53a-5, решение владельца 118.5.** До неё первым в этом порядке
/// шло «Что нового» — оно и стоит в имени теста, и имя оставлено нарочно: по нему видно, чего
/// в порядке больше нет. Ворота письма закрыты, порядок из четырёх стал порядком из трёх, а
/// вторая, третья и четвёртая ступени — «Обновление» → «Новость» → «Спасибо» — целы и
/// проверяются ниже слово в слово, как проверялись. ⭐ Это и есть контроль (б) приёмки
/// T-53a-5: закрыты **ворота одного письма**, а не письма вообще.
#[test]
fn the_priority_of_fr_101_is_whats_new_update_news_thanks() {
    let mut state = settled("0.38.0");
    state.thanks_due = Some(day(2026, 9, 1));
    let today = day(2026, 9, 3);
    // TZ E104 §0 п. 20: «Обновление» is the fallback of the updater — the menu entry stands from
    // the seventh day; the order of the letters is what this test is about (T-119-9: without it).
    state.update_noticed = Some(today.plus_days(-letters::UPDATE_FALLBACK_DAYS));
    let items = [news(12)];
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &items,
    };

    // ⛔ Ступень, которой не стало: `last_seen_version` = «0.38.0» при сборке «0.39.0» — ровно
    // то состояние, что прежде давало письмо «Что нового» первым (решение 118.5 закрыло его,
    // 208.94 убрало).
    //
    // ⛔ И вторая ступень, которой не стало, — **«Обновление», задача T-119-9** (решения 208.98,
    // 208.99): письмо само не приходит вовсе, даже на седьмой день и с неизвестной ещё версией —
    // его открывает только сам человек. Первой идёт «Новость», и это проверяется здесь же.
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::News(12)),
        "решения 118.5 и 208.98: ни «Что нового», ни «Обновление» сами не приходят — первой идёт \
         «Новость»"
    );

    state.last_seen_version = "0.39.0".to_owned();
    assert_ne!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::Update),
        "the version they could be running is not a letter of its own accord"
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

/// ⚠⚠ **ПЕРЕВЁРНУТ задачей T-53a-5 (решение 118.5: ворота закрыты), письмо убрано задачей
/// T-119-8 (решение 208.94 п. 4).**
///
/// Письма «Что нового» нет. **Метка версии по-прежнему движется** — на всяком старте, её ведёт
/// `initialise`: по ней `was_updated` узнаёт первый старт после обновления.
#[test]
fn whats_new_is_not_shown_but_the_version_mark_keeps_moving() {
    let mut state = settled("0.38.0");
    let today = day(2026, 9, 3);

    // Ворота: состояние ровно то, что прежде давало письмо, — и письма нет.
    assert_eq!(
        letters::due(&state, today, "0.39.0", FeedView::EMPTY, true),
        None,
        "решение 118.5: непросмотренная версия письма «Что нового» больше не даёт"
    );

    // ⭐ Метка движется — и её двигает `initialise`, на всяком старте, а не показ письма.
    assert!(
        letters::initialise(&mut state, today, "0.39.0"),
        "подвинутая версия обязана считаться записью — иначе файл не сохранится"
    );
    assert_eq!(
        state.last_seen_version, "0.39.0",
        "иначе отметка застынет на версии закрытия ворот"
    );
    assert!(
        !letters::initialise(&mut state, today, "0.39.0"),
        "и второй раз на той же версии писать нечего"
    );

    // Следующая версия двигает её снова.
    let later = today.plus_days(2);
    assert!(letters::initialise(&mut state, later, "0.40.0"));
    assert_eq!(state.last_seen_version, "0.40.0");
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

    // ⭐ Решение 192.1 (stage Э105, посылка П4): 1.0.0 follows 0.104.0, and the installed
    // 0.88…0.104 and their updater compare the same way; then the ordinary rule.
    assert!(letters::version_is_newer("1.0.0", "0.104.0"));
    assert!(letters::version_is_newer("1.0.0", "0.88.0"));
    assert!(!letters::version_is_newer("1.0.0", "1.0.0"));
    assert!(!letters::version_is_newer("0.104.0", "1.0.0"));
    assert!(letters::version_is_newer("1.0.1", "1.0.0"));
    assert!(letters::version_is_newer("1.1.0", "1.0.9"));
}

/// **The letter «Обновление» never comes by itself, and the menu entry outlives the letter** —
/// task T-119-9 (решения 208.98, 208.99): no balloon, no window of its own accord, even with the
/// fallback open and the version not yet announced; a letter the person opened from the menu is
/// still remembered as announced.
#[test]
fn the_update_letter_is_shown_once_for_each_announced_version() {
    let mut state = settled("0.39.0");
    let today = day(2026, 9, 8);
    // The fallback of TZ E104 §0 п. 20 is open: the feed named 0.40.0 seven days ago.
    state.update_noticed = Some(day(2026, 9, 1));
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    assert!(letters::update_fallback_is_open(&state, today));
    assert!(letters::update_is_due(&state, "0.39.0", feed));
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        None,
        "решение 208.98: «Обновление» само не открывается"
    );
    assert_eq!(
        letters::toast_strings(Letter::Update),
        None,
        "и не стучится — 208.99 п. 1"
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
// The updater's fallback and the balloon after an update — TZ E104 §0 п. 20, 21 (T-104-7)
// =========================================================================================

/// **No door to the letter «Обновление» until the feed has named the newer version for seven
/// days** — §0 п. 20, **seven since task T-119-9** (решение 208.98; three until then): the updater
/// installs in the next idle moment, and the letter is its fallback. The day is the first read
/// that named it ([`letters::notice_update`]), kept while the feed goes on naming it and cleared
/// by a read that does not. The doors are the menu entry and the row of «Последние письма»; the
/// letter itself is never due on its own.
#[test]
fn the_update_letter_waits_seven_days_for_the_updater() {
    let mut state = settled("0.103.0");
    let first = day(2026, 10, 24);
    let announced = update("0.104.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    assert!(letters::notice_update(&mut state, "0.103.0", feed, first));
    assert_eq!(
        state.update_noticed,
        Some(first),
        "the first read that named it"
    );
    assert!(
        !letters::notice_update(&mut state, "0.103.0", feed, first.plus_days(1)),
        "a later read keeps the day of the first"
    );
    assert_eq!(state.update_noticed, Some(first));

    // ⚠ The days are written out and not taken from the constant: the first mutant «zero days»
    // passed a loop over `0..UPDATE_FALLBACK_DAYS` (scratchpad-E104\t7-mutants.log). Seven is the
    // number of решение 208.98, and the test holds it.
    assert_eq!(
        letters::UPDATE_FALLBACK_DAYS,
        7,
        "решение 208.98: seven days"
    );

    for offset in 0..7 {
        assert!(
            !letters::update_fallback_is_open(&state, first.plus_days(offset)),
            "day {offset}: the updater still has its chance"
        );
        assert!(
            letters::list_entries(&state, Some(first.plus_days(offset)), "0.103.0", feed)
                .is_empty(),
            "day {offset}: and «Последние письма» say nothing of it"
        );
    }

    let seventh = first.plus_days(7);

    assert!(letters::update_fallback_is_open(&state, seventh));
    assert_eq!(
        letters::list_entries(&state, Some(seventh), "0.103.0", feed).len(),
        1,
        "seven days and still the old build — the row of the fallback"
    );

    for offset in 0..=10 {
        assert_ne!(
            letters::due(&state, first.plus_days(offset), "0.103.0", feed, true),
            Some(Letter::Update),
            "day {offset}: the letter never comes by itself (T-119-9)"
        );
    }

    // The updater did its work: the build is the version the feed names, and nothing is pending.
    assert!(letters::notice_update(&mut state, "0.104.0", feed, seventh));
    assert_eq!(
        state.update_noticed, None,
        "a read that names nothing newer clears it"
    );
    assert!(!letters::update_fallback_is_open(
        &state,
        seventh.plus_days(10)
    ));
}

/// **The balloon «обновлён до X» comes once per version, after an update and only then** — §0
/// п. 21: not on the first run (nothing was seen), not on a start of the same version, not after
/// going back to an older build, and not twice — `initialise` moves the version seen.
#[test]
fn the_balloon_after_an_update_comes_once_per_version() {
    let today = day(2026, 10, 25);

    let fresh = Letters::default();
    assert!(
        !letters::was_updated(&fresh, "0.104.0"),
        "the first run is not an update"
    );

    let mut state = settled("0.103.0");
    assert!(
        letters::was_updated(&state, "0.104.0"),
        "0.103.0 → 0.104.0 is an update"
    );
    assert!(
        !letters::was_updated(&state, "0.103.0"),
        "the same version is not"
    );
    assert!(
        !letters::was_updated(&state, "0.102.0"),
        "a rollback to an older build is not an update"
    );

    letters::initialise(&mut state, today, "0.104.0");
    assert!(
        !letters::was_updated(&state, "0.104.0"),
        "once per version: the tick that knocked has moved the version seen"
    );
}

/// **The tick asks whether it was an update before `initialise` moves the version, and the first
/// tick after an update gives no letter** — the order of the code lines of `tick`, as
/// `the_attempt_is_written_before_the_read` holds the feed's. ⛔ The balloon «обновлён до X» it
/// knocked with is gone (решение 208.94 п. 4, T-119-8).
#[test]
fn the_tick_asks_about_the_update_before_it_moves_the_version_and_knocks_once() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/letters.rs"),
    )
    .expect("src/letters.rs must be readable");

    let start = source.find("pub fn tick(owner: HWND) {").expect("the tick");
    let body = &source[start
        ..start
            + source[start..]
                .find("\n}\n")
                .or_else(|| source[start..].find("\n}\r\n"))
                .expect("its end")];
    let at = |needle: &str| {
        body.find(needle)
            .unwrap_or_else(|| panic!("tick has no «{needle}»"))
    };

    assert!(
        at("let updated = was_updated(&stored, &version);")
            < at("if initialise(&mut stored, today, &version)")
    );
    assert!(
        at("if updated {") < at("let due = with_feed("),
        "the update comes before any letter"
    );
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
        "«Напомнить позже» left it unread"
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

/// **Once a day** — решение 189.2 (вопрос 189, task T-104-3), once in fifteen days until `e104` —
/// the first read on the first run, and nothing at all when the feed is off.
///
/// A day and not fifteen because the feed now carries the updates the updater installs by itself
/// (TZ E104 §0 п. 2): a version the person learns of a fortnight late is a fortnight of the old
/// one. The day of a read is a calendar day, so the next read is the next day whatever the hour.
#[test]
fn the_feed_is_read_once_a_day_and_not_at_all_when_it_is_off() {
    let mut state = settled("0.39.0");
    let today = day(2026, 9, 3);

    assert!(
        letters::feed_read_is_due(&state, today),
        "a machine that has never read the feed reads it now"
    );

    state.feed_last_read = Some(today);
    assert!(
        !letters::feed_read_is_due(&state, today),
        "a feed read today is not read again today"
    );
    assert!(
        letters::feed_read_is_due(&state, today.plus_days(1)),
        "a feed read yesterday is due today — once a day, решение 189.2"
    );

    assert_eq!(letters::days_until_feed_read(&state, today), 1);
    assert_eq!(letters::days_until_feed_read(&state, today.plus_days(1)), 0);
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

/// **A feed asked today is not asked again before tomorrow** — task T-88-1, решение 148.8.
///
/// A read that fails leaves `feed_last_read` where it was, so the feed stays due. Until `e88`
/// that meant a new request at every tick of the letters — once an hour, for as long as the
/// host did not answer — and the placeholder address hid it, because a placeholder is never
/// read at all. `feed_last_try` is the day the feed was last asked, whatever came of it, and
/// one attempt a day is all a feed gets; a good read closes it for the day, and since решение
/// 189.2 a day is also the whole of the period — the two rules now say the same thing on a good
/// day and differ only on a bad one.
#[test]
fn a_feed_asked_today_is_not_asked_again_before_tomorrow() {
    let mut state = settled("0.88.0");
    let today = day(2026, 9, 25);

    // The machine of the owner on the day of `e88`: never read, never asked.
    assert!(letters::feed_read_is_due(&state, today));

    // Asked today, and nothing came back: no second request today, the next one tomorrow.
    state.feed_last_try = Some(today);
    assert!(
        !letters::feed_read_is_due(&state, today),
        "one attempt a day — the next tick of the letters must not ask again"
    );
    assert!(letters::feed_read_is_due(&state, today.plus_days(1)));

    // A good read closes the feed until the next day, whatever day the last attempt was on.
    state.feed_last_read = Some(today);
    state.feed_last_try = Some(day(2026, 9, 24));
    assert!(!letters::feed_read_is_due(&state, today));
    assert!(letters::feed_read_is_due(&state, today.plus_days(1)));
    state.feed_last_try = Some(today);

    // Overdue and already asked today: the panel says one day, because the next attempt is
    // tomorrow; asked yesterday, it says zero — the read is due now.
    state.feed_last_read = Some(day(2026, 9, 1));
    assert_eq!(
        letters::days_until_feed_read(&state, today),
        1,
        "a read that was tried today and failed waits for tomorrow"
    );
    state.feed_last_try = Some(day(2026, 9, 24));
    assert_eq!(letters::days_until_feed_read(&state, today), 0);
    assert!(letters::feed_read_is_due(&state, today));

    // And the switch still wins over everything.
    state.feed = false;
    assert!(!letters::feed_read_is_due(&state, today));
}

/// Whether the branch of `tick` that starts the read writes the day of the attempt **first** —
/// the due check, then the write, then the start, in that order among the code lines.
fn the_attempt_is_written_before_the_read(body: &str) -> bool {
    let lines: Vec<&str> = code_lines(body).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));

    matches!(
        (
            at("if feed_read_is_due("),
            at("feed_last_try = Some(today)"),
            at("start_feed_read("),
        ),
        (Some(due), Some(written), Some(started)) if due < written && written < started
    )
}

/// **The tick writes the day of the attempt before it starts the read** — task T-88-1, решение
/// 148.8.
///
/// `tick` needs the tray, the configuration and a window, so this is a sweep over its body, the
/// road Э70 took for the same reason. Written after the start, the day would be lost with a
/// read that never came back — a thread that could not start, a program closed mid-request —
/// and the next tick would ask again.
///
/// ⚠ Отрицательный контроль — **the same predicate** over the branch `e87` carried, which starts
/// the read and writes nothing, and over the same write put after the start.
#[test]
fn the_tick_writes_the_day_of_the_attempt_before_it_starts_the_read() {
    let source = letters_source();
    let body = body_of(&source, "pub fn tick(");

    assert!(
        the_attempt_is_written_before_the_read(body),
        "tick must write feed_last_try between the due check and the start of the read:\n{body}"
    );

    let e87 = "    if feed_read_is_due(&stored, today) {\n\
               \x20       start_feed_read(owner, settings::ui_language().tag().to_owned());\n\
               \x20   }\n";
    assert!(
        !the_attempt_is_written_before_the_read(e87),
        "the sweep must refuse the branch of e87, which writes nothing"
    );

    let after = "    if feed_read_is_due(&stored, today) {\n\
                 \x20       start_feed_read(owner, settings::ui_language().tag().to_owned());\n\
                 \x20       update_state(|state| state.feed_last_try = Some(today));\n\
                 \x20   }\n";
    assert!(
        !the_attempt_is_written_before_the_read(after),
        "and the write put after the start"
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

/// While an address is a placeholder, the button that would open it is disabled.
///
/// ⚠ This test is written so that it goes on being true when the real addresses arrive: it
/// asserts the *rule*, not the placeholder — and it stayed true through both days they arrived
/// (tasks T-78-1 and T-88-1). The assertions about the state of the constants were the ones
/// edited on those days, which is exactly what they were for; they live in
/// `the_hard_wired_addresses_are_real_and_so_is_the_feed`.
#[test]
fn a_placeholder_address_is_recognised_by_its_reserved_domain() {
    assert!(links::is_placeholder("https://example.invalid/news.toml"));
    assert!(links::is_placeholder("https://example.invalid/channel"));
    assert!(!links::is_placeholder("https://t.me/lang_switcher"));
    assert!(!links::is_placeholder("https://example.com/support"));
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

/// **The hard-wired addresses of решения 139.2 и 147.3 are real — the feed's among them** —
/// tasks T-78-1 and T-88-1.
///
/// ⚠ Until `e88` the second half of this test said the opposite and was named
/// `the_hard_wired_addresses_are_real_and_the_feed_is_not`: `FEED_URLS` stayed a placeholder
/// **on purpose** (138.4), and a guard said so out loud, so that it read as a decision and not
/// as forgetfulness. The user named the address on 2026-09-25 (147.3, 148.1) and the guard
/// turned over with it: all five are real, the program has a feed to read, and the feed's host
/// is one the gate of С52 lets through.
///
/// ⭐ **Seven since task T-103-2** — решение 181.1: the topic of the group and the issues on GitHub,
/// where the wizard of FR-104 and the letter «Спасибо» send a person to write. Both are written
/// down here a second time, apart from the code, as the feed's is: an address that drifted to a
/// placeholder or to another host would still pass the two checks of the loop.
///
/// ⭐ **Nine since task T-103-9** — решение 181.33: the English pairs of the download page and of
/// the program's page; their values are held by `every_language_opens_the_pages_of_the_site_on_its_own_side`.
#[test]
fn the_hard_wired_addresses_are_real_and_so_is_the_feed() {
    for (what, url) in [
        ("CHANNEL_URL", links::CHANNEL_URL),
        ("GROUP_URL", links::GROUP_URL),
        ("ISSUES_URL", links::ISSUES_URL),
        ("SUPPORT_URL", links::SUPPORT_URL),
        ("DOWNLOAD_URL", links::DOWNLOAD_URL),
        ("DOWNLOAD_URL_EN", links::DOWNLOAD_URL_EN),
        ("PROGRAM_URL", links::PROGRAM_URL),
        ("PROGRAM_URL_EN", links::PROGRAM_URL_EN),
    ]
    .into_iter()
    .chain(links::FEED_URLS.map(|url| ("FEED_URLS", url)))
    {
        assert!(
            !links::is_placeholder(url),
            "{what} is still a placeholder: {url}"
        );
        assert!(links::is_allowed(url), "{what} must reach the shell: {url}");
    }

    // Решение 148.1: one address, the author's, and no second road behind it.
    assert_eq!(
        links::FEED_URLS,
        ["https://panda-pishet-kod.dev/langswitcher/news.toml"]
    );
    assert!(
        links::feed_is_configured(),
        "the program has a real feed address to read from"
    );

    // Решение 181.1: the topic of Lang Switcher in the group, and the list of the issues — the
    // address the site and README name.
    assert_eq!(links::GROUP_URL, "https://t.me/panda_koding/45");
    assert_eq!(
        links::ISSUES_URL,
        "https://github.com/panda-pishet-kod/lang-switcher/issues"
    );

    // And the gate did not open on the way: a foreign host is refused as it always was — and so
    // is another host under the name of the one the issues are on: the gate compares hosts whole.
    assert!(!links::is_allowed("https://evil.example/news.toml"));
    assert!(!links::is_allowed("https://gist.github.com/"));
}

/// The code lines of `text` that name the reserved domain, each with its line number.
///
/// Comments are not code: the module `links` names the domain in its doc to say why it was
/// chosen, and a sweep that counted that sentence would be red for a sentence.
fn lines_naming_the_reserved_domain(text: &str) -> Vec<(usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.starts_with("//") && line.contains("example.invalid"))
        .collect()
}

/// **No placeholder address is left in the program** — task T-88-1.
///
/// A sweep over every code line of `src\`: the reserved domain may be named in exactly one
/// place, the definition of `PLACEHOLDER_HOST` that [`links::is_placeholder`] recognises it by.
/// Any other line naming it is a placeholder address back in the build — a feed that is never
/// read, or a button drawn dead.
///
/// ⚠ Отрицательный контроль — **the same predicate** over the lines `ec05521` carried: the
/// definition and the feed's address must both be found, and the doc line beside them must not.
/// Without it the sweep could be green by reading nothing.
#[test]
fn no_placeholder_address_is_left_in_the_program() {
    const DEFINITION: &str = r#"const PLACEHOLDER_HOST: &str = "example.invalid";"#;

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut read = 0;
    let mut found = Vec::new();

    for entry in std::fs::read_dir(&src).expect("src must be readable") {
        let path = entry.expect("an entry of src").path();

        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }

        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        read += 1;
        found.extend(
            lines_naming_the_reserved_domain(&text)
                .into_iter()
                .map(|(number, line)| format!("{name}:{number}: {line}")),
        );
    }

    assert!(
        found.len() == 1 && found[0].starts_with("links.rs:") && found[0].ends_with(DEFINITION),
        "in {read} files of src the reserved domain must be named by one line of code, its \
         definition, and by no address: {found:#?}"
    );

    // Отрицательный контроль: the same predicate over the text of `ec05521`.
    let base = "/// `example.invalid` and not a made-up host: RFC 2606 reserves `.invalid`\n\
                const PLACEHOLDER_HOST: &str = \"example.invalid\";\n\
                pub const FEED_URLS: [&str; 1] = [\"https://example.invalid/news.toml\"];\n";

    assert_eq!(
        lines_naming_the_reserved_domain(base)
            .iter()
            .map(|(number, _)| *number)
            .collect::<Vec<_>>(),
        [2, 3],
        "the sweep must find both lines of ec05521 and skip the comment above them"
    );
}

/// **С52, task T-41-1 — the gatekeeper of the one door.** What the shell is handed is `https://`
/// at a host this program carries in `mod links`; everything else is refused.
///
/// The process runs with `uiAccess`, so whatever `ShellExecuteW` starts inherits that pass. The
/// address travels in the feed, the feed is signed, and a signature names the **author** and not
/// the safety of a string: one typo in the TOML is enough. This table is the whole of what may
/// get through, and the refusals below are the finding written out.
#[test]
fn only_https_at_a_host_of_this_program_reaches_the_shell() {
    let refused = [
        // Nothing at all.
        "",
        // The examples the finding names.
        "file:///C:/Windows/System32/cmd.exe",
        "ms-settings:privacy",
        "http://example.com",
        "http://panda-pishet-kod.dev/langswitcher/news.toml",
        "javascript:alert(1)",
        r"\\сервер\общая\пуск.exe",
        r"C:\Windows\System32\cmd.exe",
        "https://злой.example.org/",
        // A foreign host — the reason a white list exists at all.
        "https://example.com/news.toml",
        // ⚠ Task T-88-1, решение 148.7: the reserved domain was a host of this program for as
        // long as the feed's address was a placeholder on it, and it left with the placeholder.
        // Every example below that needs «our host» names the author's real one since.
        "https://example.invalid/news.toml",
        // A host that merely **ends** with ours, and one that merely starts with it.
        "https://evil-panda-pishet-kod.dev/",
        "https://panda-pishet-kod.dev.evil.example/",
        // Credentials and a port: the reading of the host must not be talked into answering
        // the left half of either.
        "https://panda-pishet-kod.dev@evil.example/",
        "https://panda-pishet-kod.dev:8080/",
        // A backslash is a slash to every browser, so it ends the host here too — otherwise
        // `evil.example` would be read as one long host and refused for the wrong reason,
        // and a browser would go somewhere this program never allowed.
        r"https://evil.example\panda-pishet-kod.dev/",
        // Degenerate shapes of the scheme itself.
        "https://",
        "https:/panda-pishet-kod.dev/",
        "https:panda-pishet-kod.dev",
    ];

    for url in refused {
        assert!(!links::is_allowed(url), "{url} must not reach the shell");
    }

    let allowed = [
        // The three addresses of this program — «Открыть канал» and «Открыть страницу
        // поддержки» take theirs from constants and go through the same gate, which is what
        // makes the door one door.
        links::FEED_URLS[0],
        links::CHANNEL_URL,
        links::SUPPORT_URL,
        // The hard-wired host, and the same host with a path, a query and a fragment under it.
        "https://panda-pishet-kod.dev/langswitcher/news.toml",
        "https://panda-pishet-kod.dev/a/b/c?d=e#f",
        // A scheme is a scheme in whatever case it is written, and so is a host name.
        "HTTPS://PANDA-PISHET-KOD.DEV/langswitcher/news.toml",
    ];

    for url in allowed {
        assert!(
            links::is_allowed(url),
            "{url} is an address of this program"
        );
    }
}

// =========================================================================================
// Where a person writes to the author — решение 181.1, задача T-103-2
// =========================================================================================

/// The two addresses of решение 181.1, written down **apart from the code**: the topic of the
/// group and the issues on GitHub.
const THE_GROUP: &str = "https://t.me/panda_koding/45";
const THE_ISSUES: &str = "https://github.com/panda-pishet-kod/lang-switcher/issues";

/// The languages that read the Russian side — решение 181.1 (Russian, Ukrainian) and, since stage
/// E108, решение 202.6 (Kazakh, Belarusian, Tatar, Uzbek in Cyrillic: «Делаем так, пока нет
/// адаптированных страниц сайта под каждый из языков»).
const THE_RUSSIAN_SIDE: [Language; 6] = [
    Language::Ru,
    Language::Uk,
    Language::Kk,
    Language::Be,
    Language::Tt,
    Language::UzCyrl,
];

/// The address of решения 181.1 и 202.6 for every value of `general.language`, in the order of
/// `Language::ALL`: the topic of the group on the Russian side, the issues on GitHub on the other.
/// Until stage E108 fourteen rows written out; the side table above is the same fact for forty-four.
fn feedback_of_decision_181_1() -> Vec<(Language, &'static str)> {
    Language::ALL
        .into_iter()
        .map(|language| {
            if THE_RUSSIAN_SIDE.contains(&language) {
                (language, THE_GROUP)
            } else {
                (language, THE_ISSUES)
            }
        })
        .collect()
}

/// The languages whose address in `canon` is not the one [`links::feedback_url`] answers.
fn feedback_mismatches(canon: &[(Language, &str)]) -> Vec<Language> {
    canon
        .iter()
        .filter(|(language, url)| links::feedback_url(*language) != *url)
        .map(|(language, _)| *language)
        .collect()
}

/// **Задача T-103-2, сторож (1), решение 181.1 — a person writes to the author where the
/// language they read the program in says**: the topic of the group in Russian and Ukrainian, the
/// issues on GitHub in the other twelve. Until `e102` both the wizard and «Спасибо» sent everybody
/// to the channel, where there is nowhere to write (вопрос 181).
///
/// ⭐ **Stage E108, решение 202.6:** the forty-four values, the four of group A that read the
/// Russian side among them.
///
/// ⚠ Control: the canon with Ukrainian sent to GitHub Issues is caught, and only on Ukrainian.
#[test]
fn every_language_writes_to_the_author_where_decision_181_1_says() {
    let canon = feedback_of_decision_181_1();

    assert_eq!(
        (THE_GROUP, THE_ISSUES),
        (links::GROUP_URL, links::ISSUES_URL),
        "the two addresses are the ones of 181.1"
    );

    assert_eq!(
        feedback_mismatches(&canon),
        Vec::<Language>::new(),
        "181.1, 202.6: the address of these languages is not the one the decisions give them"
    );

    let to_the_group: Vec<Language> = Language::ALL
        .into_iter()
        .filter(|language| links::feedback_url(*language) == links::GROUP_URL)
        .collect();

    assert!(
        to_the_group.len() == THE_RUSSIAN_SIDE.len()
            && to_the_group.iter().all(|l| THE_RUSSIAN_SIDE.contains(l)),
        "the group is for the six languages that read it and for no other: {to_the_group:?}"
    );

    let mut spoilt = canon;
    spoilt[2].1 = links::ISSUES_URL;

    assert_eq!(
        feedback_mismatches(&spoilt),
        [Language::Uk],
        "the control: a canon that sends Ukrainian to GitHub Issues must be caught there"
    );
}

/// The two pages of the site — `(download, program)` — on the Russian side, written down apart from
/// the code (решение 181.33), each with the mark of its language for the site (решение 181.35).
const RUSSIAN_PAGES: (&str, &str) = (
    "https://panda-pishet-kod.dev/langswitcher/download/?lang=ru",
    "https://panda-pishet-kod.dev/langswitcher/?lang=ru",
);

/// The same two pages on the English side — their pairs under `/en/` (решение 181.33), marked
/// `en` (решение 181.35).
const ENGLISH_PAGES: (&str, &str) = (
    "https://panda-pishet-kod.dev/en/langswitcher/download/?lang=en",
    "https://panda-pishet-kod.dev/en/langswitcher/?lang=en",
);

/// `url` without its query — the page the site serves, whatever the mark says.
fn page_of(url: &str) -> &str {
    url.split_once('?').map_or(url, |(page, _)| page)
}

/// The pages решение 181.33 gives `language`: the Russian ones on the Russian side (Russian,
/// Ukrainian and, since stage E108, the four of решение 202.6), the English ones on the other.
fn pages_of_decision_181_33(language: Language) -> (&'static str, &'static str) {
    // Stage E114, task T-114-3 (решение 208.6, 1а): hy and uz read the Russian pages too.
    if THE_RUSSIAN_SIDE.contains(&language) || THE_RUSSIAN_PAGES_TOO.contains(&language) {
        RUSSIAN_PAGES
    } else {
        ENGLISH_PAGES
    }
}

/// The languages on a side whose pages by `rule` are not the ones решение 181.33 gives them.
fn pages_mismatches(rule: impl Fn(Language) -> (&'static str, &'static str)) -> Vec<Language> {
    the_languages_on_a_side()
        .into_iter()
        .filter(|language| rule(*language) != pages_of_decision_181_33(*language))
        .collect()
}

/// **Задача T-103-9, сторож (1), решение 181.33 — the two pages of the site open in the language a
/// person reads the program in**: «Открыть страницу загрузки» and the name of the program open the
/// Russian pages in Russian and Ukrainian and their pairs under `/en/` in the other twelve — the
/// same side as where the person writes (181.1). Until the third build of `e103` both opened the
/// Russian pages in all fourteen languages (находка 181.32).
///
/// ⭐ **And each address tells the site its language** — решение 181.35: the site picks the language
/// of a page by the choice made with its own button, or else by the language of the browser
/// (поправка 181.34), so the page alone would not decide it. `?lang=ru` on the Russian side,
/// `?lang=en` on the English one — the name and the values the site already uses for that word.
///
/// ⭐ **Stage E112, Q-206-1:** Ukrainian, Arabic and Persian left the two sides for pages of their
/// own, held by `all_sixty_six_languages_open_what_the_decisions_give_them`; this holds the rest.
///
/// ⚠ Controls: the rule of the third build — the Russian pages for every language — is caught on
/// exactly the twelve of the English side; the addresses of the fourth build — the right pages with
/// no mark — are caught in all fourteen.
#[test]
fn every_language_opens_the_pages_of_the_site_on_its_own_side() {
    assert_eq!(
        pages_mismatches(|language| (links::download_url(language), links::program_url(language))),
        Vec::<Language>::new(),
        "181.33: the pages of these languages are not the ones the decision gives them"
    );

    // One side for writing and for reading: the pages follow the line 181.1 drew.
    let off_the_side_of_181_1: Vec<Language> = the_languages_on_a_side()
        .into_iter()
        .filter(|language| {
            let russian = links::feedback_url(*language) == links::GROUP_URL;
            (links::download_url(*language) == links::DOWNLOAD_URL) != russian
                || (links::program_url(*language) == links::PROGRAM_URL) != russian
        })
        .collect();

    // ⭐ Stage E114, task T-114-3 (решение 208.6, 1а): Armenian and Uzbek in the Latin script are the
    // one deliberate split — the Russian pages, the issues on GitHub. Nobody else.
    assert_eq!(
        off_the_side_of_181_1,
        THE_RUSSIAN_PAGES_TOO.to_vec(),
        "a person reads the site on the side they write to the author on"
    );

    // The constants are the pages, and the English one is the Russian one under `/en/` — the pair
    // the site makes.
    assert_eq!((links::DOWNLOAD_URL, links::PROGRAM_URL), RUSSIAN_PAGES);
    assert_eq!(
        (links::DOWNLOAD_URL_EN, links::PROGRAM_URL_EN),
        ENGLISH_PAGES
    );

    for (russian, english) in [
        (RUSSIAN_PAGES.0, ENGLISH_PAGES.0),
        (RUSSIAN_PAGES.1, ENGLISH_PAGES.1),
    ] {
        assert_eq!(
            page_of(russian).replacen("panda-pishet-kod.dev/", "panda-pishet-kod.dev/en/", 1),
            page_of(english),
            "the English page is the pair of the Russian one under /en/"
        );
        assert!(
            russian.ends_with("/?lang=ru") && english.ends_with("/?lang=en"),
            "181.35: each address carries the mark of its own side and nothing after it"
        );
        assert!(links::is_allowed(russian), "{russian} must reach the shell");
        assert!(links::is_allowed(english), "{english} must reach the shell");
    }

    // Stage E114, task T-114-3: the English pages are read by the English side less hy and uz,
    // whose pages are the Russian ones (решение 208.6, 1а).
    let english_side: Vec<Language> = the_languages_on_a_side()
        .into_iter()
        .filter(|language| {
            !THE_RUSSIAN_SIDE.contains(language) && !THE_RUSSIAN_PAGES_TOO.contains(language)
        })
        .collect();

    assert_eq!(
        pages_mismatches(|_| RUSSIAN_PAGES),
        english_side,
        "the control: the Russian pages for every language must be caught on the English pages"
    );
    assert_eq!(
        pages_mismatches(|language| {
            let (download, program) = pages_of_decision_181_33(language);
            (page_of(download), page_of(program))
        }),
        the_languages_on_a_side(),
        "the control: the right pages with no mark must be caught in every language"
    );
}

/// The languages the site has pages of their own for — Q-206-1, the signals of the controller of the
/// languages of the site (site 1.23.0) — with the code the site gives each. Written down apart from
/// the table of `links`: `he` comes next, and it does not go out without a line from that
/// controller. In the order of `Language::ALL`: the controls below compare against it as a list.
///
/// ⭐ Stage E114, task T-114-3 (Q-208-1…4, Q-208-6, Q-208-8, решения 208.6 и 208.8): fourteen more,
/// every address checked by the controller (200, the page's own `lang`). Both Serbian scripts read
/// `/sr/` and are marked `sr` (the page is `lang="sr-Latn"`); Portuguese is marked `pt` (the page is
/// `lang="pt-BR"`). Hungarian after them (208.1, the line of the controller of 2026-11-10: both
/// addresses 200, `lang=hu`). ⭐ Stage E116, task T-116-6 (Q-208-13 п. 1): Swedish and Slovak — the
/// line of the controller of the languages of the site of 2026-11-12, and all four addresses checked
/// in Chrome on 2026-11-14 (200, `lang=sv` and `lang=sk`).
const THE_OWN_PAGES: [(Language, &str); 21] = [
    (Language::Uk, "uk"),
    (Language::De, "de"),
    (Language::Fr, "fr"),
    (Language::Pt, "pt"),
    (Language::It, "it"),
    (Language::Pl, "pl"),
    (Language::Cs, "cs"),
    (Language::Tr, "tr"),
    (Language::El, "el"),
    (Language::He, "he"),
    (Language::Ar, "ar"),
    (Language::Fa, "fa"),
    (Language::Th, "th"),
    (Language::Az, "az"),
    (Language::Sr, "sr"),
    (Language::SrLatn, "sr"),
    (Language::Bg, "bg"),
    (Language::Nl, "nl"),
    (Language::Hu, "hu"),
    (Language::Sv, "sv"),
    (Language::Sk, "sk"),
];

/// ⭐ Stage E114, task T-114-3 (решение 208.6, 1а; Q-208-5): Armenian and Uzbek in the Latin script
/// read the **pages** of the Russian side and go on writing where they wrote — the issues on GitHub.
/// The pages and the channel are two decisions, as in task T-112-1.
const THE_RUSSIAN_PAGES_TOO: [Language; 2] = [Language::Uz, Language::Hy];

/// The languages that read one of the two sides of the site — every row of the table of languages
/// but [`THE_OWN_PAGES`].
fn the_languages_on_a_side() -> Vec<Language> {
    Language::ALL
        .into_iter()
        .filter(|language| !THE_OWN_PAGES.iter().any(|(own, _)| own == language))
        .collect()
}

/// The page «What's new» of a language with pages of its own: it has none yet, so the English one,
/// **with no mark** — the mark outranks the browser and the site remembers it, so `?lang=en` would
/// take the reader off their language (Q-206-1).
const THE_UNMARKED_CHANGELOG: &str = "https://panda-pishet-kod.dev/en/langswitcher/changelog/";

/// What `language` opens and where it writes: `(download, program, changelog, feedback)`, by the
/// decisions and apart from the code. Pages of its own first (Q-206-1); otherwise the side of
/// решения 181.1, 181.33 и 202.6. Where a person writes does not move with the pages.
fn the_addresses_of(language: Language) -> (String, String, String, &'static str) {
    let feedback = if THE_RUSSIAN_SIDE.contains(&language) {
        THE_GROUP
    } else {
        THE_ISSUES
    };

    if let Some((_, code)) = THE_OWN_PAGES.iter().find(|(own, _)| *own == language) {
        return (
            format!("https://panda-pishet-kod.dev/{code}/langswitcher/download/?lang={code}"),
            format!("https://panda-pishet-kod.dev/{code}/langswitcher/?lang={code}"),
            THE_UNMARKED_CHANGELOG.to_owned(),
            feedback,
        );
    }

    let (root, code) =
        if THE_RUSSIAN_SIDE.contains(&language) || THE_RUSSIAN_PAGES_TOO.contains(&language) {
            ("", "ru")
        } else {
            ("en/", "en")
        };

    (
        format!("https://panda-pishet-kod.dev/{root}langswitcher/download/?lang={code}"),
        format!("https://panda-pishet-kod.dev/{root}langswitcher/?lang={code}"),
        format!("https://panda-pishet-kod.dev/{root}langswitcher/changelog/?lang={code}"),
        feedback,
    )
}

/// The languages whose four addresses by `rule` are not the ones [`the_addresses_of`] gives them.
fn addresses_mismatches(
    rule: impl Fn(Language) -> (String, String, String, &'static str),
) -> Vec<Language> {
    Language::ALL
        .into_iter()
        .filter(|language| rule(*language) != the_addresses_of(*language))
        .collect()
}

/// What `links` answers for `language`, in the order of [`the_addresses_of`].
fn the_addresses_of_links(language: Language) -> (String, String, String, &'static str) {
    (
        links::download_url(language).to_owned(),
        links::program_url(language).to_owned(),
        links::changelog_url(language).to_owned(),
        links::feedback_url(language),
    )
}

/// **Задача T-112-1, Q-206-1 — Ukrainian opens the three pages the site made for it**, and not the
/// Russian ones it opened until 1.4.2. The address of its feedback stays the group.
#[test]
fn ukrainian_opens_the_pages_the_site_made_for_it() {
    assert_eq!(
        links::download_url(Language::Uk),
        "https://panda-pishet-kod.dev/uk/langswitcher/download/?lang=uk"
    );
    assert_eq!(
        links::program_url(Language::Uk),
        "https://panda-pishet-kod.dev/uk/langswitcher/?lang=uk"
    );
    assert_eq!(
        links::changelog_url(Language::Uk),
        "https://panda-pishet-kod.dev/en/langswitcher/changelog/"
    );
    assert_eq!(links::feedback_url(Language::Uk), THE_GROUP);
}

/// **Задача T-113-3 — Thai opens the two pages the site made for it** (line of the controller of the
/// languages of the site; both answered 200 with `lang="th"` when the controller opened them). Its
/// page «What's new» does not exist yet (404), so the English one with no mark, as for `uk`, `ar`,
/// `fa`. Where a Thai reader writes stays GitHub Issues.
#[test]
fn thai_opens_the_pages_the_site_made_for_it() {
    assert_eq!(
        links::download_url(Language::Th),
        "https://panda-pishet-kod.dev/th/langswitcher/download/?lang=th"
    );
    assert_eq!(
        links::program_url(Language::Th),
        "https://panda-pishet-kod.dev/th/langswitcher/?lang=th"
    );
    assert_eq!(
        links::changelog_url(Language::Th),
        "https://panda-pishet-kod.dev/en/langswitcher/changelog/"
    );
    assert_eq!(links::feedback_url(Language::Th), THE_ISSUES);
}

/// **Задача T-112-1, Q-206-1 — each of the sixty-six rows of the table of languages opens what the
/// decisions give it**, and only `uk`, `fa`, `ar`, `th` moved. A language with no line from the
/// controller of the languages of the site (`he`) must not carry a mark of its own; a language moved off
/// its side by accident shows up here by name (стоп-условие C4).
///
/// ⚠ Controls: the rule of 1.4.2 — the side alone — is caught on exactly the four languages; `he`
/// sent to `?lang=he` ahead of its line is caught on `he` alone; Ukrainian sent to GitHub Issues,
/// the road of the channel changed along with the pages, is caught on `uk` alone.
#[test]
fn all_sixty_six_languages_open_what_the_decisions_give_them() {
    assert_eq!(Language::ALL.len(), 66, "the table of languages has grown");

    assert_eq!(
        addresses_mismatches(the_addresses_of_links),
        Vec::<Language>::new(),
        "Q-206-1: these languages do not open what the decisions give them"
    );

    for language in Language::ALL {
        let (download, program, changelog, feedback) = the_addresses_of_links(language);

        for url in [
            download.as_str(),
            program.as_str(),
            changelog.as_str(),
            feedback,
        ] {
            assert!(!links::is_placeholder(url), "{language:?}: {url}");
            assert!(
                links::is_allowed(url),
                "{language:?}: {url} must reach the shell"
            );
        }
    }

    let the_side_alone = |language: Language| {
        let (root, code) = if THE_RUSSIAN_SIDE.contains(&language) {
            ("", "ru")
        } else {
            ("en/", "en")
        };

        (
            format!("https://panda-pishet-kod.dev/{root}langswitcher/download/?lang={code}"),
            format!("https://panda-pishet-kod.dev/{root}langswitcher/?lang={code}"),
            format!("https://panda-pishet-kod.dev/{root}langswitcher/changelog/?lang={code}"),
            links::feedback_url(language),
        )
    };

    // Stage E114, task T-114-3: the rule of 1.4.2 is caught on the twenty-one languages of their own
    // pages and on hy and uz, whose pages moved to the Russian side — and on no other.
    let moved: Vec<Language> = Language::ALL
        .into_iter()
        .filter(|language| {
            THE_OWN_PAGES.iter().any(|(own, _)| own == language)
                || THE_RUSSIAN_PAGES_TOO.contains(language)
        })
        .collect();

    assert_eq!(
        addresses_mismatches(the_side_alone),
        moved,
        "the control: the rule of 1.4.2 must be caught on these languages and no other"
    );

    assert_eq!(
        addresses_mismatches(|language| {
            let mut addresses = the_addresses_of(language);

            // Stage E114: Hebrew has pages of its own now, so the control takes Hindi, which has none.
            if language == Language::Hi {
                addresses.0 =
                    "https://panda-pishet-kod.dev/hi/langswitcher/download/?lang=hi".to_owned();
            }

            addresses
        }),
        [Language::Hi],
        "the control: a mark ahead of the line from the controller must be caught"
    );

    assert_eq!(
        addresses_mismatches(|language| {
            let mut addresses = the_addresses_of(language);

            if language == Language::Uk {
                addresses.3 = THE_ISSUES;
            }

            addresses
        }),
        [Language::Uk],
        "the control: the channel of Ukrainian moved along with its pages must be caught"
    );
}

/// **Задача T-103-9, сторож (3), решение 181.33 — every road to the two pages opens the page of its
/// own language.** `act` — the one body of the buttons of the letters, of «Последние письма» and
/// of «От автора» (T-120-16) — hands the shell what [`letters::links`] answers for the language of
/// the window it is handed; «О программе» — a window of the shell since FR-92а moved it, whose
/// address under the panda is the road (208.172, as its name was, 139.2 п. 4) — asks the rule with
/// the language of its window, which is the language of the interface; and no code line opens
/// either Russian constant straight any more — the shape of the third build.
///
/// ⚠ Control: the same predicate refuses the two lines of `act` and the one of «О программе» as
/// the third build carried them.
#[test]
fn every_road_to_the_two_pages_opens_the_page_of_its_language() {
    fn opens_by_the_window(act: &str) -> bool {
        act.contains("Action::OpenDownload => open_link(links::download_url(deed.language)),")
            && act.contains("Action::OpenProgram => open_link(links::program_url(deed.language)),")
    }
    fn opens_by_the_interface(about: &str) -> bool {
        about.contains("language: settings::ui_language(),")
            && about.contains(
                "Event::Pressed(Id::Site) => { \
                 crate::letters::open_link(crate::letters::links::program_url(self.language));",
            )
    }
    fn opens_a_constant_straight(code: &str) -> bool {
        code.contains("open_link(links::DOWNLOAD_URL)")
            || code.contains("open_link(links::PROGRAM_URL)")
            || code.contains("open_link(crate::letters::links::PROGRAM_URL)")
    }

    let source = letters_source();
    let about_source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("windows")
            .join("about.rs"),
    )
    .expect("src\\ui\\windows\\about.rs must be readable");
    let perform = squeezed_code(
        letters_function_body(&source, "fn act(")
            .expect("src\\letters.rs must still declare `act`"),
    );
    let about = squeezed_code(&about_source);

    assert!(
        opens_by_the_window(&perform),
        "181.33: «Открыть страницу загрузки» and the name must open the pages of their window:\n\
         {perform}"
    );
    assert!(
        opens_by_the_interface(&about),
        "181.33, 208.172: the address under the panda of «О программе» must open the page of the \
         language of the interface"
    );
    assert!(
        !opens_a_constant_straight(&squeezed_code(&source)) && !opens_a_constant_straight(&about),
        "no road may open the Russian page straight, whatever the language"
    );

    // The controls: the three lines as the third build carried them.
    let third_build = perform
        .replace(
            "Action::OpenDownload => open_link(links::download_url(deed.language)),",
            "Action::OpenDownload => open_link(links::DOWNLOAD_URL),",
        )
        .replace(
            "Action::OpenProgram => open_link(links::program_url(deed.language)),",
            "Action::OpenProgram => open_link(links::PROGRAM_URL),",
        );
    assert_ne!(third_build, perform, "the control must change the body");
    assert!(
        !opens_by_the_window(&third_build) && opens_a_constant_straight(&third_build),
        "the control: `act` in the shape of the third build must be refused"
    );

    let third_about = "open_link(crate::letters::links::PROGRAM_URL);";
    assert!(
        !opens_by_the_interface(third_about) && opens_a_constant_straight(third_about),
        "the control: «О программе» of the third build must be refused"
    );
}

/// The code of the arm of [`letters::plan_for`] that builds `letter`, from its head to the head of
/// the arm after it.
fn plan_arm<'a>(source: &'a str, head: &str, next: &str) -> &'a str {
    let start = source
        .find(head)
        .unwrap_or_else(|| panic!("src\\letters.rs must still build «{head}»"));
    let length = source[start..]
        .find(next)
        .unwrap_or_else(|| panic!("«{next}» must follow «{head}»"));

    &source[start..start + length]
}

/// Whether the arm of «Спасибо» gives its button the action and the address of решение 181.1.
fn the_thanks_sends_a_person_to_write(arm: &str) -> bool {
    let code = squeezed_code(arm);

    code.contains("text(IDS_FEEDBACK_OPEN), Action::OpenFeedback,")
        && code.contains("Action::OpenFeedback, links::feedback_url(language),")
        && !code.contains("CHANNEL_URL")
        && !code.contains("Action::OpenChannel")
        && !code.contains("IDS_CHANNEL_OPEN")
}

/// **Задача T-103-2, сторож (3), решение 181.1 — «Спасибо», which asks a person to write, sends
/// them where they can, and the three that are about the news stay on the channel.**
///
/// The half a pure function cannot show: which address the window's own code hands the shell. So
/// the bodies are read — the arm of «Спасибо» in [`letters::plan_for`] must name
/// [`links::feedback_url`] and not `CHANNEL_URL`; the action of «Спасибо» must open the address of
/// its window; and the «От автора» of FR-103 and the thanks of the wizard (`Id::Channel`) are
/// on the channel as before.
///
/// ⭐ Until stage E107 the wizard's «Скопировать и открыть…» was the second of the two; решение
/// 199.3 took the group and the issues out of the wizard, and `wizard_copy` is gone —
/// `the_letter_copies_its_text_and_opens_nothing_and_the_wizard_opens_no_page` holds that.
///
/// ⚠ Control: «Спасибо» back on «Открыть канал» is caught.
#[test]
fn the_one_that_asks_a_person_to_write_goes_where_they_can_and_the_news_stays_on_the_channel() {
    let source = letters_module_source();

    let thanks = plan_arm(
        &source,
        "Letter::Thanks => LetterPlan {",
        "Letter::Update | Letter::News(_) =>",
    );
    let perform = squeezed_code(
        letters_function_body(&source, "fn act(")
            .expect("src\\letters.rs must still declare `act`"),
    );
    let wizard_thanks = wizard_events();

    assert!(
        the_thanks_sends_a_person_to_write(thanks),
        "181.1: the button of «Спасибо» must send a person where they can write:\n{thanks}"
    );
    assert!(
        perform.contains("Action::OpenFeedback => open_link(links::feedback_url(deed.language)),"),
        "the action of «Спасибо» must open the address of its window:\n{perform}"
    );

    // The news stays where the news is. (The letter «Что нового», which opened the channel too,
    // is gone — решение 208.94 п. 4, T-119-8.)
    assert!(
        perform.contains("Action::OpenChannel => open_link(links::CHANNEL_URL)"),
        "«Открыть канал» opens the channel as before"
    );
    assert_eq!(
        lang_switcher::ui::windows::author::Id::Channel.action(),
        Some(letters::Action::OpenChannel),
        "«Открыть канал» of «От автора» (FR-103) is the channel as before"
    );
    assert!(
        squeezed_code(&wizard_thanks).contains("Event::Pressed(Id::Channel) => {")
            && squeezed_code(&wizard_thanks).contains("Action::OpenChannel"),
        "the thanks of the wizard opens the channel as before:\n{wizard_thanks}"
    );

    // The control.
    let channel_again = thanks
        .replace("IDS_FEEDBACK_OPEN", "IDS_CHANNEL_OPEN")
        .replace("Action::OpenFeedback", "Action::OpenChannel");
    assert_ne!(channel_again, thanks, "the control must change the arm");
    assert!(
        !the_thanks_sends_a_person_to_write(&channel_again),
        "the sweep does not see «Спасибо» back on «Открыть канал»"
    );
}

/// Whether a body of the wizard writes the clipboard and opens nothing — no page, no shell, no
/// `mailto:`.
fn copies_and_opens_nothing(body: &str) -> bool {
    let code = squeezed_code(body);

    code.contains("write_unicode_text(")
        && !code.contains("open_link(")
        && !code.contains("open_in_the_shell(")
        && !code.contains("mailto")
}

/// Whether the code of the wizard's buttons hands the shell nothing but the channel of its thanks —
/// no group, no GitHub Issues, no page of the site (решение 199.3). Since T-120-19 the buttons are
/// the events of `ui::windows::wizard::Product`, and what they open is an action of `letters`.
fn the_wizard_opens_no_page_but_its_channel(body: &str) -> bool {
    let code = squeezed_code(body);
    code.matches("Action::Open").count() == 1
        && code.contains("Action::OpenChannel")
        && !code.contains("open_link(")
        && !code.contains("feedback_url")
        && !code.contains("GROUP_URL")
        && !code.contains("ISSUES_URL")
}

/// The body of a method of an `impl` — from its signature to the brace of its indentation.
fn method_body<'a>(source: &'a str, signature: &str) -> Option<&'a str> {
    let (_, after) = source.split_once(signature)?;

    after.split_once("\n    }\n").map(|(body, _)| body)
}

/// What the window of the wizard does with an event — `Product::event` of `ui::windows::wizard`,
/// a window of the shell since T-120-19.
fn wizard_events() -> String {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("windows")
            .join("wizard.rs"),
    )
    .expect("src\\ui\\windows\\wizard.rs must be readable")
    .replace("\r\n", "\n");
    let (_, product) = source
        .split_once("impl Content for Product {")
        .expect("the window of the wizard is a content of the shell");
    method_body(product, "fn event(")
        .expect("the window of the wizard answers its events")
        .to_owned()
}

/// **Задача T-107-3, решение 199.3 (until 1.2.0 — задача T-103-7, сторож (2), решение 181.6) — the
/// letter is copied and nothing is opened, and the wizard opens no page.** «Скопировать текст» and
/// «Скопировать адрес» write the clipboard and go no further: the person writes the letter to the
/// box of the appeals. The one button `IDC_WZ_COPY` is «Скопировать текст» for every appeal, and
/// `wizard_copy` — «Скопировать и открыть группу» / «Copy and open GitHub Issues» — is gone; the
/// buttons of the wizard hand the shell nothing but the channel of «Готово». Not a line of the
/// module's code says `mailto`.
///
/// ⚠ Controls: «Скопировать текст» that opens the group as well is caught; a button of the wizard
/// that opens the issues is caught.
#[test]
fn the_letter_copies_its_text_and_opens_nothing_and_the_wizard_opens_no_page() {
    let source = letters_module_source();
    let text = method_body(&source, "    pub fn copy_appeal(")
        .expect("src\\letters.rs must declare `Wizard::copy_appeal`");
    let address = method_body(&source, "    pub fn copy_address(")
        .expect("src\\letters.rs must declare `Wizard::copy_address`");
    let command = wizard_events();
    let command = command.as_str();
    let button = squeezed_code(command);

    assert!(
        copies_and_opens_nothing(text),
        "199.3: «Скопировать текст» must copy and open nothing:\n{text}"
    );
    assert!(
        copies_and_opens_nothing(address),
        "199.3: «Скопировать адрес» must copy and open nothing:\n{address}"
    );
    assert!(
        letters_function_body(&source, "fn wizard_copy(").is_none(),
        "199.3: «Скопировать и открыть группу» left the wizard"
    );
    assert!(
        button.contains("Event::Pressed(Id::CopyText) => wizard.copy_appeal(self.owner),"),
        "the one button copies the text of the letter for every appeal:\n{button}"
    );
    assert!(
        the_wizard_opens_no_page_but_its_channel(command),
        "199.3: the buttons of the wizard open no page but the channel of «Готово»:\n{command}"
    );

    let issues = command.replace(
        "letters::act(&deed, Action::OpenChannel);",
        "letters::act(&deed, Action::OpenChannel);\n                letters::act(&deed, Action::OpenFeedback);",
    );
    assert_ne!(issues, command, "the control must change the body");
    assert!(
        !the_wizard_opens_no_page_but_its_channel(&issues),
        "the sweep does not see a button of the wizard that opens the issues"
    );
    assert!(
        code_lines(&source).all(|line| !line.contains("mailto")),
        "SEC-03: no line of the module's code may hand the shell a mailto"
    );

    let opening = text.replace(
        "self.status = if",
        "open_link(links::GROUP_URL);\n        self.status = if",
    );
    assert_ne!(opening, text, "the control must change the body");
    assert!(
        !copies_and_opens_nothing(&opening),
        "the sweep does not see «Скопировать текст» that opens a page as well"
    );
}

/// **T-120-22 Б — a report about the translation is the appeal of an idea under the mark of its
/// language.** Its first line is the mark with the code of the language, «Ошибка в переводе: de»;
/// under it, word for word, the appeal of the same idea, its two labels those of the translation —
/// «Что написано сейчас», «Как правильно». The labels are the Russian table's, out of the built
/// binary.
///
/// ⚠ Control: the idea without the mark begins with the name, as before; a trouble carrying the
/// mark — the person came for the translation and chose another card — is written as without it.
#[test]
fn a_report_about_the_translation_is_the_idea_under_the_mark_of_its_language() {
    use letters::report::{Draft, Facts, Trouble, build};
    use settings::{
        IDS_WIZARD_IDEA_HELPS, IDS_WIZARD_IDEA_WHAT, IDS_WIZARD_TRANSLATION_LINE,
        IDS_WIZARD_TRANSLATION_NOW, IDS_WIZARD_TRANSLATION_RIGHT,
    };

    let _guard = with_product_strings();
    let ru = |id| settings::text_in(Language::Ru, id);
    // A label as the appeal writes it: without its own colon or question mark, then «: ».
    let label = |id| format!("{}: ", ru(id).trim_end_matches([':', '?']));
    let idea = Draft {
        trouble: Trouble::Idea,
        idea: "Kopieren".to_owned(),
        helps: "Копировать".to_owned(),
        ..Draft::default()
    };
    let marked = Draft {
        translation: Some(Language::De),
        ..idea.clone()
    };
    let facts = Facts {
        version: "1.8.0".to_owned(),
        ..Facts::default()
    };

    let plain = build(&idea, &facts);
    let built = build(&marked, &facts);
    let (first, rest) = built
        .split_once("\r\n")
        .expect("the appeal has more than one line");
    println!("{built}");

    assert!(
        ru(IDS_WIZARD_TRANSLATION_LINE).contains("{0}"),
        "the mark has its place for the code"
    );
    assert_eq!(
        first,
        ru(IDS_WIZARD_TRANSLATION_LINE).replace("{0}", "de"),
        "the first line is the mark and the code of the language"
    );
    assert_eq!(
        rest,
        plain
            .replace(
                &label(IDS_WIZARD_IDEA_WHAT),
                &label(IDS_WIZARD_TRANSLATION_NOW)
            )
            .replace(
                &label(IDS_WIZARD_IDEA_HELPS),
                &label(IDS_WIZARD_TRANSLATION_RIGHT)
            ),
        "under the mark, the appeal of the idea with the labels of the translation"
    );
    assert!(
        rest.contains(&format!("{}Kopieren", label(IDS_WIZARD_TRANSLATION_NOW)))
            && rest.contains(&format!(
                "{}Копировать",
                label(IDS_WIZARD_TRANSLATION_RIGHT)
            )),
        "the control: the labels did change"
    );
    assert!(
        plain.starts_with("Lang Switcher 1.8.0\r\n"),
        "an idea without the mark begins with the name: {plain:?}"
    );
    let trouble = Draft {
        trouble: Trouble::WrongResult,
        ..marked.clone()
    };
    assert_eq!(
        build(&trouble, &facts),
        build(
            &Draft {
                translation: None,
                ..trouble.clone()
            },
            &facts
        ),
        "a trouble is written without the mark"
    );
}

/// **T-120-22 Б, C3 of the controller — the page of a report about the translation is headed with
/// the words of its button, «Нашли ошибку в переводе?»;** the explanation under the heading stays the
/// idea's, and the page of an ordinary idea keeps «Опишите идею». Out of the Russian table of the
/// built binary, in the window the page is described for.
#[test]
fn the_page_of_a_report_about_the_translation_is_headed_with_the_words_of_its_button() {
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::windows::wizard;
    use settings::{IDS_TRANSLATION_REPORT, IDS_WIZARD_IDEA_NOTE, IDS_WIZARD_IDEA_TITLE};

    let _guard = with_product_strings();
    let ru = |id| format!("{:?}", settings::text_in(Language::Ru, id));
    let header = |name: &str| {
        let wizard = letters::wizard_pages()
            .into_iter()
            .find(|(page, _)| *page == name)
            .map(|(_, wizard)| wizard)
            .unwrap_or_else(|| panic!("the page {name}"));
        let body = format!(
            "{:?}",
            wizard::Page {
                wizard,
                language: Language::Ru,
                theme: lang_switcher::theme::ThemeSetting::Light,
            }
            .describe()
            .body
        );
        // The head shown: since AUDIT-180 Z8 the heads of every step stand in `Steps(shown, …)`
        // (the head as tall as the tallest), and the shown one is the `shown`-th of them.
        let shown: usize = body
            .find("Steps(")
            .and_then(|at| body[at + "Steps(".len()..].split(',').next())
            .and_then(|number| number.trim().parse().ok())
            .unwrap_or(0);
        let at = body
            .match_indices("Header {")
            .nth(shown)
            .map(|(at, _)| at)
            .expect("the page has its header");
        let end = body[at..].find("}").map_or(body.len(), |end| at + end);
        body[at..end].to_owned()
    };
    let (idea, translation) = (header("wizard idea"), header("wizard translation"));
    println!("idea: {idea}\ntranslation: {translation}");

    assert!(
        translation.contains(&format!("title: {}", ru(IDS_TRANSLATION_REPORT))),
        "a report about the translation is headed «Нашли ошибку в переводе?»: {translation}"
    );
    assert!(
        idea.contains(&format!("title: {}", ru(IDS_WIZARD_IDEA_TITLE))),
        "an ordinary idea keeps «Опишите идею»: {idea}"
    );
    for header in [&idea, &translation] {
        assert!(
            header.contains(&ru(IDS_WIZARD_IDEA_NOTE)),
            "the explanation stays: {header}"
        );
    }
}

/// **Stage E107, task T-107-3, решение 199.10 — the line of the contact follows the field.**
/// «Для связи: …» after an empty line at the end of the text of the appeal, only when there is a
/// contact, replaced as the contact is typed and taken away when it is erased; what the person
/// wrote in the text stays. The label is the Russian table's, out of the built binary.
///
/// ⚠ Control: a text edited at its very end is not mistaken for the line of the contact — the line
/// goes after the edit and the edit stays.
#[test]
fn the_line_of_the_contact_follows_the_field() {
    use letters::report::{Draft, Facts, build, contact_line, with_contact};

    let _guard = with_product_strings();

    let built = build(
        &Draft {
            idea: "Переключать раскладку двойным Ctrl".to_owned(),
            trouble: letters::report::Trouble::Idea,
            ..Draft::default()
        },
        &Facts {
            version: "1.2.0".to_owned(),
            ..Facts::default()
        },
    );

    assert!(
        built.ends_with("\r\n"),
        "the text ends with its last line: {built:?}"
    );
    assert_eq!(contact_line(""), "");
    assert_eq!(contact_line("   "), "");
    assert_eq!(
        contact_line(" ivan@example.com "),
        "\r\nДля связи: ivan@example.com\r\n"
    );

    let one = with_contact(&built, "", "i");
    let two = with_contact(&one, "i", "iv");
    let none = with_contact(&two, "iv", "");

    assert_eq!(one, format!("{built}\r\nДля связи: i\r\n"));
    assert_eq!(
        two,
        format!("{built}\r\nДля связи: iv\r\n"),
        "replaced, not added"
    );
    assert_eq!(none, built, "taken away with the contact");

    let edited = format!("{one}P.S. спасибо");

    assert_eq!(
        with_contact(&edited, "i", "iv"),
        format!("{edited}\r\nДля связи: iv\r\n"),
        "the control: the edit stays and the line goes after it"
    );
}

/// **Задача T-103-7, сторож (3), решение 181.6 — the address of the letter is the site's, it is not
/// a link, and no string table carries it.** The address is written down here apart from the code;
/// the gate of С52 does not let it through (nothing may open it); and `app.rc` names it nowhere —
/// the fourteen tables carry `{0}`, so a fifteenth language cannot hold a stale copy.
///
/// ⭐ Since stage E107 (решение 199.3) the address is the box of the appeals, `lang.switcher@…`, and
/// no longer the author's own `send.me@…`.
///
/// ⚠ Control: the Russian line of the address with the address written into it is caught.
#[test]
fn the_address_of_the_letter_is_the_sites_and_no_table_carries_it() {
    assert_eq!(links::MAIL_ADDRESS, "lang.switcher@panda-pishet-kod.dev");
    assert!(
        !links::is_allowed(links::MAIL_ADDRESS),
        "the address is not a link: nothing may hand it to the shell"
    );

    let rc = resource_script::resource_script();
    let carrying = |text: &str| {
        text.lines()
            .filter(|line| line.contains(links::MAIL_ADDRESS))
            .count()
    };

    assert_eq!(
        carrying(&rc),
        0,
        "181.6: no line of app.rc and its language files may carry the address — the tables hold \
         it as {{0}}"
    );

    let spoilt = rc.replacen(
        "Адрес для письма: {0}",
        &format!("Адрес для письма: {}", links::MAIL_ADDRESS),
        1,
    );
    assert_ne!(spoilt, rc, "the control must change the file");
    assert_eq!(
        carrying(&spoilt),
        1,
        "the control: the address written into the Russian line must be caught"
    );
}

// =========================================================================================
// The demonstration of «Привет» — FR-101
// =========================================================================================

/// The whole animation is a table: typed letter by letter, the key flashes, the word is fixed,
/// the key flashes again, the word comes back — and it repeats exactly.
///
/// ⭐ **Решение 178.6, задача T-102-2: the canon of six letters moved.** Until `e101` every
/// language showed the one word «ghbdtn», six letters typed in frames 0…5. Each language has a
/// pair of its own now, four to eight letters long, so the typing is walked for every pair and
/// takes as many frames as its word has letters; the rest of the turn is what it was.
#[test]
fn the_demonstration_types_a_word_fixes_it_and_starts_over() {
    for language in Language::shipped() {
        let pair = letters::demo_pair(language);
        let letters_typed = pair.typed.chars().count() as u32;

        // The typing: one more character every frame, and nothing else moves.
        for frame in 0..letters_typed {
            let shown = letters::demo_frame(frame, pair);

            assert_eq!(
                shown.shown,
                frame as usize + 1,
                "{language:?}, frame {frame}"
            );
            assert!(!shown.pressed, "the key is not down while a word is typed");
            assert!(!shown.converted, "and the layout has not switched yet");
        }

        // The key goes down, and only then does the word change.
        assert!(letters::demo_frame(14, pair).pressed);
        assert!(
            !letters::demo_frame(14, pair).converted,
            "the press comes first"
        );
        assert!(
            letters::demo_frame(16, pair).converted,
            "and the word after it"
        );
        assert_ne!(
            letters::demo_frame(16, pair).text,
            letters::demo_frame(14, pair).text,
            "{language:?}: the word really changes — that is the whole picture"
        );

        // The second press puts it back, letter for letter (FR-05).
        assert!(letters::demo_frame(30, pair).pressed);
        assert_eq!(
            letters::demo_frame(40, pair).text,
            letters::demo_frame(0, pair).text,
            "{language:?}: and the word comes back exactly as it was"
        );

        // And the turn repeats: frame `n` and frame `n + DEMO_FRAMES` are the same picture.
        for frame in 0..letters::DEMO_FRAMES {
            assert_eq!(
                letters::demo_frame(frame, pair),
                letters::demo_frame(frame + letters::DEMO_FRAMES, pair),
                "{language:?}: the turn must repeat exactly at frame {frame}"
            );
        }
    }
}

/// **The frame without a tick is the final one — T-120-25: «анимация отключена — стоит итоговый
/// кадр».** Windows with its animation off gives the window no tick, and a window not shown has
/// none either; the example then stands at `letters::DEMO_STILL` and must show what it is for:
/// the word fixed, whole, the layout switched, the key up. Frame 0, where it stood before, is the
/// first typed letter of the wrong word.
#[test]
fn the_demonstration_without_a_tick_stands_at_the_fixed_word() {
    for language in Language::shipped() {
        let pair = letters::demo_pair(language);
        let still = letters::demo_frame(letters::DEMO_STILL, pair);

        assert_eq!(
            still.word_shown(),
            pair.fixed,
            "{language:?}: the still frame shows the fixed word, whole"
        );
        assert!(
            still.converted && !still.pressed,
            "{language:?}: the layout switched and the key up"
        );
    }
}

/// **The pairs of решение 178.6, written down here apart from the code** — `(language, typed,
/// fixed, [badge before the press, after it])`.
///
/// ⛔ Not read out of `letters::demo_pair`: a canon derived from the table it guards cannot fail
/// (the lesson of Э53а). Every word was taken off the keys of the Windows layouts, not out of
/// memory — the lesson of Э87, «цреф» is not «what» — and the instrument
/// `scratchpad-E102\pairs\pairs-e102.ps1 -Canon` reads this very array and holds it against the
/// keys again.
const PAIRS_OF_DECISION_178_6: &[(Language, &str, &str, [&str; 2])] = &[
    (Language::Ru, "ghbdtn", "привет", ["EN", "RU"]),
    (Language::En, "руддщ", "hello", ["RU", "EN"]),
    (Language::Uk, "ghbdsn", "привіт", ["EN", "UK"]),
    (Language::De, "Gr[-e", "Grüße", ["EN", "DE"]),
    (Language::Fr, "sqlut", "salut", ["EN", "FR"]),
    (Language::Es, "ma;ana", "mañana", ["EN", "ES"]),
    (Language::Pt, "for;a", "força", ["EN", "PT"]),
    (Language::It, "caff[", "caffè", ["EN", "IT"]),
    (Language::Pl, "руддщ", "hello", ["RU", "EN"]),
    (Language::Cs, "d2kuji", "děkuji", ["EN", "CS"]),
    (Language::Tr, "g]naydin", "günaydın", ["EN", "TR"]),
    (Language::El, "geia", "γεια", ["EN", "EL"]),
    (Language::He, "akuo", "שלום", ["EN", "HE"]),
    (Language::Ar, "lvpfh", "مرحبا", ["EN", "AR"]),
    // Stage E108, group A: the word of the translator, the keys off the layouts (pairs-e108.ps1).
    (Language::Hi, "vcmdls", "नमस्ते", ["EN", "HI"]),
    (Language::Ur, "vgja", "سلام", ["EN", "UR"]),
    // Wave 1 (pairs-wave.ps1, with the way back).
    (Language::Th, "l;ylfu", "สวัสดี", ["EN", "TH"]),
    (Language::Lo, "lt[kpfu", "ສະບາຍດີ", ["EN", "LO"]),
    (Language::Bn, "vcmdkej", "নমস্কার", ["EN", "BN"]),
    (Language::Mr, "vcmdkej", "नमस्कार", ["EN", "MR"]),
    (Language::Ne, "gd;|t]", "नमस्ते", ["EN", "NE"]),
    (Language::As, "vcmdkej", "নমস্কাৰ", ["EN", "AS"]),
    (Language::Sr, "ydravo", "здраво", ["EN", "SR"]),
    (Language::SrLatn, "ydravo", "zdravo", ["EN", "SR"]),
    (Language::Fa, "sghl", "سلام", ["EN", "FA"]),
    (Language::Ug, "sflfm", "سالام", ["EN", "UG"]),
    (Language::Ta, "bCidicd", "வணக்கம்", ["EN", "TA"]),
    (Language::Ml, "vcmdkejx", "നമസ്കാരം", ["EN", "ML"]),
    (Language::UzCyrl, "Cj-vbcbp", "Соғмисиз", ["EN", "UZ"]),
    // Uzbek in Latin — the other way round, as Polish (решение 178.5): typed with Russian on.
    (Language::Uz, "Ыфдщь", "Salom", ["RU", "UZ"]),
    (Language::Te, "vcmdkejx", "నమస్కారం", ["EN", "TE"]),
    (Language::Kn, "vcmdkej", "ನಮಸ್ಕಾರ", ["EN", "KN"]),
    (Language::Pa, "vcmls", "ਨਮਸਤੇ", ["EN", "PA"]),
    (Language::Gu, "vcmdls", "નમસ્તે", ["EN", "GU"]),
    (Language::Az, "Nec's'n", "Necəsən", ["EN", "AZ"]),
    (Language::Be, "ghsdbnfyyt", "прывітанне", ["EN", "BE"]),
    (Language::Kk, "c2ktv", "сәлем", ["EN", "KK"]),
    (Language::Tt, "cokfv", "сәлам", ["EN", "TT"]),
    (Language::Bg, "zdrawej", "здравей", ["EN", "BG"]),
    (Language::Mk, "zdravo", "здраво", ["EN", "MK"]),
    (Language::Or, "vcmdkej", "ନମସ୍କାର", ["EN", "OR"]),
    (Language::Km, "CMrabsYr", "ជំរាបសួរ", ["EN", "KM"]),
    (Language::Ka, "gamarjoba", "გამარჯობა", ["EN", "KA"]),
    (Language::Hy, "ba87", "բարև", ["EN", "HY"]),
    // Stage E109, group B (pairs-wave.ps1, with the way back).
    (Language::Hr, "ydravo", "zdravo", ["EN", "HR"]),
    (Language::Sk, "4au", "čau", ["EN", "SK"]),
    (Language::Sl, "\\ivjo", "živjo", ["EN", "SL"]),
    (Language::Bs, "'ao", "ćao", ["EN", "BS"]),
    (Language::Lt, "a2i8", "ačiū", ["EN", "LT"]),
    (Language::Lv, "Ымушлш", "Sveiki", ["RU", "LV"]),
    (Language::Sv, "hall[", "hallå", ["EN", "SV"]),
    (Language::Da, "hall'", "hallø", ["EN", "DA"]),
    (Language::Fi, "p'iv''", "päivää", ["EN", "FI"]),
    (Language::Nb, "hall[", "hallå", ["EN", "NB"]),
    (Language::Et, "t'nan", "tänan", ["EN", "ET"]),
    (Language::Nn, "hall[", "hallå", ["EN", "NN"]),
    (Language::Nl, "wqterdqg", "zaterdag", ["EN", "NL"]),
    (Language::Lb, "\"ddi", "äddi", ["EN", "LB"]),
    (Language::Cy, "рудщ", "helo", ["RU", "CY"]),
    (Language::Is, "h;", "hæ", ["EN", "IS"]),
    (Language::Vi, "al4", "alô", ["EN", "VI"]),
    (Language::Ro, "bun[", "bună", ["EN", "RO"]),
    (Language::Hu, "-dv", "üdv", ["EN", "HU"]),
    (Language::Sq, "mir;dita", "mirëdita", ["EN", "SQ"]),
    (Language::Mt, "bon[u", "bonġu", ["EN", "MT"]),
    (Language::Mi, "фкщрф", "aroha", ["RU", "MI"]),
];

/// **Задача T-102-2, сторож (1) — every language of the interface shows the pair of решение
/// 178.6.** A word in the reader's own language typed with the English layout on, and the word it
/// becomes; Russian keeps «ghbdtn» → «привет», English and Polish show «руддщ» → «hello».
///
/// ⚠ Control: the canon with the pairs of two languages swapped is caught — the comparison is made
/// language by language, not as a set of pairs.
#[test]
fn every_language_shows_the_pair_of_decision_178_6() {
    for language in Language::shipped() {
        assert_eq!(
            PAIRS_OF_DECISION_178_6
                .iter()
                .filter(|(of, ..)| *of == language)
                .count(),
            1,
            "{language:?}: the canon names every language of the interface exactly once"
        );
    }

    // Stage E108: a value of group A without its pair yet stands in the arm of the pairs to come,
    // and the build does not offer it.
    for language in Language::ALL {
        if PAIRS_OF_DECISION_178_6
            .iter()
            .all(|(of, ..)| *of != language)
        {
            assert_eq!(
                letters::demo_pair(language),
                letters::PAIR_TO_COME,
                "{language:?}: a language with no pair in the canon is one whose pair is to come"
            );
            assert!(
                !language.is_shipped(),
                "{language:?}: offered without its pair of решение 178.6"
            );
        }
    }

    let pair_of =
        |(_, typed, fixed, badge): (Language, &'static str, &'static str, [&'static str; 2])| {
            letters::DemoPair {
                typed,
                fixed,
                badge,
            }
        };

    for &entry in PAIRS_OF_DECISION_178_6 {
        assert_eq!(
            letters::demo_pair(entry.0),
            pair_of(entry),
            "{:?}: the pair of решение 178.6",
            entry.0
        );
    }

    let mut swapped = PAIRS_OF_DECISION_178_6.to_vec();
    let german = (swapped[3].1, swapped[3].2, swapped[3].3);
    (swapped[3].1, swapped[3].2, swapped[3].3) = (swapped[4].1, swapped[4].2, swapped[4].3);
    (swapped[4].1, swapped[4].2, swapped[4].3) = german;

    assert!(
        swapped
            .into_iter()
            .any(|entry| letters::demo_pair(entry.0) != pair_of(entry)),
        "the control: the pairs of two languages swapped must be caught"
    );
}

/// **Задача T-102-2, сторож (3) — the typing takes exactly as many frames as the word has
/// letters, one a frame, and then the word waits.** The words of решение 178.6 are four to eight
/// letters long; the six frames of `e101` would show a five-letter word one letter past its end.
/// The rest of the turn does not move: the press at 14, the fixed word at 16, the press again at
/// 30, the word back from 32, and the turn repeats every 48 frames.
///
/// ⚠ A five-letter word of the test's own goes through the same table: the rule is about the
/// length of a word, not about the fourteen that happen to be in the program.
#[test]
fn the_demonstration_types_as_many_frames_as_the_word_has_letters() {
    let five = letters::DemoPair {
        typed: "abcde",
        fixed: "vwxyz",
        badge: ["AB", "CD"],
    };

    for pair in Language::ALL
        .map(letters::demo_pair)
        .into_iter()
        .chain([five])
    {
        let typed = pair.typed.chars().count();
        let fixed = pair.fixed.chars().count();

        assert!(
            typed < 14,
            "«{}»: the word must be typed before the press at frame 14",
            pair.typed
        );

        for frame in 0..letters::DEMO_FRAMES {
            let shown = letters::demo_frame(frame, pair);
            let at = frame as usize;
            let expected = match frame {
                _ if at < typed => (pair.typed, at + 1, false, false),
                0..=13 => (pair.typed, typed, false, false),
                14..=15 => (pair.typed, typed, true, false),
                16..=29 => (pair.fixed, fixed, false, true),
                30..=31 => (pair.fixed, fixed, true, true),
                _ => (pair.typed, typed, false, false),
            };

            assert_eq!(
                (shown.text, shown.shown, shown.pressed, shown.converted),
                expected,
                "«{}», frame {frame}",
                pair.typed
            );
            assert_eq!(
                shown,
                letters::demo_frame(frame + letters::DEMO_FRAMES, pair),
                "«{}»: the turn repeats at frame {frame}",
                pair.typed
            );
        }
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
/// letters out of the feed refuse to be built without an entry behind them.
#[test]
fn each_letter_fills_the_slots_of_its_own_shape() {
    let _guard = with_product_strings();

    let context = letters::PlanContext {
        version: "0.39.0".to_owned(),
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
    assert!(hello.cards.is_empty(), "and no cards");
    assert_eq!(
        hello.foot,
        settings::text(settings::IDS_HELLO_FOOT),
        "and the line about the tray icon under the body — 208.106 п. 3"
    );

    let thanks = letters::plan_for(Letter::Thanks, &context);
    assert!(!thanks.demo);
    assert_eq!(
        thanks
            .cards
            .iter()
            .map(|card| card.button.action)
            .collect::<Vec<_>>(),
        [letters::Action::OpenSupport, letters::Action::OpenWizard],
        "«Спасибо» carries the pair of cards: the support page and the wizard (T-120-16)"
    );
    assert_eq!(
        thanks.link.as_ref().map(|link| link.action),
        Some(letters::Action::OpenFeedback),
        "and the link to where a person can tell about it — решение 181.1"
    );
    // ⭐ Task T-78-1 ended the «while» this line used to carry: until then both addresses were
    // placeholders and both buttons were drawn dead. They are the author's own now (139.2), and
    // the assertion says the same rule from the other side — which is the half worth guarding,
    // because a button that silently went back to grey is the failure a person would see.
    assert!(
        thanks.all_buttons().all(|button| button.enabled),
        "the support page, the wizard and the group are live once their addresses are real \
         (П7, П8; решение 139.2)"
    );
    assert!(thanks.left.is_some(), "«Напомнить позже» is offered");
    assert!(
        !thanks.foot.is_empty(),
        "and the letter says it comes once and can be put off once, for a week"
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
    let _guard = with_product_strings();

    let mut item = news(12);
    // ⚠ **Task T-41-1: this address used to be `https://example.com/post`**, and task T-88-1
    // moved it again. The button of a news entry is drawn for an address the gate of
    // `links::is_allowed` passes, and that gate knows only the hosts this build carries — which
    // is the whole of finding С52. Until `e88` the one such host a test could name was the
    // reserved one, so the button came out dead; since then it is the author's own, the address
    // is the one the first file of the feed carries, and the button is live (решение 148.7).
    item.link = links::PROGRAM_URL.to_owned();

    let context = letters::PlanContext {
        version: "0.39.0".to_owned(),
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
    let link = plan
        .right
        .as_ref()
        .expect("and the link it carried, beside «Прочитано»");
    assert!(
        link.enabled,
        "an address of this program leaves the button drawn and live"
    );
    assert_eq!(link.action, letters::Action::OpenLink);
    assert!(
        plan.accent
            .as_ref()
            .is_some_and(|button| button.action == letters::Action::MarkRead),
        "«Прочитано» is the accented button of a news letter — FR-101"
    );
    assert!(
        !plan.foot.is_empty(),
        "the line under it says what «Напомнить позже» promises — FR-101, word for word from the mock-up"
    );

    // **Task T-41-1, finding С52 — the road an empty address already took.** A news entry whose
    // link points at a host this build does not carry keeps its heading, its text and its place
    // in the list; what it does not get is a button. The entry is not thrown away — a person
    // reads the news either way — and nothing this program can be talked into opening is opened.
    let mut foreign = news(12);
    foreign.link = "https://example.com/post".to_owned();

    let plan = letters::plan_for(
        Letter::News(12),
        &letters::PlanContext {
            item: Some(&foreign),
            ..context.clone()
        },
    );

    assert_eq!(plan.panel_title, foreign.title, "the entry is still shown");
    assert_eq!(plan.panel_text, foreign.text, "with all of its text");
    assert!(
        plan.right.is_none(),
        "and no button at all for an address this build will not open — С52"
    );

    // Task T-88-1, решение 148.7: the reserved domain takes the same road since `e88` — its host
    // left the program with the feed's placeholder, so an entry pointing there gets no button at
    // all rather than a dead one.
    let mut reserved = news(12);
    reserved.link = "https://example.invalid/post".to_owned();

    let plan = letters::plan_for(
        Letter::News(12),
        &letters::PlanContext {
            item: Some(&reserved),
            ..context.clone()
        },
    );

    assert_eq!(plan.panel_text, reserved.text, "the entry is still shown");
    assert!(
        plan.right.is_none(),
        "and no button for the reserved domain either — 148.7"
    );

    // An update has nothing to mark as read: its accented button is the download page, and the
    // three steps of FR-102 stand in the panel under the author's own words.
    //
    // ⭐ **Task T-119-9 (решения 208.95–208.99), the honest letter by the mock-up.** The heading
    // is the program's — the updater could not, and the letter says so; the quiet line names the
    // version of the feed, the one installed here and the day of the release; the paragraph opens
    // with the notice (a date, not a count of days), then «Что нового в X» over the author's words;
    // the panel's fourth row offers the wizard, and its button opens it.
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
    assert_eq!(
        plan.right.as_ref().map(|button| button.action),
        Some(letters::Action::Close),
        "and it closes with the plain button"
    );
    assert_eq!(
        plan.title,
        settings::text(settings::IDS_FALLBACK_TITLE),
        "the heading is the program's and says what happened — T-119-9"
    );
    assert_ne!(
        plan.title, update.title,
        "not the author's heading of the release"
    );

    let released = update.date.map(letters::format_date).unwrap_or_default();

    assert!(!released.is_empty(), "the entry is dated");
    assert_eq!(
        plan.subtitle,
        settings::format_text(settings::IDS_FALLBACK_SUB, &["0.41.0", "0.39.0", &released]),
        "«доступна X · у вас Y · выпуск <дата>»"
    );

    let notice = settings::format_text(settings::IDS_FALLBACK_NOTICE, &["0.41.0", &released]);
    let whats_new = settings::format_text(settings::IDS_FALLBACK_WHATS_NEW, &["0.41.0"]);

    assert_eq!(
        plan.note, notice,
        "the notice with the date, in its own block"
    );
    assert_eq!(
        (plan.whats_new.as_str(), plan.panel_text.as_str()),
        (whats_new.as_str(), update.text.as_str()),
        "then «Что нового в X» over what the author says changed"
    );
    assert!(
        plan.note.contains(&released) && !released.chars().all(|c| c.is_ascii_digit()),
        "the notice carries the date of the release — решение 208.99 п. 3"
    );
    assert_eq!(
        plan.rows.len(),
        4,
        "the program says how to update, and what if it repeats"
    );
    assert_eq!(
        plan.rows
            .iter()
            .map(|row| row.marker.as_str())
            .collect::<Vec<_>>(),
        ["1", "2", "3", ""],
        "the steps are numbered, not bulleted, and the line about the author is not a step"
    );
    assert_eq!(
        plan.rows[3].text,
        settings::text(settings::IDS_FALLBACK_REPEATS)
    );
    let wizard = plan
        .left
        .as_ref()
        .expect("«Написать автору…» first in the row — решение 208.97 п. 2");
    assert_eq!(wizard.action, letters::Action::OpenWizard);
    assert_eq!(wizard.label, settings::text(settings::IDS_WRITE_TO_AUTHOR));
    assert!(wizard.enabled, "and it can be pressed");
    assert!(
        plan.rows.iter().all(|row| !row.text.trim().is_empty()),
        "every step is written in all fourteen tables"
    );
    assert!(
        plan.subtitle.contains("0.39.0") && plan.subtitle.contains("0.41.0"),
        "the quiet line names the version installed here and the one announced"
    );
    assert!(
        plan.foot.contains("0.41.0"),
        "and the line under the buttons names the one in the menu"
    );
}

/// **«Новости и обновления» since task T-119-10 (решение 208.119): the switch is «Сообщать об
/// обновлениях программы» — `[letters] release_window`, the window «Обновлено» — and it is there from
/// the first day; the switch of the feed and its rule of ninety days are gone. A feed turned off in
/// the file stays off, and the panel says so under the switch.**
#[test]
fn the_author_window_has_two_states_and_the_switch_is_the_difference() {
    let _guard = with_product_strings();

    let today = day(2027, 1, 1);
    let mut state = settled("0.39.0");
    state.first_run = Some(day(2026, 9, 1));

    let view = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);

    assert_eq!(view.switch, Some(true), "the switch is there, and it is on");
    assert!(
        view.switch_note.is_empty(),
        "the feed is on: nothing under it"
    );
    assert!(
        !view.feed_state.is_empty(),
        "and whether the feed has been read"
    );
    assert!(!view.letters, "and nothing in «Последние письма»");

    state.release_window = false;
    assert_eq!(
        letters::author_view(&state, today, "0.39.0", FeedView::EMPTY).switch,
        Some(false),
        "it follows the windows of updates"
    );

    state.feed = false;
    let off = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);
    assert_eq!(
        off.switch,
        Some(false),
        "the feed in the file does not move it"
    );
    assert!(
        !off.switch_note.is_empty(),
        "and the feed off in the file is said under it"
    );
}

/// **Task T-70-1, finding Э69-Б-1 — the switch follows the setting a click has written.**
///
/// `draw_switch` paints the tick out of `AuthorView::switch`, a copy [`letters::author_view`]
/// makes when «От автора» opens. Before the task a click wrote `Letters::feed` and repainted
/// that very copy: the file said the feed was off while the tick still said it was on. After a
/// click the copy has to be the one the window would build afresh on the setting now in force.
#[test]
fn the_switch_follows_the_setting_a_click_has_written() {
    let _guard = with_product_strings();

    let today = day(2027, 1, 1);
    let mut state = settled("0.39.0");
    state.first_feed_letter = Some(day(2026, 9, 10));

    let opened = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);
    assert_eq!(
        opened.switch,
        Some(true),
        "the window opens on a feed that is on"
    );

    // The click writes the setting, and the window's copy follows it. Since task T-119-10 the
    // switch is `release_window`.
    let mut view = opened.clone();
    state.release_window = false;
    view.follow_feed(&state);

    assert_eq!(
        view.switch,
        Some(false),
        "the tick goes off with the setting"
    );
    assert_eq!(
        view,
        letters::author_view(&state, today, "0.39.0", FeedView::EMPTY),
        "and the copy is the one the window would build on the setting now in force"
    );

    // The second click puts both back together.
    state.release_window = true;
    view.follow_feed(&state);

    assert_eq!(
        view, opened,
        "the second click brings the tick back with the setting"
    );
}

/// The source of `src\letters.rs`, line endings normalised — the sweep below matches on it.
fn letters_module_source() -> String {
    // The module, not the file alone: `letters.rs` and the three files of T-120-23.
    letters_source()
}

/// The source of «От автора», a window of the shell since T-120-17.
fn author_module_source() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("windows")
            .join("author.rs"),
    )
    .expect("src\\ui\\windows\\author.rs must be readable")
    .replace("\r\n", "\n")
}

/// The arm of «От автора» that answers its switch — from the event to the reply that ends it — or
/// `None` unless there is exactly one.
fn switch_arm(source: &str) -> Option<&str> {
    let mut heads = source.match_indices("Event::Toggled(Id::Switch, on) => {");
    let (start, _) = heads.next()?;

    if heads.next().is_some() {
        return None;
    }

    let length = source[start..].find("Reply::Stay")?;

    Some(&source[start..start + length])
}

/// Whether a branch writes the setting, reads the configuration again, and only then hands what
/// it read to the copy the switch is drawn from.
///
/// The reading stands after the write — as a statement before the call (the dialog until
/// T-120-17) or as its argument (`.follow_feed(&letters::state_now())`, the arm of the shell).
fn follows_what_was_written(branch: &str) -> bool {
    let (Some(write), Some(follow)) = (branch.find("update_state("), branch.find(".follow_feed("))
    else {
        return false;
    };
    let call = &branch[follow..];
    let argument = call[..call.find(';').unwrap_or(call.len())].contains("state_now()");

    write < follow && (branch[write..follow].contains("state_now()") || argument)
}

/// **Task T-70-1, finding Э69-Б-1 — the click moves the copy the switch is drawn from**, in the
/// shell since T-120-17.
///
/// The half of the repair a pure function cannot show: that the window calls it. The click goes
/// through the configuration, which only the tray writes, and a test binary has no tray — so the
/// arm of `Event::Toggled(Id::Switch, …)` of «От автора» (`ui::windows::author`) is read instead:
/// it has to write the setting (FR-102; `release_window` since T-119-10), read the configuration
/// again and hand what it read to `AuthorView::follow_feed`. The host describes the window again
/// after every event (`deliver`: `rebuild`, `invalidate`), so the repaint is the host's and not
/// the arm's.
///
/// ⚠ Controls: the arm is found exactly once and is the one that writes the setting; the same arm
/// without the call — the shape of `e69` — is caught, and so are a call fed with the reading made
/// before the write and a call put before the write.
#[test]
fn a_click_on_the_switch_moves_the_copy_the_switch_is_drawn_from() {
    let source = author_module_source();
    let branch = switch_arm(&source)
        .expect("the sweep must find exactly one arm of «От автора» for its switch");

    assert!(
        branch.contains("update_state(|state| state.release_window = on)"),
        "the arm found is not the one that writes the setting:\n{branch}"
    );

    assert!(
        follows_what_was_written(branch),
        "Э69-Б-1: the click writes the setting but does not move the copy the switch is drawn \
         from:\n{branch}"
    );

    let lines: Vec<&str> = branch.lines().collect();
    let without = |needle: &str| {
        lines
            .iter()
            .filter(|line| !line.contains(needle))
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    };

    assert!(
        !follows_what_was_written(&without(".follow_feed(")),
        "the sweep does not see an arm without the call — it cannot fail"
    );
    let read_before = format!(
        "let state = letters::state_now();\n{}",
        branch.replace(
            ".follow_feed(&letters::state_now())",
            ".follow_feed(&state)"
        )
    );
    assert!(
        read_before.contains(".follow_feed(&state)"),
        "the control must change the arm it is made of"
    );
    assert!(
        !follows_what_was_written(&read_before),
        "the sweep does not see a call fed with the reading made before the write"
    );

    let call = lines
        .iter()
        .find(|line| line.contains(".follow_feed("))
        .expect("the call was found above");

    assert!(
        !follows_what_was_written(&format!("{call}\n{}", without(".follow_feed("))),
        "the sweep does not see a call put before the write"
    );
}

/// The text of one function of `src\letters.rs`, from its signature to the brace that closes it in
/// the first column — or `None` if the module no longer declares it.
fn letters_function_body<'a>(source: &'a str, signature: &str) -> Option<&'a str> {
    let (_, after) = source.split_once(signature)?;

    after.split_once("\n}\n").map(|(body, _)| body)
}

/// The code of a stretch of source with its comment lines dropped and its whitespace squeezed —
/// task T-71-2. A needle then says «these tokens in this order», whatever `cargo fmt` does to the
/// lines of a call, and a sentence of prose that names a call is not taken for the call.
fn squeezed_code(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
        .replace("( ", "(")
        .replace(" )", ")")
}

/// The version line of «От автора» says what is installed, and what is available when the feed
/// names something newer.
#[test]
fn the_author_window_names_the_version_and_the_one_that_is_waiting() {
    let _guard = with_product_strings();

    let state = settled("0.39.0");
    let today = day(2026, 9, 3);
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &[],
    };

    let latest = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);
    assert!(latest.version_state.contains("0.39.0"));

    let waiting = letters::author_view(&state, today, "0.39.0", feed);
    assert!(waiting.version_state.contains("0.39.0"));
    assert!(
        waiting.version_state.contains("0.40.0"),
        "and the one that is waiting: {}",
        waiting.version_state
    );
    assert!(waiting.letters, "and something for «Последние письма»");
}

// ---------------------------------------------------------------------------------------
// Reading `src\letters.rs` as text — the shape `tests\watchdog.rs` and `tests\selection.rs`
// settled on
// ---------------------------------------------------------------------------------------

/// The module `letters` read as text with line endings normalised — since T-120-23 the file
/// `src\letters.rs` and the three it was divided into, `links.rs`, `feed.rs` and `appeal.rs` — the order
/// they stood in the file: a guard that looked for something in the module, or for its absence, looks where it
/// went, and an emptied file cannot make it green.
fn letters_source() -> String {
    ["letters.rs", "links.rs", "feed.rs", "appeal.rs"]
        .iter()
        .map(|file| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file);
            std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
                .replace("\r\n", "\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The body of the item whose signature line starts with `signature` — from that line to the
/// first `}` in column 0 after it.
///
/// A region and not the whole module: a sweep over everything would count the prose of a file
/// that documents itself at length, and would be a sweep nobody could keep green.
fn body_of<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).unwrap_or_else(|| {
        panic!("src\\letters.rs must contain \"{signature}\" — the sweep below is about it")
    });

    let rest = &source[start..];
    let end = rest
        .find("\n}")
        .expect("the item must end at a closing brace in column 0");

    let region = &rest[..end];

    assert!(
        !region.is_empty() && region.len() < rest.len(),
        "\"{signature}\" was bounded rather than taken as the rest of the module"
    );

    region
}

/// The code lines of `text` — comments dropped.
///
/// Comments are dropped on purpose: this file explains its rules in prose beside them, and a
/// sweep that counted a sentence about a call would assert nothing.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
}

/// **«Открыть страницу загрузки» stands there whatever the feed says** — task T-78-2, решение
/// 139.2 п. 3 и п. 6, in the shell since T-120-17.
///
/// Until `e77` the button was put into the row only when an `update` entry of the feed existed,
/// and enabled only when that entry's own `link` was not a placeholder. There is no feed
/// (138.4), so the button was translated into fourteen languages and shown to nobody.
///
/// The description of «От автора» is pure now, so the window itself is asked: with no feed, with
/// news only, with an update, the switch off and on — the button stands there every time, and it
/// is disabled by the address of this build (`links::DOWNLOAD_URL` a placeholder) and never by the
/// feed.
#[test]
fn the_download_button_stands_there_whatever_the_feed_says() {
    use lang_switcher::letters::{FeedView, author_view_in, sample_entry};
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::windows::author::{Author, Id};

    let _guard = with_product_strings();

    let item = sample_entry();
    let today = item.date.expect("the sample entry carries its day");
    let news = [sample_entry()];
    let feeds = [
        ("no feed", None, &news[..0]),
        ("news only", None, &news[..]),
        ("an update", Some(&item), &news[..]),
    ];
    let placeholder = links::is_placeholder(links::DOWNLOAD_URL);

    for (what, update, news) in feeds {
        for switch in [false, true] {
            let state = Letters {
                feed: switch,
                release_window: switch,
                ..Letters::default()
            };
            let view = author_view_in(
                Language::Ru,
                &state,
                today,
                "1.7.0",
                FeedView { update, news },
            );
            let window = Author {
                view,
                language: Language::Ru,
                theme: ThemeSetting::Light,
            }
            .describe();

            assert!(
                format!("{:?}", window.body).contains("Button { id: Download,"),
                "{what}, switch {switch}: «Открыть страницу загрузки» must stand in «От автора»"
            );
            assert_eq!(
                window.disabled.contains(&Id::Download),
                placeholder,
                "{what}, switch {switch}: the button is disabled by the address of this build, \
                 not by the feed"
            );
        }
    }
}

/// **«Последние письма»** — the update first, then the news newest first, each marked read or
/// not.
#[test]
fn the_list_shows_the_update_and_the_three_news_newest_first() {
    let _guard = with_product_strings();

    let mut state = settled("0.39.0");
    let announced = update("0.40.0");
    let items = [news(11), news(12), news(13)];
    let feed = FeedView {
        update: Some(&announced),
        news: &items,
    };

    state.mark_read(12);

    // ⭐ Task T-119-9 (решение 208.99 п. 2): the row of the update is the second door of the letter
    // «Обновление» and stands, as the menu entry does, from the seventh day the feed has named the
    // newer version — the days written out, not taken from the constant.
    let first = day(2026, 9, 1);
    state.update_noticed = Some(first);

    for offset in 0..7 {
        let entries = letters::list_entries(&state, Some(first.plus_days(offset)), "0.39.0", feed);

        assert_eq!(
            entries.len(),
            3,
            "day {offset}: three news and no update yet"
        );
        assert!(entries.iter().all(|entry| entry.news.is_some()));
    }

    assert_eq!(
        letters::list_entries(&state, None, "0.39.0", feed).len(),
        3,
        "no day, no row of the update"
    );

    let entries = letters::list_entries(&state, Some(first.plus_days(7)), "0.39.0", feed);

    assert_eq!(entries.len(), 4, "one update and three news");
    assert_eq!(entries[0].news, None, "the update is first and is not news");
    assert!(!entries[0].unread, "and it is never «unread»");
    assert_eq!(
        entries[0].title,
        settings::text(settings::IDS_FALLBACK_TITLE),
        "the row says what the letter says — the program could not update by itself"
    );
    assert_eq!(
        entries[0].meta,
        settings::format_text(settings::IDS_ENTRY_FALLBACK_MARK, &["0.40.0", "0.39.0"]),
        "«доступна X · у вас Y»"
    );
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
    assert!(
        letters::list_entries(&state, Some(first.plus_days(7)), "0.39.0", FeedView::EMPTY)
            .is_empty()
    );
}

/// A letter out of the feed can find its own entry again — task Т-32-6.
///
/// Why this exists: a change of interface language **destroys and rebuilds** the window (Э31),
/// and the rebuild starts from the letter and nothing else. Before this, the rebuilt window
/// asked `plan_for` with no entry at all and «Новость» came back as an empty frame. The test
/// walks the same road the rebuild does: publish a feed, then ask for the entry by letter.
#[test]
fn a_letter_out_of_the_feed_finds_its_entry_again_by_name() {
    let _guard = with_product_strings();

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

    for letter in [Letter::Welcome, Letter::Thanks] {
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
    let _guard = with_product_strings();

    let mut announced = update("0.41.0");
    announced.link = "https://example.invalid/download".to_owned();

    let plan = letters::plan_for(
        Letter::Update,
        &letters::PlanContext {
            version: "0.40.0".to_owned(),
            hotkey: String::new(),
            snooze_offered: false,
            item: Some(&announced),
        },
    );

    assert!(
        plan.accent.as_ref().is_some_and(|button| !button.enabled),
        "a placeholder address leaves the button drawn and dead — П5"
    );

    // ⚠ **Task T-41-1 moved this half of the test, and the move is the finding С52 itself.**
    // Until this stage the sentence here read «an address that is not on the reserved domain is
    // a live button», and `https://example.com/download` proved it. That is precisely what the
    // finding is about: the address travels in the feed, and a signature names the author of
    // the feed rather than the safety of the string in it. A foreign host is now a dead button
    // too — the gate of `links::is_allowed` passes only the hosts this build carries.
    announced.link = "https://example.com/download".to_owned();

    let plan = letters::plan_for(
        Letter::Update,
        &letters::PlanContext {
            version: "0.40.0".to_owned(),
            hotkey: String::new(),
            snooze_offered: false,
            item: Some(&announced),
        },
    );

    assert!(
        plan.accent.as_ref().is_some_and(|button| !button.enabled),
        "a host this build does not carry is a dead button as well — С52"
    );
}

// =========================================================================================
// The colours and the grounds of the windows — FR-92а's vocabulary, reused
// =========================================================================================

// =========================================================================================
// The mark of the first run — stage E116, task T-116-12, решение 208.59
// =========================================================================================

/// ⭐ **The automaton of the mark.** A fresh file is `due`; any HTTP answer to the marked read
/// makes it `sent`; no answer leaves it `due`; a file without the key never sends it; and the read
/// after `sent` carries no mark.
#[test]
fn the_mark_of_the_first_run_goes_out_once_with_the_first_answer() {
    use letters::{FirstMark, first_mark_after, first_query};

    let fresh = Some(FirstMark::Due);

    assert!(
        first_query(fresh),
        "T-116-12: the read after a fresh file is marked"
    );
    assert_eq!(
        first_mark_after(fresh, true),
        Some(FirstMark::Sent),
        "an answer of any status sends the mark"
    );
    assert_eq!(
        first_mark_after(fresh, false),
        fresh,
        "no answer at all: the mark stays due and is asked again on the schedule of FR-102"
    );
    assert!(
        !first_query(first_mark_after(fresh, true)),
        "the read after the mark was sent carries none"
    );
    assert_eq!(
        first_mark_after(Some(FirstMark::Sent), true),
        Some(FirstMark::Sent)
    );

    for answered in [false, true] {
        assert_eq!(
            first_mark_after(None, answered),
            None,
            "a file without the key never becomes a first run"
        );
    }
    assert!(!first_query(None) && !first_query(Some(FirstMark::Sent)));

    for mark in [FirstMark::Due, FirstMark::Sent] {
        assert_eq!(FirstMark::from_config_str(mark.as_config_str()), Some(mark));
    }
    assert_eq!(FirstMark::from_config_str("maybe"), None);
}

/// ⭐ **The mark is `?first=1` on the path of the feed, and nothing else.** Every address of the
/// feed carries no query of its own, so the mark is the only one; a placeholder address is not
/// read at all, marked or not — no socket in the battery, and no answer to count.
#[test]
fn the_mark_is_first_one_on_the_path_of_the_feed_and_nothing_else() {
    for url in links::FEED_URLS {
        assert!(
            !url.contains('?'),
            "the feed address carries no query: {url}"
        );
        assert_eq!(letters::feed::marked(url), format!("{url}?first=1"));
    }

    assert_eq!(
        letters::feed::marked("https://panda-pishet-kod.dev/langswitcher/news.toml"),
        "https://panda-pishet-kod.dev/langswitcher/news.toml?first=1"
    );
    assert_eq!(
        letters::feed::read_from_marked(&["https://example.invalid/news.toml"], "ru", true),
        (None, false),
        "a placeholder is not asked, so nothing answered"
    );
}

/// ⭐ **The wiring of the mark** — the roads the automaton above is fed by:
/// * `tick` asks for the marked read by `first_query(stored.first_mark)`;
/// * the feed thread reads through `feed::read_now_marked` and raises `FIRST_ANSWERED` only when
///   the read was marked and answered;
/// * `take_feed` writes the mark as sent **before** it asks the mailbox, which holds only a good
///   document — a refused one is an answer all the same;
/// * `read_over` says «answered» **before** it looks at the status — any status counts;
/// * the updater never sends the mark: `update.rs` reads the feed through `feed::fetch`.
#[test]
fn the_mark_of_the_first_run_is_wired_from_the_tick_to_the_answer() {
    let source = letters_source();

    let tick = body_of(&source, "pub fn tick(");
    assert_eq!(
        code_lines(tick)
            .filter(|line| line.contains("first_query(stored.first_mark)"))
            .count(),
        1,
        "tick asks for the marked read by the mark of the file"
    );

    let start = body_of(&source, "pub fn start_feed_read(");
    let lines: Vec<&str> = code_lines(start).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(at("feed::read_now_marked(&language, first)").is_some());
    assert!(
        matches!((at("if first && answered"), at("FIRST_ANSWERED.store(true")), (Some(a), Some(b)) if a < b),
        "the flag is raised only for a marked read that was answered:\n{start}"
    );

    let take = body_of(&source, "pub fn take_feed(");
    let lines: Vec<&str> = code_lines(take).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(
        matches!(
            (at("FIRST_ANSWERED.swap(false"), at("first_mark_after(state.first_mark, true)"), at("MAILBOX")),
            (Some(a), Some(b), Some(c)) if a < b && b < c
        ),
        "take_feed writes the mark as sent before it asks the mailbox:\n{take}"
    );

    let read_over = body_of(&source, "fn read_over(");
    let lines: Vec<&str> = code_lines(read_over).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(
        matches!(
            (at("answered.set(true)"), at("status_of(request)")),
            (Some(a), Some(b)) if a < b
        ),
        "an answer counts before its status is looked at"
    );

    let update = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("update.rs"),
    )
    .expect("src/update.rs is readable")
    .replace("\r\n", "\n");
    assert!(
        !update.contains("read_now_marked")
            && !update.contains("first=1")
            && !update.contains("marked("),
        "the updater never sends the mark"
    );
}

// =========================================================================================
// T-119-8 — the letter «Что нового» and the balloon «обновлён до X» are gone, решение 208.94 п. 4
// =========================================================================================

/// The names the cleanup of task T-119-8 took away. A line that is only a comment may still name
/// them (the holes are documented where the numbers were); a line of code or a row of a table may
/// not.
const RETIRED_T_119_8: [&str; 14] = [
    "IDS_WHATSNEW_TITLE",
    "IDS_WHATSNEW_FROM",
    "IDS_WHATSNEW_TODAY",
    "IDS_WHATSNEW_PANEL",
    "IDS_WHATSNEW_1",
    "IDS_WHATSNEW_2",
    "IDS_WHATSNEW_3",
    "IDS_WHATSNEW_FULL",
    "IDS_TOAST_WHATSNEW",
    "IDS_TOAST_UPDATED_TITLE",
    "IDS_TOAST_UPDATED",
    "Letter::WhatsNew",
    "fn announce_updated",
    "UPDATED_ANNOUNCED",
];

/// Every line of `text` that is not a comment and names one of [`RETIRED_T_119_8`], as
/// `file:line: text`. A name is matched as a whole word, so `IDS_TOAST_UPDATE` (the letter
/// «Обновление», which stays) is not taken for `IDS_TOAST_UPDATED`.
fn retired_names_in(file: &str, text: &str) -> Vec<String> {
    let whole = |line: &str, name: &str| {
        line.match_indices(name).any(|(at, _)| {
            let after = line[at + name.len()..].chars().next();
            !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    };

    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            !code.starts_with("//") && !code.starts_with('#') || code.starts_with("#define")
        })
        .filter(|(_, line)| RETIRED_T_119_8.iter().any(|name| whole(line, name)))
        .map(|(n, line)| format!("{file}:{}: {}", n + 1, line.trim()))
        .collect()
}

/// **Task T-119-8, решение 208.94 п. 4 — the remains of 118.5 are gone from the tables and the
/// code.** The letter «Что нового» (closed since 118.5 and kept for a return that will not come),
/// its eight strings and its balloon, and the balloon «Lang Switcher обновлён до X» with its two
/// strings: not a row in any of the sixty-six `res\lang\*.rc`, not a `#define` in `app.rc`, not a
/// line of code in `src`. The window «Обновлено» says what they said.
///
/// Red before (on `46673be`): 66 × 11 rows of the tables, eleven `#define`s, the variant and its
/// arms, `announce_updated`.
///
/// ⚠ Control: a row put back into a table and a variant put back into the code are found.
#[test]
fn the_letter_whats_new_and_the_balloon_after_an_update_are_gone() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found: Vec<String> = Vec::new();
    let mut tables = 0;

    for entry in std::fs::read_dir(root.join("res").join("lang")).expect("res\\lang") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|ext| ext == "rc") {
            tables += 1;
            let text = std::fs::read_to_string(&path).expect("a table reads");
            found.extend(retired_names_in(&path.display().to_string(), &text));
        }
    }

    let app_rc = std::fs::read_to_string(root.join("app.rc")).expect("app.rc");
    found.extend(retired_names_in("app.rc", &app_rc));

    let mut sources = 0;
    for entry in std::fs::read_dir(root.join("src")).expect("src") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            sources += 1;
            let text = std::fs::read_to_string(&path).expect("a source reads");
            found.extend(retired_names_in(&path.display().to_string(), &text));
        }
    }

    assert_eq!(tables, 66, "sixty-six tables are swept");
    assert!(sources > 20, "the sources are swept: {sources}");
    assert!(
        found.is_empty(),
        "the remains of 118.5 are back:\n{}",
        found.join("\n")
    );

    // The control: one row of a table and one arm of code put back are each found.
    let uk_back = "    IDS_TOAST_UPDATED       \"Клацніть, щоб дізнатися, що нового.\"\n";
    assert_eq!(
        retired_names_in("uk.rc", uk_back).len(),
        1,
        "a row put back is found"
    );
    let code_back = "        Letter::WhatsNew => None,\n        // Letter::WhatsNew in a comment\n";
    assert_eq!(
        retired_names_in("letters.rs", code_back).len(),
        1,
        "an arm put back is found"
    );
    assert!(
        retired_names_in("settings.rs", "pub const IDS_TOAST_UPDATE: u16 = 3138;\n").is_empty(),
        "and the letter «Обновление», which stays, is not taken for the balloon that left"
    );
}

/// ⭐ **The good feed is kept beside the configuration and comes back at the start** — task
/// T-119-9, решение 208.98 (п. 2 of 208.97): a restart on the same day, after the day's read, no
/// longer loses the menu entry «Доступна версия X…», the letter and the row of «Последние письма»
/// until tomorrow. Where the copy lives, who writes it and how it comes back — held here by the
/// shape of the code; that a copy nobody signed is no feed is held in `tests\feed.rs`.
#[test]
fn the_good_feed_is_kept_beside_the_configuration_and_comes_back_at_the_start() {
    let home = Path::new(r"C:\Users\A\AppData\Roaming");

    assert_eq!(
        settings::kept_feed_path_in(home),
        home.join("Lang_Switcher").join("news.toml")
    );
    assert_eq!(
        settings::kept_feed_path_in(home).parent(),
        settings::config_path_in(home).parent(),
        "beside the configuration"
    );

    let source = letters_source();

    let start = body_of(&source, "pub fn start_feed_read(");
    let lines: Vec<&str> = code_lines(start).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(
        matches!(
            (at("feed::read_now_marked(&language, first)"), at("keep_feed_document(&document)"), at("MAILBOX.lock()")),
            (Some(a), Some(b), Some(c)) if a < b && b < c
        ),
        "the good document is kept on the feed's own thread, after the read and before the nudge:\n{start}"
    );

    let keep = body_of(&source, "fn keep_feed_document(");
    assert!(
        code_lines(keep).any(|line| line.contains("settings::write_bytes_to(&path")),
        "written by the body that writes the configuration — atomically:\n{keep}"
    );

    // Task T-119-13: the keys are the ones the feed is read with — `feed::sources()`, the author's
    // in a Release image (`FEED_KEYS`), the stand's under the testing feature only.
    let kept = body_of(&source, "pub fn kept_feed(");
    let lines: Vec<&str> = code_lines(kept).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(
        matches!(
            (at("let (_, keys) = feed::sources();"), at("feed::read_document(document, language,"), at("feed::verify_with(body, signature, &keys)")),
            (Some(a), Some(b), Some(c)) if a < b && b < c
        ),
        "read again through the keys of the feed, every time:\n{kept}"
    );
    let sources = body_of(&source, "pub fn sources(");
    assert!(
        code_lines(sources).any(|line| line.contains("FEED_KEYS,")),
        "and those keys are the author's unless the stand's doors say otherwise:\n{sources}"
    );

    let restore = body_of(&source, "pub fn restore_feed(");
    let lines: Vec<&str> = code_lines(restore).collect();
    let at = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(
        matches!(
            (at("if !stored.feed"), at("kept_feed(&document"), at("publish_feed(")),
            (Some(a), Some(b), Some(c)) if a < b && b < c
        ),
        "a feed turned off restores nothing, and only a copy that passed is published:\n{restore}"
    );
    assert!(
        at("feed_last_read").is_none() && at("feed_last_try").is_none(),
        "a restore is not a read: the days of the read do not move"
    );

    // The product's own start, and only it: the tests that attach a tray never read the
    // `%APPDATA%` of the machine they run on.
    let tray_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tray.rs");
    let tray = std::fs::read_to_string(&tray_path)
        .expect(r"src\tray.rs must be readable")
        .replace("\r\n", "\n");

    assert!(
        code_lines(body_of(&tray, "pub fn attach("))
            .any(|line| line.contains("crate::letters::restore_feed()")),
        "the product's start puts the kept feed back"
    );
    assert!(
        !code_lines(body_of(&tray, "pub fn attach_at_via("))
            .any(|line| line.contains("restore_feed")),
        "and the start the tests use does not"
    );
}

// =========================================================================================
// The letters and «Последние письма» on the shell — task T-120-16
// =========================================================================================

/// A hidden owner for a window of `letters` opened by a test, destroyed with what it owns.
#[cfg(feature = "testing")]
struct HiddenOwner(windows::Win32::Foundation::HWND);

#[cfg(feature = "testing")]
impl HiddenOwner {
    fn new() -> Self {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WS_EX_TOOLWINDOW, WS_POPUP,
        };
        use windows::core::w;

        // SAFETY: a system class, no parent, no pointer of ours; destroyed in `Drop`.
        let owner = unsafe {
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
        }
        .expect("a hidden owner must be creatable");
        Self(owner)
    }
}

#[cfg(feature = "testing")]
impl Drop for HiddenOwner {
    fn drop(&mut self) {
        // SAFETY: the window this value made; destroying it destroys the windows it owns.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0) };
    }
}

/// **T-120-16, the red «before»: a letter opens as a window of the shell, and the template of the
/// old one is gone.** «Привет» and «Спасибо» are opened by the product's own `letters::show_letter`
/// through the door of the tests; each must be of the class of an ordinary window of the shell and
/// answer `ui::host::probe`; `app.rc` carries neither `IDD_LETTER` nor `IDD_LETTERS_LIST`.
///
/// The class is asked first, by `GetClassNameW`, which any window answers: `probe` reads the data
/// of a window of the shell, and asked of a dialog it would read the dialog's state as the host's.
/// Before the task the door made a dialog out of `IDD_LETTER` — the class `#32770` — and the
/// script declared both templates.
#[cfg(feature = "testing")]
#[test]
fn a_letter_is_a_window_of_the_shell_and_its_template_is_gone() {
    let _guard = with_product_strings();

    let script = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc"))
        .expect("app.rc must be readable");
    let declared: Vec<&str> = ["IDD_LETTER DIALOGEX", "IDD_LETTERS_LIST DIALOGEX"]
        .into_iter()
        .filter(|template| script.contains(template))
        .collect();

    let mut not_shell = Vec::new();
    for letter in [Letter::Welcome, Letter::Thanks] {
        let owner = HiddenOwner::new();
        let window = letters::testing::open_letter_hidden(owner.0, letter, None)
            .unwrap_or_else(|| panic!("{letter:?} must open"));
        let mut class = [0u16; 64];
        // SAFETY: a live window this test opened; the buffer is a local of this frame.
        let length =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassNameW(window, &mut class) };
        let class = String::from_utf16_lossy(&class[..usize::try_from(length).unwrap_or(0)]);
        let shell =
            class == "LangSwitcher.Shell" && lang_switcher::ui::host::probe(window).is_some();
        if !shell {
            not_shell.push(format!("{letter:?} ({class})"));
        }
    }

    assert!(
        declared.is_empty() && not_shell.is_empty(),
        "app.rc declares {declared:?}; not windows of the shell: {not_shell:?}"
    );
}

/// **T-120-19, the red «before»: the wizard opens as a window of the shell, and «Закрыть» of the
/// thanks stands where «Далее» stands on the steps.** The wizard is opened by the product's own
/// `letters::open_wizard` through the door of the tests and must be of the class of an ordinary
/// window of the shell and answer `ui::host::probe`; `app.rc` carries no `IDD_WIZARD`. Then the
/// eight pages are laid out at the narrowest window and at two thirds of a 1920 × 1080 screen: the
/// main button of the row — «Далее», «Отправить», «Повторить», «Закрыть» — has one right edge and
/// one top on every page (the sheet: «ряд кнопок в одной точке на всех шагах»).
///
/// Before the task the door made a dialog out of `IDD_WIZARD` — the class `#32770`.
#[cfg(feature = "testing")]
#[test]
fn the_wizard_is_a_window_of_the_shell_and_its_row_stands_still() {
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::{Class, client_size, lay_out};
    use lang_switcher::ui::text::{Direction, Text};
    use lang_switcher::ui::windows::wizard;

    let _guard = with_product_strings();

    let script = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc"))
        .expect("app.rc must be readable");
    let owner = HiddenOwner::new();
    let window = letters::testing::open_wizard_hidden(owner.0, HMODULE::default())
        .expect("the wizard must open");
    let mut class = [0u16; 64];
    // SAFETY: a live window this test opened; the buffer is a local of this frame.
    let length =
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetClassNameW(window, &mut class) };
    let class = String::from_utf16_lossy(&class[..usize::try_from(length).unwrap_or(0)]);
    let shell = class == "LangSwitcher.Shell" && lang_switcher::ui::host::probe(window).is_some();
    assert!(
        shell && !script.contains("IDD_WIZARD DIALOGEX"),
        "the wizard must be a window of the shell ({class}) and app.rc must carry no IDD_WIZARD"
    );

    let text = Text::new(Language::Ru.tag(), Direction::LeftToRight).expect("DirectWrite");
    let next = wizard::Id::Next.number();
    for (class, w, h) in [(Class::Narrow, 620.0, 500.0), (Class::Large, 1280.0, 720.0)] {
        let k = fit::k_for(Language::Ru.tag(), class);
        let mut places = Vec::new();
        for (name, page) in wizard::states(Language::Ru) {
            let page = page.map(&wizard::Id::number);
            let layout = lay_out(&page, client_size(w, h), k, false, &text);
            let rect = layout
                .items
                .iter()
                .find(|item| item.id == Some(next))
                .map(|item| item.rect)
                .unwrap_or_else(|| panic!("{name}: the main button must be laid out"));
            println!(
                "{w} × {h}, {name}: right {:.1}, top {:.1}",
                rect.x + rect.w,
                rect.y
            );
            places.push((name, rect.x + rect.w, rect.y));
        }
        let (first, right, top) = &places[0];
        for (name, other_right, other_top) in &places {
            assert!(
                (other_right - right).abs() < 0.5 && (other_top - top).abs() < 0.5,
                "{w} × {h}: the main button of «{name}» stands at ({other_right:.1}, \
                 {other_top:.1}), of «{first}» at ({right:.1}, {top:.1})"
            );
        }
    }
}

/// Every button of a description, the cards' and the row's, with what it says about the browser.
fn buttons_of<I: Copy>(window: &lang_switcher::ui::scene::Window<I>) -> Vec<(I, String, bool)> {
    use lang_switcher::ui::scene::Node;

    fn walk<I: Copy>(node: &Node<I>, found: &mut Vec<(I, String, bool)>) {
        match node {
            Node::Column(nodes)
            | Node::Rows(nodes)
            | Node::Row(nodes)
            | Node::Steps(_, nodes)
            | Node::Buttons(nodes) => nodes.iter().for_each(|node| walk(node, found)),
            Node::Pair(a, b) => {
                walk(a, found);
                walk(b, found);
            }
            Node::Triple(a, b, c) => {
                walk(a, found);
                walk(b, found);
                walk(c, found);
            }
            Node::Fill(node) | Node::Surface(_, node) => walk(node, found),
            Node::Action { buttons, .. } => buttons.iter().for_each(|node| walk(node, found)),
            Node::Button {
                id, label, browser, ..
            } => found.push((*id, label.clone(), *browser)),
            _ => {}
        }
    }

    let mut found = Vec::new();
    walk(&window.body, &mut found);
    window
        .buttons
        .iter()
        .for_each(|node| walk(node, &mut found));
    found
}

/// **T-120-16: every button that opens the browser carries ↗, and no button that opens a window of
/// the program does** (passport §6) — over the descriptions of the four letters and of «Последние
/// письма». What opens the browser is written out here, apart from `Action::opens_browser`, by what
/// each action does in `letters::act`.
#[test]
fn a_button_carries_the_arrow_exactly_when_it_opens_the_browser() {
    use lang_switcher::letters::Action;
    use lang_switcher::ui::windows::{letter, letters_list};

    let _guard = with_product_strings();

    let to_the_browser = |action: Action| {
        matches!(
            action,
            Action::OpenChannel
                | Action::OpenFeedback
                | Action::OpenSupport
                | Action::OpenDownload
                | Action::OpenProgram
                | Action::OpenLink
        )
    };

    let mut faults = Vec::new();
    let mut seen = 0;
    for language in [Language::Ru, Language::En, Language::He] {
        for (name, window) in letter::states(language) {
            for (id, label, browser) in buttons_of(&window) {
                let letter::Id::Do(action) = id else {
                    faults.push(format!("{name}: «{label}» is a button without an action"));
                    continue;
                };
                if browser != to_the_browser(action) {
                    faults.push(format!(
                        "{name} ({language:?}): «{label}» ({action:?}) ↗ = {browser}"
                    ));
                }
                seen += 1;
            }
        }
        for (name, window) in letters_list::states(language) {
            for (_, label, browser) in buttons_of(&window) {
                if browser {
                    faults.push(format!(
                        "{name} ({language:?}): «{label}» opens no browser and carries ↗"
                    ));
                }
                seen += 1;
            }
        }
    }
    assert!(
        seen > 30,
        "the descriptions must have been walked: {seen} buttons"
    );
    assert!(
        faults.is_empty(),
        "↗ exactly on the buttons that open the browser:\n{}",
        faults.join("\n")
    );

    // Both kinds of button that carry the sign are met: the main button of «Обновление» (the page
    // of downloads) and the card of «Спасибо» (the page of support).
    let all: Vec<_> = letter::states(Language::Ru)
        .iter()
        .flat_map(|(_, window)| buttons_of(window))
        .filter(|(_, _, browser)| *browser)
        .map(|(id, _, _)| id)
        .collect();
    for action in [Action::OpenDownload, Action::OpenSupport] {
        assert!(
            all.contains(&letter::Id::Do(action)),
            "{action:?} is a button with ↗"
        );
    }
}

/// **T-120-16: «Спасибо» keeps its pair of cards side by side in the narrowest window, 620 × 500,
/// and does not scroll** (the rule of the pair; 208.166) — in every language, at the factor of the
/// table, laid out by the layout of the shell and measured by DirectWrite.
#[test]
fn thanks_keeps_its_pair_side_by_side_in_the_narrowest_window() {
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::{Class, Part, client_size, lay_out};
    use lang_switcher::ui::scene::Role;
    use lang_switcher::ui::text::{Direction, Text};
    use lang_switcher::ui::windows::letter;

    let _guard = with_product_strings();

    let mut faults = Vec::new();
    for language in Language::ALL {
        // A language excused from fitting «Спасибо» at 620 × 500 (208.175): printed, not tested.
        if let Some(excuse) = excused::excused("letter thanks", Class::Narrow, language.tag()) {
            println!("{language:?}: excused — {}", excused::excuse_line(excuse));
            continue;
        }
        let window = letter::states(language)
            .into_iter()
            .find(|(name, _)| name == "letter thanks")
            .expect("the catalogue has «Спасибо»")
            .1;
        let cards = [settings::IDS_SUPPORT_PANEL, settings::IDS_FEEDBACK_PANEL]
            .map(|id| settings::text_in(language, id));
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("DirectWrite");
        let k = fit::k_for(language.tag(), Class::Narrow);
        let layout = lay_out(&window, client_size(620.0, 500.0), k, rtl, &text);

        let heads: Vec<_> = layout
            .items
            .iter()
            .filter_map(|item| match &item.part {
                Part::Text {
                    role: Role::Strong,
                    text,
                    ..
                } if cards.contains(text) => Some(item.rect),
                _ => None,
            })
            .collect();
        let side_by_side = heads.len() == 2
            && (heads[0].y - heads[1].y).abs() < 0.5
            && (heads[0].x - heads[1].x).abs() > 1.0;
        if !side_by_side {
            faults.push(format!(
                "{language:?}: the pair stands one under another {heads:?}"
            ));
        }
        if !layout.shortfall.fits() {
            faults.push(format!(
                "{language:?}: does not fit, {:.1} down",
                layout.shortfall.h
            ));
        }
    }
    assert!(
        faults.is_empty(),
        "«Спасибо» at 620 × 500:\n{}",
        faults.join("\n")
    );
}

/// **T-120-16: the knock of a letter is the window in the corner** (T-120-14): «Спасибо» and
/// «Новость» knock with «LANG SWITCHER · ПИСЬМО», their heading, the words of the old balloon and
/// one button, «Открыть письмо»; «Привет» and «Обновление» do not knock. The balloon of
/// the icon is no longer asked by `letters.rs`.
#[test]
fn the_knock_of_a_letter_is_the_window_in_the_corner() {
    let _guard = with_product_strings();

    for (letter, title) in [
        (Letter::Thanks, settings::IDS_THANKS_TITLE),
        (Letter::News(7), settings::IDS_NEWS_LETTER_TITLE),
    ] {
        let content = letters::notice_content_in(Language::Ru, letter)
            .unwrap_or_else(|| panic!("{letter:?} knocks"));
        let (_, words) = letters::toast_strings(letter).expect("its two strings");
        assert_eq!(
            content.about,
            settings::text(settings::IDS_CORNER_LETTER),
            "{letter:?}: whose window — one short word, as the hint's and «Обновлено»'s"
        );
        assert_eq!(
            content.title,
            settings::text(title),
            "{letter:?}: the heading"
        );
        assert_eq!(content.text, settings::text(words), "{letter:?}: the words");
        assert_eq!(
            content.main.0,
            settings::text(settings::IDS_LETTERS_OPEN),
            "{letter:?}: «Открыть письмо»"
        );
        assert!(
            content.second.is_none() && content.check.is_none() && !content.picture,
            "{letter:?}: one button and the ✕ — the letter comes by itself later, as after an \
             unnoticed knock"
        );
    }
    for letter in [Letter::Welcome, Letter::Update] {
        assert!(
            letters::notice_content_in(Language::Ru, letter).is_none(),
            "{letter:?} does not knock"
        );
    }

    let code = squeezed_code(&letters_source());
    assert!(
        code.contains("crate::corner::show(content, true, notice_ended);"),
        "the knock is shown by the window in the corner, with the sound of a notification"
    );
    assert!(
        !code.contains("announce_letter("),
        "and the balloon of the icon is not asked any more"
    );
}

/// **T-120-16 п. 4: no number over the heading of a letter** — the only window with a version over
/// its title is «О программе». The line over the heading of each of the four is whose letter it is.
#[test]
fn the_line_over_the_heading_of_a_letter_has_no_version() {
    use lang_switcher::ui::scene::Node;
    use lang_switcher::ui::windows::letter;

    let _guard = with_product_strings();

    for language in [Language::Ru, Language::En, Language::Ar] {
        for (name, window) in letter::states(language) {
            let Node::Column(parts) = &window.body else {
                panic!("{name}: the body is a column");
            };
            let Some(Node::Header { eyebrow, .. }) = parts.first() else {
                panic!("{name}: the head comes first");
            };
            assert_eq!(
                eyebrow.as_deref(),
                Some(settings::text_in(language, settings::IDS_LETTER_CAPTION).as_str()),
                "{name} ({language:?}): the line over the heading says whose letter it is"
            );
            assert!(
                !eyebrow
                    .as_deref()
                    .unwrap_or_default()
                    .contains(env!("CARGO_PKG_VERSION")),
                "{name}: and carries no version"
            );
        }
    }
}

/// **Задача T-103-2, сторож (4), решение 181.1 — «Спасибо» sends a person to write in the language of
/// its window, and a change of language on the fly moves it** (moved here from `tests\settings.rs`
/// with the window, T-120-16). «Спасибо» opened hidden by the product's own `letters::show_letter`
/// and asked by the door where its link «Открыть группу» would take them — the very language
/// `letters::act` reads the address by. Built in Russian, the group; the language moved and
/// `letters::language_changed` told the windows — the letter is built again in English, a new window
/// whose caption is English — GitHub Issues; Ukrainian, the group again.
///
/// ⚠ Control: the language moved and **nobody told the window** — the address stays with the words
/// the window shows.
#[cfg(feature = "testing")]
#[test]
fn thanks_sends_a_person_to_write_in_the_language_of_its_window() {
    let _guard = with_product_strings();

    let caption = |window| {
        let mut buffer = [0u16; 128];
        // SAFETY: a live window of this thread; the buffer is a local of this frame.
        let length =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(window, &mut buffer) };
        String::from_utf16_lossy(&buffer[..usize::try_from(length).unwrap_or(0)])
    };

    let owner = HiddenOwner::new();
    let first = letters::testing::open_letter_hidden(owner.0, Letter::Thanks, None)
        .expect("«Спасибо» must open");
    let now = || {
        let window = letters::testing::letter_window().expect("«Спасибо» is open");
        let address = letters::testing::feedback_url(window).expect("a window of `letters`");
        (window, address)
    };

    let mut seen = Vec::new();
    seen.push(("built in Ru", now().1));

    // The control: the language moved and nobody told the window.
    settings::set_ui_language(Language::En);
    seen.push(("En, not told", now().1));

    // The road of FR-94.
    letters::testing::language_changed_hidden(owner.0);
    let (english, address) = now();
    seen.push(("En, told", address));
    let english_caption = caption(english);

    settings::set_ui_language(Language::Uk);
    letters::testing::language_changed_hidden(owner.0);
    seen.push(("Uk, told", now().1));

    settings::set_ui_language(Language::Ru);

    let group = links::GROUP_URL;
    let issues = links::ISSUES_URL;
    assert_eq!(
        seen,
        [
            ("built in Ru", group),
            ("En, not told", group),
            ("En, told", issues),
            ("Uk, told", group),
        ],
        "181.1: the address follows the language of the window, and moves when it moves"
    );
    assert_ne!(
        english, first,
        "the letter is built again in the new language"
    );
    assert_eq!(
        english_caption,
        settings::text_in(Language::En, settings::IDS_LETTER_CAPTION),
        "and the new window speaks English"
    );
}

/// **Задача T-103-2, сторож (4), решение 181.1 — a change of language on the fly moves where a
/// person is sent to write**, the wizard a window of the shell since T-120-19 (moved here from
/// `tests\settings.rs`, where it opened the dialog `IDD_WIZARD`). Opened hidden and asked by the
/// door of the task where its address would take them: built in Russian, the group; the language
/// moved and [`letters::language_changed`] told the window — which opens again in the new
/// language — GitHub Issues; Ukrainian, the group again.
///
/// ⚠ Control: the language moved and **nobody told the window** — the address stays with the words
/// the window shows.
#[cfg(feature = "testing")]
#[test]
fn a_change_of_language_on_the_fly_moves_where_a_person_is_sent_to_write() {
    let _guard = with_product_strings();

    let owner = HiddenOwner::new();
    let first = letters::testing::open_wizard_hidden(owner.0, HMODULE::default())
        .expect("the wizard must open");
    let now = || {
        let window = letters::wizard_window().expect("the wizard is open");
        let address = letters::testing::feedback_url(window).expect("a window of `letters`");
        (window, address)
    };

    let mut seen = Vec::new();
    seen.push(("built in Ru", now().1));

    // The control: the language moved and nobody told the window.
    settings::set_ui_language(Language::En);
    seen.push(("En, not told", now().1));

    letters::testing::language_changed_hidden(owner.0);
    let (english, address) = now();
    seen.push(("En, told", address));

    settings::set_ui_language(Language::Uk);
    letters::testing::language_changed_hidden(owner.0);
    seen.push(("Uk, told", now().1));

    settings::set_ui_language(Language::Ru);

    println!("{seen:#?}");

    let group = links::GROUP_URL;
    let issues = links::ISSUES_URL;
    assert_eq!(
        seen,
        [
            ("built in Ru", group),
            ("En, not told", group),
            ("En, told", issues),
            ("Uk, told", group),
        ],
        "181.1: the address follows the language of the window, and moves when it moves"
    );
    assert_ne!(
        english, first,
        "the wizard is built again in the new language"
    );
}

/// **Задача T-103-9, сторож (2), решение 181.33 — a change of language on the fly moves the two
/// pages of the site too.** «От автора», a window of the shell since T-120-17 (the test moved here
/// from `tests\settings.rs`, where it opened the dialog `IDD_AUTHOR`), opened hidden and asked by
/// the door of the task which pages «Открыть страницу загрузки» and the address under the panda
/// would open — by the language `letters::act` chooses them by. Built in Russian, the Russian
/// pages; English after [`letters::language_changed`], the pages under `/en/`; Ukrainian, its own
/// under `/uk/` (Q-206-1); Kazakh, the Russian ones again (181.33 п. 2, 202.6).
///
/// ⚠ Control: the language moved and **nobody told the window** — the pages stay with the words
/// the window shows.
#[cfg(feature = "testing")]
#[test]
fn a_change_of_language_on_the_fly_moves_the_pages_of_the_site_too() {
    let _guard = with_product_strings();

    let owner = HiddenOwner::new();
    let first = letters::testing::open_author_hidden(owner.0, HMODULE::default(), "0.83.0")
        .expect("«От автора» must open");
    let now = || {
        let window = letters::testing::author_window().expect("«От автора» is open");
        let pages = letters::testing::site_pages(window).expect("a window of `letters`");
        (window, pages)
    };

    let mut seen = Vec::new();
    seen.push(("built in Ru", now().1));

    // The control: the language moved and nobody told the window.
    settings::set_ui_language(Language::En);
    seen.push(("En, not told", now().1));

    letters::testing::language_changed_hidden(owner.0);
    let (english, pages) = now();
    seen.push(("En, told", pages));

    settings::set_ui_language(Language::Uk);
    letters::testing::language_changed_hidden(owner.0);
    seen.push(("Uk, told", now().1));

    settings::set_ui_language(Language::Kk);
    letters::testing::language_changed_hidden(owner.0);
    seen.push(("Kk, told", now().1));

    settings::set_ui_language(Language::Ru);

    println!("{seen:#?}");

    let russian = (links::DOWNLOAD_URL, links::PROGRAM_URL);
    let english_pages = (links::DOWNLOAD_URL_EN, links::PROGRAM_URL_EN);
    let ukrainian = (
        "https://panda-pishet-kod.dev/uk/langswitcher/download/?lang=uk",
        "https://panda-pishet-kod.dev/uk/langswitcher/?lang=uk",
    );

    assert_eq!(
        seen,
        [
            ("built in Ru", russian),
            ("En, not told", russian),
            ("En, told", english_pages),
            ("Uk, told", ukrainian),
            ("Kk, told", russian),
        ],
        "181.33: the pages follow the language of the window, and move when it moves"
    );
    assert_ne!(
        english, first,
        "«От автора» is built again in the new language"
    );
}

/// **The pictures of C3 of T-120-16 and T-120-17** — the four letters, «Последние письма» and «От
/// автора» drawn in memory by the drawing of the shell itself (the layout and `elements::draw`, as a
/// live window draws them): Russian and German straight and Hebrew mirrored, both themes, the window of two thirds of a 1920 × 1080
/// screen (1280 × 720) and the narrowest one (620 × 500), at the factor of the table. Printed only
/// when `LANGSW_LETTERS_DIR` names a folder.
#[cfg(feature = "testing")]
#[test]
fn print_the_letters() {
    use lang_switcher::ui::canvas::{Canvas, Mode};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::{client_size, lay_out};
    use lang_switcher::ui::text::{Direction, Text};
    use lang_switcher::ui::tokens::{Color, Theme};
    use lang_switcher::ui::windows::{author, letter, letters_list, wizard};
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    let Some(folder) = std::env::var_os("LANGSW_LETTERS_DIR") else {
        return;
    };
    let _guard = with_product_strings();
    // SAFETY: the apartment of this test's thread, for the whole test (`Art` outlives each canvas).
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let mut art = lang_switcher::ui::art::Art::new();
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("the folder of the pictures");

    for language in [Language::Ru, Language::De, Language::He] {
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("the text of the language");
        let windows: Vec<(String, lang_switcher::ui::scene::Window<u16>)> =
            letter::states(language)
                .into_iter()
                .map(|(name, window)| (name, window.map(&letter::Id::number)))
                .chain(
                    letters_list::states(language)
                        .into_iter()
                        .map(|(name, window)| (name, window.map(&letters_list::Id::number))),
                )
                .chain(
                    author::states(language)
                        .into_iter()
                        .map(|(name, window)| (name, window.map(&author::Id::number))),
                )
                .chain(
                    wizard::states(language)
                        .into_iter()
                        .map(|(name, window)| (name, window.map(&wizard::Id::number))),
                )
                .collect();
        for (name, window) in &windows {
            for (w, h) in [(1280.0_f32, 720.0_f32), (620.0, 500.0)] {
                let size = client_size(w, h);
                let k = fit::k_for(language.tag(), size.class);
                let layout = lay_out(window, size, k, rtl, &text);
                for theme in [Theme::Light, Theme::Dark] {
                    let look = Look {
                        theme,
                        rtl,
                        b: layout.b,
                        h: size.h,
                        text: &text,
                    };
                    let mut canvas =
                        Canvas::in_memory(size.w, size.h, 1.0, Mode::Direct2D).expect("a canvas");
                    canvas.frame_with(Some(&mut art), |painter| {
                        painter.clear(Color::Bg.get(theme));
                        for item in &layout.items {
                            elements::draw(painter, &look, &item.part, item.rect, State::NORMAL);
                        }
                    });
                    let file = folder.join(format!(
                        "{}-{}-{}x{}-{}.png",
                        name.replace(' ', "-"),
                        language.tag(),
                        w as u32,
                        h as u32,
                        if theme == Theme::Dark {
                            "dark"
                        } else {
                            "light"
                        },
                    ));
                    canvas.save_png(&file).expect("the picture is written");
                }
            }
        }
    }
}

/// What the demonstration of a description shows: the word on the screen of the laptop and whether
/// the key is lit — the node over the picture of «Привет», or none.
fn shown_on_the_picture<I>(window: &lang_switcher::ui::scene::Window<I>) -> Option<(String, bool)> {
    use lang_switcher::ui::scene::{Node, OnPicture};

    fn find<I>(node: &Node<I>) -> Option<(String, bool)> {
        match node {
            Node::OverPicture { over, .. } => {
                let word = over.iter().find_map(|(anchor, on)| match (anchor, on) {
                    (&"screen", OnPicture::Word { word, .. }) => Some(word.clone()),
                    _ => None,
                })?;
                let pressed = over.iter().find_map(|(anchor, on)| match (anchor, on) {
                    (&"key", OnPicture::Keys { pressed, .. }) => Some(*pressed),
                    _ => None,
                })?;
                Some((word, pressed))
            }
            Node::Column(nodes) | Node::Rows(nodes) | Node::Row(nodes) => {
                nodes.iter().find_map(find)
            }
            _ => None,
        }
    }

    find(&window.body)
}

/// **T-120-25 п. 4, the red «before»: in «Привет» on the shell, after three ticks the frame shown
/// is `demo_frame(3, pair)`** — the word typed so far on the screen of the laptop and the key —
/// and after the turn of 48 it is the first frame again. «Привет» asks the window for the tick of
/// the demonstration (`DEMO_TICK_MS`); the other letters ask none («в покое — ноль»).
///
/// Before the task the letter had no tick: the example stood still and there was nothing over the
/// picture.
#[test]
fn hello_shows_the_frame_of_its_tick() {
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::scene::Event;
    use lang_switcher::ui::windows::letter::{Id, LetterView};

    let _guard = with_product_strings();

    let context = letters::PlanContext {
        version: "1.8.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: true,
        item: None,
    };
    let view = |letter| LetterView {
        letter,
        plan: letters::plan_in(Language::Ru, letter, &context),
        key: "Pause".to_owned(),
        tick: 0,
        language: Language::Ru,
        theme: lang_switcher::theme::ThemeSetting::Light,
    };

    let mut hello = view(Letter::Welcome);
    let pair = hello.plan.demo_pair;
    let expected = |tick| {
        let frame = letters::demo_frame(tick, pair);
        (
            frame.text.chars().take(frame.shown).collect::<String>(),
            frame.pressed,
        )
    };

    for _ in 0..3 {
        hello.event(Event::<Id>::Tick);
    }
    let after_three = shown_on_the_picture(&hello.describe());
    for _ in 3..letters::DEMO_FRAMES {
        hello.event(Event::<Id>::Tick);
    }
    let after_the_turn = shown_on_the_picture(&hello.describe());

    assert_eq!(
        after_three,
        Some(expected(3)),
        "after three ticks «Привет» shows the frame demo_frame(3, pair)"
    );
    assert_eq!(
        after_the_turn,
        Some(expected(0)),
        "and after the turn of {} the first one again",
        letters::DEMO_FRAMES
    );
    assert_ne!(
        expected(3),
        expected(0),
        "the control: the third frame differs from the first"
    );
    assert_eq!(hello.tick(), Some(letters::DEMO_TICK_MS), "«Привет» ticks");
    for letter in [Letter::Thanks, Letter::Update, Letter::News(7)] {
        assert_eq!(view(letter).tick(), None, "{letter:?} stands still");
    }
}

/// **208.217: in the steps of «Привет» the name of the key is in bold** — so in the mock-up of the
/// letters, by the marks of «Как пользоваться» (П2, 208.209): every step carries the key as a
/// strong run, in every language, and nothing else of the step is marked.
#[test]
fn the_steps_of_hello_name_the_key_in_bold() {
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::scene::{Node, strong};
    use lang_switcher::ui::windows::letter::LetterView;

    fn steps<I>(node: &Node<I>, into: &mut Vec<String>) {
        match node {
            Node::Numbered(items) => into.extend(items.iter().cloned()),
            Node::Column(nodes) | Node::Rows(nodes) | Node::Row(nodes) => {
                nodes.iter().for_each(|node| steps(node, into))
            }
            _ => {}
        }
    }

    let _guard = with_product_strings();
    let context = letters::PlanContext {
        version: "1.8.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: true,
        item: None,
    };
    for language in Language::ALL {
        let hello = LetterView {
            letter: Letter::Welcome,
            plan: letters::plan_in(language, Letter::Welcome, &context),
            key: "Pause".to_owned(),
            tick: 0,
            language,
            theme: lang_switcher::theme::ThemeSetting::Light,
        };
        let mut shown = Vec::new();
        steps(&hello.describe().body, &mut shown);
        let ids = [
            settings::IDS_HELLO_ROW_1,
            settings::IDS_HELLO_ROW_2,
            settings::IDS_HELLO_ROW_3,
        ];
        assert_eq!(shown.len(), ids.len(), "{language:?}: three steps");
        for (step, id) in shown.iter().zip(ids) {
            let line = settings::text_in(language, id);
            assert!(line.contains("{0}"), "{language:?}: each step names the key");
            assert_eq!(
                *step,
                line.replace("{0}", &strong("Pause")),
                "{language:?}: the key in bold, the rest as the string says"
            );
        }
    }
}

/// **«Обновление» dates the release in the language of its window** — the same as «От автора»
/// below (T-120-17): `plan_in` speaks `language`, and the day of the release came out in the
/// program's («Version 1.7.1 came out on 16 ноября» in Tamil, the list of the wave of T-120-24).
#[test]
fn the_update_letter_dates_the_release_in_the_language_of_its_window() {
    use lang_switcher::letters::{format_date_in, sample_entry};

    let _guard = with_product_strings();

    let item = sample_entry();
    let day = item.date.expect("the sample entry carries its day");
    let context = letters::PlanContext {
        version: "1.7.0".to_owned(),
        hotkey: String::new(),
        snooze_offered: false,
        item: Some(&item),
    };
    let plan = letters::plan_in(Language::De, Letter::Update, &context);
    let german = format_date_in(Language::De, day);
    let russian = format_date_in(Language::Ru, day);

    println!("De: {} (the program speaks Russian)", plan.note);

    assert_ne!(
        german, russian,
        "the control: the two languages name the month apart"
    );
    assert!(
        plan.note.contains(&german) && !plan.note.contains(&russian),
        "the notice of «Обновление» must carry «{german}», the day in the language of the window: {}",
        plan.note
    );
}

/// **T-120-17: «От автора» dates the reading of the feed in the language of its window**, not in
/// the program's. The two are one in the running program; a description built for another language
/// — the pictures of C3, the instrument of fitting, the window built again after a change — printed
/// the day with the Russian month of the program («16 ноября» in Hebrew).
#[test]
fn the_author_window_dates_the_feed_in_the_language_of_its_window() {
    use lang_switcher::letters::{FeedView, author_view_in, format_date_in, sample_entry};

    let _guard = with_product_strings();

    let item = sample_entry();
    let today = item.date.expect("the sample entry carries its day");
    let state = Letters {
        feed_last_read: Some(today),
        ..Letters::default()
    };
    let view = author_view_in(
        Language::De,
        &state,
        today,
        "1.7.0",
        FeedView {
            update: None,
            news: &[],
        },
    );
    let german = format_date_in(Language::De, today);
    let russian = format_date_in(Language::Ru, today);

    println!("De: {} (the program speaks Russian)", view.feed_state);

    assert_ne!(
        german, russian,
        "the control: the two languages name the month apart"
    );
    assert!(
        view.feed_state.contains(&german) && !view.feed_state.contains(&russian),
        "the line of the feed must carry «{german}», the day in the language of the window: {}",
        view.feed_state
    );
}

/// **T-120-17: the two lines of the state of «От автора» are the element `Status` of the shell, and
/// its dot says whether the line is good** (passport §6): the feed read — the accent, never read —
/// the quiet dot; the version the latest — the accent, a newer one announced — the warning. Both
/// ways, so that a window that put one dot on every line is caught; and through `author_view_in`,
/// so that the view says it from the state and not the test.
#[test]
fn the_lines_of_the_state_of_the_author_window_say_by_their_dot_whether_they_are_good() {
    use lang_switcher::letters::{FeedView, author_view_in, sample_entry};
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::scene::{MenuState, Node};
    use lang_switcher::ui::windows::author::{Author, Id};

    let _guard = with_product_strings();

    let newer = sample_entry();
    let today = newer.date.expect("the sample entry carries its day");
    let cases = [
        (
            Some(today),
            Some(&newer),
            MenuState::Working,
            MenuState::Restart,
        ),
        (None, None, MenuState::Paused, MenuState::Working),
    ];

    for (read, update, feed, version) in cases {
        let state = Letters {
            feed_last_read: read,
            ..Letters::default()
        };
        let view = author_view_in(
            Language::Ru,
            &state,
            today,
            "1.7.0",
            FeedView { update, news: &[] },
        );
        let lines = [
            Node::<Id>::Status {
                state: feed,
                words: view.feed_state.clone(),
            },
            Node::<Id>::Status {
                state: version,
                words: view.version_state.clone(),
            },
        ];
        let window = Author {
            view,
            language: Language::Ru,
            theme: ThemeSetting::Light,
        }
        .describe();
        let body = format!("{:?}", window.body);

        for line in lines {
            println!("read {read:?}, update {}: {line:?}", update.is_some());
            assert!(
                body.contains(&format!("{line:?}")),
                "read {read:?}, update {}: the window must carry {line:?}",
                update.is_some()
            );
        }
    }
}

/// **208.209, AUDIT-180 А5 — the switch under the block of the news.** «Сообщать об обновлениях
/// программы» is a line of its own right after the block «Новости и обновления», not a row inside
/// it: the word of the owner moved it out of the block of the mock-up.
#[test]
fn the_switch_of_the_author_window_stands_under_the_block_of_the_news() {
    use lang_switcher::letters::author_view_sample;
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::scene::{Node, Surface};
    use lang_switcher::ui::windows::author::{Author, Id};

    let _guard = with_product_strings();
    let news = settings::text_in(Language::Ru, settings::IDS_NEWS_PANEL);
    let window = Author {
        view: author_view_sample(Language::Ru),
        language: Language::Ru,
        theme: ThemeSetting::Light,
    }
    .describe();
    let Node::Column(children) = &window.body else {
        panic!("the body of «От автора» is a column: {:?}", window.body);
    };
    let block = children
        .iter()
        .position(|node| {
            matches!(node, Node::Surface(Surface::Block, _)) && format!("{node:?}").contains(&news)
        })
        .expect("the block of the news");
    let switch = children
        .iter()
        .position(|node| matches!(node, Node::Toggle { id: Id::Switch, .. }))
        .expect("the switch, a node of the column itself");

    assert_eq!(switch, block + 1, "the switch right under the block");
    assert!(
        !format!("{:?}", children[block]).contains("Switch"),
        "no switch inside the block: {:?}",
        children[block]
    );
}

/// **T-120-17, the red «before»: «От автора» is a window of the shell and fits the narrowest
/// window, 620 × 500, at a type not under 12 points** — in every language, at the factor of the
/// table, laid out by the layout of the shell and measured by DirectWrite; the pair of its cards
/// stands side by side there (the rule of the pair). A language that needs more than the pair gives
/// way to is named by the instrument of fitting (`tests\ui_fit.rs`) and the common list of what is
/// excused.
///
/// Before the task «От автора» was the dialog `IDD_AUTHOR`: there was no description to lay out, and
/// its template did not fit 620 × 500 (the mock-up's measure: 84 points short).
#[test]
fn the_author_window_fits_the_narrowest_window() {
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::{Class, client_size, lay_out};
    use lang_switcher::ui::text::{Direction, Text};
    use lang_switcher::ui::tokens::TYPE_FLOOR;
    use lang_switcher::ui::windows::catalogue;

    let _guard = with_product_strings();

    let mut faults = Vec::new();
    for language in [Language::Ru, Language::De, Language::En, Language::He] {
        let Some((_, window)) = catalogue(language)
            .into_iter()
            .find(|(name, _)| name == "author")
        else {
            faults.push(format!(
                "{language:?}: «От автора» is not a window of the shell"
            ));
            continue;
        };
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("DirectWrite");
        let k = fit::k_for(language.tag(), Class::Narrow);
        let layout = lay_out(&window, client_size(620.0, 500.0), k, rtl, &text);
        if !layout.shortfall.fits() {
            faults.push(format!(
                "{language:?}: does not fit 620 × 500, {:.1} down",
                layout.shortfall.h
            ));
        }
        if layout.b < TYPE_FLOOR.0 {
            faults.push(format!("{language:?}: type {:.2} under 12", layout.b));
        }
    }
    assert!(faults.is_empty(), "«От автора»:\n{}", faults.join("\n"));
}

/// **AUDIT-180 А1, решение 208.211 — the owner's text split without a loss.** The title of «От
/// автора» is the first sentence of the owner's text, without its full stop; the card «Поддержать
/// автора» says the rest word for word; together they are the text the owner wrote.
#[test]
fn the_title_and_the_card_of_the_author_window_are_the_owners_text_split() {
    use lang_switcher::letters::author_view_sample;
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::windows::author::Author;

    let _guard = with_product_strings();
    let title = settings::text_in(Language::Ru, settings::IDS_AUTHOR_TITLE);
    let card = settings::text_in(Language::Ru, settings::IDS_AUTHOR_TEXT);
    assert_eq!(
        format!("{title}. {card}"),
        "Программу делает один человек в свободное время. Программа бесплатная и останется \
         такой. Если хотите сказать спасибо делом, на странице поддержки есть несколько \
         способов. Классно, что вы цените чужой труд ♥",
        "the title and the card are the owner's text, no word lost (208.180 п. 1, 208.211)"
    );
    let window = Author {
        view: author_view_sample(Language::Ru),
        language: Language::Ru,
        theme: ThemeSetting::Light,
    }
    .describe();
    let body = format!("{:?}", window.body);
    assert!(
        body.contains(&format!("title: {title:?}")) && body.contains(&format!("words: {card:?}")),
        "the head says the first sentence and the card the rest: {}",
        &body[..body.len().min(300)]
    );
}
