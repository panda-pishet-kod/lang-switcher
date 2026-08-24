//! Synthetic input, and the two guarantees that make sending it safe.
//!
//! # The signature — requirement 2 of §11.5, and the most dangerous failure of this task
//!
//! FR-03 has the product filter its own injected input by a magic value in `dwExtraInfo`. If
//! the bench signed its input with that same value, the product would discard every keystroke
//! the bench sends as its own, the buffer would stay empty, the hotkey would find nothing to
//! convert — and **every position would report a green result without a single real check
//! having happened**. The failure looks exactly like success, which is why [`BENCH_SIGNATURE`]
//! is checked against the product's constant at compile time, not by inspection.
//!
//! # The foreground guard — the safety half
//!
//! There is a person at this machine and their documents are not the bench's sandbox. Nothing
//! is ever sent to whatever window happens to be in front: every send goes through
//! [`send_verified`], which asks the system which window is foreground, compares its process
//! against the window the bench itself launched and identified, and **returns without sending**
//! when they differ.
//!
//! `SendInput` is called on **two** lines of the whole bench, both of them in this module, and
//! what makes each of them safe is a different thing:
//!
//! * inside [`send_verified`] — the guarded send, and the only one that can carry text. The
//!   function is private to this module and every public entry point that types anything
//!   ([`type_text`], [`tap`], [`chord`]) ends in it, so no other module of the bench can reach
//!   `SendInput` at all: the guard is not bypassed by a new call site because there is nowhere
//!   else to put one.
//! * inside [`emergency_combination`] — `Ctrl+Alt+Shift+F12` of FR-96, sent **without** a
//!   foreground check on purpose, for the reason set out at that function. It is safe not
//!   because it is guarded but because of what it carries: four modifier-and-function keys with
//!   no text among them, aimed at the product's global hook and not at anybody's document.
//!
//! ⚠ **The pair is checked, not asserted.** This paragraph used to say «exactly one line», and
//! had been wrong since the second call site was added; a reader who believes such a sentence
//! never goes to look, which makes a false invariant worse than none. The test
//! `send_input_is_called_only_in_the_two_named_functions` at the foot of this file re-derives
//! both call sites from the source of `tests\e2e\` on every `cargo test --features testing`, so
//! a third one — or either of these two moving out of its function — turns the claim red.

use std::fmt;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_F12, VK_LWIN, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// `dwExtraInfo` every event this bench sends carries.
///
/// ⚠ **Must differ from [`lang_switcher::hook::INJECTED_SIGNATURE`]** — requirement 2 of §11.5.
/// The bytes spell `LSW_E2E1`, chosen to be recognisable in a debugger next to the product's
/// `LSW_INJ01` and to share no nibble pattern with it by accident.
pub const BENCH_SIGNATURE: usize = 0x4C53_575F_4532_4531;

/// The compile-time form of requirement 2 of §11.5.
///
/// A runtime check would be a check the bench could be built without. This one cannot be: if
/// the two constants ever become equal, `cargo build --features testing --bin langsw-e2e`
/// stops with this assertion instead of producing a bench that silently proves nothing.
const _: () = assert!(
    BENCH_SIGNATURE != lang_switcher::hook::INJECTED_SIGNATURE,
    "the bench signature must differ from the product's INJECTED_SIGNATURE: FR-03 would make \
     the product discard every keystroke the bench sends, and every position of the matrix \
     would report success without having tested anything"
);

