//! Keyboard layouts: the second assertion of every scenario, the **ambient layout** of the
//! session, and footnote 3 of §11.3.
//!
//! The scenario of §11.3 asserts two things — that the text became `привет` **and** that the
//! active layout of the window became RU. This module answers the second question the way
//! FR-52 specifies, and it settles the bench's position on footnote 3 — see the note at the
//! bottom of the file.
//!
//! # The ambient layout, and why it is a resource of the run — task T-04-3-3
//!
//! A window created on Windows 11 does not choose its layout: it is given the one the session
//! last activated, which is the layout of the window that was in front (FR-51 — "a layout per
//! application window"). Measured with `langsw-e2e --measure-layout`:
//!
//! | Measurement | Answer |
//! |---|---|
//! | does the system dialog `#32770` honour `WM_INPUTLANGCHANGEREQUEST`? | no |
//! | does it honour `AttachThreadInput` + `ActivateKeyboardLayout`? | no — that moves the **calling** thread |
//! | does a new window inherit the layout of the thread that spawned its process? | no |
//! | does a new window inherit the layout of the window that was in front? | **yes** |
//!
//! So the ambient layout is not a detail of the machine the bench may read and forget: it is
//! **state the bench changes**, because every position asks its own window into en-US and the
//! product then switches it to RU, and it is **state the next position inherits**. Requirement 5
//! of §11.5 — "возвращает раскладку" — is therefore about this value, and [`Ambient`] is what
//! makes it settable at all without touching a window of anybody else's.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, GetKeyboardLayoutList, HKL, KLF_ACTIVATE,
    LoadKeyboardLayoutW, UnloadKeyboardLayout,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetForegroundWindow, GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, RegisterClassW,
    SW_SHOWNORMAL, SetForegroundWindow, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, w};

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
        // The third layout of footnote 3, attached for position 17 and detached after it. Named
        // here so that the report of that position reads as a layout rather than as a `?`.
        0x0407 => "de-DE",
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

