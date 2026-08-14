//! The SEC-04a debug control channel: a named pipe reporting program state.
//!
//! The single documented exception to SEC-04, and the reason it exists is that SEC-06 is
//! otherwise not provable by automated means: the contents of a password field are not
//! reachable from outside by the design of Windows.
//!
//! Compiled only under the `testing` feature, which is absent from the Release
//! configuration. Metadata only, read only, and restricted to the owner of the current
//! session.
//!
//! Requirements this module covers: SEC-04a, for the acceptance bench of section 11.5 and
//! acceptance criterion 8 of section 13 of SPEC — tasks **T-03-4** and **T-03-4-2**.
//! Task **T-05-2a** added the `cycle_position` key of FR-32 and FR-33, which is the position
//! counter task T-05-2 put in [`crate::buffer::Recorder`] made visible from outside the
//! process; the choice of the target layout is section 4.4's and is not this module's.
//! Implemented by backlog tasks: T-03-4 (name, descriptor, snapshot), T-03-4-2 (the server),
//! T-05-2a (`cycle_position`).
//!
//! # The four conditions of SEC-04a, and where each of them is met
//!
//! 1. **Compiled only under `testing`.** The whole module is behind
//!    `#[cfg(feature = "testing")] pub mod control;` in `src\lib.rs`, the feature is absent
//!    from the Release configuration, and the two mirrors this module reads in
//!    [`crate::buffer`] — the ring's length and the cycle position — carry the same gate.
//!    Nothing here has a counterpart outside it.
//! 2. **Metadata only.** [`Snapshot`] is the whole of what leaves this module, and every
//!    field of it is a count, a duration or a published configuration value. There is no
//!    field for a stroke, a key code, a scan code or a character, and none may ever be added:
//!    SEC-01 and SEC-07 forbid it, and `buffer::Stroke` has neither `Debug` nor `Display`, so
//!    the content is not even expressible here by mistake. `buffer_len` is a number, and so is
//!    `cycle_position` — a step along the cycle of section 4.4, not a layout and not a
//!    character — which is what condition 2 of SEC-04a names as the one thing allowed out.
//! 3. **Owner of the current session only.** [`OwnerOnly`] builds an explicit security
//!    descriptor whose DACL holds exactly one allow entry, on the SID taken from this
//!    process's own token. A `NULL` DACL means "everyone" and is the opposite of the
//!    condition; there is none here, and [`OwnerOnly::dacl_facts`] is the assertion of it.
//! 4. **Read only.** Nothing in this module reads from a pipe handle, and the access mask of
//!    the one allow entry is [`CHANNEL_RIGHTS`] — `GENERIC_READ` and nothing else, so a
//!    client cannot obtain a writable handle in the first place.
//!
//! # Two sinks, one source
//!
//! Decision **Р-28**: the counters and the start-up marks live here and here only, and both
//! sinks read them through [`snapshot`] — the live channel, and the file the run writes at
//! shutdown when `LANGSW_TESTING_REPORT` names one (`app::acceptance`). A channel needs a
//! live process and a client; the file also serves the run that merely timed out under FR-97.
//! Two sinks over one source is cheaper than two sources.
//!
//! # Keys that are deliberately absent
//!
//! [`render`] emits a key only when the value behind it exists. One key of SEC-04a is
//! therefore **missing from the output rather than published as zero** — a stand that reports
//! a fabricated zero would count it as the truth:
//!
//! * `password_field` — SEC-06, FR-70 to FR-73, position 14 of the matrix of section 11.3.
//!   Task **T-06-1**. This is the key SEC-04a exists for at all.
//!
//! It is one line in [`Snapshot`] and one line in [`render`] when its owning task arrives;
//! neither the format nor the server has to change to accept it.
//!
//! `cycle_position` was the second such key until task **T-05-2a**, which is the task this
//! paragraph is being edited by. The counter itself is task T-05-2's and is not touched here:
//! what T-05-2a added is the mirror that makes it observable, [`note_cycle_position`], built
//! on the pattern [`note_buffer_len`] set — see [`CYCLE_POSITION`].
//!
//! # The server — task T-03-4-2
//!
//! [`start`] creates one instance of the named pipe and hands it to a thread of its own,
//! which spends its life inside `ConnectNamedPipe`; every client that arrives is written the
//! rendering of one [`snapshot`] and is then disconnected. [`Channel::stop`] brings that
//! thread down. The three questions this design had to answer are argued where the code is:
//! the thread at [`start`], the wake-up at [`Channel::stop`], and the write at [`publish`].

use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::mem::ManuallyDrop;
use std::os::windows::io::FromRawHandle;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_DATA, ERROR_PIPE_CONNECTED,
    ERROR_PIPE_NOT_CONNECTED, GENERIC_READ, HANDLE,
};
use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_REVISION, ACL_SIZE_INFORMATION, AclSizeInformation,
    AddAccessAllowedAce, EqualSid, GetAce, GetAclInformation, GetLengthSid,
    GetSecurityDescriptorDacl, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
    SetSecurityDescriptorDacl, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::Storage::FileSystem::{
    FILE_FLAGS_AND_ATTRIBUTES, FlushFileBuffers, PIPE_ACCESS_OUTBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, NAMED_PIPE_MODE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcessToken};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult};

use crate::settings::ReplacementMethod;

// ---------------------------------------------------------------------------------------
// The name of the channel
// ---------------------------------------------------------------------------------------

/// The one constant the name of the channel is built from — acceptance criterion 8 of
/// section 13 of SPEC searches the shipped binary for exactly this string.
///
/// One constant and not several, and recognisable rather than clever: the criterion is a
/// string search over a binary, and a name assembled from fragments would pass it by
/// accident rather than by construction.
///
/// The full name carries the session identifier as well — see [`pipe_name`] — because the
/// named-pipe namespace is machine-wide while the program is per session: two sessions of one
/// computer run two copies, and a fixed name would have the second one fail to create its
/// channel and read the first one's numbers instead.
pub const PIPE_NAME_PREFIX: &str = r"\\.\pipe\Lang_Switcher.control.";

/// The full name of this process's channel: [`PIPE_NAME_PREFIX`] followed by the decimal
/// identifier of the session this process runs in.
///
/// The session comes from `ProcessIdToSessionId` on this process rather than from
/// `WTSGetActiveConsoleSessionId`: the question is which session *this copy* belongs to, and
/// on a machine with a switched-away console those two answers differ.
///
/// A session that cannot be determined is reported as `0`, which is the session the name
/// would have had on a single-session machine anyway. A channel is a debugging aid, and
/// refusing to name it because the session could not be read would turn a diagnostic into an
/// outage.
pub fn pipe_name() -> String {
    format!("{PIPE_NAME_PREFIX}{}", session_id())
}

