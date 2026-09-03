//! The author's feed — **FR-102**, task Т-32-6: the signature, the parse and the choice of
//! language.
//!
//! ⚠ **The vector below was signed with a throw-away key, not the author's** — the mandate of
//! Э32 says so in as many words («тестовые ключи — отдельные от рабочих»), and the reason is
//! that a check which could only be run against the author's own key could only be run by the
//! author. The key and the signature were made by
//! `scratchpad-Э32\тестовый-ключ.ps1`, whose key exists for the length of that script and is
//! written nowhere.
//!
//! What the vector exercises, deliberately, is every rule at once: an `update` and three
//! `news`; one entry with **no** `ru` and no `en`, which FR-102 says is skipped without the
//! file being refused; identifiers that ascend; a text of two lines.

use lang_switcher::letters::feed::{self, Refusal};

/// The public half of the throw-away key the vector below was signed with.
const TEST_KEY: [u8; 64] = [
    0xF0, 0x8E, 0x7F, 0xF3, 0x56, 0x68, 0x89, 0x1D, 0x45, 0x7C, 0xAD, 0x47, 0x3F, 0x23, 0x9F, 0x35,
    0xFE, 0x49, 0xD8, 0x1A, 0x3A, 0x6B, 0xE5, 0x74, 0x82, 0x94, 0xB4, 0xF0, 0x97, 0x9A, 0x17, 0x2C,
    0xCF, 0xAB, 0xB9, 0xA1, 0x19, 0x29, 0xDC, 0xAB, 0x9B, 0x30, 0x93, 0x13, 0x2E, 0xCA, 0xED, 0x82,
    0xAE, 0x0D, 0xC6, 0xDA, 0x67, 0x57, 0x84, 0x63, 0x3B, 0x04, 0x05, 0x19, 0x68, 0xDC, 0x61, 0x5F,
];

/// The signature the script produced over [`BODY`], base64 of the raw `r ‖ s`.
const TEST_SIGNATURE: &str =
    "d4If0+46FuzdVv6JvWxthzmq/tdnIgRrYs6e9JL+SarbDYoH7n9flr5JMK6NedbH179nFgUR5mqtgNjPRvW4Gg==";

/// The signed body, byte for byte as the script signed it — LF line endings and a trailing
/// newline. ⚠ A single byte of difference here and every signature test goes red, which is the
/// property being relied on.
const BODY: &str = "schema = 1\n\
\n\
[[item]]\n\
id = 10\n\
type = \"update\"\n\
date = 2026-10-14\n\
version = \"0.41.0\"\n\
link = \"https://example.invalid/download\"\n\
[item.ru]\n\
title = \"Вышла версия 0.41.0\"\n\
text = \"Три изменения и путь обновления.\"\n\
[item.en]\n\
title = \"Version 0.41.0 is out\"\n\
text = \"Three changes and the way to update.\"\n\
\n\
[[item]]\n\
id = 11\n\
type = \"news\"\n\
date = 2026-11-01\n\
link = \"https://example.invalid/one\"\n\
[item.ru]\n\
title = \"Первая новость\"\n\
text = \"Первая строка.\\nВторая строка.\"\n\
[item.en]\n\
title = \"The first news\"\n\
text = \"The first line.\\nThe second line.\"\n\
\n\
[[item]]\n\
id = 12\n\
type = \"news\"\n\
date = 2026-11-02\n\
link = \"\"\n\
[item.de]\n\
title = \"Nur Deutsch\"\n\
text = \"Diese Nachricht hat kein ru und kein en.\"\n\
\n\
[[item]]\n\
id = 13\n\
type = \"news\"\n\
date = 2026-11-03\n\
link = \"https://example.invalid/three\"\n\
[item.ru]\n\
title = \"Третья новость\"\n\
text = \"Третья.\"\n\
[item.en]\n\
title = \"The third news\"\n\
text = \"The third.\"\n\
[item.de]\n\
title = \"Die dritte Nachricht\"\n\
text = \"Die dritte.\"\n";

/// The whole document as it travels: the signature line and the body under it.
fn signed() -> String {
    format!("signature = \"{TEST_SIGNATURE}\"\n{BODY}")
}