/// The layout of the bench's own thread.
///
/// Not an assertion about anything: it is the number that says what a *newly created* window of
/// this session is likely to start in, and the measurement of position 11 turns on exactly that.
pub fn of_this_thread() -> HKL {
    // SAFETY: a zero thread id is the documented way to ask about the calling thread, which is
    // the question here. Nothing is written anywhere.
    unsafe { GetKeyboardLayout(0) }
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
/// The **second** link of the FR-50 chain, used as a precondition: `AttachThreadInput`,
/// `ActivateKeyboardLayout`, `DetachThreadInput`.
///
/// ⛔ **Requirement E of the A–E protection, enforced here rather than assumed.** Attaching this
/// thread's input queue to another thread's is not a message that a well-behaved window may
/// ignore — it joins the two queues — so it is refused outright for any process the bench did not
/// start. `own::is_ours` is the same gate `input::send_verified` and `shell::activate_window`
/// pass, and it is asked before the window handle is used for anything at all.
///
/// Why it exists: `PostMessage(WM_INPUTLANGCHANGEREQUEST)` is a *request*, and a window that
/// pumps its messages through a system-provided procedure need not honour it. [`ensure`] tries the
/// request first because it is the gentler of the two; this is what the FR-50 chain itself does
/// next, on the bench's own window and for the same reason.
pub fn attach_activate(
    hwnd: HWND,
    pid: u32,
    language: u32,
    timeout: std::time::Duration,
) -> Result<u32, String> {
    if !crate::own::is_ours(pid) {
        return Err(format!(
            "⛔ процесс {pid} не в реестре A — стенд не присоединяет очередь ввода чужого потока"
        ));
    }

    let Some(wanted) = handle_for(language) else {
        return Err(format!(
            "раскладка {} не подключена в этом сеансе",
            describe(language)
        ));
    };

    // SAFETY: `hwnd` belongs to a process already proved to be the bench's own. `None` for the
    // process id is documented as "do not report it". NFR-13: a zero thread id means the window
    // has died and is rejected instead of being passed on.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, None) };
    if thread == 0 {
        return Err("у окна не читается поток — окно уже закрыто".to_owned());
    }

    // SAFETY: `GetCurrentThreadId` takes nothing and cannot fail.
    let ours = unsafe { GetCurrentThreadId() };

    // SAFETY: both thread ids are live — one just read from the window, one our own. The
    // attachment is undone below on every path out of this function. NFR-13: the result is
    // examined, and a failed attachment still lets the activation be tried, because
    // `ActivateKeyboardLayout` acting on our own thread alone is what a session without the
    // per-window setting of FR-51 needs anyway.
    let attached = unsafe { AttachThreadInput(ours, thread, true) }.as_bool();

    // SAFETY: `wanted` is a layout handle taken from the system's own list, so nothing is
    // installed here. `KLF_ACTIVATE` is the flag FR-50 step 2 names. NFR-13: examined below.
    let activated = unsafe { ActivateKeyboardLayout(wanted, KLF_ACTIVATE) };

    if attached {
        // SAFETY: undoes exactly the attachment made above, with the same two ids.
        let _ = unsafe { AttachThreadInput(ours, thread, false) };
    }

    if let Err(error) = activated {
        return Err(format!(
            "ActivateKeyboardLayout: {error} (присоединение очереди: {attached})"
        ));
    }

    let settled = crate::wait::until(timeout, || {
        of_window(hwnd).filter(|hkl| id_of(*hkl) & 0xFFFF == language & 0xFFFF)
    });

    match settled {
        Some(hkl) => Ok(id_of(hkl)),
        None => Err(format!(
            "окно не перешло в раскладку {} за {} с даже после ActivateKeyboardLayout \
             (присоединение очереди: {attached}); сейчас {}",
            describe(language),
            timeout.as_secs(),
            of_window(hwnd).map_or("неизвестно".to_owned(), |h| describe(id_of(h)))
        )),
    }
}

// ---------------------------------------------------------------------------------------
// The ambient layout of the session — requirement 5 of §11.5, task T-04-3-3
// ---------------------------------------------------------------------------------------

/// A window of the bench's own — **the instrument that reads the ambient layout, and the one
/// that was measured not to be able to write it.**
///
/// # What it is used for today
///
/// [`ambient`]: a window is created, asked what layout the session gave it, and destroyed. That
/// is the reading, and it is exact — the question "what will the next window be born in" is
/// answered by being a next window.
///
/// # ⚠ What it is **not** used for, and the measurement that decided it
///
/// [`Ambient::set`] does work on the window: `WM_INPUTLANGCHANGEREQUEST` is honoured by
/// `DefWindowProcW`, the window really moves, and `raise` really makes `GetForegroundWindow`
/// return it. And it changes **nothing** about what the next window is born with — measured
/// three times, with `WS_POPUP` and with `WS_OVERLAPPEDWINDOW`, with the anchor held and with
/// the anchor freshly opened:
///
/// ```text
/// окружающая раскладка выставлена в 0x04090409 (en-US)
/// в момент запуска диалога: якорь в 0x04090409 (en-US), активен: true
/// проба свежим окном: 0x04190419 (ru-RU)      <-- the next window, born in the old layout
/// ```
///
/// An **application** window of another process, moved with the same message, does change it —
/// `scenarios::stage_ambient`. So the write goes through an application and the read stays here.
/// `set` is kept because it is what makes that comparison reproducible: it is called by
/// `--measure-layout`, where its negative result is part of the output.
///
/// ⛔ Both instruments are equally within requirements A to E, and the choice between them was
/// decided by measurement and not by which one felt safer. Broadcasting
/// `WM_INPUTLANGCHANGEREQUEST` to `HWND_BROADCAST` would have reached every window on the
/// desktop, and that one is refused on principle rather than on results.
///
/// # Requirement 1 of §11.5
///
/// The wait after the request is on a condition — "the window reports the layout asked for" —
/// and the message pump inside it is what makes the condition reachable: `DefWindowProcW` is
/// what carries out `WM_INPUTLANGCHANGEREQUEST`, and it only runs when the message is
/// dispatched.
pub struct Ambient {
    hwnd: HWND,
}

/// The window class of [`Ambient`]. One per process; the name is the bench's own.
const AMBIENT_CLASS: PCWSTR = w!("LangSwE2eAmbientAnchor");

/// Registers the class of [`AMBIENT_CLASS`], once for the life of the process.
///
/// ⚠ **Once, and never unregistered.** A window class is per process, not per window: a second
/// `RegisterClassW` of the same name fails with `ERROR_CLASS_ALREADY_EXISTS`, which is how the
/// first version of this code broke the moment anything opened a second anchor. Unregistering
/// it in `Drop` would be worse than useless — it would fail while another anchor still existed
/// and succeed only to be redone — so the class is registered on first use and left to die with
/// the process, which is what happens to every window class of every program.
fn ambient_class() -> Result<(), String> {
    static REGISTERED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();

    REGISTERED
        .get_or_init(|| {
            // SAFETY: `None` asks for the handle of the running executable, which is the
            // documented use. NFR-13: the result is examined.
            let instance = unsafe { GetModuleHandleW(None) }
                .map_err(|error| format!("GetModuleHandleW: {error}"))?;

            let class = WNDCLASSW {
                lpfnWndProc: Some(ambient_proc),
                hInstance: instance.into(),
                lpszClassName: AMBIENT_CLASS,
                ..Default::default()
            };

            // SAFETY: `class` is a live, fully initialised `WNDCLASSW` whose two pointers — the
            // window procedure and the static class name — outlive the call and the class.
            // NFR-13: a zero atom is a failure and is examined.
            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 {
                return Err(format!(
                    "RegisterClassW: {}",
                    std::io::Error::last_os_error()
                ));
            }

            Ok(())
        })
        .clone()
}

impl Ambient {
    /// Creates the anchor. `Err` means the session's layout cannot be set at all, which is a
    /// finding for the report and never an assumption that it was set anyway.
    pub fn open() -> Result<Self, String> {
        ambient_class()?;

        // SAFETY: `None` asks for the handle of the running executable — the same one the class
        // was registered under. NFR-13: the result is examined.
        let instance = unsafe { GetModuleHandleW(None) }
            .map_err(|error| format!("GetModuleHandleW: {error}"))?;

        // SAFETY: every argument is a plain value or a null option, and the class name is the
        // registered one. NFR-13: the `Result` is examined.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                AMBIENT_CLASS,
                w!("langsw-e2e"),
                // ⚠ **An ordinary overlapped window, and the style is load-bearing.** A
                // `WS_POPUP` of one pixel *is* returned by `GetForegroundWindow` once it is
                // raised — and a `#32770` dialog created behind it still opened in the old
                // layout. Measured, twice. What a newly created process inherits is the input
                // context of the **active application window**, and a caption-less unowned popup
                // is not one of those. This is the same shape every other window of the matrix
                // has, which is the shape that was measured to work.
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                160,
                60,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .map_err(|error| format!("CreateWindowExW: {error}"))?;

        // SAFETY: `hwnd` is the window just created and owned by this thread. `ShowWindow`
        // returns the previous visibility, which carries no failure to examine.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };

        Ok(Self { hwnd })
    }

    /// Brings the anchor forward and puts the session into `language` — the one write.
    ///
    /// ⚠ **Being in front is half of the mechanism and is therefore checked, not hoped for.**
    /// The layout of a *window* is one thing; the layout the session hands to the **next** window
    /// created is the layout of the window that is **active**. An anchor that changed its own
    /// layout without ever taking the foreground would report success and change nothing that
    /// matters, which is exactly what the first version of this function did.
    pub fn set(&self, language: u32, timeout: std::time::Duration) -> Result<u32, String> {
        let front = self.raise(timeout);

        // The request, and then a wait that **pumps**: `WM_INPUTLANGCHANGEREQUEST` is carried out
        // by `DefWindowProcW`, which runs only when the message is dispatched. A wait that merely
        // polled `GetKeyboardLayout` would sit out its whole timeout with the message still in
        // the queue — requirement 1 of §11.5 asks for a condition, and this is what makes the
        // condition reachable.
        if let Some(wanted) = handle_for(language) {
            // SAFETY: `self.hwnd` is this process's own window and `wanted` a layout handle from
            // the system's own list. NFR-13: the result is examined.
            let posted = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(self.hwnd),
                    WM_INPUTLANGCHANGEREQUEST,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(wanted.0 as isize),
                )
            };
            if let Err(error) = posted {
                return Err(format!("PostMessage окну-якорю: {error}"));
            }
        } else {
            return Err(format!(
                "раскладка {} не подключена в этом сеансе",
                describe(language)
            ));
        }

        let observed = crate::wait::until(timeout, || {
            self.pump();
            of_window(self.hwnd).filter(|hkl| id_of(*hkl) & 0xFFFF == language & 0xFFFF)
        });

        match observed {
            Some(hkl) if front => Ok(id_of(hkl)),
            // The layout took, but the anchor never became the active window — so the session
            // was **not** told, and the next window created will not inherit it. Reporting this
            // as a success would be reporting the mechanism instead of the fact it exists for.
            Some(hkl) => Err(format!(
                "окно-якорь перешло в {}, но НЕ стало активным окном за {} с — раскладка сеанса \
                 не изменилась; передний план у {}",
                describe(id_of(hkl)),
                timeout.as_secs(),
                foreground_owner()
            )),
            None => Err(format!(
                "окно-якорь стенда не перешло в раскладку {} за {} с (стало активным: {front}); \
                 сейчас {}",
                describe(language),
                timeout.as_secs(),
                of_window(self.hwnd)
                    .map_or("неизвестно".to_owned(), |h| describe(id_of(h)))
            )),
        }
    }

    /// The layout the anchor is in — that is, the layout of the session as the anchor sees it.
    pub fn observed(&self) -> Option<u32> {
        of_window(self.hwnd).map(id_of)
    }

    /// Where the anchor stands at this instant, in one sentence for a note.
    ///
    /// Both halves matter and neither implies the other: the layout is what the anchor *is* in,
    /// and the foreground is whether the session is listening to it.
    pub fn describe_now(&self) -> String {
        // SAFETY: no arguments; returns a handle by value, possibly null.
        let front = unsafe { GetForegroundWindow() } == self.hwnd;
        format!(
            "якорь в {}, активен: {front}",
            self.observed().map_or("<не читается>".to_owned(), describe)
        )
    }

    /// Takes the foreground for the anchor, and **answers whether it got it**.
    ///
    /// The same technique `shell::raise_own_window` uses, and for the same reason:
    /// `SetForegroundWindow` from a background console process is ignored unless the input
    /// queues are attached first. Repeated on `wait::until`'s poll rather than once, because
    /// losing a race for the foreground is an ordinary state of a desktop.
    fn raise(&self, timeout: std::time::Duration) -> bool {
        crate::wait::until_true(timeout, || {
            self.pump();

            // SAFETY: no arguments; returns a handle by value, possibly null.
            let front = unsafe { GetForegroundWindow() };
            if front == self.hwnd {
                return true;
            }

            // SAFETY: a null `front` yields zero, which is examined; `None` writes nothing.
            let owner = unsafe { GetWindowThreadProcessId(front, None) };
            // SAFETY: no arguments; returns this thread's id.
            let ours = unsafe { GetCurrentThreadId() };

            let attached = owner != 0 && owner != ours;
            if attached {
                // SAFETY: both ids name live threads. NFR-13: a failed attach only means the
                // calls below run unprivileged, and the answer is re-read either way.
                let _ = unsafe { AttachThreadInput(ours, owner, true) };
            }

            // SAFETY: `self.hwnd` is this process's own window. The `BOOL`s are deliberately not
            // fatal — whether the foreground was taken is decided by re-reading it.
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_SHOWNORMAL);
                let _ = BringWindowToTop(self.hwnd);
                let _ = SetForegroundWindow(self.hwnd);
            }

            if attached {
                // SAFETY: undoes exactly the attachment made above, with the same two ids.
                let _ = unsafe { AttachThreadInput(ours, owner, false) };
            }

            // SAFETY: as above; this is the re-read that decides.
            unsafe { GetForegroundWindow() == self.hwnd }
        })
    }

    /// Dispatches whatever is waiting for this thread. Not a wait: it never blocks.
    fn pump(&self) {
        let mut message = MSG::default();
        loop {
            // SAFETY: `message` is a live, correctly shaped `MSG` this call fills. `None` asks
            // for the messages of every window of this thread, which is what has to be pumped.
            let waiting = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) };
            if !waiting.as_bool() {
                return;
            }
            // SAFETY: `message` was just filled by `PeekMessageW` and is dispatched unchanged.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

