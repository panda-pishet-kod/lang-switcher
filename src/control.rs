//! The SEC-04a debug control channel: a named pipe reporting program state.
//!
//! The single documented exception to SEC-04, and the reason it exists is that SEC-06 is
//! otherwise not provable by automated means: the contents of a password field are not
//! reachable from outside by the design of Windows.
//!
//! Compiled only under the `testing` feature, which is absent from the Release
//! configuration. Metadata only, read only, and reachable by **the owner of this process's
//! token**; the **session** is kept apart by the name of the channel, not by the rights on it
//! (finding Т13, task T-41-9 — see condition 3 below).
//!
//! Requirements this module covers: SEC-04a, for the acceptance bench of section 11.5 and
//! acceptance criterion 8 of section 13 of SPEC — tasks **T-03-4** and **T-03-4-2**.
//! Task **T-05-2a** added the `cycle_position` key of FR-32 and FR-33, which is the position
//! counter task T-05-2 put in [`crate::buffer::Recorder`] made visible from outside the
//! process; the choice of the target layout is section 4.4's and is not this module's.
//! Task **T-06-1a** added `focus_changes` and `password_probes`, the two counts that say whether
//! the verdict behind `password_field` is about the window in front of the user right now — see
//! [`Snapshot::focus_changes`].
//! Implemented by backlog tasks: T-03-4 (name, descriptor, snapshot), T-03-4-2 (the server),
//! T-05-2a (`cycle_position`), T-06-1a (`focus_changes`, `password_probes`).
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
//! 3. **The owner of the token, and the session by the name.** ⭐ **Finding Т13, task T-41-9:
//!    this condition used to read «owner of the current session only», and that phrase put two
//!    different mechanisms under one word.** The behaviour was right and the sentence was not,
//!    so the sentence is written out in two halves:
//!
//!    * **access** is decided by the **owner**: [`OwnerOnly`] builds an explicit security
//!      descriptor whose DACL holds exactly one allow entry, on the SID taken from this
//!      process's own token. A `NULL` DACL means "everyone" and is the opposite of the
//!      condition; there is none here, and [`OwnerOnly::dacl_facts`] is the assertion of it.
//!      Nothing in the descriptor mentions a session, and nothing in it could.
//!    * **the session** is kept apart by the **name**: [`pipe_name`] carries the identifier
//!      `ProcessIdToSessionId` gives for this process, because the named-pipe namespace is
//!      machine-wide while the program is per session. Two sessions of one user therefore never
//!      meet on this channel even though the rights of both would admit either.
//!
//!    Task T-41-9 deliberately did **not** add a session term to the descriptor (variant 2 of
//!    the finding): hours of code for a build that does not ship, to say in a second place what
//!    the name already says.
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
//! # Keys that were deliberately absent
//!
//! [`render`] emits a key only when the value behind it exists, so a key whose owning task had
//! not arrived was **missing from the output rather than published as zero** — a bench that
//! reads a fabricated zero would count it as the truth.
//!
//! Both such keys are published now. `cycle_position` arrived with task **T-05-2a**, built on
//! the pattern [`note_buffer_len`] set — see [`CYCLE_POSITION`]. `password_field` — SEC-06,
//! FR-70 to FR-73, position 14 of the matrix of section 11.3 — arrived with task **T-06-1**,
//! and it is the key SEC-04a exists for at all. It cost exactly what task T-03-4 promised it
//! would: one field of [`Snapshot`], one line of [`render`], one entry of [`KEYS`], and no
//! change to the format or to the server.
//!
//! [`RESERVED_KEYS`] is therefore **empty**, and task **T-06-3** emptied it — the one edit task
//! T-06-1 could not make, because `tests\control.rs` compared the constant against a
//! one-element array with `assert_eq!` and that file was outside its scope, so shortening the
//! constant was a compilation error rather than a failing test. The two edits were one change
//! and were made together.
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
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
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
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAGS_AND_ATTRIBUTES, FlushFileBuffers,
    PIPE_ACCESS_OUTBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
    NAMED_PIPE_MODE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
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
// Whose server is on the other end — finding Н42, task T-41-9
// ---------------------------------------------------------------------------------------

/// The process that created the server end of a connected channel — finding Н42, task T-41-9.
///
/// # Why the bench has to ask
///
/// ⭐ **This is the larger half of the repair, and [`PIPE_OPEN_MODE`] is the smaller one.** The
/// flag there stops a second server joining **our** name; it cannot help when the other server
/// was raised **first**, before this process existed. The client then connects by name, is
/// answered, and has no way of telling a real snapshot from one a stranger composed: a green run
/// would be a conversation with a substitute, and a red one would be unexplainable.
///
/// Asking costs one call and closes it. The kernel knows which process owns the server end, and
/// a name proves nothing the kernel cannot be asked about directly.
///
/// `handle` must be a handle to the **client** end of a connected named pipe — what
/// `File::open` on the name hands back. NFR-13: a refusal is returned and not swallowed, because
/// «не удалось спросить» is not «ответил правильный».
pub fn server_process_id(handle: HANDLE) -> WinResult<u32> {
    let mut pid = 0u32;

    // SAFETY: `handle` is the caller's live pipe handle, and `pid` is a live local of this frame
    // the call writes one `u32` into. Nothing else of ours is touched.
    unsafe { GetNamedPipeServerProcessId(handle, &raw mut pid) }?;

    Ok(pid)
}

/// The sentence a mismatched server is reported with — finding Н42, task T-41-9.
///
/// Pure, and in this module rather than in the bench, so that the run and the tests say the same
/// thing in the same words. **Both numbers are named**: «красный без внятной причины» is the
/// failure mode the finding is about, and a run that merely said «канал ответил не то» would
/// leave whoever reads it to guess.
///
/// Process identifiers are not secrets and carry nothing of what was typed — SEC-01 and SEC-07
/// are untouched by this sentence.
pub fn wrong_server(expected: u32, actual: u32) -> String {
    format!(
        "канал SEC-04a отвечает не тот процесс: ожидался PID {expected}, на другом конце трубы \
         PID {actual}. Имя канала занял посторонний сервер — замеру этого прогона верить нельзя."
    )
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
///
/// # `FILE_FLAG_FIRST_PIPE_INSTANCE` — finding Н42, task T-41-9
///
/// The flag says out loud what this server requires: that it be the **first** instance of its
/// name. A name already taken is then an outright refusal of `CreateNamedPipeW` —
/// `ERROR_ACCESS_DENIED`, reported through [`start`] and journalled (`Kind::Channel`) — rather
/// than an instance quietly added beside somebody else's.
///
/// ⚠⚠ **And it changes nothing observable today, which was measured rather than assumed.**
/// [`PIPE_INSTANCES`] is **1**, and `nMaxInstances` is enforced on the name kernel-wide: the
/// limit is reached after the first instance, so a second `CreateNamedPipeW` on this name is
/// refused **with the same `ERROR_ACCESS_DENIED`, with the flag or without it**. Both
/// configurations were built and run — journal
/// `scratchpad-E41\red-T-41-9-second-server.log`.
///
/// So the first half of finding Н42's reasoning — «программа того же пользователя поднимет
/// сервер с тем же именем» *beside* ours — does not hold: joining this name was never possible.
/// The flag is kept all the same, because it states the requirement where a reader looks for it
/// and it survives a future edit of `PIPE_INSTANCES`; it is belt beside braces, and the report
/// says so instead of claiming a repair it did not make.
///
/// ⭐ **The substance of the finding is entirely in the other half, and that one is real.** A
/// stranger who raises a server on this name **before** this process exists takes the name: our
/// `CreateNamedPipeW` then fails, the product runs on (NFR-13, a channel is a diagnostic), and
/// the bench connects to **the stranger's** server and believes it. Nothing about the open mode
/// can help there — the client has to ask *whose* server answered. See [`server_process_id`] and
/// [`wrong_server`].
pub const PIPE_OPEN_MODE: FILE_FLAGS_AND_ATTRIBUTES =
    FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_OUTBOUND.0 | FILE_FLAG_FIRST_PIPE_INSTANCE.0);

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
///
/// ⭐ **Task T-41-9 measured that this number, and not the open mode, is what actually refuses a
/// second server.** `nMaxInstances` is enforced on the name kernel-wide, so the limit is reached
/// after the first instance and every later `CreateNamedPipeW` on this name fails with
/// `ERROR_ACCESS_DENIED` — with [`FILE_FLAG_FIRST_PIPE_INSTANCE`] in [`PIPE_OPEN_MODE`] or
/// without it. Published so that the test of that refusal can name **both** reasons for it: a
/// task that moves either one has to come and say which.
pub const PIPE_INSTANCES: u32 = 1;