/// Why a send did not happen, or did not happen fully.
#[derive(Debug, Clone)]
pub enum SendError {
    /// The foreground window was not the one this scenario owns. **Nothing was sent.**
    ///
    /// Carries what was found and what was expected, because this is a fact a report needs to
    /// state precisely: it is the difference between "the product failed" and "the bench
    /// refused to type into a stranger's window".
    NotOurs {
        expected_pid: u32,
        expected_title: String,
        actual_pid: u32,
        actual_title: String,
    },
    /// The foreground window belongs to a process the bench never started — **requirement E**.
    ///
    /// Distinct from [`SendError::NotOurs`] on purpose. That one means "the right application,
    /// wrong window"; this one means "a stranger's process", and it is the condition that had
    /// to exist before the bench could be trusted with a keyboard at all.
    NotInRegistry { pid: u32, title: String },
    /// There is no foreground window at all.
    NoForeground,
    /// `SendInput` accepted fewer events than it was given — FR-45's situation, seen from the
    /// other side.
    Short { sent: u32, expected: u32 },
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOurs {
                expected_pid,
                expected_title,
                actual_pid,
                actual_title,
            } => write!(
                f,
                "переднее окно принадлежит процессу {actual_pid} ({actual_title:?}), а не \
                 процессу {expected_pid} ({expected_title:?}), который запустил этот сценарий; \
                 НИЧЕГО НЕ ОТПРАВЛЕНО"
            ),
            Self::NotInRegistry { pid, title } => write!(
                f,
                "⛔ переднее окно принадлежит процессу {pid} ({title:?}), которого нет в реестре \
                 собственных процессов стенда (пункт E); НИЧЕГО НЕ ОТПРАВЛЕНО"
            ),
            Self::NoForeground => write!(f, "there is no foreground window; nothing was sent"),
            Self::Short { sent, expected } => {
                write!(f, "SendInput accepted {sent} of {expected} events")
            }
        }
    }
}

/// The window a scenario is allowed to type into.
///
/// Constructed only from a window the bench itself launched and then identified through UI
/// Automation, which is what makes the check in [`send_verified`] meaningful: "is the
/// foreground window ours" is a question with an answer only because something recorded what
/// "ours" is.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    /// Process the window belongs to.
    pub pid: u32,
    /// The window itself, kept for diagnostics and for the layout query of FR-52.
    pub hwnd: HWND,
}

/// Process id of the current foreground window, and its title.
///
/// `None` when there is no foreground window — which happens, briefly, while the desktop is
/// switching windows, and is a reason to wait rather than a reason to send.
pub fn foreground() -> Option<(u32, HWND)> {
    // SAFETY: `GetForegroundWindow` takes no arguments and returns a window handle or a null
    // one; it dereferences nothing of ours and cannot fail in a way that needs an error code.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }

    let mut pid = 0u32;
    // SAFETY: `hwnd` was just returned by the system and checked non-null. The second argument
    // is an optional out-parameter; the pointer given is to a live local that outlives the
    // call. NFR-13: a zero return means the window died between the two calls, and that is
    // examined below rather than assumed away.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if thread == 0 {
        return None;
    }

    Some((pid, hwnd))
}

/// Title of a window, for diagnostics only.
fn title_of(hwnd: HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    let mut buffer = [0u16; 256];
    // SAFETY: the buffer is a live local array and its length is passed as the bound, so the
    // call cannot write past it. A zero return means an empty or unreadable title, which is
    // rendered as an empty string — this value is diagnostic and never a verdict.
    let length = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

/// **The guarded call site of `SendInput`**, and the foreground check in front of it.
///
/// One of the two call sites in the bench — the module comment names both — and the only one
/// that can carry text into an application. It is private, and every public entry point that
/// types anything ends here.
///
/// The order is the point: the check happens first and a mismatch returns before a single
/// event is written. Requirement of the "Окружение и безопасность" section of the task —
/// "не совпало — не отправлять".
fn send_verified(events: &[INPUT], target: &Target) -> Result<(), SendError> {
    let Some((pid, hwnd)) = foreground() else {
        return Err(SendError::NoForeground);
    };

    if pid != target.pid {
        return Err(SendError::NotOurs {
            expected_pid: target.pid,
            // The window the scenario identified, named in the refusal: a report that says only
            // "the wrong window was in front" leaves the reader to guess which one was right.
            expected_title: title_of(target.hwnd),
            actual_pid: pid,
            actual_title: title_of(hwnd),
        });
    }

    // ⛔ Requirement E, and the second half of the guard. The check above says the foreground
    // window is the one this scenario expects; this one says the bench started it. Both, in
    // this order, and only then is a single event written.
    if !crate::own::is_ours(pid) {
        return Err(SendError::NotInRegistry {
            pid,
            title: title_of(hwnd),
        });
    }

    let expected = events.len() as u32;
    // SAFETY: `events` is a live slice of `INPUT` and its length is derived from the slice
    // itself; `cbsize` is the size of the very type the slice holds. `SendInput` reads the
    // array and returns how many events it accepted. NFR-13: the return is compared with the
    // number given rather than discarded — the mismatch is exactly what FR-45 is about.
    let sent = unsafe { SendInput(events, core::mem::size_of::<INPUT>() as i32) };

    if sent != expected {
        return Err(SendError::Short { sent, expected });
    }

    Ok(())
}

/// One keyboard event, signed with [`BENCH_SIGNATURE`].
fn key_event(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(flags),
                time: 0,
                dwExtraInfo: BENCH_SIGNATURE,
            },
        },
    }
}

