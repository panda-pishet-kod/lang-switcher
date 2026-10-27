//! The appeal of the wizard — **what goes to the site and how**: stage E107, task T-107-2,
//! решения 199.1, 199.7, 199.9 и 199.12.
//!
//! The body is a pure function ([`appeal::body`]), so what goes is read here byte for byte. The
//! request itself is checked against a server of this file on the loopback of the machine
//! (`#[ignore]`, as every test of this crate that opens a socket — run them by hand with
//! `--ignored`): the verb, the headers, the length, the body, «only 201 is a yes» and «a
//! redirection is not followed» are what the server saw and answered, not what the code says.

use lang_switcher::letters::report::Trouble;
use lang_switcher::letters::{appeal, feed, links};

/// A nonce written out here — 32 lowercase hexadecimal digits, the shape 199.8 (2) names.
const NONCE: &str = "0123456789abcdef0123456789abcdef";

/// An appeal with every field but the contact.
fn sample(contact: &'static str) -> appeal::Appeal<'static> {
    appeal::Appeal {
        kind: appeal::kind_of(Trouble::Idea),
        lang: "ru",
        version: "1.2.0",
        text: "Что предлагаете?\r\nПереключать раскладку двойным Ctrl",
        contact,
        nonce: NONCE,
    }
}

/// What a build of the stand adds at the end of every body, and a shipped build never does.
#[cfg(feature = "testing")]
const STAND_TAIL: &str = ",\"test\":true";
#[cfg(not(feature = "testing"))]
const STAND_TAIL: &str = "";

// =========================================================================================
// The body — решения 199.7 и 199.9
// =========================================================================================

/// **The body is the format of 199.7 and 199.9, field for field and in that order** — `schema`,
/// `type`, `lang`, `version`, `text`, `contact` only when there is one, `nonce`, and `test` only
/// in a build of the stand. Written out here byte for byte, apart from the code.
#[test]
fn the_body_is_the_format_of_decisions_199_7_and_199_9() {
    let without = appeal::body(&sample(""));
    let with = appeal::body(&sample("ivan@example.com"));

    assert_eq!(
        without,
        format!(
            "{{\"schema\":1,\"type\":\"wish\",\"lang\":\"ru\",\"version\":\"1.2.0\",\"text\":\
             \"Что предлагаете?\\r\\nПереключать раскладку двойным Ctrl\",\"nonce\":\"{NONCE}\"\
             {STAND_TAIL}}}"
        )
    );
    assert_eq!(
        with,
        format!(
            "{{\"schema\":1,\"type\":\"wish\",\"lang\":\"ru\",\"version\":\"1.2.0\",\"text\":\
             \"Что предлагаете?\\r\\nПереключать раскладку двойным Ctrl\",\"contact\":\
             \"ivan@example.com\",\"nonce\":\"{NONCE}\"{STAND_TAIL}}}"
        )
    );
}

/// **A contact goes only when there is one — absent, never `null` and never `""`** (199.9: the
/// site answers `null` with 400). Spaces alone are no contact.
///
/// ⚠ Control: the body of a typed contact does carry the field — the check of absence can fail.
#[test]
fn a_contact_goes_only_when_there_is_one_and_never_as_null() {
    for nothing in ["", "   ", "\t \t"] {
        let body = appeal::body(&sample(nothing));

        assert!(
            !body.contains("\"contact\"") && !body.contains("null"),
            "«{nothing}» is no contact: {body}"
        );
    }

    assert!(
        appeal::body(&sample("  @panda  ")).contains(",\"contact\":\"@panda\","),
        "the control: a typed contact goes, without the spaces around it"
    );
}

/// **The contact is cut to the 200 characters the site takes** — characters and not bytes
/// (199.7), and with no space left dangling at the cut.
#[test]
fn the_contact_is_cut_to_the_two_hundred_characters_the_site_takes() {
    assert_eq!(appeal::CONTACT_MAX, 200);

    let long = "я".repeat(250);
    let cut = appeal::contact_of(&long);

    assert_eq!(cut.chars().count(), 200, "characters, not bytes");

    let spaced = format!("{} {}", "a".repeat(199), "b".repeat(10));

    assert_eq!(
        appeal::contact_of(&spaced),
        "a".repeat(199),
        "the space at the cut is not sent"
    );
    assert_eq!(appeal::contact_of("x"), "x");
}

/// **Every character JSON needs escaped is escaped, and nothing else is touched** — the quotation
/// mark, the reverse solidus, the control characters; Cyrillic, Hebrew and an emoji go as UTF-8.
#[test]
fn every_character_json_needs_escaped_is_escaped() {
    let mut out = String::new();

    appeal::string(&mut out, "a\"b\\c\nd\re\tf\u{1}g\u{1f}h — שלום 🖤");

    assert_eq!(out, "\"a\\\"b\\\\c\\nd\\re\\tf\\u0001g\\u001fh — שלום 🖤\"");
}

/// **A trouble is `error` and an idea is `wish`** — the two kinds the site knows (199.7).
#[test]
fn the_kind_is_error_for_both_troubles_and_wish_for_an_idea() {
    assert_eq!(appeal::kind_of(Trouble::WrongResult), "error");
    assert_eq!(appeal::kind_of(Trouble::NothingHappened), "error");
    assert_eq!(appeal::kind_of(Trouble::Idea), "wish");
}

/// **A nonce is 32 lowercase hexadecimal digits from the system's generator** — 199.7 (б), 199.8
/// (2) — and two drawn one after the other are not the same.
#[test]
fn a_nonce_is_thirty_two_lowercase_hex_digits_and_two_are_not_the_same() {
    let first = appeal::nonce().expect("the system draws a nonce");
    let second = appeal::nonce().expect("and a second one");

    for nonce in [&first, &second] {
        assert_eq!(nonce.len(), 32, "{nonce}");
        assert!(
            nonce
                .chars()
                .all(|digit| digit.is_ascii_digit() || ('a'..='f').contains(&digit)),
            "{nonce}"
        );
    }

    assert_ne!(first, second, "two appeals, two nonces");
    assert_eq!(appeal::hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
}

/// **«Повторить» sends the nonce of the first press** — 199.7 (б): the site answers a repeat with
/// the same 201 and no second record, so an appeal whose answer was lost on the way is not filed
/// twice.
///
/// ⚠ Control: the generator is asked on the first press — the rule is not «never draw».
#[test]
fn a_retry_sends_the_nonce_of_the_first_press() {
    let mut kept = None;
    let mut drawn = 0;

    let first = appeal::keep_or_draw(&mut kept, || {
        drawn += 1;
        Some(NONCE.to_owned())
    })
    .map(str::to_owned);

    let again = appeal::keep_or_draw(&mut kept, || {
        drawn += 1;
        Some("ffffffffffffffffffffffffffffffff".to_owned())
    })
    .map(str::to_owned);

    assert_eq!(
        first.as_deref(),
        Some(NONCE),
        "the control: the first press draws"
    );
    assert_eq!(again, first, "a retry sends the same nonce");
    assert_eq!(drawn, 1, "and the generator is not asked a second time");

    let mut refused = None;

    assert_eq!(
        appeal::keep_or_draw(&mut refused, || None),
        None,
        "a refused generator is no nonce, and then nothing is sent"
    );
}

// =========================================================================================
// The test folder of the site — only the stand writes it
// =========================================================================================

/// **The shipped body carries no `test` field** — решения 199.7, 199.9: «Тест» is the stand's
/// folder, and an appeal of a person must land in «Новое». Compiled only without the feature,
/// which is the shipped build.
#[cfg(not(feature = "testing"))]
#[test]
fn the_shipped_body_carries_no_test_field() {
    for contact in ["", "ivan@example.com"] {
        let body = appeal::body(&sample(contact));

        assert!(!body.contains("\"test\""), "{body}");
        assert!(
            body.ends_with(&format!("\"nonce\":\"{NONCE}\"}}")),
            "{body}"
        );
    }
}

/// **The body of the stand files the appeal under «Тест»** — the other half of the rule, so that
/// the stand of T-107-7 cannot send into «Новое» either.
#[cfg(feature = "testing")]
#[test]
fn the_body_of_the_stand_files_the_appeal_under_test() {
    assert!(appeal::body(&sample("")).ends_with(",\"test\":true}"));
}

/// **The line that writes `test` is compiled only under the feature of the stand** — read out of
/// the source, because the shipped build cannot show a line it does not have: the push of
/// `,"test":true` stands directly under `#[cfg(feature = "testing")]`, and it stands once.
///
/// ⚠ Control: the same reading over a copy with the gate taken off is caught.
#[test]
fn the_test_field_is_written_only_under_the_feature_of_the_stand() {
    const PUSH: &str = "out.push_str(\",\\\"test\\\":true\");";
    const GATE: &str = "#[cfg(feature = \"testing\")]";

    fn gated(source: &str) -> Result<(), String> {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        let at: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == PUSH).collect();

        match at.as_slice() {
            [one] if *one > 0 && lines[*one - 1] == GATE => Ok(()),
            [one] => Err(format!(
                "the push stands under «{}»",
                lines[one.saturating_sub(1)]
            )),
            other => Err(format!("the push stands {} times", other.len())),
        }
    }

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/letters.rs");
    let source = std::fs::read_to_string(&path).expect("src/letters.rs must be readable");

    assert_eq!(gated(&source), Ok(()));
    assert!(
        gated(&source.replace(GATE, "")).is_err(),
        "the control: a push without its gate must be caught"
    );
}

// =========================================================================================
// Where, and how long
// =========================================================================================

/// **The appeal goes to the address of 199.7 and the letter to the box of 199.3** — written down
/// here apart from the code. Neither is a page: the shell is never handed either of them — the
/// address of the appeal is named by its declaration and by [`appeal::send`] and by no other line
/// of the module's code.
///
/// ⚠ `links::is_allowed` would say yes to the address of the appeal, and that is not a hole: the
/// gate of С52 judges a **host**, and the site's host is the host of the support page. What keeps
/// the shell away from the address is that no code hands it there — which is what is read here.
#[test]
fn the_appeal_goes_to_the_site_and_the_letter_to_the_box_of_the_appeals() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/letters.rs");
    let source = std::fs::read_to_string(&path).expect("src/letters.rs must be readable");
    let naming: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && line.contains("APPEAL_URL"))
        .collect();

    assert_eq!(
        naming,
        [
            "pub const APPEAL_URL: &str = \"https://panda-pishet-kod.dev/langswitcher/feedback\";",
            "let Some(target) = feed::target_of(links::APPEAL_URL) else {",
        ],
        "the address of the appeal is named by its declaration and by `appeal::send` alone"
    );

    assert_eq!(
        links::APPEAL_URL,
        "https://panda-pishet-kod.dev/langswitcher/feedback"
    );
    assert_eq!(links::MAIL_ADDRESS, "lang.switcher@panda-pishet-kod.dev");
    assert_eq!(
        feed::split_https(links::APPEAL_URL),
        Some((
            "panda-pishet-kod.dev".to_owned(),
            "/langswitcher/feedback".to_owned()
        )),
        "over HTTPS, port 443, no credentials"
    );
    assert!(!links::is_allowed(links::MAIL_ADDRESS));
    assert_eq!(appeal::CONTENT_TYPE, "application/json; charset=utf-8");
}

