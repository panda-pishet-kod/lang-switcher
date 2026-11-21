//! **One word for one thing in the strings a person reads** — stage E120, task T-120-10
//! (решение 208.169: the review of the interface of 2026-11-19, С9 and С11, notes 11 and 13; the
//! passport of the styles of E120, §8 «Словарь кнопок»).
//!
//! Before the dictionary the main action of the program was named by four words — «исправляет»,
//! «чинит», «перекодирует», «конвертирует» — closing a window by five, and «remind me later» by
//! two. The wave of translations into 66 languages translates a term once per language, so the
//! Russian and the English strings say one word for each meaning first:
//!
//! * the main action is «исправить» / `fix`;
//! * a button that closes a window says one of the words of §8 of the passport;
//! * both buttons that put a letter off say the same words, and the foot of the news letter quotes
//!   that button as it is.
//!
//! The guard reads `res/lang/ru.rc` and `res/lang/en.rc` key by key — the value of each string,
//! never the file as a substring: the comments of a language file may name the old words, and do.
//! The 64 other languages are not held here: the wave of translations brings them to the
//! dictionary, `l10n\GLOSSARY-E120.md` in the control folder is its brief. Every exception is a
//! line of a list below, with its reason, and every refusal names the file, the key and the word.
//! Red «до»: on `6d128bf` (`release/1.8.0` of 2026-11-19) all three tests.
//!
//! Task T-120-21, part A, adds the fourth: the block «Состояние» speaks to a person — whether the
//! program works, what it fixes, by which key, whether it starts with Windows — and the words of the
//! developer stay under «Подробности для автора». Red «до»: on `e073fe4` the keys of the block are
//! not there.

#[path = "support/resource_script.rs"]
mod resource_script;

use std::collections::BTreeMap;

/// The strings of one language file as id → text, quotes doubled inside a string undone.
///
/// A reader of values only: the form of the file — its code page line, its blocks, its escapes,
/// a key met twice — is the guard of `tests\l10n.rs`, which refuses a broken file by name. This
/// one trusts that form and takes the text between the first and the last quote of each line of
/// a `STRINGTABLE` block.
fn strings_of(tag: &str) -> BTreeMap<String, String> {
    let files = resource_script::language_files();
    let (_, text) = files
        .iter()
        .find(|(file, _)| file == tag)
        .unwrap_or_else(|| panic!("app.rc must include res/lang/{tag}.rc"));
    let mut strings = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if !line.starts_with("IDS_") {
            continue;
        }
        let (Some(open), Some(close)) = (line.find('"'), line.rfind('"')) else {
            continue;
        };
        if close <= open {
            continue;
        }
        let id = line[..open].trim().to_owned();
        strings.insert(id, line[open + 1..close].replace("\"\"", "\""));
    }
    assert!(
        strings.len() > 200,
        "{tag}.rc: read {} strings — the reader missed the tables",
        strings.len()
    );
    strings
}

/// The words the main action is no longer called, as roots matched in lower case. «чини» catches
/// «чинит», «чинить» and «починить»; «перекодир» — «перекодирует», «перекодировка»; «переводит» —
/// «переводить» for the action (the noun «перевод» of a translation does not match); `convert` —
/// `converts`, `converted`; `conversion` does not contain `convert` and stands on its own.
const OLD_ACTION_ROOTS: [(&str, &[&str]); 2] = [
    ("ru", &["перекодир", "конверт", "чини", "переводит"]),
    ("en", &["convert", "conversion"]),
];

/// Strings where a root of [`OLD_ACTION_ROOTS`] is not the action of the program — file, key,
/// root, why. The strings of the journal and of the diagnostics for the author carry none of the
/// roots today, so none of them is listed: a line here that matches nothing is refused too.
const NOT_THE_ACTION: [(&str, &str, &str, &str); 1] = [(
    "ru",
    "IDS_THANKS_PARA",
    "чини",
    "«только узнав о проблеме, её можно починить» — чинит автор, а не программа",
)];