/// Types a string by **pressing the keys that bear those characters**, not by injecting the
/// characters themselves.
///
/// ⚠ **This distinction is the whole scenario, and getting it wrong makes the bench prove
/// nothing.** The first version of this function sent `KEYEVENTF_UNICODE` events, the way
/// `probe-word.ps1` does — and every position reported `fail` with the text still reading
/// `ghbdtn`, because a Unicode event carries `wVk = 0` and puts the character in `wScan`. FR-04
/// has the product record the **scan code** of each press, and FR-06 decode it with
/// `ToUnicodeEx`; given `vk = 0` there is nothing to record and nothing to decode, so the
/// buffer stays empty and the hotkey finds nothing to convert.
///
/// The scenario of §11.3 is "набрать `ghbdtn`" — a person pressing the G, H, B, D, T and N keys
/// with the English layout active. That is what this sends: the virtual-key code of each
/// letter, with the scan code the current layout maps it to, exactly as a keyboard would.
///
/// Non-ASCII characters have no single key to press and fall back to `KEYEVENTF_UNICODE`. The
/// matrix never types any; the branch exists so that a future caller gets a working call rather
/// than a silent omission.
pub fn type_text(text: &str, target: &Target) -> Result<(), SendError> {
    let mut events = Vec::new();

    for character in text.chars() {
        match virtual_key_for(character) {
            Some(vk) => {
                let scan = scan_code_of(vk);
                events.push(key_event(vk, scan, 0));
                events.push(key_event(vk, scan, KEYEVENTF_KEYUP.0));
            }
            None => {
                let mut buffer = [0u16; 2];
                for unit in character.encode_utf16(&mut buffer) {
                    events.push(key_event(0, *unit, KEYEVENTF_UNICODE.0));
                    events.push(key_event(0, *unit, KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0));
                }
            }
        }
    }

    send_verified(&events, target)
}

/// The virtual-key code of an unshifted ASCII letter or digit.
///
/// For `A`–`Z` and `0`–`9` the virtual-key code *is* the uppercase ASCII value — the same fact
/// `hook::vk_from_name` rests on. Everything else returns `None`.
fn virtual_key_for(character: char) -> Option<u16> {
    match character {
        'a'..='z' => Some(character.to_ascii_uppercase() as u16),
        'A'..='Z' | '0'..='9' => Some(character as u16),
        _ => None,
    }
}

/// The scan code the current layout gives this virtual key.
///
/// FR-04 stores the scan code and FR-05 the extended flag; sending a press with its real scan
/// code is what makes the product's record of it indistinguishable from a physical one. A zero
/// answer — a key this layout has no position for — is passed through, and the system then
/// fills the scan code in itself.
fn scan_code_of(vk: u16) -> u16 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_VSC, MapVirtualKeyW};

    // SAFETY: `MapVirtualKeyW` takes two integers and returns one; it dereferences nothing and
    // cannot fail in a way that needs an error code. A zero return means "no mapping", which is
    // handled by the caller passing it on unchanged.
    unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) as u16 }
}

/// Presses and releases one virtual key.
pub fn tap(vk: u16, target: &Target) -> Result<(), SendError> {
    let extended = if is_extended(vk) {
        KEYEVENTF_EXTENDEDKEY.0
    } else {
        0
    };

    let events = [
        key_event(vk, 0, extended),
        key_event(vk, 0, extended | KEYEVENTF_KEYUP.0),
    ];

    send_verified(&events, target)
}

