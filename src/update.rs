//! **The updater** — `LangSwitcherUpdate.exe`, stage Э104 (вопрос 189; TZ E104 §0 п. 13–19, 22).
//!
//! # Why a second file, and why this is the only code that downloads and starts anything
//!
//! Lang Switcher itself carries `uiAccess`, and SEC-06b promises that it downloads nothing and
//! starts nothing. The owner asked for the program to update itself without a question (189.2,
//! «Б — тихое без вопросов»); the two meet here. A separate executable — no window, no
//! `uiAccess`, a manifest of its own (`updater.rc`) — run once a day and at sign-in by the task
//! «Lang Switcher Update» of the scheduler under the system account (§0 п. 14), only when the
//! computer is idle. It and the program never talk to each other (SEC-04): what the updater
//! knows of the program is its file and whether its process runs.
//!
//! **Nothing of the feed is copied here.** The read, the signature, the parse, the hash and the
//! one `GET` are [`crate::letters::feed`]'s, the body the program reads its letters through.
//!
//! # The checks, in the order of §0 п. 15
//!
//! The signature of the feed (the keys of FR-102) → a `[download]` → the version strictly newer
//! than the installed one (read out of the version resource of `LangSwitcher.exe`; no going
//! back) → not stopped by `[stop]` and not marked failed → `https://`, the author's host, the
//! folder `/langswitcher/files/` → the download into `%ProgramData%\Lang_Switcher\update\`
//! (SYSTEM and Administrators only, checked every run) → the size the feed named, at most
//! 16 MB → the SHA-256 the feed named → the silent installation. Any refusal installs nothing,
//! writes one line in `updater.log` and leaves the next attempt to the next day.
//!
//! # The two phases (§0 п. 18)
//!
//! **Phase one** — this file as it was before the update: the checks, the sessions where the
//! program runs, `state.toml`, the installer started with `/UPDATE`, and out — it does not hold
//! its own file. **Phase two** — the updater the installer has just put down, started by the
//! installer at its very end under `/UPDATE` (`DeinitializeSetup`, so that a failed installation
//! gets a phase two too): the program is raised again in every session it ran in, through
//! `explorer.exe` in that session with that person's token, and if it does not appear within a
//! minute the copy of the previous installer is put back and the new version is marked failed.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::letters::{self, Date, feed};

// =========================================================================================
// 1. Names
// =========================================================================================

/// The task of the scheduler that runs this file — §0 п. 14. The installer makes it and the
/// uninstaller removes it; the name is the one a person sees in «Планировщик заданий».
pub const TASK_NAME: &str = "Lang Switcher Update";

/// The file of the updater, beside `LangSwitcher.exe` in the program folder.
pub const EXE_NAME: &str = "LangSwitcherUpdate.exe";

/// The folder under `%ProgramData%` that holds `update\` — the same name as the program's own
/// folder under `%APPDATA%` ([`crate::CONFIG_DIR_NAME`]).
pub const DATA_DIR: &str = crate::CONFIG_DIR_NAME;

/// The folder of the updater inside [`DATA_DIR`]: the state, the journal, the installers.
pub const UPDATE_DIR: &str = "update";

/// The state of the updater — §0 п. 17.
pub const STATE_FILE: &str = "state.toml";

/// The journal of the updater — §0 п. 17: events and codes and nothing else (SEC-07).
pub const LOG_FILE: &str = "updater.log";

/// The journal before the last turn — one file, replaced at every turn.
pub const LOG_KEPT: &str = "updater.log.1";

/// The size the journal turns at — §0 п. 17, 64 KB.
pub const LOG_CAP: u64 = 64 * 1024;

/// The copy of the installer of the version installed now. **The installer puts it here
/// itself**, from `{srcexe}`, on every installation (§0 п. 17, посылка П9): what a rollback
/// installs.
pub const INSTALLED_SETUP: &str = "installed-setup.exe";

/// [`INSTALLED_SETUP`] while a newer version is being installed — moved aside by phase one,
/// because the new installer writes its own copy over the old name.
pub const ROLLBACK_SETUP: &str = "rollback-setup.exe";

/// The installer the feed named, once its size and its sum have been checked.
pub const DOWNLOADED_SETUP: &str = "downloaded-setup.exe";

/// The same file while it arrives.
pub const DOWNLOAD_PART: &str = "downloaded-setup.part";

/// The journal of Inno Setup for the runs this updater starts.
pub const SETUP_LOG: &str = "setup.log";

/// The largest installer this updater downloads — §0 п. 15. The installer of 0.103.0 was
/// 3 485 568 bytes.
pub const MAX_INSTALLER_BYTES: u64 = 16 * 1024 * 1024;

/// How long phase two waits for the program to appear in a session it raised it in — §0 п. 18.
pub const RAISE_WAIT_SECONDS: u64 = 60;

/// How long phase two waits for the installer that started it to end.
pub const SETUP_WAIT_SECONDS: u64 = 180;

/// A phase one older than this, with no phase two after it, is an installation that never got
/// to its end (§0 п. 18): the next run raises the program it closed and forgets it.
pub const STALE_PENDING_SECONDS: u64 = 60 * 60;

/// The switch the installer starts phase two with.
pub const PHASE_TWO_SWITCH: &str = "/phase2";

/// The switch of the helper that asks the program to close in its own session — решение 189.33.
pub const CLOSE_SWITCH: &str = "/close";

/// How long phase one waits for the program to close by itself before it ends it — решение 189.33,
/// the ten seconds the uninstaller gives it (155.25).
pub const CLOSE_WAIT_SECONDS: u64 = 10;

/// The class of the one top-level window of the program — `WINDOW_CLASS_NAME` of `src\app.rs` and
/// `ProductWindowClass` of the installer; `tests\installer_updater.rs` holds the three together.
pub const PROGRAM_WINDOW_CLASS: &str = "LangSwitcher.Hidden";

/// The switches phase one starts an installer with — silent, no question, no restart, and
/// `/UPDATE`, which tells the installer to start phase two at its end (§0 п. 18).
pub const SETUP_SWITCHES: &str = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /UPDATE";

/// One updater at a time — beside the task's own «no second instance» (§0 п. 14).
pub const UPDATER_MUTEX: &str = "Global\\Lang_Switcher.Update";

/// The mutex the installer holds while it runs (`SetupMutex` of `installer\LangSwitcher.iss`):
/// phase two waits for it to go before it raises anything or starts a rollback.
pub const SETUP_MUTEX: &str = "Global\\Lang_Switcher.Setup";

// =========================================================================================
// 2. The decision — pure
// =========================================================================================

/// Which of the two keys of FR-102 the feed was signed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signer {
    /// The working key — the first of [`feed::FEED_KEYS`].
    Working,
    /// The reserve key — the second, off the author's machine (189.14).
    Reserve,
}

impl Signer {
    /// The signer of the key at `index` of [`feed::FEED_KEYS`].
    pub fn of_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Working),
            1 => Some(Self::Reserve),
            _ => None,
        }
    }
}