/// The session this process runs in, or `0` when Windows will not say.
fn session_id() -> u32 {
    let mut session = 0u32;

    // SAFETY: `GetCurrentProcessId` returns this process's identifier and takes no arguments.
    // `ProcessIdToSessionId` writes one `u32` through the pointer, which points at a live
    // local of exactly that type; the call dereferences nothing else. NFR-13: the result is
    // examined below rather than discarded.
    let read = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &raw mut session) };

    match read {
        Ok(()) => session,
        Err(_error) => 0,
    }
}

// ---------------------------------------------------------------------------------------
// The security descriptor — condition 3 of SEC-04a
// ---------------------------------------------------------------------------------------

/// Rights the single allow entry grants: read, and nothing else.
///
/// `GENERIC_READ` maps, on a pipe, onto `FILE_READ_DATA | FILE_READ_ATTRIBUTES |
/// FILE_READ_EA | READ_CONTROL | SYNCHRONIZE`. No write right is granted to anybody at all,
/// which is condition 4 of SEC-04a stated in the access mask rather than merely in the
/// server's behaviour: a client cannot obtain a writable handle to ask with.
pub const CHANNEL_RIGHTS: u32 = GENERIC_READ.0;

/// `SECURITY_DESCRIPTOR_REVISION` of `winnt.h`.
///
/// Spelled out here because `windows-rs` generates what the Windows metadata carries and this
/// one is a preprocessor constant, which the metadata does not.
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

/// `ACCESS_ALLOWED_ACE_TYPE` of `winnt.h` — the same case as
/// [`SECURITY_DESCRIPTOR_REVISION`].
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;

/// A security descriptor whose DACL admits the owner of this process and nobody else —
/// **condition 3 of SEC-04a**.
///
/// # Why it is built by hand
///
/// The alternative is to pass `NULL` for the security attributes, or a descriptor with a
/// `NULL` DACL. Those are not the same thing and neither is acceptable here:
///
/// * `NULL` **attributes** give the object the caller's default descriptor, which is derived
///   from the token's default DACL — right on most machines and a guess on any of them.
/// * A `NULL` **DACL** grants everyone full access. It is the exact opposite of condition 3,
///   it is the classic way of getting this wrong, and it is the one outcome this type exists
///   to make impossible.
///
/// # Ownership
///
/// The descriptor points at the ACL and the ACL carries its own copy of the SID
/// (`AddAccessAllowedAce` copies it), so the three allocations below must outlive every
/// handle created with them. Each is behind its own heap block, whose address survives this
/// value being moved.
pub struct OwnerOnly {
    /// The descriptor. Boxed so that [`OwnerOnly::attributes`] can hand out its address and
    /// have that address stay valid when this value moves.
    descriptor: Box<SECURITY_DESCRIPTOR>,
    /// The DACL: an `ACL` header followed by exactly one `ACCESS_ALLOWED_ACE`. A `u32`
    /// element type and not `u8`, because `ACL` requires `DWORD` alignment and a byte vector
    /// promises none.
    dacl: Vec<u32>,
    /// The `TOKEN_USER` block `GetTokenInformation` filled in; the SID sits inside it. A
    /// `u64` element type for the alignment `TOKEN_USER` needs.
    token_user: Vec<u64>,
}

impl OwnerOnly {
    /// Builds the descriptor from this process's own token.
    ///
    /// The steps are the ones SEC-04a's condition 3 comes down to, in order: read the user
    /// SID out of the process token, size and initialise an ACL, put one allow entry on that
    /// SID into it, initialise a descriptor, and attach the ACL to it as the DACL.
    pub fn for_this_process() -> WinResult<Self> {
        let token_user = owner_token_user()?;
        let sid = sid_of(&token_user);

        // The ACE carries the SID inline, starting at the `SidStart` field: the size of the
        // entry is therefore the fixed part minus that placeholder `u32` plus the real SID.
        //
        // SAFETY: `sid` points into `token_user`, which `GetTokenInformation` filled with a
        // valid `TOKEN_USER` whose `Sid` addresses a valid SID inside the same block; the
        // block is alive for the whole of this function. `GetLengthSid` only reads it.
        let sid_len = unsafe { GetLengthSid(sid) } as usize;

        let ace_bytes = size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>() + sid_len;
        let acl_bytes = size_of::<ACL>() + ace_bytes;

        // `InitializeAcl` wants a length that is a multiple of four, and the vector is `u32`
        // for the alignment, so rounding up and counting in words are the same operation.
        let mut dacl = vec![0u32; acl_bytes.div_ceil(size_of::<u32>())];
        let acl_len = u32::try_from(dacl.len() * size_of::<u32>()).unwrap_or(u32::MAX);
        let acl: *mut ACL = dacl.as_mut_ptr().cast();

        // SAFETY: `acl` is the start of a `dacl.len() * 4`-byte heap block owned by this
        // frame, aligned to four bytes by the element type, and `acl_len` is exactly that
        // block's size, so the call writes only inside it. NFR-13: checked with `?`.
        unsafe { InitializeAcl(acl, acl_len, ACL_REVISION) }?;

        // SAFETY: `acl` addresses the ACL initialised on the line above, and the block was
        // sized for its header plus one entry over `sid`, so the entry fits. `sid` is the
        // valid SID described above; the call copies it into the entry and keeps no pointer
        // to it. NFR-13: checked with `?`.
        unsafe { AddAccessAllowedAce(acl, ACL_REVISION, CHANNEL_RIGHTS, sid) }?;

        let mut descriptor = Box::new(SECURITY_DESCRIPTOR::default());
        let raw = PSECURITY_DESCRIPTOR((&raw mut *descriptor).cast());

        // SAFETY: `raw` addresses a live, owned, correctly aligned `SECURITY_DESCRIPTOR`, and
        // the call writes the absolute form of a descriptor into it — which is what the
        // structure is. NFR-13: checked with `?`.
        unsafe { InitializeSecurityDescriptor(raw, SECURITY_DESCRIPTOR_REVISION) }?;

        // SAFETY: `raw` is the descriptor initialised above and `acl` the ACL filled in above;
        // both outlive this call, and both outlive the returned value, which owns them.
        // `bdaclpresent` is `true` and the ACL is `Some`: **this is the line that makes the
        // DACL present and non-`NULL`**, and passing `None` here is precisely the "everyone"
        // descriptor SEC-04a forbids. `bdacldefaulted` is `false` — the DACL is this
        // program's own decision and not one inherited from the token's default.
        // NFR-13: checked with `?`.
        unsafe { SetSecurityDescriptorDacl(raw, true, Some(acl), false) }?;

        Ok(Self {
            descriptor,
            dacl,
            token_user,
        })
    }

