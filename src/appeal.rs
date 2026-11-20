//! **What the wizard sends to the site, and how** — решения 199.1, 199.7, 199.9 и 199.12.
//!
//! # What goes, and what never does
//!
//! Exactly the text the last step of the wizard showed the person (199.1: for a trouble what the
//! boxes of FR-104 let in, for an idea the person's words and the version), the language of the
//! interface, the kind of the appeal, the version of the program, the contact **if** the person
//! wrote one, and a nonce that makes a repeated press one appeal and not two. Nothing else: no
//! identifier of the machine, no name of the person, nothing the person did not see on the screen.
//! The address the site sees is the site's business (199.4: kept in memory against the floods
//! and never written beside the appeal).
//!
//! # When
//!
//! Only when the person presses «Отправить» (199.5, 199.12: «уходит на сайт, только когда вы сами
//! нажмёте «Отправить»»), and again only when they press «Повторить» — on a thread of its own
//! that lives one request ([`super::start_appeal`]). Never on its own, never in the background,
//! never a second time behind the person's back.
//!
//! # The test folder of the site
//!
//! `"test": true` sends an appeal into the folder «Тест» of the site, with no notice to the author
//! and cleared after seven days (199.7, 199.9). **Only a build with the `testing` feature writes
//! it** — the stand of TZ E107; the code that writes the field does not exist in a Release image,
//! and `the_shipped_body_carries_no_test_field` holds that.

use crate::letters::report;
use crate::{feed, links};

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
