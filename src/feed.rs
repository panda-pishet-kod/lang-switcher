//! **The one network operation this program has**, and everything that decides what to do with
//! what it brings back.
//!
//! # What is in here and what is deliberately not
//!
//! In: one `GET` over HTTPS from a fixed list of addresses, no more often than once a day
//! (решение 189.2); the ECDSA P-256 signature of the answer, checked by the system's own cryptography; the
//! parsing of the document; the choice of language. **Nothing else in this program opens a
//! socket** — that is what makes «which code can reach the network» a question with a one-line
//! answer, and what ворота 3 of `tools\verify-perimeter.ps1` checks from the outside by reading
//! the strings of the shipped binary.
//!
//! Not in, and never: sending anything, downloading anything, running anything, an identifier,
//! a version, a language, a cookie, a conditional request. The request carries **no parameters
//! of any kind** (вопрос 101 п. 5): the file holds every language and the choice is made on
//! this machine.
//!
//! # ⚠ And the one thing the program sends — the appeal, since 1.2.0
//!
//! Задача T-107-2, решения 199.1 и 199.7: an appeal of the wizard goes to the site by one `POST`
//! ([`post`]) — **only when the person presses «Отправить»**, and exactly the text they saw. It
//! runs on the same body as the read ([`exchange`]), so «which code can reach the network» is
//! still this module and nothing else; what it sends is built by [`super::appeal::body`], where
//! the rule of what goes and what never does is written down. The read of the feed is untouched by
//! it: the same verb, the same headers (none), the same timeouts, the same redirections.
//!
//! # The trust is in the signature and not in the host
//!
//! The addresses are ordinary web hosting and may be anybody's tomorrow. What the program
//! believes is the author's signature over the body, checked against **two** public keys
//! compiled into it — a working one and a reserve. A file whose signature does not check out is
//! dropped with one line in the journal and no trace in the interface at all: a person must
//! never be shown a letter this program cannot prove is the author's.