impl Drop for Ambient {
    fn drop(&mut self) {
        // SAFETY: `self.hwnd` is this process's own window, created in `open` and destroyed
        // exactly once here. NFR-13: the failure is not actionable at this point — the window
        // goes with the process either way. The **class** is deliberately left registered, see
        // `ambient_class`.
        let _ = unsafe { DestroyWindow(self.hwnd) };
        self.pump();
    }
}

/// Who holds the foreground right now, named for a report rather than for a decision.
fn foreground_owner() -> String {
    // SAFETY: no arguments; returns a handle by value, possibly null.
    let front = unsafe { GetForegroundWindow() };
    let mut pid = 0u32;
    // SAFETY: `front` may be null, which yields a zero thread id and leaves `pid` at zero;
    // `&mut pid` is a live `u32` the system writes at most once.
    let thread = unsafe { GetWindowThreadProcessId(front, Some(&mut pid)) };
    if thread == 0 {
        return "никого — переднего окна нет".to_owned();
    }

    format!(
        "процесса {pid} ({})",
        crate::own::process_table_name(pid).unwrap_or_else(|| "<имя неизвестно>".to_owned())
    )
}

/// ⚠ **The layout the session will hand to the next window created** — one reading, no state.
///
/// Opens an anchor, asks it what layout it was born in, and closes it. A window inherits the
/// session's layout at creation, so asking a brand-new window *is* the reading.
pub fn ambient() -> Option<u32> {
    Ambient::open().ok().and_then(|anchor| anchor.observed())
}