/// Presses `vk` while `modifiers` are held down, then releases everything in reverse order.
///
/// Position 22 of the matrix is exactly this with `Shift`: the hotkey pressed under a held
/// modifier must still produce an undistorted result, because FR-40 steps 3 and 6 exist.
pub fn chord(modifiers: &[u16], vk: u16, target: &Target) -> Result<(), SendError> {
    let mut events = Vec::new();

    for &modifier in modifiers {
        events.push(key_event(modifier, 0, 0));
    }

    let extended = if is_extended(vk) {
        KEYEVENTF_EXTENDEDKEY.0
    } else {
        0
    };
    events.push(key_event(vk, 0, extended));
    events.push(key_event(vk, 0, extended | KEYEVENTF_KEYUP.0));

    for &modifier in modifiers.iter().rev() {
        events.push(key_event(modifier, 0, KEYEVENTF_KEYUP.0));
    }

    send_verified(&events, target)
}

/// The emergency combination of FR-96 — `Ctrl+Alt+Shift+F12`.
///
/// ⚠ **Sent without a foreground check, and that is deliberate.** This is the one send in the
/// bench whose purpose is to reach the *product*, not an application window: the product's
/// hook is global and sees the combination whatever has focus. Requiring a foreground match
/// here would make the safety net depend on the very state that has already gone wrong by the
/// time it is needed. It types nothing into anybody's document — four modifier-and-function
/// keys with no text among them.
pub fn emergency_combination() -> Result<(), SendError> {
    let mut events = Vec::new();

    for modifier in [VK_CONTROL.0, VK_MENU.0, VK_SHIFT.0] {
        events.push(key_event(modifier, 0, 0));
    }
    events.push(key_event(VK_F12.0, 0, 0));
    events.push(key_event(VK_F12.0, 0, KEYEVENTF_KEYUP.0));
    for modifier in [VK_SHIFT.0, VK_MENU.0, VK_CONTROL.0] {
        events.push(key_event(modifier, 0, KEYEVENTF_KEYUP.0));
    }

    let expected = events.len() as u32;
    // SAFETY: identical to the call in `send_verified` — a live slice of `INPUT` with its own
    // length and the size of its own element type. NFR-13: the return is compared below.
    let sent = unsafe { SendInput(&events, core::mem::size_of::<INPUT>() as i32) };

    if sent != expected {
        return Err(SendError::Short { sent, expected });
    }

    Ok(())
}

/// Keys that need `KEYEVENTF_EXTENDEDKEY` to be recognised as themselves — FR-05's concern,
/// seen from the sending side.
fn is_extended(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_DELETE, VK_DOWN, VK_END, VK_HOME, VK_INSERT, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RIGHT,
        VK_RWIN, VK_UP,
    };

    [
        VK_INSERT.0,
        VK_DELETE.0,
        VK_HOME.0,
        VK_END.0,
        VK_PRIOR.0,
        VK_NEXT.0,
        VK_LEFT.0,
        VK_RIGHT.0,
        VK_UP.0,
        VK_DOWN.0,
        VK_LWIN.0,
        VK_RWIN.0,
    ]
    .contains(&vk)
}

#[cfg(test)]
mod tests {
    use crate::wait::sweep;

    /// Both call sites of `SendInput` in the bench: the file, and the function each is inside.
    ///
    /// One entry per bullet of the module comment. The table and the bullets are changed
    /// together or not at all — a third send is a decision, not an edit.
    const SENDS: [(&str, &str); 2] = [
        ("input.rs", "send_verified"),
        ("input.rs", "emergency_combination"),
    ];

    /// **The foreground guard of §11.5 stated as a fact about the source, and checked.**
    ///
    /// The guard is only worth what the absence of an unguarded call site is worth, and that
    /// absence is what this re-derives every run: every send that carries text has to go through
    /// [`super::send_verified`], because there is no other place in the bench where `SendInput`
    /// is named as a call.
    #[test]
    fn send_input_is_called_only_in_the_two_named_functions() {
        let mut found: Vec<(String, String)> = Vec::new();

        for (name, text) in sweep::sources() {
            for line in sweep::call_lines(&text, sweep::SEND_INPUT_CALL) {
                found.push((name.clone(), sweep::function_at(&text, line)));
            }
        }

        let mut expected: Vec<(String, String)> = SENDS
            .iter()
            .map(|(file, owner)| ((*file).to_owned(), (*owner).to_owned()))
            .collect();

        found.sort();
        expected.sort();

        assert_eq!(
            found, expected,
            "the bench sends from somewhere the module comment of `input.rs` does not name. A \
             send that carries text belongs behind `send_verified`; one that does not still \
             belongs in this module, in the comment and in this table."
        );
    }
}
