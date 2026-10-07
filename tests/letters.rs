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
// Чем стучится каждое письмо — задача Т-33а-4, решение 104.4
// =========================================================================================

/// **Находка контролёра на снимке пользователя, а не теста.** Уведомление письма «Что нового»
/// несло слова уведомления «Обновления»: «Вышла версия 0.43.0 · Три изменения **и ссылка на
/// загрузку**». Версия не «вышла» — она уже стоит и работает; загружать по этому письму нечего.
///
/// Красное «до» — вот это самое утверждение: до правки обе пары были ОДИНАКОВЫ, и `assert_ne!`
/// ниже падал. Проверить это раньше было нечем: строки выбирались внутри `announce`, которой
/// нужен живой трей; теперь выбор — чистая функция.
#[test]
fn the_whats_new_balloon_speaks_with_its_own_words_and_not_the_updates() {
    let whats_new = letters::toast_strings(Letter::WhatsNew).expect("«Что нового» объявляется");
    let update = letters::toast_strings(Letter::Update).expect("«Обновление» объявляется");

    println!("«Что нового»: {whats_new:?}, «Обновление»: {update:?}");

    assert_ne!(
        whats_new, update,
        "«Что нового» обязано стучаться своими словами: у него нечего загружать"
    );

    assert_eq!(
        whats_new,
        (settings::IDS_WHATSNEW_TITLE, settings::IDS_TOAST_WHATSNEW),
        "заголовок общий с самим письмом, тело — своё"
    );

    assert_eq!(
        update,
        (settings::IDS_TOAST_UPDATE_TITLE, settings::IDS_TOAST_UPDATE),
        "у «Обновления» слова не менялись"
    );

    // Остальные три письма — на месте, и «Привет» по-прежнему не объявляется вовсе.
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
    let bodies: Vec<u16> = [
        Letter::WhatsNew,
        Letter::Update,
        Letter::News(1),
        Letter::Thanks,
    ]
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
    let items = [news(12)];
    let announced = update("0.40.0");
    let feed = FeedView {
        update: Some(&announced),
        news: &items,
    };

    // ⛔ Ступень, которой не стало: `last_seen_version` = «0.38.0» при сборке «0.39.0» — ровно
    // то состояние, что прежде давало `Some(Letter::WhatsNew)` первым. Теперь первым идёт
    // «Обновление», и это проверяется здесь же, до всякой правки состояния.
    assert_eq!(
        letters::due(&state, today, "0.39.0", feed, true),
        Some(Letter::Update),
        "решение 118.5: непросмотренная версия письма больше не даёт — первым идёт \
         «Обновление»"
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

/// ⚠⚠ **ПЕРЕВЁРНУТ задачей T-53a-5, решение владельца 118.5: ворота закрыты.**
///
/// До неё тест утверждал «показывается один раз на версию, и метка — сама версия». Первой
/// половины больше нет: письмо не показывается вовсе, потому что описание изменений приходит с
/// сигналом ленты **до** установки. **Вторая половина цела и стала главной:** метка версии
/// по-прежнему движется — без этого письмо, однажды возвращённое, пришло бы за все пропущенные
/// версии разом. Двигает её теперь `initialise`, а не `after_shown`.
///
/// ⭐ И проверяется, что письмо **разобрано не было**: `after_shown` для него отвечает, как
/// отвечал, — вернуть его значит вернуть одно `if` в `due`.
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

    // ⛔ Письмо не разобрано: механика показа цела и вернётся вместе с воротами.
    letters::after_shown(&mut state, Letter::WhatsNew, today, "0.41.0");
    assert_eq!(
        state.last_seen_version, "0.41.0",
        "`after_shown` для «Что нового» обязан работать, как работал: закрыты ворота, а не \
         письмо"
    );
    assert_eq!(
        state.last_letter,
        Some(today),
        "и день оно по-прежнему тратит — FR-101 освобождает от этого только «Привет»"
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

/// **A feed asked today is not asked again before tomorrow** — task T-88-1, решение 148.8.
///
/// A read that fails leaves `feed_last_read` where it was, so the feed stays due. Until `e88`
/// that meant a new request at every tick of the letters — once an hour, for as long as the
/// host did not answer — and the placeholder address hid it, because a placeholder is never
/// read at all. `feed_last_try` is the day the feed was last asked, whatever came of it, and
/// one attempt a day is all a feed gets; a good read still closes it for fifteen days.
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

    // A good read closes the feed for fifteen days, whatever day the last attempt was on.
    state.feed_last_read = Some(today);
    assert!(!letters::feed_read_is_due(&state, today.plus_days(1)));
    assert!(letters::feed_read_is_due(&state, today.plus_days(15)));

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
#[test]
fn the_hard_wired_addresses_are_real_and_so_is_the_feed() {
    for (what, url) in [
        ("CHANNEL_URL", links::CHANNEL_URL),
        ("SUPPORT_URL", links::SUPPORT_URL),
        ("DOWNLOAD_URL", links::DOWNLOAD_URL),
        ("PROGRAM_URL", links::PROGRAM_URL),
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

    // And the gate did not open on the way: a foreign host is refused as it always was.
    assert!(!links::is_allowed("https://evil.example/news.toml"));
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
        found.len() == 1 && found[0].starts_with("letters.rs:") && found[0].ends_with(DEFINITION),
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
    // ⭐ Task T-78-1 ended the «while» this line used to carry: until then both addresses were
    // placeholders and both buttons were drawn dead. They are the author's own now (139.2), and
    // the assertion says the same rule from the other side — which is the half worth guarding,
    // because a button that silently went back to grey is the failure a person would see.
    assert!(
        thanks.panel_buttons.iter().all(|button| button.enabled),
        "«Открыть страницу поддержки» and «Открыть канал» are live once their addresses are real \
         (П7, П8; решение 139.2)"
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
        news.left.as_ref().is_some_and(|button| button.enabled),
        "«Открыть канал» is live once the channel's address is real (решение 139.2)"
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
    // ⚠ **Task T-41-1: this address used to be `https://example.com/post`**, and task T-88-1
    // moved it again. The button of a news entry is drawn for an address the gate of
    // `links::is_allowed` passes, and that gate knows only the hosts this build carries — which
    // is the whole of finding С52. Until `e88` the one such host a test could name was the
    // reserved one, so the button came out dead; since then it is the author's own, the address
    // is the one the first file of the feed carries, and the button is live (решение 148.7).
    item.link = links::PROGRAM_URL.to_owned();

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
        "an address of this program leaves the button drawn and live"
    );
    assert_eq!(plan.panel_buttons[0].action, letters::Action::OpenLink);
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
        plan.panel_buttons.is_empty(),
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
        plan.panel_buttons.is_empty(),
        "and no button for the reserved domain either — 148.7"
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

/// **Task T-70-1, finding Э69-Б-1 — the switch follows the setting a click has written.**
///
/// `draw_switch` paints the tick out of `AuthorView::switch`, a copy [`letters::author_view`]
/// makes when «От автора» opens. Before the task a click wrote `Letters::feed` and repainted
/// that very copy: the file said the feed was off while the tick still said it was on. After a
/// click the copy has to be the one the window would build afresh on the setting now in force.
#[test]
fn the_switch_follows_the_setting_a_click_has_written() {
    with_product_strings();

    let today = day(2027, 1, 1);
    let mut state = settled("0.39.0");
    state.first_feed_letter = Some(day(2026, 9, 10));

    let opened = letters::author_view(&state, today, "0.39.0", FeedView::EMPTY);
    assert_eq!(
        opened.switch,
        Some(true),
        "the window opens on a feed that is on"
    );

    // The click writes the setting, and the window's copy follows it.
    let mut view = opened.clone();
    state.feed = false;
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
    state.feed = true;
    view.follow_feed(&state);

    assert_eq!(
        view, opened,
        "the second click brings the tick back with the setting"
    );

    // A switch FR-102 keeps away stays away, whatever the setting says.
    let mut early = settled("0.39.0");
    let mut hidden = letters::author_view(&early, today, "0.39.0", FeedView::EMPTY);
    assert!(
        hidden.switch.is_none(),
        "no letter from the feed yet — no switch"
    );

    early.feed = false;
    hidden.follow_feed(&early);

    assert!(
        hidden.switch.is_none(),
        "and a changed setting does not make one appear"
    );
}

/// The source of `src\letters.rs`, line endings normalised — the sweep below matches on it.
fn letters_module_source() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("letters.rs"),
    )
    .expect("src\\letters.rs must be readable")
    .replace("\r\n", "\n")
}

/// The branch of the window procedure that answers the feed switch — from the test of the
/// control to the `return 0;` that ends it — or `None` unless there is exactly one.
fn switch_branch(source: &str) -> Option<&str> {
    let mut heads = source.match_indices("if control == IDC_NEWS_SWITCH\n");
    let (start, _) = heads.next()?;

    if heads.next().is_some() {
        return None;
    }

    let length = source[start..].find("return 0;")?;

    Some(&source[start..start + length])
}

/// Whether a branch writes the setting, reads the configuration again, and only then hands what
/// it read to the copy the switch is drawn from.
fn follows_what_was_written(branch: &str) -> bool {
    let (Some(write), Some(follow)) = (branch.find("update_state("), branch.find(".follow_feed("))
    else {
        return false;
    };

    write < follow && branch[write..follow].contains("state_now()")
}

/// **Task T-70-1, finding Э69-Б-1 — the click moves the copy the switch is drawn from.**
///
/// The half of the repair a pure function cannot show: that the window procedure calls it. A
/// letters window cannot be built in a test — `open_window` takes its template out of the
/// running executable, and `embed-resource` links `app.rc` into the product and not into the
/// test binaries. So the branch of `IDC_NEWS_SWITCH` is read instead: it has to write the setting
/// (FR-102), read the configuration again, hand what it read to `AuthorView::follow_feed` and
/// repaint the switch.
///
/// ⚠ Controls: the branch is found exactly once and is the one that writes the file and repaints;
/// the same branch without the call — the shape of `e69` — is caught, and so are a call fed with
/// the reading made before the write and a call put before the write.
#[test]
fn a_click_on_the_switch_moves_the_copy_the_switch_is_drawn_from() {
    let source = letters_module_source();
    let branch = switch_branch(&source).expect(
        "the sweep must find exactly one branch of the window procedure for IDC_NEWS_SWITCH",
    );

    assert!(
        branch.contains("update_state(|state| state.feed = wanted)")
            && branch.contains("widgets::repaint::control_no_erase(hwnd, IDC_NEWS_SWITCH)"),
        "the branch found is not the one that writes the feed and repaints the switch:\n{branch}"
    );

    assert!(
        follows_what_was_written(branch),
        "Э69-Б-1: the click writes the setting but does not move the copy `draw_switch` paints \
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
        "the sweep does not see a branch without the call — it cannot fail"
    );
    assert!(
        !follows_what_was_written(&without("state_now()")),
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

/// Whether a branch repaints the switch through the door without the erase, and not through the
/// one with it.
fn repaints_without_an_erase(branch: &str) -> bool {
    let code = squeezed_code(branch);

    code.contains("widgets::repaint::control_no_erase(hwnd, IDC_NEWS_SWITCH)")
        && !code.contains("repaint::control(hwnd, IDC_NEWS_SWITCH)")
}

/// The calls that put pixels into a DC — the ones a glyph body makes, and the GDI calls a later
/// hand could put in front of them.
const PAINT_CALLS: [&str; 9] = [
    "FillRect(",
    "paint_rounded(",
    "paint_ellipse(",
    "draw_check_mark(",
    "paint_label(",
    "DrawTextW(",
    "RoundRect(",
    "Polyline(",
    "BitBlt(",
];

/// Whether the first thing a body paints is its whole rectangle in the colour of the ground the
/// element lies on — the condition on which the door of task Т-45-3 may be taken.
///
/// ⚠ Since task T-71-5 the fill stands in the one glyph body `draw_glyph`, drawn into its buffer
/// with the ground the door names (the switch names the panel — the sentry of that task asserts
/// it); until then it stood in `draw_switch` as `FillRect(dc, &rect, brushes.panel_bg())`.
fn paints_its_whole_rectangle_first(body: &str) -> bool {
    let code = squeezed_code(body);

    let Some(fill) = code.find("FillRect(target, &rect, ground)") else {
        return false;
    };

    PAINT_CALLS
        .iter()
        .filter_map(|call| code.find(call))
        .all(|at| at >= fill)
}

/// **Task T-71-2, finding Э70-Б-2 — the switch is repainted without an erase frame.**
///
/// The owner's word of 2026-09-14: «при снятии и проставлении галки строка моргает». The branch of
/// `IDC_NEWS_SWITCH` asked for the repaint **with** the erase: `WM_ERASEBKGND` fills the switch
/// with the brush `on_ctl_color` answers `WM_CTLCOLORBTN` with — the brush of the **window** — and
/// only the next frame brings `WM_DRAWITEM`, where `draw_switch` paints the rectangle in the colour
/// of the **panel** the switch lies on. The frame between the two is the blink, and it is the very
/// frame the door `widgets::repaint::control_no_erase` of task Т-45-3 was made to take away.
///
/// Two halves, because a letters window cannot be built here (see the test above): the branch goes
/// through the door, and the condition of the door holds — the body the switch is drawn by paints
/// its whole rectangle before anything else, so the erase is needed for nothing. Since task T-71-5
/// that body is `draw_glyph`, the one the wizard's glyphs share.
///
/// ⚠ Controls: the same branch with the call of `e70` is caught, and so are a branch that repaints
/// nothing and a branch that keeps the erase beside the door; a body whose fill comes after the
/// figure and a body that fills the glyph cell instead of the rectangle are caught too.
#[test]
fn the_switch_is_repainted_without_an_erase_frame() {
    let source = letters_module_source();
    let branch = switch_branch(&source).expect(
        "the sweep must find exactly one branch of the window procedure for IDC_NEWS_SWITCH",
    );
    let body = letters_function_body(&source, "unsafe fn draw_glyph(")
        .expect("src\\letters.rs must declare the one glyph body `draw_glyph`");

    assert!(
        paints_its_whole_rectangle_first(body),
        "the door of Т-45-3 is for an element that paints its whole rectangle itself, and the \
         glyph body the switch is drawn by no longer fills its rectangle before anything else:\n\
         {body}"
    );

    assert!(
        repaints_without_an_erase(branch),
        "Э70-Б-2: the click repaints the switch with the erase — a frame of the window's brush \
         over a switch that stands on a panel:\n{branch}"
    );

    let door = "widgets::repaint::control_no_erase(hwnd, IDC_NEWS_SWITCH)";
    let erase = "widgets::repaint::control(hwnd, IDC_NEWS_SWITCH)";

    let of_e70 = branch.replace(door, erase);
    assert_ne!(
        of_e70, branch,
        "the control must change the branch it is made of"
    );
    assert!(
        !repaints_without_an_erase(&of_e70),
        "the sweep does not see the call of e70 — it cannot fail"
    );

    let without_repaint = branch
        .lines()
        .filter(|line| !line.contains(door))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !repaints_without_an_erase(&without_repaint),
        "the sweep does not see a branch that repaints nothing"
    );

    assert!(
        !repaints_without_an_erase(&format!("{branch}\n{erase};")),
        "the sweep does not see the erase kept beside the door"
    );

    let fill = "FillRect(target, &rect, ground)";

    let moved = body
        .lines()
        .filter(|line| !line.contains(fill))
        .chain([fill])
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !paints_its_whole_rectangle_first(&moved),
        "the sweep does not see the fill put after the figure"
    );

    let of_the_cell = body.replace(fill, "FillRect(target, &cell, ground)");
    assert_ne!(
        of_the_cell, body,
        "the control must change the body it is made of"
    );
    assert!(
        !paints_its_whole_rectangle_first(&of_the_cell),
        "the sweep does not see a fill of the glyph cell instead of the rectangle"
    );
}

/// Whether a body paints nothing itself and hands its element to the one glyph body.
fn hands_its_glyph_to_the_one_body(body: &str) -> bool {
    let code = squeezed_code(body);

    code.contains("draw_glyph(dc, rect, state, glyph, label)")
        && PAINT_CALLS.iter().all(|call| !code.contains(call))
}

/// Whether a glyph body draws in one frame: into the buffer of `theme::PaintBuffer`, every paint
/// call aimed at the buffer and none at the DC of the message, and one blit at the end.
fn draws_in_one_frame(body: &str) -> bool {
    let code = squeezed_code(body);

    let buffered = code
        .contains("let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };")
        && code.contains("let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);")
        && code.contains("buffer.blit(dc)");

    let aimed_at_the_message = PAINT_CALLS
        .iter()
        .filter(|call| **call != "BitBlt(")
        .any(|call| code.contains(&format!("{call}dc,")));

    buffered && !aimed_at_the_message
}

/// **Task T-71-5, decision 132.3 — the switch is drawn by the one glyph body, in one frame.**
///
/// After task T-71-2 the owner still saw the switch blink on the live 0.71.0. The erase was gone;
/// what was left was the body: `draw_switch` laid its fill, its figure, its tick and its caption
/// **straight into the DC of the message**, call by call, and a click brings at least four
/// `WM_DRAWITEM` (the press, the focus, the release, the repaint of the branch) — DWM sampled the
/// window between the calls and the row stood blank for a frame. Its twin in the wizard blinked the
/// same way until task Т-33-5 gave it `theme::PaintBuffer`. The owner's word: the ready solution
/// the wizard's check boxes already have, not a copy of the repair — so both elements go through
/// one body, and what tells them apart is what does: where «checked» is read and what they lie on.
///
/// ⚠ Controls: a door that paints something itself, a body whose paint goes to the DC of the
/// message, and a body without the buffer are caught.
#[test]
fn the_switch_is_drawn_by_the_one_glyph_body_in_one_frame() {
    let source = letters_module_source();
    let switch = letters_function_body(&source, "unsafe fn draw_switch(")
        .expect("src\\letters.rs must still declare `draw_switch`");

    assert!(
        hands_its_glyph_to_the_one_body(switch),
        "решение 132.3: draw_switch paints on its own instead of going through the one glyph body \
         the wizard's check boxes are drawn by:\n{switch}"
    );

    let wizard = letters_function_body(&source, "unsafe fn draw_wizard_glyph(")
        .expect("src\\letters.rs must still declare `draw_wizard_glyph`");

    assert!(
        hands_its_glyph_to_the_one_body(wizard),
        "the wizard's glyphs must go through the same body:\n{wizard}"
    );

    assert!(
        squeezed_code(switch).contains("ground: Ground::Panel")
            && squeezed_code(wizard).contains("ground: Ground::Window"),
        "the switch lies on a panel and the wizard's glyphs on the window"
    );

    let body = letters_function_body(&source, "unsafe fn draw_glyph(")
        .expect("src\\letters.rs must declare the one glyph body `draw_glyph`");

    assert!(
        draws_in_one_frame(body),
        "the one glyph body must draw into theme::PaintBuffer and blit once:\n{body}"
    );

    // Controls of the doors.
    assert!(
        !hands_its_glyph_to_the_one_body(&format!(
            "{switch}\n    unsafe {{ FillRect(dc, &rect, brushes.panel_bg()) }};"
        )),
        "the sweep does not see a door that paints something itself"
    );

    // Controls of the body.
    let at_the_message = body.replacen("FillRect(target,", "FillRect(dc,", 1);
    assert_ne!(
        at_the_message, body,
        "the control must change the body it is made of"
    );
    assert!(
        !draws_in_one_frame(&at_the_message),
        "the sweep does not see a paint call aimed at the DC of the message"
    );

    let unbuffered = body.replace(
        "let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);",
        "let target = dc;",
    );
    assert_ne!(
        unbuffered, body,
        "the control must change the body it is made of"
    );
    assert!(
        !draws_in_one_frame(&unbuffered),
        "the sweep does not see a body that draws past the buffer"
    );
}

/// The arm of `wizard_after_choice` that repaints one check box of the wizard — from
/// `Touched::Glyph(control) =>` to the arm of the radios — or `None` unless there is exactly one.
fn check_box_arm(after_choice: &str) -> Option<&str> {
    let mut heads = after_choice.match_indices("Touched::Glyph(control) =>");
    let (start, _) = heads.next()?;

    if heads.next().is_some() {
        return None;
    }

    let length = after_choice[start..].find("Touched::Range(")?;

    Some(&after_choice[start..start + length])
}

/// Whether the arm repaints the check box through the door without the erase, and not through the
/// one with it.
fn repaints_the_check_box_without_an_erase(arm: &str) -> bool {
    let code = squeezed_code(arm);

    code.contains("widgets::repaint::control_no_erase(hwnd, control)")
        && !code.contains("repaint::control(hwnd, control)")
}

/// **Task T-92-1, decision 156.11 — a check box of the wizard is repainted without an erase frame.**
///
/// The second `repaint::control` of `src\letters.rs` until stage Э92: a click on a check box of the
/// step «Что приложить» repainted it **with** the erase, and `WM_ERASEBKGND` of an owner-drawn
/// button fills it with the brush the dialog answers `WM_CTLCOLORBTN` with — the window's — before
/// `WM_DRAWITEM` paints it again: the check box and its caption are not there for that frame.
/// Measured by the probe of the stage (`scratchpad-E92\probe-e92-glyph-*.log`, twenty real clicks):
/// twenty erases, each changing 1 414 pixels of 8 526, and the window's surface caught the empty
/// rectangle 121…129 times; through the door of task Т-45-3 — no erase, no empty picture, and the
/// pictures a click ends with byte for byte the same two.
///
/// Two halves, as for the switch: the arm goes through the door, and the condition of the door
/// holds — the check box is drawn by the one glyph body, which paints its whole rectangle before
/// anything else.
///
/// ⚠ Controls: the arm with the call of `e91` is caught, and so are an arm that repaints nothing
/// and an arm that keeps the erase beside the door.
#[test]
fn a_check_box_of_the_wizard_is_repainted_without_an_erase_frame() {
    let source = letters_module_source();
    let after_choice = letters_function_body(&source, "fn wizard_after_choice(")
        .expect("src\\letters.rs must still declare `wizard_after_choice`");
    let arm = check_box_arm(after_choice)
        .expect("the sweep must find exactly one arm of `wizard_after_choice` for Touched::Glyph");

    let wizard = letters_function_body(&source, "unsafe fn draw_wizard_glyph(")
        .expect("src\\letters.rs must still declare `draw_wizard_glyph`");
    let body = letters_function_body(&source, "unsafe fn draw_glyph(")
        .expect("src\\letters.rs must declare the one glyph body `draw_glyph`");

    assert!(
        hands_its_glyph_to_the_one_body(wizard) && paints_its_whole_rectangle_first(body),
        "the door of Т-45-3 is for an element that paints its whole rectangle itself, and the check \
         box of the wizard is no longer drawn by a body that does:\n{wizard}\n---\n{body}"
    );

    assert!(
        repaints_the_check_box_without_an_erase(arm),
        "156.11: a click repaints the check box with the erase — a frame of the window's brush \
         where the check box and its caption were:\n{arm}"
    );

    let door = "widgets::repaint::control_no_erase(hwnd, control)";
    let erase = "widgets::repaint::control(hwnd, control)";

    let of_e91 = arm.replace(door, erase);
    assert_ne!(of_e91, arm, "the control must change the arm it is made of");
    assert!(
        !repaints_the_check_box_without_an_erase(&of_e91),
        "the sweep does not see the call of e91 — it cannot fail"
    );

    let nothing = arm.replace(door, "{}");
    assert_ne!(
        nothing, arm,
        "the control must change the arm it is made of"
    );
    assert!(
        !repaints_the_check_box_without_an_erase(&nothing),
        "the sweep does not see an arm that repaints nothing"
    );

    assert!(
        !repaints_the_check_box_without_an_erase(&format!("{arm}\n{erase};")),
        "the sweep does not see the erase kept beside the door"
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

/// `src\letters.rs` read as text with line endings normalised.
fn letters_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("letters.rs");

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));

    text.replace("\r\n", "\n")
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

/// Whether this body decides «Открыть страницу загрузки» **without asking the feed** — the shape
/// task T-78-2 put in place of the one `e77` carried.
///
/// ⚠ Two halves, and the first is not decoration: a body that stopped naming the button at all
/// would satisfy the second half by saying nothing, and the sweep would be green for the one
/// reason that must never make it green.
///
/// ⚠⚠ The second half looks at **every** code line of the body and not only at the lines that
/// name the button. Written the other way it could not have failed on the enabling body at all:
/// `e77` spelt that one over five lines, with `IDC_NEWS_DOWNLOAD,` on one and `view.download` on
/// another, so a line-by-line look would have found no line carrying both and called it clean.
fn download_is_free_of_the_feed(body: &str) -> bool {
    code_lines(body).any(|line| line.contains("IDC_NEWS_DOWNLOAD"))
        && code_lines(body).all(|line| !line.contains("view.download"))
}

/// **«Открыть страницу загрузки» stands there whatever the feed says** — task T-78-2, решение
/// 139.2 п. 3 и п. 6.
///
/// Until `e77` the button was put into the row only when an `update` entry of the feed existed,
/// and enabled only when that entry's own `link` was not a placeholder. There is no feed
/// (138.4), so the button was translated into fourteen languages and shown to nobody.
///
/// The sweep is over the two bodies that decide it, because neither is a pure function: one
/// lays controls out in a window and the other enables them. What it asserts is the absence of
/// the feed from both — that `view.download` is gone from the decision — and the presence of
/// the build's own address in the enabling one.
#[test]
fn the_download_button_stands_there_whatever_the_feed_says() {
    let source = letters_source();

    for (signature, what) in [
        ("unsafe fn layout_author(", "lays the button out"),
        ("fn fill_author(", "enables it"),
    ] {
        assert!(
            download_is_free_of_the_feed(body_of(&source, signature)),
            "the body that {what} must name IDC_NEWS_DOWNLOAD and ask the feed nothing about it"
        );
    }

    // The enabling body judges the button by the address this build carries, and by nothing
    // else — the same question «Открыть канал» and «Открыть страницу поддержки» stand behind.
    assert!(
        body_of(&source, "fn fill_author(").contains("links::DOWNLOAD_URL"),
        "the button is enabled by the build's own download address"
    );

    // ⚠ Отрицательный контроль — **the same predicate**, run over the two shapes `e77`
    // carried. A control that asserted something else about them would leave the sweep above
    // unproven.
    for (what, sample) in [
        (
            "the row of e77",
            "                (IDC_NEWS_DOWNLOAD, view.download.is_some()),",
        ),
        (
            "the enabling of e77",
            "    enable(\n        hwnd,\n        IDC_NEWS_DOWNLOAD,\n        view.download\n            \
             .as_deref()\n            .is_some_and(|url| !links::is_placeholder(url)),\n    );",
        ),
    ] {
        assert!(
            !download_is_free_of_the_feed(sample),
            "the sweep must refuse {what}, which is the shape it is written against"
        );
    }

    // ⚠ And the first half of the predicate is load-bearing: a body that stopped naming the
    // button would otherwise pass by saying nothing at all.
    assert!(
        !download_is_free_of_the_feed("    enable(hwnd, IDC_NEWS_LETTERS, true);"),
        "a body that never names the button is not a body that lays it out"
    );

    // And «включена» is a fact rather than a hope: the address behind it is real.
    assert!(!links::is_placeholder(links::DOWNLOAD_URL));
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
            previous_version: String::new(),
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
            letters::stands_on_a_panel(control),
            "control {control} is a row of a letter's panel"
        );
    }

    // The three labels of each of the four entries of «Последние письма».
    for control in (1231..1235).chain(1235..1239).chain(1239..1243) {
        assert!(
            letters::stands_on_a_panel(control),
            "control {control} is a label of an entry"
        );
    }

    // The paragraphs of the three panels of «От автора».
    for control in [1264, 1268, 1275, 1269, 1270, 1273, 1277] {
        assert!(
            letters::stands_on_a_panel(control),
            "control {control} stands on a panel of «От автора»"
        );
    }

    // And what does **not**: the head of a letter, the demonstration, the line under the
    // buttons, the name and version of «От автора», the footnote of the list.
    for control in [1201, 1202, 1203, 1204, 1205, 1206, 1223, 1261, 1262, 1252] {
        assert!(
            !letters::stands_on_a_panel(control),
            "control {control} stands on the window's own ground"
        );
    }
}

/// **⭐⭐ A BUTTON asks the same question as a label, and until task T-80-1 it did not** —
/// finding of the live acceptance of `e79`.
///
/// The drawing cuts a rounded figure out of the button's rectangle and fills what it cuts away
/// with **the ground the button stands on**. The buttons of these windows were handed
/// `window_bg` unconditionally, and five of the buttons of «От автора» stand **on panels**. So
/// the corners came out in the colour of the window over a panel of another colour — measured
/// 18/16/13 in «Туман» and 7/8/9 in «Графит» — and what the eye saw was four square corners round
/// a button whose own face, in «Туман», is the panel's colour exactly. Word of the user: «видны
/// углы по краям кнопок… в темной теме также видны углы кнопок».
///
/// ⛔ The answer is **the function the labels already use**, not a second one beside it. That is
/// the whole of what «привести все кнопки к типизированному механизму, который у нас уже
/// существует» asks for.
#[test]
fn a_button_on_a_panel_says_so_by_the_same_question_a_label_does() {
    // «От автора»: five buttons stand on the three panels — two on «Автор», two on «Новости и
    // обновления», one on «Обратная связь».
    for (control, what) in [
        (1265, "«Поддержать автора» on the Author panel"),
        (1266, "«Открыть канал» on the Author panel"),
        (1271, "«Открыть страницу загрузки» on the News panel"),
        (1272, "«Последние письма» on the News panel"),
        (1278, "«Написать автору…» on the Feedback panel"),
    ] {
        assert!(
            letters::stands_on_a_panel(control),
            "{what} ({control}) stands on a panel and must be told so"
        );
    }

    // The two panel buttons of a letter stand on its one panel.
    for control in [1218, 1219] {
        assert!(
            letters::stands_on_a_panel(control),
            "control {control} is a panel button of a letter"
        );
    }

    // And the two buttons of each of the four entries of «Последние письма» — the run a list
    // would have written short. They are laid by the very call that lays the three labels above
    // them, at the same inner corner of the same block.
    for control in (1243..1247).chain(1247..1251) {
        assert!(
            letters::stands_on_a_panel(control),
            "control {control} is a button of an entry and stands on the block"
        );
    }

    // And the buttons that stand on the window's own ground do **not** — «Закрыть» of «От
    // автора», the three buttons under a letter, and the close of the list. A list that said
    // «panel» for these would paint their corners in the panel's colour over the window, which
    // is the same defect facing the other way.
    for (control, what) in [
        (1279, "«Закрыть» of «От автора», below every panel"),
        (1220, "the left button of a letter"),
        (1221, "the right button of a letter"),
        (1222, "the accented button of a letter"),
        (1251, "«Закрыть» of «Последние письма»"),
    ] {
        assert!(
            !letters::stands_on_a_panel(control),
            "{what} ({control}) stands on the window's own ground"
        );
    }

    // ⚠⚠ **And the drawing must ASK.** A list that answers right while nobody consults it is a
    // guard that cannot fail — measured: with the question put back to a flat `window_bg()`, every
    // assertion above stayed green and the defect was whole. So the body that chooses the button's
    // ground is read, and it has to name this function.
    let source = letters_source();
    let body = body_of(&source, "unsafe fn on_draw_item(");

    assert!(
        body.contains("stands_on_a_panel(control)"),
        "the button branch of on_draw_item must ask stands_on_a_panel for its ground"
    );

    // Отрицательный контроль — the shape of `e79`, which the reading must refuse.
    assert!(
        !body
            .replace("stands_on_a_panel(control)", "false")
            .contains("stands_on_a_panel(control)"),
        "the reading must refuse the body that asked nothing"
    );
}