    /// The `SECURITY_ATTRIBUTES` to hand to an object-creating call.
    ///
    /// Borrows rather than copies: the returned value points into `self`, so `self` has to
    /// outlive every object created with it, and the lifetime says so.
    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
            lpSecurityDescriptor: (&raw const *self.descriptor).cast_mut().cast(),
            // The channel is never inherited by a child process. This program starts none,
            // and a handle that could be inherited is a handle that could leave the process.
            bInheritHandle: false.into(),
        }
    }

    /// What the DACL actually contains — the evidence for condition 3 of SEC-04a.
    ///
    /// Reads the descriptor back through the Win32 accessors rather than through the fields
    /// this module wrote, so that the answer is the one the kernel would give.
    pub fn dacl_facts(&self) -> WinResult<DaclFacts> {
        let raw = PSECURITY_DESCRIPTOR((&raw const *self.descriptor).cast_mut().cast());
        let mut present = windows::core::BOOL(0);
        let mut acl: *mut ACL = core::ptr::null_mut();
        let mut defaulted = windows::core::BOOL(0);

        // SAFETY: `raw` addresses the descriptor this value owns and which was initialised in
        // `for_this_process`; the three out-pointers address live locals of the expected
        // types. The call only reads the descriptor. NFR-13: checked with `?`.
        unsafe {
            GetSecurityDescriptorDacl(raw, &raw mut present, &raw mut acl, &raw mut defaulted)
        }?;

        if !present.as_bool() || acl.is_null() {
            // Either of these is the `NULL` DACL — "access for everyone" — and there is
            // nothing further to count.
            return Ok(DaclFacts {
                present: present.as_bool(),
                null_dacl: true,
                own_acl: false,
                ace_count: 0,
                allow_aces_on_owner: 0,
                granted: 0,
            });
        }

        // The descriptor is supposed to point at the block this value owns and at nothing
        // else. Comparing the addresses is what turns "a DACL is attached" into "the DACL we
        // built is attached", and it is a pointer comparison, so nothing is dereferenced.
        let own_acl = acl.cast_const().cast::<u32>() == self.dacl.as_ptr();

        let mut sizes = ACL_SIZE_INFORMATION::default();

        // SAFETY: `acl` is the non-null ACL the accessor above returned, `sizes` is a live
        // local of the class asked for, and the length passed is its own size, so the call
        // writes only inside it. NFR-13: checked with `?`.
        unsafe {
            GetAclInformation(
                acl,
                (&raw mut sizes).cast(),
                u32::try_from(size_of::<ACL_SIZE_INFORMATION>()).unwrap_or(0),
                AclSizeInformation,
            )
        }?;

        let owner = sid_of(&self.token_user);
        let mut allow_aces_on_owner = 0;
        let mut granted = 0;

        for index in 0..sizes.AceCount {
            let mut entry: *mut core::ffi::c_void = core::ptr::null_mut();

            // SAFETY: `acl` is the ACL above and `index` is below the entry count it
            // reported, so the entry exists; `entry` is a live local the call writes a
            // pointer into. NFR-13: checked with `?`.
            unsafe { GetAce(acl, index, &raw mut entry) }?;

            if entry.is_null() {
                continue;
            }

            // SAFETY: `entry` is the non-null pointer `GetAce` returned; it addresses an ACE
            // inside the ACL this value owns, and every ACE begins with an `ACE_HEADER`.
            let header = unsafe { *entry.cast::<ACE_HEADER>() };

            if header.AceType != ACCESS_ALLOWED_ACE_TYPE {
                continue;
            }

            // SAFETY: the header above says the entry is an `ACCESS_ALLOWED_ACE`, so the
            // wider read is of the type actually stored there. The SID starts at the
            // `SidStart` field and runs on past it, which is why its address is taken rather
            // than its value read.
            let (mask, sid) = unsafe {
                let ace = entry.cast::<ACCESS_ALLOWED_ACE>();
                (
                    (*ace).Mask,
                    PSID((&raw const (*ace).SidStart).cast_mut().cast()),
                )
            };

            // SAFETY: `sid` addresses the SID copied into this entry and `owner` the SID
            // inside the token block this value owns; both are valid SIDs and both outlive
            // the call, which only reads them.
            if unsafe { EqualSid(sid, owner) }.is_ok() {
                allow_aces_on_owner += 1;
                granted |= mask;
            }
        }

        Ok(DaclFacts {
            present: true,
            null_dacl: false,
            own_acl,
            ace_count: sizes.AceCount,
            allow_aces_on_owner,
            granted,
        })
    }
}

/// What [`OwnerOnly::dacl_facts`] found. Condition 3 of SEC-04a holds when `present` is true,
/// `null_dacl` is false, and `ace_count` and `allow_aces_on_owner` are both exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaclFacts {
    /// Whether the descriptor says it has a DACL at all.
    pub present: bool,
    /// Whether that DACL is `NULL` — access for everyone, and the thing SEC-04a forbids.
    pub null_dacl: bool,
    /// Whether the attached DACL is the block this value built, by address.
    pub own_acl: bool,
    /// Entries in the DACL, of every kind.
    pub ace_count: u32,
    /// Allow entries whose SID is the owner of this process.
    pub allow_aces_on_owner: u32,
    /// Rights those entries grant, or-ed together.
    pub granted: u32,
}

/// Reads the `TOKEN_USER` block out of this process's own token.
fn owner_token_user() -> WinResult<Vec<u64>> {
    let mut raw = HANDLE::default();

    // SAFETY: `GetCurrentProcess` returns the process pseudo-handle, a constant naming the
    // calling process. `TOKEN_QUERY` is the least access that answers the question below.
    // The out-pointer addresses a live local of the expected type. NFR-13: checked with `?`.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut raw) }?;

    let token = OwnedToken(raw);
    let mut needed = 0u32;

    // SAFETY: `token.0` is the handle just opened, `TokenUser` is the class being asked for,
    // and a `None` buffer of length zero is the documented way of asking how large the answer
    // is. `needed` is a live local the call writes the size into.
    let sized = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &raw mut needed) };

    // NFR-13: the result is examined. The expected outcome is the failure that reports the
    // size; a success would mean the class needs no bytes, which cannot happen for this one
    // but is not an error either, and anything else is a real failure and propagates.
    match sized {
        Ok(()) => {}
        Err(error) if error.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => {}
        Err(error) => return Err(error),
    }

    let words = (needed as usize).div_ceil(size_of::<u64>()).max(1);
    let mut block = vec![0u64; words];
    let length = u32::try_from(block.len() * size_of::<u64>()).unwrap_or(u32::MAX);

    // SAFETY: the buffer is a live heap block of `length` bytes owned by this frame, aligned
    // to eight by the element type, which is what `TOKEN_USER` and the SID inside it need;
    // `length` is at least the size the call asked for above. NFR-13: checked with `?`.
    unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(block.as_mut_ptr().cast()),
            length,
            &raw mut needed,
        )
    }?;

    Ok(block)
}