/// Outbound buffer of the pipe, in bytes.
///
/// The payload is twenty-one short `key=value` lines — some three hundred bytes — and a
/// page is comfortably more than the widest it could grow to.
/// Sizing it above the payload is what lets [`publish`] hand the bytes over without waiting
/// for the client to read them.
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
/// The bytes are [`render`] of one [`snapshot`], which is sixteen counts, three flags, one word
/// of section 7 and one of the five words of [`crate::watchdog::Reason`]. Nothing else can be
/// sent from here: there is no other write in this module.
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
/// needs the value published rather than fetched. Module `buffer` writes this from the one place
/// its counter is written — `Recorder::set_cycle`, since task T-69-3 — under the `testing`
/// feature and nowhere else.
///
/// **SEC-01, SEC-07.** A step along the cycle — a number in `0..len` — and nothing else.
/// Which layout that step names is not here, and neither is a scan code, a character or a
/// stroke. A zero means "what is on the screen is what was typed", which is the state every
/// flush of FR-10 leaves behind (FR-34).
static CYCLE_POSITION: AtomicUsize = AtomicUsize::new(0);

/// The layout strokes are being recorded under **right now** — the mirror of the stamp of
/// FR-04, `Recorder::active`.
///
/// The same construction as [`CYCLE_POSITION`] and for the same reason: the stamp is a field
/// of a thread-local of the input thread (section 6.3), so a reader outside that thread needs
/// it published rather than fetched. Module `app` writes this from the one place the stamp is
/// written — `publish_active_layout` — and only when the write really landed in a recorder,
/// under the `testing` feature and nowhere else.
///
/// Zero means "nothing has been published yet", which is the state between process start and
/// the first `publish_active_layout`.
///
/// **SEC-01, SEC-07.** An `HKL` — the identifier of a keyboard layout, exactly what module
/// `switch` already reasons about in the open: "an `HKL` is an identifier of a layout, not a
/// keystroke, and is fair game". It is not a scan code, not a character and not a stroke, and
/// nothing about what the user typed can be recovered from it.
static ACTIVE_LAYOUT: AtomicUsize = AtomicUsize::new(0);

/// The **shape** of the last replacement packet FR-41 built: how many characters it erased, how
/// many UTF-16 code units it typed back, and how many of those units were **distinct**.
///
/// # Why the shape, and why `distinct` — task T-10-6
///
/// The defect that task was given is a packet that came out as six copies of one character where
/// six different ones were expected. Every key that existed said the press had gone perfectly:
/// `buffer_len=6`, `cycle_position` alternating, `send_mismatches=0`. None of them could tell
/// «продукт построил верный пакет, а приложение отобразило его схлопнутым» from «продукт построил
/// схлопнутый пакет», and those are different defects in different modules. `distinct` separates
/// them in one reading: six units and one distinct value is a collapsed packet, six and six is a
/// correct one — whatever ends up on the screen.
///
/// `erase` and `units` travel with it because the other half of the same defect was «слово
/// стёрлось»: FR-41 erases `N` characters and types the conversion back, and a packet whose
/// `erase` and `units` disagree is exactly that report, seen from inside.
///
/// # One atomic and not three
///
/// The three numbers are read together and compared with each other, so a torn reading would be
/// a *wrong* measurement rather than a slightly stale one — unlike every other mirror on this
/// channel, which is a count that stands alone. Packed as three sixteen-bit fields of one
/// `u64`, they are published by a single store and read by a single load, and cannot disagree.
/// Each field saturates at `u16::MAX`; the ring of FR-07 holds 256 strokes, so nothing this
/// program can build comes near it.
///
/// Zero — the value before the first replacement — renders as `0/0/0`.
///
/// **SEC-01, SEC-07.** Three counts. Not a character, not a code unit, not a scan code: which
/// units the packet held is exactly what is *not* here, and «сколько среди них различных» cannot
/// be turned back into any of them.
static LAST_REPLACEMENT: AtomicU64 = AtomicU64::new(0);

/// Width of one field of [`LAST_REPLACEMENT`].
const REPLACEMENT_FIELD_BITS: u32 = 16;

/// The method the last replacement really ran — task **T-10-8**, FR-42а.
///
/// `replacement_method` reports the *configured* value, and since FR-42а that value may be
/// `auto` — a rule, not a method. Which of the two real packets a given press actually built
/// is decided per replacement, against the class of the foreground window at that moment, and
/// without this mirror the decision is invisible from outside the process: the live evidence
/// the task asks for («в Блокноте пошёл путь выделения, в cmd — путь Backspace») would be
/// unprovable.
///
/// Holds [`METHOD_NONE`] until the first replacement, then the code of the resolved method —
/// never [`crate::settings::ReplacementMethod::Auto`], which cannot be the outcome of a
/// resolution.
///
/// **SEC-01, SEC-07.** One of four words about the program's own behaviour. Not a window
/// class, not a window, and certainly not a stroke: the class the decision was made from is
/// deliberately *not* published — it is somebody else's window title material, and the method
/// is the whole of what the bench needs.
///
/// ⭐ **Stage Э96: a third packet, and so a fourth word** — `keystrokes`, the keystroke path of the
/// classic console ([`crate::inject::Delivery::Keystrokes`]). The bench of the classic console
/// tells it apart from the `backspace` of Windows Terminal by this word and by nothing else.
static LAST_REPLACEMENT_METHOD: AtomicU8 = AtomicU8::new(METHOD_NONE);

/// [`LAST_REPLACEMENT_METHOD`] before the first replacement: nothing has run yet.
const METHOD_NONE: u8 = 0;

/// The last replacement ran the `Backspace` packet of FR-41.
const METHOD_BACKSPACE: u8 = 1;

/// The last replacement ran the `Selection` packet of FR-42.
const METHOD_SELECTION: u8 = 2;

/// ⭐ The last replacement ran the keystroke packet of the classic console — stage Э96.
const METHOD_KEYSTROKES: u8 = 3;

/// **What [`note_replacement_method`] takes** — the code a method or a delivery is published as.
///
/// Two kinds of value reach that function and both must keep their own honest answer: a resolved
/// [`crate::inject::Delivery`] — what `inject::on_hotkey` publishes since stage Э96 — and a
/// configured [`ReplacementMethod`], which is what the channel's own test publishes and which
/// answers `none` for `auto`, a rule and never an outcome.
pub trait PublishedMethod {
    /// The code [`LAST_REPLACEMENT_METHOD`] holds for this value.
    fn code(self) -> u8;
}

impl PublishedMethod for ReplacementMethod {
    fn code(self) -> u8 {
        match self {
            Self::Backspace => METHOD_BACKSPACE,
            Self::Selection => METHOD_SELECTION,
            Self::Auto => METHOD_NONE,
        }
    }
}

impl PublishedMethod for crate::inject::Delivery {
    fn code(self) -> u8 {
        match self {
            Self::Backspace => METHOD_BACKSPACE,
            Self::Selection => METHOD_SELECTION,
            Self::Keystrokes => METHOD_KEYSTROKES,
        }
    }
}

/// ⭐ Whether the last replacement packet **inserted something other than what it took off the
/// screen** — task **T-10-17**.
///
/// # Why this bit exists
///
/// The live protocol of defect E has all thirty-four presses reading `last_replacement=6/6/6`
/// while the person reports the text «моргает» and stays as it was. `6/6/6` is the same reading
/// for a press that rewrote six characters into six different ones and for a press that took six
/// characters off and typed the very same six back: the shape of the packet says how much moved,
/// never **whether anything did**. Four repairs of that defect were guesses because this bit did
/// not exist to be read.
///
/// # What the two sides are, exactly — and what they are not
///
/// The product knows two texts at the moment [`crate::inject::replace_in_with`] forms the packet:
/// what it **believes** it is erasing — the characters the recorded strokes produced when they
/// were pressed, `Keystroke::produced` — and what it is inserting, the conversion of those same
/// strokes into the target layout. This bit is the comparison of exactly those two.
///
/// ⚠ **It is not "the screen changed".** The product never reads the screen (SEC-01 forbids it
/// and nothing here could), so what it can honestly publish is its own two sides. The difference
/// is load-bearing, not pedantic: at `D = 0` of task T-10-16 the stamp is stale, the product
/// believes it is erasing `ghbdtn`, inserts `привет`, and the field — which really held `привет`
/// all along — does not change. That press is `1` here and «не изменилось» on the screen, and the
/// two readings together are what separates it from a press that genuinely returned what it took.
///
/// `false` before the first replacement, which is the state [`LAST_REPLACEMENT`] reports as
/// `0/0/0`: a reader that sees no replacement has no flag to interpret.
///
/// **SEC-01, SEC-07.** One bit. Neither text is published, neither can be recovered from it, and
/// «равны ли они» is not a character, a code unit or a scan code.
static LAST_REPLACEMENT_CHANGED: AtomicBool = AtomicBool::new(false);