// =========================================================================================
// The signature — the system's own cryptography
// =========================================================================================

/// **The vector: the body, the signature and the key that made it — green.**
#[test]
fn the_signature_of_the_vector_verifies_against_its_key() {
    assert_eq!(
        BODY.len(),
        1028,
        "the body must be the exact bytes the script signed"
    );

    let signature = decode(TEST_SIGNATURE);

    assert_eq!(signature.len(), 64, "P-256 signs with sixty-four bytes");
    assert!(
        feed::verify_with(BODY.as_bytes(), &signature, &[TEST_KEY]),
        "the signature of the vector must verify against the key that made it"
    );
}

/// **Every way of being wrong is a refusal**, and each is checked on its own so that a test
/// which passed for the wrong reason would show.
#[test]
fn a_signature_that_is_not_the_authors_is_refused() {
    let signature = decode(TEST_SIGNATURE);

    // One byte of the body changed — the last full stop.
    let mut damaged = BODY.to_owned();
    damaged.pop();
    damaged.push('!');
    assert!(
        !feed::verify_with(damaged.as_bytes(), &signature, &[TEST_KEY]),
        "a body of one changed byte must not verify"
    );

    // One byte of the signature changed.
    let mut bent = signature.clone();
    bent[0] ^= 0x01;
    assert!(
        !feed::verify_with(BODY.as_bytes(), &bent, &[TEST_KEY]),
        "a signature of one changed byte must not verify"
    );

    // A truncated signature.
    assert!(
        !feed::verify_with(BODY.as_bytes(), &signature[..32], &[TEST_KEY]),
        "half a signature is not a signature"
    );

    // Somebody else's key — the author's own working key, which did not sign this.
    assert!(
        !feed::verify_with(BODY.as_bytes(), &signature, &feed::FEED_KEYS),
        "the vector was signed with a throw-away key and must not verify against the product's"
    );

    // And no keys at all.
    assert!(!feed::verify_with(BODY.as_bytes(), &signature, &[]));
}

/// The two-key rule of FR-102: a signature is accepted from **either** of them, whichever way
/// round they are listed.
#[test]
fn a_signature_is_accepted_from_either_of_the_two_keys() {
    let signature = decode(TEST_SIGNATURE);
    let other = feed::FEED_KEYS[0];

    assert!(feed::verify_with(
        BODY.as_bytes(),
        &signature,
        &[TEST_KEY, other]
    ));
    assert!(feed::verify_with(
        BODY.as_bytes(),
        &signature,
        &[other, TEST_KEY]
    ));
}

/// The product's own keys are two, distinct, and neither is all zeros — the shape a key made by
/// `tools\make-news-key.ps1` has, and the shape a forgotten placeholder would not.
#[test]
fn the_program_carries_two_distinct_keys() {
    assert_eq!(feed::FEED_KEYS.len(), 2, "a working key and a reserve");
    assert_ne!(
        feed::FEED_KEYS[0],
        feed::FEED_KEYS[1],
        "the reserve key must not be the working one"
    );

    for key in feed::FEED_KEYS {
        assert!(
            key.iter().any(|byte| *byte != 0),
            "a key of zeros is a key nobody made"
        );
    }
}

// =========================================================================================
// The document
// =========================================================================================

/// **The whole read**: the signature line is split off, the signature checks out, the document
/// parses, and what comes back is one update and the news items kept.
#[test]
fn a_signed_document_is_read_into_one_update_and_three_news() {
    let feed = feed::read_document(&signed(), "ru", |body, signature| {
        feed::verify_with(body, signature, &[TEST_KEY])
    })
    .expect("the vector must read");

    let update = feed.update.as_ref().expect("the vector carries an update");

    assert_eq!(update.id, 10);
    assert_eq!(update.version.as_deref(), Some("0.41.0"));
    assert_eq!(update.title, "Вышла версия 0.41.0");
    assert_eq!(update.link, "https://example.invalid/download");

    // Three news entries in the document, one of which has neither ru nor en: FR-102 skips it
    // and does **not** refuse the file over it.
    assert_eq!(
        feed.news.len(),
        2,
        "the entry with no ru and no en is skipped"
    );
    assert_eq!(
        feed.news[0].id, 11,
        "and what is left is in ascending order"
    );
    assert_eq!(feed.news[1].id, 13);
    assert_eq!(feed.news[0].title, "Первая новость");
    assert!(
        feed.news[0].text.contains('\n'),
        "a text of two lines stays two"
    );
}