#[test]
fn the_main_action_is_called_fix_in_russian_and_english() {
    let mut refused = Vec::new();
    let mut excused = vec![0_usize; NOT_THE_ACTION.len()];
    for (tag, roots) in OLD_ACTION_ROOTS {
        for (id, text) in &strings_of(tag) {
            let lower = text.to_lowercase();
            for root in roots.iter().filter(|root| lower.contains(*root)) {
                let excuse = NOT_THE_ACTION
                    .iter()
                    .position(|(file, key, word, _)| *file == tag && key == id && word == root);
                match excuse {
                    Some(index) => excused[index] += 1,
                    None => refused.push(format!(
                        "{tag}.rc: {id} — «{root}»: главное действие называется «исправить» / \
                         fix — {text}"
                    )),
                }
            }
        }
    }
    for ((tag, id, root, why), used) in NOT_THE_ACTION.iter().zip(&excused) {
        if *used == 0 {
            refused.push(format!(
                "{tag}.rc: {id} — исключение «{root}» ({why}) больше не нужно: строка его не \
                 несёт, уберите строку из NOT_THE_ACTION"
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "{} отказов:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// The keys of the buttons that close a window. The last step of the wizard closes with
/// `IDS_CLOSE` too; the settings dialog of today is not among them ([`NOT_A_CLOSE_BUTTON`]).
const CLOSE_BUTTONS: [&str; 4] = [
    "IDS_ABOUT_OK",
    "IDS_CLOSE",
    "IDS_HELLO_OK",
    "IDS_NEWS_READ_BUTTON",
];

/// The words of §8 of the passport, Russian and English side by side: close a window where there
/// is nothing to keep; the person confirms reading; mark the news read.
const CLOSE_WORDS: [(&str, &str); 3] = [
    ("Закрыть", "Close"),
    ("Понятно", "Got it"),
    ("Прочитано", "Mark as read"),
];

/// The words a window that closes no longer says — «ОК» and «Готово», §8: settings apply at
/// once. Matched against the whole value of every key, not only of [`CLOSE_BUTTONS`]: a new
/// button that closes is caught by its word.
const GONE_WORDS: [(&str, &[&str]); 2] = [("ru", &["ОК", "Готово"]), ("en", &["OK", "Done"])];

/// Keys that may say a word of [`GONE_WORDS`] because they do not close a window — key, why.
const NOT_A_CLOSE_BUTTON: [(&str, &str); 3] = [
    (
        "IDS_OK",
        "the old settings dialog: «ОК» applies and closes; it lives until task T-120-20 moves \
         it onto the shell, where the settings apply at once",
    ),
    (
        "IDS_CANCEL",
        "the old settings dialog: «Отмена» throws away; it goes with the dialog, T-120-20",
    ),
    (
        "IDS_WIZARD_DONE",
        "not a button: the word in the place of «Шаг N из M» on the last step of the wizard \
         (letters.rs, fill_wizard); the button of that step is IDS_CLOSE",
    ),
];

#[test]
fn the_buttons_that_close_a_window_say_the_words_of_the_passport() {
    let ru = strings_of("ru");
    let en = strings_of("en");
    let mut refused = Vec::new();

    for id in CLOSE_BUTTONS {
        let (Some(russian), Some(english)) = (ru.get(id), en.get(id)) else {
            refused.push(format!(
                "{id} — нет в ru.rc или en.rc: уберите из CLOSE_BUTTONS"
            ));
            continue;
        };
        match CLOSE_WORDS.iter().find(|(word, _)| word == russian) {
            None => refused.push(format!(
                "ru.rc: {id} — «{russian}»: кнопка закрытия говорит одно из слов паспорта §8: {}",
                CLOSE_WORDS.map(|(word, _)| word).join(", ")
            )),
            Some((_, pair)) if pair != english => refused.push(format!(
                "en.rc: {id} — «{english}»: русское «{russian}» по-английски — «{pair}»"
            )),
            Some(_) => {}
        }
    }

    for (tag, words) in GONE_WORDS {
        let strings = if tag == "ru" { &ru } else { &en };
        for (id, text) in strings {
            let excused = NOT_A_CLOSE_BUTTON.iter().any(|(key, _)| key == id);
            if words.contains(&text.as_str()) && !excused {
                refused.push(format!(
                    "{tag}.rc: {id} — «{text}»: в окнах закрытия «ОК» и «Готово» больше нет, \
                     паспорт §8"
                ));
            }
        }
    }
    for (id, why) in NOT_A_CLOSE_BUTTON {
        if !ru.contains_key(id) {
            refused.push(format!(
                "{id} — ключа нет ({why}): уберите его из NOT_A_CLOSE_BUTTON"
            ));
        }
    }

    assert!(
        refused.is_empty(),
        "{} отказов:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// The two buttons that put a letter off — «Спасибо» and «Новость». The button says that it
/// reminds; when it reminds is said by the foot of each letter.
const REMIND_LATER: [&str; 2] = ["IDS_THANKS_SNOOZE", "IDS_NEWS_LATER"];

#[test]
fn remind_me_later_is_one_word_and_the_foot_of_the_news_quotes_it() {
    let mut refused = Vec::new();
    for (tag, open, close) in [("ru", '«', '»'), ("en", '“', '”')] {
        let strings = strings_of(tag);
        let [thanks, news] = REMIND_LATER.map(|id| strings[id].as_str());
        if thanks != news {
            refused.push(format!(
                "{tag}.rc: {} — «{thanks}», {} — «{news}»: «напомнить позже» — одно слово во \
                 всех окнах",
                REMIND_LATER[0], REMIND_LATER[1]
            ));
        }
        let quoted = format!("{open}{news}{close}");
        let foot = &strings["IDS_NEWS_FOOT"];
        if !foot.contains(&quoted) {
            refused.push(format!(
                "{tag}.rc: IDS_NEWS_FOOT не цитирует кнопку {quoted}: {foot}"
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "{} отказов:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// The strings of the block «Состояние» a person sees before «Подробности для автора» — task
/// T-120-21, part A (решение 208.169, С10): the state of the program in three words, what it fixes,
/// the key, the start with Windows, and the button that opens the rest. The lines of the developer —
/// `IDS_STATE_*`, `IDS_AUTOSTART_PRESENT`, `IDS_AUTOSTART_ABSENT` — stay unchanged under that button.
const STATUS_BLOCK: [&str; 9] = [
    "IDS_STATUS_WORKING",
    "IDS_STATUS_SUSPENDED",
    "IDS_STATUS_RESTART",
    "IDS_STATUS_FIXES",
    "IDS_STATUS_FIXES_NONE",
    "IDS_STATUS_HOTKEY",
    "IDS_STATUS_AUTOSTART_ON",
    "IDS_STATUS_AUTOSTART_OFF",
    "IDS_STATUS_DETAILS",
];

/// The three states of the program — one first line each, as the menu of the icon names them.
const STATUS_STATES: [&str; 3] = [
    "IDS_STATUS_WORKING",
    "IDS_STATUS_SUSPENDED",
    "IDS_STATUS_RESTART",
];

/// The words of the developer the block does not say, in lower case: the registry and its path,
/// the published exclusions, the hook, the failures, the journal — what the lines under
/// «Подробности для автора» are made of. A backslash is a path.
const DEVELOPER_WORDS: [(&str, &[&str]); 2] = [
    (
        "ru",
        &[
            "реестр",
            "hkcu",
            "опубликован",
            "перехват",
            "отказ",
            "журнал",
            "\\",
        ],
    ),
    (
        "en",
        &[
            "registry",
            "hkcu",
            "published",
            "hook",
            "failure",
            "journal",
            "\\",
        ],
    ),
];

#[test]
fn the_status_block_speaks_to_a_person_before_the_details() {
    let mut refused = Vec::new();
    for (tag, words) in DEVELOPER_WORDS {
        let strings = strings_of(tag);
        for id in STATUS_BLOCK {
            let Some(text) = strings.get(id) else {
                refused.push(format!(
                    "{tag}.rc: {id} — ключа нет: строки блока «Состояние»"
                ));
                continue;
            };
            let lower = text.to_lowercase();
            for word in words.iter().filter(|word| lower.contains(*word)) {
                refused.push(format!(
                    "{tag}.rc: {id} — «{word}»: до «Подробностей для автора» блок говорит \
                     словами человека — {text}"
                ));
            }
        }
        let firsts: Vec<&str> = STATUS_STATES
            .iter()
            .filter_map(|id| strings.get(*id).map(String::as_str))
            .collect();
        let mut distinct = firsts.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() != firsts.len() {
            refused.push(format!(
                "{tag}.rc: три состояния программы — три разные первые строки: {firsts:?}"
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "{} отказов:\n{}",
        refused.len(),
        refused.join("\n")
    );
}