/// The window procedure of [`Ambient`]: everything to `DefWindowProcW`, which is the point.
///
/// `WM_INPUTLANGCHANGEREQUEST` is **carried out** by `DefWindowProcW` — a window that handled it
/// itself would be a window that decides its own layout, which is not what the anchor is for.
extern "system" fn ambient_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the four arguments are the ones the system passed to this procedure, forwarded
    // unchanged to the default implementation, which is the documented contract.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

// ---------------------------------------------------------------------------------------
// Footnote 3 of §11.3 — the temporary third layout
// ---------------------------------------------------------------------------------------
//
// Footnote 3 allows position 17 to attach a third layout with `LoadKeyboardLayout` and
// `KLF_ACTIVATE`, and requires it detached afterwards. Task T-04-3-3 is where that ability was
// written: [`Temporary`] below, used by `scenarios::position_17` and by nothing else.
//
// The requirement is **also** satisfied by measurement and not only by the guard: `attached()` is
// read before the run and after it, and the run reports whether the two lists are identical. That
// check catches a layout attached by accident, which a `Drop` on a type the bench happens to
// construct would not, and it is the check that stays honest if `Drop` never runs.

/// ⚠ **The KLID of the third layout — read the warning before touching this line.**
///
/// `LoadKeyboardLayoutW` takes a **KLID**, not an `HKL`, and the two are strings that look
/// alike: the KLID of German is `"00000407"` while `04070407` is its `HKL`. Passing an `HKL`
/// **does not fail** — the system takes it for the KLID of a layout it does not know and quietly
/// changes the set of layouts in the session. Fact 7 of §9 of `STATE.md`: that is how the layout
/// of somebody's working session was once knocked out.
///
/// So the value below is eight hexadecimal digits whose **high half is zero**, and
/// [`Temporary::attach`] checks the handle it got back before it trusts anything.
const THIRD_KLID: PCWSTR = w!("00000407");