/// **The language is chosen on this machine** — FR-102, вопрос 101 п. 5: the interface language,
/// then English, then Russian, and the request never says which.
#[test]
fn the_language_is_chosen_here_and_falls_back_the_way_fr_102_says() {
    let read = |language: &str| {
        feed::read_document(&signed(), language, |body, signature| {
            feed::verify_with(body, signature, &[TEST_KEY])
        })
        .expect("the vector must read")
    };

    let german = read("de");

    // ⚠ **A German reader sees one news item more.** Entry 12 has neither `ru` nor `en` and is
    // skipped for everybody else; for a German reader it is readable, so it is kept — which is
    // FR-102's rule read forwards rather than backwards, and the reason the entries below are
    // named by identifier and not by position.
    assert_eq!(
        german.news.len(),
        3,
        "the German-only entry becomes readable"
    );
    assert_eq!(
        title_of(&german.news, 13),
        "Die dritte Nachricht",
        "the interface language wins where the entry has it"
    );
    assert_eq!(
        title_of(&german.news, 11),
        "The first news",
        "and English stands in where it has not"
    );
    assert_eq!(title_of(&german.news, 12), "Nur Deutsch");

    // A language nobody wrote falls all the way to English, and then to Russian.
    let japanese = read("ja");
    assert_eq!(
        japanese.news.len(),
        2,
        "the German-only entry is skipped again"
    );
    assert_eq!(title_of(&japanese.news, 11), "The first news");
    assert_eq!(title_of(&read("ru").news, 11), "Первая новость");
}

/// Every refusal of the reading half, each on its own.
#[test]
fn every_way_of_being_wrong_is_its_own_refusal() {
    let good = |body: &[u8], signature: &[u8]| feed::verify_with(body, signature, &[TEST_KEY]);

    // No signature line at all.
    assert_eq!(
        feed::read_document(BODY, "ru", good),
        Err(Refusal::NoSignature)
    );

    // A signature line that is not base64 of sixty-four bytes.
    assert_eq!(
        feed::read_document("signature = \"AAAA\"\nschema = 1\n", "ru", good),
        Err(Refusal::BadSignature)
    );
    assert_eq!(
        feed::read_document("signature = \"not base64!!\"\nschema = 1\n", "ru", good),
        Err(Refusal::BadSignature)
    );

    // A signature that does not check out.
    let mut bent = signed();
    bent = bent.replace("schema = 1", "schema = 1 ");
    assert_eq!(
        feed::read_document(&bent, "ru", good),
        Err(Refusal::Unsigned)
    );

    // A document of a schema this build does not know — refused **as a schema**, so that the
    // journal can tell it from damage.
    assert!(matches!(
        feed::parse_body("schema = 2\n", "ru"),
        Err(Refusal::Schema(2))
    ));

    // Not TOML at all.
    assert_eq!(feed::parse_body("[[[", "ru"), Err(Refusal::Malformed));

    // An answer over the ceiling.
    let huge = "x".repeat(feed::RESPONSE_CAP + 1);
    assert_eq!(feed::read_document(&huge, "ru", good), Err(Refusal::TooBig));
}

/// **Four news entries: the fourth pushes the oldest out** — FR-101's «не более трёх», enforced
/// on the reading side as well as on the author's.
#[test]
fn only_the_three_newest_news_entries_are_kept() {
    let mut body = String::from("schema = 1\n");

    for id in 1..=5 {
        body.push_str(&format!(
            "\n[[item]]\nid = {id}\ntype = \"news\"\nlink = \"\"\n[item.ru]\ntitle = \"н{id}\"\ntext = \"т\"\n[item.en]\ntitle = \"n{id}\"\ntext = \"t\"\n"
        ));
    }

    let feed = feed::parse_body(&body, "ru").expect("five news entries must parse");

    assert_eq!(feed.news.len(), 3, "three are kept");
    assert_eq!(feed.news[0].id, 3, "and they are the three greatest");
    assert_eq!(feed.news[1].id, 4);
    assert_eq!(feed.news[2].id, 5);
    assert!(feed.update.is_none());
}