/// The SID inside a block [`owner_token_user`] filled in.
fn sid_of(block: &[u64]) -> PSID {
    // SAFETY: the block was filled by `GetTokenInformation(TokenUser)`, which writes a
    // `TOKEN_USER` at its start, and the block is aligned for it by its element type. The
    // read copies two pointer-sized fields out of it and dereferences neither.
    unsafe { (*block.as_ptr().cast::<TOKEN_USER>()).User.Sid }
}

/// A process token handle that closes itself.
struct OwnedToken(HANDLE);

impl Drop for OwnedToken {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the handle `OpenProcessToken` returned into this value and has
        // not been closed anywhere else — nothing copies it out. NFR-13: the result is
        // examined; a token that will not close is not worth ending the process over, and the
        // process is the only thing that could leak.
        if let Err(error) = unsafe { CloseHandle(self.0) } {
            crate::app::report_non_critical("CloseHandle(token)", &error);
        }
    }
}

// ---------------------------------------------------------------------------------------
// The server — task T-03-4-2
// ---------------------------------------------------------------------------------------

/// `dwOpenMode` of the pipe: the server writes, the client reads, and there is no other
/// direction.
///
/// **Condition 4 of SEC-04a in the shape of the object itself.** `PIPE_ACCESS_OUTBOUND` gives
/// the instance no inbound half at all, so the pipe this program creates is one a client
/// cannot send anything through — the read this module refuses to perform is a read that does
/// not exist to be performed.
///
/// **No `FILE_FLAG_OVERLAPPED`, deliberately.** The flag belongs to the handle and not to a
/// single operation: on an overlapped handle every operation is overlapped, writes included,
/// and `std`'s `File::write` calls `WriteFile` with a null `lpOverlapped`, which Microsoft
/// documents as able to "incorrectly report that the write operation is complete". The task
/// requires the payload to be written through `std::fs::File`, so the handle has to be a
/// synchronous one, and the way this thread is unblocked follows from that — see
/// [`Channel::stop`].
pub const PIPE_OPEN_MODE: FILE_FLAGS_AND_ATTRIBUTES = PIPE_ACCESS_OUTBOUND;

/// `dwPipeMode` of the pipe: a byte stream, blocking, and **local clients only**.
///
/// `PIPE_REJECT_REMOTE_CLIENTS` is the load-bearing bit. SEC-03 and NFR-11 require the total
/// absence of network activity, and a named pipe without this flag is reachable over SMB from
/// another machine: the channel would become the one network surface in a program that is
/// supposed to have none. `PIPE_TYPE_BYTE` and `PIPE_WAIT` are both zero, so this constant is
/// numerically the reject bit alone; they are spelled out anyway, because what a reader has to
/// be able to check is the intent, and a `0` would hide two decisions.
///
/// Assembled by hand rather than with `|` because `BitOr` is not a `const fn` on this type.
pub const PIPE_MODE: NAMED_PIPE_MODE =
    NAMED_PIPE_MODE(PIPE_TYPE_BYTE.0 | PIPE_WAIT.0 | PIPE_REJECT_REMOTE_CLIENTS.0);

/// `nMaxInstances`: exactly one.
///
/// One thread serves one client at a time and reuses a single instance for its whole life, so
/// one is the truth. It is also the tighter statement: `PIPE_UNLIMITED_INSTANCES` would
/// declare that further instances of this name may exist, and nothing about this channel wants
/// a second server behind the same name.
const PIPE_INSTANCES: u32 = 1;

/// Outbound buffer of the pipe, in bytes.
///
/// The payload is thirteen short `key=value` lines — some two hundred bytes — and a page is
/// comfortably more than the widest it could grow to when task T-06-1 adds its key. Sizing it
/// above the payload is what lets [`publish`] hand the bytes over without waiting for the
/// client to read them.
const PIPE_OUT_BUFFER_BYTES: u32 = 4096;

/// Inbound buffer of the pipe: none. **Condition 4 of SEC-04a.** There is nothing to read
/// from, so nothing can be sent to this process through the channel.
const PIPE_IN_BUFFER_BYTES: u32 = 0;

/// `nDefaultTimeOut` of the instance, in milliseconds — the value a client gets when it calls
/// `WaitNamedPipe` with `NMPWAIT_USE_DEFAULT_WAIT`. Nothing in this program waits on it.
const PIPE_DEFAULT_TIMEOUT_MS: u32 = 5_000;

/// Name of the channel thread, as the debugger and Process Explorer show it.
const THREAD_NAME: &str = "langsw-control";

/// How long [`Channel::stop`] waits for the channel thread before giving up on it.
///
/// Generous next to what the operation actually costs — one local connection and one loop
/// iteration, sub-millisecond — and still far below anything a person could notice on the way
/// out. Neither FR-96 nor FR-97 waits on it: see [`Channel::stop`].
const STOP_GRACE: Duration = Duration::from_millis(500);

/// How often [`Channel::stop`] retries the wake-up inside [`STOP_GRACE`].
const STOP_POLL: Duration = Duration::from_millis(5);

/// Asks the channel thread to leave its loop. Read at the top of every iteration and once
/// more the moment a client arrives.
static STOP: AtomicBool = AtomicBool::new(false);

/// A running channel — the value [`start`] returns and [`Channel::stop`] consumes.
///
/// Holding it keeps the security descriptor alive for as long as the pipe exists, which is
/// what [`OwnerOnly`] asks of its owner.
///
/// Dropping it instead of calling [`Channel::stop`] detaches the thread. That is not a leak
/// worth guarding against: a Rust process ends when `main` returns and takes every other
/// thread with it, which is also why the channel cannot hold up FR-96 or FR-97.
pub struct Channel {
    /// The channel thread. It owns the pipe handle and closes it on its way out.
    thread: JoinHandle<()>,
    /// Kept alive, not used: the descriptor the pipe was created with.
    _descriptor: OwnerOnly,
}

