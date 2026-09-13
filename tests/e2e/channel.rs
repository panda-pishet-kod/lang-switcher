//! Client of the SEC-04a debug channel — requirement 4 of §11.5.
//!
//! The channel is how the bench sees things that leave no trace outside the program. Position
//! 14 is the case the requirement names: "the buffer must stay empty" is an assertion about
//! the *inside* of the product, and no amount of UI Automation on a password field can answer
//! it.
//!
//! # What is known about the channel, and what is asked of it
//!
//! Built by tasks T-03-4 and T-03-4-2. Name `\\.\pipe\Lang_Switcher.control.<session>`; the
//! bench does not spell it out but takes it from [`lang_switcher::control::pipe_name`], which
//! is the same function the server names itself with — the bench and the product run in one
//! session, so the identifier matches by construction rather than by agreement.
//!
//! Line-oriented `key=value`. The keys are listed in [`lang_switcher::control::KEYS`] — thirteen
//! when this client was written, fifteen today, and the count is deliberately **not** written
//! into the code below: [`Snapshot::present_keys`] reads the product's own list. One key is still
//! reserved and absent, `password_field`, which arrives with task T-06-1; `cycle_position` was
//! reserved with it until task T-05-2a published it. The position 14 stub is written to survive
//! an absent key, which is what the task requires of it and what [`Snapshot::get`] returning an
//! `Option` expresses.
//!
//! The server end is `PIPE_ACCESS_OUTBOUND`, so the client can only read. [`write_is_refused`]
//! confirms that from this side rather than taking the report's word for it.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::io::AsRawHandle;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use windows::Win32::Foundation::HANDLE;

use crate::wait;

/// One reading of the channel — every key the running product published.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    values: BTreeMap<String, String>,
    /// The bytes as they arrived, for the report.
    pub raw: String,
}

impl Snapshot {
    /// The value of a key, or `None` when the running product does not publish it yet.
    ///
    /// The `Option` is the whole point for position 14: `password_field` does not exist until
    /// T-06-1, and a stub that indexed instead of asking would panic the run rather than
    /// report a pending verdict.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Which of the documented keys are present, in the order the product lists them.
    pub fn present_keys(&self) -> Vec<&'static str> {
        lang_switcher::control::KEYS
            .iter()
            .copied()
            .filter(|key| self.values.contains_key(*key))
            .collect()
    }

    /// Which of the reserved keys have appeared — one is left, `password_field` of T-06-1.
    pub fn reserved_present(&self) -> Vec<&'static str> {
        lang_switcher::control::RESERVED_KEYS
            .iter()
            .copied()
            .filter(|key| self.values.contains_key(*key))
            .collect()
    }
}

/// `ERROR_BROKEN_PIPE` — the ordinary way a pipe reaches end of stream.
const ERROR_BROKEN_PIPE: i32 = 109;

/// `ERROR_PIPE_NOT_CONNECTED`.
///
/// ⚠ **This is the normal end of a snapshot on this channel, not a failure**, and finding that
/// out cost this task a round of debugging worth writing down.
///
/// The server writes the snapshot, calls `FlushFileBuffers` — which on a pipe server blocks
/// until the client has taken every byte — and then `DisconnectNamedPipe`. So by the time the
/// connection is torn down the data has already been delivered in full. What the client is
/// still doing at that moment is one more `ReadFile`, the one whose zero-byte return would mean
/// end of file; that read arrives at an instance which is no longer connected and answers 233
/// instead of 109.
///
/// `.NET`'s `PipeStream` folds both codes into "end of stream", which is why
/// `NamedPipeClientStream` reads this channel without noticing anything, and why the report of
/// T-03-4-2 records the channel as working — it does. `std::io::Read::read_to_string` does not
/// fold it: it surfaces 233 as an error and discards the buffer along with it, which is what
/// made the first version of this client see an empty channel that was in fact answering
/// correctly.
///
/// So the loop below keeps the bytes it has and stops on either code. This is a note about the
/// **client contract** of SEC-04a, not a defect of the product: nothing is lost, and a Rust
/// reader simply has to know it. Recorded in the report for whoever writes the checks of
/// T-06-1 against the same channel.
const ERROR_PIPE_NOT_CONNECTED: i32 = 233;