/// An `update` with no version says nothing that can be compared, so it is not an update at
/// all; and of two updates the greater identifier wins.
#[test]
fn an_update_needs_a_version_and_the_newest_one_wins() {
    let without = "schema = 1\n\n[[item]]\nid = 1\ntype = \"update\"\nlink = \"\"\n\
                   [item.ru]\ntitle = \"а\"\ntext = \"б\"\n[item.en]\ntitle = \"a\"\ntext = \"b\"\n";

    assert!(
        feed::parse_body(without, "ru")
            .expect("it must parse")
            .update
            .is_none(),
        "an update with no version is not an update"
    );

    let two = "schema = 1\n\n[[item]]\nid = 1\ntype = \"update\"\nversion = \"0.1.0\"\nlink = \"\"\n\
               [item.ru]\ntitle = \"а\"\ntext = \"б\"\n[item.en]\ntitle = \"a\"\ntext = \"b\"\n\
               \n[[item]]\nid = 2\ntype = \"update\"\nversion = \"0.2.0\"\nlink = \"\"\n\
               [item.ru]\ntitle = \"в\"\ntext = \"г\"\n[item.en]\ntitle = \"c\"\ntext = \"d\"\n";

    assert_eq!(
        feed::parse_body(two, "ru")
            .expect("it must parse")
            .update
            .and_then(|item| item.version),
        Some("0.2.0".to_owned()),
        "of two updates the greater identifier wins"
    );
}

/// An entry of a type this build does not know is skipped, and the file is not refused over it
/// — the same rule as the one about a missing language, and for the same reason: a later
/// version of the feed must not silence an older program.
#[test]
fn an_entry_of_an_unknown_type_is_skipped_and_not_refused() {
    let body = "schema = 1\n\n[[item]]\nid = 1\ntype = \"poll\"\nlink = \"\"\n\
                [item.ru]\ntitle = \"а\"\ntext = \"б\"\n[item.en]\ntitle = \"a\"\ntext = \"b\"\n\
                \n[[item]]\nid = 2\ntype = \"news\"\nlink = \"\"\n\
                [item.ru]\ntitle = \"в\"\ntext = \"г\"\n[item.en]\ntitle = \"c\"\ntext = \"d\"\n";

    let feed = feed::parse_body(body, "ru").expect("it must parse");

    assert_eq!(feed.news.len(), 1, "the unknown type is skipped");
    assert_eq!(feed.news[0].id, 2, "and the news beside it is kept");
}

/// The names a refusal goes into the journal under are a closed set, and **not one of them
/// carries a value out of the document** — SEC-07.
#[test]
fn a_refusal_takes_no_word_of_the_document_into_the_journal() {
    for refusal in [
        Refusal::NoSignature,
        Refusal::BadSignature,
        Refusal::Malformed,
        Refusal::Schema(99),
        Refusal::Unsigned,
        Refusal::TooBig,
    ] {
        let name = refusal.journal_name();

        assert!(!name.is_empty());
        assert!(
            name.is_ascii() && !name.contains("99"),
            "a journal name is a fixed sentence and carries nothing of the file: «{name}»"
        );
    }
}

// =========================================================================================
// The request — SEC-03, вопрос 101 п. 5
// =========================================================================================

/// **The user agent names the program and nothing about the machine.**
///
/// The one field of the request the author could be tempted to put something in. A version
/// there would tell the host how many people run which build, which is telemetry under another
/// name — so this test fails on a **digit**, and the positive control is that it would: put
/// `env!("CARGO_PKG_VERSION")` into `USER_AGENT` and the first assertion goes red.
#[test]
fn the_user_agent_names_the_program_and_nothing_about_the_machine() {
    let agent = feed::USER_AGENT;

    println!("User-Agent: «{agent}»");

    assert!(
        !agent.chars().any(|letter| letter.is_ascii_digit()),
        "a digit in the user agent is a version, and a version is telemetry: «{agent}»"
    );
    assert!(
        !agent.contains(env!("CARGO_PKG_VERSION")),
        "and the version of this build most of all"
    );
    assert!(agent.is_ascii() && !agent.is_empty());
    assert!(
        !agent.to_lowercase().contains("windows") && !agent.to_lowercase().contains("mozilla"),
        "and nothing about the system either"
    );
}

