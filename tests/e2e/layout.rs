//! Keyboard layouts: the second assertion of every scenario, and footnote 3 of §11.3.
//!
//! The scenario of §11.3 asserts two things — that the text became `привет` **and** that the
//! active layout of the window became RU. This module answers the second question the way
//! FR-52 specifies, and it settles the bench's position on footnote 3 — see the note at the
//! bottom of the file.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

/// `en-US`.
pub const US: u32 = 0x0000_0409;

/// `ru-RU`.
pub const RUSSIAN: u32 = 0x0000_0419;

/// The low word of an `HKL` is the language identifier; the whole value is what §7 of SPEC
/// writes as `0x00000409`.
pub fn id_of(hkl: HKL) -> u32 {
    hkl.0 as usize as u32
}

/// Human-readable form of a layout id, for the report.
pub fn describe(id: u32) -> String {
    let name = match id & 0xFFFF {
        0x0409 => "en-US",
        0x0419 => "ru-RU",
        _ => "?",
    };
    format!("0x{id:08X} ({name})")
}

/// The layout active in the thread that owns `hwnd` — **FR-52, verbatim**.
///
/// `GetKeyboardLayout(GetWindowThreadProcessId(hwnd, null))`. This is the per-window answer,
/// which is the one the assertion is about: FR-51 notes that Windows 11 keeps a layout per
/// application window by default, so asking about the bench's own thread would answer a
/// different question.
pub fn of_window(hwnd: HWND) -> Option<HKL> {
    // SAFETY: `hwnd` is a handle the caller obtained from the system moments earlier. The
    // second argument is optional and `None` is passed, as FR-52 writes it. NFR-13: a zero
    // thread id means the window has died, and it is examined rather than passed on to
    // `GetKeyboardLayout`, which would answer about the calling thread instead.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, None) };
    if thread == 0 {
        return None;
    }

    // SAFETY: `thread` is a live thread id just obtained. `GetKeyboardLayout` returns the
    // layout handle of that thread, or of the calling thread for a zero id — which is why the
    // zero was rejected above.
    Some(unsafe { GetKeyboardLayout(thread) })
}

/// Every layout currently attached to this session.
///
/// Called before and after a run so that "temporary layouts were detached" is a comparison of
/// two observations rather than an assurance.
pub fn attached() -> Vec<u32> {
    // SAFETY: passing `None` asks only for the count, which is the documented use of a null
    // buffer. Nothing is written anywhere.
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count <= 0 {
        return Vec::new();
    }

    let mut list = vec![HKL::default(); count as usize];
    // SAFETY: `list` is a live, correctly sized buffer and its length bounds the write; the
    // system fills at most `count` entries. NFR-13: the second return is used as the real
    // length rather than trusting the first.
    let written = unsafe { GetKeyboardLayoutList(Some(&mut list)) };

    list.into_iter()
        .take(written.max(0) as usize)
        .map(id_of)
        .collect()
}

/// The same list as [`attached`], but as handles.
///
/// Handles are what `WM_INPUTLANGCHANGEREQUEST` needs, and taking them from the system's own
/// list is what keeps [`ensure`] from ever *installing* a layout: it can only ask for one that
/// is already there. Installing a layout on somebody's machine to make a test pass is not the
/// bench's business.
pub fn attached_handles() -> Vec<HKL> {
    // SAFETY: as in `attached` — a null buffer asks only for the count.
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count <= 0 {
        return Vec::new();
    }

    let mut list = vec![HKL::default(); count as usize];
    // SAFETY: as in `attached` — the buffer is live and its length bounds the write.
    let written = unsafe { GetKeyboardLayoutList(Some(&mut list)) };

    list.truncate(written.max(0) as usize);
    list
}

/// The handle of an already-attached layout with this language id, if there is one.
pub fn handle_for(language: u32) -> Option<HKL> {
    attached_handles()
        .into_iter()
        .find(|hkl| id_of(*hkl) & 0xFFFF == language & 0xFFFF)
}

/// `WM_INPUTLANGCHANGEREQUEST`.
const WM_INPUTLANGCHANGEREQUEST: u32 = 0x0050;

/// Asks the window to switch to `hkl` and waits until FR-52 says it did.
///
/// ⚠ This is the bench arranging a **precondition**, not doing the product's work. The scenario
/// of §11.3 is "type `ghbdtn` **in the English layout**, press the hotkey"; putting the window
/// into the English layout first is setting the stage, and the assertion it enables — that the
/// layout afterwards became RU — is one the bench then **asserts as a verdict**, because the
/// switch exists: task T-05-1 wrote the chain of FR-50 to FR-52 (commit `c8e463f`) and task
/// T-05-2 gave it the target of §4.4. The bench sets the stage and reads the result; it never
/// performs the switch the assertion is about.
///
/// The same message is the first link of the FR-50 chain, which is a coincidence of mechanism,
/// not of purpose: there is no other documented way to ask a foreign window to change layout.
pub fn ensure(hwnd: HWND, language: u32, timeout: std::time::Duration) -> Result<u32, String> {
    if let Some(current) = of_window(hwnd)
        && id_of(current) & 0xFFFF == language & 0xFFFF
    {
        return Ok(id_of(current));
    }

    let Some(wanted) = handle_for(language) else {
        return Err(format!(
            "раскладка {} не подключена в этом сеансе; стенд её не устанавливает",
            describe(language)
        ));
    };

    // SAFETY: `hwnd` is a live window handle from the caller and `wanted` a layout handle from
    // the system's own list. `PostMessage` copies the message into the target queue and
    // dereferences nothing of ours. NFR-13: the result is examined below.
    let posted = unsafe {
        windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(hwnd),
            WM_INPUTLANGCHANGEREQUEST,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(wanted.0 as isize),
        )
    };
    if let Err(error) = posted {
        return Err(format!("PostMessage(WM_INPUTLANGCHANGEREQUEST): {error}"));
    }

    let settled = crate::wait::until(timeout, || {
        of_window(hwnd).filter(|hkl| id_of(*hkl) & 0xFFFF == language & 0xFFFF)
    });

    match settled {
        Some(hkl) => Ok(id_of(hkl)),
        None => Err(format!(
            "окно не перешло в раскладку {} за {} с; сейчас {}",
            describe(language),
            timeout.as_secs(),
            of_window(hwnd).map_or("неизвестно".to_owned(), |h| describe(id_of(h)))
        )),
    }
}
// ---------------------------------------------------------------------------------------
// Footnote 3 of §11.3 — the temporary third layout
// ---------------------------------------------------------------------------------------
//
// Footnote 3 allows position 17 to attach a third layout with `LoadKeyboardLayout` and
// `KLF_ACTIVATE`, and requires it detached afterwards. **This bench attaches none**, and there
// is no code here that could: position 17 is the only position that needs one, and the bench
// cannot prepare the rest of that position's environment either — writing `config.toml` with
// `mode = "cycle"` and three layouts, which it only ever reads. The "Цикл" mode of FR-31 itself
// **exists**, written by task T-05-2 and covered by the automated tests of `tests\cycle.rs`;
// what is missing is the bench's ability to set the stage, and that is task T-04-3-3's, which is
// where position 17's `pending` now points. Attaching a keyboard layout to somebody's machine
// before that ability exists would be doing harm for no information.
//
// The requirement is therefore satisfied by measurement rather than by mechanism: `attached()`
// is read before the run and after it, and the run reports whether the two lists are identical.
// That check would also catch a layout attached by accident, which a `Drop` on a type the bench
// happens to construct would not.