/// ⭐ **The direction the last replacement actually applied**: the layout the strokes were
/// recorded under, and the layout they were rendered into. Task **T-10-17**.
///
/// # Why the direction, and why it is not derivable from what was already here
///
/// [`ACTIVE_LAYOUT`] is the stamp **now**, and step 5 of FR-40 switches the foreground window
/// straight after the packet goes out, so by the time any reader gets there the stamp is the
/// *target* of the press it is trying to describe. `cycle_position` is a step, not a layout.
/// Neither of them can answer «куда именно эта замена переводила», and that question has a
/// specific wrong answer worth being able to see: `Cycle::target` returns `origin` itself
/// whenever `step % len == 0` — the rollback of FR-32 and FR-33 — so a press whose direction is
/// `X→X` converts the strokes into the layout they were typed in and reproduces the original text
/// code unit for code unit. That press is a **legitimate** identity, and it is indistinguishable
/// on today's channel from an identity produced by a defect.
///
/// # One word and not two
///
/// The two halves are compared **with each other** — the whole question is whether they are
/// equal — so a reading that mixed the origin of one press with the target of another would be a
/// wrong measurement rather than a stale one. That is the argument [`LAST_REPLACEMENT`] makes for
/// packing three counts into one atomic, and it applies here for the same reason: `from` in the
/// upper thirty-two bits, `to` in the lower, published by a single store and read by a single
/// load.
///
/// Each half saturates at [`u32::MAX`] rather than truncating. An `HKL` of a keyboard layout is a
/// language identifier and a device handle in one thirty-two bit word — `0x0409_0409`,
/// `0x0419_0419` — so nothing this program can hold comes near the bound; a value that did would
/// render as `0xffffffff`, which names no layout, instead of silently becoming a different one.
///
/// Zero on both halves — rendered `0x00000000/0x00000000` — is "no replacement yet", the same
/// convention `active_layout` and `last_replacement` already use.
///
/// **SEC-01, SEC-07.** Two layout handles, of exactly the kind [`ACTIVE_LAYOUT`] already
/// publishes: "an `HKL` is an identifier of a layout, not a keystroke, and is fair game". Not a
/// scan code, not a character, not a stroke.
static LAST_REPLACEMENT_DIRECTION: AtomicU64 = AtomicU64::new(0);

/// Width of one half of [`LAST_REPLACEMENT_DIRECTION`].
const DIRECTION_FIELD_BITS: u32 = 32;

/// The five outcomes of [`crate::buffer::Recorder::restamp`], counted — task **T-10-15**.
///
/// # Why counters and not one more flag
///
/// The repair of defect E (task T-10-14) reads the real layout on the first stroke of a new word
/// and re-stamps the recorder with it. That path has **five** ways to end and, until this task,
/// **not one of them was visible from outside the process**: `active_layout` shows the stamp
/// after the fact, so a stamp that stayed stale reads identically whether the probe was never
/// consulted, answered with the value already held, or answered with a layout the cache of FR-20
/// has no map for. Three different defects, one reading. The task that measures why the repair
/// does not help a person coming back from a real absence cannot start from a reading like that.
///
/// The refusal on the cache is the one this task was written around: it is **silent** by
/// construction — a callback cannot rebuild the cache, so refusing is the only correct thing it
/// can do — and it is exactly the outcome a stale cache would produce.
///
/// # One index per outcome
///
/// Indexed by [`Restamp`] so that the publisher is a single relaxed `fetch_add` on an entry
/// chosen by a `match`, in the shape [`note_replacement_method`] already uses. The five are
/// separate atomics rather than fields of one word because they are counts that stand alone:
/// nothing compares two of them for a torn reading, unlike [`LAST_REPLACEMENT`].
///
/// **SEC-01, SEC-07.** Five counts of the program's own decisions. Not a character, not a scan
/// code, not a stroke — and deliberately **not the layout** that was refused: which layout the
/// cache lacked is `active_layout`'s business and not a fact this key needs in order to say
/// "the refusal happened".
static RESTAMP_OUTCOMES: [AtomicU32; Restamp::COUNT] =
    [const { AtomicU32::new(0) }; Restamp::COUNT];

/// How a call of [`crate::buffer::Recorder::restamp`] ended — task **T-10-15**.
///
/// The five rows of the table in that task, in the order the code reaches them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restamp {
    /// **Not called at all**: the ring was not empty, so this stroke is not the first of a word.
    /// The ordinary case for every letter of a word after its first, and the one that says the
    /// branch of the repair was never reached for this keystroke.
    Skipped = 0,
    /// Called, and `Recorder::stamp` holds no probe — the recorder was built without one, or
    /// crossed the FR-70 gate without it. Nothing can be read, so nothing is.
    NoProbe = 1,
    /// The probe answered with an empty layout, or with the layout already stamped. Nothing to
    /// correct; this is what a *healthy* first stroke looks like.
    Unchanged = 2,
    /// ⭐ **The silent refusal.** The probe answered with a real layout, different from the one
    /// stamped, and the cache of FR-20 has no map for it. The stamp stays stale and the callback
    /// says nothing, because a callback cannot rebuild the cache.
    Uncached = 3,
    /// The stamp was corrected — the repair of defect E, doing its work.
    Accepted = 4,
}

impl Restamp {
    /// How many outcomes there are. The length of [`RESTAMP_OUTCOMES`] and of nothing else.
    const COUNT: usize = 5;

    /// Every outcome, in the order [`render`] emits them and [`KEYS`] lists them.
    const ALL: [Self; Self::COUNT] = [
        Self::Skipped,
        Self::NoProbe,
        Self::Unchanged,
        Self::Uncached,
        Self::Accepted,
    ];
}

/// Records the outcome of one call — or one deliberate non-call — of
/// [`crate::buffer::Recorder::restamp`]. Task **T-10-15**.
///
/// # NFR-01 to NFR-05
///
/// One relaxed `fetch_add` and nothing else: no allocation (NFR-03), no lock (NFR-04), no I/O
/// and nothing formatted (NFR-05), a constant handful of instructions (NFR-01, NFR-02).
/// `Relaxed` for the reason [`note_buffer_len`] gives — these are counts read for their value,
/// never for ordering against anything else.
///
/// ⚠ **The caller is the hook callback**, and [`Restamp::Skipped`] is published on *every*
/// stroke that is not the first of a word. That is the same term `note_buffer_len` has always
/// been called under from the same path, and the whole of this module — statics included — is
/// compiled out of every build that is not a `testing` one.
///
/// Plain `fetch_add` and not a saturating one, which is how every other counter this channel
/// publishes is written (`watchdog::DESKTOP_SWITCHES`, `guard::PROBES`): saturating costs a
/// compare-and-swap loop on a path taken once per keystroke, and what it would protect against
/// is `u32::MAX` strokes inside one session.
#[inline]
pub fn note_restamp(outcome: Restamp) {
    RESTAMP_OUTCOMES[outcome as usize].fetch_add(1, Ordering::Relaxed);
}

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

/// Publishes the position in the cycle of section 4.4 — called by module `buffer` from the one
/// place its counter changes, `Recorder::set_cycle` (task T-69-3), and by nothing else.
///
/// # NFR-01 to NFR-05
///
/// The caller sits on the input thread and is reached from two paths: the hotkey path
/// (`Recorder::advance_cycle`) and the flush path every rule of FR-10 arrives at
/// (`Recorder::clear_ring` and the partial arm of `Recorder::reset_up_to`, both reached from
/// inside the hook callback). So this is one relaxed atomic store and nothing
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

/// Publishes the layout the typing buffer stamps strokes with — called from the two places the
/// stamp is written, and from nowhere else.
///
/// ⚠ **There are two since task T-10-14.** `app::publish_active_layout` is the write that
/// answers an *event*, and `buffer::Recorder::restamp` is the read the buffer makes for itself
/// on the first stroke of a new word — the repair of defect E, which exists precisely because
/// the first of those two can carry a value the system had not applied yet. Both mirror here,
/// because a stamp corrected in one of them and mirrored from only the other would make this key
/// a value the program has already stopped using — and this key is what a detector of that very
/// defect reads.
///
/// # Why this key exists — task T-10-5
///
/// Twenty-eight keys said what the program *did* and none of them said what the stamp *held*.
/// FR-26 takes the direction of every conversion from that value (through
/// `Stroke::layout`), so a stamp that has gone stale is the difference between a conversion
/// and the «моргание» of the acceptance session — and until this key there was no way to see
/// it from outside the process, only to infer it from `cache_builds` and `layout_probes`.
/// The whole of defect A of that task is a statement about this number, and a statement about
/// a number nobody can read is a guess.
///
/// # NFR-01 to NFR-05
///
/// One relaxed atomic store and nothing else — no allocation (NFR-03), no lock (NFR-04), no
/// I/O and nothing formatted (NFR-05), a constant handful of instructions (NFR-01, NFR-02).
/// `Relaxed` for the reason [`note_buffer_len`] gives. ⚠ One of the two callers *is* in the hook
/// callback since task T-10-14, so that list is now a requirement rather than a description —
/// the same terms `note_cycle_position` has always been called under.
///
/// # SEC-01, SEC-07
///
/// A layout handle. See [`ACTIVE_LAYOUT`].
#[inline]
pub fn note_active_layout(layout: usize) {
    ACTIVE_LAYOUT.store(layout, Ordering::Relaxed);
}

