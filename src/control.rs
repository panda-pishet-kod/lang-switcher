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
//! acceptance criterion 8 of section 13 of SPEC — task **T-03-4**.
//! Implemented by backlog tasks: T-03-4.
//!
//! # The four conditions of SEC-04a, and where each of them is met
//!
//! 1. **Compiled only under `testing`.** The whole module is behind
//!    `#[cfg(feature = "testing")] pub mod control;` in `src\lib.rs`, the feature is absent
//!    from the Release configuration, and the length mirror this module reads in
//!    [`crate::buffer`] carries the same gate. Nothing here has a counterpart outside it.
//! 2. **Metadata only.** [`Snapshot`] is the whole of what leaves this module, and every
//!    field of it is a count, a duration or a published configuration value. There is no
//!    field for a stroke, a key code, a scan code or a character, and none may ever be added:
//!    SEC-01 and SEC-07 forbid it, and `buffer::Stroke` has neither `Debug` nor `Display`, so
//!    the content is not even expressible here by mistake. `buffer_len` is a number, which is
//!    exactly what condition 2 of SEC-04a names as the one thing allowed out.
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
//! [`render`] emits a key only when the value behind it exists. Two keys of SEC-04a are
//! therefore **missing from the output rather than published as zero** — a stand that reports
//! a fabricated zero would count it as the truth:
//!
//! * `password_field` — SEC-06, FR-70 to FR-73, position 14 of the matrix of section 11.3.
//!   Task **T-06-1**. This is the key SEC-04a exists for at all.
//! * `cycle_position` — FR-32, FR-33. Task **T-05-2**.
//!
//! Both are one line each in [`Snapshot`] and one line each in [`render`] when their owning
//! task arrives; neither the format nor the server has to change to accept them.
//!
//! # ⛔ The pipe server is not here, and why
//!
//! Task T-03-4 was to put a named pipe over this snapshot. Three of the items it needs are
//! outside the closed feature list of section 3.2 of SPEC, in `windows 0.62.2`:
//!
//! | Item | Feature it is gated behind |
//! |---|---|
//! | `CreateNamedPipeW` | `Win32_Security` **and** `Win32_Storage_FileSystem` |
//! | `PIPE_ACCESS_OUTBOUND` (a `FILE_FLAGS_AND_ATTRIBUTES`) | `Win32_Storage_FileSystem` |
//! | `ConnectNamedPipe` (an `*mut OVERLAPPED` parameter) | `Win32_System_IO` |
//!
//! `Win32_System_Pipes` pulls in neither. Section 3.2 states the opposite, and the list is
//! closed by decision of the user as the enforcement mechanism behind SEC-03, so adding the
//! two missing features is not this task's to do. Everything the server would stand on is
//! here and is tested — the name ([`PIPE_NAME_PREFIX`], [`pipe_name`]), the descriptor
//! ([`OwnerOnly`]) and the payload ([`snapshot`], [`render`]).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::Instant;

use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, GENERIC_READ, HANDLE};
use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_REVISION, ACL_SIZE_INFORMATION, AclSizeInformation,
    AddAccessAllowedAce, EqualSid, GetAce, GetAclInformation, GetLengthSid,
    GetSecurityDescriptorDacl, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
    SetSecurityDescriptorDacl, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcessToken};
use windows::core::Result as WinResult;

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
    //
    // Deliberately absent until their owning task exists, so that the bench cannot mistake a
    // fabricated zero for an answer:
    //   password_field — SEC-06, FR-70..FR-73, position 14 of section 11.3. Task T-06-1.
    //   cycle_position — FR-32, FR-33. Task T-05-2.
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
    }
}

/// The channel's payload: one `key=value` per line, ASCII, newline-terminated.
///
/// The order is the order of the table in task T-03-4. A reader that parses line by line does
/// not depend on it; a human diffing two runs does.
///
/// `password_field` and `cycle_position` are **not** here. See the module documentation: they
/// arrive with tasks T-06-1 and T-05-2 respectively, one line each, and until then their
/// absence is the honest answer.
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
         replacement_method={}\n",
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
pub const KEYS: [&str; 12] = [
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
];

/// Keys SEC-04a reserves and this build does not answer — see [`KEYS`] and the module
/// documentation. `password_field` is task **T-06-1**, `cycle_position` task **T-05-2**.
pub const RESERVED_KEYS: [&str; 2] = ["password_field", "cycle_position"];
