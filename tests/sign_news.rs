//! **`tools\sign-news.ps1` and the author's cards** — stage Э121, task T-121-6 (решение 208.244 пп.
//! 8–9, 11), and the gates of `[[item]]` (1.8.1, 208.242) kept: a mistake of that script switches the
//! feed off for everybody, so its refusals are part of the battery and not of one stage's notes.
//!
//! Every case runs the script with `-Check` on a feed of the test's own in a folder under `%TEMP%`,
//! with the folder `cards\` beside it: **no key is opened and nothing is signed**. The pictures are
//! headers and nothing more — the script reads the signature of the format and the size in pixels
//! out of the header, it does not decode — and the terms are counted from today, so the battery does
//! not go red the day a date of it has passed. The script is the one of this tree; it needs
//! PowerShell 7 (`pwsh`), as its own header says.

use std::path::{Path, PathBuf};
use std::process::Command;

use lang_switcher::letters::Date;

/// The head of a JPEG of `w` × `h`: SOI, APP0, a frame header SOF0 — what the script reads.
fn jpeg(w: u16, h: u16) -> Vec<u8> {
    let mut bytes = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
    ];
    bytes.extend_from_slice(&[0xFF, 0xC0, 0, 17, 8]);
    bytes.extend_from_slice(&h.to_be_bytes());
    bytes.extend_from_slice(&w.to_be_bytes());
    bytes.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1, 0xFF, 0xD9]);
    bytes
}