/// Publishes the shape of the replacement packet FR-41 has just built — called by module
/// `inject` from the one place a packet is formed, and by nothing else.
///
/// `erase` is the `N` of FR-41, `units` the UTF-16 code units of the conversion, `distinct` how
/// many different values those units take. See [`LAST_REPLACEMENT`] for why the third number is
/// the one task T-10-6 needed and why the three travel in one atomic.
///
/// # NFR-01 to NFR-05
///
/// The caller sits on the input thread's message loop, never in the hook callback, and this is
/// one relaxed atomic store over three shifts: no allocation (NFR-03), no lock (NFR-04), no I/O
/// and nothing formatted (NFR-05). `Relaxed` for the reason [`note_buffer_len`] gives — the three
/// fields are ordered against each other by being one word, and against nothing else.
///
/// # SEC-01, SEC-07
///
/// Three counts. See [`LAST_REPLACEMENT`].
#[inline]
pub fn note_replacement(erase: usize, units: usize, distinct: usize) {
    let field = |value: usize| u64::from(u16::try_from(value).unwrap_or(u16::MAX));

    let packed = (field(erase) << (2 * REPLACEMENT_FIELD_BITS))
        | (field(units) << REPLACEMENT_FIELD_BITS)
        | field(distinct);

    LAST_REPLACEMENT.store(packed, Ordering::Relaxed);
}

/// Publishes the method the replacement being built really runs — called by
/// `inject::on_hotkey` from the one place FR-42а resolves `auto`, and by nothing else.
/// Task **T-10-8**.
///
/// `method` is the *resolved* method: one of the three packets since stage Э96 — a
/// [`crate::inject::Delivery`] — and never [`ReplacementMethod::Auto`]. `Auto` cannot be the
/// outcome of a resolution, and if a defect ever delivered it here anyway it is recorded as
/// [`METHOD_NONE`] — "nothing decidable ran" — rather than silently renamed to a real method, so
/// the defect stays visible on the channel instead of masquerading as a decision. See
/// [`PublishedMethod`] for the two kinds of value this takes.
///
/// # NFR-01 to NFR-05
///
/// The caller sits on the input thread's message loop, never in the hook callback, and this is
/// one relaxed atomic store: no allocation (NFR-03), no lock (NFR-04), no I/O and nothing
/// formatted (NFR-05). `Relaxed` for the reason [`note_buffer_len`] gives.
///
/// # SEC-01, SEC-07
///
/// A word about the program's own choice. See [`LAST_REPLACEMENT_METHOD`].
#[inline]
pub fn note_replacement_method(method: impl PublishedMethod) {
    LAST_REPLACEMENT_METHOD.store(method.code(), Ordering::Relaxed);
}

/// ⭐ Publishes the two facts task **T-10-17** adds about the replacement being built: whether
/// what it inserts differs from what it takes off the screen, and the direction it really
/// applied. Called by module `inject` from the one place a packet is formed — beside
/// [`note_replacement`] — and by nothing else.
///
/// `changed` is the comparison of the product's own two sides, argued at
/// [`LAST_REPLACEMENT_CHANGED`]; `from` is the numeric `HKL` the strokes were recorded under
/// (FR-26) and `to` the one they were rendered into. `from == to` is the rollback of FR-32, and
/// being able to read it is the point of the pair — see [`LAST_REPLACEMENT_DIRECTION`].
///
/// # NFR-01 to NFR-05
///
/// The caller sits on the input thread's message loop, never in the hook callback, and this is
/// two relaxed atomic stores over two shifts: no allocation (NFR-03), no lock (NFR-04), no I/O
/// and nothing formatted (NFR-05). `Relaxed` for the reason [`note_buffer_len`] gives.
///
/// Two stores and not one, and the split is where the comparisons are: the two halves of the
/// direction are compared with each other and therefore share a word, while the flag is read
/// *beside* the direction rather than against it — the same terms
/// [`note_replacement_method`] has published a word beside [`note_replacement`]'s counts since
/// task T-10-8.
///
/// # SEC-01, SEC-07
///
/// One bit and two layout handles. See the two statics.
#[inline]
pub fn note_replacement_outcome(changed: bool, from: usize, to: usize) {
    let half = |value: usize| u64::from(u32::try_from(value).unwrap_or(u32::MAX));

    LAST_REPLACEMENT_CHANGED.store(changed, Ordering::Relaxed);
    LAST_REPLACEMENT_DIRECTION.store(
        (half(from) << DIRECTION_FIELD_BITS) | half(to),
        Ordering::Relaxed,
    );
}

/// Unpacks [`LAST_REPLACEMENT_DIRECTION`] into the two halves it carries — from, then to.
fn last_replacement_direction() -> (u32, u32) {
    let packed = LAST_REPLACEMENT_DIRECTION.load(Ordering::Relaxed);

    (
        (packed >> DIRECTION_FIELD_BITS) as u32,
        (packed & u64::from(u32::MAX)) as u32,
    )
}

/// Unpacks [`LAST_REPLACEMENT_METHOD`] into the word [`render`] prints.
fn last_replacement_method() -> &'static str {
    match LAST_REPLACEMENT_METHOD.load(Ordering::Relaxed) {
        METHOD_BACKSPACE => "backspace",
        METHOD_SELECTION => "selection",
        METHOD_KEYSTROKES => "keystrokes",
        // `METHOD_NONE`, and — total match over a `u8` — every code nothing ever stores.
        _ => "none",
    }
}