/// Creates the channel and starts the thread that serves it.
///
/// # Why a thread of its own — section 6.1
///
/// None of the three threads of section 6.1 can hold a blocking `ConnectNamedPipe`. The input
/// thread must never block, which is the whole of section 6.3 and of NFR-01 to NFR-05. A UI
/// thread stuck for a few seconds has its low-level hook removed by the system without a word
/// (section 6.1, FR-80). The watcher thread is a COM single-threaded apartment with `WinEvent`
/// subscriptions on it, and a blocked STA delivers no events, which would silently break two
/// rows of the FR-10 reset table.
///
/// **This is not a change to the threading model of section 6.1**, and the reason is the `cfg`
/// on the module rather than an argument about what a fourth thread costs. Section 6.1
/// describes the program that ships. This thread is created only under `testing`, the feature
/// is absent from the Release configuration (SEC-04a condition 1), and acceptance criterion 8
/// of section 13 checks the shipped binary for traces of it. The shipped program therefore has
/// exactly the three threads section 6.1 prescribes; the fourth exists only in a build that
/// section 6.1 does not describe and that no user is given. The fault injection of module
/// `hook` stands on the same ground.
///
/// # Failure
///
/// Returns the error rather than ending the process. The caller reports it and runs on: the
/// channel is a diagnostic, and a program that refused to start because its diagnostic did not
/// come up would have made the diagnostic the most dangerous part of itself.
pub fn start() -> WinResult<Channel> {
    // A previous channel in the same process — only a test does this — must not leave its stop
    // request behind for this one to trip over.
    STOP.store(false, Ordering::Release);

    let descriptor = OwnerOnly::for_this_process()?;
    let pipe = create_pipe(&descriptor)?;

    // `HANDLE` is a raw pointer and therefore not `Send`, so it crosses the thread boundary as
    // the integer it actually is and is rebuilt on the other side. The conversion is exact in
    // both directions, and this is the same device `app` uses for the window handles the hook
    // callback has to reach.
    let raw = pipe.0.0 as usize;

    let spawned = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || serve(HANDLE(raw as *mut c_void)));

    match spawned {
        Ok(thread) => {
            // The thread owns the pipe now and closes it when it ends; this frame must not.
            std::mem::forget(pipe);

            Ok(Channel {
                thread,
                _descriptor: descriptor,
            })
        }
        // `pipe` is dropped here, which closes the instance: a channel nobody serves would
        // accept a client and then never answer it.
        Err(error) => Err(WinError::from(error)),
    }
}

impl Channel {
    /// Stops the channel thread and waits, up to [`STOP_GRACE`], for it to end.
    ///
    /// Returns whether it ended. `false` means the thread is still inside a call — see the
    /// last paragraph — and has been detached.
    ///
    /// # How a blocking `ConnectNamedPipe` is unblocked
    ///
    /// By connecting to our own channel. The flag is set first, then this loop opens the pipe
    /// by name exactly as any client would; that completes the `ConnectNamedPipe` the thread is
    /// parked in, the thread re-reads the flag before writing anything, publishes nothing,
    /// disconnects and leaves. The connection is dropped unread — it is a doorbell, not a
    /// client.
    ///
    /// The alternative, overlapped I/O with an event, is ruled out by the requirement that the
    /// payload be written through `std::fs::File`: the flag that would make the connect
    /// cancellable also makes every write on that handle overlapped, and `std`'s write passes a
    /// null `OVERLAPPED`. See [`PIPE_OPEN_MODE`].
    ///
    /// **Why the wake-up cannot be missed.** The one objection to this route is that it turns
    /// shutdown into an operation that might not succeed. Three things remove it:
    ///
    /// * The instance is created **once**, in [`start`], and lives for the whole life of the
    ///   thread, so the name always resolves while there is a thread to wake.
    /// * Every outcome of the open is progress. It succeeds — the thread was parked and is now
    ///   awake. It fails with `ERROR_PIPE_BUSY` — the thread is not parked at all but serving
    ///   somebody, and it re-reads the flag as soon as it disconnects. It fails with
    ///   `ERROR_FILE_NOT_FOUND` — the thread has already closed the pipe and ended. So the
    ///   attempt is retried rather than judged, which is why its result is not inspected.
    /// * Nothing waits on this. FR-96 ends the process with `TerminateProcess` from inside the
    ///   hook callback and never reaches this code; FR-97 has already given the three threads
    ///   of section 6.1 their grace and their own termination before this runs. This call sits
    ///   after them, is bounded, and a thread that outlives it is detached and dies with the
    ///   process.
    pub fn stop(self) -> bool {
        STOP.store(true, Ordering::Release);

        let deadline = Instant::now() + STOP_GRACE;

        while !self.thread.is_finished() {
            // The doorbell. Read access, which is all this program's own DACL grants anybody
            // (see `CHANNEL_RIGHTS`), and the result is deliberately not examined: every one of
            // its outcomes means the thread is on its way out. Nothing is read from the
            // handle — the connection exists only to make the server's `ConnectNamedPipe`
            // return, and dropping it here closes it.
            let _ = OpenOptions::new().read(true).open(pipe_name());

            if self.thread.is_finished() || Instant::now() >= deadline {
                break;
            }

            thread::sleep(STOP_POLL);
        }

        if !self.thread.is_finished() {
            // Detached on purpose. `JoinHandle::join` has no timed form, and the one situation
            // that gets us here — a client that connected and then never read what it asked
            // for, leaving the thread inside `FlushFileBuffers` — is exactly the situation in
            // which joining would hang the shutdown it is supposed to be tidying up.
            return false;
        }

        // Finished, so this cannot block. The body returns `()` and panics nowhere; a panic
        // would already have gone through the hook of FR-98, and there is nothing here that
        // could act on the payload (SEC-01, SEC-07).
        self.thread.join().is_ok()
    }
}

/// Creates the one instance of the channel — **the call every flag of SEC-04a lands on**.
fn create_pipe(descriptor: &OwnerOnly) -> WinResult<OwnedPipe> {
    let name = pipe_name();

    // The project's idiom for a wide string argument: a NUL-terminated UTF-16 buffer owned by
    // this frame, which outlives the call below.
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    let attributes = descriptor.attributes();

    // SAFETY: `wide` is a NUL-terminated UTF-16 buffer alive for the whole call and only read
    // through. `attributes` is a live local whose `lpSecurityDescriptor` addresses the
    // descriptor `descriptor` owns, and `descriptor` outlives this call; the kernel copies the
    // descriptor into the object it creates, so nothing here has to outlive the return. Every
    // other argument is a value.
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wide.as_ptr()),
            PIPE_OPEN_MODE,
            PIPE_MODE,
            PIPE_INSTANCES,
            PIPE_OUT_BUFFER_BYTES,
            PIPE_IN_BUFFER_BYTES,
            PIPE_DEFAULT_TIMEOUT_MS,
            Some(&raw const attributes),
        )
    };

    // NFR-13. This one binding returns the raw handle rather than a `Result`, so the check is
    // written out: `INVALID_HANDLE_VALUE` is the documented failure and the reason is in the
    // thread's last error.
    if handle.is_invalid() {
        return Err(WinError::from_thread());
    }

    Ok(OwnedPipe(handle))
}