/// Why nothing is installed — one closed set, one word for the journal each (SEC-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// The feed is signed with the working key, and the reserve has revoked it.
    WorkingKeyRevoked,
    /// The feed has no `[download]` — the ordinary day.
    NoDownload,
    /// The `[download]` is not of the shape of §0 п. 16.
    DownloadMalformed,
    /// A `[stop]` of the reserve key that cannot be read: nothing is installed.
    StopMalformed,
    /// The `[download]` has no `update` entry of the same version beside it.
    NoUpdateEntry,
    /// The version is not newer than the installed one — equal, older, or unreadable.
    NotNewer,
    /// The version is stopped by `[stop]`.
    Stopped,
    /// The version was installed here once and did not start; it is never tried again.
    FailedBefore,
    /// The address is not under [`crate::letters::links::UPDATE_FILES_URL`].
    ForeignAddress,
    /// The size named is over [`MAX_INSTALLER_BYTES`].
    TooBig,
}

impl Skip {
    /// The word the journal says.
    pub fn journal_name(self) -> &'static str {
        match self {
            Self::WorkingKeyRevoked => "refused: the working key is revoked",
            Self::NoDownload => "nothing to install: no [download]",
            Self::DownloadMalformed => "refused: [download] malformed",
            Self::StopMalformed => "refused: [stop] malformed",
            Self::NoUpdateEntry => "refused: no update entry of the version",
            Self::NotNewer => "nothing to install: not newer",
            Self::Stopped => "refused: the version is stopped",
            Self::FailedBefore => "refused: the version failed here before",
            Self::ForeignAddress => "refused: foreign address",
            Self::TooBig => "refused: over 16 MB",
        }
    }
}

/// **What to install, or why nothing** — §0 п. 15, the checks that need no network, in their
/// order. `files` is the folder the installer must be under: [`crate::letters::links::
/// UPDATE_FILES_URL`] for every build but the stand's.
pub fn decide<'f>(
    installed: &str,
    feed: &'f feed::Feed,
    signer: Signer,
    state: &State,
    files: &str,
) -> Result<&'f feed::Download, Skip> {
    if signer == Signer::Working && state.working_key_revoked {
        return Err(Skip::WorkingKeyRevoked);
    }

    let download = match &feed.download {
        feed::Table::Absent => return Err(Skip::NoDownload),
        feed::Table::Malformed => return Err(Skip::DownloadMalformed),
        feed::Table::Read(download) => download,
    };

    if feed
        .update
        .as_ref()
        .and_then(|entry| entry.version.as_deref())
        != Some(download.version.as_str())
    {
        return Err(Skip::NoUpdateEntry);
    }

    if installed.is_empty() || !letters::version_is_newer(&download.version, installed) {
        return Err(Skip::NotNewer);
    }

    // `[stop]` counts in a feed of the reserve key only (§0 п. 16); a working key that signed
    // one is simply not heard on it. What a reserve feed once stopped stays stopped
    // ([`State::stopped`]) — a later feed of the working key cannot bring it back.
    if signer == Signer::Reserve {
        match &feed.stop {
            feed::Table::Malformed => return Err(Skip::StopMalformed),
            feed::Table::Read(stop) if stop.versions.contains(&download.version) => {
                return Err(Skip::Stopped);
            }
            feed::Table::Read(_) | feed::Table::Absent => {}
        }
    }

    if state.stopped.contains(&download.version) {
        return Err(Skip::Stopped);
    }

    if state.failed.contains(&download.version) {
        return Err(Skip::FailedBefore);
    }

    if !address_is_ours(&download.url, files) {
        return Err(Skip::ForeignAddress);
    }

    if download.size > MAX_INSTALLER_BYTES {
        return Err(Skip::TooBig);
    }

    Ok(download)
}

/// **Whether an address is a file in the one folder** — `files` and then plain path segments of
/// letters, digits, `.`, `-` and `_`: no `..`, no query, no fragment, no `\`, no `%`, no port,
/// no credentials. The scheme and the host are the prefix's, compared byte for byte — the
/// author's own script writes them exactly so (`tools\sign-news.ps1`).
pub fn address_is_ours(url: &str, files: &str) -> bool {
    let Some(rest) = url.strip_prefix(files) else {
        return false;
    };

    !files.is_empty()
        && !rest.is_empty()
        && rest.len() <= 200
        && rest.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
        })
}

/// Whether what arrived is what the feed named — the size first, then the sum (§0 п. 15).
pub fn received_matches(
    size: u64,
    sum: &[u8; 32],
    download: &feed::Download,
) -> Result<(), Refused> {
    if size != download.size || size > MAX_INSTALLER_BYTES {
        return Err(Refused::Size);
    }

    if *sum != download.sha256 {
        return Err(Refused::Sum);
    }

    Ok(())
}

/// Why a download was refused — a word for the journal each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The address has no target this build reaches.
    Address,
    /// The host did not answer, or answered with more than the feed named.
    Network,
    /// The size is not the one the feed named.
    Size,
    /// The SHA-256 is not the one the feed named.
    Sum,
    /// The file could not be written, or moved into its name.
    Disk,
}

impl Refused {
    /// The word the journal says.
    pub fn journal_name(self) -> &'static str {
        match self {
            Self::Address => "download refused: no target",
            Self::Network => "download refused: network or oversize",
            Self::Size => "download refused: size differs",
            Self::Sum => "download refused: SHA-256 differs",
            Self::Disk => "download refused: disk",
        }
    }
}

// =========================================================================================
// 3. The state — `state.toml`, §0 п. 17
// =========================================================================================

/// What the updater remembers between two runs.
///
/// No `deny_unknown_fields`: a field a later updater adds is read past by this one, and the
/// older updater of a rollback reads the file of the newer one (§0 п. 18).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct State {
    /// The reserve key has revoked the working key — for good (§0 п. 16).
    #[serde(default)]
    pub working_key_revoked: bool,
    /// The versions a feed of the reserve key has stopped — remembered, so that a later feed
    /// of the working key cannot offer them again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stopped: Vec<String>,
    /// The versions that were installed here and did not start — never tried again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
    /// The day of the last attempt, whatever came of it — one a day (§0 п. 2, as 148.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_try: Option<Date>,
    /// An installation phase one has started and phase two has not finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

/// The installation in flight — what phase two needs to finish it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Pending {
    /// The version being installed.
    pub version: String,
    /// The version that was installed before it.
    pub from: String,
    /// The sessions the program ran in when phase one closed it — where phase two raises it
    /// (§0 п. 6: not running — not raised).
    #[serde(default)]
    pub sessions: Vec<u32>,
    /// Whether [`ROLLBACK_SETUP`] holds the installer of `from`.
    #[serde(default)]
    pub rollback_copy: bool,
    /// The new version did not start and the old one is being put back.
    #[serde(default)]
    pub rolling_back: bool,
    /// When phase one wrote this, in seconds since 1970 — what makes a phase one stale.
    #[serde(default)]
    pub since: u64,
}