/// The process the channel is required to belong to — finding Н42, task T-41-9. Zero until the
/// bench has a product to name.
static EXPECTED_SERVER: AtomicU32 = AtomicU32::new(0);

/// Declares whose channel every later [`read`] must have reached — finding Н42, task T-41-9.
///
/// Called by the bench the moment it has started the product and knows its identifier. Until
/// then the check is off: the first reads of a run happen while the product is still coming up,
/// and there is nothing to compare against yet.
pub fn expect_server(pid: u32) {
    EXPECTED_SERVER.store(pid, Ordering::Relaxed);
}

/// Reads one snapshot from the channel.
///
/// A connection is one snapshot: the server writes, flushes and disconnects.
///
/// # ⭐ Whose server answered — finding Н42, task T-41-9
///
/// The client used to connect **by name** and read whatever came back. A name is not proof: a
/// program of this user that raises a server on this name **before** the product exists takes the
/// name for itself — the product's own `CreateNamedPipeW` then fails, the product runs on (a
/// channel is a diagnostic, NFR-13), and this function would read a snapshot a stranger composed.
/// A green run would be a conversation with a substitute, and a red one would be unexplainable.
///
/// So the kernel is asked who owns the far end, and the answer is compared with the product the
/// bench started. The check lives **here**, inside the one reader, rather than at the call sites:
/// a check a caller can forget is a check that will be forgotten.
pub fn read() -> std::io::Result<Snapshot> {
    let name = lang_switcher::control::pipe_name();

    // Read-only, which is all the server's `PIPE_ACCESS_OUTBOUND` end permits.
    let mut pipe = File::open(&name)?;

    // Finding Н42. Zero means the bench has not named a product yet — see [`expect_server`].
    let expected = EXPECTED_SERVER.load(Ordering::Relaxed);

    if expected != 0 {
        let handle = HANDLE(pipe.as_raw_handle().cast());
        let actual = lang_switcher::control::server_process_id(handle).map_err(|error| {
            std::io::Error::other(format!("GetNamedPipeServerProcessId: {error}"))
        })?;

        if actual != expected {
            return Err(std::io::Error::other(lang_switcher::control::wrong_server(
                expected, actual,
            )));
        }
    }

    let mut bytes = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => bytes.extend_from_slice(&chunk[..read]),
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(ERROR_BROKEN_PIPE) | Some(ERROR_PIPE_NOT_CONNECTED)
                ) =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }

    let raw = String::from_utf8_lossy(&bytes).into_owned();

    let mut values = BTreeMap::new();
    for line in raw.lines() {
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }

    Ok(Snapshot { values, raw })
}

/// Waits until the channel reports the hook installed — the product's readiness signal.
///
/// The program under test has no window, so requirement 1's "readiness through UI Automation"
/// has nothing to look at; the channel answering, and answering `hook_installed=1`, is the
/// equivalent and stronger fact, because it is published by the program itself rather than
/// inferred from a user interface. It is also the precondition every position depends on:
/// without the hook there is nothing to test.
pub fn await_hook(timeout: Duration) -> Option<Snapshot> {
    wait::until(timeout, || {
        read().ok().filter(|s| s.get("hook_installed") == Some("1"))
    })
}

/// Confirms from the client side that the channel refuses to be opened for writing.
///
/// Condition 4 of SEC-04a and the DACL of T-03-4 both say so; this asks the kernel. The value
/// of doing it here rather than believing the report is that a regression in the server's open
/// mode would otherwise be invisible until somebody wrote to the channel on purpose.
pub fn write_is_refused() -> Result<String, String> {
    let name = lang_switcher::control::pipe_name();

    match OpenOptions::new().write(true).open(&name) {
        Ok(_) => Err("канал открылся на запись — SEC-04a условие 4 нарушено".to_owned()),
        Err(error) => Ok(error.to_string()),
    }
}