/// **An appeal waits 15 seconds at the very most, and the read of the feed waits as it always
/// has** — TZ E107 §0 п. 10. The four of the appeal are each something, and the feed's are the
/// numbers of FR-102.
#[test]
fn an_appeal_waits_fifteen_seconds_at_the_most_and_the_feed_as_it_did() {
    let send = appeal::TIMEOUTS;

    assert!(send.total() <= 15_000, "{send:?}");
    assert!(send.resolve > 0 && send.connect > 0 && send.send > 0 && send.receive > 0);
    assert_eq!(
        feed::READ_TIMEOUTS,
        feed::Timeouts {
            resolve: 10_000,
            connect: 10_000,
            send: 20_000,
            receive: 20_000,
        },
        "the read of the feed is not touched by the appeal"
    );
}

// =========================================================================================
// The request on the wire — a server of this file on the loopback (`--ignored`)
// =========================================================================================

/// One request as the server of [`serve`] saw it.
#[cfg(feature = "testing")]
struct Seen {
    head: String,
    body: Vec<u8>,
}

/// A server on the loopback that answers each of `answers` to one connection, in order, and hands
/// back what it saw. Every connection after the last answer is counted and refused.
#[cfg(feature = "testing")]
fn serve(answers: Vec<String>) -> (u16, std::thread::JoinHandle<(Vec<Seen>, usize)>) {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();

    listener
        .set_nonblocking(true)
        .expect("a listener that can be asked and not waited on");

    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        let mut extra = 0;
        let started = std::time::Instant::now();
        let mut last = std::time::Instant::now();
        let total = answers.len();
        let mut answers = answers.into_iter();

        // Until every answer is given and a second and a half more have passed with no further
        // connection — the time a followed redirection would have had to arrive in — or 20 s.
        while started.elapsed() < std::time::Duration::from_secs(20)
            && !(seen.len() == total && last.elapsed() > std::time::Duration::from_millis(1500))
        {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            };

            last = std::time::Instant::now();

            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

            let Some(answer) = answers.next() else {
                extra += 1;
                continue;
            };

            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];

            let (head, body) = loop {
                let Ok(read) = stream.read(&mut chunk) else {
                    break (String::from_utf8_lossy(&raw).into_owned(), Vec::new());
                };

                if read == 0 {
                    break (String::from_utf8_lossy(&raw).into_owned(), Vec::new());
                }

                raw.extend_from_slice(&chunk[..read]);

                if let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&raw[..end]).into_owned();
                    let length = head
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);

                    while raw.len() < end + 4 + length {
                        match stream.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => raw.extend_from_slice(&chunk[..read]),
                        }
                    }

                    let body = raw[end + 4..].to_vec();
                    break (head, body);
                }
            };

            let _ = stream.write_all(answer.as_bytes());
            seen.push(Seen { head, body });
        }

        (seen, extra)
    });

    (port, handle)
}