/// The language id the [`THIRD_KLID`] above must resolve to — `de-DE`.
///
/// German and not, say, Greek on purpose: `ghbdtn` renders into German as the same six
/// characters, so nothing but the bench's own test vector can reach the report from the middle of
/// the cycle. SEC-01 and SEC-07 are about the product's log, and this keeps the bench's output to
/// the same standard. What separates "the text matched by accident" from "the cycle came back to
/// its start" is the `cycle_position` key of SEC-04a, which is the third assertion of position 17.
pub const THIRD: u32 = 0x0000_0407;

/// A layout attached for the length of one position and detached after it — **footnote 3 of
/// §11.3**.
///
/// # The guarantee, and why `Drop` alone is not it
///
/// Leaving a layout attached changes the machine of whoever ran the bench, which is worse than
/// failing a position. So detaching happens on three independent paths:
///
/// | Path | What detaches |
/// |---|---|
/// | the position succeeds or fails ordinarily | [`Temporary::detach`], called by the position |
/// | the position panics | this `Drop`, run by the unwind |
/// | the bench is killed from outside | **the note on disk**, acted on by [`recover_temporary`] at the start of the next run |
///
/// ⚠ The third row was written after it happened. A run of task T-04-3-3 was killed on purpose,
/// between attaching the layout and detaching it, to see what the kill really costs — and it
/// cost a third layout left in the session, which is a change to somebody's machine. So this
/// type now leaves a note in the bench's scratch area the moment it attaches, exactly as
/// `config::Borrowed` leaves a copy of the file: the run that comes next finds the note and
/// finishes the job.
///
/// The run's final comparison of [`attached`] stays where it is regardless. A mechanism that
/// claims a thing cannot happen is worth less than a check that says whether it did.
pub struct Temporary {
    hkl: HKL,
    /// The layout this thread was in before `KLF_ACTIVATE` moved it — restored before the unload,
    /// because a layout that is active on any thread cannot be unloaded.
    restore_thread_to: HKL,
    detached: bool,
}