/// The body of the channel thread: wait for a client, answer it, disconnect, repeat.
///
/// One instance for the whole life of the thread — `DisconnectNamedPipe` returns it to the
/// listening state and `ConnectNamedPipe` parks on it again. Creating a fresh instance per
/// client would leave windows in which the name does not resolve, and [`Channel::stop`] rings
/// the doorbell by name.
fn serve(pipe: HANDLE) {
    // Takes ownership: the instance is closed when this function returns, however it returns.
    let pipe = OwnedPipe(pipe);

    while !stop_requested() {
        let connected = match await_client(pipe.0) {
            Ok(connected) => connected,
            Err(error) => {
                // NFR-13: examined and journalled, not escalated. There is no recovery from a
                // connect that fails for a reason other than the two below — the instance is
                // in a state this code did not put it in — so the channel ends and the program
                // carries on without it.
                crate::app::report_non_critical("ConnectNamedPipe", &error);
                break;
            }
        };

        // The flag is re-read here and not only at the top of the loop, because the client that
        // just arrived may be `Channel::stop`'s doorbell, and a doorbell must not be answered
        // with a snapshot.
        if connected && !stop_requested() {
            publish(pipe.0);
        }

        // Back to the listening state for the next client. Reached after every outcome,
        // including the ones `await_client` reports as "no client": a connect that returned
        // `ERROR_NO_DATA` still leaves the instance connected to a client that has gone.
        //
        // SAFETY: `pipe.0` is the instance this thread owns and has not closed. The call
        // forcibly ends whatever connection the instance has and dereferences nothing; ending
        // it without waiting for the client is the intent, because this program never waits on
        // one. NFR-13: the result is examined below.
        if let Err(error) = unsafe { DisconnectNamedPipe(pipe.0) } {
            // Nothing here can put the instance back into a state this loop understands, so the
            // channel ends and the program carries on without it.
            crate::app::report_non_critical("DisconnectNamedPipe", &error);
            break;
        }
    }
}

/// Parks until a client connects. `Ok(false)` means one came and went before it could be
/// answered.
fn await_client(pipe: HANDLE) -> WinResult<bool> {
    // SAFETY: `pipe` is the instance this thread owns and has not closed. `None` for the
    // overlapped structure is the documented way to ask for the synchronous form, which is the
    // form a handle created without `FILE_FLAG_OVERLAPPED` supports; the call blocks until a
    // client connects, which is the point of this thread existing. NFR-13: every outcome is
    // classified below.
    let connected = unsafe { ConnectNamedPipe(pipe, None) };

    match connected {
        Ok(()) => Ok(true),
        // A client that connected in the window between `DisconnectNamedPipe` and this call.
        // Documented, ordinary, and a connection all the same.
        Err(error) if error.code() == ERROR_PIPE_CONNECTED.to_hresult() => Ok(true),
        // A client connected and closed its end before this call ran. There is nobody to write
        // to; the instance still has to be disconnected.
        Err(error) if error.code() == ERROR_NO_DATA.to_hresult() => Ok(false),
        Err(error) => Err(error),
    }
}

/// Writes one snapshot to the connected client — **the whole of what leaves this process**.
///
/// # The write goes through `std`
///
/// `std::fs::File` over the pipe handle, as the task requires: the standard library owns the
/// `WriteFile` call, its retry loop and its error mapping, and this module gains no second way
/// of writing to a handle. The `File` is a borrowed view and not an owner —
/// `File::from_raw_handle` takes ownership and would close the instance this thread reuses, so
/// the value is wrapped in `ManuallyDrop` and its destructor never runs.
///
/// # SEC-01, SEC-07, condition 2 of SEC-04a
///
/// The bytes are [`render`] of one [`snapshot`], which is twelve counts and one word of
/// section 7. Nothing else can be sent from here: there is no other write in this module.
fn publish(pipe: HANDLE) {
    let payload = render(&snapshot());

    // SAFETY: `pipe` is the instance this thread owns; it was created with
    // `PIPE_ACCESS_OUTBOUND`, so the handle carries write access, and without
    // `FILE_FLAG_OVERLAPPED`, so the synchronous `WriteFile` `std` performs on it is the
    // correct form. `ManuallyDrop` is what keeps `File`'s destructor from closing a handle this
    // thread still owns and reuses for the next client.
    let mut file = ManuallyDrop::new(unsafe { File::from_raw_handle(pipe.0.cast()) });

    if let Err(error) = file.write_all(payload.as_bytes()) {
        // NFR-13: examined. A client that closed its end mid-read is the ordinary case and not
        // a fault of this program; anything else is journalled.
        let error = WinError::from(error);

        if !is_client_gone(&error) {
            crate::app::report_non_critical("write(control channel)", &error);
        }

        return;
    }

    // The payload is in the pipe's buffer, not yet in the client. `DisconnectNamedPipe`
    // discards whatever the client has not read, so without this the answer could be thrown
    // away before it arrived.
    //
    // SAFETY: `pipe` is the instance this thread owns, as above. The call waits for the client
    // to drain the buffer and dereferences nothing. NFR-13: the result is examined below.
    if let Err(error) = unsafe { FlushFileBuffers(pipe) }
        && !is_client_gone(&error)
    {
        // NFR-13: examined; same classification as the write above.
        crate::app::report_non_critical("FlushFileBuffers", &error);
    }
}

/// Whether a failure means "the client is no longer there", which is an outcome and not a
/// fault: a reader is free to close its end at any moment, and this program never waits on one.
fn is_client_gone(error: &WinError) -> bool {
    let code = error.code();

    code == ERROR_BROKEN_PIPE.to_hresult()
        || code == ERROR_NO_DATA.to_hresult()
        || code == ERROR_PIPE_NOT_CONNECTED.to_hresult()
}