/// Unpacks [`LAST_REPLACEMENT`] into the three numbers it carries — erase, units, distinct.
fn last_replacement() -> (u16, u16, u16) {
    let packed = LAST_REPLACEMENT.load(Ordering::Relaxed);
    let field = |shift: u32| ((packed >> shift) & u64::from(u16::MAX)) as u16;

    (
        field(2 * REPLACEMENT_FIELD_BITS),
        field(REPLACEMENT_FIELD_BITS),
        field(0),
    )
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
    /// Completed reinstallations of the hook — **FR-80**, and position 18 of section 11.3.
    ///
    /// The one number that says the watchdog did something, and the reason it is out here at
    /// all: from outside the process this quantity is not observable even in principle
    /// (decision Р-37), so a bench with no channel has nothing to confirm the position with.
    pub watchdog_recoveries: u32,
    /// Why the hook was last put back — one of the five words of
    /// [`crate::watchdog::Reason::name`], never anything else.
    pub watchdog_last_reason: crate::watchdog::Reason,
    /// Whether the focused field is a password field — **SEC-06, FR-70 to FR-73**, and position
    /// 14 of the matrix of section 11.3. Task **T-06-1**.
    ///
    /// ⚠ **Condition 2 of SEC-04a: a flag, and never the content.** This is one bit — `1` in a
    /// password field, `0` everywhere else — derived from the four-state publication of module
    /// [`crate::guard`]. The contents of a password field are read nowhere in this program, so
    /// there is nothing else that could arrive here.
    ///
    /// The reason it is out here at all is the reason SEC-04a exists at all: footnote 2 of
    /// section 11.3 makes the bench prove position 14 «через … отладочный канал SEC-04a,
    /// подтверждающий нулевую длину буфера», and the contents of a password field are not
    /// reachable from outside by the design of Windows. Without this key, SEC-06 is not provable
    /// by automated means.
    pub password_field: bool,
    /// Focus changes module [`crate::guard`] has been told about — **SEC-06**, task **T-06-1a**.
    ///
    /// The input thread's end of the chain: `guard::note_focus_moved` counts one for every
    /// `watchdog::WM_APP_FLUSH` that reaches the input window, which is one for every
    /// `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` the subscription of task T-03-3
    /// delivered.
    ///
    /// ⚠ **Why a bench needs it.** `password_field` alone cannot tell "the verdict is about the
    /// window you are looking at" from "the verdict is about the window before it": both are one
    /// bit, and a stale bit reads exactly like a fresh one. This counter and
    /// [`Snapshot::password_probes`] are what turn that into an observable — a focus change that
    /// does not raise the first, or does not raise the second after it, is a caret the program
    /// did not follow.
    ///
    /// SEC-01, SEC-07: a count of events. Not a window, not a title, not a stroke.
    pub focus_changes: u32,
    /// Probes carried out at the far end of that chain — **SEC-06**, task **T-06-1a**.
    ///
    /// `guard::run_pending_probe` counts one for every `guard::WM_APP_PROBE` that reached the
    /// watcher window with a probe actually pending. Lower than [`Snapshot::focus_changes`]
    /// whenever two focus changes coalesced into one probe, which is the design (see
    /// `guard::PROBE_PENDING`); lower by a widening margin is the chain no longer running.
    ///
    /// SEC-01, SEC-07: a count of events.
    pub password_probes: u32,
    /// Whether FR-99 has disarmed buffering — task **T-08-3**, point 5 of its exclusion list.
    ///
    /// The flag [`crate::hook::fail_safe`] publishes. It is the second half of the panic
    /// question: a program that has gone fail-safe answers `Outcome::PASS` to everything, so a
    /// bench measuring "the product did not react" has to be able to tell that state from a
    /// callback that was never called at all.
    ///
    /// SEC-01, SEC-07: one bit about the program's own mode. Not a stroke.
    pub fail_safe: bool,
    /// Panics inside the callback since the last one that returned normally — **FR-99**.
    ///
    /// Task **T-08-3**, point 5. The claim it exists to settle is "FR-96 does not fire in state
    /// X", and one of the five things that had to be excluded before the product could be blamed
    /// is that three panics in a row had already disarmed the program. From outside the process
    /// that is not observable at all, which is the same argument decision Р-37 makes for
    /// [`Snapshot::watchdog_recoveries`].
    ///
    /// SEC-01, SEC-07: a count. The payload of a panic is dropped unread in `hook`, so there is
    /// nothing here that could carry a key code even in principle.
    pub consecutive_panics: u32,
    /// Keyboard arrivals and removals the input thread answered — **FR-21**, task **T-08-4**.
    ///
    /// [`crate::watchdog::Counters::device_changes`]. The reason it is out here is that task
    /// T-08-4 replaced the mechanism that delivers them: the keyboard entry of the Raw Input
    /// registration was measured to take keyboard input away from every low-level hook in the
    /// session while a window of this process is in front, so it is gone, and
    /// `watchdog::register_device_notice` brings the same news as the `WM_DEVICECHANGE` FR-21
    /// actually names. A replacement of a delivery mechanism has to be *shown* to deliver, and
    /// from outside the process the only observable is this number moving when a keyboard is
    /// plugged in. `cache_builds` is the other half of the same evidence: this counts the
    /// message, that counts the rebuild it caused.
    ///
    /// SEC-01, SEC-07: a count of events. The `WM_DEVICECHANGE` behind it carries a device name
    /// and that name is read nowhere in this program, so there is nothing here that could carry
    /// one.
    pub device_changes: u32,
    /// `WinEvent` flush events ignored as background noise — **FR-10 as read by Р-60**, task
    /// **T-10-0**.
    ///
    /// [`crate::watchdog::Counters::background_skips`]. The reason it is out here is the trap
    /// that task's specification names: the product subscribes with `WINEVENT_SKIPOWNPROCESS`,
    /// so a storm staged inside the product's own process is invisible to it and a test would
    /// come out green having checked nothing. A storm staged by a **foreign** process is
    /// visible, and this number growing under it is the positive control — the storm was
    /// delivered and turned away — while `window_flushes` standing still is the repair itself.
    /// One number cannot say both things, which is why the pair exists.
    ///
    /// SEC-01, SEC-07: a count of events. Not a window, not a title, not a stroke — the event
    /// behind it is dropped whole and nothing of it is read but the handle relation that
    /// decided it.
    pub background_skips: u32,
    /// Foreground focus events skipped as the frontmost window's own churn — **FR-10 as read
    /// by Р-60, narrowed by task T-10-0e**.
    ///
    /// [`crate::watchdog::Counters::focus_repeats`]. The reason it is out here is the same
    /// trap that put `background_skips` out: the product subscribes with
    /// `WINEVENT_SKIPOWNPROCESS`, so the same-hwnd churn a test stages must come from a
    /// **foreign** process, and this number growing under it is the positive control that
    /// the churn was delivered and turned away — while `window_flushes` standing still
    /// beside it is the repair itself. On a live run it is also the one number that says the
    /// frontmost window really was fidgeting while the user's typing survived.
    ///
    /// SEC-01, SEC-07: a count of events. Not a window, not a title, not a stroke — the
    /// handle behind it is compared as a value and dropped.
    pub focus_repeats: u32,
    /// Layout probes of FR-21 the input thread answered — `WM_APP_LAYOUT` arrivals at the
    /// probe handler, task **T-10-0f**.
    ///
    /// [`crate::watchdog::Counters::layout_probes`], until now published through the file
    /// report alone. The reason it moves onto the channel is the reason `focus_repeats` did:
    /// the number is the observable of a repair. Before T-10-0f the probe posted for every
    /// focus change died in the parking interval of FR-71 — measured as `layout_probes=1`
    /// against `window_flushes=12`, felt as «первое нажатие моргает» — and the repair is
    /// exactly this number growing beside [`Snapshot::focus_changes`] on a live run, press by
    /// press, without waiting for the process to exit. Beside `cache_builds` it also splits
    /// «the probe arrived» from «the layout really moved», which is the pair FR-21's delivery
    /// is judged by.
    ///
    /// SEC-01, SEC-07: a count of messages. Not a layout name, not a window, not a stroke.
    pub layout_probes: u32,
    /// Callback invocations the QPC instrument of criterion 2 §13 has measured — task
    /// **T-10-1**.
    ///
    /// [`crate::hook::callback_latency`]. The criterion demands «не менее 10 000 нажатий»,
    /// and this is the number that proves the sample was taken rather than assumed; position
    /// 23 of §11.3 reads it before it trusts the three percentile keys below. One invocation
    /// is one sample, whatever path the callback took; a press and its release are two.
    ///
    /// SEC-01, SEC-07: a count of invocations. Not a key, not a scan code, not a stroke.
    pub callback_samples: u64,
    /// Median duration of the callback, in nanoseconds — task **T-10-1**.
    ///
    /// The reason these are on the channel is the reason `watchdog_recoveries` is (решение
    /// Р-37): from outside the process the duration of a hook callback is not observable
    /// even in principle, and criterion 2 of §13 requires it *measured*. Reported as the
    /// upper bound of the histogram cell — never understated. Nanoseconds and not
    /// microseconds because the callback is routinely submicrosecond and an integer of
    /// microseconds would publish zeros.
    ///
    /// SEC-01, SEC-07: a duration. Nothing about any stroke enters the instrument.
    pub callback_p50_ns: u64,
    /// 99th percentile of the callback duration, in nanoseconds — the number **NFR-01**
    /// bounds by 100 µs (100 000 here). Task **T-10-1**, same terms as
    /// [`Snapshot::callback_p50_ns`].
    pub callback_p99_ns: u64,
    /// Exact worst case of the callback duration, in nanoseconds — the number **NFR-02**
    /// bounds by 1 ms (1 000 000 here). Unclamped, unlike the percentiles, which read from
    /// fixed histogram cells. Task **T-10-1**, same terms as [`Snapshot::callback_p50_ns`].
    pub callback_max_ns: u64,
    /// The layout the typing buffer is stamping strokes with — task **T-10-5**.
    ///
    /// [`ACTIVE_LAYOUT`], written by `app::publish_active_layout`. **FR-26 takes the
    /// direction of every conversion from this value** and from nothing else, so the pair
    /// «what the stamp holds» against «what the foreground window really runs» is the whole
    /// question defect A of that task asks. Every other key on this channel answers "did
    /// something happen"; this one answers "what is in the register the answer is computed
    /// from", which no count can be substituted for.
    ///
    /// Rendered as the hexadecimal `HKL` — `0x04090409` for US, `0x04190419` for RU — because
    /// the reader of this key is comparing it against a layout handle and a decimal `usize`
    /// would have to be converted by hand at every comparison.
    ///
    /// SEC-01, SEC-07: a layout handle. Not a scan code, not a character, not a stroke.
    pub active_layout: usize,
    /// Characters the last replacement packet erased — the `N` of **FR-41**. Task **T-10-6**.
    pub replacement_erase: u16,
    /// UTF-16 code units that packet typed back — **FR-41**. Task **T-10-6**.
    pub replacement_units: u16,
    /// How many of those units were **distinct** — the number that tells a correct packet from a
    /// collapsed one. Task **T-10-6**; see [`LAST_REPLACEMENT`].
    pub replacement_distinct: u16,
    /// The method the last replacement really ran — `"none"` before the first one, then
    /// `"backspace"`, `"selection"` or, since stage Э96, `"keystrokes"` (the classic console).
    /// Task **T-10-8**, FR-42а: [`Snapshot::replacement_method`]
    /// is the *configured* value and may read `auto`, which is a rule and not a method; this is
    /// what the rule decided for the most recent press. See [`LAST_REPLACEMENT_METHOD`].
    pub last_replacement_method: &'static str,
    /// Flush requests the two `WinEvent` subscriptions raised — **FR-10**, task **T-10-9**.
    ///
    /// [`crate::watchdog::Counters::window_flushes`], published through the file report of
    /// `LANGSW_TESTING_REPORT` alone until now. The reason it moves onto the channel is the
    /// reason `background_skips` and `focus_repeats` moved: **it is the number a hypothesis is
    /// decided by, and the file sink cannot answer it in time.** The report is written by the
    /// main thread *after the process has exited* (`app.rs`), so a scenario that requires the
    /// process to survive the step under investigation — «открыть Проводник и продолжить
    /// работать» — cannot read it at all. This number, `full_clears` and `strokes_removed`
    /// beside it, is what separates «буфер стирается сбросами» from «в буфер ничего не
    /// попадает»: the first moves all three, the second leaves all three standing while
    /// `buffer_len` stays at zero.
    ///
    /// SEC-01, SEC-07: a count of events. Not a window, not a title, not a stroke.
    pub window_flushes: u32,
    /// Flushes that emptied the whole buffer — task **T-10-9**.
    ///
    /// [`crate::watchdog::Counters::full_clears`], on the channel for the reason
    /// [`Snapshot::window_flushes`] gives. Beside it, it is the half of the pair that says a
    /// flush *reached the typing the user had done* rather than arriving at an empty ring: a
    /// storm against an empty buffer moves `window_flushes` and leaves this standing.
    ///
    /// SEC-01, SEC-07: a count of events.
    pub full_clears: u32,
    /// Strokes all flushes together removed — task **T-10-9**.
    ///
    /// [`crate::watchdog::Counters::strokes_removed`], on the channel for the reason
    /// [`Snapshot::window_flushes`] gives, and the sharpest of the three: it is measured in
    /// *the user's keystrokes*, so `strokes_removed` moving by exactly the length of what was
    /// typed is the whole of hypothesis 1 — the acceptance session read `strokes_removed=18`
    /// of 18 against nine presses on an empty buffer — while a буфер that never received the
    /// strokes in the first place leaves it at rest.
    ///
    /// SEC-01, SEC-07: a count of strokes removed, never a stroke. No scan code, no character
    /// and no layout of any of them enters this number.
    pub strokes_removed: u32,
    /// Focus events **FR-14** exempted — the application's own answer to the user's typing, task
    /// **Т-48-2**.
    ///
    /// [`crate::watchdog::Counters::focus_after_typing`], on the channel for the reason
    /// `focus_repeats` is: **it is the number the hypothesis is decided by, and the file sink
    /// cannot answer it in time.** The scenario is «type in the address bar of Edge and go on
    /// working», which requires the process to survive the step under investigation.
    ///
    /// Read beside `window_flushes` and `strokes_removed`, it separates the two ways the rule
    /// can be wrong: this standing still while the other two rise is FR-14 not firing where it
    /// should, and this rising while a genuine field change goes unflushed is FR-14 firing where
    /// it should not.
    ///
    /// SEC-01, SEC-07: a count of events. Not a window, not an element, not a stroke.
    pub focus_after_typing: u32,
    /// Which of the four states of [`crate::guard::Field`] the gate is in — task **T-10-10**.
    ///
    /// [`crate::guard::field`], printed through [`crate::guard::Field::name`] — one of
    /// `pending`, `ordinary`, `password`, `undetermined`, and never anything else. The same
    /// shape [`Snapshot::watchdog_last_reason`] uses, and for the same reason: the list of
    /// words lives beside the list of arms so the two cannot drift apart.
    ///
    /// ⚠ **Condition 2 of SEC-04a: this is the state of the gate, not the content of the
    /// field.** The set of things it can say is closed and written out in `guard.rs`; the
    /// contents of a password field are read nowhere in this program.
    /// [`Snapshot::password_field`] is unchanged and remains the **flag** SEC-06 requires —
    /// this key does not replace it and does not restate it.
    ///
    /// ⚠ **Why a bench needs it: `password_field` is `0` for two different situations.** It is
    /// `0` for [`Field::Ordinary`](crate::guard::Field::Ordinary) — an ordinary field,
    /// buffering on — and it is also `0` for [`Field::Pending`](crate::guard::Field::Pending),
    /// where the focus has moved, no verdict exists yet, buffering is **off** and
    /// `app::park_buffer` has emptied the buffer through `buffer::reset()` — past the flush
    /// counters, so [`Snapshot::strokes_removed`] does not move either. On the channel those
    /// two читаются одинаково: `password_field=0`, `buffer_len=0`, `strokes_removed` standing.
    /// That is the first row of the break-down table of task T-10-9 — «нажатия не доходят
    /// до буфера, потому что ворота держат буфер снятым» — and until this key it was the one
    /// row of four that a live process could not be asked about directly. A gate that never
    /// leaves `pending` prints `pending` here while everything else on the channel looks
    /// healthy.
    ///
    /// SEC-01, SEC-07: one of four named constants.
    pub field_state: crate::guard::Field,
    /// Clipboard accesses that exhausted `selection::OPEN_ATTEMPTS` — task **T-10-11**, FR-62.
    ///
    /// [`crate::selection::Counters::open_refusals`], and **this is the refusal of the selection
    /// path itself**: FR-60/FR-61 replace a word by selecting it and going through the clipboard,
    /// so an access that never opened is a replacement that cannot run. FR-42а resolves `auto` to
    /// `selection` for every non-console window — Блокнот, Telegram, Word, Chrome, VS Code — which
    /// is to say for almost everything the user types into, and until this key that whole path had
    /// **not one number** on the channel. The fourth row of the break-down table of task T-10-9,
    /// «замена отказывает», could be read only as `last_replacement=0/0/0`: that the replacement
    /// did not happen, never why.
    ///
    /// SEC-01, SEC-07: a count of refused accesses. Not the text of the clipboard, not its
    /// format, not its size — `selection::snapshot` and `selection::restore` publish nothing of
    /// what they carry and cannot, and nothing of it reaches this number.
    pub clipboard_refusals: u32,
    /// `CloseClipboard` calls that failed — task **T-10-11**.
    ///
    /// [`crate::selection::Counters::close_failures`], and the one of the three that predicts a
    /// **lasting** failure rather than a momentary one: a clipboard this process opened and could
    /// not close stays open, and every later access — ours and everyone else's — is refused while
    /// it does. «Переключение перестало работать **везде**» is the shape a stuck clipboard has,
    /// and this is the number that would say so.
    ///
    /// SEC-01, SEC-07: a count of failed calls.
    pub clipboard_close_failures: u32,
    /// Clipboard opens that were refused once and retried — task **T-10-11**, FR-62.
    ///
    /// [`crate::selection::Counters::open_retries`]. Beside the two above it is the **gradient**:
    /// contention that is growing but has not yet exhausted `OPEN_ATTEMPTS` moves this and leaves
    /// [`Snapshot::clipboard_refusals`] standing — «становится тяжелее, но ещё работает». A run
    /// looking for a rare event needs the reading that comes *before* the event as much as the
    /// event itself.
    ///
    /// SEC-01, SEC-07: a count of retries.
    pub clipboard_retries: u32,
    /// ⭐ The five outcomes of `buffer::Recorder::restamp` — task **T-10-15**, indexed by
    /// [`Restamp`].
    ///
    /// [`RESTAMP_OUTCOMES`] says why they exist: the repair of defect E has five ways to end and
    /// four of them leave the stamp exactly as it was, so `active_layout` alone cannot tell them
    /// apart. `restamp_uncached` is the one the task was written around — the refusal that is
    /// silent by construction.
    ///
    /// The five are read in one pass and rendered as five keys, so a single snapshot of the
    /// channel separates every row of that task's table.
    ///
    /// SEC-01, SEC-07: five counts of the program's own decisions. See [`RESTAMP_OUTCOMES`].
    pub restamp: [u32; Restamp::COUNT],
    /// ⭐ Whether the last replacement inserted something other than what it took off the screen
    /// — task **T-10-17**.
    ///
    /// [`LAST_REPLACEMENT_CHANGED`] says why one bit was the missing quantity and what its two
    /// sides are. `false` before the first replacement, which is the state
    /// [`Snapshot::replacement_erase`] and its two neighbours report as `0/0/0`.
    ///
    /// SEC-01, SEC-07: one bit about the program's own packet. Neither text is here.
    pub last_replacement_changed: bool,
    /// ⭐ The layout the strokes of the last replacement were **recorded under** — task
    /// **T-10-17**, the `from` half of [`LAST_REPLACEMENT_DIRECTION`].
    ///
    /// SEC-01, SEC-07: a layout handle, the kind [`Snapshot::active_layout`] already publishes.
    pub last_replacement_from: u32,
    /// ⭐ The layout they were **rendered into** — the `to` half of the same word.
    ///
    /// Equal to [`Snapshot::last_replacement_from`] exactly when the press was the rollback of
    /// FR-32 and FR-33, which is a legitimate identity and the one this channel could not name
    /// until now.
    pub last_replacement_to: u32,
}