/// The note that says a third layout is attached and whose it is.
///
/// In the bench's scratch area under `%TEMP%` and not beside anything of the user's. Its mere
/// existence is the claim; its content is the language id, so that [`recover_temporary`] detaches
/// the layout that was attached and never merely "a third one".
fn temporary_note() -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push("langsw-e2e-layout-stash.txt");
    path
}

/// ⚠ **The path no code of ours runs on**: a third layout left attached by a run that was killed.
///
/// Called once at the start of every run. `None` means there was nothing to clean up, which is
/// the ordinary case and is not printed.
///
/// ⛔ The note is what makes this safe. The bench detaches a layout **only** when its own note
/// says it attached it — the same rule as requirement A for processes, applied to layouts: never
/// take away something somebody else put there.
pub fn recover_temporary() -> Option<String> {
    let note = temporary_note();
    let language = std::fs::read_to_string(&note).ok()?;
    let language = u32::from_str_radix(language.trim(), 16).ok();
    let _ = std::fs::remove_file(&note);

    let language = language?;
    let Some(hkl) = handle_for(language) else {
        return Some(format!(
            "⚠ прежний прогон был прерван, но раскладки {} в сеансе уже нет — снимать нечего",
            describe(language)
        ));
    };

    // SAFETY: `hkl` came from the system's own list a line ago, and the note says this bench
    // attached it. NFR-13: the result is examined, and so is the list afterwards.
    let unloaded = unsafe { UnloadKeyboardLayout(hkl) };
    let still_here = attached().contains(&id_of(hkl));

    Some(match (unloaded, still_here) {
        (Ok(()), false) => format!(
            "⚠ прежний прогон был прерван: оставленная им раскладка {} снята",
            describe(id_of(hkl))
        ),
        (_, false) => format!(
            "⚠ прежний прогон был прерван: раскладки {} в списке сеанса больше нет",
            describe(id_of(hkl))
        ),
        (result, true) => format!(
            "⚠⚠ оставленная прежним прогоном раскладка {} НЕ СНЯТА: {result:?}",
            describe(id_of(hkl))
        ),
    })
}