impl State {
    /// The state of the file — the default when there is none, `None` when there is one this
    /// build cannot read (and then nothing is done: a lost revocation is not a default).
    pub fn load(path: &Path) -> Option<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).ok(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Self::default()),
            Err(_) => None,
        }
    }

    /// Writes the state: a temporary file beside it, then one move over the old name, so that
    /// a power cut leaves the old state or the new one and never half of either.
    pub fn save(&self, path: &Path) -> bool {
        let Ok(text) = toml::to_string(self) else {
            return false;
        };

        let temporary = path.with_extension("toml.tmp");

        if std::fs::write(&temporary, text.replace('\n', "\r\n")).is_err() {
            return false;
        }

        replace_file(&temporary, path)
    }

    /// Remembers a version as failed, once.
    pub fn mark_failed(&mut self, version: &str) {
        if !self.failed.iter().any(|known| known == version) {
            self.failed.push(version.to_owned());
        }
    }

    /// Takes in what a feed of the **reserve** key says in `[stop]`: the revocation, for good,
    /// and the stopped versions. A feed of the working key changes nothing here.
    pub fn take_stop(&mut self, feed: &feed::Feed, signer: Signer) -> bool {
        let (Signer::Reserve, feed::Table::Read(stop)) = (signer, &feed.stop) else {
            return false;
        };

        let before = self.clone();

        self.working_key_revoked |= stop.revoke_working;

        for version in &stop.versions {
            if !self.stopped.contains(version) {
                self.stopped.push(version.clone());
            }
        }

        *self != before
    }
}

/// Moves `from` over `to` — `MoveFileExW` with replace and write-through.
fn replace_file(from: &Path, to: &Path) -> bool {
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let from = wide_path(from);
    let to = wide_path(to);

    // SAFETY: both names are NUL-terminated buffers of this frame.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .is_ok()
}

/// A path as a NUL-terminated wide string.
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt as _;

    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

// =========================================================================================
// 4. The journal — `updater.log`, §0 п. 17
// =========================================================================================

/// The journal of the updater: one line per event, a time, the event and a code — **no path,
/// no name, no address and no version** (SEC-07). It turns at [`LOG_CAP`] into [`LOG_KEPT`].
pub struct Journal {
    path: PathBuf,
    kept: PathBuf,
}

impl Journal {
    /// The journal of a folder.
    pub fn at(folder: &Path) -> Self {
        Self {
            path: folder.join(LOG_FILE),
            kept: folder.join(LOG_KEPT),
        }
    }

    /// Writes one line. A journal that cannot be written costs nothing else.
    pub fn line(&self, event: &str, code: u32) {
        let text = format!("{} {event} {code}\r\n", stamp_of(seconds_now()));

        let size = std::fs::metadata(&self.path).map_or(0, |meta| meta.len());

        if size + text.len() as u64 > LOG_CAP {
            let _ = replace_file(&self.path, &self.kept);
        }

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(text.as_bytes());
        }
    }
}

/// Seconds since 1970, by the clock of the machine.
pub fn seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A moment as the journal writes it — `2026-10-25T12:00:00Z`, UTC: the journal is read across
/// machines and the zone of the machine is nobody's business (SEC-07).
pub fn stamp_of(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let rest = seconds % 86_400;

    format!(
        "{}T{:02}:{:02}:{:02}Z",
        Date::from_days(days),
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

// =========================================================================================
// 5. The folder — SYSTEM and Administrators only, §0 п. 15 (посылка П10)
// =========================================================================================

/// One access-allowed entry of a folder's list, as far as the verdict needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowed {
    /// The rights the entry grants.
    pub mask: u32,
    /// Whether the entry is for SYSTEM, Administrators or CREATOR OWNER.
    pub trusted: bool,
}

/// Every right that lets a holder change what the folder holds or who may: write and append,
/// extended attributes and attributes, delete a child, delete, change the list, take the owner,
/// and the two generic rights that include them.
pub const WRITE_RIGHTS: u32 = 0x0002 // FILE_WRITE_DATA / FILE_ADD_FILE
    | 0x0004 // FILE_APPEND_DATA / FILE_ADD_SUBDIRECTORY
    | 0x0010 // FILE_WRITE_EA
    | 0x0040 // FILE_DELETE_CHILD
    | 0x0100 // FILE_WRITE_ATTRIBUTES
    | 0x0001_0000 // DELETE
    | 0x0004_0000 // WRITE_DAC
    | 0x0008_0000 // WRITE_OWNER
    | 0x1000_0000 // GENERIC_ALL
    | 0x4000_0000; // GENERIC_WRITE

/// **Whether a folder may be trusted with an installer** — pure: its owner is SYSTEM or
/// Administrators, it has a list, and no entry of that list gives anybody else a right to
/// write. An entry that only children inherit counts too: a right a new file would carry is a
/// right to that file.
pub fn folder_verdict(owner_trusted: bool, list: Option<&[Allowed]>) -> bool {
    let Some(list) = list else {
        // A folder without a list is open to everybody.
        return false;
    };

    owner_trusted
        && list
            .iter()
            .all(|entry| entry.trusted || entry.mask & WRITE_RIGHTS == 0)
}

/// Whether the folder on disk passes [`folder_verdict`] — and is a folder, and not a link.
pub fn folder_is_trusted(folder: &Path) -> bool {
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, GetFileAttributesW,
        INVALID_FILE_ATTRIBUTES,
    };
    use windows::core::PCWSTR;

    let name = wide_path(folder);

    // SAFETY: a NUL-terminated buffer of this frame.
    let attributes = unsafe { GetFileAttributesW(PCWSTR(name.as_ptr())) };

    if attributes == INVALID_FILE_ATTRIBUTES
        || attributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0
        || attributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
    {
        return false;
    }

    let Some((owner, list)) = security_of(&name) else {
        return false;
    };

    folder_verdict(owner, list.as_deref())
}