use crate::cards::{CARDS_KEPT, CardImage, CardView, FeedCard, IMAGE_CAP};
use crate::letters::{Date, FeedItem, NEWS_KEPT};

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
        0x44, 0xCF, 0xB8, 0xB2, 0x7F, 0x43, 0x40, 0xF0, 0x58, 0x5E, 0x82, 0x4A, 0x46, 0xEE, 0xA1,
        0x8B, 0xD9, 0x84, 0x48, 0x47, 0x45, 0x50, 0xB4, 0x12, 0xA8, 0x3D, 0x45, 0x71, 0xB3, 0x9B,
        0xC0, 0x8E, 0x25, 0xF3, 0x49, 0xD0, 0x99, 0xC8, 0x6D, 0xA6, 0x9F, 0x96, 0x58, 0x41, 0x86,
        0x91, 0x86, 0xA4, 0x33, 0x81, 0xB3, 0xAD, 0x71, 0xC9, 0xB5, 0x84, 0x74, 0xDB, 0x90, 0x38,
        0x34, 0x7B, 0x3D, 0x66,
    ],
    // The reserve key — it does not live on this machine.
    [
        0x94, 0xFC, 0x7F, 0x20, 0x1A, 0x69, 0x5E, 0x35, 0xD2, 0x09, 0x46, 0xE1, 0xF9, 0x49, 0x4E,
        0x45, 0x88, 0x90, 0xF6, 0xF7, 0x18, 0x23, 0x46, 0x56, 0x02, 0x19, 0xA1, 0x9F, 0xC8, 0xBC,
        0x4D, 0x27, 0xF1, 0x25, 0x37, 0x4A, 0x7F, 0x7E, 0x0A, 0xDA, 0x2D, 0xAD, 0x30, 0x41, 0xE8,
        0x67, 0x98, 0xC9, 0x56, 0x26, 0xA8, 0x0D, 0x9D, 0xD9, 0x9C, 0xCA, 0x04, 0xB6, 0x7C, 0x86,
        0x62, 0xFC, 0xFF, 0x9C,
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
    /// ⭐ The author's cards — stage Э121, task T-121-1: at most [`CARDS_KEPT`] of the records
    /// `[[card]]`, the ones with the greatest numbers, **oldest first**. A record of the wrong shape
    /// costs itself and not the file ([`cards_of`]).
    pub cards: Vec<FeedCard>,
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
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

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
        cards: cards_of(document.card, language),
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
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WinHttpCloseHandle, WinHttpOpen, WinHttpSetTimeouts,
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
    let connection = unsafe { WinHttpConnect(session, PCWSTR(host.as_ptr()), request_of.port, 0) };

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
        crate::app::report_non_critical("WinHttpOpenRequest", &windows::core::Error::from_thread());
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
        crate::app::report_non_critical("WinHttpSendRequest", &windows::core::Error::from_thread());
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
    read_from(&crate::links::FEED_URLS, language)
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
        crate::links::FEED_URLS
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

    (places.feed_url != crate::links::FEED_URLS[0]).then(|| (vec![places.feed_url], places.keys))
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
        if crate::links::is_placeholder(url) {
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
    /// ⭐ `[[card]]` — stage Э121, taken as raw TOML and read card by card ([`cards_of`]): a card
    /// of the wrong shape must cost itself, never the letters of the file.
    #[serde(default)]
    card: Option<toml::Value>,
}

/// One `[[card]]` as written — stage Э121, task T-121-1. Every field but the number and the term
/// may be left out.
///
/// ⚠ **The languages are raw TOML, and that is the lesson of `[[item]]`** (the finding of the
/// window of the cards, п. 1): `RawItem` takes every key it does not know as a language and refuses
/// the whole feed when it is not one, so not one field can ever be added to an `[[item]]`. Here a
/// key this build does not know is simply not a language it reads — a later version may give the
/// card a field, and this one reads the card past it.
#[derive(serde::Deserialize)]
struct RawCard {
    id: u64,
    from: Date,
    to: Date,
    #[serde(default)]
    view: String,
    #[serde(default)]
    side: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    sha256: String,
    #[serde(default)]
    size: u64,
    /// Everything else in the record — the language tables among it.
    #[serde(flatten)]
    rest: std::collections::BTreeMap<String, toml::Value>,
}

/// **The cards of the feed** — every `[[card]]` read on its own, the bad ones skipped, the newest
/// [`CARDS_KEPT`] kept in the order of their numbers. Nothing for a file without the array, or with
/// something that is not an array of tables under its name.
fn cards_of(raw: Option<toml::Value>, language: &str) -> Vec<FeedCard> {
    let Some(toml::Value::Array(records)) = raw else {
        return Vec::new();
    };

    let mut cards: Vec<FeedCard> = records
        .into_iter()
        .filter_map(|record| record.try_into::<RawCard>().ok())
        .filter_map(|raw| card_of(raw, language))
        .collect();

    cards.sort_unstable_by_key(|card| card.id);
    cards.dedup_by_key(|card| card.id);

    if cards.len() > CARDS_KEPT {
        cards.drain(..cards.len() - CARDS_KEPT);
    }

    cards
}

/// One card checked: a term that runs forward, words in a language this program shows (its own,
/// else English, else Russian — as an entry's), the picture only when the record names the whole of
/// it — an address, the sum and a size from one byte to [`IMAGE_CAP`]. Whether the address is the
/// author's is the download's question, not the shape's.
fn card_of(mut raw: RawCard, language: &str) -> Option<FeedCard> {
    if raw.to < raw.from {
        return None;
    }

    let words: RawText = [language, "en", "ru"]
        .into_iter()
        .find_map(|tag| raw.rest.remove(tag)?.try_into::<RawText>().ok())?;

    let image = crate::cards::sum_of(&raw.sha256)
        .filter(|_| !raw.image.is_empty() && raw.size > 0 && raw.size <= IMAGE_CAP)
        .map(|sha256| CardImage {
            url: raw.image,
            sha256,
            size: raw.size,
        });

    Some(FeedCard {
        id: raw.id,
        from: raw.from,
        to: raw.to,
        view: CardView::of(&raw.view, &raw.side),
        image,
        title: words.title,
        text: words.text,
        link: raw.link,
    })
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
    if raw.size == 0 || raw.version.is_empty() || raw.url.is_empty() {
        return None;
    }

    // One reading of the sixty-four digits for the installer and the picture of a card.
    let sha256 = crate::cards::sum_of(&raw.sha256)?;

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