impl Temporary {
    /// Attaches the third layout of footnote 3 and proves it is the one asked for.
    pub fn attach() -> Result<Self, String> {
        let before = attached();
        // SAFETY: no arguments; answers about the calling thread.
        let restore_thread_to = unsafe { GetKeyboardLayout(0) };

        // SAFETY: `THIRD_KLID` is a static, null-terminated wide string of eight hexadecimal
        // digits — a KLID and not an `HKL`, see the constant. `KLF_ACTIVATE` is the flag
        // footnote 3 names. NFR-13: the `Result` is examined, and so is the handle inside it.
        let hkl = unsafe { LoadKeyboardLayoutW(THIRD_KLID, KLF_ACTIVATE) }
            .map_err(|error| format!("LoadKeyboardLayout(KLID {THIRD_KLID:?}): {error}"))?;

        // ⚠ The check that turns "the call returned something" into "the call did what was
        // meant". A KLID the system does not know is not an error — it is a *different layout*,
        // and this is the line that notices.
        if id_of(hkl) & 0xFFFF != THIRD & 0xFFFF {
            let stray = Self {
                hkl,
                restore_thread_to,
                detached: false,
            };
            let got = describe(id_of(hkl));
            drop(stray);
            return Err(format!(
                "LoadKeyboardLayout вернула {got}, а ожидалась {} — раскладка снята немедленно",
                describe(THIRD)
            ));
        }

        // Back to the layout this thread had: `KLF_ACTIVATE` moved it, and a layout active on
        // any thread refuses to unload.
        // SAFETY: `restore_thread_to` is a handle the system itself gave a moment ago.
        // NFR-13: the failure is not fatal — the unload examines its own result.
        let _ = unsafe { ActivateKeyboardLayout(restore_thread_to, KLF_ACTIVATE) };

        let after = attached();
        if after.len() <= before.len() {
            // The layout was already in the session. Detaching it afterwards would take away
            // something the bench did not add, which is exactly the damage this type exists to
            // avoid, so the position is refused instead.
            return Err(format!(
                "раскладка {} уже была подключена в этом сеансе (до: {}, после: {}); \
                 стенд не снимает того, чего не подключал",
                describe(THIRD),
                before.len(),
                after.len()
            ));
        }

        // The note goes down now, while the layout is attached and the process is alive. From
        // here on a kill costs the next run one `UnloadKeyboardLayout` and nobody a layout.
        if let Err(error) = std::fs::write(temporary_note(), format!("{:08X}", THIRD)) {
            eprintln!("  ⚠ записка о третьей раскладке не записана: {error}");
        }

        Ok(Self {
            hkl,
            restore_thread_to,
            detached: false,
        })
    }

    /// The handle the session gave the third layout — the number the report prints.
    pub fn handle(&self) -> u32 {
        id_of(self.hkl)
    }

    /// Detaches it, and says what happened in a sentence fit for the report.
    pub fn detach(&mut self) -> String {
        if self.detached {
            return "третья раскладка уже была снята".to_owned();
        }
        self.detached = true;

        // SAFETY: puts this thread back on the layout it had before `attach`, so that the
        // unload below is not refused because our own thread is holding the layout.
        let _ = unsafe { ActivateKeyboardLayout(self.restore_thread_to, KLF_ACTIVATE) };

        // SAFETY: `self.hkl` is the handle `LoadKeyboardLayoutW` returned to this very type and
        // has not been unloaded before — the flag above makes that true. NFR-13: examined.
        let unloaded = unsafe { UnloadKeyboardLayout(self.hkl) };

        let still_here = attached().contains(&id_of(self.hkl));

        // The note is only removed once the layout really is gone: a note left standing costs the
        // next run one harmless call, and a note removed too early costs somebody a layout.
        if !still_here {
            let _ = std::fs::remove_file(temporary_note());
        }

        match (unloaded, still_here) {
            (Ok(()), false) => format!("третья раскладка {} снята", describe(id_of(self.hkl))),
            (Ok(()), true) => format!(
                "⚠ UnloadKeyboardLayout прошла, но {} всё ещё в списке сеанса",
                describe(id_of(self.hkl))
            ),
            (Err(error), false) => format!(
                "UnloadKeyboardLayout вернула ошибку ({error}), но {} в списке сеанса больше нет",
                describe(id_of(self.hkl))
            ),
            (Err(error), true) => format!(
                "⚠⚠ ТРЕТЬЯ РАСКЛАДКА {} НЕ СНЯТА: {error}",
                describe(id_of(self.hkl))
            ),
        }
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        if !self.detached {
            eprintln!("  {}", self.detach());
        }
    }
}