/// Reads [`RESTAMP_OUTCOMES`] in one pass, in the order of [`Restamp::ALL`].
///
/// Not a consistent cut, for the reason [`snapshot`] gives about every other number it takes:
/// five separate relaxed loads of five monotone counts, none of which has to agree with another.
fn restamp_outcomes() -> [u32; Restamp::COUNT] {
    Restamp::ALL.map(|outcome| RESTAMP_OUTCOMES[outcome as usize].load(Ordering::Relaxed))
}

/// Takes the numbers in one pass — **the single source both sinks read** (decision Р-28).
///
/// Not a consistent cut of anything: the values come from separate atomics and the program
/// keeps running while they are read. That is deliberate and it is enough, because every one
/// of them is a monotone count or a published setting, and nothing here is a pair that has to
/// agree.
pub fn snapshot() -> Snapshot {
    let (send_mismatches, events_lost) = crate::inject::send_mismatches();
    let watchdog = crate::watchdog::health();
    let subscriptions = crate::watchdog::counters();
    let guard = crate::guard::counters();
    let clipboard = crate::selection::counters();
    // One load, three fields — see [`LAST_REPLACEMENT`] for why they may not be read apart.
    let replacement = last_replacement();
    // One load, two halves — the same argument, for the same reason: the reading this pair is
    // taken for is whether they are equal.
    let direction = last_replacement_direction();

    let mut state = Snapshot {
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
        watchdog_recoveries: watchdog.recoveries,
        watchdog_last_reason: watchdog.last_reason,
        password_field: crate::guard::password_field(),
        focus_changes: guard.focus_changes,
        password_probes: guard.probes,
        fail_safe: crate::hook::fail_safe(),
        consecutive_panics: crate::hook::consecutive_panics(),
        device_changes: subscriptions.device_changes,
        background_skips: subscriptions.background_skips,
        focus_repeats: subscriptions.focus_repeats,
        layout_probes: subscriptions.layout_probes,
        callback_samples: 0,
        callback_p50_ns: 0,
        callback_p99_ns: 0,
        callback_max_ns: 0,
        active_layout: ACTIVE_LAYOUT.load(Ordering::Relaxed),
        replacement_erase: replacement.0,
        replacement_units: replacement.1,
        replacement_distinct: replacement.2,
        last_replacement_method: last_replacement_method(),
        window_flushes: subscriptions.window_flushes,
        full_clears: subscriptions.full_clears,
        strokes_removed: subscriptions.strokes_removed,
        focus_after_typing: subscriptions.focus_after_typing,
        field_state: crate::guard::field(),
        clipboard_refusals: clipboard.open_refusals,
        clipboard_close_failures: clipboard.close_failures,
        clipboard_retries: clipboard.open_retries,
        restamp: restamp_outcomes(),
        last_replacement_changed: LAST_REPLACEMENT_CHANGED.load(Ordering::Relaxed),
        last_replacement_from: direction.0,
        last_replacement_to: direction.1,
    };

    // ⚠ Read **after** every mirror above, deliberately. Summarising the latency histogram
    // walks its eight thousand cells — tens of microseconds — where everything above is a
    // handful of atomic loads. Put ahead of them it would sit between a caller's publish and
    // this function's read of the cheap mirrors, widening a window that was previously a few
    // loads wide; a test that publishes `cycle_position` and immediately snapshots — while
    // other tests of its binary publish the same process-wide mirror in parallel — is exactly
    // the caller that noticed. The instrument's own keys lose nothing: they are monotone
    // counts of a histogram no other publisher writes.
    let callback = crate::hook::callback_latency();
    state.callback_samples = callback.samples;
    state.callback_p50_ns = callback.p50_ns;
    state.callback_p99_ns = callback.p99_ns;
    state.callback_max_ns = callback.max_ns;

    state
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
/// The two `watchdog_` lines of task **T-06-2** were appended for the same reason and nowhere
/// else — not beside `hook_installed`, where they belong by subject. They are the counter of
/// FR-80 and the reason the hook was last put back, and they are here rather than inside the
/// program because decision Р-37 says what the bench of section 11.5 needs to confirm position
/// 18 has to leave the process somehow.
///
/// `password_field` of task **T-06-1** is appended for the same reason, and it is the key
/// SEC-04a was written for: footnote 2 of section 11.3 makes position 14 provable through it and
/// through nothing else. It is a flag — `0` or `1` — and never the content of the field.
///
/// `focus_changes` and `password_probes` of task **T-06-1a** are appended after it, and they are
/// what makes that flag *readable*: one bit cannot say whose window it describes, and a bench
/// that cannot tell a fresh verdict from a stale one cannot confirm SEC-06 at all — see
/// [`Snapshot::focus_changes`].
///
/// `fail_safe` and `consecutive_panics` of task **T-08-3** are appended after them, for the same
/// reason as everything above them, and they exist for one job: point 5 of that task's exclusion
/// list asks whether FR-99 had already disarmed the program in the state under investigation, and
/// there is no other way to ask that from outside the process.
///
/// `device_changes` of task **T-08-4** is appended after them, and it is the one number that shows
/// FR-21 still being delivered after that task replaced the mechanism which delivered it. See
/// [`Snapshot::device_changes`].
///
/// `background_skips` of task **T-10-0** is appended after them, by the same rule, and it is the
/// number that makes the repair of the acceptance defect *observable*: a staged storm that
/// reaches the product moves it, and `window_flushes` — read from `watchdog::counters()` by the
/// file sink — standing still beside it is the repair. See [`Snapshot::background_skips`].
///
/// `focus_repeats` of task **T-10-0e** is appended by the same rule, and it is the same shape
/// one layer in: the churn of the *frontmost* window passes the gate of T-10-0 legitimately,
/// and this is the number that shows it being turned away by the focus memory instead of
/// erasing the user's typing. See [`Snapshot::focus_repeats`].
///
/// `layout_probes` of task **T-10-0f** is appended last, again by the same rule: it is the
/// number whose refusal to grow beside `focus_changes` was the defect, and whose growth beside
/// it is the repair. See [`Snapshot::layout_probes`].
///
/// The four `callback_` keys of task **T-10-1** are appended after it, by the same rule and
/// for the oldest reason on this channel: criterion 2 of §13 requires the callback's latency
/// *measured* on ten thousand keystrokes, the duration of a hook callback is not observable
/// from outside the process even in principle (the argument of решение Р-37), and position 23
/// of §11.3 takes its three verdicts from these numbers. Samples first, then the median, the
/// 99th percentile and the exact maximum, all in nanoseconds — see
/// [`Snapshot::callback_samples`] through [`Snapshot::callback_max_ns`].
///
/// `active_layout` of task **T-10-5** is appended last, by the same rule, and it is the first
/// key on this channel that is a *register* rather than a count: FR-26 computes the direction
/// of every conversion from the stamp, and no count of probes, rebuilds or handoffs can say
/// what value the stamp ended up holding. See [`Snapshot::active_layout`].
///
/// `last_replacement` of task **T-10-6** and `last_replacement_method` of task **T-10-8** are
/// appended after it, in that order and each by the same rule. The first is the shape of the
/// packet the last press built; the second is which of the two packets it was — the decision
/// FR-42а makes per press against the class of the foreground window, which no configured
/// value can stand in for now that the configured value may be `auto`. See
/// [`Snapshot::last_replacement_method`].
///
/// `window_flushes`, `full_clears` and `strokes_removed` of task **T-10-9** are appended after
/// them, in that order and by the same rule, and they arrive here for the reason `focus_repeats`
/// and `layout_probes` did — one step further. Those two were counters of `watchdog` whose
/// *growth* was the observable of a repair; these three are counters of `watchdog` that were
/// published through the file sink of `LANGSW_TESTING_REPORT` **only**, and that sink is written
/// after the process exits. A scenario whose whole question is what happens to a **surviving**
/// process therefore could not read them at all, and they are precisely the three numbers that
/// tell «набранное стирается сбросами» from «набранное не попадает в буфер». See
/// [`Snapshot::window_flushes`].
///
/// `field_state` of task **T-10-10** is appended after them, by the same rule, and it is the
/// blind spot those three left standing. They separate «буфер стирают сбросы» from «в буфер
/// ничего не попадает»; they cannot say **why** nothing is going in, because `password_field`
/// prints `0` both for an ordinary field and for a gate still waiting on a verdict with
/// buffering off. This key names the gate's own state in one of four closed words, so a gate
/// that never leaves `pending` is readable directly rather than by inference. See
/// [`Snapshot::field_state`].
///
/// The three `clipboard_` keys of task **T-10-11** are appended after it, in that order and by
/// the same rule, and they close the last unlit corner of the four-row break-down table. Every
/// key before them describes the path *into* the buffer or the state of the gate above it; the
/// fourth row — «замена отказывает» — had only `last_replacement=0/0/0`, which says a
/// replacement did not run and never why. FR-42а resolves `auto` to `selection` for every
/// non-console window, so that path carries almost everything the user types, and it went
/// through `OpenClipboard`/`CloseClipboard` with **not one number** leaving the process. These
/// three mirror `selection::counters()`: the refusal, the close that failed — the one failure
/// that lasts, because a clipboard left open refuses everybody — and the retry beside them as
/// the gradient. See [`Snapshot::clipboard_refusals`].
///
/// ⭐ The five `restamp_` keys of task **T-10-15** are appended after them, in the order of
/// [`Restamp::ALL`] and by the same rule every key above them followed: they are the outcome of a
/// decision the program makes on the hook path, and until they existed the only reading of that
/// decision was `active_layout` **after** it — which is the same value for four of the five
/// outcomes. Their whole point is that one snapshot separates «перештамповка не звалась» from
/// «звалась и отказала, потому что карты нет в кэше FR-20», and those are different defects in
/// different modules. See [`RESTAMP_OUTCOMES`].
///
/// ⭐ `last_replacement_changed` and `last_replacement_direction` of task **T-10-17** are appended
/// after them, in that order and by the same rule, and they are the pair the live protocol of
/// defect E was missing. Every key of the `last_replacement` family before them describes the
/// *size* of a press — six off, six back, six of them different, through which packet — and all
/// of them read identically whether the press rewrote the word or handed back the one that was
/// already there. The first of the two is that missing bit; the second says which direction the
/// press applied, so that a `X→X` press — the rollback `Cycle::target` produces whenever
/// `step % len == 0`, a **legitimate** identity — can be told apart from an identity nobody asked
/// for. See [`LAST_REPLACEMENT_CHANGED`] and [`LAST_REPLACEMENT_DIRECTION`].
///
/// ⭐ `focus_after_typing` of task **Т-48-2** is appended last, by the same rule: it is the
/// observable of **FR-14**, the rule that stopped the suggestion list of an address bar from
/// cutting the user's word in half, and it is the number that separates the rule not firing from
/// the rule firing where it should not. See [`Snapshot::focus_after_typing`].
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
         cycle_position={}\n\
         watchdog_recoveries={}\n\
         watchdog_last_reason={}\n\
         password_field={}\n\
         focus_changes={}\n\
         password_probes={}\n\
         fail_safe={}\n\
         consecutive_panics={}\n\
         device_changes={}\n\
         background_skips={}\n\
         focus_repeats={}\n\
         layout_probes={}\n\
         callback_samples={}\n\
         callback_p50_ns={}\n\
         callback_p99_ns={}\n\
         callback_max_ns={}\n\
         active_layout={:#010x}\n\
         last_replacement={}/{}/{}\n\
         last_replacement_method={}\n\
         window_flushes={}\n\
         full_clears={}\n\
         strokes_removed={}\n\
         field_state={}\n\
         clipboard_refusals={}\n\
         clipboard_close_failures={}\n\
         clipboard_retries={}\n\
         restamp_skips={}\n\
         restamp_no_probe={}\n\
         restamp_unchanged={}\n\
         restamp_uncached={}\n\
         restamp_accepted={}\n\
         last_replacement_changed={}\n\
         last_replacement_direction={:#010x}/{:#010x}\n\
         focus_after_typing={}\n",
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
        state.watchdog_recoveries,
        state.watchdog_last_reason.name(),
        u8::from(state.password_field),
        state.focus_changes,
        state.password_probes,
        u8::from(state.fail_safe),
        state.consecutive_panics,
        state.device_changes,
        state.background_skips,
        state.focus_repeats,
        state.layout_probes,
        state.callback_samples,
        state.callback_p50_ns,
        state.callback_p99_ns,
        state.callback_max_ns,
        state.active_layout,
        state.replacement_erase,
        state.replacement_units,
        state.replacement_distinct,
        state.last_replacement_method,
        state.window_flushes,
        state.full_clears,
        state.strokes_removed,
        state.field_state.name(),
        state.clipboard_refusals,
        state.clipboard_close_failures,
        state.clipboard_retries,
        state.restamp[Restamp::Skipped as usize],
        state.restamp[Restamp::NoProbe as usize],
        state.restamp[Restamp::Unchanged as usize],
        state.restamp[Restamp::Uncached as usize],
        state.restamp[Restamp::Accepted as usize],
        u8::from(state.last_replacement_changed),
        state.last_replacement_from,
        state.last_replacement_to,
        state.focus_after_typing,
    )
}

/// `[replacement] method` as section 7 spells it — FR-42, FR-42а.
///
/// The three words of the configuration file and not the Rust spelling of the enum: the bench
/// compares this against what it wrote into `config.toml`.
pub const fn method_name(method: ReplacementMethod) -> &'static str {
    match method {
        ReplacementMethod::Auto => "auto",
        ReplacementMethod::Backspace => "backspace",
        ReplacementMethod::Selection => "selection",
    }
}