/// The owner (trusted or not) and the access-allowed entries of a file's list — `None` when the
/// descriptor cannot be read; the list `None` when the descriptor has none.
fn security_of(name: &[u16]) -> Option<(bool, Option<Vec<Allowed>>)> {
    use windows::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, GetFileSecurityW,
        GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID,
    };
    use windows::core::{BOOL, PCWSTR};

    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

    let wanted = (OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION).0;
    let mut needed = 0u32;

    // SAFETY: the first call asks for the size only — no buffer, a length of zero.
    let _ = unsafe { GetFileSecurityW(PCWSTR(name.as_ptr()), wanted, None, 0, &mut needed) };

    if needed == 0 || needed > 64 * 1024 {
        return None;
    }

    // `u64` elements for the alignment a descriptor's fields want.
    let mut buffer = vec![0u64; usize::try_from(needed).ok()?.div_ceil(8)];
    let descriptor = PSECURITY_DESCRIPTOR(buffer.as_mut_ptr().cast());

    // SAFETY: `buffer` holds at least `needed` bytes and lives for this frame.
    let read = unsafe {
        GetFileSecurityW(
            PCWSTR(name.as_ptr()),
            wanted,
            Some(descriptor),
            needed,
            &mut needed,
        )
    };

    if !read.as_bool() {
        return None;
    }

    let mut owner = PSID::default();
    let mut defaulted = BOOL::default();

    // SAFETY: `descriptor` is the descriptor just read; the two outputs are live locals.
    unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted) }.ok()?;

    let owner_trusted = !owner.0.is_null()
        && (sid_is(owner, WellKnown::System) || sid_is(owner, WellKnown::Administrators));

    let mut present = BOOL::default();
    let mut list: *mut ACL = std::ptr::null_mut();

    // SAFETY: as above.
    unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut list, &mut defaulted) }
        .ok()?;

    if !present.as_bool() || list.is_null() {
        return Some((owner_trusted, None));
    }

    let mut size = ACL_SIZE_INFORMATION::default();

    // SAFETY: `list` points into `buffer`; `size` is a live local of the size the call is told.
    unsafe {
        GetAclInformation(
            list,
            std::ptr::from_mut(&mut size).cast(),
            u32::try_from(size_of::<ACL_SIZE_INFORMATION>()).ok()?,
            AclSizeInformation,
        )
    }
    .ok()?;

    let mut entries = Vec::new();

    for index in 0..size.AceCount {
        let mut entry: *mut std::ffi::c_void = std::ptr::null_mut();

        // SAFETY: `index` is below the count the list reported; `entry` a live local.
        unsafe { GetAce(list, index, &mut entry) }.ok()?;

        // SAFETY: every entry starts with an `ACE_HEADER`, and `entry` points at one inside
        // `buffer`.
        let header = unsafe { entry.cast::<ACE_HEADER>().read_unaligned() };

        // Deny entries take rights away and are no threat to the verdict; every allowing kind
        // other than the plain one is judged as «somebody else may write» if it grants a right.
        match header.AceType {
            ACCESS_ALLOWED_ACE_TYPE => {
                // SAFETY: an entry of this type is an `ACCESS_ALLOWED_ACE`, the SID at its
                // `SidStart`.
                let allowed = unsafe { entry.cast::<ACCESS_ALLOWED_ACE>().read_unaligned() };
                let sid = PSID(
                    // SAFETY: `SidStart` is a field of the entry `entry` points at.
                    unsafe {
                        std::ptr::addr_of_mut!((*entry.cast::<ACCESS_ALLOWED_ACE>()).SidStart)
                    }
                    .cast(),
                );

                entries.push(Allowed {
                    mask: allowed.Mask,
                    trusted: sid_is(sid, WellKnown::System)
                        || sid_is(sid, WellKnown::Administrators)
                        || sid_is(sid, WellKnown::CreatorOwner),
                });
            }
            0x01 | 0x06 | 0x0A | 0x0E => {} // ACCESS_DENIED_*, the four kinds
            _ => {
                // SAFETY: every allowing kind has its mask right after the header.
                let mask = unsafe { entry.cast::<u8>().add(4).cast::<u32>().read_unaligned() };

                entries.push(Allowed {
                    mask,
                    trusted: false,
                });
            }
        }
    }

    Some((owner_trusted, Some(entries)))
}

/// The three accounts the folder may belong to.
#[derive(Clone, Copy)]
enum WellKnown {
    System,
    Administrators,
    CreatorOwner,
}

/// Whether a SID is one of [`WellKnown`].
fn sid_is(sid: windows::Win32::Security::PSID, which: WellKnown) -> bool {
    use windows::Win32::Security::{
        IsWellKnownSid, WinBuiltinAdministratorsSid, WinCreatorOwnerSid, WinLocalSystemSid,
    };

    let kind = match which {
        WellKnown::System => WinLocalSystemSid,
        WellKnown::Administrators => WinBuiltinAdministratorsSid,
        WellKnown::CreatorOwner => WinCreatorOwnerSid,
    };

    // SAFETY: `sid` points at a SID inside a descriptor or a token the caller holds.
    unsafe { IsWellKnownSid(sid, kind) }.as_bool()
}

/// Whether this process runs as the system account — the only account phase one and two run
/// under (§0 п. 14).
pub fn runs_as_system() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = HANDLE::default();

    // SAFETY: the pseudo-handle of this process and a live local.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }

    let mut buffer = [0u64; 32];
    let mut length = 0u32;

    // SAFETY: `buffer` is 256 bytes of this frame, enough for a TOKEN_USER and its SID.
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            u32::try_from(size_of_val(&buffer)).unwrap_or(0),
            &mut length,
        )
    };

    // SAFETY: closes exactly the token opened above, once.
    let _ = unsafe { CloseHandle(token) };

    if read.is_err() {
        return false;
    }

    // SAFETY: the call filled `buffer` with a TOKEN_USER.
    let user = unsafe { buffer.as_ptr().cast::<TOKEN_USER>().read_unaligned() };

    sid_is(user.User.Sid, WellKnown::System)
}

// =========================================================================================
// 6. The places — the program folder, the data folder, the feed and the keys
// =========================================================================================

/// Everything a run needs to know about where things are.
pub struct Places {
    /// `{app}` — the folder of this file.
    pub app_dir: PathBuf,
    /// `{app}\LangSwitcher.exe`.
    pub program: PathBuf,
    /// `%ProgramData%\Lang_Switcher`.
    pub data_dir: PathBuf,
    /// `%ProgramData%\Lang_Switcher\update`.
    pub update_dir: PathBuf,
    /// Where the feed is read from — [`crate::letters::links::FEED_URLS`].
    pub feed_url: String,
    /// The keys of FR-102, working first.
    pub keys: [[u8; 64]; 2],
    /// The one folder an installer may lie in.
    pub files: String,
}

impl Places {
    /// The places of this machine; under the `testing` feature, with the doors of the stand
    /// (TZ E104 §0 п. 22) laid over them.
    pub fn of_this_machine() -> Option<Self> {
        let app_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        let data_dir =
            known_folder(&windows::Win32::UI::Shell::FOLDERID_ProgramData)?.join(DATA_DIR);

        #[allow(unused_mut)]
        let mut places = Self {
            program: app_dir.join(crate::EXE_NAME),
            update_dir: data_dir.join(UPDATE_DIR),
            data_dir,
            app_dir,
            feed_url: letters::links::FEED_URLS[0].to_owned(),
            keys: feed::FEED_KEYS,
            files: letters::links::UPDATE_FILES_URL.to_owned(),
        };

        #[cfg(feature = "testing")]
        doors::lay_over(&mut places);

        Some(places)
    }
}

/// A known folder of the shell, by its GUID.
fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};

    // SAFETY: `id` is a GUID of this image; no token means «this process's».
    let path = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;

    // SAFETY: the call answered a NUL-terminated string it allocated; it is read once and
    // freed once, below.
    let text = unsafe { path.to_string() }.ok();

    // SAFETY: frees exactly the string the call allocated, once.
    unsafe { CoTaskMemFree(Some(path.0.cast())) };

    text.map(PathBuf::from)
}