/// The head of a PNG of `w` × `h`: the signature and `IHDR`.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&w.to_be_bytes());
    bytes.extend_from_slice(&h.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

/// The SHA-256 of `bytes`, in lower-case hexadecimal digits.
fn sum(bytes: &[u8]) -> String {
    let mut hasher = lang_switcher::letters::feed::Sha256::new().expect("CNG");
    assert!(hasher.update(bytes));
    hasher
        .finish()
        .expect("a sum")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A site of the test's own: `news.toml` will be written per case, `cards\` holds the pictures.
fn site() -> PathBuf {
    let place = std::env::temp_dir()
        .join("langsw-e121-sign-news")
        .join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&place);
    let cards = place.join("cards");
    std::fs::create_dir_all(&cards).expect("the test's own folder");
    std::fs::write(cards.join("panel.jpg"), jpeg(1000, 1440)).expect("written");
    std::fs::write(cards.join("full.jpg"), jpeg(2560, 1280)).expect("written");
    std::fs::write(cards.join("panel.png"), png(1000, 1440)).expect("written");
    std::fs::write(cards.join("panel-999.jpg"), jpeg(999, 1440)).expect("written");
    let mut heavy = jpeg(1000, 1440);
    heavy.resize(1100 * 1024, 0);
    std::fs::write(cards.join("heavy.jpg"), heavy).expect("written");
    std::fs::write(cards.join("note.txt"), b"not a picture").expect("written");
    place
}

/// `sign-news.ps1 <feed> -Check`: the exit code and everything it printed.
fn check(script: &Path, feed: &Path) -> (i32, String) {
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(script)
        .arg(feed)
        .arg("-Check")
        .output()
        .expect("PowerShell 7 (pwsh) runs tools\\sign-news.ps1");
    let text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    (output.status.code().unwrap_or(-1), text)
}

const HEAD: &str = "schema = 1\n\n[[item]]\nid = 20\ntype = \"update\"\ndate = 2026-12-01\n\
version = \"1.9.0\"\nlink = \"https://panda-pishet-kod.dev/langswitcher/download/?lang=ru\"\n\
[item.ru]\ntitle = \"Вышла версия 1.9.0\"\ntext = \"Открытки автора.\"\n\
[item.en]\ntitle = \"Version 1.9.0 is out\"\ntext = \"The author's cards.\"\n";

const LANGS: &str = "[card.ru]\ntitle = \"С Новым годом!\"\ntext = \"Спасибо.\"\n\
[card.en]\ntitle = \"Happy New Year!\"\ntext = \"Thank you.\"";

fn card(fields: &str, langs: &str) -> String {
    format!("\n[[card]]\n{fields}\n{langs}\n")
}

/// A text of `lines` lines of 120 characters: each line under the limit of the excerpt (150), the
/// whole over a cap.
fn long(lines: usize) -> String {
    let line = "слово ".repeat(20);
    format!(
        "[card.ru]\ntitle = \"а\"\ntext = \"\"\"\n{}\"\"\"\n[card.en]\ntitle = \"a\"\ntext = \"b\"",
        vec![line; lines].join("\n")
    )
}

/// **Every refusal of `[[card]]` refuses with its reason, the good feed passes and is signed with
/// the sums the script read off the pictures, and the gates of `[[item]]` hold.**
#[test]
fn sign_news_checks_the_cards_and_keeps_the_gates_of_the_entries() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join("sign-news.ps1");
    let place = site();
    let today = lang_switcher::letters::today().expect("today");
    let day = |offset: i64| today.plus_days(offset).to_string();
    let (from, to) = (day(30), day(40));
    let panel = format!(
        "id = 1\nfrom = {from}\nto = {to}\nview = \"panel\"\nside = \"end\"\n\
         image = \"https://panda-pishet-kod.dev/langswitcher/cards/panel.jpg\""
    );
    let full = format!(
        "id = 2\nfrom = {from}\nto = {to}\nview = \"full\"\n\
         image = \"https://panda-pishet-kod.dev/langswitcher/cards/full.jpg\""
    );
    let panel_sum = sum(&jpeg(1000, 1440));
    let with = |fields: String| HEAD.to_owned() + &card(&fields, LANGS);
    let both = HEAD.to_owned() + &card(&panel, LANGS) + &card(&full, LANGS);

    let cases: Vec<(&str, String, bool, String)> = vec![
        (
            "good: panel and full",
            both.clone(),
            true,
            "card 2    picture JPEG, 2560 x 1280".into(),
        ),
        (
            "good: the author's sum and size replaced",
            with(format!("{panel}\nsha256 = \"00\"\nsize = 1")),
            true,
            format!("signed: sha256 = {panel_sum}"),
        ),
        (
            "good: a PNG",
            with(panel.replace("panel.jpg", "panel.png")),
            true,
            "picture PNG, 1000 x 1440".into(),
        ),
        (
            "good: a quoted date",
            with(panel.replace(&format!("from = {from}"), &format!("from = \"{from}\""))),
            true,
            "card  1".into(),
        ),
        (
            "unknown field",
            with(format!("{panel}\nlayout = \"x\"")),
            false,
            "unknown field 'layout'".into(),
        ),
        (
            "a field twice",
            with(format!("{panel}\nview = \"panel\"")),
            false,
            "gives view twice".into(),
        ),
        (
            "a table under a card",
            HEAD.to_owned() + &card(&panel, "[card.image]\nurl = \"x\""),
            false,
            "is not a language table".into(),
        ),
        (
            "no id",
            with(panel.replace("id = 1\n", "")),
            false,
            "a card with no id".into(),
        ),
        (
            "ids descend",
            HEAD.to_owned() + &card(&full, LANGS) + &card(&panel, LANGS),
            false,
            "identifiers must ascend".into(),
        ),
        (
            "four cards",
            HEAD.to_owned()
                + &(1..=4)
                    .map(|id| card(&panel.replace("id = 1", &format!("id = {id}")), LANGS))
                    .collect::<String>(),
            false,
            "the program keeps 3".into(),
        ),
        (
            "no term",
            with(panel.replace(&format!("to = {to}\n"), "")),
            false,
            "has no term".into(),
        ),
        (
            "term backwards",
            with(panel.replace(&format!("from = {from}"), &format!("from = {}", day(50)))),
            false,
            "runs backwards".into(),
        ),
        (
            "term over",
            with(
                panel
                    .replace(&format!("from = {from}"), &format!("from = {}", day(-20)))
                    .replace(&format!("to = {to}"), &format!("to = {}", day(-10))),
            ),
            false,
            "the term is over".into(),
        ),
        (
            "no such day",
            with(panel.replace(&format!("to = {to}"), "to = 2027-02-30")),
            false,
            "no day of the calendar".into(),
        ),
        (
            "view text",
            with(panel.replace("\"panel\"", "\"text\"")),
            false,
            "panel or full".into(),
        ),
        (
            "side middle",
            with(panel.replace("\"end\"", "\"middle\"")),
            false,
            "side must be start or end".into(),
        ),
        (
            "side of a full card",
            with(format!("{full}\nside = \"end\"")),
            false,
            "a full card has no side".into(),
        ),
        (
            "no image",
            with(
                panel
                    .lines()
                    .filter(|line| !line.starts_with("image"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            false,
            "has no image".into(),
        ),
        (
            "foreign host",
            with(panel.replace("panda-pishet-kod.dev", "example.com")),
            false,
            "image must start with".into(),
        ),
        (
            "the folder of installers",
            with(panel.replace("/cards/", "/files/")),
            false,
            "image must start with".into(),
        ),
        (
            "a path that climbs",
            with(panel.replace("cards/panel.jpg", "cards/../panel.jpg")),
            false,
            "plain path segments".into(),
        ),
        (
            "no file in cards",
            with(panel.replace("panel.jpg", "missing.jpg")),
            false,
            "no picture at".into(),
        ),
        (
            "not a picture",
            with(panel.replace("panel.jpg", "note.txt")),
            false,
            "is not a PNG or a JPEG".into(),
        ),
        (
            "wrong pixels",
            with(panel.replace("panel.jpg", "panel-999.jpg")),
            false,
            "is 1000 x 1440 pixels, found 999 x 1440".into(),
        ),
        (
            "a panel picture for a full card",
            with(full.replace("full.jpg", "panel.jpg")),
            false,
            "is 2560 x 1280 pixels, found 1000 x 1440".into(),
        ),
        (
            "over 1 MB",
            with(panel.replace("panel.jpg", "heavy.jpg")),
            false,
            "the limit is 1048576".into(),
        ),
        (
            "no en",
            HEAD.to_owned() + &card(&panel, "[card.ru]\ntitle = \"а\"\ntext = \"б\""),
            false,
            "has no [en] text".into(),
        ),
        (
            "no text",
            HEAD.to_owned()
                + &card(
                    &panel,
                    "[card.ru]\ntitle = \"а\"\n[card.en]\ntitle = \"a\"\ntext = \"b\"",
                ),
            false,
            "[ru] has no text".into(),
        ),
        (
            "[item.ru] under a card",
            HEAD.to_owned() + &card(&panel, "[item.ru]\ntitle = \"а\"\ntext = \"б\""),
            false,
            "under a card".into(),
        ),
        (
            "panel text over the news cap",
            HEAD.to_owned() + &card(&panel, &long(10)),
            false,
            "the cap of card is".into(),
        ),
        (
            "full text over the card cap",
            HEAD.to_owned() + &card(&full, &long(2)),
            false,
            "Pismo ot avtora (card)".into(),
        ),
        (
            "gate of [[item]]: a field of an entry",
            HEAD.replace("link = \"https", "layout = \"panel\"\nlink = \"https")
                + &card(&panel, LANGS),
            false,
            "unknown field 'layout'".into(),
        ),
        (
            "gate of [[item]]: a table under an entry",
            HEAD.replace("[item.en]", "[item.image]\nurl = \"x\"\n[item.en]")
                + &card(&panel, LANGS),
            false,
            "is not a language table".into(),
        ),
    ];

    let mut wrong = Vec::new();
    for (index, (name, body, passes, needle)) in cases.iter().enumerate() {
        let feed = place.join(format!("news-{index}.toml"));
        std::fs::write(&feed, body).expect("the test's own feed");
        let (code, printed) = check(&script, &feed);
        let unchanged = std::fs::read_to_string(&feed).expect("the feed") == *body;
        if (code == 0) != *passes || !printed.contains(needle.as_str()) || !unchanged {
            let reason = printed
                .lines()
                .find(|line| line.contains("REFUSED"))
                .unwrap_or("(passed)")
                .trim()
                .to_owned();
            wrong.push(format!(
                "{name}: exit {code}, wanted {} with «{needle}», file unchanged {unchanged}; {reason}",
                if *passes { "PASS" } else { "a refusal" }
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&place);

    assert!(
        wrong.is_empty(),
        "{} of {} cases:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
    let _: Date = today;
}