/// **Only `https://`, and only a host with no credentials and no port.**
///
/// SEC-03 says the one network operation is over HTTPS. A plain-HTTP address in the list would
/// leak which machine reads the author's feed to everybody on the way — the signature would
/// still catch a replaced document, and the leak would already have happened.
#[test]
fn only_an_https_address_with_a_plain_host_is_read() {
    assert_eq!(
        feed::split_https("https://example.com/news.toml"),
        Some(("example.com".to_owned(), "/news.toml".to_owned()))
    );
    assert_eq!(
        feed::split_https("https://example.com/a/b/c.toml"),
        Some(("example.com".to_owned(), "/a/b/c.toml".to_owned()))
    );
    assert_eq!(
        feed::split_https("https://example.com"),
        Some(("example.com".to_owned(), "/".to_owned())),
        "an address with no path is the root"
    );

    for refused in [
        "http://example.com/news.toml",
        "ftp://example.com/news.toml",
        "example.com/news.toml",
        "https:///news.toml",
        "https://user@example.com/news.toml",
        "https://example.com:8443/news.toml",
        "",
    ] {
        assert_eq!(
            feed::split_https(refused),
            None,
            "«{refused}» must not be read"
        );
    }
}

/// The ceilings of FR-102 are the numbers the requirement names, and the reading one is above
/// the signing one on purpose.
#[test]
fn the_ceilings_are_the_ones_fr_102_names() {
    assert_eq!(feed::RESPONSE_CAP, 256 * 1024, "the program reads 256 KB");
    assert_eq!(feed::FEED_SCHEMA, 1);
    assert_eq!(feed::CONNECT_TIMEOUT_MS, 10_000);
    assert_eq!(feed::RECEIVE_TIMEOUT_MS, 20_000);

    // The author's own script refuses to sign anything over 192 KB. That the reader's ceiling
    // is **above** it is the point: the script's limit protects the reader from a mistake, and
    // this one protects it from a host that is not the author's.
    //
    // ⚠ The script's number is **read out of the script**, not typed here again. A copy of it
    // in this file would agree with itself for ever, including on the day someone raises
    // `$BODY_CAP` and the author signs a file the program then refuses to read.
    let script = std::fs::read_to_string("tools/sign-news.ps1").expect("the signing script");
    let signing_cap = script
        .lines()
        .find_map(|line| line.strip_prefix("$BODY_CAP = "))
        .and_then(|value| value.split_once(" * "))
        .map(|(kilobytes, block)| {
            kilobytes.trim().parse::<usize>().expect("a number of KB")
                * block.trim().parse::<usize>().expect("the block size")
        })
        .expect("`$BODY_CAP = <n> * <m>` in tools\\sign-news.ps1");

    assert_eq!(signing_cap, 192 * 1024, "the script signs up to 192 KB");
    assert!(
        feed::RESPONSE_CAP > signing_cap,
        "the reading ceiling must be above the signing one"
    );
}

/// The file that will be published, read with the keys that are **shipped** — the one check
/// the throw-away key above cannot make.
///
/// Every other test in this file proves the reader against a key made for the test, and would
/// stay green if `make-news-key.ps1` had printed the wrong halves into `letters.rs`, or if
/// `sign-news.ps1` signed with the reserve key while the build carried only the working one.
/// This test cannot: it takes the file the author is about to publish and puts it through
/// [`feed::read_document`] with the real [`feed::FEED_KEYS`].
///
/// ⚠ `#[ignore]` because it reads a path **outside the repository** — the site is built in
/// `<dev>\artifacts\news-site\`, and on any other machine there is nothing there. Run it by
/// hand:
///
/// ```text
/// cargo test --test feed -- --ignored
/// ```
#[test]
#[ignore = "reads the site under <dev>\\artifacts, which exists only on the author's machine"]
fn the_file_that_will_be_published_verifies_with_the_shipped_keys() {
    const SITE: &str = r"<dev>\artifacts\news-site\news.toml";

    let text = match std::fs::read_to_string(SITE) {
        Ok(text) => text,
        Err(error) => panic!("{SITE}: {error} — sign the file before running this"),
    };

    let feed = feed::read_document(&text, "ru", feed::verify_signature)
        .expect("the published file must verify with a key that is in the build");

    // Not just «it verified»: the entries the program will actually show.
    assert!(
        feed.update.is_some(),
        "the published file carries one update entry"
    );
    assert!(
        !feed.news.is_empty(),
        "the published file carries at least one news entry"
    );

    // And the negative half — one byte of the body changed must break it. Without this the
    // test above could pass on a reader that verified nothing at all.
    let broken = text.replacen("schema = 1", "schema = 1 ", 1);
    assert!(
        feed::read_document(&broken, "ru", feed::verify_signature).is_err(),
        "a changed body must not verify"
    );
}