/// Whether [`Channel::stop`] has asked the thread to leave.
fn stop_requested() -> bool {
    // Acquire against the Release store in `Channel::stop`, for the same reason
    // `app::shutdown_requested` pairs that way: a thread woken by the doorbell must observe
    // the request that was made before it rang.
    STOP.load(Ordering::Acquire)
}

/// A named-pipe instance that closes itself — the same shape as [`OwnedToken`], and for the
/// same reason: the handle has exactly one owner at every moment and no path out of a function
/// leaks it.
struct OwnedPipe(HANDLE);

impl Drop for OwnedPipe {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the handle `CreateNamedPipeW` returned into this value. It is
        // moved, never copied — `start` gives it to the thread with `mem::forget` rather than
        // duplicating it — so it is closed exactly once. NFR-13: the result is examined; a
        // handle that will not close is not worth ending the process over.
        if let Err(error) = unsafe { CloseHandle(self.0) } {
            crate::app::report_non_critical("CloseHandle(pipe)", &error);
        }
    }
}

// ---------------------------------------------------------------------------------------
// The numbers — the single source both sinks read
// ---------------------------------------------------------------------------------------

/// The instant [`crate::app::run`] was entered — the reference point of NFR-08.
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Microseconds from [`STARTED`] to the installed hook — the quantity NFR-08 bounds.
static HOOK_READY_US: AtomicU32 = AtomicU32::new(0);

/// Microseconds from [`STARTED`] to the published layout cache of FR-20.
static CACHE_READY_US: AtomicU32 = AtomicU32::new(0);

/// Completed builds of the layout cache: one at start-up, one per message of FR-21.
static CACHE_BUILDS: AtomicU32 = AtomicU32::new(0);

/// Strokes in the input thread's buffer **right now** — the mirror of section 6.3.
///
/// The buffer is a thread-local of the input thread and no other thread can reach it, so a
/// reader outside that thread needs the length published rather than fetched. Module
/// `buffer` writes this from the one place its ring length is written, so nothing can change
/// the buffer without the mirror following.
static BUFFER_LEN: AtomicUsize = AtomicUsize::new(0);

/// Where the strokes in the buffer stand in the cycle of section 4.4 **right now** — the
/// mirror of the counter FR-32 puts beside the ring.
///
/// The same construction as [`BUFFER_LEN`] and for the same reason: `Recorder::cycle` is a
/// field of a thread-local of the input thread (section 6.3), so a reader outside that thread
/// needs the value published rather than fetched. Module `buffer` writes this from the places
/// its counter is written, under the `testing` feature and nowhere else.
///
/// **SEC-01, SEC-07.** A step along the cycle — a number in `0..len` — and nothing else.
/// Which layout that step names is not here, and neither is a scan code, a character or a
/// stroke. A zero means "what is on the screen is what was typed", which is the state every
/// flush of FR-10 leaves behind (FR-34).
static CYCLE_POSITION: AtomicUsize = AtomicUsize::new(0);

/// [`BUFFER_LEN`] as it stood when the input thread left its message loop.
///
/// The file sink runs on the main thread after every thread has been joined, by which time
/// the input thread's buffer has been dropped and zeroed (SEC-02) and the live mirror
/// truthfully reads zero. This is the value that sink reports, and latching it is what keeps
/// the file's `buffer_len` meaning what it meant before task T-03-4.
static BUFFER_LEN_AT_EXIT: AtomicUsize = AtomicUsize::new(0);

/// Records the reference point. Idempotent; the first call wins.
pub fn note_start() {
    let _ = STARTED.set(Instant::now());
}

/// Records the moment `SetWindowsHookExW` returned — NFR-08.
pub fn note_hook_installed() {
    HOOK_READY_US.store(elapsed_us(), Ordering::Relaxed);
}

/// Records the moment the cache of FR-20 reached the buffer.
pub fn note_cache_ready() {
    CACHE_READY_US.store(elapsed_us(), Ordering::Relaxed);
}

/// Counts one completed build of the layout cache — FR-20 at start-up, FR-21 afterwards.
pub fn note_cache_built() {
    CACHE_BUILDS.fetch_add(1, Ordering::Relaxed);
}

/// Publishes the length of the typing buffer — called by module `buffer` from the one place
/// its ring length changes.
///
/// # NFR-01 to NFR-05
///
/// This sits on the callback path of the keyboard hook, so it is one relaxed atomic store and
/// nothing else: no allocation (NFR-03), no lock and no mutex (NFR-04), no I/O and nothing
/// formatted (NFR-05), and a constant handful of instructions (NFR-01, NFR-02). `Relaxed` is
/// what section 6.3 prescribes for exactly this kind of cross-thread flag: there is no other
/// datum whose visibility has to be ordered against it.
///
/// # SEC-01, SEC-07
///
/// A `usize`. Not the stroke, not the key, not the character — a count.
#[inline]
pub fn note_buffer_len(len: usize) {
    BUFFER_LEN.store(len, Ordering::Relaxed);
}

/// Publishes the position in the cycle of section 4.4 — called by module `buffer` from the
/// places its counter changes, and by nothing else.
///
/// # NFR-01 to NFR-05
///
/// Both callers sit on the input thread: one on the hotkey path (`Recorder::advance_cycle`),
/// one on the flush path every rule of FR-10 arrives at (`Recorder::clear_ring`, which is
/// reached from inside the hook callback). So this is one relaxed atomic store and nothing
/// else — no allocation (NFR-03), no lock and no mutex (NFR-04), no I/O and nothing formatted
/// (NFR-05), a constant handful of instructions (NFR-01, NFR-02). `Relaxed` for the reason
/// [`note_buffer_len`] gives: there is no other datum whose visibility has to be ordered
/// against it.
///
/// # SEC-01, SEC-07
///
/// A `usize`, and a small one: the position in the cycle. Not the layout it names, not the
/// stroke it was reached by, not the character either of them would produce.
#[inline]
pub fn note_cycle_position(position: usize) {
    CYCLE_POSITION.store(position, Ordering::Relaxed);
}