/// **The doors of the stand** — TZ E104 §0 п. 22, and only under the `testing` feature: a file
/// `update-doors.toml` beside the updater names another feed address, other keys and another
/// folder of installers, for the guest of `Test_VM`. A Release image has no trace of it
/// (ворота 6 of `tools\verify-perimeter.ps1`).
#[cfg(feature = "testing")]
mod doors {
    /// The file of the doors.
    pub const FILE: &str = "update-doors.toml";

    #[derive(serde::Deserialize)]
    struct Doors {
        feed_url: Option<String>,
        working_key: Option<String>,
        reserve_key: Option<String>,
        files: Option<String>,
    }

    /// Sixty-four bytes from 128 hexadecimal digits.
    fn key_of(text: &str) -> Option<[u8; 64]> {
        let digits = text.trim().as_bytes();

        if digits.len() != 128 {
            return None;
        }

        let mut key = [0u8; 64];

        for (byte, pair) in key.iter_mut().zip(digits.chunks_exact(2)) {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;

            *byte = u8::try_from(high * 16 + low).ok()?;
        }

        Some(key)
    }

    /// Lays the doors over the places of this machine, when the file is there.
    pub fn lay_over(places: &mut super::Places) {
        let Ok(text) = std::fs::read_to_string(places.app_dir.join(FILE)) else {
            return;
        };

        let Ok(doors) = toml::from_str::<Doors>(&text) else {
            return;
        };

        if let Some(url) = doors.feed_url {
            places.feed_url = url;
        }

        if let Some(key) = doors.working_key.as_deref().and_then(key_of) {
            places.keys[0] = key;
        }

        if let Some(key) = doors.reserve_key.as_deref().and_then(key_of) {
            places.keys[1] = key;
        }

        if let Some(files) = doors.files {
            places.files = files;
        }
    }
}

// =========================================================================================
// 7. The program in the sessions — §0 п. 6, 18
// =========================================================================================

/// The sessions the program of `program` runs in now, each once, ascending. A copy of the
/// program anywhere else — a build of a developer — is not the program.
pub fn sessions_running(program: &Path) -> Vec<u32> {
    let mut sessions: Vec<u32> = processes_of(program)
        .into_iter()
        .map(|(_, session)| session)
        .collect();

    sessions.sort_unstable();
    sessions.dedup();
    sessions
}

/// Whether the program runs in this session.
pub fn running_in(session: u32, program: &Path) -> bool {
    processes_of(program).iter().any(|(_, at)| *at == session)
}

/// The processes whose image is `program` — `(pid, session)`.
fn processes_of(program: &Path) -> Vec<(u32, u32)> {
    use windows::Win32::System::RemoteDesktop::{
        WTS_CURRENT_SERVER_HANDLE, WTS_PROCESS_INFOW, WTSEnumerateProcessesW, WTSFreeMemory,
    };

    let wanted = program.to_string_lossy().to_lowercase();
    let file = crate::EXE_NAME.to_lowercase();

    let mut list: *mut WTS_PROCESS_INFOW = std::ptr::null_mut();
    let mut count = 0u32;

    // SAFETY: the local server, version 1, and two live locals the call fills.
    if unsafe {
        WTSEnumerateProcessesW(Some(WTS_CURRENT_SERVER_HANDLE), 0, 1, &mut list, &mut count)
    }
    .is_err()
    {
        return Vec::new();
    }

    let mut found = Vec::new();

    for index in 0..usize::try_from(count).unwrap_or(0) {
        // SAFETY: `index` is below the count the call answered for this array.
        let process = unsafe { list.add(index).read() };

        // SAFETY: the name is a NUL-terminated string inside the array the call allocated.
        let name = unsafe { process.pProcessName.to_string() }.unwrap_or_default();

        if name.to_lowercase() != file {
            continue;
        }

        // The one reader of a process's image is module `guard` (the sweep of tests\guard.rs).
        if crate::guard::process_image_path(process.ProcessId)
            .is_some_and(|image| image.to_lowercase() == wanted)
        {
            found.push((process.ProcessId, process.SessionId));
        }
    }

    // SAFETY: frees exactly the array the call allocated, once.
    unsafe { WTSFreeMemory(list.cast()) };

    found
}

/// What raising the program in a session came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raise {
    /// Nobody is signed in to that session any more — nothing to raise, nothing failed.
    NoUser,
    /// `explorer.exe` was started in the session with the program's path.
    Started,
    /// The session has a person, and the start was refused.
    Refused,
}

/// **Raises the program in a session the way a person does** — `explorer.exe "<program>"`,
/// started with that session's own token on its own desktop (§0 п. 18, посылка П5). Through the
/// shell because the program carries `uiAccess`: `CreateProcess` refuses such an image (740,
/// урок `[Run]` Э91), and the shell starts it with the pass and at the middle level.
pub fn raise_in(session: u32, program: &Path) -> Raise {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::RemoteDesktop::WTSQueryUserToken;
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
    };
    use windows::core::{PCWSTR, PWSTR};

    let mut token = HANDLE::default();

    // SAFETY: a live local the call fills; the caller runs as the system account.
    if unsafe { WTSQueryUserToken(session, &mut token) }.is_err() {
        return Raise::NoUser;
    }

    let Some(windows_dir) = known_folder(&windows::Win32::UI::Shell::FOLDERID_Windows) else {
        // SAFETY: closes exactly the token opened above, once.
        let _ = unsafe { CloseHandle(token) };
        return Raise::Refused;
    };

    let explorer = windows_dir.join("explorer.exe");
    let application = wide_path(&explorer);
    let mut command: Vec<u16> = format!("\"{}\" \"{}\"", explorer.display(), program.display())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut desktop: Vec<u16> = "winsta0\\default".encode_utf16().chain(Some(0)).collect();

    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(0),
        lpDesktop: PWSTR(desktop.as_mut_ptr()),
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();

    // SAFETY: `token` is the primary token of the session; every buffer is NUL-terminated and
    // lives for this frame; the command line is writable as the call requires.
    let started = unsafe {
        CreateProcessAsUserW(
            Some(token),
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            &startup,
            &mut process,
        )
    };

    // SAFETY: closes exactly the token and, when the start succeeded, the two handles of the
    // new process, once each.
    unsafe {
        let _ = CloseHandle(token);

        if started.is_ok() {
            let _ = CloseHandle(process.hThread);
            let _ = CloseHandle(process.hProcess);
        }
    }

    if started.is_ok() {
        Raise::Started
    } else {
        Raise::Refused
    }
}