/// **The instrument that can say «no request was made»** — ТЗ Б-6, and it comes with its own
/// positive control.
///
/// `netstat` is no good here: the connection lives milliseconds and would be missed between
/// two samples. What can be relied on is the journal — `fetch` writes «feed request» before
/// it opens the session, so a read that never reached the wire leaves no such line.
///
/// The two halves:
///
/// * **negative** — `read_now` over the addresses actually in the build. Every one of them is
///   a placeholder today (П5), the reader skips placeholders, and the journal stays clean;
/// * **positive** — one `fetch` at an address that is real in form and dead in fact
///   (`localhost`, where nothing listens on 443). If the line failed to appear here it would be
///   worthless above.
///
/// ⚠ The address of the positive half may not carry a port: `split_https` refuses a host with
/// a `:` in it, so `127.0.0.1:1` never reaches `fetch` at all — measured, and it is the reason
/// the first version of this control read «0 lines» and looked like a broken instrument.
///
/// ⚠ `#[ignore]`: the positive half opens a socket. It is a **loopback** socket to a port
/// nothing listens on — no traffic leaves the machine and SEC-03 is about the shipped program,
/// not about its tests — but a battery that opens sockets by default is a battery that behaves
/// differently on someone else's machine. Run it by hand with `--ignored`.
#[test]
#[ignore = "opens a loopback socket for its positive control"]
fn the_journal_says_whether_a_request_was_made_at_all() {
    use lang_switcher::diag;

    fn requests_in_journal() -> usize {
        diag::snapshot()
            .iter()
            .filter(|event| event.operation.name() == "feed request")
            .count()
    }

    let before = requests_in_journal();

    // The negative half: what the shipped build would do on its own schedule today.
    let answer = feed::read_now("ru");

    assert!(
        answer.is_none(),
        "a placeholder address brings back nothing"
    );
    assert_eq!(
        requests_in_journal(),
        before,
        "no line in the journal — the program did not reach the network at all"
    );

    // The positive half: a real address that refuses at once.
    let reached = feed::fetch("https://localhost/news.toml");

    assert!(reached.is_none(), "nothing listens there");
    assert_eq!(
        requests_in_journal(),
        before + 1,
        "the wrapper writes exactly one line per request it makes — without this the check \
         above proves nothing"
    );
}

/// The heading of the entry with this identifier — the entries are named by number and never
/// by position, because which of them survive depends on the language.
fn title_of(news: &[lang_switcher::letters::FeedItem], id: u64) -> &str {
    news.iter()
        .find(|item| item.id == id)
        .map(|item| item.title.as_str())
        .unwrap_or_else(|| panic!("no entry {id} among the ones kept"))
}

/// Decodes the base64 of the vector — the test's own decoder, so that a defect in the
/// program's would not hide behind it.
fn decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let bytes: Vec<u8> = text.bytes().filter(|byte| *byte != b'=').collect();
    let mut out = Vec::new();
    let mut accumulator = 0u32;
    let mut held = 0u32;

    for byte in bytes {
        let value = ALPHABET
            .iter()
            .position(|letter| *letter == byte)
            .expect("the vector is base64") as u32;

        accumulator = (accumulator << 6) | value;
        held += 6;

        if held >= 8 {
            held -= 8;
            out.push(((accumulator >> held) & 0xFF) as u8);
        }
    }

    out
}