/// Latches the live length for the file sink — see [`BUFFER_LEN_AT_EXIT`].
///
/// Called on the input thread once its message loop has ended and while its buffer still
/// exists.
pub fn note_buffer_len_at_exit() {
    BUFFER_LEN_AT_EXIT.store(BUFFER_LEN.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Microseconds since [`note_start`], saturating rather than wrapping.
fn elapsed_us() -> u32 {
    STARTED
        .get()
        .map(|started| u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------
// The snapshot — what both sinks publish
// ---------------------------------------------------------------------------------------

/// Everything the channel is allowed to report, taken in one pass.
///
/// **SEC-01, SEC-07, and condition 2 of SEC-04a.** Every field is a count, a duration or a
/// published configuration value. No key code, no scan code, no character, and nothing
/// derived from one — and nothing of that kind may ever be added, which is the whole point of
/// the snapshot being a named structure rather than a free-form string built at the call
/// site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Strokes in the input thread's buffer right now — section 6.3, and the one number
    /// condition 2 of SEC-04a allows out.
    pub buffer_len: usize,
    /// [`Snapshot::buffer_len`] as it stood when the input thread left its loop. The file
    /// sink reports this one; the channel reports the live value.
    pub buffer_len_at_exit: usize,
    /// Whether the `WH_KEYBOARD_LL` hook of FR-01 is installed.
    pub hook_installed: bool,
    /// Microseconds from start-up to the installed hook — NFR-08.
    pub hook_ready_us: u32,
    /// Microseconds from start-up to the published layout cache — FR-20.
    pub cache_ready_us: u32,
    /// Completed builds of the layout cache — FR-20, FR-21.
    pub cache_builds: u32,
    /// Failed builds answered by the hardwired table — FR-25.
    pub layout_cache_failures: u32,
    /// Hotkey presses handed from the callback to the input thread — FR-02.
    pub hotkey_handoffs: u32,
    /// `PostMessageW` calls from the callback that did not go through.
    pub post_failures: u32,
    /// `SendInput` calls whose return differed from the count handed to them — **FR-45**.
    pub send_mismatches: u32,
    /// Events lost across those calls — the size of the discrepancy, not its count.
    pub events_lost: u32,
    /// `[replacement] inter_event_delay_ms` as published — **FR-44**, section 7.
    pub inter_event_delay_ms: u32,
    /// `[replacement] method` as published — **FR-42**, section 7.
    pub replacement_method: ReplacementMethod,
    /// Where the buffer stands in the cycle of section 4.4 — **FR-32, FR-33**.
    ///
    /// `0` is "as typed", and it is what every flush of FR-10 leaves behind (FR-34). A number
    /// in `0..len`, never a layout and never a character: see [`CYCLE_POSITION`].
    pub cycle_position: usize,
    //
    // Deliberately absent until its owning task exists, so that the bench cannot mistake a
    // fabricated zero for an answer:
    //   password_field — SEC-06, FR-70..FR-73, position 14 of section 11.3. Task T-06-1.
}

/// Takes the numbers in one pass — **the single source both sinks read** (decision Р-28).
///
/// Not a consistent cut of anything: the values come from separate atomics and the program
/// keeps running while they are read. That is deliberate and it is enough, because every one
/// of them is a monotone count or a published setting, and nothing here is a pair that has to
/// agree.
pub fn snapshot() -> Snapshot {
    let (send_mismatches, events_lost) = crate::inject::send_mismatches();

    Snapshot {
        buffer_len: BUFFER_LEN.load(Ordering::Relaxed),
        buffer_len_at_exit: BUFFER_LEN_AT_EXIT.load(Ordering::Relaxed),
        hook_installed: crate::hook::is_installed(),
        hook_ready_us: HOOK_READY_US.load(Ordering::Relaxed),
        cache_ready_us: CACHE_READY_US.load(Ordering::Relaxed),
        cache_builds: CACHE_BUILDS.load(Ordering::Relaxed),
        layout_cache_failures: crate::app::layout_cache_failures(),
        hotkey_handoffs: crate::hook::hotkey_handoffs(),
        post_failures: crate::hook::post_failures(),
        send_mismatches,
        events_lost,
        inter_event_delay_ms: crate::inject::inter_event_delay_ms(),
        replacement_method: crate::inject::replacement_method(),
        cycle_position: CYCLE_POSITION.load(Ordering::Relaxed),
    }
}

/// The channel's payload: one `key=value` per line, ASCII, newline-terminated.
///
/// The order is the order of the table in task T-03-4. A reader that parses line by line does
/// not depend on it; a human diffing two runs does.
///
/// `cycle_position` is last rather than beside `buffer_len`, and that is a decision about the
/// diff: the twelve lines above stood in this order for two tasks, and appending leaves every
/// one of them where a human comparing two runs already expects it. Task **T-05-2a**.
///
/// `password_field` is **not** here. See the module documentation: it arrives with task
/// T-06-1, one line, and until then its absence is the honest answer.
pub fn render(state: &Snapshot) -> String {
    format!(
        "buffer_len={}\n\
         hook_installed={}\n\
         hook_ready_us={}\n\
         cache_ready_us={}\n\
         cache_builds={}\n\
         layout_cache_failures={}\n\
         hotkey_handoffs={}\n\
         post_failures={}\n\
         send_mismatches={}\n\
         events_lost={}\n\
         inter_event_delay_ms={}\n\
         replacement_method={}\n\
         cycle_position={}\n",
        state.buffer_len,
        u8::from(state.hook_installed),
        state.hook_ready_us,
        state.cache_ready_us,
        state.cache_builds,
        state.layout_cache_failures,
        state.hotkey_handoffs,
        state.post_failures,
        state.send_mismatches,
        state.events_lost,
        state.inter_event_delay_ms,
        method_name(state.replacement_method),
        state.cycle_position,
    )
}

/// `[replacement] method` as section 7 spells it — FR-42.
///
/// The two words of the configuration file and not the Rust spelling of the enum: the bench
/// compares this against what it wrote into `config.toml`.
pub const fn method_name(method: ReplacementMethod) -> &'static str {
    match method {
        ReplacementMethod::Backspace => "backspace",
        ReplacementMethod::Selection => "selection",
    }
}

/// The keys [`render`] emits, in the order it emits them.
///
/// Exported so that a check of condition 2 of SEC-04a can assert the set exactly rather than
/// merely look for what it expects: a key that appeared here without being listed would be a
/// key nobody reviewed.
pub const KEYS: [&str; 13] = [
    "buffer_len",
    "hook_installed",
    "hook_ready_us",
    "cache_ready_us",
    "cache_builds",
    "layout_cache_failures",
    "hotkey_handoffs",
    "post_failures",
    "send_mismatches",
    "events_lost",
    "inter_event_delay_ms",
    "replacement_method",
    "cycle_position",
];

/// Keys SEC-04a reserves and this build does not answer — see [`KEYS`] and the module
/// documentation.
///
/// One key, and it is task **T-06-1**'s. `cycle_position` was the other until task T-05-2a
/// published it; the list shrinks as the tasks arrive, and a key that is *reserved* is one no
/// build may answer with a fabricated zero.
pub const RESERVED_KEYS: [&str; 1] = ["password_field"];