/// Raises the program in every session of `sessions` it does not run in now and waits, up to
/// [`RAISE_WAIT_SECONDS`], for it to appear in each it was raised in. `false` only when it was
/// raised somewhere and did not appear there — a session nobody is signed in to fails nothing.
pub fn raise_and_check(sessions: &[u32], program: &Path, journal: &Journal) -> bool {
    let mut awaited = Vec::new();

    for &session in sessions {
        if running_in(session, program) {
            continue;
        }

        match raise_in(session, program) {
            Raise::Started => awaited.push(session),
            Raise::NoUser => journal.line("raise: nobody in the session", session),
            Raise::Refused => {
                journal.line("raise: start refused", session);
                awaited.push(session);
            }
        }
    }

    let deadline = seconds_now() + RAISE_WAIT_SECONDS;

    loop {
        awaited.retain(|&session| !running_in(session, program));

        if awaited.is_empty() {
            return true;
        }

        if seconds_now() >= deadline {
            for &session in &awaited {
                journal.line("raise: the program did not appear", session);
            }

            return false;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// **Closes the program in a session before the installer runs** — решение 189.33. Politely
/// first: a copy of this file under the system account, started in that session on its desktop
/// ([`start_system_in_session`]), posts the program's window `WM_ENDSESSION` with
/// `ENDSESSION_CLOSEAPP` — the message the uninstaller posts (155.25) and the end of a session
/// sends, which the program answers with its one way out, FR-83. If the program is still there
/// after [`CLOSE_WAIT_SECONDS`], its process is ended. `true` when it runs there no more.
///
/// Why a copy in the session and not a message from here: a window of another session is not on
/// this desktop — that is what «Session Mismatch» of the Restart Manager says (189.32) — and the
/// system account of that session is above the integrity of the program, whatever it is.
pub fn close_program_in(session: u32, program: &Path, journal: &Journal) -> bool {
    if !running_in(session, program) {
        return true;
    }

    let asked = std::env::current_exe()
        .ok()
        .is_some_and(|me| start_system_in_session(session, &me, CLOSE_SWITCH));

    journal.line(
        if asked {
            "close: asked politely"
        } else {
            "close: the helper did not start"
        },
        session,
    );

    let deadline = seconds_now() + CLOSE_WAIT_SECONDS;

    while asked && seconds_now() < deadline {
        if !running_in(session, program) {
            journal.line("close: the program closed by itself", session);
            return true;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    for (pid, at) in processes_of(program) {
        if at == session {
            terminate(pid);
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    let gone = !running_in(session, program);
    journal.line(
        if gone {
            "close: ended"
        } else {
            "close: still running"
        },
        session,
    );
    gone
}

/// Ends a process — the fallback of [`close_program_in`].
fn terminate(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    // SAFETY: a plain call; the handle is closed below.
    let Ok(process) = (unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }) else {
        return;
    };

    // SAFETY: `process` is the handle just opened with the right to end it.
    let _ = unsafe { TerminateProcess(process, 1) };
    // SAFETY: closes exactly the handle opened above, once.
    let _ = unsafe { CloseHandle(process) };
}

/// Starts `exe` with `arguments` **under this process's own account** (the system) **in
/// `session`**, on that session's desktop — the token of this process duplicated and given the
/// session's number.
fn start_system_in_session(session: u32, exe: &Path, arguments: &str) -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        DuplicateTokenEx, SecurityImpersonation, SetTokenInformation, TOKEN_ALL_ACCESS,
        TokenPrimary, TokenSessionId,
    };
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, GetCurrentProcess, OpenProcessToken, PROCESS_CREATION_FLAGS,
        PROCESS_INFORMATION, STARTUPINFOW,
    };
    use windows::core::{PCWSTR, PWSTR};

    let mut own = HANDLE::default();

    // SAFETY: the pseudo-handle of this process and a live local.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &mut own) }.is_err() {
        return false;
    }

    let mut token = HANDLE::default();

    // SAFETY: `own` is the token just opened; `token` a live local the call fills.
    let duplicated = unsafe {
        DuplicateTokenEx(
            own,
            TOKEN_ALL_ACCESS,
            None,
            SecurityImpersonation,
            TokenPrimary,
            &mut token,
        )
    };

    // SAFETY: closes exactly the token opened above, once.
    let _ = unsafe { CloseHandle(own) };

    if duplicated.is_err() {
        return false;
    }

    let mut number = session;

    // SAFETY: `token` is the primary token just made; the value is a live `u32` of this frame and
    // the length is its size.
    let moved = unsafe {
        SetTokenInformation(
            token,
            TokenSessionId,
            std::ptr::from_mut(&mut number).cast(),
            u32::try_from(size_of::<u32>()).unwrap_or(4),
        )
    };

    let application = wide_path(exe);
    let mut command: Vec<u16> = format!("\"{}\" {arguments}", exe.display())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut desktop: Vec<u16> = "winsta0\\default".encode_utf16().chain(Some(0)).collect();
    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(0),
        lpDesktop: PWSTR(desktop.as_mut_ptr()),
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();

    let started = moved.is_ok()
        // SAFETY: `token` is a primary token of this account in the session asked for; every
        // buffer is NUL-terminated and lives for this frame; the command line is writable.
        && unsafe {
            CreateProcessAsUserW(
                Some(token),
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                false,
                PROCESS_CREATION_FLAGS(0),
                None,
                PCWSTR::null(),
                &startup,
                &mut process,
            )
        }
        .is_ok();

    // SAFETY: closes exactly the token and, when the start succeeded, the two handles of the new
    // process, once each.
    unsafe {
        let _ = CloseHandle(token);

        if started {
            let _ = CloseHandle(process.hThread);
            let _ = CloseHandle(process.hProcess);
        }
    }

    started
}

/// **The helper's whole work** — finds the program's window on this desktop and posts it
/// `WM_ENDSESSION` (`wParam` TRUE, `lParam` `ENDSESSION_CLOSEAPP`), as the uninstaller does
/// (155.25). Posted, not sent: a program that hangs cannot hang the updater. `false` when there
/// is no such window.
fn ask_the_program_to_close() -> bool {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_ENDSESSION};
    use windows::core::PCWSTR;

    const ENDSESSION_CLOSEAPP: isize = 0x0000_0001;

    let class: Vec<u16> = PROGRAM_WINDOW_CLASS.encode_utf16().chain(Some(0)).collect();

    // SAFETY: a NUL-terminated class name of this frame; no window name.
    let Ok(window) = (unsafe { FindWindowW(PCWSTR(class.as_ptr()), PCWSTR::null()) }) else {
        return false;
    };

    // SAFETY: `window` is the window just found; the message carries two plain values.
    unsafe {
        PostMessageW(
            Some(window),
            WM_ENDSESSION,
            WPARAM(1),
            LPARAM(ENDSESSION_CLOSEAPP),
        )
    }
    .is_ok()
}

// =========================================================================================
// 8. The download and the installer
// =========================================================================================

/// **Downloads the installer the feed named** into `to`: through [`feed::get`], hashed as it
/// arrives, the size and the sum checked before the file takes its name (§0 п. 15).
pub fn download(download: &feed::Download, to: &Path, part: &Path) -> Result<(), Refused> {
    let target = feed::target_of(&download.url).ok_or(Refused::Address)?;

    let _ = std::fs::remove_file(part);

    let mut file = std::fs::File::create(part).map_err(|_| Refused::Disk)?;
    let mut hasher = feed::Sha256::new().ok_or(Refused::Disk)?;
    let mut written = true;

    let total = feed::get(&target, download.size, &mut |chunk| {
        written = written && file.write_all(chunk).is_ok() && hasher.update(chunk);
        written
    });

    let flushed = file.sync_all().is_ok();
    drop(file);

    let outcome = match (total, written && flushed) {
        (_, false) => Err(Refused::Disk),
        (None, true) => Err(Refused::Network),
        (Some(size), true) => match hasher.finish() {
            Some(sum) => received_matches(size, &sum, download),
            None => Err(Refused::Disk),
        },
    };

    if outcome.is_ok() && replace_file(part, to) {
        return Ok(());
    }

    let _ = std::fs::remove_file(part);

    outcome.and(Err(Refused::Disk))
}

/// Starts an installer with [`SETUP_SWITCHES`] and its journal, and does not wait for it.
fn start_setup(setup: &Path, journal_file: &Path) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        CreateProcessW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
    };
    use windows::core::{PCWSTR, PWSTR};

    let application = wide_path(setup);
    let mut command: Vec<u16> = format!(
        "\"{}\" {SETUP_SWITCHES} \"/LOG={}\"",
        setup.display(),
        journal_file.display()
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();

    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(0),
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();

    // SAFETY: every buffer is NUL-terminated and lives for this frame; the command line is
    // writable as the call requires.
    let started = unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command.as_mut_ptr())),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            &startup,
            &mut process,
        )
    };

    if started.is_ok() {
        // SAFETY: closes exactly the two handles of the new process, once each.
        unsafe {
            let _ = CloseHandle(process.hThread);
            let _ = CloseHandle(process.hProcess);
        }
    }

    started.is_ok()
}