/// The answer of the server for one status.
#[cfg(feature = "testing")]
fn status_line(status: u32, extra: &str) -> String {
    format!("HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n{extra}\r\n")
}

/// The value of one header of a request head, by its name in any case.
#[cfg(feature = "testing")]
fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

/// **The request is one `POST` with its length, its type and its agent, and the body is the body**
/// — 199.7 and 199.9: `Content-Length` and never chunked (the site answers that with 411),
/// `application/json; charset=utf-8`, `User-Agent: LangSwitcher` exactly (otherwise «Спам»).
#[cfg(feature = "testing")]
#[test]
#[ignore = "opens a loopback socket"]
fn the_request_is_one_post_with_its_length_its_type_and_its_agent() {
    let (port, server) = serve(vec![status_line(201, "")]);
    let target = feed::target_of(&format!("http://127.0.0.1:{port}/langswitcher/feedback"))
        .expect("the loopback door of the stand");
    let body = appeal::body(&sample("ivan@example.com"));

    let taken = feed::post(
        &target,
        appeal::CONTENT_TYPE,
        body.as_bytes(),
        appeal::TIMEOUTS,
    );
    let (seen, extra) = server.join().expect("the server");

    assert!(taken, "the server answered 201");
    assert_eq!(seen.len(), 1, "one request");
    assert_eq!(extra, 0, "and not a second one");

    let request = &seen[0];
    let first = request.head.lines().next().unwrap_or_default();

    println!("{}", request.head);

    assert_eq!(first, "POST /langswitcher/feedback HTTP/1.1");
    assert_eq!(
        header(&request.head, "Content-Length"),
        Some(body.len().to_string().as_str()),
        "the length of the body, in bytes"
    );
    assert_eq!(
        header(&request.head, "Transfer-Encoding"),
        None,
        "never chunked"
    );
    assert_eq!(
        header(&request.head, "Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(header(&request.head, "User-Agent"), Some("LangSwitcher"));
    assert_eq!(request.body, body.as_bytes(), "the body, byte for byte");
}

/// **Only 201 is a yes** — 199.7, 199.9: 200, 400, 411, 413, 415, 429, 502 and 503 are every one
/// «Не удалось отправить».
///
/// ⚠ Control: 201 itself is a yes — the check of «no» can fail.
#[cfg(feature = "testing")]
#[test]
#[ignore = "opens a loopback socket"]
fn only_201_is_a_yes() {
    let statuses = [201, 200, 202, 400, 411, 413, 415, 429, 502, 503];
    let (port, server) = serve(
        statuses
            .iter()
            .map(|&status| status_line(status, ""))
            .collect(),
    );
    let target = feed::target_of(&format!("http://127.0.0.1:{port}/langswitcher/feedback"))
        .expect("the loopback door of the stand");
    let body = appeal::body(&sample(""));

    let answers: Vec<(u32, bool)> = statuses
        .iter()
        .map(|&status| {
            (
                status,
                feed::post(
                    &target,
                    appeal::CONTENT_TYPE,
                    body.as_bytes(),
                    appeal::TIMEOUTS,
                ),
            )
        })
        .collect();

    let _ = server.join();

    assert_eq!(
        answers,
        statuses
            .iter()
            .map(|&status| (status, status == 201))
            .collect::<Vec<_>>()
    );
}

/// **A redirection is not followed** — TZ E107 §0 п. 9: a `307` to another path of the same host
/// is a «no», and the host sees no second request.
#[cfg(feature = "testing")]
#[test]
#[ignore = "opens a loopback socket"]
fn a_redirection_is_not_followed() {
    let (port, server) = serve(vec![status_line(
        307,
        "Location: /langswitcher/elsewhere\r\n",
    )]);
    let target = feed::target_of(&format!("http://127.0.0.1:{port}/langswitcher/feedback"))
        .expect("the loopback door of the stand");
    let body = appeal::body(&sample(""));

    let taken = feed::post(
        &target,
        appeal::CONTENT_TYPE,
        body.as_bytes(),
        appeal::TIMEOUTS,
    );
    let (seen, extra) = server.join().expect("the server");

    assert!(!taken, "a redirection is not the site's yes");
    assert_eq!(seen.len(), 1);
    assert_eq!(extra, 0, "and it was not followed");
}