/// The keys [`render`] emits, in the order it emits them.
///
/// Exported so that a check of condition 2 of SEC-04a can assert the set exactly rather than
/// merely look for what it expects: a key that appeared here without being listed would be a
/// key nobody reviewed.
pub const KEYS: [&str; 46] = [
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
    "watchdog_recoveries",
    "watchdog_last_reason",
    "password_field",
    "focus_changes",
    "password_probes",
    "fail_safe",
    "consecutive_panics",
    "device_changes",
    "background_skips",
    "focus_repeats",
    "layout_probes",
    "callback_samples",
    "callback_p50_ns",
    "callback_p99_ns",
    "callback_max_ns",
    "active_layout",
    "last_replacement",
    "last_replacement_method",
    "window_flushes",
    "full_clears",
    "strokes_removed",
    "field_state",
    "clipboard_refusals",
    "clipboard_close_failures",
    "clipboard_retries",
    "restamp_skips",
    "restamp_no_probe",
    "restamp_unchanged",
    "restamp_uncached",
    "restamp_accepted",
    "last_replacement_changed",
    "last_replacement_direction",
    "focus_after_typing",
];

/// Keys SEC-04a reserves and this build does not answer — see [`KEYS`] and the module
/// documentation.
///
/// **Empty: every key SEC-04a reserved is published now.** `cycle_position` arrived with task
/// T-05-2a and `password_field` with task T-06-1, and each moved from here into [`KEYS`] in the
/// same change that started emitting it.
///
/// The constant stays rather than being deleted, because it is what the reservation *means* that
/// is worth keeping: a key named here is one [`render`] must not emit and [`KEYS`] must not
/// list, and `tests\control.rs` asserts both of those over whatever this array holds. Empty, that
/// pair of assertions is vacuous and costs nothing; the day a key is reserved again it is one
/// entry here and no new test.
pub const RESERVED_KEYS: [&str; 0] = [];