/// Waits, up to `seconds`, for the installer's mutex to be gone — the installer has ended.
fn wait_for_setup_to_end(seconds: u64) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    use windows::core::PCWSTR;

    let name: Vec<u16> = SETUP_MUTEX.encode_utf16().chain(Some(0)).collect();
    let deadline = seconds_now() + seconds;

    loop {
        // SAFETY: a NUL-terminated name of this frame; the handle is closed at once.
        match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(name.as_ptr())) } {
            Err(_) => return true,
            Ok(handle) => {
                // SAFETY: closes exactly the handle just opened, once.
                let _ = unsafe { CloseHandle(handle) };
            }
        }

        if seconds_now() >= deadline {
            return false;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Takes the one-at-a-time mutex of the updater; `None` when another updater holds it.
fn take_updater_mutex() -> Option<windows::Win32::Foundation::HANDLE> {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let name: Vec<u16> = UPDATER_MUTEX.encode_utf16().chain(Some(0)).collect();

    // SAFETY: a NUL-terminated name of this frame.
    let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }.ok()?;

    // SAFETY: a plain read of this thread's last error, right after the call that set it.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: closes exactly the handle just opened, once.
        let _ = unsafe { CloseHandle(handle) };
        return None;
    }

    Some(handle)
}

// =========================================================================================
// 9. The run
// =========================================================================================

/// The exit codes of a run. Zero whatever the feed said: a refusal is in the journal.
pub mod code {
    /// Done — something was installed, or the journal says why not.
    pub const DONE: u8 = 0;
    /// Not the system account: the task runs this file, a person does not.
    pub const NOT_SYSTEM: u8 = 2;
    /// Another updater is running.
    pub const BUSY: u8 = 3;
    /// The folder is missing or not SYSTEM's and Administrators' only — nothing is written.
    pub const FOLDER: u8 = 4;
    /// `state.toml` cannot be read — nothing is done.
    pub const STATE: u8 = 5;
    /// The folders of this machine cannot be told.
    pub const PLACES: u8 = 6;
    /// The helper `/close` found no window of the program to ask.
    pub const CLOSE: u8 = 7;
}

/// **The whole of a run of `LangSwitcherUpdate.exe`** — phase one, or phase two under
/// [`PHASE_TWO_SWITCH`].
pub fn run() -> ExitCode {
    if !runs_as_system() {
        return ExitCode::from(code::NOT_SYSTEM);
    }

    // The helper of [`close_program_in`]: a copy of this file under the system account in the
    // session of the program, which asks its window to close and is gone. It takes no mutex — the
    // updater that started it holds it — and touches no file.
    if std::env::args()
        .skip(1)
        .any(|argument| argument == CLOSE_SWITCH)
    {
        return ExitCode::from(if ask_the_program_to_close() {
            code::DONE
        } else {
            code::CLOSE
        });
    }

    let Some(places) = Places::of_this_machine() else {
        return ExitCode::from(code::PLACES);
    };

    let Some(_mutex) = take_updater_mutex() else {
        return ExitCode::from(code::BUSY);
    };

    if !folder_is_trusted(&places.data_dir) || !folder_is_trusted(&places.update_dir) {
        return ExitCode::from(code::FOLDER);
    }

    let journal = Journal::at(&places.update_dir);
    let phase_two = std::env::args()
        .skip(1)
        .any(|argument| argument == PHASE_TWO_SWITCH);

    let outcome = if phase_two {
        phase_two_of(&places, &journal)
    } else {
        phase_one_of(&places, &journal)
    };

    ExitCode::from(outcome)
}