// ---------------------------------------------------------------------------------------
// Where every value comes from — condition 2 of SEC-04a, finding С66, task T-41-10
// ---------------------------------------------------------------------------------------

/// What kind of thing the value behind one key of [`KEYS`] is — finding С66, task T-41-10.
///
/// Condition 2 of SEC-04a says the channel may carry **state and nothing else**, and it names
/// examples: the length of the buffer, the «password field» verdict, the position in the cycle,
/// counts of refusals. Examples are not a list, and the finding is about exactly that gap: two of
/// the values that go out are computed **from the typed word**, the letter of condition 2 names
/// no such category, and the test that guards the channel looks for «как бы набранный» text and
/// sees no derived number at all.
///
/// So every key is given its origin by name. The acceptance refuses a key whose origin is
/// [`Origin::DerivedFromText`] unless that key is in a short list written out in the test — which
/// is what «закрытый список» means here: not that nothing derived may go out, but that nothing
/// derived may go out **quietly**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A count of this program's own events: presses, refusals, retries, recoveries. Says how
    /// often something happened and nothing about what it was.
    Count,
    /// A duration or a quantile this program measured of itself — microseconds, nanoseconds.
    Measurement,
    /// A configuration value this program published at start-up or on «Применить». It came from
    /// the person's own file and goes back out unchanged.
    Setting,
    /// A state read at the instant of the snapshot: a flag, a layout handle, a position, a word
    /// from a closed vocabulary. Not a count and not a setting, and nothing of the text.
    State,
    /// ⚠⚠ **Computed from the typed word**, and permitted only by being named here.
    ///
    /// Two keys carry such a value, both about the shape of a replacement packet and neither
    /// about its content: how many of the units in it were **distinct** (inside
    /// `last_replacement`), and whether the text typed back **equalled** the text erased
    /// (`last_replacement_changed`). Решение 125.1 kept both — a stand that reads them cannot
    /// recover a character, a code unit or a scan code from either, and the value of the finding
    /// is the instrument rather than the removal.
    DerivedFromText,
}

/// Every key of [`KEYS`] with the origin of its value — finding С66, task T-41-10.
///
/// **In the order of [`KEYS`], name for name**, because the acceptance walks the two side by side:
/// a key that appeared in one and not the other is a key nobody reviewed, and the silent growth
/// 45 → 46 the finding was written about is exactly that. The count is written out as a literal
/// rather than taken from `KEYS.len()` on purpose — a table whose length follows the thing it
/// checks cannot disagree with it.
pub const ORIGINS: [(&str, Origin); 46] = [
    ("buffer_len", Origin::Count),
    ("hook_installed", Origin::State),
    ("hook_ready_us", Origin::Measurement),
    ("cache_ready_us", Origin::Measurement),
    ("cache_builds", Origin::Count),
    ("layout_cache_failures", Origin::Count),
    ("hotkey_handoffs", Origin::Count),
    ("post_failures", Origin::Count),
    ("send_mismatches", Origin::Count),
    ("events_lost", Origin::Count),
    ("inter_event_delay_ms", Origin::Setting),
    ("replacement_method", Origin::Setting),
    ("cycle_position", Origin::State),
    ("watchdog_recoveries", Origin::Count),
    ("watchdog_last_reason", Origin::State),
    ("password_field", Origin::State),
    ("focus_changes", Origin::Count),
    ("password_probes", Origin::Count),
    ("fail_safe", Origin::State),
    ("consecutive_panics", Origin::Count),
    ("device_changes", Origin::Count),
    ("background_skips", Origin::Count),
    ("focus_repeats", Origin::Count),
    ("layout_probes", Origin::Count),
    ("callback_samples", Origin::Count),
    ("callback_p50_ns", Origin::Measurement),
    ("callback_p99_ns", Origin::Measurement),
    ("callback_max_ns", Origin::Measurement),
    ("active_layout", Origin::State),
    // ⚠ The first of the two. `erase` and `units` are counts; `distinct` is computed from the
    // word — see [`LAST_REPLACEMENT`] for why task T-10-6 needed it.
    ("last_replacement", Origin::DerivedFromText),
    ("last_replacement_method", Origin::State),
    ("window_flushes", Origin::Count),
    ("full_clears", Origin::Count),
    ("strokes_removed", Origin::Count),
    ("field_state", Origin::State),
    ("clipboard_refusals", Origin::Count),
    ("clipboard_close_failures", Origin::Count),
    ("clipboard_retries", Origin::Count),
    ("restamp_skips", Origin::Count),
    ("restamp_no_probe", Origin::Count),
    ("restamp_unchanged", Origin::Count),
    ("restamp_uncached", Origin::Count),
    ("restamp_accepted", Origin::Count),
    // ⚠ The second. One bit: whether the text typed back equalled the text erased.
    ("last_replacement_changed", Origin::DerivedFromText),
    ("last_replacement_direction", Origin::State),
    ("focus_after_typing", Origin::Count),
];