/// Phase one — §0 п. 15, 18.
fn phase_one_of(places: &Places, journal: &Journal) -> u8 {
    let state_file = places.update_dir.join(STATE_FILE);

    let Some(mut state) = State::load(&state_file) else {
        journal.line("state unreadable", 0);
        return code::STATE;
    };

    if let Some(pending) = state.pending.clone() {
        if seconds_now().saturating_sub(pending.since) < STALE_PENDING_SECONDS {
            journal.line("an installation is in flight", 0);
            return code::DONE;
        }

        // A phase one with no phase two after it: the installer never got to its end. The
        // program it closed is raised again, the copy put back, the installation forgotten.
        journal.line("an installation did not finish", 0);
        let _ = raise_and_check(&pending.sessions, &places.program, journal);
        put_rollback_copy_back(places, &pending);
        state.pending = None;
        let _ = state.save(&state_file);
    }

    let Some(today) = letters::today() else {
        journal.line("no date", 0);
        return code::DONE;
    };

    if state.last_try == Some(today) {
        return code::DONE;
    }

    state.last_try = Some(today);

    if !state.save(&state_file) {
        journal.line("state not written", 0);
        return code::STATE;
    }

    journal.line("feed request", 0);

    let Some(text) = feed::fetch(&places.feed_url) else {
        journal.line("feed: no answer", 0);
        return code::DONE;
    };

    let read = feed::read_document_by(&text, "en", |body, signature| {
        feed::verify_which(body, signature, &places.keys)
    });

    let (document, signer) = match read {
        Ok((document, index)) => match Signer::of_index(index) {
            Some(signer) => (document, signer),
            None => return code::DONE,
        },
        Err(refusal) => {
            journal.line(refusal.journal_name(), 0);
            return code::DONE;
        }
    };

    if signer == Signer::Working && state.working_key_revoked {
        journal.line(Skip::WorkingKeyRevoked.journal_name(), 0);
        return code::DONE;
    }

    if state.take_stop(&document, signer) {
        journal.line("the reserve key spoke: [stop] taken in", 0);
        let _ = state.save(&state_file);
    }

    let installed = version_of_file(&places.program).unwrap_or_default();

    let chosen = match decide(&installed, &document, signer, &state, &places.files) {
        Ok(chosen) => chosen.clone(),
        Err(skip) => {
            journal.line(skip.journal_name(), 0);
            return code::DONE;
        }
    };

    let setup = places.update_dir.join(DOWNLOADED_SETUP);

    if let Err(refused) = download(&chosen, &setup, &places.update_dir.join(DOWNLOAD_PART)) {
        journal.line(refused.journal_name(), 0);
        return code::DONE;
    }

    journal.line("download verified", 0);

    let installed_copy = places.update_dir.join(INSTALLED_SETUP);
    let rollback_copy = installed_copy.exists()
        && replace_file(&installed_copy, &places.update_dir.join(ROLLBACK_SETUP));

    state.pending = Some(Pending {
        version: chosen.version.clone(),
        from: installed,
        sessions: sessions_running(&places.program),
        rollback_copy,
        rolling_back: false,
        since: seconds_now(),
    });

    if !state.save(&state_file) {
        journal.line("state not written", 0);
        return code::STATE;
    }

    // ⛔ Решение 189.33 (C4 of 189.32, measured on Test_VM): the Restart Manager of a Setup that runs
    // as the system account does not close a program in a session of its own («Session Mismatch»),
    // the file stays busy and the installation aborts. So the program is closed HERE, in every
    // session it runs in, before the installer starts — politely and, failing that, for good.
    let sessions = state
        .pending
        .as_ref()
        .map(|p| p.sessions.clone())
        .unwrap_or_default();
    let closed = sessions
        .iter()
        .all(|&session| close_program_in(session, &places.program, journal));

    if !closed {
        journal.line("the program could not be closed: nothing installed", 0);

        if let Some(pending) = state.pending.take() {
            let _ = raise_and_check(&pending.sessions, &places.program, journal);
            put_rollback_copy_back(places, &pending);
        }

        let _ = state.save(&state_file);
        return code::DONE;
    }

    if !start_setup(&setup, &places.update_dir.join(SETUP_LOG)) {
        journal.line("setup not started", 0);

        if let Some(pending) = state.pending.take() {
            put_rollback_copy_back(places, &pending);
        }

        let _ = state.save(&state_file);
        return code::DONE;
    }

    journal.line("setup started", 0);
    code::DONE
}

/// Phase two — §0 п. 18.
fn phase_two_of(places: &Places, journal: &Journal) -> u8 {
    if !wait_for_setup_to_end(SETUP_WAIT_SECONDS) {
        journal.line("phase two: the installer did not end", 0);
    }

    let state_file = places.update_dir.join(STATE_FILE);

    let Some(mut state) = State::load(&state_file) else {
        journal.line("state unreadable", 0);
        return code::STATE;
    };

    let Some(mut pending) = state.pending.clone() else {
        journal.line("phase two: nothing in flight", 0);
        return code::DONE;
    };

    let installed = version_of_file(&places.program).unwrap_or_default();
    let _ = std::fs::remove_file(places.update_dir.join(DOWNLOADED_SETUP));

    if pending.rolling_back {
        let _ = raise_and_check(&pending.sessions, &places.program, journal);

        if installed == pending.from {
            journal.line("rolled back", 0);
            let _ = std::fs::remove_file(places.update_dir.join(ROLLBACK_SETUP));
        } else {
            journal.line("rollback did not restore the version", 0);
        }

        state.pending = None;
        let _ = state.save(&state_file);
        return code::DONE;
    }

    if installed != pending.version {
        journal.line("setup did not install", 0);
        let _ = raise_and_check(&pending.sessions, &places.program, journal);
        put_rollback_copy_back(places, &pending);
        state.pending = None;
        let _ = state.save(&state_file);
        return code::DONE;
    }

    if raise_and_check(&pending.sessions, &places.program, journal) {
        journal.line("updated", 0);
        let _ = std::fs::remove_file(places.update_dir.join(ROLLBACK_SETUP));
        state.pending = None;
        let _ = state.save(&state_file);
        return code::DONE;
    }

    // The new version did not start: the old one goes back and the new one is never tried
    // here again (§0 п. 18).
    state.mark_failed(&pending.version);

    let rollback = places.update_dir.join(ROLLBACK_SETUP);

    if !pending.rollback_copy || !rollback.exists() {
        journal.line("did not start, and there is no copy to roll back to", 0);
        state.pending = None;
        let _ = state.save(&state_file);
        return code::DONE;
    }

    pending.rolling_back = true;
    pending.since = seconds_now();
    state.pending = Some(pending);

    if !state.save(&state_file) {
        journal.line("state not written", 0);
        return code::STATE;
    }

    if start_setup(&rollback, &places.update_dir.join(SETUP_LOG)) {
        journal.line("did not start: rollback started", 0);
    } else {
        journal.line("did not start: rollback not started", 0);
        state.pending = None;
        let _ = state.save(&state_file);
    }

    code::DONE
}

/// Puts [`ROLLBACK_SETUP`] back under [`INSTALLED_SETUP`] when no installer wrote a copy of its
/// own — an installation that did not happen leaves the copy of the version still installed.
fn put_rollback_copy_back(places: &Places, pending: &Pending) {
    let installed = places.update_dir.join(INSTALLED_SETUP);
    let rollback = places.update_dir.join(ROLLBACK_SETUP);

    if pending.rollback_copy && rollback.exists() && !installed.exists() {
        let _ = replace_file(&rollback, &installed);
    }
}

// =========================================================================================
// 10. The version of a file
// =========================================================================================

/// The version of an executable, `major.minor.patch`, out of its version resource — what the
/// updater compares the feed with (§0 п. 15). `None` for a file without one.
pub fn version_of_file(path: &Path) -> Option<String> {
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW};
    use windows::core::PCWSTR;

    let name = wide_path(path);

    // SAFETY: a NUL-terminated name of this frame; no handle is asked for.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(name.as_ptr()), None) };

    if size == 0 || size > 1024 * 1024 {
        return None;
    }

    let mut buffer = vec![0u8; usize::try_from(size).ok()?];

    // SAFETY: `buffer` holds `size` bytes and lives for this frame.
    unsafe {
        GetFileVersionInfoW(
            PCWSTR(name.as_ptr()),
            None,
            size,
            buffer.as_mut_ptr().cast(),
        )
    }
    .ok()?;

    let (major, minor, patch, _) = crate::tray::parse_fixed_file_version(&buffer)?;

    Some(format!("{major}.{minor}.{patch}"))
}
