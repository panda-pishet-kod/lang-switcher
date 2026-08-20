//! The positions of the acceptance matrix of §11.3.
//!
//! # The shape of a position
//!
//! Every text-replacement position is the same seven steps, in [`replacement`]:
//!
//! 1. bring the window forward — `shell::activate`, never `SetForegroundWindow`;
//! 2. put the window into the English layout, which is the precondition the scenario states;
//! 3. empty the field, so that what is read back afterwards is only what this run produced;
//! 4. type `ghbdtn` — through the foreground check, with the bench's own signature;
//! 5. wait until UI Automation reads it back, which is how the bench knows the keystrokes
//!    landed rather than assuming they did;
//! 6. press the hotkey;
//! 7. wait for `привет`, and read the window's layout.
//!
//! Steps 5 and 7 are waits on a condition, never on a clock — requirement 1 of §11.5.
//!
//! **Position 16 is those same seven steps and one more** — [`Scene::rollback`]: the hotkey is
//! pressed a second time and what is asserted is the *original* text, the layout back at en-US
//! and `cycle_position` back at zero. The wait before the second press is step 7's wait for
//! `привет` and not a pause, which is requirement 1 of §11.5 again and matters more here than
//! anywhere else: a second press sent while the first replacement is still on its way would
//! measure the race and not the rollback.
//!
//! # Two assertions, three verdicts
//!
//! Step 7 produces two rows, not one: the text, and the layout. **Both are real verdicts.** The
//! layout row was `pending` on **T-05-1** while the switch of FR-40 step 5 did not exist; task
//! T-05-1 wrote the chain of §4.6 (commit `c8e463f`) and task T-05-2 gave it the target of §4.4
//! to switch to, so the row now says `pass` or `fail` by the layout the bench really reads. The
//! third verdict of decision Р-30 stays where it belongs: the positions the bench may not drive
//! at all, which §11.6 hands to a person, and the requirements no task has written yet.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{
    UIA_ButtonControlTypeId, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_A, VK_BACK, VK_CONTROL, VK_DELETE, VK_E, VK_HOME, VK_LWIN, VK_MENU, VK_SHIFT, VK_SPACE,
    VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, PostMessageW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_CLOSE,
};
use windows::core::w;

use crate::report::{Assertion, Row, Verdict};
use crate::uia::{Automation, Element, normalise};
use crate::{clip, input, layout, shell, wait, word};

/// What the bench types. The English-layout keystrokes that spell `привет` in Russian.
pub const TYPED: &str = "ghbdtn";

/// What the product is expected to leave behind.
pub const EXPECTED: &str = "привет";

/// Everything a scenario needs from the run.
pub struct Context<'a> {
    pub automation: &'a Automation,
    /// Virtual-key code of the hotkey, read from the product's own configuration.
    pub hotkey_vk: u16,
    /// The ambient layout the run found before it changed anything, and the value every position
    /// is put back to — requirement 5 of §11.5.
    ///
    /// The layout itself is written through [`layout::set_ambient`], which opens a window of the
    /// bench's **own** for the purpose and closes it again. ⛔ No window of anybody else's is
    /// asked anything, and there is no shared handle here that a scenario could reach past.
    pub ambient_before: Option<u32>,
}

// ---------------------------------------------------------------------------------------
// A launched application the bench owns
// ---------------------------------------------------------------------------------------

/// How an application is put away again — requirement 5 of §11.5.
enum CloseWith {
    /// `WM_CLOSE` to the main window. Well-behaved applications honour it.
    WmClose,
    /// Terminate the process.
    ///
    /// Used **only** for applications the bench started against a throwaway profile of its
    /// own, where termination can damage nothing that belongs to the user. Never for Word —
    /// rake 5 of §2 of `TOOLCHAIN.md`.
    Terminate,
    /// Word: COM `Quit(0)`, and nothing else.
    WordCom,
}

/// An application under a scenario.
struct App {
    name: String,
    pid: u32,
    child: Option<Child>,
    window: Option<HWND>,
    close: CloseWith,
    /// A throwaway profile directory, removed with the application.
    scratch: Option<PathBuf>,
}

impl App {
    /// ⛔ **Requirement B.** Adopts a window only if the bench started the process behind it.
    ///
    /// The process that owns a window is often not the one `spawn` returned — Chrome and VS Code
    /// spread over a process tree, `wt.exe` exits at once, `explorer.exe` hands the request to
    /// the running shell, and the System32 `notepad.exe` is a stub for a packaged application.
    /// The earlier version of this function adopted whatever the predicate matched, and that is
    /// how the bench came to close the controller's editor twice.
    ///
    /// Now the predicate only nominates; `own::claim_window_process` decides. On refusal
    /// **nothing at all is done to the window** — it is not adopted, `self.pid` keeps pointing
    /// at the process the bench really did start, and the caller records a `fail`.
    fn adopt(&mut self, window: &Element) -> Result<(), String> {
        let Some(pid) = window.pid() else {
            return Err("у окна не читается идентификатор процесса".to_owned());
        };

        crate::own::claim_window_process(pid)?;

        self.pid = pid;
        self.window = window.hwnd();
        Ok(())
    }

    /// Puts the application away and cleans up after it — requirement 5 of §11.5.
    fn close(&mut self) -> String {
        let mut said = format!("{}: ", self.name);

        match self.close {
            CloseWith::WordCom => match word::quit() {
                Ok(answer) => said.push_str(&format!("COM Quit(0): {answer}")),
                Err(error) => said.push_str(&format!("COM Quit(0) не сработала: {error}")),
            },
            CloseWith::WmClose => {
                if let Some(hwnd) = self.window {
                    // SAFETY: `hwnd` was obtained from UI Automation and may since have died,
                    // in which case `PostMessageW` fails — which is examined. Nothing of ours
                    // is dereferenced.
                    let posted =
                        unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
                    said.push_str(match posted {
                        Ok(()) => "WM_CLOSE отправлено",
                        Err(_) => "WM_CLOSE отправить не удалось",
                    });
                } else {
                    said.push_str("окна нет, закрывать нечего");
                }
            }
            CloseWith::Terminate => match shell::terminate(self.pid) {
                Ok(()) => said.push_str(&format!("процесс {} снят", self.pid)),
                Err(error) => said.push_str(&format!("снять процесс не удалось: {error}")),
            },
        }

        // Whatever the method, wait for the window's process to actually go.
        let gone = wait::until_true(wait::CLOSE_TIMEOUT, || !shell::process_is_alive(self.pid));

        if gone {
            said.push_str("; процесс завершился");
        } else {
            said.push_str("; процесс не завершился в отведённый срок");
            // ⚠ The last resort, and **never** for Word: rake 5 says a killed Word poisons the
            // next launch, so a Word that will not quit is reported and left alone.
            if !matches!(self.close, CloseWith::WordCom) {
                match shell::terminate(self.pid) {
                    Ok(()) => said.push_str(", снят принудительно"),
                    Err(error) => said.push_str(&format!(", снять не удалось: {error}")),
                }
            }
        }

        // The launcher process, when it is a different one and is still around.
        //
        // ⛔ Requirement D, and it holds here by construction rather than by a check:
        // `Child::kill` acts on the process **handle** returned by this bench's own
        // `Command::spawn`, not on a process id looked up from a window. It cannot name a
        // process the bench did not start, so there is nothing for the registry to decide.
        if let Some(child) = self.child.as_mut()
            && matches!(child.try_wait(), Ok(None))
        {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(scratch) = self.scratch.take() {
            let _ = std::fs::remove_dir_all(&scratch);
        }

        said
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if shell::process_is_alive(self.pid) {
            let _ = self.close();
        }
    }
}

/// Waits for a top-level window matching `predicate`, then puts it through requirement B.
///
/// Three outcomes, and the report must be able to tell them apart:
/// * `Ok(window)` — the window is the bench's own and may be worked with;
/// * `Err(reason)` — a window matched but belongs to somebody else, so it was **refused**;
/// * `Err("окно не появилось")` — nothing matched inside the timeout.
///
/// ⚠ The refusal case scans the candidates rather than taking the first: with a predicate as
/// broad as "class `Chrome_WidgetWin_1`" the first match may well be a stranger's window while
/// the bench's own is second in the list. Taking the first and failing would be safe but would
/// also make several positions unrunnable for no reason.
fn adopt_window(
    automation: &Automation,
    app: &mut App,
    predicate: &dyn Fn(&Element) -> bool,
) -> Result<Element, String> {
    /// How long to keep looking after a matching window turned out to belong to somebody else.
    ///
    /// Long enough for the bench's own window to arrive behind a stranger's that matched the
    /// same predicate, short enough that a position which can never be adopted does not spend
    /// the whole window timeout proving it.
    const REFUSAL_GRACE: Duration = Duration::from_secs(10);

    let started = Instant::now();
    let mut last_refusal: Option<String> = None;

    let found = wait::until(wait::WINDOW_TIMEOUT, || {
        for candidate in automation.top_level_of_any(predicate) {
            match app.adopt(&candidate) {
                Ok(()) => return Some(Ok(candidate)),
                Err(reason) => last_refusal = Some(reason),
            }
        }

        // A window matched but was refused, and the answer will not change: a process does not
        // become the bench's own by being looked at again.
        if last_refusal.is_some() && started.elapsed() >= REFUSAL_GRACE {
            return Some(Err(()));
        }

        None
    });

    match (found, last_refusal) {
        (Some(Ok(window)), _) => Ok(window),
        // A window matched the predicate but was not ours. This is the case requirement B is
        // about, and it is reported verbatim so the run says whose window it declined to touch.
        (_, Some(reason)) => Err(reason),
        (_, None) => Err("окно приложения не появилось".to_owned()),
    }
}

/// The `App` for something launched by a command that may or may not own the window.
///
/// ⛔ **Requirement A.** This is the funnel every launch goes through, and it registers the
/// process id here — at the moment of launch, which is the only moment requirement A allows the
/// registry to grow.
fn launched(name: &str, child: Child, close: CloseWith, scratch: Option<PathBuf>) -> App {
    crate::own::register_spawned(child.id());

    App {
        name: name.to_owned(),
        pid: child.id(),
        child: Some(child),
        window: None,
        close,
        scratch,
    }
}

/// A throwaway profile directory under the system temp area.
fn scratch_dir(tag: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("langsw-e2e-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&path);
    path
}

// ---------------------------------------------------------------------------------------
// The seven steps
// ---------------------------------------------------------------------------------------

/// Both rows of a position that could not even be set up.
fn both_failed(position: u8, app: &str, reason: &str) -> Vec<Row> {
    vec![
        Row::new(
            position,
            app,
            Assertion::Text,
            Verdict::Fail,
            reason,
            EXPECTED,
        ),
        Row::new(
            position,
            app,
            Assertion::Layout,
            Verdict::Fail,
            reason,
            layout::describe(layout::RUSSIAN),
        ),
    ]
}

/// Every row of position 16 when the scenario could not reach the second press.
///
/// Its own helper and not [`both_failed`] because position 16 asserts more things and expects
/// different values for two of them: the *original* text, and the *source* layout. A row that
/// failed while claiming to have expected `привет` would misreport what the position is for.
///
/// The rows of the series of task T-10-6 come with it, so that the position reports the **same
/// number of assertions** whether it ran or fell over at the first step: a run whose row count
/// depends on how far it got cannot be compared with the run before it, and comparing runs is
/// what the bench is for.
fn rollback_failed(position: u8, app: &str, reason: &str) -> Vec<Row> {
    let mut rows = vec![
        Row::new(
            position,
            app,
            Assertion::Text,
            Verdict::Fail,
            reason,
            format!("{TYPED:?} побитово"),
        ),
        Row::new(
            position,
            app,
            Assertion::Layout,
            Verdict::Fail,
            reason,
            layout::describe(layout::US),
        ),
        Row::new(
            position,
            app,
            Assertion::Other(CYCLE_ASSERTION),
            Verdict::Fail,
            reason,
            "cycle_position=0",
        ),
    ];

    rows.extend(series_failed(position, app, reason));

    rows
}

/// What the third row of position 16 is about, in the report.
const CYCLE_ASSERTION: &str = "позиция в цикле";

/// Everything [`replacement`] needs about one already-set-up application.
struct Scene<'a> {
    position: u8,
    app_name: &'a str,
    /// The process the foreground check compares against — the window's own, which for Chrome,
    /// VS Code, Windows Terminal and Explorer is not the process `spawn` returned.
    pid: u32,
    window: HWND,
    content: &'a Element,
    /// Modifiers held while the hotkey is pressed. Position 22 supplies `Shift`; the rest
    /// supply nothing.
    modifiers: &'a [u16],
    /// Whether the field can be emptied with `Ctrl+A`, `Delete` first. False for consoles,
    /// where both keys mean something else.
    clear_first: bool,
    /// **Position 16 only** — press the hotkey a second time and assert the rollback of FR-33
    /// instead of the replacement.
    ///
    /// A flag on the existing scene rather than a scenario of its own, because that is what
    /// position 16 is: the seven steps of [`replacement`], unchanged, and one more press. Every
    /// other position passes `false` and takes byte for byte the path it took before.
    rollback: bool,
}

/// The seven steps, for one already-launched application and its content element.
fn replacement(ctx: &Context, scene: Scene<'_>) -> Vec<Row> {
    let Scene {
        position,
        app_name,
        pid,
        window,
        content,
        modifiers,
        clear_first,
        rollback,
    } = scene;

    // A setup failure has to be reported in the shape of the position it happened in: two rows
    // for the ordinary scenario, three for the rollback of position 16, and with that position's
    // own expected values. For `rollback == false` this is `both_failed` called exactly as it was
    // called before, argument for argument.
    let failed = |reason: &str| -> Vec<Row> {
        if rollback {
            rollback_failed(position, app_name, reason)
        } else {
            both_failed(position, app_name, reason)
        }
    };

    // Step 1 — forward. Rake 3, for every application and not only for Word. The window handle
    // goes too: the window is in registry A, so Win32 may be used on it directly.
    if let Err(error) = shell::activate_window(pid, Some(window)) {
        return failed(&format!("вывод окна вперёд: {error}"));
    }

    let target = input::Target { pid, hwnd: window };

    // Step 2 — the precondition the scenario states: the English layout.
    let source_layout = match layout::ensure(window, layout::US, Duration::from_secs(5)) {
        Ok(id) => id,
        Err(error) => {
            return failed(&format!("исходная раскладка: {error}"));
        }
    };

    // Step 3 — empty the field, so the reading afterwards is only this run's doing.
    if clear_first {
        if let Err(error) = input::chord(&[VK_CONTROL.0], VK_A.0, &target) {
            return failed(&format!("очистка поля: {error}"));
        }
        if let Err(error) = input::tap(VK_DELETE.0, &target) {
            return failed(&format!("очистка поля: {error}"));
        }
    }

    // Step 4 — type. The foreground check is inside `type_text`; a refusal ends the position
    // here rather than typing into somebody else's window.
    if let Err(error) = input::type_text(TYPED, &target) {
        return failed(&format!("ввод {TYPED:?}: {error}"));
    }

    // Step 5 — wait until the application has actually taken the keystrokes. Not a delay: the
    // condition is "the element reads back what was typed".
    let typed_seen = wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, source)| (normalise(&raw), source))
            .filter(|(text, _)| text.contains(TYPED))
    });

    let Some((_, source)) = typed_seen else {
        let actual = content.read().map(|(raw, _)| normalise(&raw));
        return failed(&format!(
            "введённое не дошло до приложения: прочитано {:?}, ожидалось содержащее {TYPED:?}",
            actual.unwrap_or_else(|| "<чтение не удалось>".to_owned())
        ));
    };

    // What the product itself saw of those keystrokes, through SEC-04a. Not a verdict — an
    // observation that separates "the product never got the input" from "the product got it and
    // did not convert it", which are different findings and would otherwise look identical.
    let seen_by_product = crate::channel::read()
        .ok()
        .map(|snapshot| {
            format!(
                "buffer_len={}, hotkey_handoffs={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?")
            )
        })
        .unwrap_or_else(|| "канал не ответил".to_owned());

    // Step 6 — the hotkey. Position 22 supplies `Shift` here; everything else supplies nothing.
    let pressed = if modifiers.is_empty() {
        input::tap(ctx.hotkey_vk, &target)
    } else {
        input::chord(modifiers, ctx.hotkey_vk, &target)
    };
    if let Err(error) = pressed {
        return failed(&format!("горячая клавиша: {error}"));
    }

    // Step 7 — wait for the replacement, then read the layout.
    let replaced = wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| normalise(&raw))
            .filter(|text| text.contains(EXPECTED))
    });

    let final_text = replaced
        .clone()
        .or_else(|| content.read().map(|(raw, _)| normalise(&raw)));
    let shown = final_text
        .clone()
        .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    // ⚠ **Position 16 — the eighth step, and the only difference from the seven above.**
    //
    // Everything before this line has already happened for position 16 exactly as it happens
    // for position 1, including step 7's wait for `привет`. That wait *is* the wait between the
    // two presses (requirement 1 of §11.5): it is a condition and not a clock, and it is the
    // reason the second press can be trusted to be a second press rather than half of a race.
    if rollback {
        return rolled_back(
            ctx,
            &target,
            RollbackScene {
                position,
                app_name,
                window,
                content,
                modifiers,
                source_layout,
                first_press_gave: replaced.as_deref(),
                shown: &shown,
            },
        );
    }

    let text_row = Row::new(
        position,
        app_name,
        Assertion::Text,
        if replaced.is_some() {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        format!("{shown:?}"),
        format!("{EXPECTED:?}"),
    )
    .with_note(format!(
        "прочитано через {}; исходная раскладка окна {}; продукт до горячей клавиши: {}; \
         после: {}",
        source.as_str(),
        layout::describe(source_layout),
        seen_by_product,
        crate::channel::read()
            .ok()
            .map(|snapshot| format!(
                "buffer_len={}, hotkey_handoffs={}, post_failures={}, send_mismatches={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?"),
                snapshot.get("post_failures").unwrap_or("?"),
                snapshot.get("send_mismatches").unwrap_or("?"),
            ))
            .unwrap_or_else(|| "канал не ответил".to_owned())
    ));

    // The layout assertion — a verdict, not a deferral. The chain of §4.6 exists (FR-50 to
    // FR-52, task T-05-1, commit `c8e463f`) and the target it is given is the choice of §4.4
    // (task T-05-2), so the scenario of §11.3 can be asserted whole: `привет` **and** the RU
    // layout in the active window.
    //
    // The wait is on the condition and never on a clock — requirement 1 of §11.5. Step 5 of
    // FR-40 is sent after the replacement (FR-43), so the layout may still be the source one at
    // the instant the text arrives; the same timeout the text half uses is given to it, and it
    // is spent only when the switch really did not happen.
    let observed = wait::until(wait::TEXT_TIMEOUT, || {
        layout::of_window(window)
            .map(layout::id_of)
            .filter(|id| id & 0xFFFF == layout::RUSSIAN & 0xFFFF)
    })
    .or_else(|| layout::of_window(window).map(layout::id_of));

    let layout_row = Row::new(
        position,
        app_name,
        Assertion::Layout,
        match observed {
            Some(id) if id & 0xFFFF == layout::RUSSIAN & 0xFFFF => Verdict::Pass,
            _ => Verdict::Fail,
        },
        observed.map_or("<чтение не удалось>".to_owned(), layout::describe),
        layout::describe(layout::RUSSIAN),
    )
    .with_note(format!(
        "исходная раскладка окна {}; переключение — FR-40 шаг 5 и §4.6, цель выбрана по §4.4",
        layout::describe(source_layout)
    ));

    vec![text_row, layout_row]
}

// ---------------------------------------------------------------------------------------
// Position 16 — the eighth step: the rollback of FR-33
// ---------------------------------------------------------------------------------------

/// What [`rolled_back`] needs from the seven steps that have already run.
struct RollbackScene<'a> {
    position: u8,
    app_name: &'a str,
    window: HWND,
    content: &'a Element,
    /// Modifiers held over the hotkey — empty for position 16, and carried through so that the
    /// second press is the same gesture as the first.
    modifiers: &'a [u16],
    /// The layout the window was in before the first press: en-US, and the value the rollback
    /// has to bring it back to.
    source_layout: u32,
    /// What step 7 read after the first press, when it read `привет` — `None` when it never did.
    first_press_gave: Option<&'a str>,
    /// What is in the field now, whatever it is, for the failure message.
    shown: &'a str,
}

/// The second press of the hotkey, and the three assertions of position 16 — **FR-32, FR-33**.
///
/// # What is asserted, and why the third one exists
///
/// | Row | Expected |
/// |---|---|
/// | text | `ghbdtn` **bit for bit** — not "contains", see below |
/// | layout | back at `0x00000409`, en-US |
/// | `cycle_position` | `0` — the cycle of length two is back at its start |
///
/// The first two could both hold by accident: a product that never saw the second press at all
/// would leave the field holding `привет`, and one that flushed the buffer on some rule of FR-10
/// and then re-typed nothing would leave the text alone too. The third row is what tells "the
/// text happens to match" from "the program really came back to the start of the cycle", and it
/// is the reason task T-05-2a put `cycle_position` on the channel of SEC-04a in the first place.
///
/// # Bit for bit, and what that means through UI Automation
///
/// FR-32: "Возврат к началу цикла восстанавливает исходный текст побитово точно." The comparison
/// is therefore `==` against the whole reading and **not** `contains`: `contains` would accept
/// `привет ghbdtn`, which is the shape a replacement that appended instead of replacing would
/// leave, and that is precisely the failure FR-32 is about. What is compared is the reading
/// after [`normalise`], which folds the line breaks and the padding characters the providers add
/// and touches nothing else; the raw reading goes into the row's note either way, so a
/// disagreement can be read rather than guessed at.
fn rolled_back(ctx: &Context, target: &input::Target, scene: RollbackScene<'_>) -> Vec<Row> {
    let RollbackScene {
        position,
        app_name,
        window,
        content,
        modifiers,
        source_layout,
        first_press_gave,
        shown,
    } = scene;

    // ⚠ Requirement 1 of §11.5. The first press must have landed before the second is sent, and
    // "landed" is a condition — step 7's wait for `привет`, already spent by the caller — and
    // never a pause. Without it the bench would be measuring the race between two replacements.
    let Some(after_first) = first_press_gave else {
        return rollback_failed(
            position,
            app_name,
            &format!(
                "первое нажатие не дало {EXPECTED:?}, второе не отправлялось: прочитано {shown:?}"
            ),
        );
    };

    // The second press — the same gesture as the first, modifiers included.
    let pressed = if modifiers.is_empty() {
        input::tap(ctx.hotkey_vk, target)
    } else {
        input::chord(modifiers, ctx.hotkey_vk, target)
    };
    if let Err(error) = pressed {
        return rollback_failed(position, app_name, &format!("второе нажатие: {error}"));
    }

    // Step 7 again, for the *original* text this time. A condition, not a clock.
    let restored = wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| (normalise(&raw), raw))
            .filter(|(text, _)| text == TYPED)
    });

    let (shown_now, raw_now) = restored
        .clone()
        .or_else(|| content.read().map(|(raw, _)| (normalise(&raw), raw)))
        .unwrap_or_else(|| ("<чтение не удалось>".to_owned(), String::new()));

    // The layout the rollback has to have brought back. The same wait-on-a-condition the
    // ordinary scenario gives the switch of FR-40 step 5: the switch follows the replacement
    // (FR-43), so the layout may still be RU at the instant the text arrives.
    let observed = wait::until(wait::TEXT_TIMEOUT, || {
        layout::of_window(window)
            .map(layout::id_of)
            .filter(|id| id & 0xFFFF == layout::US & 0xFFFF)
    })
    .or_else(|| layout::of_window(window).map(layout::id_of));

    // And what the product itself says about where the cycle is — the key task T-05-2a added.
    let snapshot = crate::channel::read();
    let cycle = snapshot
        .as_ref()
        .ok()
        .and_then(|snapshot| snapshot.get(CYCLE_KEY).map(str::to_owned));
    let counters = snapshot
        .as_ref()
        .ok()
        .map(|snapshot| {
            format!(
                "buffer_len={}, hotkey_handoffs={}, post_failures={}, send_mismatches={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?"),
                snapshot.get("post_failures").unwrap_or("?"),
                snapshot.get("send_mismatches").unwrap_or("?"),
            )
        })
        .unwrap_or_else(|| "канал не ответил".to_owned());

    let mut rows = vec![
        Row::new(
            position,
            app_name,
            Assertion::Text,
            if restored.is_some() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            format!("{shown_now:?}"),
            format!("{TYPED:?} побитово"),
        )
        .with_note(format!(
            "после первого нажатия {after_first:?}; сырое чтение после второго {raw_now:?}; \
             сравнение — равенство целиком, не «содержит» (FR-32); продукт: {counters}"
        )),
        Row::new(
            position,
            app_name,
            Assertion::Layout,
            match observed {
                Some(id) if id & 0xFFFF == layout::US & 0xFFFF => Verdict::Pass,
                _ => Verdict::Fail,
            },
            observed.map_or("<чтение не удалось>".to_owned(), layout::describe),
            layout::describe(layout::US),
        )
        .with_note(format!(
            "исходная раскладка окна {}; после двух нажатий цикл длины 2 возвращает её же \
             (FR-33), переключение — FR-40 шаг 5 и §4.6",
            layout::describe(source_layout)
        )),
        Row::new(
            position,
            app_name,
            Assertion::Other(CYCLE_ASSERTION),
            match cycle.as_deref() {
                Some("0") => Verdict::Pass,
                _ => Verdict::Fail,
            },
            match &cycle {
                Some(value) => format!("{CYCLE_KEY}={value}"),
                None => match &snapshot {
                    Ok(_) => format!("ключ {CYCLE_KEY} в снимке отсутствует"),
                    Err(error) => format!("канал SEC-04a не ответил: {error}"),
                },
            },
            format!("{CYCLE_KEY}=0"),
        )
        .with_note(format!(
            "FR-32, FR-33: цикл длины 2 вернулся в начало; ключ канала — задача T-05-2a; \
             прочие счётчики: {counters}"
        )),
    ];

    // ⚠ **The blind spot of §11.5, closed by task T-10-6.** Everything above this line presses
    // the hotkey **twice**, on **one** word, in a field that held nothing else. The user broke
    // the product with the household version of the same gesture — several words in one window,
    // spaces between them, four presses on the second word — and every position of the matrix
    // gave `pass` while it did. The series below is that gesture, as assertions of this position.
    rows.extend(series(
        ctx,
        target,
        position,
        app_name,
        content,
        shown_now.as_str(),
    ));

    rows
}

/// The key of the SEC-04a channel this position rests on — task T-05-2a.
const CYCLE_KEY: &str = "cycle_position";

/// The shape of the last replacement packet, `erase/units/distinct` — task T-10-6.
///
/// The key that separates "the product built a collapsed packet" from "the product built a
/// correct one and what is on the screen is somebody else's doing". Read after every press of the
/// series, because a reading taken only at the end would be the shape of the last press alone.
const PACKET_KEY: &str = "last_replacement";

// ---------------------------------------------------------------------------------------
// Position 16, the series of task T-10-6 — several words, spaces, four presses
// ---------------------------------------------------------------------------------------

/// How many presses the series makes on the **second** word.
///
/// Four, because four is what the user made and four is where the defect showed its whole shape:
/// press 2 collapsed, press 3 was right but a press late, press 4 collapsed again and took the
/// word with it. Three would have shown the first two of those and hidden the rest.
const SERIES_PRESSES: usize = 4;

/// One report row per press of the series. Named rather than numbered in a loop, because
/// [`Assertion::Other`] takes a `&'static str` and a row whose name is built at runtime cannot be
/// compared between two runs of the bench.
const SERIES_ROWS: [&str; SERIES_PRESSES] = [
    "серия: второе слово, нажатие 1",
    "серия: второе слово, нажатие 2",
    "серия: второе слово, нажатие 3",
    "серия: второе слово, нажатие 4",
];

/// The row that answers criterion 13 of the task over the whole series at once.
const SERIES_SHAPE: &str = "серия: ни схлопывания, ни стирания без вставки";

/// The row that says the series ran at all.
const SERIES_SETUP: &str = "серия: второе слово набрано";

/// Bound on the wait for one press of the series to reach the field.
///
/// **A bound on a condition, not a delay** — requirement 1 of §11.5. The condition is "the field
/// no longer reads what it read before the press", which a replacement satisfies in tens of
/// milliseconds whether the replacement is right or wrong, so the bound is spent only when the
/// product did nothing at all. Shorter than [`wait::TEXT_TIMEOUT`] on purpose: four presses that
/// each spent ten seconds would outlive `LANGSW_DEBUG_TIMEOUT_SEC` and the product would die
/// mid-position, which reports as a defect of the product and is a defect of the bench.
const SERIES_STEP_TIMEOUT: Duration = Duration::from_secs(5);

/// What the field is expected to read after press `press` of the series, given `first` — the word
/// already in the field, which the series never touches.
///
/// Odd presses convert (`привет`), even ones roll back (`ghbdtn`) — FR-33 over a cycle of length
/// two. The **whole** field is compared and not the last word of it: a press that erased the word
/// without typing anything back would leave the first word alone in the field, and a comparison
/// of last words would read that as a correct rollback. Criterion 13 of the task is precisely
/// about not doing so.
fn series_expected(first: &str, press: usize) -> String {
    let word = if press % 2 == 1 { EXPECTED } else { TYPED };
    format!("{first} {word}")
}

/// Whether `text` is a run of one character repeated — the shape of the defect, `nnnnnn`.
///
/// Two or more characters, all equal. One character is not a collapse of anything, and the empty
/// string is the erasure the neighbouring check answers.
fn is_collapsed(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    text.chars().count() > 1 && characters.all(|character| character == first)
}

/// The keys of SEC-04a this task reads after **every** press.
fn series_channel() -> String {
    crate::channel::read()
        .ok()
        .map(|snapshot| {
            format!(
                "buffer_len={}, cycle_position={}, active_layout={}, hotkey_handoffs={}, \
                 send_mismatches={}, events_lost={}, replacement_method={}, {PACKET_KEY}={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get(CYCLE_KEY).unwrap_or("?"),
                snapshot.get("active_layout").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?"),
                snapshot.get("send_mismatches").unwrap_or("?"),
                snapshot.get("events_lost").unwrap_or("?"),
                snapshot.get("replacement_method").unwrap_or("?"),
                snapshot.get(PACKET_KEY).unwrap_or("?"),
            )
        })
        .unwrap_or_else(|| "канал не ответил".to_owned())
}

/// Reads the field, normalised, or `None` while the provider is not answering.
fn read_field(content: &Element) -> Option<String> {
    content.read().map(|(raw, _)| normalise(&raw))
}

/// Types `text` one character at a time, each character confirmed in the field before the next
/// one is sent — **the tempo of a person**, expressed as a condition.
///
/// The bench's [`input::type_text`] hands the whole word to one `SendInput` call, which is the
/// one thing the user's scenario did not do. Rather than sleep between keystrokes — a fixed delay,
/// which requirement 1 of §11.5 forbids — each character is followed by a wait for the field to
/// read it back. The pace that results is the round trip of UI Automation, tens of milliseconds,
/// which is inside the range a person types in, and the wait is a condition throughout.
fn type_paced(
    text: &str,
    target: &input::Target,
    content: &Element,
    prefix: &str,
) -> Result<(), String> {
    let mut expected = prefix.to_owned();

    for character in text.chars() {
        let mut one = [0u8; 4];
        let one = character.encode_utf8(&mut one);
        input::type_text(one, target).map_err(|error| format!("ввод {one:?}: {error}"))?;

        expected.push(character);
        let landed = wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(content).filter(|text| text == &expected)
        });

        if landed.is_none() {
            return Err(format!(
                "символ {one:?} не дошёл до приложения: прочитано {:?}, ожидалось {expected:?}",
                read_field(content).unwrap_or_else(|| "<чтение не удалось>".to_owned())
            ));
        }
    }

    Ok(())
}

/// A setup failure of the series, in the shape of the rows the series would have produced.
fn series_failed(position: u8, app_name: &str, reason: &str) -> Vec<Row> {
    let mut rows = vec![
        Row::new(
            position,
            app_name,
            Assertion::Other(SERIES_SETUP),
            Verdict::Fail,
            reason.to_owned(),
            format!("второе слово {TYPED:?} в том же поле, через пробел"),
        )
        .with_note(format!(
            "серия T-10-6 не начиналась; продукт: {}",
            series_channel()
        )),
    ];

    for (index, name) in SERIES_ROWS.iter().enumerate() {
        rows.push(Row::new(
            position,
            app_name,
            Assertion::Other(name),
            Verdict::Fail,
            "нажатие не отправлялось".to_owned(),
            series_expected(TYPED, index + 1),
        ));
    }

    rows.push(Row::new(
        position,
        app_name,
        Assertion::Other(SERIES_SHAPE),
        Verdict::Fail,
        "серия не исполнялась".to_owned(),
        "ни одного чтения из повторов одного символа и ни одного без вставки".to_owned(),
    ));

    rows
}

/// **The scenario the user broke the product with** — several words in one field, a space between
/// them, four presses of the hotkey on the second word.
///
/// # Why it is here and not a row of its own in §11.3
///
/// Because §11.3 is `SPEC.md` and a new line of that matrix is the user's to write. The class of
/// defect is nevertheless the one the matrix was blind to, so it enters the bench the only way it
/// may: as further assertions of a position that already exists. Position 16 rather than 1
/// because what the series measures is FR-32 and FR-33 — the alternation of conversion and
/// rollback — over a run longer than the two presses this position used to make.
///
/// # What is asserted
///
/// | Row | Expected |
/// |---|---|
/// | setup | the second word reaches the field, through a real `Space` |
/// | press 1…4 | the **whole** field, bit for bit: `ghbdtn привет` and `ghbdtn ghbdtn` in turn |
/// | shape | no reading is one character repeated, and no press erased without typing back |
///
/// The space is sent as `VK_SPACE` and not through [`input::type_text`], which would fall back to
/// a `KEYEVENTF_UNICODE` event for it: a unicode event carries `wVk = 0`, the product would record
/// it as `VK_PACKET` rather than as the boundary key of FR-10, and the buffer would **not** be
/// flushed between the two words. The whole point of the scenario is that it is.
fn series(
    ctx: &Context,
    target: &input::Target,
    position: u8,
    app_name: &str,
    content: &Element,
    after_rollback: &str,
) -> Vec<Row> {
    let failed = |reason: &str| series_failed(position, app_name, reason);

    // The word the rollback left in the field. The series never touches it, and every expected
    // reading below is built from it, so a first word that is not what this position thinks it is
    // ends the series here instead of producing rows about the wrong thing.
    if after_rollback != TYPED {
        return failed(&format!(
            "перед серией поле читается {after_rollback:?}, а не {TYPED:?}"
        ));
    }

    // The boundary key of FR-10, and the first half of "несколько слов, пробелы между ними".
    if let Err(error) = input::tap(VK_SPACE.0, target) {
        return failed(&format!("пробел между словами: {error}"));
    }

    // `normalise` trims, so the space alone is invisible in the reading; the second word is what
    // makes it appear, and `type_paced` waits for each of its characters in turn.
    if let Err(reason) = type_paced(TYPED, target, content, &format!("{after_rollback} ")) {
        return failed(&reason);
    }

    let mut rows = vec![
        Row::new(
            position,
            app_name,
            Assertion::Other(SERIES_SETUP),
            Verdict::Pass,
            format!("{:?}", format!("{after_rollback} {TYPED}")),
            format!("второе слово {TYPED:?} в том же поле, через пробел"),
        )
        .with_note(format!(
            "набрано посимвольно, каждый символ подтверждён чтением поля; продукт: {}",
            series_channel()
        )),
    ];

    let mut collapsed: Vec<String> = Vec::new();
    let mut erased: Vec<String> = Vec::new();

    for (index, name) in SERIES_ROWS.iter().enumerate() {
        let press = index + 1;
        let expected = series_expected(after_rollback, press);

        let before = read_field(content).unwrap_or_default();

        if let Err(error) = input::tap(ctx.hotkey_vk, target) {
            rows.push(
                Row::new(
                    position,
                    app_name,
                    Assertion::Other(name),
                    Verdict::Fail,
                    format!("нажатие {press}: {error}"),
                    expected,
                )
                .with_note(format!("продукт: {}", series_channel())),
            );
            continue;
        }

        // ⚠ A condition, not a clock, and the condition is the **expected** reading rather than
        // "the field changed".
        //
        // The first version of this loop waited for a change, and it caught the packet of FR-41
        // in flight: `ghbdtn ghbdtn` had become `ghbdtn ghbdt` — one `Backspace` in — and the row
        // reported that as the product's answer. A press that succeeds satisfies the condition
        // below at once; a press that fails spends the bound, by which time the packet has long
        // finished and the reading that follows is the settled one.
        let landed = wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(content).filter(|text| text == &expected)
        });

        let shown = landed
            .or_else(|| read_field(content))
            .unwrap_or_else(|| "<чтение не удалось>".to_owned());
        let channel = series_channel();

        // The two shapes criterion 13 names, judged on the word this series is pressing on — the
        // last one in the field — and collected for the aggregate row below.
        let word = shown.split_whitespace().last().unwrap_or("").to_owned();
        if is_collapsed(&word) {
            collapsed.push(format!("нажатие {press}: {word:?}"));
        }
        if shown.split_whitespace().count() < 2 {
            erased.push(format!("нажатие {press}: {shown:?}"));
        }

        rows.push(
            Row::new(
                position,
                app_name,
                Assertion::Other(name),
                if shown == expected {
                    Verdict::Pass
                } else {
                    Verdict::Fail
                },
                format!("{shown:?}"),
                format!("{expected:?} побитово"),
            )
            .with_note(format!(
                "до нажатия {before:?}; чередование FR-33 по циклу длины 2; продукт после \
                 нажатия: {channel}"
            )),
        );
    }

    let mut shape = Vec::new();
    if collapsed.is_empty() {
        shape.push("схлопываний нет".to_owned());
    } else {
        shape.push(format!("СХЛОПЫВАНИЕ — {}", collapsed.join("; ")));
    }
    if erased.is_empty() {
        shape.push("стираний без вставки нет".to_owned());
    } else {
        shape.push(format!("СТИРАНИЕ БЕЗ ВСТАВКИ — {}", erased.join("; ")));
    }

    rows.push(
        Row::new(
            position,
            app_name,
            Assertion::Other(SERIES_SHAPE),
            if collapsed.is_empty() && erased.is_empty() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            shape.join("; "),
            "ни одного чтения из повторов одного символа и ни одного без вставки".to_owned(),
        )
        .with_note(format!(
            "FR-41, FR-45 и суть дефекта T-10-6: чётное нажатие давало шесть копий последнего \
             символа целевого текста; продукт в конце серии: {}",
            series_channel()
        )),
    );

    rows
}

/// Runs a position end to end: launch, find, replace, restore.
/// What a position needs beyond the command that starts it.
struct Plan<'a> {
    number: u8,
    name: &'a str,
    /// Recognises the top-level window of this application among all of them.
    window_is: &'a dyn Fn(&Element) -> bool,
    /// Finds the element the scenario types into and reads back.
    content_in: &'a dyn Fn(&Automation, &Element) -> Option<Element>,
    modifiers: &'a [u16],
    clear_first: bool,
    /// **Position 16 only** — see [`Scene::rollback`].
    rollback: bool,
}

/// Runs a position end to end: launch, find the window, find the content, replace, restore.
fn run_position(
    ctx: &Context,
    plan: Plan<'_>,
    launch: impl FnOnce() -> Result<App, String>,
) -> Vec<Row> {
    // Requirement 5: the clipboard is captured before the scenario and put back by the guard's
    // `Drop`, which also covers the path where the scenario ends early.
    let _clipboard = clip::Guard::capture();

    // A position that could not even be set up still has to report itself in **its own** shape:
    // two rows expecting `привет` for the ordinary scenario, three expecting the original text
    // for the rollback of position 16. `replacement` already chooses between them for failures
    // that happen inside it; before this line the choice was `both_failed` for everybody, so a
    // position 16 that never found its window claimed to have expected `привет`.
    let failed = |reason: &str| -> Vec<Row> {
        if plan.rollback {
            rollback_failed(plan.number, plan.name, reason)
        } else {
            both_failed(plan.number, plan.name, reason)
        }
    };

    let mut app = match launch() {
        Ok(app) => app,
        Err(error) => return failed(&format!("запуск: {error}")),
    };

    let mut rows = match adopt_window(ctx.automation, &mut app, plan.window_is) {
        Err(reason) => failed(&reason),
        Ok(window) => match (app.window, (plan.content_in)(ctx.automation, &window)) {
            (None, _) => failed("у окна нет дескриптора"),
            (_, None) => failed("элемент ввода не найден в дереве UI Automation"),
            (Some(hwnd), Some(content)) => replacement(
                ctx,
                Scene {
                    position: plan.number,
                    app_name: plan.name,
                    pid: app.pid,
                    window: hwnd,
                    content: &content,
                    modifiers: plan.modifiers,
                    clear_first: plan.clear_first,
                    rollback: plan.rollback,
                },
            ),
        },
    };

    let closed = app.close();
    for row in &mut rows {
        row.note = format!("{}; закрытие: {closed}", row.note);
    }

    rows
}

// ---------------------------------------------------------------------------------------
// Task T-10-6 — the delivery experiment, and what it separates
// ---------------------------------------------------------------------------------------

/// **The experiment that asks whether `KEYEVENTF_UNICODE` alone can produce the collapse** —
/// task T-10-6, and the product takes no part in it.
///
/// The channel says the product builds a packet of six **distinct** code units
/// (`last_replacement=6/6/6`) on the very presses whose result on the screen is six copies of one
/// character. That leaves exactly one place for the collapse to be: between `SendInput` and the
/// application. This experiment puts the bench in the product's place — the same shape of packet,
/// the same window — and reads the field back, so that "the packet is delivered as six copies"
/// stops being an inference and becomes a reading.
///
/// Three forms, ten runs each, in one Notepad:
///
/// 1. **one call** — all six characters in a single `SendInput`, which is the shape FR-41 requires
///    of the product's own packet;
/// 2. **six calls** — the same six characters, one `SendInput` each, which is what a person's
///    keyboard produces and what FR-44 offers as an opt-in;
/// 3. **one call, then the layout switch** — the same single call followed at once by
///    `WM_INPUTLANGCHANGEREQUEST` to that same window, which is method 1 of FR-50 and therefore
///    **the order FR-43 fixes**: the packet is sent, and then step 5 moves the layout.
///
/// Nothing here is a verdict of the matrix: it prints and returns an exit code, exactly as the
/// other experiments of `e2e.rs` do.
pub fn experiment_unicode(ctx: &Context) -> std::process::ExitCode {
    const RUNS: usize = 10;

    println!("--- ОПЫТ T-10-6: доставка KEYEVENTF_UNICODE без участия продукта ---\n");
    println!(
        "Стенд шлёт {EXPECTED:?} парами down+up с чужой сигнатурой в собственный Блокнот.\n\
         Форма 3 добавляет ровно то, что делает шаг 5 FR-40 после пакета.\n"
    );

    let _clipboard = clip::Guard::capture();

    let mut app = match launch_notepad() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Блокнот не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let outcome = (|| -> Result<([usize; 3], [usize; 3]), String> {
        let window = adopt_window(ctx.automation, &mut app, &|element: &Element| {
            element.class() == "Notepad"
        })?;
        let hwnd = app
            .window
            .ok_or_else(|| "у окна нет дескриптора".to_owned())?;
        let content = ctx
            .automation
            .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e))
            .ok_or_else(|| "элемент ввода не найден".to_owned())?;

        shell::activate_window(app.pid, Some(hwnd))?;
        let target = input::Target { pid: app.pid, hwnd };

        let forms: [&str; 3] = [
            "один вызов          ",
            "шесть вызовов       ",
            "вызов + раскладка   ",
        ];
        let mut collapsed = [0usize; 3];
        let mut wrong = [0usize; 3];

        for run in 1..=RUNS {
            for (form, name) in forms.iter().enumerate() {
                input::chord(&[VK_CONTROL.0], VK_A.0, &target).map_err(|e| e.to_string())?;
                input::tap(VK_DELETE.0, &target).map_err(|e| e.to_string())?;
                wait::until(SERIES_STEP_TIMEOUT, || {
                    read_field(&content).filter(|text| text.is_empty())
                });

                // The window is put back onto en-US before every run, so that form 3's switch is
                // always a real change of layout and never the `AlreadyActive` short circuit.
                layout::ensure(hwnd, layout::US, SERIES_STEP_TIMEOUT)?;

                if form == 1 {
                    for character in EXPECTED.chars() {
                        let mut one = [0u8; 4];
                        input::type_text(character.encode_utf8(&mut one), &target)
                            .map_err(|e| e.to_string())?;
                    }
                } else {
                    input::type_text(EXPECTED, &target).map_err(|e| e.to_string())?;
                }

                // ⚠ Form 3 and nothing else: `WM_INPUTLANGCHANGEREQUEST` to the window that has
                // just been sent the packet, with nothing waited for in between — which is where
                // FR-43 puts step 5 of FR-40.
                if form == 2 {
                    let _ = layout::ensure(hwnd, layout::RUSSIAN, SERIES_STEP_TIMEOUT);
                }

                let shown = wait::until(SERIES_STEP_TIMEOUT, || {
                    read_field(&content).filter(|text| text == EXPECTED)
                })
                .or_else(|| read_field(&content))
                .unwrap_or_else(|| "<чтение не удалось>".to_owned());

                let verdict = if shown == EXPECTED {
                    "верно"
                } else if is_collapsed(&shown) {
                    collapsed[form] += 1;
                    "СХЛОПЫВАНИЕ"
                } else {
                    wrong[form] += 1;
                    "иное"
                };
                println!("  прогон {run:2}  {name}  {shown:?}  — {verdict}");
            }
        }

        // ---- form 4: the product's packet, without the product ------------------------------
        //
        // Forms 1 to 3 send the insertion alone into an empty field. The product's press does
        // something else: it takes `N` characters **off** a field that already holds text and puts
        // the conversion in their place, all in one call, over and over on the same word. This
        // walks that, four alternating bursts on a second word, with `U+0008` standing in for the
        // `Backspace` of FR-41 — an `EDIT` and a `RichEdit` both delete a character on receiving
        // it, and it is the only way to put an erasure and an insertion into a single `SendInput`
        // through the bench's one and only send function.
        println!("\n  --- форма 4: стирание и вставка одним вызовом, как в пакете FR-41 ---");

        input::chord(&[VK_CONTROL.0], VK_A.0, &target).map_err(|e| e.to_string())?;
        input::tap(VK_DELETE.0, &target).map_err(|e| e.to_string())?;
        layout::ensure(hwnd, layout::US, SERIES_STEP_TIMEOUT)?;
        input::type_text(&format!("{TYPED} {TYPED}"), &target).map_err(|e| e.to_string())?;
        wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(&content).filter(|text| text == &format!("{TYPED} {TYPED}"))
        });

        let erase: String = core::iter::repeat_n('\u{8}', TYPED.chars().count()).collect();
        let mut collapsed_burst = 0;

        for round in 1..=RUNS {
            let word = if round % 2 == 1 { EXPECTED } else { TYPED };
            let want = format!("{TYPED} {word}");

            input::type_text(&format!("{erase}{word}"), &target).map_err(|e| e.to_string())?;

            let shown = wait::until(SERIES_STEP_TIMEOUT, || {
                read_field(&content).filter(|text| text == &want)
            })
            .or_else(|| read_field(&content))
            .unwrap_or_else(|| "<чтение не удалось>".to_owned());

            let last = shown.split_whitespace().last().unwrap_or("").to_owned();
            let verdict = if shown == want {
                "верно"
            } else if is_collapsed(&last) {
                collapsed_burst += 1;
                "СХЛОПЫВАНИЕ"
            } else {
                "иное"
            };
            println!("  бросок {round:2}  стирание+вставка   {shown:?}  — {verdict}");
        }

        println!("  схлопываний в форме 4: {collapsed_burst} из {RUNS}");

        Ok((collapsed, wrong))
    })();

    let closed = app.close();

    match outcome {
        Ok((collapsed, wrong)) => {
            println!("\nИТОГО из {RUNS} на форму:");
            for (index, name) in ["один вызов", "шесть вызовов", "вызов + раскладка"]
                .iter()
                .enumerate()
            {
                println!(
                    "  {name:<20} схлопываний {}, иных расхождений {}",
                    collapsed[index], wrong[index]
                );
            }
            println!("закрытие: {closed}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("\nопыт не доведён: {error}\nзакрытие: {closed}");
            std::process::ExitCode::from(1)
        }
    }
}

/// **The experiment that asks which dial of §7 the collapse answers to** — task T-10-6.
///
/// Position 16 with its series, run three times against three configurations of the **same**
/// product: as the user has it, with the pause of FR-44 at one millisecond, and in the
/// compatibility mode of FR-42. The three differ in exactly one thing each, and between them they
/// separate «пакет уходит одним броском» from «стирание идёт `Backspace`'ами» — the two shapes the
/// packet of FR-41 can have.
///
/// The user's `config.toml` is borrowed through [`crate::config::Borrowed`], which is the one
/// mechanism in this bench allowed to touch it, and given back byte for byte on every path out.
pub fn experiment_modes(ctx: &Context) -> std::process::ExitCode {
    println!("--- ОПЫТ T-10-6: серия под тремя настройками §7 ---\n");

    let Some(path) = lang_switcher::settings::default_config_path() else {
        eprintln!("путь %APPDATA%\\Lang_Switcher\\config.toml не определён");
        return std::process::ExitCode::from(1);
    };

    let variants: [(
        &str,
        Option<(u32, lang_switcher::settings::ReplacementMethod)>,
    ); 3] = [
        ("как есть (умолчания §7)", None),
        (
            "пауза FR-44 = 1 мс",
            Some((1, lang_switcher::settings::ReplacementMethod::Backspace)),
        ),
        (
            "режим FR-42 selection",
            Some((0, lang_switcher::settings::ReplacementMethod::Selection)),
        ),
    ];

    for (name, dial) in variants {
        println!("\n=== {name} ===");

        let mut borrowed = match dial {
            None => None,
            Some((delay, method)) => {
                let (mut config, _) = lang_switcher::settings::read_or_default(&path);
                config.replacement.inter_event_delay_ms = delay;
                config.replacement.method = method;

                let text = match config.to_toml_string() {
                    Ok(text) => text,
                    Err(error) => {
                        eprintln!("  не удалось построить config.toml: {error}");
                        continue;
                    }
                };

                match crate::config::Borrowed::take(&path, &text) {
                    Ok(borrowed) => Some(borrowed),
                    Err(error) => {
                        eprintln!("  подмена config.toml: {error}");
                        continue;
                    }
                }
            }
        };

        let rows = match crate::sut::Sut::launch() {
            Err(error) => {
                eprintln!("  продукт не запустился: {error}");
                Vec::new()
            }
            Ok(mut product) => {
                let rows = if product.await_ready(Duration::from_secs(30)).is_none() {
                    eprintln!("  продукт не сообщил о готовности за 30 с");
                    Vec::new()
                } else {
                    position_16(ctx)
                };
                match product.stop() {
                    Ok(code) => println!("  продукт остановлен, код {code}"),
                    Err(error) => println!("  ⚠ {error}"),
                }
                rows
            }
        };

        if let Some(borrowed) = borrowed.as_mut() {
            println!("  {}", borrowed.give_back());
        }

        let collapses = rows
            .iter()
            .filter(|row| matches!(row.assertion, Assertion::Other(SERIES_SHAPE)))
            .map(|row| row.actual.clone())
            .collect::<Vec<_>>()
            .join(" | ");

        for row in &rows {
            println!(
                "  [{}] {} — {}",
                row.assertion.as_str(),
                row.verdict,
                row.actual
            );
        }
        println!("  сводка формы: {collapses}");

        println!("  {}", restore_ambient(ctx));
    }

    std::process::ExitCode::SUCCESS
}

/// An `Edit` or a `Document` — what most of the matrix types into.
fn text_field(element: &Element) -> bool {
    let kind = element.control_type();
    kind == Some(UIA_EditControlTypeId) || kind == Some(UIA_DocumentControlTypeId)
}

// ---------------------------------------------------------------------------------------
// Position 1 — Notepad
// ---------------------------------------------------------------------------------------

/// ⚠ `notepad.exe` in System32 is a **stub**: it hands over to the packaged Notepad and exits,
/// so the window belongs to a process the bench never spawned. The window is therefore found
/// by class among all top-level windows and its process adopted — see `App::adopt`.
pub fn position_1(ctx: &Context) -> Vec<Row> {
    notepad_position(ctx, 1, "Блокнот", &[], false)
}

/// Position 22 is the same application with `Shift` held over the hotkey.
pub fn position_22(ctx: &Context) -> Vec<Row> {
    notepad_position(
        ctx,
        22,
        "Модификаторы: Shift + горячая клавиша (Блокнот)",
        &[VK_SHIFT.0],
        false,
    )
}

/// Position 16 — **the rollback of FR-33**: the hotkey twice, and the original text back.
///
/// # Why Notepad, and why that is not a shortcut
///
/// Position 1's application, deliberately. It is already launched, adopted and closed by code
/// that has been passing for two tasks, and position 16 is not a test of an application: it is a
/// test of a **property of the product** — that the buffer keeps the strokes the user really
/// made (FR-32) and that walking the cycle of length two round to its start restores them
/// (FR-33). Choosing an application whose text field is known to work is what keeps the verdict
/// about that property rather than about UI Automation.
///
/// The three assertions and the reason there are three of them are in [`rolled_back`].
///
/// # The two lines of wiring this function waited for — decision Р-52
///
/// Task T-05-2a wrote the scenario and could not run it: the list of positions and the `match`
/// that maps a number to a function both live in `tests\e2e\e2e.rs`, and that file was outside
/// its permissions, so it stopped at the boundary and wrote the two lines into its report
/// instead of widening its own area. That was an error of the controller's boundaries rather
/// than an omission of the task — decision Р-52 — and task T-04-3-3 added the list entry and the
/// arm, which is where the `allow(dead_code)` that used to sit here went.
pub fn position_16(ctx: &Context) -> Vec<Row> {
    notepad_position(ctx, 16, "Откат двойным нажатием (Блокнот)", &[], true)
}

fn notepad_position(
    ctx: &Context,
    number: u8,
    name: &str,
    modifiers: &[u16],
    rollback: bool,
) -> Vec<Row> {
    run_position(
        ctx,
        Plan {
            number,
            name,
            window_is: &|element: &Element| element.class() == "Notepad",
            content_in: &|automation: &Automation, window: &Element| {
                automation.await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e))
            },
            modifiers,
            clear_first: true,
            rollback,
        },
        launch_notepad,
    )
}

/// Starts Notepad for the positions that type into it — 1, 16, 22, and 23.
fn launch_notepad() -> Result<App, String> {
    let child = Command::new("notepad.exe")
        .spawn()
        .map_err(|error| error.to_string())?;
    // Terminating is safe here and it is not rake 5: unlike Word, Notepad keeps no
    // recovery state that a kill would poison, and the document is the bench's own six
    // characters. `WM_CLOSE` would raise a save prompt and leave a modal window behind.
    Ok(launched("Блокнот", child, CloseWith::Terminate, None))
}

// ---------------------------------------------------------------------------------------
// Position 23 — performance: ≥10 000 presses, the callback percentiles, the working set
// ---------------------------------------------------------------------------------------

/// The application name of position 23's three rows.
const APP_23: &str = "Производительность (Блокнот)";

/// Presses of the main volley. The matrix says ten thousand; the extra two hundred are
/// margin, not generosity — a criterion met exactly is a criterion one lost event away from
/// unmet.
const VOLLEY_PRESSES: usize = 10_200;

/// Presses per `SendInput` batch — [`input::type_text`] sends each batch as one call, and
/// the foreground guard inside it decides per batch: a stolen foreground refuses the whole
/// remainder instead of typing into a stranger's window.
const BATCH_PRESSES: usize = 300;

/// Presses spent before the first memory snapshot, so that one-time costs — the first
/// channel connections, the first pages the input path touches — are paid before the
/// growth this position judges is measured.
const WARMUP_PRESSES: usize = 300;

/// How much the product's working set may grow over the volley and still count as
/// «не растёт».
///
/// Not zero, because a working set is page-granular and the channel's server touches its
/// stack on every one of this position's reads. 256 KiB is 64 pages of noise allowance —
/// while a leak of even 26 bytes per press, summed over the volley, would already blow it.
const WORKING_SET_TOLERANCE: u64 = 256 * 1024;

/// What the three rows assert, in the report.
const P99_ASSERTION: &str = "p50/p99 callback — NFR-01";
const MAX_ASSERTION: &str = "максимум callback — NFR-02";
const MEMORY_ASSERTION: &str = "рабочий набор не растёт";

/// `PROCESS_MEMORY_COUNTERS` of `psapi.h`, declared by hand.
///
/// By hand and not through the `windows` crate, because the binding lives in the feature
/// `Win32_System_ProcessStatus`, which is **outside the closed list of §3.2** — and that
/// list is closed as the enforcement mechanism of SEC-03, the same reasoning footnote 4 of
/// §11.3 applied to `Win32_System_StationsAndDesktops`. A hand declaration adds no crate
/// and no feature: the function is exported by `kernel32.dll`, which every Windows process
/// links already. Field names are this file's own; the layout is the contract.
#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

// SAFETY: the declaration matches the documented export of kernel32 — `K32GetProcessMemoryInfo`
// is the kernel32 export behind psapi's `GetProcessMemoryInfo`, same signature, available
// since Windows 7. The stdcall convention is what `extern "system"` means on this target.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn K32GetProcessMemoryInfo(
        process: HANDLE,
        counters: *mut ProcessMemoryCounters,
        cb: u32,
    ) -> i32;
}

/// One reading of a process's memory, the fields position 23 judges and reports.
#[derive(Clone, Copy)]
struct MemoryReading {
    /// The working set, in bytes — the number the matrix row is about.
    working_set: u64,
    /// Its peak, reported beside the verdict, never judged.
    peak_working_set: u64,
    /// Committed private bytes (`PagefileUsage`), reported for context.
    commit: u64,
}

/// Reads the product's memory through `GetProcessMemoryInfo`, by process id.
fn process_memory(pid: u32) -> Result<MemoryReading, String> {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: `OpenProcess` takes three plain values and returns a handle or an error; it
    // dereferences nothing of ours. The limited-information right is the least one that
    // satisfies `K32GetProcessMemoryInfo`. NFR-13: the binding surfaces failure as `Err`.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|error| format!("OpenProcess({pid}): {error}"))?;

    let mut counters = ProcessMemoryCounters {
        cb: u32::try_from(std::mem::size_of::<ProcessMemoryCounters>()).unwrap_or(0),
        ..ProcessMemoryCounters::default()
    };

    // SAFETY: `handle` is the live handle opened above with the right this call requires;
    // `counters` is a live local whose true size is passed as `cb`, so the call cannot write
    // past it. The return is examined below (NFR-13).
    let ok = unsafe { K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) };
    let failure = std::io::Error::last_os_error();

    // SAFETY: closing the handle opened above, exactly once. NFR-13: examined — a close that
    // fails is reported rather than swallowed, and the reading is not trusted past it.
    if let Err(error) = unsafe { CloseHandle(handle) } {
        return Err(format!("CloseHandle({pid}): {error}"));
    }

    if ok == 0 {
        return Err(format!("GetProcessMemoryInfo({pid}): {failure}"));
    }

    Ok(MemoryReading {
        working_set: counters.working_set_size as u64,
        peak_working_set: counters.peak_working_set_size as u64,
        commit: counters.pagefile_usage as u64,
    })
}

/// The four `callback_` keys of SEC-04a in one read: samples, p50, p99, max — nanoseconds.
fn callback_reading() -> Result<(u64, u64, u64, u64), String> {
    let snapshot = crate::channel::read().map_err(|error| format!("канал SEC-04a: {error}"))?;

    let number = |key: &str| -> Result<u64, String> {
        snapshot
            .get(key)
            .ok_or_else(|| format!("канал не публикует {key}"))?
            .parse::<u64>()
            .map_err(|error| format!("{key} не число: {error}"))
    };

    Ok((
        number("callback_samples")?,
        number("callback_p50_ns")?,
        number("callback_p99_ns")?,
        number("callback_max_ns")?,
    ))
}

/// All three rows of position 23 when the measurement could not run.
fn performance_failed(reason: &str) -> Vec<Row> {
    let row = |assertion: &'static str, expected: &str| {
        Row::new(
            23,
            APP_23,
            Assertion::Other(assertion),
            Verdict::Fail,
            format!("измерение не состоялось: {reason}"),
            expected,
        )
    };

    vec![
        row(P99_ASSERTION, "p99 < 100 000 нс (NFR-01)"),
        row(MAX_ASSERTION, "максимум < 1 000 000 нс (NFR-02)"),
        row(MEMORY_ASSERTION, "рост ≤ 256 КБ"),
    ]
}

/// **Position 23 — «10 000 синтетических нажатий → p50/p99/максимум длительности callback
/// в пределах NFR-01, NFR-02; рабочий набор памяти не растёт»** (footnote 5 of §11.3).
///
/// # Where the numbers come from
///
/// The durations come from the QPC instrument task T-10-1 put inside the callback — feature
/// `testing`, criterion 2 of §13 — published as the four `callback_` keys of SEC-04a. The
/// same rule as position 14: what happens inside a hook callback is not observable from
/// outside the process even in principle, so the channel is not a convenience here, it is
/// the only honest source. The bench does not merely read the three durations: it first
/// reads `callback_samples` and refuses a verdict measured on fewer invocations than the
/// criterion's ten thousand presses produce (two per press — down and up).
///
/// # The shape of the scenario
///
/// Notepad, the application of position 1, adopted through the same gates. A warm-up of
/// [`WARMUP_PRESSES`] pays the one-time costs, **then** the first memory snapshot is taken;
/// the volley of [`VOLLEY_PRESSES`] follows in [`BATCH_PRESSES`]-press batches, each batch
/// one `SendInput` behind the foreground guard; then the bench waits on the condition — the
/// sample having grown by two per press — never on a clock (§11.5 requirement 1), and takes
/// the second memory snapshot. Memory is read with `GetProcessMemoryInfo` by the product's
/// process id, and the growth is judged against [`WORKING_SET_TOLERANCE`].
///
/// The hotkey is never pressed: the matrix row is about the callback under load and the
/// working set, not about conversion, and pressing it would put the product's own injected
/// backspaces into the middle of the volley being counted.
pub fn position_23(ctx: &Context) -> Vec<Row> {
    let mut app = match launch_notepad() {
        Ok(app) => app,
        Err(error) => return performance_failed(&format!("запуск: {error}")),
    };

    let mut rows = match adopt_window(ctx.automation, &mut app, &|element: &Element| {
        element.class() == "Notepad"
    }) {
        Err(reason) => performance_failed(&reason),
        Ok(window) => {
            let content =
                ctx.automation
                    .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e));

            match (app.window, content) {
                (None, _) => performance_failed("у окна нет дескриптора"),
                (_, None) => performance_failed("элемент ввода не найден в дереве UI Automation"),
                // The content element is the readiness gate of §11.5 requirement 1 — the
                // window is up when its editor answers UI Automation — and is not read
                // afterwards: nothing about this position compares text.
                (Some(hwnd), Some(_)) => measure_performance(app.pid, hwnd),
            }
        }
    };

    let closed = app.close();
    for row in &mut rows {
        row.note = format!("{}; закрытие: {closed}", row.note);
    }

    rows
}

/// The measurement itself, once the window is up and adopted.
fn measure_performance(pid: u32, window: HWND) -> Vec<Row> {
    if let Err(error) = shell::activate_window(pid, Some(window)) {
        return performance_failed(&format!("вывод окна вперёд: {error}"));
    }

    let target = input::Target { pid, hwnd: window };

    // The product under test, by pid — for the memory half. Exactly one must be running:
    // zero means the dispatcher's copy died, two means a stray is skewing every number.
    let products = crate::sut::any_running();
    let product = match products.as_slice() {
        &[product] => product,
        other => return performance_failed(&format!("продуктов не один: {other:?}")),
    };

    // Warm-up, condition-checked: the strokes must be *seen* by the product, not merely sent.
    let chunk = TYPED.repeat(WARMUP_PRESSES / TYPED.len());
    let before_warmup = match callback_reading() {
        Ok((samples, ..)) => samples,
        Err(error) => return performance_failed(&error),
    };
    if let Err(error) = input::type_text(&chunk, &target) {
        return performance_failed(&format!("разминка: {error}"));
    }
    let warmup_target = before_warmup + 2 * WARMUP_PRESSES as u64;
    if wait::until(Duration::from_secs(15), || {
        callback_reading()
            .ok()
            .filter(|(samples, ..)| *samples >= warmup_target)
    })
    .is_none()
    {
        return performance_failed("разминка не дошла до callback за 15 с");
    }

    // The first snapshot — memory and sample count — after the warm-up, before the volley.
    let memory_before = match process_memory(product) {
        Ok(reading) => reading,
        Err(error) => return performance_failed(&error),
    };
    let samples_before = match callback_reading() {
        Ok((samples, ..)) => samples,
        Err(error) => return performance_failed(&error),
    };

    // The volley. No sleep between batches: `SendInput` paces itself through the hook chain,
    // and the wait below is on the condition, not on a clock.
    let batch = TYPED.repeat(BATCH_PRESSES / TYPED.len());
    let batches = VOLLEY_PRESSES / BATCH_PRESSES;
    let started = Instant::now();
    for _ in 0..batches {
        if let Err(error) = input::type_text(&batch, &target) {
            return performance_failed(&format!("залп: {error}"));
        }
    }
    let injection = started.elapsed();

    // §11.5 requirement 1: the wait is on "the sample covers the volley", never on a clock.
    let volley_target = samples_before + 2 * VOLLEY_PRESSES as u64;
    let Some((samples, p50_ns, p99_ns, max_ns)) = wait::until(Duration::from_secs(25), || {
        callback_reading()
            .ok()
            .filter(|(samples, ..)| *samples >= volley_target)
    }) else {
        let seen = callback_reading().map(|(samples, ..)| samples);
        return performance_failed(&format!(
            "выборка не набралась за 25 с: нужно ≥ {volley_target}, канал показывает {seen:?}"
        ));
    };

    let memory_after = match process_memory(product) {
        Ok(reading) => reading,
        Err(error) => return performance_failed(&error),
    };

    let delta = samples - samples_before;
    let enough = delta >= 2 * VOLLEY_PRESSES as u64;
    let growth = memory_after
        .working_set
        .saturating_sub(memory_before.working_set);
    let kib = |bytes: u64| bytes / 1024;

    let note = format!(
        "продукт PID {product}; разминка {WARMUP_PRESSES}, залп {VOLLEY_PRESSES} нажатий \
         пакетами по {BATCH_PRESSES}, инжекция {} мс; выборка залпа {delta} вызовов \
         (всего {samples}); память до/после: рабочий набор {}/{} КБ (пик {} КБ), \
         частная {}/{} КБ",
        injection.as_millis(),
        kib(memory_before.working_set),
        kib(memory_after.working_set),
        kib(memory_after.peak_working_set),
        kib(memory_before.commit),
        kib(memory_after.commit),
    );

    let percentile_row = Row::new(
        23,
        APP_23,
        Assertion::Other(P99_ASSERTION),
        if enough && p99_ns < 100_000 {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        format!("p50={p50_ns} нс, p99={p99_ns} нс на выборке {delta}"),
        format!(
            "p99 < 100 000 нс (NFR-01) на ≥ {} вызовах",
            2 * VOLLEY_PRESSES
        ),
    )
    .with_note(note.clone());

    let maximum_row = Row::new(
        23,
        APP_23,
        Assertion::Other(MAX_ASSERTION),
        if enough && max_ns < 1_000_000 {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        format!("max={max_ns} нс"),
        "максимум < 1 000 000 нс (NFR-02)",
    )
    .with_note(note.clone());

    let memory_row = Row::new(
        23,
        APP_23,
        Assertion::Other(MEMORY_ASSERTION),
        if growth <= WORKING_SET_TOLERANCE {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        format!(
            "рост {} КБ (до {} КБ, после {} КБ)",
            kib(growth),
            kib(memory_before.working_set),
            kib(memory_after.working_set)
        ),
        format!("рост ≤ {} КБ", kib(WORKING_SET_TOLERANCE)),
    )
    .with_note(note);

    vec![percentile_row, maximum_row, memory_row]
}

// ---------------------------------------------------------------------------------------
// Position 15 — the selection path, §4.7: FR-60, FR-61, FR-64
// ---------------------------------------------------------------------------------------

/// What the third row of position 15 is about, in the report.
const CLIPBOARD_ASSERTION: &str = "буфер обмена восстановлен";

/// The application name of position 15's three rows.
const APP_15: &str = "Путь выделения (окно стенда)";

/// What the bench puts on the clipboard before the scenario.
///
/// The third row of position 15 has to compare something the bench **chose** against something
/// the bench chose. Comparing whatever the person at the machine happened to have copied would
/// pass vacuously on an empty clipboard, which is the one state a scenario about restoring the
/// clipboard must not be allowed to pass in.
const SEEDED: &str = "T-07-2 clipboard sentinel";

/// A message-only window of the bench's own, for the two clipboard accesses of position 15.
///
/// `OpenClipboard` wants a window of the **calling thread** — the report of T-07-1 explains why
/// a null one is not good enough — so the bench makes itself one. The predefined `STATIC` class
/// is used, so no window class of ours is registered and no window procedure of ours can be
/// reached.
struct ClipWindow(HWND);

impl ClipWindow {
    fn create() -> Option<Self> {
        // SAFETY: `STATIC` is a predefined class that is always registered; both strings are
        // `'static` literals; the parent is `HWND_MESSAGE`, which asks for a message-only
        // window; no `lpParam` is passed, so nothing of ours reaches the class's procedure.
        // NFR-13: the binding turns a null handle into `Err`, which is examined here.
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("langsw-e2e-position-15"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .ok()
        .map(Self)
    }
}

impl Drop for ClipWindow {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
        // destroyed exactly once — the type is neither `Copy` nor `Clone`.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

/// All three rows of position 15 when the scenario could not reach the hotkey.
fn selection_failed(reason: &str) -> Vec<Row> {
    vec![
        Row::new(15, APP_15, Assertion::Text, Verdict::Fail, reason, EXPECTED),
        Row::new(
            15,
            APP_15,
            Assertion::Layout,
            Verdict::Fail,
            reason,
            layout::describe(layout::RUSSIAN),
        ),
        Row::new(
            15,
            APP_15,
            Assertion::Other(CLIPBOARD_ASSERTION),
            Verdict::Fail,
            reason,
            format!("{SEEDED:?}"),
        ),
    ]
}

/// **Position 15 — «выделить фразу → горячая клавиша → фраза конвертирована, буфер обмена
/// восстановлен».**
///
/// # Why this is not position 1 with one extra key
///
/// The seven steps of [`replacement`] type `ghbdtn` and press the hotkey, and the *typing buffer*
/// is what converts it. Position 15 is about the other source of data, so the scenario has to
/// make sure the typing buffer cannot be the one that answers — otherwise a product whose
/// selection path did nothing at all would pass it.
///
/// It is made sure of by the selection itself. `Ctrl+A` is a `Ctrl` combination, and the fourth
/// row of the FR-10 table says a `Ctrl` combination is «полный сброс (команда, а не текст)» — so
/// selecting the text **empties the typing buffer**, and the SEC-04a channel is read to show it
/// really did. What is left for the hotkey to convert is the selection and nothing else.
///
/// # Three assertions
///
/// | Row | Expected |
/// |---|---|
/// | text | `привет` — the phrase was converted |
/// | layout | RU in the active window — step 7 of FR-61 |
/// | clipboard | what it held before the scenario, character for character — step 8 |
///
/// The third row is the one the matrix names beside the conversion, and it is asserted against a
/// reading taken **before the product was given anything to do**: [`clip::Guard`] captures on the
/// way in, `unchanged` compares afterwards, and its `Drop` puts the text back whatever happens.
/// ⚠ The guard round-trips `CF_UNICODETEXT` only — that limit is `clip.rs`'s and is stated there
/// — so the bench seeds the clipboard with a phrase of its own first. That makes the comparison
/// exact and, more to the point, means the run never depends on what the person at the machine
/// happened to have copied.
///
/// # Requirement 1 of §11.5
///
/// Every wait is on a condition. The selection is waited for by reading the element back through
/// UI Automation, the conversion by waiting for `привет`, the layout by waiting for RU. The one
/// place a clock is unavoidable is the clipboard row: step 8 of FR-61 restores **after a delay**
/// of 200 ms by section 7, so the bench waits for the clipboard to come back rather than
/// asserting immediately — a condition again, with the timeout as its bound.
pub fn position_15(ctx: &Context) -> Vec<Row> {
    // Requirement 5 of §11.5: the clipboard of whoever is at the machine is captured before
    // anything and put back by the guard's `Drop`, including on the path where this returns
    // early. The seed below is written **after** the capture, so the guard still holds the
    // user's own content.
    let _clipboard = clip::Guard::capture();

    // ⚠ **The window that writes the seed is destroyed the moment the seed is written**, and
    // that is not tidiness — it is the difference between this position working and taking five
    // seconds to fail.
    //
    // `EmptyClipboard` makes the window passed to `OpenClipboard` the **owner** of the
    // clipboard, and when somebody else later empties it the system delivers
    // `WM_DESTROYCLIPBOARD` to that owner with a **blocking** send. The bench's main thread is a
    // console loop and pumps no messages, so the application's `Ctrl+C` would sit in
    // `EmptyClipboard` until the system's hung-window timeout — measured at 5091 ms, against the
    // 300 ms step 3 of FR-61 gives it. The product would then correctly report "there is no
    // selection", and the position would be measuring the bench.
    //
    // A destroyed window cannot be sent to, so the notification is dropped and the copy is
    // immediate. The clipboard *data* is unaffected: it lives in global memory the clipboard
    // owns, not in the window.
    {
        let Some(seeder) = ClipWindow::create() else {
            return selection_failed("не удалось создать окно стенда для доступа к буферу обмена");
        };

        if lang_switcher::selection::write_unicode_text(seeder.0, SEEDED).is_err() {
            return selection_failed("не удалось положить в буфер обмена контрольную строку");
        }
    }

    // Reading never makes anybody the owner, so this one may live for the whole position.
    let Some(clip_window) = ClipWindow::create() else {
        return selection_failed("не удалось создать окно стенда для чтения буфера обмена");
    };

    let mut app = match launch_selection_window() {
        Ok(app) => app,
        Err(error) => return selection_failed(&error),
    };

    let mut rows = match adopt_window(ctx.automation, &mut app, &|element: &Element| {
        element.name().starts_with(SELECTION_WINDOW_TITLE)
    }) {
        Err(reason) => selection_failed(&reason),
        Ok(window) => {
            let content =
                ctx.automation
                    .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e));

            match (app.window, content) {
                (None, _) => selection_failed("у окна нет дескриптора"),
                (_, None) => selection_failed("элемент ввода не найден в дереве UI Automation"),
                (Some(hwnd), Some(content)) => {
                    selection_body(ctx, app.pid, hwnd, &window, &content, clip_window.0)
                }
            }
        }
    };

    let closed = app.close();
    for row in &mut rows {
        row.note = format!("{}; закрытие: {closed}", row.note);
    }

    rows
}

/// Reads `CF_UNICODETEXT` through the accepted primitive of module `selection`.
fn clipboard_text(owner: HWND) -> Option<String> {
    lang_switcher::selection::read_unicode_text(owner)
        .ok()
        .flatten()
}

/// Title of the bench's own window for position 15 — the string the position adopts by.
const SELECTION_WINDOW_TITLE: &str = "LangSw-Selection-15";

/// ⛔ **The window of position 15 is the bench's own** — the shape position 14 already uses.
///
/// Requirements A to E are not relaxed here. The process is started by this function and is
/// therefore in the registry as a **root**, so `claim_window_process` answers on the first
/// question it asks and never has to walk a parent chain that has already lost its parent. That
/// is not a convenience: on this machine a `Notepad.exe` that the bench did not start is running,
/// the System32 `notepad.exe` is a stub whose process exits as soon as the packaged application
/// takes over, and a position that adopted by window class would be deciding between two
/// strangers' windows. A window of the bench's own removes the question instead of answering it
/// better.
///
/// One plain multiline text box, nothing masked, nothing saved anywhere.
///
/// ⚠ **The window publishes the length of its own selection in its title**, the way position
/// 14's publishes the length of its password box, and stops doing so as soon as there is one.
/// That is what lets the scenario wait for the selection **on a condition** (requirement 1 of
/// §11.5) instead of on a clock, and it matters more here than anywhere else in the matrix: step
/// 3 of FR-61 waits 300 ms for the clipboard to move, so a hotkey pressed while the application
/// was still making the selection would measure that race and report "there is no selection".
/// The timer stops itself once the selection exists, so the window is idle from that moment on.
fn launch_selection_window() -> Result<App, String> {
    let scratch = scratch_dir("selection");
    let script = scratch.join("selection-window.ps1");

    let body = format!(
        "Add-Type -AssemblyName System.Windows.Forms\n\
         $form = New-Object System.Windows.Forms.Form\n\
         $form.Text = '{SELECTION_WINDOW_TITLE} sel=0'\n\
         $form.Width = 640\n\
         $form.Height = 260\n\
         $form.StartPosition = 'CenterScreen'\n\
         $form.TopMost = $true\n\
         $box = New-Object System.Windows.Forms.TextBox\n\
         $box.Multiline = $true\n\
         $box.Left = 20\n\
         $box.Top = 30\n\
         $box.Width = 580\n\
         $box.Height = 150\n\
         $box.TabIndex = 0\n\
         $form.Controls.Add($box)\n\
         $tick = New-Object System.Windows.Forms.Timer\n\
         $tick.Interval = 150\n\
         $tick.Add_Tick({{ $form.Text = '{SELECTION_WINDOW_TITLE} sel=' + $box.SelectionLength; \
         if ($box.SelectionLength -gt 0) {{ $tick.Stop() }} }})\n\
         $tick.Start()\n\
         $form.Add_Shown({{ $form.Activate(); [void]$box.Focus() }})\n\
         [void]$form.ShowDialog()\n"
    );

    std::fs::write(&script, body)
        .map_err(|error| format!("не удалось записать {}: {error}", script.display()))?;

    let child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-STA",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .spawn()
        .map_err(|error| format!("запуск окна позиции 15: {error}"))?;

    Ok(launched(
        "Путь выделения (окно стенда)",
        child,
        CloseWith::WmClose,
        Some(scratch),
    ))
}

/// The body of position 15, for a window that has already been adopted.
fn selection_body(
    ctx: &Context,
    pid: u32,
    window: HWND,
    window_element: &Element,
    content: &Element,
    clip_window: HWND,
) -> Vec<Row> {
    // Step 1 — forward. Rake 3, and requirement B has already been satisfied by `adopt_window`.
    if let Err(error) = shell::activate_window(pid, Some(window)) {
        return selection_failed(&format!("вывод окна вперёд: {error}"));
    }

    let target = input::Target { pid, hwnd: window };

    // Step 2 — the precondition of §11.3: the English layout.
    let source_layout = match layout::ensure(window, layout::US, Duration::from_secs(5)) {
        Ok(id) => id,
        Err(error) => return selection_failed(&format!("исходная раскладка: {error}")),
    };

    // Step 3 — type the phrase. The window is the bench's own and was launched empty for this
    // position, so there is nothing to clear first.
    if let Err(error) = input::type_text(TYPED, &target) {
        return selection_failed(&format!("ввод {TYPED:?}: {error}"));
    }

    // Step 4 — wait until the application really has it. A condition, not a clock.
    if wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| normalise(&raw))
            .filter(|text| text.contains(TYPED))
    })
    .is_none()
    {
        return selection_failed("введённое не дошло до приложения");
    }

    // ⚠ **Step 5 — select it, and by selecting it empty the typing buffer.** `Shift+Home` and
    // not `Ctrl+A`: a WinForms `TextBox` does not answer `Ctrl+A` with select-all of its own
    // accord (measured on this machine), and `Shift+Home` is the gesture every edit control
    // implements. `input::chord` sends `Home` with the `E0` prefix of FR-05, which is what makes
    // it a *shifted* `Home` — injected without it the same key reaches the application as
    // `Shift+Numpad7`, Windows suppresses the `Shift`, and the caret moves without selecting.
    //
    // Either way the row of the FR-10 table that matters here is the same: `Home` is «полный
    // сброс», so the typing buffer is **empty** from this point and the selection is the only
    // thing the hotkey can possibly convert.
    if let Err(error) = input::chord(&[VK_SHIFT.0], VK_HOME.0, &target) {
        return selection_failed(&format!("выделение: {error}"));
    }

    // ⚠ **Requirement 1 of §11.5, and it is not a formality here.** The hotkey must not be sent
    // until the selection really exists: the product answers `Ctrl+C` and waits 300 ms (§7) for
    // the clipboard to move, so a press that arrived while the application was still making the
    // selection would measure that race and report "there is no selection". The window publishes
    // the length of its own selection in its title — the shape position 14 uses for `pass=N` —
    // and this waits for it. A condition, published by the application, never a pause.
    let selected = wait::until_true(wait::TEXT_TIMEOUT, || {
        window_element.name().contains("sel=6")
    });

    if !selected {
        return selection_failed(&format!(
            "выделение не состоялось: заголовок окна {:?}, ожидалось sel=6",
            window_element.name()
        ));
    }

    // And that is read back from the product itself rather than assumed — SEC-04a, and it is
    // what tells "the selection path converted the selection" from "the typing buffer did".
    let buffer_after_select = wait::until(Duration::from_secs(2), || {
        crate::channel::read()
            .ok()
            .and_then(|snapshot| snapshot.get("buffer_len").map(str::to_owned))
            .filter(|length| length == "0")
    })
    .unwrap_or_else(|| {
        crate::channel::read()
            .ok()
            .and_then(|snapshot| snapshot.get("buffer_len").map(str::to_owned))
            .unwrap_or_else(|| "?".to_owned())
    });

    // Step 7 — the hotkey.
    let sequence_before = lang_switcher::selection::sequence_number();

    if let Err(error) = input::tap(ctx.hotkey_vk, &target) {
        return selection_failed(&format!("горячая клавиша: {error}"));
    }

    // ⚠ **Step 8 — wait for the application to answer the product's `Ctrl+C`, and wait for it on
    // the clipboard sequence number rather than by asking the application anything.**
    //
    // This is requirement 1 of §11.5 applied to a place where the obvious reading of it is
    // actively harmful. Step 3 of FR-61 gives the application 300 ms to put the selection on the
    // clipboard, and the application here is a single-threaded WinForms window: UI Automation
    // calls are marshalled onto that same thread, so a bench that started polling the element
    // the instant the hotkey went out would be **competing with the very keystroke it is waiting
    // for** — measured, and it is what made this position report "the text was not converted"
    // while the clipboard plainly held the copied phrase.
    //
    // `GetClipboardSequenceNumber` costs one read of a window-station counter, needs no clipboard
    // lock and cannot block anybody, so watching it disturbs neither side. Its first move after
    // the press *is* the application answering `Ctrl+C` — which is the same fact step 3 of FR-61
    // reads, from the other side of the same machine.
    let pressed_at = Instant::now();
    let copied = wait::until_true(wait::TEXT_TIMEOUT, || {
        lang_switcher::selection::sequence_number() != sequence_before
    });
    let answered_in = pressed_at.elapsed();

    if !copied {
        return selection_failed(
            "буфер обмена не изменился после горячей клавиши: приложение не ответило на Ctrl+C",
        );
    }

    // Step 9 — the conversion. A condition, and the timeout has to cover the rest of FR-61:
    // the read, the recoding, the paste and the 200 ms delay of step 8.
    let converted = wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| normalise(&raw))
            .filter(|text| text.contains(EXPECTED))
    });

    let shown = converted
        .clone()
        .or_else(|| content.read().map(|(raw, _)| normalise(&raw)))
        .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    // Step 10 — the layout of FR-61 step 7.
    let observed = wait::until(wait::TEXT_TIMEOUT, || {
        layout::of_window(window)
            .map(layout::id_of)
            .filter(|id| id & 0xFFFF == layout::RUSSIAN & 0xFFFF)
    })
    .or_else(|| layout::of_window(window).map(layout::id_of));

    // Step 11 — the clipboard of FR-61 step 8. A condition with a bound, because the restore is
    // deliberately delayed by 200 ms (§7) and asserting immediately would measure the delay
    // instead of the restore.
    let restored = wait::until_true(wait::TEXT_TIMEOUT, || {
        clipboard_text(clip_window).as_deref() == Some(SEEDED)
    });
    let clipboard_now =
        clipboard_text(clip_window).unwrap_or_else(|| "<нет текста в буфере обмена>".to_owned());

    let counters = crate::channel::read()
        .ok()
        .map(|snapshot| {
            format!(
                "buffer_len={}, hotkey_handoffs={}, post_failures={}, send_mismatches={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?"),
                snapshot.get("post_failures").unwrap_or("?"),
                snapshot.get("send_mismatches").unwrap_or("?"),
            )
        })
        .unwrap_or_else(|| "канал не ответил".to_owned());

    vec![
        Row::new(
            15,
            APP_15,
            Assertion::Text,
            if converted.is_some() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            format!("{shown:?}"),
            format!("{EXPECTED:?}"),
        )
        .with_note(format!(
            "выделение через Shift+Home (с префиксом E0, FR-05); буфер набора после выделения: \
             {buffer_after_select} — FR-10 «Home — полный сброс», поэтому конвертировать могла \
             только выделенная фраза; приложение ответило на Ctrl+C продукта за {} мс (шаг 3 \
             FR-61 ждёт 300 мс, §7); исходная раскладка окна {}; {counters}",
            answered_in.as_millis(),
            layout::describe(source_layout)
        )),
        Row::new(
            15,
            APP_15,
            Assertion::Layout,
            match observed {
                Some(id) if id & 0xFFFF == layout::RUSSIAN & 0xFFFF => Verdict::Pass,
                _ => Verdict::Fail,
            },
            observed.map_or("<чтение не удалось>".to_owned(), layout::describe),
            layout::describe(layout::RUSSIAN),
        )
        .with_note("шаг 7 FR-61 — переключение раскладки, §4.6"),
        Row::new(
            15,
            APP_15,
            Assertion::Other(CLIPBOARD_ASSERTION),
            if restored {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            format!("{clipboard_now:?}"),
            format!("{SEEDED:?}"),
        )
        .with_note(
            "шаг 8 FR-61 с задержкой 200 мс (§7); контрольная строка положена стендом до \
             сценария, буфер обмена пользователя снят до неё и возвращается стражем clip::Guard",
        ),
    ]
}

// ---------------------------------------------------------------------------------------
// Position 17 — the cycle over three layouts, FR-31, FR-32
// ---------------------------------------------------------------------------------------

/// **Position 17 — "три нажатия горячей клавиши возвращают исходный текст побитово; раскладка
/// вернулась к исходной."**
///
/// # What has to be true before a single key is pressed
///
/// | Precondition | How it is arranged | Undone by |
/// |---|---|---|
/// | a third layout attached to the session | [`layout::Temporary`], footnote 3 of §11.3 | `detach`, and its `Drop` |
/// | `mode = "cycle"` over three layouts | [`crate::config::Borrowed`] — the user's own file with one section changed | `give_back`, its `Drop`, and the stash on disk |
/// | the product running **on that file** | a copy launched here, after the write: §7 reads the configuration at start-up and never again | `stop`, and the job object |
///
/// That last row is why this position launches its own product instead of using the one the run
/// starts for every position: a configuration written under a running program changes nothing.
///
/// # Three assertions, and why the third is not a duplicate of the first
///
/// | Row | Expected |
/// |---|---|
/// | text | `ghbdtn` **bit for bit** — `==`, not "contains" (FR-32) |
/// | layout | back at `0x00000409` |
/// | `cycle_position` | `0` |
///
/// `ghbdtn` renders into the German layout as the same six characters, so after the *second*
/// press the field already reads `ghbdtn` — which is exactly the accident the third row exists to
/// catch. `cycle_position` is the product's own statement of where in the cycle it is, and only
/// `0` means the third press really happened and really came round.
///
/// # Requirement 1 of §11.5 — waiting between the presses
///
/// Never on a clock. Each press is followed by a wait for `cycle_position` to reach the value
/// that press produces — `1`, then `2`, then `0` — read from the SEC-04a channel. It is a
/// condition, it is published by the product rather than guessed at by the bench, and it is what
/// makes the next press a *next* press instead of half of a race.
pub fn position_17(ctx: &Context) -> Vec<Row> {
    const APP: &str = "Цикл при трёх раскладках";

    // ---- the third layout, before anything else, so that the product enumerates it ----
    let mut third = match layout::Temporary::attach() {
        Ok(third) => third,
        Err(error) => {
            return rollback_failed(17, APP, &format!("третья раскладка: {error}"));
        }
    };
    let third_handle = third.handle();

    // ---- the configuration ----
    let Some(path) = lang_switcher::settings::default_config_path() else {
        return rollback_failed(
            17,
            APP,
            "путь %APPDATA%\\Lang_Switcher\\config.toml не определён",
        );
    };
    let text = match crate::config::cycle_of_three(&path, third_handle) {
        Ok(text) => text,
        Err(error) => return rollback_failed(17, APP, &error),
    };
    let mut borrowed = match crate::config::Borrowed::take(&path, &text) {
        Ok(borrowed) => borrowed,
        Err(error) => return rollback_failed(17, APP, &format!("подмена config.toml: {error}")),
    };
    let borrowed_note = borrowed.describe_original();

    let mut rows = cycle_body(ctx, APP, third_handle);

    // ---- and back, in the order that makes the unload possible ----
    //
    // The ambient goes first: a layout that is still the session's current one is a layout the
    // system may refuse to unload, and this is also requirement 5 of §11.5 for this position.
    let ambient = restore_ambient(ctx);

    let detached = third.detach();
    let returned = borrowed.give_back();

    for row in &mut rows {
        row.note = format!(
            "{}; третья раскладка {}; {ambient}; {detached}; {borrowed_note}; {returned}",
            row.note,
            layout::describe(third_handle)
        );
    }

    rows
}

/// The body of position 17: its own product, its own window, three presses.
///
/// Split out so that every early return still passes through the restoration in
/// [`position_17`] — the third layout and the user's file are given back on this function's way
/// out whatever it returns.
fn cycle_body(ctx: &Context, app_name: &str, third_handle: u32) -> Vec<Row> {
    // A local `data:` URL with a textarea — position 4's field, for the same reasons: no network
    // (SEC-03, NFR-11), and a process of the bench's own with a throwaway profile. Notepad is
    // deliberately not used here: the packaged Notepad is a single process that a second launch
    // joins, so a copy already running on the machine would make this position report a refusal
    // of requirement B instead of anything about the cycle.
    const PAGE: &str = "data:text/html,<textarea id=t rows=8 cols=40 autofocus></textarea>";

    let failed = |reason: &str| rollback_failed(17, app_name, reason);

    // Requirement 5: the clipboard, as every other position captures it.
    let _clipboard = clip::Guard::capture();

    // ---- the product, launched **after** the configuration was written ----
    let mut product = match crate::sut::Sut::launch() {
        Ok(product) => product,
        Err(error) => return failed(&format!("продукт не запустился: {error}")),
    };
    let Some(ready) = product.await_ready(Duration::from_secs(30)) else {
        return failed("продукт не сообщил о готовности через канал SEC-04a за 30 с");
    };
    let started = format!(
        "продукт поднят заново под подменённой конфигурацией, PID {}, hook_installed={}",
        product.pid,
        ready.get("hook_installed").unwrap_or("?")
    );
    println!("  {started}");

    let mut app = match launch_chrome("chrome17", &[PAGE]) {
        Ok(app) => app,
        Err(error) => return failed(&format!("запуск: {error}")),
    };

    let window = match adopt_window(ctx.automation, &mut app, &is_chrome_window) {
        Ok(window) => window,
        Err(reason) => return failed(&reason),
    };
    let Some(hwnd) = app.window else {
        return failed("у окна нет дескриптора");
    };
    let Some(content) =
        ctx.automation
            .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                e.control_type() == Some(UIA_EditControlTypeId)
                    && e.class() != "Chrome_OmniboxView"
                    && !e.name().contains("Адресная строка")
                    && !e.name().to_lowercase().contains("address")
            })
    else {
        return failed("элемент ввода не найден в дереве UI Automation");
    };

    let mut rows = cycle_presses(ctx, app.pid, hwnd, &content, third_handle, &started);

    let closed = app.close();
    let stopped = match product.stop() {
        Ok(code) => format!("продукт завершён, код возврата {code}"),
        Err(error) => format!("⚠ {error}"),
    };
    for row in &mut rows {
        row.note = format!("{}; закрытие: {closed}; {stopped}", row.note);
    }

    rows
}

/// The seven steps and then three presses — the measuring part of position 17.
fn cycle_presses(
    ctx: &Context,
    pid: u32,
    window: HWND,
    content: &Element,
    third_handle: u32,
    started: &str,
) -> Vec<Row> {
    const APP: &str = "Цикл при трёх раскладках";
    /// The cycle is `[en-US, ru-RU, de-DE]`, so the counter goes 1, 2, 0 — FR-31.
    const EXPECTED_POSITIONS: [usize; 3] = [1, 2, 0];

    let failed = |reason: &str| rollback_failed(17, APP, reason);

    if let Err(error) = shell::activate_window(pid, Some(window)) {
        return failed(&format!("вывод окна вперёд: {error}"));
    }
    let target = input::Target { pid, hwnd: window };

    let source_layout = match layout::ensure(window, layout::US, Duration::from_secs(5)) {
        Ok(id) => id,
        Err(error) => return failed(&format!("исходная раскладка: {error}")),
    };

    if let Err(error) = input::chord(&[VK_CONTROL.0], VK_A.0, &target) {
        return failed(&format!("очистка поля: {error}"));
    }
    if let Err(error) = input::tap(VK_DELETE.0, &target) {
        return failed(&format!("очистка поля: {error}"));
    }
    if let Err(error) = input::type_text(TYPED, &target) {
        return failed(&format!("ввод {TYPED:?}: {error}"));
    }

    // The keystrokes have landed when the field reads them back — a condition, not a pause.
    if wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| normalise(&raw))
            .filter(|text| text.contains(TYPED))
    })
    .is_none()
    {
        return failed("введённое не дошло до приложения");
    }

    // ---- three presses, each waited out on the product's own counter ----
    let mut walked = Vec::new();
    for expected in EXPECTED_POSITIONS {
        if let Err(error) = input::tap(ctx.hotkey_vk, &target) {
            return failed(&format!("нажатие {}: {error}", walked.len() + 1));
        }

        let reached = wait::until(wait::TEXT_TIMEOUT, || {
            crate::channel::read()
                .ok()
                .and_then(|snapshot| snapshot.get(CYCLE_KEY).map(str::to_owned))
                .filter(|value| value.trim() == expected.to_string())
        });

        match reached {
            Some(value) => walked.push(value),
            None => {
                let now = crate::channel::read()
                    .ok()
                    .and_then(|snapshot| snapshot.get(CYCLE_KEY).map(str::to_owned))
                    .unwrap_or_else(|| "нет ответа".to_owned());
                return failed(&format!(
                    "после нажатия {} счётчик цикла не дошёл до {expected}: {CYCLE_KEY}={now}",
                    walked.len() + 1
                ));
            }
        }
    }

    // ---- what the field holds now ----
    let restored = wait::until(wait::TEXT_TIMEOUT, || {
        content
            .read()
            .map(|(raw, _)| normalise(&raw))
            .filter(|text| text == TYPED)
    });
    let shown = restored
        .clone()
        .or_else(|| content.read().map(|(raw, _)| normalise(&raw)))
        .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    // ---- and the layout, on the same condition-and-never-a-clock rule ----
    let observed = wait::until(wait::TEXT_TIMEOUT, || {
        layout::of_window(window)
            .map(layout::id_of)
            .filter(|id| id & 0xFFFF == layout::US & 0xFFFF)
    })
    .or_else(|| layout::of_window(window).map(layout::id_of));

    let snapshot = crate::channel::read();
    let cycle = snapshot
        .as_ref()
        .ok()
        .and_then(|snapshot| snapshot.get(CYCLE_KEY).map(str::to_owned));
    let counters = snapshot
        .as_ref()
        .ok()
        .map(|snapshot| {
            format!(
                "buffer_len={}, hotkey_handoffs={}, post_failures={}, send_mismatches={}",
                snapshot.get("buffer_len").unwrap_or("?"),
                snapshot.get("hotkey_handoffs").unwrap_or("?"),
                snapshot.get("post_failures").unwrap_or("?"),
                snapshot.get("send_mismatches").unwrap_or("?"),
            )
        })
        .unwrap_or_else(|| "канал не ответил".to_owned());

    let walk = walked.join(" -> ");

    vec![
        Row::new(
            17,
            APP,
            Assertion::Text,
            if restored.is_some() {
                Verdict::Pass
            } else {
                Verdict::Fail
            },
            format!("{shown:?}"),
            format!("{TYPED:?} побитово"),
        )
        .with_note(format!(
            "{started}; цикл из трёх: {}, {}, {}; счётчик прошёл {walk}; \
             сравнение — равенство целиком, не «содержит» (FR-32); продукт: {counters}",
            layout::describe(layout::US),
            layout::describe(layout::RUSSIAN),
            layout::describe(third_handle)
        )),
        Row::new(
            17,
            APP,
            Assertion::Layout,
            match observed {
                Some(id) if id & 0xFFFF == layout::US & 0xFFFF => Verdict::Pass,
                _ => Verdict::Fail,
            },
            observed.map_or("<чтение не удалось>".to_owned(), layout::describe),
            layout::describe(layout::US),
        )
        .with_note(format!(
            "исходная раскладка окна {}; после трёх нажатий цикл длины 3 возвращает её же \
             (FR-31), переключение — FR-40 шаг 5 и §4.6",
            layout::describe(source_layout)
        )),
        Row::new(
            17,
            APP,
            Assertion::Other(CYCLE_ASSERTION),
            match cycle.as_deref() {
                Some("0") => Verdict::Pass,
                _ => Verdict::Fail,
            },
            match &cycle {
                Some(value) => format!("{CYCLE_KEY}={value}"),
                None => match &snapshot {
                    Ok(_) => format!("ключ {CYCLE_KEY} в снимке отсутствует"),
                    Err(error) => format!("канал SEC-04a не ответил: {error}"),
                },
            },
            format!("{CYCLE_KEY}=0"),
        )
        .with_note(format!(
            "FR-31, FR-32: цикл длины 3 вернулся в начало; счётчик прошёл {walk}; \
             прочие счётчики: {counters}"
        )),
    ]
}

// ---------------------------------------------------------------------------------------
// Position 2 — Microsoft Word. Its own function because of the five rakes.
// ---------------------------------------------------------------------------------------

/// Position 2, with the dialog detector of rakes 1 and 2 running before and after.
///
/// Returns the rows and a protocol of what happened, which the run writes into
/// `reports\T-04-3-позиция-2.md`.
pub fn position_2(ctx: &Context) -> (Vec<Row>, String) {
    let mut log = String::new();
    let clipboard = clip::Guard::capture();

    log.push_str(&format!(
        "Буфер обмена до сценария: {}\n\n",
        clipboard.saved().describe()
    ));

    let child = match word::launch() {
        Ok(child) => child,
        Err(error) => {
            log.push_str(&format!("Запуск не состоялся: {error}\n"));
            return (both_failed(2, "Microsoft Word", &error), log);
        }
    };

    let pid = child.id();
    log.push_str(&format!("Запущено: {} /w, PID {pid}\n", word::EXECUTABLE));

    // ⚠ `CloseWith::WordCom` and nothing else — rakes 4 and 5.
    let mut app = launched("Microsoft Word", child, CloseWith::WordCom, None);

    let mut rows = word_body(ctx, &mut app, &mut log);

    let closed = app.close();
    log.push_str(&format!("\nЗакрытие: {closed}\n"));

    let dialogs_after = word::dialogs(ctx.automation, pid);
    if dialogs_after.is_empty() {
        log.push_str("Диалогов после закрытия не осталось.\n");
    } else {
        for dialog in &dialogs_after {
            log.push_str(&format!(
                "⚠ После закрытия остался диалог: {}\n",
                dialog.describe()
            ));
        }
    }

    log.push_str(&format!(
        "Resiliency\\DisabledItems после сценария: {}\n",
        word::resiliency_disabled_items()
    ));
    log.push_str(&format!(
        "Буфер обмена не изменился: {}\n",
        clipboard.unchanged()
    ));

    for row in &mut rows {
        row.note = format!("{}; закрытие: {closed}", row.note);
    }

    (rows, log)
}

fn word_body(ctx: &Context, app: &mut App, log: &mut String) -> Vec<Row> {
    let automation = ctx.automation;
    let pid = app.pid;

    let Some(main) = word::await_main(automation, pid) else {
        log.push_str("Главное окно OpusApp не появилось.\n");
        return both_failed(2, "Microsoft Word", "окно OpusApp не появилось");
    };
    log.push_str(&format!("Главное окно: {}\n", main.describe()));

    // ⛔ Requirement B applies to Word too, even though `word::await_main` already restricts the
    // search to the process the bench spawned. The check costs one process-table read and closes
    // the case where that process id has been reused.
    if let Err(reason) = app.adopt(&main) {
        log.push_str(&format!("{reason}\n"));
        return both_failed(2, "Microsoft Word", &reason);
    }

    // Rakes 1 and 2: both classes, both levels.
    let dialogs = word::dialogs(automation, pid);
    if dialogs.is_empty() {
        log.push_str("Диалогов при запуске нет — ни #32770, ни bosa_sdm_msword.\n");
    } else {
        for dialog in &dialogs {
            log.push_str(&format!("⚠ ДИАЛОГ ПРИ ЗАПУСКЕ: {}\n", dialog.describe()));
        }
        let texts: Vec<String> = dialogs.iter().map(word::Dialog::describe).collect();
        return both_failed(
            2,
            "Microsoft Word",
            &format!(
                "при запуске появилось модальное окно: {}",
                texts.join(" // ")
            ),
        );
    }

    let Some(hwnd) = app.window else {
        return both_failed(2, "Microsoft Word", "у окна OpusApp нет дескриптора");
    };

    let Some(content) = word::await_content(automation, &main) else {
        log.push_str("Элемент документа не найден.\n");
        return both_failed(2, "Microsoft Word", "элемент документа не найден");
    };
    log.push_str(&format!("Элемент документа: {}\n", content.describe()));

    // Rake 3, through the shared technique.
    if let Err(error) = shell::activate_window(pid, Some(hwnd)) {
        log.push_str(&format!("AppActivate: {error}\n"));
        return both_failed(2, "Microsoft Word", &format!("AppActivate: {error}"));
    }
    log.push_str("AppActivate: окно Word выведено вперёд\n");

    if let Some(focused) = automation.focused() {
        log.push_str(&format!("Элемент с фокусом: {}\n", focused.describe()));
    }

    // ⚠ `clear_first: false` — the document is brand new and empty, and `Ctrl+A`, `Delete` in a
    // word processor is a bigger gesture than this scenario needs.
    let rows = replacement(
        ctx,
        Scene {
            position: 2,
            app_name: "Microsoft Word",
            pid,
            window: hwnd,
            content: &content,
            modifiers: &[],
            clear_first: false,
            rollback: false,
        },
    );

    for row in &rows {
        log.push_str(&format!(
            "\nУтверждение «{}»: {} — фактически {}, ожидалось {}\n  {}\n",
            row.assertion.as_str(),
            row.verdict,
            row.actual,
            row.expected,
            row.note
        ));
    }

    // Autocorrect is what position 2 exists to observe — footnote 6 of §11.3 and FR-42.
    if let Some((raw, source)) = content.read() {
        log.push_str(&format!(
            "\nЧтение документа после сценария через {}: {:?}\n",
            source.as_str(),
            normalise(&raw)
        ));
    }

    let dialogs = word::dialogs(automation, pid);
    for dialog in &dialogs {
        log.push_str(&format!("⚠ Диалог после сценария: {}\n", dialog.describe()));
    }

    rows
}

// ---------------------------------------------------------------------------------------
// Positions 3 and 4 — Chrome
// ---------------------------------------------------------------------------------------

/// Chrome, started against a throwaway profile.
///
/// ⚠ `--user-data-dir` into a temporary directory is not a convenience: without it the bench
/// would be driving the user's real browser, with their tabs, their session and their history,
/// and closing it would disturb all three. With it, the instance is the bench's own and
/// terminating it can damage nothing.
fn launch_chrome(tag: &str, extra: &[&str]) -> Result<App, String> {
    const CHROME: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";

    if !PathBuf::from(CHROME).exists() {
        return Err(format!("{CHROME} не найден"));
    }

    let scratch = scratch_dir(tag);
    let child = Command::new(CHROME)
        .arg(format!("--user-data-dir={}", scratch.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-extensions",
            "--new-window",
        ])
        .args(extra)
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(launched(
        "Chrome",
        child,
        CloseWith::Terminate,
        Some(scratch),
    ))
}

/// A Chrome window, told apart from VS Code, which shares the class.
///
/// ⛔ **This is the predicate that caused both incidents.** The controller's Claude Code window
/// has class `Chrome_WidgetWin_1` and title `"Claude"` — non-empty, and not "Visual Studio
/// Code" — so it satisfied every clause below, and the bench closed it. The predicate is
/// unchanged and still matches that window; what changed is that a predicate no longer decides
/// anything. `own::claim_window_process` does, and it refuses `claude.exe` twice over: the
/// process is not in the registry, and the name is on the protected list of requirement C.
fn is_chrome_window(element: &Element) -> bool {
    element.class() == "Chrome_WidgetWin_1"
        && !element.name().is_empty()
        && !element.name().contains("Visual Studio Code")
}

pub fn position_3(ctx: &Context) -> Vec<Row> {
    run_position(
        ctx,
        Plan {
            number: 3,
            name: "Chrome — адресная строка",
            window_is: &is_chrome_window,
            content_in: &|automation: &Automation, window: &Element| {
                automation.await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                    e.control_type() == Some(UIA_EditControlTypeId)
                        && (e.name().contains("Адресная строка")
                            || e.name().to_lowercase().contains("address")
                            || e.class() == "Chrome_OmniboxView")
                })
            },
            modifiers: &[],
            clear_first: true,
            rollback: false,
        },
        || launch_chrome("chrome3", &["about:blank"]),
    )
}

pub fn position_4(ctx: &Context) -> Vec<Row> {
    // A local `data:` URL with a textarea. No network — SEC-03 and NFR-11 both matter here, and
    // a bench that fetched a page in order to test a text field would be adding a network
    // dependency to an acceptance run.
    const PAGE: &str = "data:text/html,<textarea id=t rows=8 cols=40 autofocus></textarea>";

    run_position(
        ctx,
        Plan {
            number: 4,
            name: "Chrome — многострочное поле",
            window_is: &is_chrome_window,
            content_in: &|automation: &Automation, window: &Element| {
                automation.await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                    e.control_type() == Some(UIA_EditControlTypeId)
                        && e.class() != "Chrome_OmniboxView"
                        && !e.name().contains("Адресная строка")
                        && !e.name().to_lowercase().contains("address")
                })
            },
            modifiers: &[],
            clear_first: true,
            rollback: false,
        },
        || launch_chrome("chrome4", &[PAGE]),
    )
}

// ---------------------------------------------------------------------------------------
// Position 5 — Visual Studio Code
// ---------------------------------------------------------------------------------------

/// ⛔ **П — приёмочная сессия.** Не по родству окна, а потому что читать нечего.
///
/// # The measurement
///
/// VS Code's window is in the UI Automation tree — class `Chrome_WidgetWin_1`, title
/// `e2e.txt - Visual Studio Code`, and the bench spawned it, so requirements A–E are satisfied
/// and it could be typed into. What cannot be done is the **reading back**, and requirement 3 of
/// §11.5 is explicit that the result is verified through `ValuePattern` or `TextPattern`.
///
/// Measured twice, the second time with accessibility support switched on in the bench's own
/// throwaway profile — the state in which Monaco is documented to publish an accessible text
/// area:
///
/// ```text
///   окно: 'e2e.txt - Visual Studio Code'
///     ControlType.Edit:     найдено 0
///     ControlType.Document: найдено 0
///     ControlType.Text:     найдено 0
/// ```
///
/// Not "the predicate was too narrow": **the editor is not in the tree at all**, and neither is
/// any text. Monaco draws its own glyphs and this build publishes nothing of them. Widening the
/// predicate is what the two earlier attempts did, and it could not have worked.
///
/// ⚠ **This says nothing about the product.** The bench cannot observe the outcome; a person
/// looking at the screen can. That is precisely the division §11.6 exists to make, so the
/// position goes to the acceptance session with a scenario written out in the report.
pub fn position_5(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        5,
        "Visual Studio Code",
        "окно принадлежит запущенному стендом процессу и по пунктам A–E доступно, но UI \
         Automation не отдаёт содержимое: измерено дважды, в том числе с включённым \
         editor.accessibilitySupport в собственном временном профиле стенда — Edit, Document \
         и Text найдено по нулю. Требование 3 §11.5 (считывание через ValuePattern либо \
         TextPattern) выполнить нечем",
    )
}

// ---------------------------------------------------------------------------------------
// Positions 6 and 7 — Windows Terminal
// ---------------------------------------------------------------------------------------

/// ⛔ **П — приёмочная сессия.** Классифицировано измерением, см. `--classify`.
///
/// # The measurement, and why its answer is not the one that was expected
///
/// The controller's estimate was that `wt.exe` is a launcher whose window is created by COM
/// activation, so the terminal would not be kin to anything the bench started. **The
/// measurement says otherwise**: `WindowsTerminal.exe` really is a child of the `wt.exe` the
/// bench spawns —
///
/// ```text
///   новое окно принадлежит процессу 4796
///     4796   WindowsTerminal.exe
///     11976  <завершился>  <-- ЗАПУЩЕН СТЕНДОМ, корень реестра A
///   РОДСТВО: происходит от процесса 11976, запущенного стендом
/// ```
///
/// So requirement A's kinship test **passes**. What refuses the position is requirement **C**:
/// `WindowsTerminal` is on the protected list, and C exempts only an instance the bench spawned
/// **directly**, not one it merely descends from.
///
/// That is a deliberate policy, not an accident of the code, and the bench obeys it rather than
/// arguing with it: C exists precisely because a terminal usually belongs to the person at the
/// machine, and "it descends from something of mine" is the kind of reasoning that ended with
/// the controller's editor being closed twice. Whether C should be relaxed for a proved
/// descendant is a question for the controller, and it is in the report — not a decision the
/// bench takes for itself.
pub fn position_6(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        6,
        "Windows Terminal — PowerShell",
        "окно принадлежит WindowsTerminal.exe. Измерение: оно ПРОИСХОДИТ от запущенного \
         стендом wt.exe, то есть родство по пункту A есть. Отказывает пункт C: имя \
         WindowsTerminal в запретном списке, а C освобождает только экземпляр, запущенный \
         стендом напрямую, а не потомка",
    )
}

pub fn position_7(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        7,
        "Windows Terminal — cmd",
        "то же приложение и тот же способ запуска, что позиция 6: родство есть, отказывает \
         пункт C по имени процесса",
    )
}

/// A position handed to the acceptance session of §11.6 — **`pending`, owner `П`**.
///
/// ⚠ Not `fail`. A `fail` says the product misbehaved; these positions say nothing about the
/// product at all — the bench simply may not drive the window, and §11.6 exists for exactly
/// this. Decision Р-30 counts `pending` separately and keeps it out of "successful", which is
/// the honest accounting.
fn handed_to_session(number: u8, name: &str, reason: &str) -> Vec<Row> {
    vec![
        Row::pending(number, name, Assertion::Text, "П", EXPECTED).with_note(format!(
            "{reason}; сценарий для человека — в отчёте T-04-3-2"
        )),
        Row::pending(
            number,
            name,
            Assertion::Layout,
            "П",
            layout::describe(layout::RUSSIAN),
        )
        .with_note("та же причина, что у строки текста: сценарий целиком идёт в сессию §11.6"),
    ]
}

// ---------------------------------------------------------------------------------------
// Position 8 — Telegram Desktop
// ---------------------------------------------------------------------------------------

/// ⛔ **П — приёмочная сессия, по прямому указанию сноски 7 §11.3.**
///
/// # Footnote 7 anticipates this outcome by name
///
/// > Автоматизация возможна при условии, что UI Automation отдаёт поле ввода сообщения через
/// > `ValuePattern` либо `TextPattern`; у Qt-приложений поддержка неполная. **Если поле
/// > недоступно, позиция переводится в П**, заносится в `DEFERRED.md` и выполняется
/// > в приёмочной сессии.
///
/// # What was measured, step by step, before invoking that clause
///
/// The bench got further at every attempt, and each step was fixed on evidence rather than
/// guessed at. Recorded because the next person to touch this position should not repeat it:
///
/// | Attempt | Result | Cause found |
/// |---|---|---|
/// | window by title containing `Telegram` | окно не появилось | the title carries the **open chat's** name |
/// | window by class containing `Qt` | окно не появилось | the class is `class MainWindow`, no `Qt` in it |
/// | chat row by `name == "Избранное"` | чат не найден | the accessible name is a **composite** of the row's whole preview |
/// | chat row by prefix, bounded tree walk | чат не найден | the walk's depth and node bounds are reached before the row is |
/// | chat row by prefix, `FindAll(Descendants)`, asked once | чат не найден | the chat list is filled **after** the window appears |
/// | same, waited for | **found, opened, title confirms «Избранное»**, message box found and told apart from the search box | — |
/// | typed `ghbdtn` | **прочитано `""`** | keystrokes do not reach the field |
/// | `SetFocus` on the field first — it reports success | **прочитано `""`** | Qt's UI Automation provider accepts the focus call and the field still does not receive the input |
///
/// The last line is the one footnote 7 is about: the element is in the tree and exposes
/// `ValuePattern`, but the pairing of synthetic input with that provider does not work here, and
/// the bench has no honest way around it. Clicking into the field with a synthetic mouse is not
/// one — the position would then be testing the bench's aim, and §11.5 gives it no such licence.
///
/// ⚠ **`Enter` is still never sent, and the bench never left «Избранное».** The safety gate that
/// confirms the open chat from the window title stayed in the code and passed every time.
pub fn position_8(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        8,
        "Telegram Desktop",
        "измерено: окно и поле ввода находятся, «Избранное» открывается и подтверждается по \
         заголовку, но введённое через SendInput до поля не доходит — ValuePattern возвращает \
         пустую строку и после успешного SetFocus. Это ровно условие сноски 7 §11.3 о неполной \
         поддержке UI Automation у Qt-приложений",
    )
}

// ---------------------------------------------------------------------------------------
// Position 10 — Explorer search box
// ---------------------------------------------------------------------------------------

/// ⛔ **Not attempted.** The Explorer window belongs to the shell, which the bench may not drive.
///
/// `explorer.exe` does not create a window: it hands the request to the already-running shell
/// and exits. The folder window therefore belongs to a process the bench never started, so
/// requirement B forbids adopting it — and, `explorer` being on the protected list of
/// requirement C, forbids it twice.
///
/// ⚠ **Why this position is not even started, rather than started and failed.** The first run
/// under A–E did start it, was correctly refused at the adoption step, and left a folder window
/// open on the user's desktop — because a window the bench is not allowed to adopt is also a
/// window it is not allowed to close. Refusing after the fact is not enough when the attempt
/// itself leaves a trace; the honest form is to not make it.
pub fn position_10(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        10,
        "Проводник — поле поиска",
        "измерено: запущенный стендом explorer.exe завершается сам за <10 с и окном владеть \
         не может; окно CabinetWClass создаёт уже работающая оболочка (explorer.exe PID 6396), \
         которая не происходит ни от одного процесса стенда. Окно не открывается вовсе — \
         в части 1 такая попытка оставила окно на рабочем столе пользователя",
    )
}

// ---------------------------------------------------------------------------------------
// Classification of the six doubtful positions — criterion 29
// ---------------------------------------------------------------------------------------

/// Measures, for each doubtful position, whether the window belongs to a process the bench
/// started — and prints the parent chain that decides it.
///
/// ⚠ **Measurement, not assumption.** Criterion 29 asks for exactly this: the classification of
/// positions 1, 6, 7, 10, 12 and 22 must come from an experiment, not from what the application
/// is known to do. The controller's prior estimate is written beside each result so the two can
/// be compared.
///
/// Two of the six are measured **without opening their window at all**, and deliberately:
///
/// * **10 (Explorer)** — a folder window belongs to the running shell. Opening one would leave
///   a window on the user's desktop that the bench is then forbidden to close, which is what
///   happened in part 1. Instead the two facts that settle it are measured separately: the
///   `explorer.exe` the bench spawns **exits within a second**, so it can own no window, and
///   the shell process that would own it is not kin to anything the bench started.
/// * **12 (Start menu)** — same reasoning, and the shell processes are already running.
pub fn classify(automation: &dyn Fn() -> Option<Automation>) -> std::process::ExitCode {
    println!("--- КЛАССИФИКАЦИЯ ШЕСТИ ПОЗИЦИЙ ИЗМЕРЕНИЕМ (пункт 29) ---\n");

    let Some(automation) = automation() else {
        eprintln!("UI Automation недоступна");
        return std::process::ExitCode::from(1);
    };

    measure_launched(&automation, 1, "Блокнот", "notepad.exe", &[], "Notepad");
    measure_launched(
        &automation,
        6,
        "Windows Terminal",
        "wt.exe",
        &["-w", "new", "-p", "PowerShell"],
        "CASCADIA_HOSTING_WINDOW_CLASS",
    );
    measure_shell_owned(10, "Проводник", "explorer.exe", &["CabinetWClass"]);
    measure_shell_owned(
        12,
        "Меню «Пуск»",
        "StartMenuExperienceHost.exe / SearchHost.exe",
        &["Windows.UI.Core.CoreWindow"],
    );

    println!(
        "\nПозиции 7 и 22 — те же приложения, что 6 и 1: Windows Terminal с другим профилем \
         и Блокнот с удерживаемым Shift. Родство окна с реестром у них то же самое, \
         отдельного измерения не требуют."
    );

    std::process::ExitCode::SUCCESS
}

/// Launches an application and measures who ends up owning its window.
fn measure_launched(
    automation: &Automation,
    number: u8,
    title: &str,
    command: &str,
    args: &[&str],
    window_class: &str,
) {
    println!("=== позиция {number}: {title} ===");

    let before: Vec<u32> = automation
        .top_level_of_any(&|e: &Element| e.class() == window_class)
        .iter()
        .filter_map(Element::pid)
        .collect();
    println!("  окна класса {window_class} до запуска: {before:?}");

    let child = match Command::new(command).args(args).spawn() {
        Ok(child) => child,
        Err(error) => {
            println!("  запустить {command} не удалось: {error}\n");
            return;
        }
    };

    let mut app = launched(title, child, CloseWith::Terminate, None);
    let spawned = app.pid;
    println!("  стенд запустил {command}, PID {spawned} — он в реестре A");

    // A window of this class that was not there before.
    let found = wait::until(wait::WINDOW_TIMEOUT, || {
        automation
            .top_level_of_any(&|e: &Element| e.class() == window_class)
            .into_iter()
            .find(|w| w.pid().is_some_and(|pid| !before.contains(&pid)))
    });

    let Some(window) = found else {
        println!("  нового окна класса {window_class} не появилось\n");
        return;
    };

    let window_pid = window.pid().unwrap_or(0);
    println!("  новое окно принадлежит процессу {window_pid}");
    println!("  цепочка родителей (измерение):");
    for (pid, name) in crate::own::ancestry(window_pid) {
        let mark = if crate::own::is_spawned_root(pid) {
            "  <-- ЗАПУЩЕН СТЕНДОМ, корень реестра A"
        } else {
            ""
        };
        println!("    {pid:<6} {name}{mark}");
    }

    // ⚠ Two separate questions, and conflating them answers neither. Kinship is the measurement
    // criterion 29 asks for; the gate is what the safety code actually does, and it refuses a
    // protected name **before** it ever looks at kinship.
    match crate::own::kinship(window_pid) {
        Some(root) => println!("  РОДСТВО: происходит от процесса {root}, запущенного стендом"),
        None => println!("  РОДСТВО: не происходит ни от одного процесса, запущенного стендом"),
    }

    let verdict = crate::own::claim_window_process(window_pid);
    match &verdict {
        Ok(()) => println!(
            "  ВОРОТА A–E: пропускают\n  -> позиция {number} АВТОМАТИЗИРУЕМА, прогоняется стендом"
        ),
        Err(reason) => println!("  ВОРОТА A–E: {reason}\n  -> позиция {number} переводится в П"),
    }

    // Clean up. When the gate passed, the process to close is the **window's**, not the
    // launcher's: for a stub like `notepad.exe` the launcher has already exited, and its process
    // id may by then belong to somebody else entirely.
    if verdict.is_ok() {
        app.pid = window_pid;
        app.window = window.hwnd();
        println!("  уборка: {}", app.close());
    } else {
        println!(
            "  уборка: окно закрыть нельзя (пункт B) — снимаю только запущенное самим стендом"
        );
        if crate::shell::process_is_alive(spawned) {
            match crate::shell::terminate(spawned) {
                Ok(()) => println!("    процесс {spawned} снят"),
                Err(error) => println!("    {error}"),
            }
        } else {
            println!("    пусковой процесс {spawned} завершился сам");
        }
        println!(
            "    ⚠ ОКНО ОСТАЁТСЯ НА РАБОЧЕМ СТОЛЕ: {window_class}, процесс {window_pid} — \
             это цена измерения, закрыть его стенд не вправе"
        );
    }
    println!();
}

/// Measures a position whose window can only belong to the shell, without opening one.
fn measure_shell_owned(number: u8, title: &str, owner: &str, classes: &[&str]) {
    println!("=== позиция {number}: {title} ===");
    println!(
        "  окно этой позиции создаёт {owner} (классы: {})",
        classes.join(", ")
    );
    println!(
        "  ⚠ окно НЕ открывается: в части 1 такая попытка оставила на рабочем столе \
         пользователя окно, закрыть которое стенд не вправе. Измеряется то же самое, но \
         по таблице процессов."
    );

    // Every candidate owner process that is running right now.
    let names: Vec<&str> = owner.split(" / ").collect();
    let mut measured = false;

    for name in names {
        let stem = name.trim();
        let found = crate::own::pids_named(stem);
        for pid in found {
            measured = true;
            println!("  процесс-владелец {pid} ({stem}), цепочка родителей:");
            for (ancestor, ancestor_name) in crate::own::ancestry(pid) {
                let mark = if crate::own::is_spawned_root(ancestor) {
                    "  <-- ЗАПУЩЕН СТЕНДОМ"
                } else {
                    ""
                };
                println!("    {ancestor:<6} {ancestor_name}{mark}");
            }
            match crate::own::claim_window_process(pid) {
                Ok(()) => println!("  ⚠ ВЕРДИКТ: принадлежит реестру A — это неожиданно"),
                Err(reason) => println!("  ВЕРДИКТ: {reason}"),
            }
        }
    }

    if !measured {
        println!("  ни одного процесса с таким именем не запущено");
    }

    // ⚠ The other half of position 10 — that a spawned `explorer.exe` owns no window of its own
    // — is NOT measured by spawning one here: `explorer.exe` opens a folder window whatever it
    // is given, and that window would then be left on the user's desktop, which is the very
    // thing this function exists to avoid. It was measured for real by the first run under A–E:
    // the folder window that appeared belonged to process 8036 while the bench had spawned a
    // different one, and the refusal recorded that pid.

    println!("  -> позиция {number} переводится в П\n");
}

// ---------------------------------------------------------------------------------------
// Staging the ambient layout with an application window — task T-04-3-3
// ---------------------------------------------------------------------------------------

/// ⛔ Puts the **session** into `language`, using an application window of the bench's own.
///
/// # Why this exists and `layout::Ambient` was not enough
///
/// Position 11's window — the system dialog `#32770` — takes its layout when it is created and
/// honours no request afterwards, so its precondition has to be arranged **before** it exists.
/// That means changing what a newly created window is born with, and the measurement of
/// `--measure-layout` says what does and does not do it:
///
/// | Tried | Result |
/// |---|---|
/// | `ActivateKeyboardLayout` on the bench's thread | moves the thread, and a new window is born in the session's language anyway |
/// | a `WS_OVERLAPPEDWINDOW` of the bench's own, raised and moved to en-US | `GetForegroundWindow` returns it, and the next window is **still** born in the old layout |
/// | an application window of a process the bench started, moved to en-US | **the next window is born in en-US** |
///
/// So the bench uses an application. Chrome, because it is the one the matrix already launches
/// with a throwaway profile of its own (positions 3 and 4), which means no state of the user's
/// is touched and requirement A covers the process by construction.
///
/// Returns the launched application — the caller **must keep it alive** until the window that has
/// to inherit the layout exists — and a sentence for the report.
fn stage_ambient(ctx: &Context, language: u32) -> Result<(App, String), String> {
    let mut app = launch_chrome("stage", &["about:blank"])?;

    adopt_window(ctx.automation, &mut app, &is_chrome_window)?;
    let Some(hwnd) = app.window else {
        return Err("у окна подготовки нет дескриптора".to_owned());
    };

    shell::activate_window(app.pid, Some(hwnd))?;
    let settled = layout::ensure(hwnd, language, Duration::from_secs(5))?;

    let said = format!(
        "окружающая раскладка выставлена в {} окном подготовки",
        layout::describe(settled)
    );
    Ok((app, said))
}

/// **Requirement 5 of §11.5 for the ambient layout** — puts the session back where the run found
/// it, after every position and at the end of the run.
///
/// # Why it reads before it writes
///
/// The reading is cheap — one window of the bench's own, born, asked what layout it was given,
/// and closed — and it is almost always already right: only a position that ran to the point of
/// the product switching a layout leaves the session moved. Reading first turns the ordinary case
/// into no work at all, and it means the sentence in the report is a **measurement** of the state
/// rather than a claim about an action.
pub fn restore_ambient(ctx: &Context) -> String {
    let Some(wanted) = ctx.ambient_before else {
        return "окружающая раскладка не читалась — возвращать не к чему".to_owned();
    };

    let now = layout::ambient();
    if now == Some(wanted) {
        return format!(
            "окружающая раскладка на месте: {}",
            layout::describe(wanted)
        );
    }

    let staged = match stage_ambient(ctx, wanted) {
        Ok((mut app, _)) => {
            let closed = app.close();
            format!("возвращена окном подготовки ({closed})")
        }
        Err(error) => format!("⚠ вернуть не удалось: {error}"),
    };

    format!(
        "окружающая раскладка была {}, стала {} — {staged}; сейчас {}",
        layout::describe(wanted),
        now.map_or("<не читается>".to_owned(), layout::describe),
        layout::ambient().map_or("<не читается>".to_owned(), layout::describe)
    )
}

// ---------------------------------------------------------------------------------------
// The measurement position 11 rests on — rule Р-39
// ---------------------------------------------------------------------------------------

/// ⚠ **A measurement, not a scenario.** Answers the two questions position 11 turns on, with
/// numbers rather than with reasoning — `langsw-e2e --measure-layout`.
///
/// | Question | How it is answered |
/// |---|---|
/// | does the system dialog `#32770` honour `WM_INPUTLANGCHANGEREQUEST`? | the message is posted to it and the layout is read afterwards |
/// | does it honour the **second** link of the FR-50 chain? | `AttachThreadInput` + `ActivateKeyboardLayout`, and the layout is read again |
/// | does a *newly opened* window inherit the layout of the one in front, or the session default? | a **second** dialog is opened once the first has been moved, and the layout it opens in is read |
///
/// ⛔ Every window here belongs to a process the bench started — two copies of the Run dialog,
/// launched by `rundll32` exactly as position 11 launches it. The measurement deliberately does
/// **not** use Notepad: on this machine the packaged Notepad is a single process that a second
/// launch joins, so if any copy is already running the window belongs to somebody else and
/// requirement B refuses it, which is a fact about Notepad and not about layouts.
///
/// # `leave`
///
/// Which layout the session is left in. `None` means "the one it was found in", which is what
/// requirement 5 of §11.5 asks of every mode of this bench. Naming one instead is how the
/// machine is put **into** the state position 11 used to fail in, so that the fix can be shown
/// to work rather than asserted to: `langsw-e2e --measure-layout ru`.
pub fn measure_layout(automation: &Automation, leave: Option<u32>) {
    println!("--- ИЗМЕРЕНИЕ: окружающая раскладка и системный диалог #32770 ---\n");

    // The anchor is opened first and closed last: it is both the instrument of the last two
    // steps and the reading of what the session's layout was before any of this ran.
    let anchor = layout::Ambient::open().ok();
    let ambient_before = anchor.as_ref().and_then(layout::Ambient::observed);

    println!(
        "Подключённые раскладки: {:?}",
        layout::attached()
            .iter()
            .map(|id| layout::describe(*id))
            .collect::<Vec<_>>()
    );
    println!(
        "Окружающая раскладка до измерения: {}",
        ambient_before.map_or("<не читается>".to_owned(), layout::describe)
    );
    println!(
        "Раскладка потока стенда (в ней открывается новое окно этого процесса): {}\n",
        layout::describe(layout::id_of(layout::of_this_thread()))
    );

    let Some((mut first, first_hwnd)) = measure_open_dialog(automation, "первый") else {
        measure_leave_ambient(anchor.as_ref(), leave.or(ambient_before));
        return;
    };

    println!(
        "1. Первый диалог #32770 открылся в раскладке {}",
        layout::of_window(first_hwnd).map_or("<не читается>".to_owned(), |h| {
            layout::describe(layout::id_of(h))
        })
    );

    let _ = shell::activate_window(first.pid, Some(first_hwnd));
    match layout::ensure(first_hwnd, layout::US, Duration::from_secs(5)) {
        Ok(id) => println!(
            "2. WM_INPUTLANGCHANGEREQUEST (FR-50 звено 1): перешёл в {}",
            layout::describe(id)
        ),
        Err(error) => println!("2. WM_INPUTLANGCHANGEREQUEST (FR-50 звено 1): ОТКАЗ — {error}"),
    }

    match layout::attach_activate(first_hwnd, first.pid, layout::US, Duration::from_secs(5)) {
        Ok(id) => println!(
            "3. AttachThreadInput + ActivateKeyboardLayout (FR-50 звено 2): перешёл в {}",
            layout::describe(id)
        ),
        Err(error) => println!("3. FR-50 звено 2: ОТКАЗ — {error}"),
    }

    println!(
        "4. Раскладка потока стенда после звена 2: {}",
        layout::describe(layout::id_of(layout::of_this_thread()))
    );

    // ---- the second dialog: what does a window opened *now* start in? ----
    let second = measure_open_dialog(automation, "второй");

    if let Some((mut second, second_hwnd)) = second {
        println!(
            "\n5. Второй диалог, открытый уже после того, как первый оказался в {}, \
             открылся в раскладке {}",
            layout::of_window(first_hwnd).map_or("<не читается>".to_owned(), |h| {
                layout::describe(layout::id_of(h))
            }),
            layout::of_window(second_hwnd).map_or("<не читается>".to_owned(), |h| {
                layout::describe(layout::id_of(h))
            })
        );
        println!("Закрытие: {}", second.close());
    }

    println!("Закрытие: {}", first.close());

    // ---- the confound step 5 leaves open, and the one number that settles requirement 1а ----
    //
    // Both windows above were in RU, so "a new window inherits the session default" and "a new
    // window inherits the layout of the window in front" predict the same answer. This step
    // separates them: a window that **does** honour the request is put into en-US and left in
    // front, and a fresh dialog is opened behind it.
    measure_ambient_after_english(automation);

    // ---- and the same thing again, through the instrument the run actually uses ----
    measure_ambient_through_anchor(automation);

    // Requirement 5 of §11.5 holds over a measurement too: the session goes back to the layout
    // it was in, unless the caller asked for a particular one on purpose.
    measure_leave_ambient(anchor.as_ref(), leave.or(ambient_before));

    println!(
        "Подключённые раскладки после измерения: {:?}",
        layout::attached()
            .iter()
            .map(|id| layout::describe(*id))
            .collect::<Vec<_>>()
    );
}

/// ⚠ **The measurement requirement 1а of the task stands or falls on.**
///
/// "Restore the ambient layout after every position" only means something if a newly opened
/// window takes its layout from the window that was in front. Chrome is used as the window that
/// *does* honour `WM_INPUTLANGCHANGEREQUEST` — positions 3 and 4 rest on that — so it can be put
/// into en-US and left holding the foreground while a fresh Run dialog is opened behind it.
///
/// * dialog opens in en-US → the ambient really is inherited, and restoring it between positions
///   is both meaningful and sufficient;
/// * dialog opens in ru-RU → a new window takes the **session default input language**, which is
///   a setting of the user's own and nothing the bench may write.
fn measure_ambient_after_english(automation: &Automation) {
    let mut chrome = match launch_chrome("measure", &["about:blank"]) {
        Ok(app) => app,
        Err(error) => {
            println!("\n6. Chrome не запустился, шаг не выполнен: {error}");
            return;
        }
    };

    if let Err(reason) = adopt_window(automation, &mut chrome, &is_chrome_window) {
        println!("\n6. окно Chrome не усыновлено: {reason}");
        return;
    }
    let Some(chrome_hwnd) = chrome.window else {
        println!("\n6. у окна Chrome нет дескриптора");
        return;
    };

    let _ = shell::activate_window(chrome.pid, Some(chrome_hwnd));
    let english = layout::ensure(chrome_hwnd, layout::US, Duration::from_secs(5));
    match &english {
        Ok(id) => println!(
            "\n6. Окно Chrome переведено в {} — обычное окно запрос выполняет",
            layout::describe(*id)
        ),
        Err(error) => println!("\n6. Окно Chrome в en-US не перешло: {error}"),
    }

    if let Some((mut dialog, dialog_hwnd)) = measure_open_dialog(automation, "третий") {
        println!(
            "7. ⚠ ГЛАВНОЕ ЧИСЛО. Впереди окно в {}; открытый в этот момент диалог #32770 \
             открылся в раскладке {}",
            layout::of_window(chrome_hwnd).map_or("<не читается>".to_owned(), |h| {
                layout::describe(layout::id_of(h))
            }),
            layout::of_window(dialog_hwnd).map_or("<не читается>".to_owned(), |h| {
                layout::describe(layout::id_of(h))
            })
        );
        println!("Закрытие: {}", dialog.close());
    }

    println!("Закрытие: {}", chrome.close());
}

/// The same question as [`measure_ambient_after_english`], asked of [`layout::Ambient`] — the
/// instrument the run really uses, so that the run rests on a measured fact and not on the
/// expectation that a window of our own behaves like Chrome's.
fn measure_ambient_through_anchor(automation: &Automation) {
    let anchor = match layout::Ambient::open() {
        Ok(anchor) => anchor,
        Err(error) => {
            println!("\n8. окно-якорь не создалось: {error}");
            return;
        }
    };

    for language in [layout::RUSSIAN, layout::US] {
        match anchor.set(language, Duration::from_secs(5)) {
            Ok(id) => println!(
                "\n8. Окно-якорь стенда переведено в {}",
                layout::describe(id)
            ),
            Err(error) => {
                println!(
                    "\n8. Окно-якорь в {}: ОТКАЗ — {error}",
                    layout::describe(language)
                );
                continue;
            }
        }

        if let Some((mut dialog, dialog_hwnd)) = measure_open_dialog(automation, "якорный") {
            println!(
                "9. ⚠ ГЛАВНОЕ ЧИСЛО. {}; открытый в этот момент диалог #32770 открылся \
                 в раскладке {}",
                anchor.describe_now(),
                layout::of_window(dialog_hwnd).map_or("<не читается>".to_owned(), |h| {
                    layout::describe(layout::id_of(h))
                })
            );
            println!("Закрытие: {}", dialog.close());
        }
    }
}

/// Leaves the session in `wanted`, and says which one that was.
fn measure_leave_ambient(anchor: Option<&layout::Ambient>, wanted: Option<u32>) {
    match (anchor, wanted) {
        (Some(anchor), Some(id)) => match anchor.set(id, Duration::from_secs(5)) {
            Ok(now) => println!(
                "\nОкружающая раскладка сеанса оставлена в {}",
                layout::describe(now)
            ),
            Err(error) => println!("\n⚠ окружающая раскладка не выставлена: {error}"),
        },
        _ => println!("\n⚠ окружающая раскладка не выставлялась: якоря или значения нет"),
    }
}

/// One copy of the Run dialog, launched and adopted — the building block of [`measure_layout`].
fn measure_open_dialog(automation: &Automation, which: &str) -> Option<(App, HWND)> {
    let child = match Command::new("rundll32.exe").arg("shell32.dll,#61").spawn() {
        Ok(child) => child,
        Err(error) => {
            println!("{which} диалог не запустился: {error}");
            return None;
        }
    };

    let mut app = launched("Диалог «Выполнить»", child, CloseWith::WmClose, None);

    if let Err(reason) = adopt_window(automation, &mut app, &|element: &Element| {
        element.class() == "#32770"
            && (element.name().contains("Выполнить") || element.name().contains("Run"))
    }) {
        println!("{which} диалог не усыновлён: {reason}");
        return None;
    }

    let hwnd = app.window?;
    Some((app, hwnd))
}

// ---------------------------------------------------------------------------------------
// Position 11 — the Run dialog
// ---------------------------------------------------------------------------------------

/// ⚠ The Run dialog is opened by `rundll32 shell32.dll,#61` and **not** by sending `Win+R`.
///
/// Sending `Win+R` would mean sending synthetic input while the foreground window belongs to
/// somebody else — precisely what the safety rule of this task forbids. Launching the dialog as
/// a process gives the bench a window it started and can identify, which is what makes the
/// foreground check meaningful for this position.
pub fn position_11(ctx: &Context) -> Vec<Row> {
    let clipboard = clip::Guard::capture();

    // ⚠ **The precondition, and it has to be set before the window exists.** Measured with
    // `--measure-layout`: the dialog `#32770` takes its layout at the instant it is created and
    // honours neither link of the FR-50 chain afterwards — not
    // `PostMessage(WM_INPUTLANGCHANGEREQUEST)`, not `AttachThreadInput` +
    // `ActivateKeyboardLayout`. What it *does* honour is the layout of the session, which is the
    // layout of the window that was in front.
    //
    // Every other position sets the same precondition with `layout::ensure` after its window is
    // up. This one sets it a moment earlier, and it needs a window to set it **with**.
    //
    // ⚠ **An application window, and not a window of the bench's own** — both were measured, and
    // only one works. A window this process creates, raised until `GetForegroundWindow` returns
    // it and moved to en-US, leaves the next window created still opening in the old layout: the
    // layout follows the *thread* there, and a window born on that thread takes the session's
    // language again regardless. An ordinary application window of a process the bench started,
    // asked the same thing with the same message, **does** change what the next window is born
    // with. ⛔ Both are equally within A–E — the point of the choice is that one of them is a
    // fact and the other was a theory.
    let mut stage = stage_ambient(ctx, layout::US);
    let staged = match &stage {
        Ok((_, said)) => said.clone(),
        Err(error) => format!("⚠ окружающая раскладка не выставлена: {error}"),
    };

    let child = match Command::new("rundll32.exe").arg("shell32.dll,#61").spawn() {
        Ok(child) => child,
        Err(error) => return both_failed(11, "Диалог «Выполнить»", &format!("запуск: {error}")),
    };

    let mut app = launched("Диалог «Выполнить»", child, CloseWith::WmClose, None);

    let window = adopt_window(ctx.automation, &mut app, &|element: &Element| {
        element.class() == "#32770"
            && (element.name().contains("Выполнить") || element.name().contains("Run"))
    });

    // The dialog exists now and has taken its layout for good, so the window that staged the
    // ambient has done its work and goes — it must not hold the foreground while the scenario
    // types into the dialog.
    let opened_in = app
        .window
        .and_then(layout::of_window)
        .map(layout::id_of)
        .map_or("<не читается>".to_owned(), layout::describe);
    let stage_closed = match &mut stage {
        Ok((app, _)) => app.close(),
        Err(_) => "окна подготовки не было".to_owned(),
    };

    let mut rows = match (window, app.window) {
        (Ok(window), Some(hwnd)) => {
            match ctx
                .automation
                .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                    e.control_type() == Some(UIA_EditControlTypeId)
                }) {
                Some(content) => replacement(
                    ctx,
                    Scene {
                        position: 11,
                        app_name: "Диалог «Выполнить»",
                        pid: app.pid,
                        window: hwnd,
                        content: &content,
                        modifiers: &[],
                        clear_first: true,
                        rollback: false,
                    },
                ),
                None => both_failed(11, "Диалог «Выполнить»", "поле ввода не найдено"),
            }
        }
        (Err(reason), _) => both_failed(11, "Диалог «Выполнить»", &reason),
        (_, None) => both_failed(11, "Диалог «Выполнить»", "у диалога нет дескриптора"),
    };

    // ⚠ The field is emptied before the dialog is closed. Whatever is left in it is what the
    // dialog offers next time, and leaving the bench's marker in somebody's Run history is a
    // trace the run has no business leaving.
    if let Some(hwnd) = app.window {
        let target = input::Target { pid: app.pid, hwnd };
        let _ = input::chord(&[VK_CONTROL.0], VK_A.0, &target);
        let _ = input::tap(VK_DELETE.0, &target);
    }

    let closed = app.close();
    for row in &mut rows {
        row.note = format!(
            "{}; {staged}; диалог открылся в {opened_in}; окно подготовки — {stage_closed}; \
             закрытие: {closed}; буфер обмена не изменился: {}",
            row.note,
            clipboard.unchanged()
        );
    }
    rows
}

// ---------------------------------------------------------------------------------------
// Position 12 — Start menu search
// ---------------------------------------------------------------------------------------

/// ⛔ **Not attempted.** The Start menu belongs to the shell, and attempting it broke the run.
///
/// There is no private instance of the Start menu: its window belongs to
/// `StartMenuExperienceHost` or `SearchHost`, processes the bench never started, so requirement
/// B forbids adopting it. The first run under A–E confirmed that by refusing — the recorded
/// refusal names `StartMenuExperienceHost.exe`, a process whose name is **not** on the protected
/// list of requirement C, which is what makes it a clean demonstration of requirement B acting
/// on its own.
///
/// ⚠ **Why it is no longer even started.** That same run opened the menu with `SC_TASKLIST` and
/// could not close it again, and `SearchHost` was left holding the foreground. Every position
/// that ran afterwards then failed at `AppActivate`.
///
/// The controller has since corrected part of that diagnosis: the Start menu was **not** left
/// open — `SearchHost` holds the foreground in ordinary use too, and two leftover PowerShell
/// windows were what actually interfered. The conclusion for this position is unchanged, and
/// rests on the kinship measurement above rather than on that episode.
///
/// So the bench does not open the Start menu at all. A position it cannot finish is one thing;
/// a position that touches the shell to find that out is another.
pub fn position_12(_ctx: &Context) -> Vec<Row> {
    handed_to_session(
        12,
        "Поиск в меню «Пуск»",
        "измерено: окно создают StartMenuExperienceHost.exe (PID 7344) и SearchHost.exe \
         (PID 7336), оба происходят от svchost.exe -> services.exe -> wininit.exe и ни один \
         не происходит от процессов стенда. Частного экземпляра меню «Пуск» не бывает. \
         Меню не открывается вовсе",
    )
}

// ---------------------------------------------------------------------------------------
// Position 24 — the second instance
// ---------------------------------------------------------------------------------------

/// A second copy of the product must say so and end, leaving the first one running.
///
/// # ⚠ The first version of this position hung the whole run, and the reason is a requirement
///
/// It ran the second copy with `Command::output()`, which waits for the process to exit. FR-82
/// makes that wait unbounded: the "say so" half is a **modal `MessageBoxW`**, and `src\app.rs`
/// states outright that "blocking is a property of the requirement, not a defect" (decision
/// R-20 point 1). So the second copy sits on a dialog until a person closes it, `output()` sits
/// on the second copy, and the run stops — with a modal window left on somebody's screen.
///
/// That was a defect of the bench, not of the product, and the fix makes the position stronger
/// rather than merely unblocking it. The message box is not an obstacle to route around: it is
/// the observable form of the notification FR-82 requires, so the bench now **waits for it
/// through UI Automation, reads it, and reports it** — and only then dismisses it and collects
/// the exit code. Three assertions instead of one.
///
/// Dismissing the dialog is allowed under requirements A to E because the process that owns it
/// is one the bench started and registered.
pub fn position_24(ctx: &Context) -> Vec<Row> {
    const APP: &str = "Второй запуск программы";

    let exe = match crate::sut::executable() {
        Ok(exe) => exe,
        Err(error) => {
            return vec![Row::new(
                24,
                APP,
                Assertion::Other("второй экземпляр завершается"),
                Verdict::Fail,
                error,
                format!("код возврата {}", crate::sut::EXIT_ALREADY_RUNNING),
            )];
        }
    };

    let child = match Command::new(&exe)
        .env("LANGSW_DEBUG_TIMEOUT_SEC", "20")
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return vec![Row::new(
                24,
                APP,
                Assertion::Other("второй экземпляр завершается"),
                Verdict::Fail,
                format!("не удалось запустить: {error}"),
                format!("код возврата {}", crate::sut::EXIT_ALREADY_RUNNING),
            )];
        }
    };

    // Registers the process — requirement A — so that the dialog below may be dismissed.
    let mut app = launched(APP, child, CloseWith::WmClose, None);
    let second = app.pid;

    let mut rows = Vec::new();

    // FR-82, the notification half: a modal box of class `#32770` owned by the second copy.
    let dialog =
        ctx.automation
            .await_window(second, Duration::from_secs(30), &|element: &Element| {
                element.class() == "#32770"
            });

    match &dialog {
        Some(box_window) => {
            let texts: Vec<String> = ctx
                .automation
                .find_all(box_window, &|e: &Element| !e.name().is_empty())
                .iter()
                .map(Element::name)
                .collect();
            rows.push(
                Row::new(
                    24,
                    APP,
                    Assertion::Other("FR-82: показано уведомление"),
                    Verdict::Pass,
                    format!("модальное окно #32770: {}", texts.join(" | ")),
                    "модальное окно с сообщением, что программа уже запущена",
                )
                .with_note(format!("заголовок окна {:?}", box_window.name())),
            );
        }
        None => rows.push(Row::new(
            24,
            APP,
            Assertion::Other("FR-82: показано уведомление"),
            Verdict::Fail,
            "модальное окно не появилось за 30 с",
            "модальное окно с сообщением, что программа уже запущена",
        )),
    }

    // The first copy must still be there — the other half of the position, asked while the
    // second copy is still on screen.
    let alive = crate::channel::read().is_ok();
    rows.push(Row::new(
        24,
        APP,
        Assertion::Other("первый экземпляр работает"),
        if alive { Verdict::Pass } else { Verdict::Fail },
        if alive {
            "канал SEC-04a первого экземпляра отвечает"
        } else {
            "канал SEC-04a не отвечает — первый экземпляр не работает"
        },
        "первый экземпляр продолжает работу",
    ));

    // Dismiss the box — the OK button if it is reachable, `WM_CLOSE` otherwise. `App::close`
    // below waits for the process and, if it is still there, ends it through the registry gate.
    if let Some(box_window) = &dialog {
        let pressed = ctx
            .automation
            .find(box_window, &|e: &Element| {
                e.control_type() == Some(UIA_ButtonControlTypeId)
            })
            .is_some_and(|button| button.invoke());

        if !pressed && let Some(hwnd) = box_window.hwnd() {
            // SAFETY: `hwnd` belongs to a dialog of a process this bench started and registered;
            // `PostMessageW` copies a message and dereferences nothing of ours. NFR-13: the
            // result is examined by the wait that follows.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
    }

    let ended = wait::until_true(Duration::from_secs(20), || !shell::process_is_alive(second));

    let code = app
        .child
        .as_mut()
        .and_then(|child| child.try_wait().ok().flatten())
        .and_then(|status| status.code());

    rows.push(Row::new(
        24,
        APP,
        Assertion::Other("второй экземпляр завершается"),
        if code == Some(crate::sut::EXIT_ALREADY_RUNNING) {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        match code {
            Some(code) => format!("код возврата {code}"),
            None if ended => "процесс завершился, код возврата не прочитан".to_owned(),
            None => "процесс не завершился за 20 с после закрытия окна".to_owned(),
        },
        format!("код возврата {}", crate::sut::EXIT_ALREADY_RUNNING),
    ));

    let closed = app.close();
    if let Some(row) = rows.first_mut() {
        row.note = format!("{}; закрытие: {closed}", row.note);
    }

    rows
}

// ---------------------------------------------------------------------------------------
// The positions that wait for a task that does not exist yet
// ---------------------------------------------------------------------------------------

/// Title of the bench's own password window — the string position 14 adopts by.
///
/// The window appends `len=N` to it on every change of the field, so the match is on the prefix.
const PASSWORD_WINDOW_TITLE: &str = "LangSw-Password-14";

/// Position 14 — **a password field, and the one position SEC-06 is proved by.**
///
/// ⚠ This is the one position where the result is invisible from outside — requirement 4 of
/// §11.5 names it, and footnote 2 of §11.3 says how: «через тестовое приложение с полем
/// `ES_PASSWORD` … и отладочный канал SEC-04a, подтверждающий нулевую длину буфера». The
/// contents of a password field are not reachable from outside by the design of Windows, so the
/// only witness there can be is the product's own `buffer_len`, and the only thing that says the
/// product *recognised* the field is its own `password_field` — the key task T-06-1 added.
///
/// ⛔ **The window is the bench's own.** Requirements A to E are not relaxed for this position or
/// for any other: nothing on this machine is searched for a password box, no window of anybody
/// else's is adopted, and nothing is typed into one. The bench starts a process of its own, that
/// process opens a form with a single masked text box, and the six keystrokes go there.
///
/// ⚠ **No password is typed.** What is typed is `ghbdtn`, the same six letters every other
/// position types, into a box that happens to be masked. There is no secret anywhere in this
/// scenario — the point is the *shape* of the control, not what is in it.
///
/// # What the three readings mean
///
/// | Reading | Says |
/// |---|---|
/// | the window title, `len=6` | the six keystrokes **reached the field** — FR-70's «ввод не подавляется» |
/// | `password_field=1` | the product recognised the field — FR-72 |
/// | `buffer_len=0` | the product recorded none of them — FR-70, SEC-06 |
///
/// All three are needed and none of them is enough alone: `buffer_len=0` beside a field that
/// never received the keystrokes would be a scenario that proved nothing, and `buffer_len=0`
/// beside `password_field=0` would mean the buffer was empty for some other reason.
pub fn position_14(ctx: &Context) -> Vec<Row> {
    const APP: &str = "Поле пароля (окно стенда)";

    let assertion = Assertion::Other("буфер остаётся пустым");
    let expected = "pass=6, password_field=1, buffer_len=0".to_owned();

    let failed = |reason: String| -> Vec<Row> {
        vec![Row::new(
            14,
            APP,
            assertion,
            Verdict::Fail,
            reason,
            expected.clone(),
        )]
    };

    let mut app = match launch_password_window() {
        Ok(app) => app,
        Err(error) => return failed(error),
    };

    let window = match adopt_window(ctx.automation, &mut app, &|element: &Element| {
        element.name().starts_with(PASSWORD_WINDOW_TITLE)
    }) {
        Ok(window) => window,
        Err(reason) => return failed(reason),
    };

    let Some(hwnd) = app.window else {
        return failed("у окна поля пароля нет дескриптора".to_owned());
    };

    if let Err(error) = shell::activate_window(app.pid, Some(hwnd)) {
        return failed(error);
    }

    let target = input::Target { pid: app.pid, hwnd };

    // ⚠ **`Tab` first, and it is not a formality.** The window opens with the focus in its
    // *ordinary* box; this moves it into the password one. Measured: bringing a window forward
    // and relying on the focus it restores by itself is not enough — the focus event that
    // matters can be raised while the activation is still in flight, and the product then holds
    // a verdict about the moment before. A `Tab` after everything has settled raises an
    // `EVENT_OBJECT_FOCUS` whose subject is unambiguous, and it is also what a person does.
    //
    // It is also the reading of the ordinary box, which is what makes the position a
    // *comparison* rather than an assertion about one field: the same window, the same process,
    // the same keystrokes, and a different answer.
    if let Err(error) = input::type_text(TYPED, &target) {
        let closed = app.close();
        return failed(format!(
            "ввод в обычное поле не отправлен: {error}; закрытие: {closed}"
        ));
    }

    let ordinary = wait::until(wait::TEXT_TIMEOUT, || {
        crate::channel::read()
            .ok()
            .filter(|snapshot| snapshot.get("buffer_len") == Some("6"))
    })
    .is_some();

    if let Err(error) = input::tap(VK_TAB.0, &target) {
        let closed = app.close();
        return failed(format!("Tab не отправлен: {error}; закрытие: {closed}"));
    }

    // The product needs its own moment: the focus change is delivered to it as a `WinEvent`, and
    // the three levels of FR-72 run on its watcher thread afterwards. Waiting on the **answer**
    // rather than on a clock is requirement 1 of §11.5 — and it is also the honest way to phrase
    // it, because what is being waited for is a verdict and not a duration.
    let recognised = wait::until(Duration::from_secs(10), || {
        crate::channel::read()
            .ok()
            .filter(|snapshot| snapshot.get("password_field") == Some("1"))
    })
    .is_some();

    if !recognised {
        let seen = crate::channel::read().map_or_else(
            |error| format!("канал недоступен: {error}"),
            |snapshot| {
                format!(
                    "password_field={}",
                    snapshot
                        .get("password_field")
                        .unwrap_or("<ключ отсутствует>")
                )
            },
        );
        let closed = app.close();
        return failed(format!(
            "продукт не признал поле паролем за 10 с: {seen}; закрытие: {closed}"
        ));
    }

    if let Err(error) = input::type_text(TYPED, &target) {
        let closed = app.close();
        return failed(format!("ввод не отправлен: {error}; закрытие: {closed}"));
    }

    // The window puts the **length** of its password box into its own title on every change —
    // never the text. Waiting for `pass=6` is how the bench knows the keystrokes landed instead
    // of assuming they did, which is the same rule step 5 of every other position follows, and
    // it is the whole of FR-70's «ввод не подавляется» for this position.
    let landed = wait::until_true(wait::TEXT_TIMEOUT, || window.name().contains("pass=6"));

    let typed_note = format!("заголовок окна: {:?}", window.name());

    let snapshot = crate::channel::read();

    let closed = app.close();

    let (verdict, actual, note) = match snapshot {
        Ok(snapshot) => {
            let buffer_len = snapshot.get("buffer_len").unwrap_or("<ключ отсутствует>");
            let password = snapshot
                .get("password_field")
                .unwrap_or("<ключ отсутствует>");

            let verdict = if landed && buffer_len == "0" && password == "1" {
                Verdict::Pass
            } else {
                Verdict::Fail
            };

            (
                verdict,
                format!("pass=6: {landed}, password_field={password}, buffer_len={buffer_len}"),
                format!(
                    "окно стенда: обычное поле и поле с ES_PASSWORD; в обычном поле те же \
                     {TYPED:?} дали buffer_len=6: {ordinary}; после Tab фокус в поле пароля; \
                     {typed_note}; присутствуют ключи: {}; закрытие: {closed}",
                    snapshot.present_keys().join(", ")
                ),
            )
        }
        Err(error) => (
            Verdict::Fail,
            format!("канал недоступен: {error}"),
            format!("{typed_note}; закрытие: {closed}"),
        ),
    };

    vec![Row::new(14, APP, assertion, verdict, actual, expected).with_note(note)]
}

/// Starts the bench's own window: one ordinary text box and one masked one — the "тестовое
/// приложение с полем `ES_PASSWORD`" of footnote 2 of §11.3.
///
/// # Why a `WinForms` window driven by a child process
///
/// The bench needs a real `ES_PASSWORD` control in a process **requirements A to E allow it to
/// type into**, and requirement E asks that the foreground window belong to a process the bench
/// started. A window created inside the bench's own process would fail that check unless the
/// bench's own process were put into registry A, which is a weakening of the very rule that
/// exists because this bench once closed somebody's editor. A child process the bench spawns
/// needs no such exception: it enters registry A through [`launched`], exactly as Chrome,
/// Notepad and the Run dialog do.
///
/// `System.Windows.Forms.TextBox` with `UseSystemPasswordChar` is a Win32 `EDIT` with
/// `ES_PASSWORD` — that is what the property sets — so level 2 of FR-72 sees precisely the
/// control the requirement names, and level 3 would see `IsPassword` if level 2 ever stopped
/// answering.
///
/// # Two boxes and not one
///
/// The window opens with the focus in the **ordinary** box, and the scenario tabs into the
/// masked one. That buys two things: a focus event whose subject is unambiguous and which
/// happens after the activation has completely settled, and a control reading — the same six
/// keystrokes, in the same window, of the same process, one field apart, with opposite answers.
///
/// The title carries the **lengths** of the two fields and never a character of either. That is
/// what lets the scenario show the keystrokes arrived without reading anything out of a masked
/// box (SEC-01, SEC-07 — and the same rule the product holds itself to).
///
/// The script is written into the throwaway directory the `App` removes with the process, so the
/// bench gains no permanent file and `tests\e2e\` gains no second script.
fn launch_password_window() -> Result<App, String> {
    let scratch = scratch_dir("password");
    let script = scratch.join("password-window.ps1");

    let body = format!(
        "Add-Type -AssemblyName System.Windows.Forms\n\
         $form = New-Object System.Windows.Forms.Form\n\
         $form.Text = '{PASSWORD_WINDOW_TITLE} plain=0 pass=0'\n\
         $form.Width = 460\n\
         $form.Height = 220\n\
         $form.StartPosition = 'CenterScreen'\n\
         $form.TopMost = $true\n\
         $plain = New-Object System.Windows.Forms.TextBox\n\
         $plain.Left = 20\n\
         $plain.Top = 40\n\
         $plain.Width = 400\n\
         $plain.TabIndex = 0\n\
         $box = New-Object System.Windows.Forms.TextBox\n\
         $box.UseSystemPasswordChar = $true\n\
         $box.Left = 20\n\
         $box.Top = 100\n\
         $box.Width = 400\n\
         $box.TabIndex = 1\n\
         $retitle = {{ $form.Text = '{PASSWORD_WINDOW_TITLE} plain=' + $plain.Text.Length + ' pass=' + $box.Text.Length }}\n\
         $plain.Add_TextChanged($retitle)\n\
         $box.Add_TextChanged($retitle)\n\
         $form.Controls.Add($plain)\n\
         $form.Controls.Add($box)\n\
         $form.Add_Shown({{ $form.Activate(); [void]$plain.Focus() }})\n\
         [void]$form.ShowDialog()\n"
    );

    std::fs::write(&script, body)
        .map_err(|error| format!("не удалось записать {}: {error}", script.display()))?;

    let child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-STA",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .spawn()
        .map_err(|error| format!("запуск окна поля пароля: {error}"))?;

    Ok(launched(
        "Поле пароля (окно стенда)",
        child,
        CloseWith::WmClose,
        Some(scratch),
    ))
}

/// Every position the bench cannot run, with who it now waits on.
///
/// ⚠ **After task T-10-2 there is no `pending` row owned by a closed task.** The debt this list
/// exists to prevent — a position reported as waiting for a task that has already finished — was
/// carried by three rows: position 9 named the closed T-09-1, and positions 18 and 20 named the
/// closed T-06-2. All three are corrected here to **П**, and every remaining `pending` is a
/// position of the acceptance session of §11.6, verified by a person:
///
/// * **9** — the bench cannot drive an elevated (High-integrity) window: measured in T-10-2,
///   `SetForegroundWindow` from medium integrity returns false and the elevated edit element is
///   invisible to a medium UI Automation client, both UIPI. The product's `uiAccess` hook works
///   over it; a person types into the elevated Notepad and watches the conversion.
/// * **13** — RDP, optional (question 31), never implemented by choice.
/// * **18, 20** — moved from А to П by the user (SPEC §11.3, `d063407`): the desktop switch and
///   the shell restart are a person's actions (footnote 4 of §11.3, and `explorer` is on the
///   protected list of `own.rs`).
/// * **19, 21** — locking the session and sleeping the machine are done by a person, and the
///   task was explicit that no stub is written for them.
///
/// They appear here so the summary counts them, and nowhere else.
pub fn pending_positions() -> Vec<Row> {
    let scenario = format!("текст {EXPECTED:?} и раскладка RU");

    vec![
        // ⚠ **Owner is «П», not «T-09-1».** T-09-1 is closed and its `uiAccess` mechanism is
        // shipped (the installed product carries `UIAccess=1`), so a row that still named it as
        // the missing owner would be the untruth this list exists to prevent. The position is
        // pending because the **bench** cannot drive it, not because a task has not written it:
        // task T-10-2 measured that a medium-integrity bench with no `uiAccess` can neither bring
        // the elevated (High-integrity) Notepad forward — `SetForegroundWindow` returns false —
        // nor read its edit element through UI Automation, both being UIPI consequences. The
        // product with `uiAccess` works over the elevated window; a person verifies it in the
        // acceptance session of §11.6, exactly as for positions 5, 6, 7, 10 and 12. SPEC §11.3
        // still marks this position А¹; moving it to П there is the user's decision (T-10-2 report).
        Row::pending(
            9,
            "Блокнот от администратора",
            Assertion::Other("работа поверх элевированного окна"),
            "П",
            &scenario,
        )
        .with_note(
            "П — приёмочная сессия §11.6: стенд (средняя целостность, без uiAccess) не может ни \
             вывести вперёд, ни прочитать элевированное High-окно (UIPI), измерено T-10-2; \
             продукт с uiAccess проверяется человеком. SPEC §11.3 помечает А¹ — перевод в П за \
             пользователем",
        ),
        Row::pending(
            13,
            "Сеанс RDP",
            Assertion::Other("проброс ввода"),
            "П",
            &scenario,
        )
        .with_note("не реализуется вовсе — решение по вопросу 31; позиция необязательная"),
        // ⚠ **Position 15 is no longer here.** The selection path of §4.7 exists — task T-07-2,
        // FR-60, FR-61 and FR-65 — and the bench runs the position: see [`position_15`]. A row
        // that still called it `pending` on a task that has finished would be the same kind of
        // untruth this list exists to prevent.
        //
        // ⚠ **Positions 16 and 17 are no longer here.** Both are run by the bench now: task
        // T-04-3-3 added the list entry and the `match` arm that task T-05-2a was not allowed to
        // write (decision Р-52), and gave the bench the two abilities position 17 needed — the
        // temporary third layout of footnote 3 and the borrowing of `config.toml`. A row that
        // still called them `pending` would be the same kind of untruth this list exists to
        // prevent: a matrix that reports a position as waiting for a task that has finished.
        // ⚠ **Owner is «П», not «T-06-2».** T-06-2 is closed and the watchdog of §4.9 is
        // implemented; the user moved positions 18 and 20 from А to П by decision (SPEC §11.3,
        // commit `d063407`). Footnote 4 of §11.3 is explicit that the desktop switch is verified
        // by a person pressing `Ctrl+Alt+Del`, because the protected desktop it exercises is
        // unreachable to the bench. A row still naming a closed task as owner would be the untruth
        // this list exists to prevent — the same debt that positions 16 and 17 used to carry.
        Row::pending(
            18,
            "Переключение рабочего стола",
            Assertion::Other("хук восстановлен"),
            "П",
            "хук восстановлен",
        )
        .with_note(
            "П по решению пользователя (SPEC §11.3, d063407) — приёмочная сессия §11.6; сторож \
             хука T-06-2 реализован, но защищённый рабочий стол требует живого Ctrl+Alt+Del \
             (сноска 4 §11.3)",
        ),
        Row::pending(
            19,
            "Блокировка и разблокировка сеанса",
            Assertion::Other("хук восстановлен"),
            "П",
            "хук восстановлен",
        )
        .with_note("П — приёмочная сессия §11.6; заготовка не пишется по указанию задания"),
        // ⚠ **Owner is «П», not «T-06-2».** As with position 18: the watchdog is implemented and
        // the user moved this position to П (SPEC §11.3, `d063407`). Taking down `explorer.exe` is
        // forbidden to the bench anyway — the name is on the protected list of `own.rs` — so the
        // shell restart is a person's action in §11.6.
        Row::pending(
            20,
            "Перезапуск explorer.exe",
            Assertion::Other("иконка в трее восстановлена"),
            "П",
            "иконка восстановлена",
        )
        .with_note(
            "П по решению пользователя (SPEC §11.3, d063407) — приёмочная сессия §11.6; снятие \
             explorer стенду запрещено (запретный список own.rs)",
        ),
        Row::pending(
            21,
            "Сон и пробуждение",
            Assertion::Other("хук восстановлен"),
            "П",
            "хук восстановлен",
        )
        .with_note("П — приёмочная сессия §11.6; заготовка не пишется по указанию задания"),
        // ⚠ **Position 23 is no longer here.** The QPC instrument of criterion 2 §13 exists —
        // task T-10-1 — and the bench runs the position: see [`position_23`]. A row that
        // still called it `pending` on a task that has finished would be the same kind of
        // untruth this list exists to prevent.
    ]
}

// ---------------------------------------------------------------------------------------
// Task T-10-9 — the Explorer experiment, and the instrument it is measured with
// ---------------------------------------------------------------------------------------

/// The keys of SEC-04a this experiment reads after **every** step of every round.
///
/// The list of the task, in the task's order, with the three keys T-10-9 put on the channel at
/// the end — they are the ones the first hypothesis is decided by, and until this task they
/// existed only in the file report the main thread writes *after the process exits*, which a
/// scenario that requires the process to survive `Win+E` cannot read at all.
const WATCHED_KEYS: [&str; 30] = [
    "buffer_len",
    // **Task T-10-10, and first in importance after `buffer_len` itself.** The hypothesis this
    // run exists to test is that the gate of FR-70/FR-71 stays shut for ever, and that is the
    // one thing the twenty-four keys below could not say: `password_field` reads `0` both for an
    // ordinary field and for a gate still waiting on a verdict with buffering off, and the
    // parking that follows goes past the flush counters, so `strokes_removed` stands in both
    // cases too. A gate stuck in `pending` prints `pending` here while everything else looks
    // healthy.
    "field_state",
    "hotkey_handoffs",
    "cycle_position",
    "active_layout",
    "replacement_method",
    "last_replacement",
    "last_replacement_method",
    "send_mismatches",
    "post_failures",
    "events_lost",
    "fail_safe",
    "consecutive_panics",
    "window_flushes",
    "full_clears",
    "strokes_removed",
    "background_skips",
    "focus_repeats",
    // Beyond the list of the task, and each for a reason the first run made concrete:
    // `buffer_len=0` has **two** causes and they have to be told apart — a flush of the
    // watchdog, which moves `strokes_removed`, and the parking of FR-70/FR-71, which does not.
    // These five are what say which of them, and whether the verdict that ends the parking
    // ever came back.
    "focus_changes",
    "password_probes",
    "password_field",
    "layout_probes",
    "cache_builds",
    "layout_cache_failures",
    "hook_installed",
    // ⭐ **Task T-10-15.** The five outcomes of `buffer::Recorder::restamp`, which until that task
    // were one reading: four of them leave the stamp exactly where it was, so `active_layout`
    // above says the same thing for all four. `restamp_uncached` is the silent refusal — the
    // repair of defect E declining because the cache of FR-20 has no map for what the probe read.
    "restamp_skips",
    "restamp_no_probe",
    "restamp_unchanged",
    "restamp_uncached",
    "restamp_accepted",
];

/// Where the full protocol of the experiment is written — **debt 4 of the verdict on T-10-9**.
///
/// ⚠ **A file and not the filtered output.** The one observation that would have decided the
/// previous experiment — a single `gривет`, a stroke lost in the first seconds after `Win+E` —
/// was seen on screen and then lost, because the counters of that round existed only in a
/// console line that had been filtered on the way past. The verdict wrote that up as a debt.
/// So every reading this experiment takes goes into a file **whole**: not [`WATCHED_KEYS`] but
/// every key the product published, at every step of every round, whether or not anything
/// interesting happened at the time. What is interesting is decided afterwards, and afterwards
/// is exactly when a filtered line is gone.
///
/// A fixed path under the system temporary directory. No new environment variable (Р-53), and
/// nothing written into the project tree, which has to stay clean for `git status`.
fn protocol_path() -> std::path::PathBuf {
    std::env::temp_dir().join("langsw-experiment-explorer.log")
}

/// Appends one block to the protocol file, creating it on first use.
///
/// Failures are reported and never fatal: a run that cannot open its log is still a run, and
/// losing the experiment because the log could not be written would be the wrong trade. The
/// handle is opened per call rather than held, so a crash in the middle of the experiment leaves
/// everything up to that point already on disk — which is the entire point of the file.
fn protocol(block: &str) {
    use std::io::Write;

    let path = protocol_path();
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path);

    match opened {
        Err(error) => eprintln!("⚠ протокол {} не открылся: {error}", path.display()),
        Ok(mut file) => {
            if let Err(error) = writeln!(file, "{block}") {
                eprintln!("⚠ протокол не записался: {error}");
            }
        }
    }
}

/// One reading of [`WATCHED_KEYS`], in one line, for the step-by-step protocol.
///
/// ⚠ **The line it returns is a summary; the file gets everything.** Every call writes the whole
/// snapshot — all of `control::KEYS`, as the product rendered it — into [`protocol_path`] under
/// the same label, so the console stays readable and nothing is lost to the filter. Debt 4.
fn watched(label: &str) -> String {
    match crate::channel::read() {
        Err(error) => {
            protocol(&format!("[{label}] канал не ответил: {error}"));
            format!("  [{label}] канал не ответил: {error}")
        }
        Ok(snapshot) => {
            protocol(&format!("[{label}]\n{}", snapshot.raw.trim_end()));

            let pairs: Vec<String> = WATCHED_KEYS
                .iter()
                .map(|key| format!("{key}={}", snapshot.get(key).unwrap_or("<нет>")))
                .collect();
            format!("  [{label}] {}", pairs.join(" "))
        }
    }
}

/// The verdict of **one press**, decided by comparing the field before it with the field after
/// it — the instrument, and the whole reason this experiment exists.
///
/// # ⚠ Why it is not a comparison with `привет`
///
/// The controller's first instrument compared the reading against the sample `привет` and
/// reported «конвертация работает». The window's layout was Russian at the time, so `ghbdtn`
/// **already read `привет` while it was being typed**: the check passed without executing the
/// thing it was checking. Nothing here compares against a sample as its verdict. What decides
/// is `after != before` — the field changed when the hotkey was pressed — and the expected
/// text is carried alongside as a *description*, never as the test.
#[derive(Debug, Clone)]
struct PressOutcome {
    before: String,
    after: String,
    /// What FR-33 says the field should read at this point in the cycle. Reported, not asserted.
    wanted: String,
}

impl PressOutcome {
    /// **The detector.** The press did something to the field.
    fn changed(&self) -> bool {
        self.after != self.before
    }

    /// The press did the *right* thing — reported beside [`Self::changed`], and never instead
    /// of it.
    fn correct(&self) -> bool {
        self.after == self.wanted
    }

    fn describe(&self) -> String {
        format!(
            "до={:?} после={:?} ожидалось={:?} — изменилось: {}, верно: {}",
            self.before,
            self.after,
            self.wanted,
            if self.changed() { "ДА" } else { "НЕТ" },
            if self.correct() { "да" } else { "нет" },
        )
    }
}

/// Clears the field, types `ghbdtn` at a human tempo, presses the hotkey once and reports what
/// the field read **before** and **after** the press.
///
/// The tempo is [`type_paced`]'s — each character confirmed in the field before the next one is
/// sent — because the household gesture the defect was found with is a person typing, and the
/// window between the last stroke and the hotkey is where several of this project's defects have
/// lived. The two waits are conditions and never clocks (requirement 1 of §11.5): the first waits
/// for the typing to read back, the second for the field to stop reading what it read before the
/// press, and the second's timeout is spent **only** when nothing happened at all — which is
/// exactly the outcome under investigation.
fn press_once(
    ctx: &Context,
    target: &input::Target,
    content: &Element,
    label: &str,
    source: u32,
) -> Result<PressOutcome, String> {
    // ⚠ **The precondition of the scenario, restored at the start of every round and not once
    // at the start of the run.** Step 5 of FR-40 leaves the window in RU after a successful
    // replacement, and the very first trial run of this experiment showed what that costs: the
    // second round typed the `G` key and the field read `п`. That is the controller's own trap
    // seen from the typing side — the same Russian layout that made `ghbdtn` read `привет`
    // without any conversion happening — and it is answered here rather than tolerated.
    layout::ensure(target.hwnd, source, Duration::from_secs(5))
        .map_err(|error| format!("исходная раскладка перед кругом: {error}"))?;

    // The **same six keys** whichever layout the window runs, because a person's fingers do not
    // change: what changes is what the field shows for them, and therefore which direction the
    // conversion has to go. Under US the field reads `ghbdtn` and the press must produce
    // `привет`; under RU the field already reads `привет` — the controller's trap — and the
    // press must produce `ghbdtn`.
    let (shown, wanted) = if source & 0xFFFF == layout::RUSSIAN & 0xFFFF {
        (EXPECTED, TYPED)
    } else {
        (TYPED, EXPECTED)
    };

    input::chord(&[VK_CONTROL.0], VK_A.0, target).map_err(|error| format!("Ctrl+A: {error}"))?;
    input::tap(VK_DELETE.0, target).map_err(|error| format!("Delete: {error}"))?;
    wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(String::is_empty)
    });

    println!("{}", watched(&format!("{label}: поле очищено")));

    type_paced_as(TYPED, shown, target, content, "")?;

    let before = read_field(content).ok_or_else(|| "поле не читается перед нажатием".to_owned())?;
    println!("{}", watched(&format!("{label}: набрано, до нажатия")));

    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("горячая клавиша: {error}"))?;

    // The condition is «поле больше не читается так, как читалось до нажатия» — not «поле
    // читается как образец». A wrong replacement satisfies it as fast as a right one; only a
    // press that did nothing spends the whole bound.
    let after = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|text| text != &before)
    })
    .or_else(|| read_field(content))
    .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    println!("{}", watched(&format!("{label}: после нажатия")));

    Ok(PressOutcome {
        before,
        after,
        wanted: wanted.to_owned(),
    })
}

/// [`type_paced`] for a window whose layout is not the one the keys are named in.
///
/// `keys` is what is pressed — always `ghbdtn`, the six keys of the matrix — and `shown` is what
/// the field is expected to read them back as, which under the Russian layout is `привет`. The
/// two are separate arguments precisely because conflating them is the mistake this whole task
/// is about: the same six keystrokes read as two different strings depending on a value the
/// instrument must not assume.
///
/// `prefix` is what the field already reads before the first key — empty for the rounds of
/// [`press_once`], which clear the field first, and the text left by an earlier press for the
/// sequence of task T-10-11, which deliberately does not.
fn type_paced_as(
    keys: &str,
    shown: &str,
    target: &input::Target,
    content: &Element,
    prefix: &str,
) -> Result<(), String> {
    let mut expected = prefix.to_owned();

    for (key, character) in keys.chars().zip(shown.chars()) {
        let mut one = [0u8; 4];
        let one = key.encode_utf8(&mut one);
        input::type_text(one, target).map_err(|error| format!("ввод {one:?}: {error}"))?;

        expected.push(character);
        let landed = wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(content).filter(|text| text == &expected)
        });

        if landed.is_none() {
            return Err(format!(
                "клавиша {one:?} не дошла до приложения: прочитано {:?}, ожидалось {expected:?}",
                read_field(content).unwrap_or_else(|| "<чтение не удалось>".to_owned())
            ));
        }
    }

    Ok(())
}

/// Samples the flush counters with the machine at rest and prints the deltas — **hypothesis 1
/// of the task, asked of a running process**.
///
/// The four numbers are `window_flushes`, `full_clears`, `strokes_removed` and `focus_changes`.
/// The first three are the keys this task put on the channel; the fourth is what says whether a
/// flush came from a focus event at all. Nothing is typed and nothing is pressed while this
/// runs, so any growth is the environment's doing and not the bench's.
fn idle_watch(total: Duration, every: Duration) {
    let read = || -> Option<(u64, u64, u64, u64)> {
        let snapshot = crate::channel::read().ok()?;
        // A key the running product does not publish reads as zero here, and that is the honest
        // answer for a delta: it says «этот счётчик не двигался», which is what an absent key
        // means to a reader that cannot see it.
        let value = |key: &str| -> u64 {
            snapshot
                .get(key)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0)
        };
        Some((
            value("window_flushes"),
            value("full_clears"),
            value("strokes_removed"),
            value("focus_changes"),
        ))
    };

    let Some(first) = read() else {
        println!("  канал не ответил — покой не измерен");
        return;
    };
    println!(
        "  t=0с   window_flushes={} full_clears={} strokes_removed={} focus_changes={}",
        first.0, first.1, first.2, first.3
    );

    let started = Instant::now();
    while started.elapsed() < total {
        let step = Instant::now() + every;
        while Instant::now() < step {
            std::thread::sleep(wait::POLL);
        }

        match read() {
            None => println!("  канал не ответил"),
            Some(now) => println!(
                "  t={:<4} window_flushes={} (+{}) full_clears={} (+{}) strokes_removed={} (+{}) \
                 focus_changes={} (+{})",
                format!("{}с", started.elapsed().as_secs()),
                now.0,
                now.0 - first.0,
                now.1,
                now.1 - first.1,
                now.2,
                now.2 - first.2,
                now.3,
                now.3 - first.3,
            ),
        }
    }
}

/// A round at a person's tempo: a pause between the keystrokes and a pause before the hotkey.
///
/// The same instrument as [`press_once`] — the verdict is still `after != before` — and the
/// only difference is where the time goes. It exists because every measurement this bench has
/// ever made types a word in tens of milliseconds, and the window a flush has to land in is the
/// gap between the last stroke and the hotkey, which for a person is seconds.
fn slow_press(
    ctx: &Context,
    target: &input::Target,
    content: &Element,
    label: &str,
    source: u32,
) -> Result<PressOutcome, String> {
    /// The gap between two of a person's keystrokes, and before the hotkey. Two hundred
    /// milliseconds is an unhurried typist; the pause before the press is five times that,
    /// which is the hesitation the acceptance session described.
    const GAP: Duration = Duration::from_millis(200);

    layout::ensure(target.hwnd, source, Duration::from_secs(5))
        .map_err(|error| format!("исходная раскладка: {error}"))?;

    let (shown, wanted) = if source & 0xFFFF == layout::RUSSIAN & 0xFFFF {
        (EXPECTED, TYPED)
    } else {
        (TYPED, EXPECTED)
    };

    input::chord(&[VK_CONTROL.0], VK_A.0, target).map_err(|error| format!("Ctrl+A: {error}"))?;
    input::tap(VK_DELETE.0, target).map_err(|error| format!("Delete: {error}"))?;
    wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(String::is_empty)
    });
    println!("{}", watched(&format!("{label}: поле очищено")));

    let pause = |gap: Duration| {
        let until = Instant::now() + gap;
        while Instant::now() < until {
            std::thread::sleep(wait::POLL);
        }
    };

    for key in TYPED.chars() {
        let mut one = [0u8; 4];
        let one = key.encode_utf8(&mut one);
        input::type_text(one, target).map_err(|error| format!("ввод {one:?}: {error}"))?;
        pause(GAP);
    }

    let landed = read_field(content).unwrap_or_default();
    println!(
        "{}",
        watched(&format!("{label}: набрано в темпе человека ({landed:?})"))
    );

    // The hesitation itself — five gaps, a second, the interval the acceptance session named.
    pause(GAP * 5);
    println!(
        "{}",
        watched(&format!("{label}: пауза выдержана, до нажатия"))
    );

    let before = read_field(content).ok_or_else(|| "поле не читается перед нажатием".to_owned())?;
    if before != shown {
        println!(
            "  ⚠ поле читается {before:?}, а набиралось {shown:?} — часть штрихов не дошла до \
             приложения; это факт о приложении, вердикт ниже всё равно про изменение поля"
        );
    }

    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("горячая клавиша: {error}"))?;

    let after = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|text| text != &before)
    })
    .or_else(|| read_field(content))
    .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    println!("{}", watched(&format!("{label}: после нажатия")));

    Ok(PressOutcome {
        before,
        after,
        wanted: wanted.to_owned(),
    })
}

/// The layout a round runs the window in — **both sides, alternating**.
///
/// Criterion 12 of the task asks for the scenario «при EN раскладке окна и при RU», and
/// alternating is stronger than running one block of each: a defect that needs a *change* of
/// layout between rounds — which is the whole of defect A of the previous session, closed by
/// T-10-5 — is only reachable this way.
fn round_layout(round: usize) -> u32 {
    if round % 2 == 1 {
        layout::US
    } else {
        layout::RUSSIAN
    }
}

/// Finds the Notepad window of `app`, puts it through requirement B and returns what a round
/// needs: the send target and the element it reads back.
fn adopt_notepad(ctx: &Context, app: &mut App) -> Result<(input::Target, Element), String> {
    let window = adopt_window(ctx.automation, app, &|element: &Element| {
        element.class() == "Notepad"
    })?;
    let hwnd = app
        .window
        .ok_or_else(|| "у окна нет дескриптора".to_owned())?;
    let content = ctx
        .automation
        .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e))
        .ok_or_else(|| "элемент ввода не найден".to_owned())?;

    shell::activate_window(app.pid, Some(hwnd))?;
    layout::ensure(hwnd, layout::US, Duration::from_secs(5))?;

    Ok((input::Target { pid: app.pid, hwnd }, content))
}

/// One press of the hotkey on an **empty** typing buffer — the selection path of FR-60/FR-61.
///
/// It is not a verdict and cannot be one: with nothing typed and nothing selected the correct
/// behaviour is that nothing happens. What it is for is the *state* the path leaves behind — a
/// real `Ctrl+C` goes out, the clipboard is opened, saved and put back — and no round of
/// [`press_once`] can ever reach it, because `wants_selection_path` refuses a non-empty buffer
/// (Р-62). The channel is read on both sides of it.
fn empty_press(
    ctx: &Context,
    target: &input::Target,
    content: &Element,
    label: &str,
) -> Result<(), String> {
    input::chord(&[VK_CONTROL.0], VK_A.0, target).map_err(|error| format!("Ctrl+A: {error}"))?;
    input::tap(VK_DELETE.0, target).map_err(|error| format!("Delete: {error}"))?;
    wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(String::is_empty)
    });

    println!("{}", watched(&format!("{label}: буфер пуст, до нажатия")));
    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("горячая клавиша: {error}"))?;

    // A bound on the state settling, not on a result: there is no result to wait for.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        std::thread::sleep(wait::POLL);
    }

    println!("{}", watched(&format!("{label}: после нажатия")));
    println!(
        "  поле после нажатия на пустом буфере: {:?}",
        read_field(content).unwrap_or_else(|| "<чтение не удалось>".to_owned())
    );

    Ok(())
}

/// The product this experiment runs against — the debug copy, or the installed `uiAccess` one.
///
/// Two types and not one because they are ended by different mechanisms and only one of them can
/// be a child of this bench; see `sut::Installed` for the three measurements that make that so.
/// What the experiment needs of either is the same four questions, so they are asked through
/// this and the body below does not branch.
enum Product {
    /// The debug build, launched as a child of the bench.
    Debug(crate::sut::Sut),
    /// ⭐ **The installed, signed, `uiAccess` copy** — task T-10-10, and the configuration the
    /// defect was seen in. Raised by `ShellExecuteEx`; the bench does not father it.
    Installed(crate::sut::Installed),
}

impl Product {
    fn pid(&self) -> u32 {
        match self {
            Self::Debug(sut) => sut.pid,
            Self::Installed(installed) => installed.pid,
        }
    }

    fn await_ready(&self, timeout: Duration) -> Option<crate::channel::Snapshot> {
        match self {
            Self::Debug(sut) => sut.await_ready(timeout),
            Self::Installed(installed) => installed.await_ready(timeout),
        }
    }

    fn is_alive(&mut self) -> bool {
        match self {
            Self::Debug(sut) => sut.is_alive(),
            Self::Installed(installed) => installed.is_alive(),
        }
    }

    /// Ends the product through FR-96, reporting what was observed.
    fn stop(&mut self) -> Result<String, String> {
        match self {
            Self::Debug(sut) => sut.stop().map(|code| format!("код {code}")),
            Self::Installed(installed) => installed.stop().map(|()| "процесс ушёл".to_owned()),
        }
    }
}

/// **The experiment of task T-10-9** — «после открытия Проводника конвертация перестаёт
/// работать ВЕЗДЕ», staged with the channel open at every step.
///
/// # The shape
///
/// | Step | What it establishes |
/// |---|---|
/// | 0 | the **negative control of the instrument**: one round with no product running at all |
/// | 1 | conversion works — one round, before anything else is touched |
/// | 2 | `Win+E`, sent as a chord into the bench's own foreground window, the way the user sent it |
/// | 3 | back to the bench's own window, without touching the shell's |
/// | 4…13 | ten more rounds, each with the channel read before and after every press |
///
/// ⚠ **Step 0 is not decoration.** Criterion 11 of the task and the controller's own recorded
/// mistake are both about an instrument that reported success without executing what it was
/// measuring. An instrument that has never been seen to go red is an instrument nobody knows the
/// meaning of, so the run begins by showing it red on a case that is broken by construction —
/// no product, therefore no conversion, therefore `after == before`.
///
/// ⚠ **What this experiment does NOT do.** It does not touch the Explorer window it opened: the
/// shell is not in registry A, nothing is typed into it, nothing is posted to it and nothing is
/// terminated. `Win+E` is sent while **the bench's own window** holds the foreground, through
/// `input::chord`, which refuses to send at all unless that is true; going back afterwards is
/// `shell::activate_window` on the bench's own process. The Explorer window is left standing,
/// and the operator closes it — which is what requirement B costs here and it is cheaper than
/// weakening it.
pub fn experiment_explorer(
    ctx: &Context,
    rounds: usize,
    installed: bool,
) -> std::process::ExitCode {
    /// The FR-97 deadline this experiment runs the product under, in seconds.
    ///
    /// Named in the report, as the task requires. Ten paced rounds plus the shell opening a
    /// window do not fit into the forty-five seconds every position gets, and the band the task
    /// gives is 120 to 600. Two hundred and forty is inside it, and the product is still ended
    /// explicitly by pid at the end of the run — the deadline is a net, not a stopping mechanism.
    const DEADLINE_SECS: u32 = 240;

    println!("--- ОПЫТ T-10-9/T-10-10: Win+E и конвертация после него ---\n");
    println!(
        "Прибор: сравнение поля ДО и ПОСЛЕ нажатия. Совпадение с образцом {EXPECTED:?} \
         сообщается рядом и НИКОГДА не является вердиктом."
    );

    // ⛔ Debt 4 of the verdict on T-10-9, announced before anything is measured so that the
    // reader of the console knows where the unfiltered record is.
    protocol(&format!(
        "\n\n=========== ПРОГОН {} ===========\nустановленная сборка: {installed}, кругов после Проводника: {rounds}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    ));
    println!(
        "Полный протокол каждого снимка канала (все ключи, каждый шаг каждого круга): {}",
        protocol_path().display()
    );

    // ⚠ The synthetic FR-96 ends **any** copy in the session, so a second one is not a nuisance
    // but a hazard: the run would end a process it did not start, and could not tell which of the
    // two its own readings came from. Checked before anything is launched.
    let already = crate::sut::any_running();
    if already.is_empty() {
        println!("экземпляров продукта до запуска: ни одного\n");
    } else {
        eprintln!(
            "⛔ продукт уже запущен: {already:?}. Синтетическая FR-96 сняла бы чужой экземпляр, \
             а показания канала были бы неизвестно чьими. Опыт не ставится."
        );
        return std::process::ExitCode::from(1);
    }

    let _clipboard = clip::Guard::capture();

    // ---- step 0: the instrument, shown red on a case that cannot work ----------------------
    //
    // In a Notepad of its own, which is then closed. The product is **not** running yet, so the
    // round is broken by construction and the instrument has to say so.
    println!("=== ШАГ 0: отрицательный контроль прибора — продукт НЕ запущен ===");
    let control = (|| -> Result<PressOutcome, String> {
        let mut app = launch_notepad()?;
        let outcome = (|| -> Result<PressOutcome, String> {
            let (target, content) = adopt_notepad(ctx, &mut app)?;
            press_once(ctx, &target, &content, "контроль", layout::US)
        })();
        println!("  {}", app.close());
        outcome
    })();

    match control {
        Err(error) => {
            eprintln!("отрицательный контроль не поставлен: {error}");
            return std::process::ExitCode::from(1);
        }
        Ok(control) => {
            println!("  {}", control.describe());
            protocol(&format!(
                "ВЕРДИКТ отрицательный контроль (продукт не запущен): {}",
                control.describe()
            ));
            if control.changed() {
                eprintln!(
                    "⛔ ПРИБОР НЕИСПРАВЕН: без продукта поле изменилось после нажатия ({})",
                    control.describe()
                );
                return std::process::ExitCode::from(1);
            }
            println!("  ✔ прибор краснеет на заведомо сломанном случае — ему можно верить\n");
        }
    }

    // ---- the product, and only then the window it will work in ----------------------------
    //
    // ⚠ **The order is the point.** The first pass of this experiment launched the product
    // *after* the window already existed and already held the foreground, and the channel then
    // showed `focus_changes=0` for the whole of the pre-Explorer phase: the guard of FR-70/FR-71
    // never parked the buffer once, so nothing of that path was under test at all. The user's
    // product is started by autostart and every window they open is created under it. So the
    // product goes up first here, and the Notepad is created while it is watching.
    let launched = if installed {
        // ⭐ **Task T-10-10: the last remaining difference from the конфигурация the defect was
        // seen in, removed.** The copy in `%ProgramFiles%` is now built
        // `--release --features testing` and signed with the same certificate, so it carries the
        // Release manifest with `uiAccess="true"` **and** the channel of SEC-04a at once. The
        // manifest is chosen by `PROFILE` in `build.rs` and the channel by the feature; they are
        // independent, which is what makes this configuration reachable at all.
        //
        // It is raised by `ShellExecuteEx` and is **not** a child of this bench: `CreateProcess`
        // answers 740 on a `uiAccess` binary. FR-97 is absent from it — `debug_timeout` is behind
        // `#[cfg(debug_assertions)]` — so it lives the whole run and is ended by FR-96, exactly
        // like the копия in front of the user.
        println!(
            "⚠ ПРОДУКТ — УСТАНОВЛЕННЫЙ ПОДПИСАННЫЙ RELEASE из %ProgramFiles% (uiAccess=1),\n\
             собранный --release --features testing: манифест uiAccess И канал SEC-04a сразу.\n\
             Поднят ShellExecuteEx, стендом НЕ порождён. Таймаута FR-97 в нём нет.\n"
        );
        crate::sut::Installed::launch().map(Product::Installed)
    } else {
        println!("LANGSW_DEBUG_TIMEOUT_SEC = {DEADLINE_SECS}\n");
        crate::sut::Sut::launch_with(DEADLINE_SECS).map(Product::Debug)
    };

    let mut product = match launched {
        Ok(product) => product,
        Err(error) => {
            eprintln!("продукт не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let ready = match product.await_ready(Duration::from_secs(30)) {
        Some(ready) => ready,
        None => {
            eprintln!("продукт не сообщил о готовности за 30 с");
            return std::process::ExitCode::from(1);
        }
    };
    println!(
        "Продукт PID {}, hook_installed={}, ключей в снимке {}",
        product.pid(),
        ready.get("hook_installed").unwrap_or("?"),
        ready.present_keys().len()
    );
    println!(
        "Снимок канала целиком, сразу после готовности:\n{}",
        ready.raw
    );
    protocol(&format!(
        "[готовность] PID {}\n{}",
        product.pid(),
        ready.raw.trim_end()
    ));

    // ⚠ The second copy is checked again **after** the launch and not only before it: an
    // autostart entry or a copy raised by something else between the two lines would be caught
    // here, and the synthetic FR-96 at the end must have exactly one process to take down.
    let running = crate::sut::any_running();
    if running.len() == 1 && running[0] == product.pid() {
        println!("экземпляр продукта ровно один, и это наш: {running:?}\n");
    } else {
        println!(
            "⚠ экземпляров продукта {running:?}, наш {} — FR-96 в конце снимет любой из них\n",
            product.pid()
        );
    }

    let mut app = match launch_notepad() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Блокнот не запустился: {error}");
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    };

    let outcome = (|| -> Result<(), String> {
        let (target, content) = adopt_notepad(ctx, &mut app)?;
        let hwnd = target.hwnd;

        let result = (|| -> Result<(), String> {
            shell::activate_window(app.pid, Some(hwnd))?;
            layout::ensure(hwnd, layout::US, Duration::from_secs(5))?;
            println!("{}", watched("окно создано под работающим продуктом"));

            // ---- step 1: it works ------------------------------------------------------
            println!("\n=== ШАГ 1: конвертация работает (до Проводника) ===");
            let mut before_explorer = Vec::new();
            for round in 1..=2 {
                let source = round_layout(round);
                let outcome = press_once(ctx, &target, &content, &format!("до-{round}"), source)?;
                let line = format!(
                    "круг до-{round} ({}): {}",
                    layout::describe(source),
                    outcome.describe()
                );
                println!("  {line}");
                protocol(&format!("ВЕРДИКТ {line}"));
                before_explorer.push(outcome);
            }

            // ---- step 1b: a press on an **empty** buffer --------------------------------
            //
            // ⚠ The one gesture the rounds above can never make, and the user certainly made:
            // `wants_selection_path` hands the press to the selection path **only** when the
            // typing buffer is empty (Р-62), and that path sends a real `Ctrl+C` and opens the
            // clipboard. Nothing above this line ever reaches it, so nothing above this line
            // could leave whatever state it leaves.
            println!("\n=== ШАГ 1б: нажатие на ПУСТОМ буфере — путь выделения FR-60/FR-61 ===");
            empty_press(ctx, &target, &content, "пусто-до")?;

            // ---- step 2: Win+E ---------------------------------------------------------
            println!("\n=== ШАГ 2: Win+E — открываю Проводник ===");
            println!("{}", watched("перед Win+E"));
            for attempt in 1..=2 {
                // The chord goes out only while **our own** window holds the foreground —
                // `input::chord` refuses otherwise, and after the first Explorer window the
                // foreground is the shell's. So the window is brought back first, which is also
                // what the user does between two `Win+E` presses.
                shell::activate_window(app.pid, Some(hwnd))?;
                input::chord(&[VK_LWIN.0], VK_E.0, &target)
                    .map_err(|error| format!("Win+E: {error}"))?;

                let shell_up = wait::until(Duration::from_secs(20), || {
                    input::foreground().filter(|(pid, _)| *pid != app.pid)
                });
                match shell_up {
                    Some((pid, front)) => println!(
                        "  окно {attempt}: передний план ушёл процессу {pid} ({}), класс {:?}",
                        crate::own::process_table_name(pid)
                            .unwrap_or_else(|| "<имя неизвестно>".into()),
                        window_class_of(front)
                    ),
                    None => println!("  ⚠ окно {attempt}: передний план не сменился за 20 с"),
                }

                // ⚠ **A dwell and not a poll.** Everything else in this bench waits on a
                // condition (requirement 1 of §11.5), and this is deliberately not one: what is
                // being given time to happen is the shell's own settling — the churn of focus
                // events an opening folder window makes — and there is no condition on our side
                // that says it is over. It is an interval of the *scenario*, the way «отошёл на
                // некоторое время» is, not a wait for a result.
                let dwell = Duration::from_secs(4);
                let deadline = Instant::now() + dwell;
                while Instant::now() < deadline {
                    std::thread::sleep(wait::POLL);
                }
                println!(
                    "{}",
                    watched(&format!("Проводник {attempt}, после выдержки"))
                );
            }

            // ---- step 3: back to our own window ----------------------------------------
            println!("\n=== ШАГ 3: возвращаюсь в своё окно ===");
            shell::activate_window(app.pid, Some(hwnd))?;
            println!("{}", watched("своё окно снова впереди"));
            println!(
                "  раскладка окна сейчас: {}",
                layout::of_window(hwnd)
                    .map(layout::id_of)
                    .map_or("<не читается>".to_owned(), layout::describe)
            );
            layout::ensure(hwnd, layout::US, Duration::from_secs(5))?;

            // ---- step 3b: is the buffer being eaten while nobody touches anything? ------
            //
            // ⚠ **This is hypothesis 1 of the task, asked directly.** «Буфер пуст к моменту
            // нажатия — что-то сбрасывает его непрерывно». The three counters T-10-9 put on the
            // channel are the only way to ask it of a *running* process, and the question is
            // asked with the machine at rest: nothing is typed, nothing is clicked, the bench's
            // own window is in front and the Explorer windows are open behind it. A flush storm
            // shows as `window_flushes` climbing with nobody at the keyboard.
            println!(
                "\n=== ШАГ 3б: покой 20 с — растут ли сбросы, когда никто ничего не делает ==="
            );
            idle_watch(Duration::from_secs(20), Duration::from_secs(2));

            // ---- step 3в: a round at a person's tempo, not the bench's ------------------
            //
            // ⚠ Every round of this experiment so far has typed at the round-trip of UI
            // Automation — tens of milliseconds a character — and the acceptance session's own
            // diagnosis of the very first defect was that «окно уязвимости между набором и
            // горячей клавишей у него ничтожно, а у человека оно секунды». So one round is made
            // at a human tempo, with a real pause between the last stroke and the hotkey. It is
            // an interval of the **scenario** — the thing being staged is a person hesitating —
            // and not a wait for a result, which is what requirement 1 of §11.5 forbids.
            println!("\n=== ШАГ 3в: круг в темпе человека — пауза между набором и нажатием ===");
            let slow = slow_press(ctx, &target, &content, "медленно", layout::US)?;
            println!("  {}", slow.describe());
            protocol(&format!(
                "ВЕРДИКТ круг в темпе человека: {}",
                slow.describe()
            ));

            // ---- step 4: does it still work? -------------------------------------------
            println!("\n=== ШАГ 4: те же нажатия ПОСЛЕ Проводника, {rounds} кругов ===");
            let mut after_explorer = Vec::new();
            for round in 1..=rounds {
                let source = round_layout(round);
                let outcome =
                    press_once(ctx, &target, &content, &format!("после-{round}"), source)?;
                let line = format!(
                    "круг после-{round} ({}): {}",
                    layout::describe(source),
                    outcome.describe()
                );
                println!("  {line}");
                protocol(&format!("ВЕРДИКТ {line}"));
                after_explorer.push(outcome);
            }

            // ---- ⛔ the shell windows this experiment does NOT open ---------------------
            //
            // «Прочие окна оболочки: меню «Пуск», поиск панели задач» — measured and refused,
            // and the measurement is recorded here rather than the refusal alone. Tapping `Win`
            // from our own window **does** open the Start menu and the channel can be read while
            // it is up; what cannot be done is getting the foreground back. Measured on this
            // machine in the second pass of this experiment: `SearchHost.exe` (PID 4468,
            // `Windows.UI.Core.CoreWindow`) took the foreground and `shell::activate_window`
            // spent its full thirty seconds without recovering it — the same rake §11.3 position
            // 12 already records («меню не открывается вовсе»). The only way past it is to send a
            // key into the shell's own window, which this task's environment section forbids and
            // requirement B of the bench forbids. So the Start menu stays where §11.6 put it: a
            // person's position, and one the user closed `pass` in the very session this defect
            // appeared in.
            println!(
                "\n⛔ меню «Пуск» и поиск панели задач стенд не открывает — см. позицию 12 §11.3: \
                 окно принадлежит SearchHost/StartMenuExperienceHost, передний план обратно \
                 не отдаётся, а отправлять клавиши в чужое окно запрещено"
            );

            println!("\n=== ИТОГ ===");
            let changed_before = before_explorer.iter().filter(|o| o.changed()).count();
            let correct_before = before_explorer.iter().filter(|o| o.correct()).count();
            let changed_after = after_explorer.iter().filter(|o| o.changed()).count();
            let correct_after = after_explorer.iter().filter(|o| o.correct()).count();
            println!(
                "  до Проводника:    изменилось {changed_before}/{}, верно {correct_before}/{}",
                before_explorer.len(),
                before_explorer.len()
            );
            println!(
                "  после Проводника: изменилось {changed_after}/{}, верно {correct_after}/{}",
                after_explorer.len(),
                after_explorer.len()
            );
            println!("{}", watched("конец опыта"));

            Ok(())
        })();

        // ⛔ **«Продукт прожил весь опыт без перезапуска» — показано, а не заявлено.** The pid
        // is compared against the one recorded at launch and the liveness is asked of the launch
        // handle itself, not of the process table: a copy that had died and been replaced by an
        // autostart would carry a different id, and one that had merely been restarted would not
        // answer to this handle at all.
        let alive = product.is_alive();
        println!(
            "\nпродукт PID {} к концу опыта: {}",
            product.pid(),
            if alive {
                "ЖИВ, ни разу не перезапускался"
            } else {
                "⚠ УЖЕ НЕ ЖИВ — он ушёл посреди опыта, и это само по себе находка"
            }
        );
        protocol(&format!("[конец опыта] PID {} жив: {alive}", product.pid()));

        match product.stop() {
            Ok(how) => println!("продукт остановлен по FR-96, {how}"),
            Err(error) => println!("⚠ остановка продукта: {error}"),
        }

        let left = crate::sut::any_running();
        if left.is_empty() {
            println!("экземпляров продукта после опыта: ни одного");
        } else {
            println!("⚠ после опыта остались экземпляры продукта: {left:?}");
        }

        result
    })();

    let closed = app.close();
    println!("{closed}");
    println!("{}", restore_ambient(ctx));

    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nопыт не доведён: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

// ---------------------------------------------------------------------------------------
// Task T-10-11 — the человек's whole sequence, and the two paths of FR-42а alternating
// ---------------------------------------------------------------------------------------

/// The window class the console item of the sequence needs — FR-42а, `inject::resolve_auto`.
///
/// Not a class the bench invents: it is one of the two entries of
/// `inject::CONSOLE_WINDOW_CLASSES`, and the whole reason this item exists is that it is the only
/// way the bench can make the product resolve `auto` to `Backspace`.
const CONSOLE_CLASS: &str = "ConsoleWindowClass";

/// ⭐ **A real console window the bench may drive, without weakening a single requirement.**
///
/// # The measurement this is built on, and the three routes it rules out
///
/// Measured on this machine before anything was written, because the task's own claim — «позиции
/// 1, 4, 5 и 8 стенд уже умеет водить» — turned out not to hold for the consoles:
///
/// | Launched directly | Window class | Window owned by | Owner's parent |
/// |---|---|---|---|
/// | `cmd.exe` | `CASCADIA_HOSTING_WINDOW_CLASS` | `WindowsTerminal.exe` | **`svchost.exe`** |
/// | `powershell.exe` | `CASCADIA_HOSTING_WINDOW_CLASS` | `WindowsTerminal.exe` | **`svchost.exe`** |
/// | `conhost.exe cmd.exe` | `ConsoleWindowClass` | `cmd.exe` | the spawned `conhost.exe` |
///
/// The first two are worse than §11.3 positions 6–7 record: that measurement found
/// `WindowsTerminal.exe` **descending** from the bench's `wt.exe`, so only requirement C refused.
/// Today the default-terminal handoff raises it by COM activation under `svchost.exe`, so it is
/// not kin to the bench at all and requirement **A** refuses first. The third gives the right
/// window class but hands the window to a `cmd.exe` — a name on the protected list of requirement
/// C which was not spawned **directly**, and `own::claim_window_process` refuses a protected name
/// before it walks any ancestry.
///
/// ⛔ **Requirement C is not relaxed here, and that is deliberate.** The documentation of position
/// 6 says in as many words that whether C should be relaxed for a proved descendant «is a question
/// for the controller … not a decision the bench takes for itself», and the verdict on T-10-10
/// singled out that A–E were left unweakened. So the question stays open and this goes round it.
///
/// # What is launched instead
///
/// `conhost.exe` — spawned **directly**, so requirement C is satisfied for it by
/// [`launched`]'s `register_spawned` — hosting **this bench's own binary** in the mode
/// [`crate::console_park`] parks in. The console window that results is a real conhost window of
/// class `ConsoleWindowClass`, and it is owned by `langsw-e2e.exe`: a name that is **not** on the
/// protected list, descending from a process the bench spawned directly, which is exactly the
/// case `claim_window_process` adopts.
///
/// The hosted program does nothing but read lines, which is what puts the console in **cooked
/// line-input mode with echo** — the input mode FR-42а's `Backspace` path exists for, and the one
/// the acceptance session confirmed positions 6–7 in.
fn launch_console() -> Result<App, String> {
    let bench = std::env::current_exe()
        .map_err(|error| format!("собственный путь стенда не читается: {error}"))?;

    // ⚠ **`ShellExecuteEx` and not `Command::spawn`** — three failing variants measured first; see
    // `sut::shell_execute` for the table and the cause. In short: `std`'s `Command` always passes
    // explicit standard handles, and a console **host** handed somebody else's standard streams
    // does not go on to create the console it exists to create.
    let conhost = std::path::PathBuf::from(r"C:\Windows\System32\conhost.exe");
    let pid =
        crate::sut::shell_execute(&conhost, &format!("\"{}\" --console-park", bench.display()))?;

    // ⛔ Requirement A: the id came from the handle of the launch itself, and it enters the
    // registry here, at that instant — before any window exists to be found by.
    crate::own::register_spawned(pid);

    // `Terminate` and not `WmClose`: the program the console hosts is the bench's own and holds
    // nothing a kill could damage — the same reasoning `launch_notepad` gives. `child` is `None`
    // because there is no `Child` to hold: `ShellExecuteEx` gives a handle, not a process object,
    // and `App` tracks liveness by pid.
    Ok(App {
        name: "Консоль (conhost + langsw-e2e --console-park)".to_owned(),
        pid,
        child: None,
        window: None,
        close: CloseWith::Terminate,
        scratch: None,
    })
}

/// Finds the console window, puts it through requirement B and reports what the run needs.
///
/// ⚠ The predicate is the class alone, which other consoles on the machine also carry — the
/// operator's own shells among them. That is safe and is why [`adopt_window`] scans candidates
/// instead of taking the first: every window that is not ours is refused by
/// `own::claim_window_process`, with the refusal recorded, and the loop moves on. The owner's name
/// is asserted afterwards so that an adoption which somehow matched the wrong console is caught
/// rather than measured.
fn adopt_console(ctx: &Context, app: &mut App) -> Result<(input::Target, Element), String> {
    let window = adopt_window(ctx.automation, app, &|element: &Element| {
        element.class() == CONSOLE_CLASS
    })?;

    // ⭐ **Measured, and better than the design expected.** The window turns out to belong to the
    // `conhost.exe` the bench spawned **directly**, not to the program it hosts — so requirement C
    // is satisfied outright rather than by descent: `was_spawned` is true for this very pid, which
    // is exactly the exemption C names. (A `conhost` hosting `cmd.exe` hands the window to the
    // `cmd.exe` instead, which is the case C refuses and the reason that route was abandoned.)
    // Either owner is ours; a third name means the predicate matched something else and the item
    // does not run.
    let owner = crate::own::process_table_name(app.pid).unwrap_or_else(|| "<неизвестно>".into());
    let bare = owner.to_ascii_lowercase();
    if !(bare.starts_with("conhost") || bare.starts_with("langsw-e2e")) {
        return Err(format!(
            "окном консоли владеет {owner:?} (pid {}) — ни conhost, запущенный стендом напрямую, \
             ни его программа; опыт над чужой консолью не ставится",
            app.pid
        ));
    }
    println!(
        "  консоль усыновлена: владелец окна {owner:?}, pid {}",
        app.pid
    );

    let hwnd = app
        .window
        .ok_or_else(|| "у окна консоли нет дескриптора".to_owned())?;

    // ⚠ **A console window has two process ids, and they are different.** UI Automation reports the
    // window as `conhost.exe`'s; `GetWindowThreadProcessId` reports it as the **attached console
    // application's** — the program conhost hosts. Everything below this line goes through Win32 —
    // `SetForegroundWindow`, the foreground guard inside `input::type_text`, `input::Target` — so
    // it is the Win32 answer that has to be used, and the first pass measured what happens
    // otherwise: «окно процесса 1252 не удалось вывести вперёд за 30 с; передний план принадлежит
    // процессу 5464 (langsw-e2e.exe)» — the window was already in front, under the other id.
    //
    // ⛔ Requirement B is satisfied for that id **explicitly** and not by inheritance: it is put
    // through `claim_window_process` in its own right, which admits it because `langsw-e2e` is not
    // a protected name and it descends from the `conhost.exe` this bench spawned directly.
    if let Some(win32_pid) = window_process_of(hwnd)
        && win32_pid != app.pid
    {
        crate::own::claim_window_process(win32_pid)?;
        println!(
            "  окно консоли по Win32 принадлежит процессу {win32_pid} ({}) — усыновлён отдельно",
            crate::own::process_table_name(win32_pid).unwrap_or_else(|| "<неизвестно>".into())
        );
        app.pid = win32_pid;
    }

    // ⚠ **`TextPattern` on the screen, and not whatever answers first.** The first pass took the
    // window element because `read()` on it returned `Some` — and what it returned was the window
    // *title*, `C:\Windows\System32\conhost.exe`, which never changes however much is typed. An
    // instrument whose reading cannot move is worse than one that fails: it reports «не
    // изменилось» for everything. So the screen is looked for by the pattern that actually holds
    // it, and the window itself is the last resort rather than the first.
    let screen = ctx
        .automation
        .find(&window, &|e: &Element| e.text().is_some())
        .or_else(|| window.text().map(|_| window.clone()));

    let Some(content) = screen else {
        // What the tree did offer, so the refusal is diagnosable from the protocol.
        println!("  ⚠ ни один узел консоли не отдал TextPattern. Что видно под окном:");
        for element in ctx.automation.find_all(&window, &|_: &Element| true) {
            println!("    {}", element.describe());
        }
        return Err("экран консоли не читается через TextPattern".to_owned());
    };

    shell::activate_window(app.pid, Some(hwnd))?;
    println!(
        "  раскладка окна консоли: {}",
        layout::of_window(hwnd)
            .map(layout::id_of)
            .map_or("<не читается>".to_owned(), layout::describe)
    );

    Ok((input::Target { pid: app.pid, hwnd }, content))
}

/// One round in the console — **the same instrument, a different way of clearing the line**.
///
/// The verdict is [`PressOutcome::changed`] exactly as everywhere else. What differs is the
/// housekeeping either side of it:
///
/// * the line is cleared with `Escape`, which is what the console's own line editor does with it —
///   `Ctrl+A`/`Delete` mean nothing here and [`press_once`]'s clearing would be a no-op that then
///   compared two identical screens;
/// * the reading is the whole visible screen, so «набранное дошло» is asked as *contains* rather
///   than as equality, and the reported text is the last non-empty line of the screen.
fn console_press(
    ctx: &Context,
    target: &input::Target,
    content: &Element,
    label: &str,
    source: u32,
) -> Result<PressOutcome, String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

    // ⚠ **Asked, then read — never assumed.** A console window does not take the layout the way an
    // ordinary window does: the first pass spent the whole five seconds of `layout::ensure` and the
    // window stayed in RU. That refusal is a fact about consoles, not a failure of the round, so it
    // is reported and the round goes on **in whatever layout the window is really in** — which is
    // the only thing that decides what the six keys read back as. Assuming the requested layout
    // here would be the controller's trap all over again, one level down.
    let asked = layout::ensure(target.hwnd, source, Duration::from_secs(5));
    let actual = layout::of_window(target.hwnd).map(layout::id_of);
    if let Err(error) = &asked {
        println!(
            "  ⚠ {label}: консоль не приняла раскладку — {error}. Круг идёт в той, что есть: {}",
            actual.map_or("<не читается>".to_owned(), layout::describe)
        );
    }

    let russian = actual.map(|id| id & 0xFFFF) == Some(layout::RUSSIAN & 0xFFFF);
    let (shown, wanted) = if russian {
        (EXPECTED, TYPED)
    } else {
        (TYPED, EXPECTED)
    };

    input::tap(VK_ESCAPE.0, target).map_err(|error| format!("Escape: {error}"))?;
    let cleared = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|screen| !screen.contains(shown) && !screen.contains(wanted))
    });
    if cleared.is_none() {
        println!("  ⚠ {label}: строка консоли не очистилась Escape — круг всё равно ставится");
    }
    println!("{}", watched(&format!("{label}: строка очищена")));

    input::type_text(TYPED, target).map_err(|error| format!("ввод {TYPED:?}: {error}"))?;
    let landed = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|screen| screen.contains(shown))
    });
    if landed.is_none() {
        return Err(format!(
            "набранное не дошло до консоли: экран {:?}",
            console_line(&read_field(content).unwrap_or_default())
        ));
    }

    let before =
        read_field(content).ok_or_else(|| "экран не читается перед нажатием".to_owned())?;
    println!("{}", watched(&format!("{label}: набрано, до нажатия")));

    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("горячая клавиша: {error}"))?;

    let after = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|screen| screen != &before)
    })
    .or_else(|| read_field(content))
    .unwrap_or_else(|| "<чтение не удалось>".to_owned());

    println!("{}", watched(&format!("{label}: после нажатия")));

    Ok(PressOutcome {
        before: console_line(&before),
        after: console_line(&after),
        wanted: wanted.to_owned(),
    })
}

/// The last non-empty run of characters on a console screen — what a person sees on the line
/// they are typing.
///
/// The whole screen is what `TextPattern` gives and what the verdict is decided on; this is for
/// the protocol, so that a report line is a line rather than eighty spaces and a word.
fn console_line(screen: &str) -> String {
    screen
        .split_whitespace()
        .next_back()
        .unwrap_or("")
        .to_owned()
}

/// The class name of a window, for the protocol of [`experiment_explorer`].
///
/// ⚠ Reading a class name is not «трогать окно»: it copies a string out of the window class the
/// system already published and changes nothing. It is here because the whole question of
/// hypothesis 5 is which class the resolver of FR-42а sees when the shell is in front, and a
/// protocol that only said «передний план сменился» could not answer it.
fn window_class_of(hwnd: HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;

    let mut buffer = [0u16; 257];
    // SAFETY: `buffer` is a live local array and the call is given the slice itself, so the
    // binding derives the bound from the array and cannot write past it. NFR-13: a non-positive
    // return is the documented failure and is examined.
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) };

    if length <= 0 {
        return "<класс не читается>".to_owned();
    }

    String::from_utf16_lossy(&buffer[..length as usize])
}

/// The process id Win32 reports for a window — **not always the one UI Automation reports**.
///
/// They differ for exactly the case the console item of task T-10-11 needs: UI Automation names
/// `conhost.exe`, Win32 names the console application attached to it. Everything that sends input
/// or moves the foreground is Win32, so it is this answer those need.
fn window_process_of(hwnd: HWND) -> Option<u32> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut pid = 0u32;
    // SAFETY: `hwnd` came from UI Automation and may since have died, in which case the call
    // returns zero — which is examined (NFR-13). `pid` is a live local and the only thing written.
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&raw mut pid)) };

    if thread == 0 || pid == 0 {
        None
    } else {
        Some(pid)
    }
}

/// One number of SEC-04a, or `None` when the channel did not answer or the key is absent.
fn channel_number(key: &str) -> Option<u64> {
    crate::channel::read()
        .ok()?
        .get(key)
        .and_then(|value| value.parse().ok())
}

/// What the sequence counts as it goes — **criterion 10 asks for the run to be shown in numbers**.
///
/// Not a verdict and not a判断: a tally. `selection` and `backspace` are read off
/// `last_replacement_method` after every press, which is the register FR-42а writes per press, so
/// the two counts together are «сколько замен каким путём» measured rather than assumed.
#[derive(Default)]
struct Tally {
    presses: usize,
    selection: usize,
    backspace: usize,
    none: usize,
}

impl Tally {
    /// Reads the method of the press that has just happened and adds it in.
    fn note_press(&mut self) {
        self.presses += 1;
        match crate::channel::read()
            .ok()
            .and_then(|snapshot| snapshot.get("last_replacement_method").map(str::to_owned))
            .as_deref()
        {
            Some("selection") => self.selection += 1,
            Some("backspace") => self.backspace += 1,
            _ => self.none += 1,
        }
    }

    fn describe(&self) -> String {
        format!(
            "нажатий {}, из них последняя замена читалась как selection {}, backspace {}, \
             none/не прочитано {}",
            self.presses, self.selection, self.backspace, self.none
        )
    }
}

/// Item 2 of the sequence — **нажатие → `Enter` → набор → нажатие**, in Notepad.
///
/// The gesture the acceptance session closed defect T-10-5 with, and the one thing no round of
/// [`press_once`] can reach: `Enter` is the boundary key of FR-10, so it flushes the typing buffer
/// between the two presses, and the second press therefore starts from an empty buffer in a field
/// that is **not** empty — which is what hands it to the selection path of FR-60/FR-61 (Р-62) with
/// a real `Ctrl+C` and a real clipboard round trip.
fn item_enter(
    ctx: &Context,
    target: &input::Target,
    content: &Element,
    tally: &mut Tally,
) -> Result<Vec<PressOutcome>, String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;

    layout::ensure(target.hwnd, layout::US, Duration::from_secs(5))
        .map_err(|error| format!("исходная раскладка: {error}"))?;

    input::chord(&[VK_CONTROL.0], VK_A.0, target).map_err(|error| format!("Ctrl+A: {error}"))?;
    input::tap(VK_DELETE.0, target).map_err(|error| format!("Delete: {error}"))?;
    wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(String::is_empty)
    });
    println!("{}", watched("пункт 2: поле очищено"));

    let mut outcomes = Vec::new();

    // ---- press one, on what was just typed ------------------------------------------------
    type_paced(TYPED, target, content, "")?;
    let before = read_field(content).unwrap_or_default();
    println!("{}", watched("пункт 2: набрано, до нажатия 1"));

    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("нажатие 1: {error}"))?;
    let after = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|text| text != &before)
    })
    .or_else(|| read_field(content))
    .unwrap_or_default();
    tally.note_press();
    println!("{}", watched("пункт 2: после нажатия 1"));
    outcomes.push(PressOutcome {
        before,
        after: after.clone(),
        wanted: EXPECTED.to_owned(),
    });

    // ---- `Enter`, the boundary key of FR-10 -------------------------------------------------
    //
    // ⛔ In **Notepad**, and only ever in Notepad. The prohibition on `Enter` in this task is about
    // Telegram, where it sends a message; a newline in the bench's own throwaway Notepad document
    // is the gesture the acceptance session actually made.
    input::tap(VK_RETURN.0, target).map_err(|error| format!("Enter: {error}"))?;
    println!("{}", watched("пункт 2: Enter отправлен (в СВОЙ Блокнот)"));

    // ---- type again, and press again --------------------------------------------------------
    //
    // ⚠ **The layout is read, never assumed — the controller's trap from the typing side.** Step 5
    // of FR-40 leaves the window in RU after a successful replacement, so the six keys `ghbdtn`
    // now read back as `привет` **while they are being typed**, with no conversion involved. The
    // first pass of this item asserted `"привет g"` against a field reading `"привет п"` and
    // reported the item as not executed. What a person does here is keep typing whatever the
    // window is in, so that is what is staged, and what the field is expected to show is derived
    // from the layout as it actually stands.
    let window_layout = layout::of_window(target.hwnd).map(layout::id_of);
    println!(
        "  раскладка окна после нажатия 1: {}",
        window_layout.map_or("<не читается>".to_owned(), layout::describe)
    );

    // ⚠ And put it back to US before typing again — the same restoration `press_once` makes at the
    // start of **every** round, and for a reason stronger than tidiness. Under RU the six keys read
    // back as `привет` already, so a *correct* second press would write `привет` over `привет` and
    // the field would not change: the instrument would report «не изменилось» for a product that
    // did exactly the right thing. The verdict of this bench is `after != before`, and a state in
    // which a success is indistinguishable from a failure is a state the instrument must not be
    // measured in.
    layout::ensure(target.hwnd, layout::US, Duration::from_secs(5))
        .map_err(|error| format!("возврат раскладки перед вторым набором: {error}"))?;

    // `normalise` turns the newline into a space, so the field now reads «<первое> <второе>».
    let prefix = format!("{} ", read_field(content).unwrap_or_default());
    type_paced_as(TYPED, TYPED, target, content, &prefix)?;

    let before = read_field(content).unwrap_or_default();
    println!("{}", watched("пункт 2: набрано после Enter, до нажатия 2"));

    input::tap(ctx.hotkey_vk, target).map_err(|error| format!("нажатие 2: {error}"))?;
    let after = wait::until(SERIES_STEP_TIMEOUT, || {
        read_field(content).filter(|text| text != &before)
    })
    .or_else(|| read_field(content))
    .unwrap_or_default();
    tally.note_press();
    println!("{}", watched("пункт 2: после нажатия 2"));
    outcomes.push(PressOutcome {
        before,
        after,
        // The first word as press 1 left it, and the second converted by press 2. Reported beside
        // the verdict and never as it — `PressOutcome::changed` is the verdict here as everywhere.
        wanted: format!("{EXPECTED} {EXPECTED}"),
    });

    Ok(outcomes)
}

/// **The experiment of task T-10-11** — the человек's whole sequence on one instance of the
/// product, and `Win+E` only after all of it.
///
/// # What is new here, and it is one thing
///
/// T-10-9 and T-10-10 both staged the user's *trigger* — `Win+E` — and neither staged the user's
/// *sequence*. The acceptance session went through five items in four applications before it
/// pressed `Win+E`, and the two paths of FR-42а alternated across them: `Selection` for Блокнот
/// and Telegram, `Backspace` for the two consoles. Nothing until now has made **one instance of
/// the product run both packet builders and a clipboard round trip in each of the first items**
/// and only then opened the shell.
///
/// | Item | What | The path FR-42а resolves |
/// |---|---|---|
/// | 1 | Блокнот: слово, пробел, второе слово, **четыре нажатия подряд** | `Selection` |
/// | 2 | Блокнот: нажатие → `Enter` → набор → нажатие | `Selection` |
/// | 3 | Telegram, «Избранное» | `Selection` — see the refusal recorded at run time |
/// | 4–5 | консоль (`cmd`/PowerShell) | `Backspace` |
/// | 6 | `Win+E`, Проводник, обратно в своё окно | — |
/// | 7–8 | проверка ≥10 раз, обе раскладки через один | `Selection` |
///
/// ⚠ **Item 1 is `series` — the position-16 detector, reused and not rewritten.** The numbering of
/// §11.3 is untouched and no row of the matrix is added; what the item does is call the assertions
/// that already exist and print them.
///
/// ⛔ **`Enter` is sent in Notepad and nowhere else**, and Telegram is never driven by this mode.
pub fn experiment_sequence(
    ctx: &Context,
    rounds: usize,
    installed: bool,
) -> std::process::ExitCode {
    /// The FR-97 deadline the **debug** build runs under here. The installed diagnostic build has
    /// no such deadline at all, which is why the sequence can be as long as the user's was.
    const DEADLINE_SECS: u32 = 600;

    println!("--- ОПЫТ T-10-11: ПОСЛЕДОВАТЕЛЬНОСТЬ человека, и Win+E только после неё ---\n");
    println!(
        "Прибор: сравнение поля ДО и ПОСЛЕ нажатия. Совпадение с образцом {EXPECTED:?} \
         сообщается рядом и НИКОГДА не является вердиктом."
    );

    protocol(&format!(
        "\n\n=========== ПОСЛЕДОВАТЕЛЬНОСТЬ T-10-11 {} ===========\n\
         установленная сборка: {installed}, кругов проверки после Проводника: {rounds}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    ));
    println!(
        "Полный протокол каждого снимка канала (все ключи, каждый шаг каждого пункта): {}",
        protocol_path().display()
    );

    let already = crate::sut::any_running();
    if already.is_empty() {
        println!("экземпляров продукта до запуска: ни одного\n");
    } else {
        eprintln!(
            "⛔ продукт уже запущен: {already:?}. Синтетическая FR-96 сняла бы чужой экземпляр, \
             а показания канала были бы неизвестно чьими. Опыт не ставится."
        );
        return std::process::ExitCode::from(1);
    }

    let _clipboard = clip::Guard::capture();

    // ---- step 0: the instrument, shown red ------------------------------------------------
    println!("=== ШАГ 0: отрицательный контроль прибора — продукт НЕ запущен ===");
    let control = (|| -> Result<PressOutcome, String> {
        let mut app = launch_notepad()?;
        let outcome = (|| -> Result<PressOutcome, String> {
            let (target, content) = adopt_notepad(ctx, &mut app)?;
            press_once(ctx, &target, &content, "контроль", layout::US)
        })();
        println!("  {}", app.close());
        outcome
    })();

    match control {
        Err(error) => {
            eprintln!("отрицательный контроль не поставлен: {error}");
            return std::process::ExitCode::from(1);
        }
        Ok(control) => {
            println!("  {}", control.describe());
            protocol(&format!(
                "ВЕРДИКТ отрицательный контроль (продукт не запущен): {}",
                control.describe()
            ));
            if control.changed() {
                eprintln!(
                    "⛔ ПРИБОР НЕИСПРАВЕН: без продукта поле изменилось после нажатия ({})",
                    control.describe()
                );
                return std::process::ExitCode::from(1);
            }
            println!("  ✔ прибор краснеет на заведомо сломанном случае — ему можно верить\n");
        }
    }

    // ---- the product, first, so every window below is created under it ---------------------
    let launched_product = if installed {
        println!(
            "⚠ ПРОДУКТ — УСТАНОВЛЕННЫЙ ПОДПИСАННЫЙ RELEASE из %ProgramFiles% (uiAccess=1),\n\
             собранный --release --features testing: манифест uiAccess И канал SEC-04a сразу.\n\
             Поднят ShellExecuteEx, стендом НЕ порождён. Таймаута FR-97 в нём нет.\n"
        );
        crate::sut::Installed::launch().map(Product::Installed)
    } else {
        println!("LANGSW_DEBUG_TIMEOUT_SEC = {DEADLINE_SECS}\n");
        crate::sut::Sut::launch_with(DEADLINE_SECS).map(Product::Debug)
    };

    let mut product = match launched_product {
        Ok(product) => product,
        Err(error) => {
            eprintln!("продукт не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let ready = match product.await_ready(Duration::from_secs(30)) {
        Some(ready) => ready,
        None => {
            eprintln!("продукт не сообщил о готовности за 30 с");
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    };
    println!(
        "Продукт PID {}, hook_installed={}, ключей в снимке {}",
        product.pid(),
        ready.get("hook_installed").unwrap_or("?"),
        ready.present_keys().len()
    );
    println!(
        "Снимок канала целиком, сразу после готовности:\n{}",
        ready.raw
    );
    protocol(&format!(
        "[готовность] PID {}\n{}",
        product.pid(),
        ready.raw.trim_end()
    ));

    let running = crate::sut::any_running();
    if running.len() == 1 && running[0] == product.pid() {
        println!("экземпляр продукта ровно один, и это наш: {running:?}\n");
    } else {
        println!(
            "⚠ экземпляров продукта {running:?}, наш {} — FR-96 в конце снимет любой из них\n",
            product.pid()
        );
    }

    let mut tally = Tally::default();
    let mut skipped: Vec<String> = Vec::new();
    let outcome = run_sequence(ctx, rounds, &mut tally, &mut skipped);

    // ⛔ «Продукт прожил весь опыт без перезапуска» — shown, not asserted.
    let alive = product.is_alive();
    println!(
        "\nпродукт PID {} к концу опыта: {}",
        product.pid(),
        if alive {
            "ЖИВ, ни разу не перезапускался"
        } else {
            "⚠ УЖЕ НЕ ЖИВ — он ушёл посреди опыта, и это само по себе находка"
        }
    );
    protocol(&format!("[конец опыта] PID {} жив: {alive}", product.pid()));

    println!("\n=== ЧТО ИСПОЛНЕНО, В ЧИСЛАХ (критерий 10) ===");
    println!("  {}", tally.describe());
    for key in [
        "focus_changes",
        "password_probes",
        "hotkey_handoffs",
        "clipboard_refusals",
        "clipboard_close_failures",
        "clipboard_retries",
        "window_flushes",
        "full_clears",
        "strokes_removed",
        "send_mismatches",
        "post_failures",
        "events_lost",
    ] {
        println!(
            "  {key} = {}",
            channel_number(key).map_or("<нет>".to_owned(), |value| value.to_string())
        );
    }

    if skipped.is_empty() {
        println!("\n⚠ пунктов, которые не удалось исполнить: НЕТ");
    } else {
        println!("\n⛔ ПУНКТЫ, КОТОРЫЕ НЕ УДАЛОСЬ ИСПОЛНИТЬ — названы, а не пропущены:");
        for line in &skipped {
            println!("  • {line}");
            protocol(&format!("НЕ ИСПОЛНЕНО: {line}"));
        }
    }

    match product.stop() {
        Ok(how) => println!("\nпродукт остановлен по FR-96, {how}"),
        Err(error) => println!("\n⚠ остановка продукта: {error}"),
    }
    let left = crate::sut::any_running();
    if left.is_empty() {
        println!("экземпляров продукта после опыта: ни одного");
    } else {
        println!("⚠ после опыта остались экземпляры продукта: {left:?}");
    }

    println!("{}", restore_ambient(ctx));

    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("\nопыт не доведён: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

/// The six items, in the order the acceptance session made them — split out so that the product
/// is stopped and the numbers printed on **every** path out, including a refusal in the middle.
fn run_sequence(
    ctx: &Context,
    rounds: usize,
    tally: &mut Tally,
    skipped: &mut Vec<String>,
) -> Result<(), String> {
    let mut notepad = launch_notepad()?;

    let result = (|| -> Result<(), String> {
        let (target, content) = adopt_notepad(ctx, &mut notepad)?;
        let hwnd = target.hwnd;
        println!("{}", watched("Блокнот создан под работающим продуктом"));

        // ---- item 1: слово, пробел, второе слово, четыре нажатия подряд ------------------
        println!("\n=== ПУНКТ 1: Блокнот — слово, пробел, второе слово, ЧЕТЫРЕ нажатия ===");
        input::chord(&[VK_CONTROL.0], VK_A.0, &target)
            .map_err(|error| format!("Ctrl+A: {error}"))?;
        input::tap(VK_DELETE.0, &target).map_err(|error| format!("Delete: {error}"))?;
        wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(&content).filter(String::is_empty)
        });
        type_paced(TYPED, &target, &content, "")?;
        println!("{}", watched("пункт 1: первое слово набрано"));

        // ⭐ The detector of T-10-6 itself, called and not copied — position 16's assertions.
        let rows = series(
            ctx,
            &target,
            16,
            "Блокнот (пункт 1 последовательности)",
            &content,
            TYPED,
        );
        for row in &rows {
            let line = format!(
                "{:?} [{:?}] получено {} — ожидалось {}",
                row.assertion, row.verdict, row.actual, row.expected
            );
            println!("  {line}");
            protocol(&format!("ПУНКТ 1 {line}"));
        }
        // Four presses of the series, plus nothing else: the tally reads the register once per
        // press, and the series does not expose its presses, so they are counted here as four.
        for _ in 0..4 {
            tally.note_press();
        }
        println!("{}", watched("пункт 1: серия окончена"));

        // ---- item 2: нажатие → Enter → набор → нажатие -----------------------------------
        println!("\n=== ПУНКТ 2: Блокнот — нажатие → Enter → набор → нажатие ===");
        match item_enter(ctx, &target, &content, tally) {
            Ok(outcomes) => {
                for (index, outcome) in outcomes.iter().enumerate() {
                    let line = format!("пункт 2, нажатие {}: {}", index + 1, outcome.describe());
                    println!("  {line}");
                    protocol(&format!("ВЕРДИКТ {line}"));
                }
            }
            Err(error) => {
                println!("  ⚠ пункт 2 не доведён: {error}");
                skipped.push(format!("пункт 2 (Блокнот, Enter): {error}"));
            }
        }

        // ---- item 3: Telegram ------------------------------------------------------------
        println!("\n=== ПУНКТ 3: Telegram, «Избранное» ===");
        let reason = telegram_refusal();
        println!("  ⛔ НЕ ИСПОЛНЕН: {reason}");
        skipped.push(format!("пункт 3 (Telegram, «Избранное»): {reason}"));

        // ---- items 4 and 5: the console --------------------------------------------------
        println!("\n=== ПУНКТЫ 4 и 5: консоль — путь Backspace по FR-42а ===");
        let mut console = match launch_console() {
            Ok(app) => Some(app),
            Err(error) => {
                println!("  ⚠ консоль не запустилась: {error}");
                skipped.push(format!("пункты 4–5 (консоль): запуск не удался: {error}"));
                None
            }
        };
        // Remembered **before** adoption, because adoption moves `App::pid` to the process Win32
        // names for the window — the program conhost hosts. `App::close` then puts that one away,
        // and this is the launcher it leaves standing. Requirement C admits it by name: this very
        // pid was spawned directly.
        let conhost_pid = console.as_ref().map(|app| app.pid);

        if let Some(app) = console.as_mut() {
            match adopt_console(ctx, app) {
                Err(error) => {
                    println!("  ⛔ консоль не усыновлена: {error}");
                    // What the bench actually saw, so that a refusal is diagnosable from the
                    // protocol instead of from a second run: every top-level window of the class
                    // the item needs, with the process behind it.
                    let seen = ctx
                        .automation
                        .top_level_of_any(&|e: &Element| e.class() == CONSOLE_CLASS);
                    println!(
                        "  окон класса {CONSOLE_CLASS} видно всего: {}; процесс консоли {} жив: {}",
                        seen.len(),
                        app.pid,
                        shell::process_is_alive(app.pid)
                    );
                    for window in &seen {
                        println!("    {}", window.describe());
                    }
                    skipped.push(format!("пункты 4–5 (консоль): {error}"));
                }
                Ok((console_target, console_content)) => {
                    println!(
                        "  консоль: pid {}, класс окна {:?} — FR-42а обязан выбрать backspace",
                        app.pid,
                        window_class_of(console_target.hwnd)
                    );
                    println!("{}", watched("пункты 4–5: консоль впереди"));

                    for round in 1..=4usize {
                        let source = round_layout(round);
                        match console_press(
                            ctx,
                            &console_target,
                            &console_content,
                            &format!("консоль-{round}"),
                            source,
                        ) {
                            Ok(outcome) => {
                                tally.note_press();
                                let line = format!(
                                    "консоль, круг {round} ({}): {}",
                                    layout::describe(source),
                                    outcome.describe()
                                );
                                println!("  {line}");
                                protocol(&format!("ВЕРДИКТ {line}"));
                            }
                            Err(error) => {
                                println!("  ⚠ консоль, круг {round}: {error}");
                                skipped.push(format!("пункты 4–5, круг {round} консоли: {error}"));
                                break;
                            }
                        }
                    }

                    println!(
                        "  ⚠ cmd и PowerShell продукт РАЗЛИЧИТЬ НЕ МОЖЕТ: обе оболочки живут за \
                         одним классом окна, и FR-42а выбирает по классу (inject.rs, \
                         CONSOLE_WINDOW_CLASSES). Один консольный экземпляр закрывает ровно то, \
                         что продукт различает."
                    );
                }
            }
        }

        if let Some(mut app) = console {
            println!("  {}", app.close());
        }
        // The `conhost.exe` itself, when the program it hosted was a different process and it has
        // not gone with it. Reported either way, so the run says what it left behind.
        if let Some(pid) = conhost_pid {
            if shell::process_is_alive(pid) {
                match shell::terminate(pid) {
                    Ok(()) => println!("  conhost {pid}: снят"),
                    Err(error) => println!("  ⚠ conhost {pid} снять не удалось: {error}"),
                }
            } else {
                println!("  conhost {pid}: ушёл вместе со своей программой");
            }
        }

        // Back to the Notepad, the way the user came back to their own window.
        shell::activate_window(notepad.pid, Some(hwnd))?;
        println!("{}", watched("возврат в Блокнот после консоли"));

        // ---- item 6: Win+E ---------------------------------------------------------------
        println!("\n=== ПУНКТ 6: Win+E — Проводник, ПОСЛЕ всей последовательности ===");
        println!("{}", watched("перед Win+E"));
        for attempt in 1..=2 {
            shell::activate_window(notepad.pid, Some(hwnd))?;
            input::chord(&[VK_LWIN.0], VK_E.0, &target)
                .map_err(|error| format!("Win+E: {error}"))?;

            let shell_up = wait::until(Duration::from_secs(20), || {
                input::foreground().filter(|(pid, _)| *pid != notepad.pid)
            });
            match shell_up {
                Some((pid, front)) => println!(
                    "  окно {attempt}: передний план ушёл процессу {pid} ({}), класс {:?}",
                    crate::own::process_table_name(pid)
                        .unwrap_or_else(|| "<имя неизвестно>".into()),
                    window_class_of(front)
                ),
                None => println!("  ⚠ окно {attempt}: передний план не сменился за 20 с"),
            }

            // A dwell of the scenario, not a wait for a result — see `experiment_explorer`.
            let deadline = Instant::now() + Duration::from_secs(4);
            while Instant::now() < deadline {
                std::thread::sleep(wait::POLL);
            }
            println!(
                "{}",
                watched(&format!("Проводник {attempt}, после выдержки"))
            );
        }

        println!("\n=== ПУНКТ 6б: возвращаюсь в своё окно ===");
        shell::activate_window(notepad.pid, Some(hwnd))?;
        println!("{}", watched("своё окно снова впереди"));
        println!(
            "  раскладка окна сейчас: {}",
            layout::of_window(hwnd)
                .map(layout::id_of)
                .map_or("<не читается>".to_owned(), layout::describe)
        );
        layout::ensure(hwnd, layout::US, Duration::from_secs(5))?;

        // ---- items 7 and 8: is conversion alive? ------------------------------------------
        println!("\n=== ПУНКТЫ 7 и 8: проверка после Проводника, {rounds} кругов ===");
        let mut after = Vec::new();
        for round in 1..=rounds {
            let source = round_layout(round);
            let outcome = press_once(ctx, &target, &content, &format!("после-{round}"), source)?;
            tally.note_press();
            let line = format!(
                "круг после-{round} ({}): {}",
                layout::describe(source),
                outcome.describe()
            );
            println!("  {line}");
            protocol(&format!("ВЕРДИКТ {line}"));
            after.push(outcome);
        }

        println!("\n=== ИТОГ ===");
        let changed = after.iter().filter(|o| o.changed()).count();
        let correct = after.iter().filter(|o| o.correct()).count();
        println!(
            "  после Проводника: изменилось {changed}/{}, верно {correct}/{}",
            after.len(),
            after.len()
        );
        println!("{}", watched("конец опыта"));

        Ok(())
    })();

    println!("{}", notepad.close());
    result
}

/// Why item 3 of the sequence is not executed — **named, not skipped** (criterion 11).
///
/// Two independent reasons, and both are measurements rather than opinions:
///
/// 1. **§11.3 position 8 already measured that the bench cannot drive Telegram.** The window is
///    found, «Избранное» is opened and confirmed by title, the message box is found and told apart
///    from the search box — and then `ghbdtn` typed through `SendInput` reads back as `""` from
///    `ValuePattern`, **including after a `SetFocus` that reports success**. That is the case
///    footnote 7 of §11.3 names in advance («у Qt-приложений поддержка неполная … позиция
///    переводится в П»), and it is why position 8 is `pending` and was closed by a **person**.
/// 2. **Telegram was not running when this task took its baseline**, so driving it would mean
///    starting the operator's messenger as well.
///
/// ⛔ The one route past the first reason — clicking into the field with a synthetic mouse — is
/// refused for the reason position 8 gives: the item would then be measuring the bench's aim. And
/// `Enter` is never sent to Telegram under any circumstance.
fn telegram_refusal() -> String {
    let installed = std::env::var("APPDATA")
        .map(|appdata| {
            std::path::Path::new(&appdata)
                .join("Telegram Desktop")
                .join("Telegram.exe")
                .exists()
        })
        .unwrap_or(false);

    format!(
        "стенд Telegram водить не может — измерено позицией 8 §11.3: окно и «Избранное» \
         находятся, но набранное через SendInput до Qt-поля не доходит, ValuePattern возвращает \
         пустую строку и после успешного SetFocus (сноска 7 §11.3). Telegram.exe на машине \
         {}; на момент снятия базы он НЕ был запущен. Обход мышью отвергнут — он мерил бы \
         прицел стенда. ⛔ Enter в Telegram не отправлялся и отправлен быть не мог",
        if installed {
            "установлен"
        } else {
            "не найден"
        }
    )
}

// ---------------------------------------------------------------------------------------
// ⭐ Опыт T-10-14 — штамп раскладки против настоящей раскладки, БЕЗ сна
// ---------------------------------------------------------------------------------------

/// How long the two values are given to converge on their own after a switch.
///
/// Not a delay the measurement depends on: [`wait::until`] ends at the first poll that finds
/// them equal, so this is only the bound on «they never did». Three seconds is two orders of
/// magnitude longer than any message the product answers on.
const STAMP_CONVERGENCE_BOUND: Duration = Duration::from_secs(3);

/// One `(настоящая раскладка, штамп продукта)` pair, taken at one instant.
///
/// ⚠ **Two real readings, around the channel read.** The product reads the layout of the
/// foreground window's thread and so does this; the channel read between them costs a
/// millisecond or two, and a pair taken as «real, stamp» alone could not tell a stale stamp
/// from a layout that moved while the channel was being read. The second real reading closes
/// that: a sample whose two real readings differ is unusable and is never counted as a
/// divergence.
#[derive(Debug, Clone, Copy)]
struct StampSample {
    real_before: Option<u32>,
    stamp: Option<u32>,
    real_after: Option<u32>,
    probes: Option<u64>,
    focus: Option<u64>,
}

impl StampSample {
    /// Reads one pair. The order is real, stamp, real.
    fn take(hwnd: HWND) -> Self {
        let real_before = layout::of_window(hwnd).map(layout::id_of);
        let snapshot = crate::channel::read().ok();
        let real_after = layout::of_window(hwnd).map(layout::id_of);

        let count = |key: &str| -> Option<u64> {
            snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get(key))
                .and_then(|value| value.parse::<u64>().ok())
        };

        Self {
            stamp: snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get("active_layout"))
                .and_then(parse_hkl),
            probes: count("layout_probes"),
            focus: count("focus_changes"),
            real_before,
            real_after,
        }
    }

    /// The real layout, when both readings around the channel agree — otherwise `None`.
    fn real(&self) -> Option<u32> {
        match (self.real_before, self.real_after) {
            (Some(first), Some(second)) if first == second => Some(first),
            _ => None,
        }
    }

    /// `Some(true)` — the pair diverges; `Some(false)` — it agrees; `None` — unusable.
    fn diverged(&self) -> Option<bool> {
        Some(self.real()? & 0xFFFF != self.stamp? & 0xFFFF)
    }

    fn describe(&self) -> String {
        format!(
            "настоящая={} штамп={} layout_probes={} focus_changes={} — {}",
            self.real()
                .map_or_else(|| "<менялась при чтении>".to_owned(), layout::describe),
            self.stamp
                .map_or_else(|| "<нет>".to_owned(), layout::describe),
            self.probes
                .map_or_else(|| "?".to_owned(), |v| v.to_string()),
            self.focus.map_or_else(|| "?".to_owned(), |v| v.to_string()),
            match self.diverged() {
                Some(true) => "⛔ РАСХОДЯТСЯ",
                Some(false) => "совпадают",
                None => "нечитаемо",
            }
        )
    }
}

/// `0x04190419`, as the channel writes it.
fn parse_hkl(value: &str) -> Option<u32> {
    u32::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()
}

/// The other of the two layouts of this session.
fn other_layout(id: u32) -> u32 {
    if id & 0xFFFF == layout::RUSSIAN & 0xFFFF {
        layout::US
    } else {
        layout::RUSSIAN
    }
}

/// What the field reads back for the six keys of [`TYPED`] under `id`.
fn shown_under(id: u32) -> &'static str {
    if id & 0xFFFF == layout::RUSSIAN & 0xFFFF {
        EXPECTED
    } else {
        TYPED
    }
}

/// Title of the bench's **own** window for this experiment.
const STAMP_WINDOW_TITLE: &str = "LangSw-Stamp-14";

/// ⛔ **The window of this experiment is the bench's own** — the shape position 15 already uses,
/// and for the reason written down there: on this machine a `Notepad.exe` the bench did not
/// start is running, the System32 `notepad.exe` is a stub whose process exits as soon as the
/// packaged application takes over, and a window opened by that stub is created **inside the
/// stranger's process**. Requirement B refuses it, correctly, and the experiment would be
/// measuring the machine's window ownership instead of the product.
///
/// Measured, not assumed: the first run of this experiment refused the window of process 15788,
/// «*привет — Блокнот», which is the controller's own live session and is not this task's to
/// close.
///
/// One plain multiline text box, nothing masked, nothing saved anywhere. `TopMost` so that the
/// foreground guard of [`input::send_verified`] has a stable answer.
fn launch_stamp_window() -> Result<App, String> {
    let scratch = scratch_dir("stamp");
    let script = scratch.join("stamp-window.ps1");

    let body = format!(
        "Add-Type -AssemblyName System.Windows.Forms\n\
         $form = New-Object System.Windows.Forms.Form\n\
         $form.Text = '{STAMP_WINDOW_TITLE}'\n\
         $form.Width = 640\n\
         $form.Height = 260\n\
         $form.StartPosition = 'CenterScreen'\n\
         $form.TopMost = $true\n\
         $box = New-Object System.Windows.Forms.TextBox\n\
         $box.Multiline = $true\n\
         $box.Left = 20\n\
         $box.Top = 30\n\
         $box.Width = 580\n\
         $box.Height = 150\n\
         $box.TabIndex = 0\n\
         $form.Controls.Add($box)\n\
         $form.Add_Shown({{ $form.Activate(); [void]$box.Focus() }})\n\
         [void]$form.ShowDialog()\n"
    );

    std::fs::write(&script, body)
        .map_err(|error| format!("не удалось записать {}: {error}", script.display()))?;

    let child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-STA",
            "-WindowStyle",
            "Hidden",
            "-File",
        ])
        .arg(&script)
        .spawn()
        .map_err(|error| format!("запуск окна опыта T-10-14: {error}"))?;

    Ok(launched(
        "Штамп раскладки (окно стенда)",
        child,
        CloseWith::WmClose,
        Some(scratch),
    ))
}

/// Adopts the window of [`launch_stamp_window`], brings it forward and puts it into US.
fn adopt_stamp_window(ctx: &Context, app: &mut App) -> Result<(input::Target, Element), String> {
    let window = adopt_window(ctx.automation, app, &|element: &Element| {
        element.name().starts_with(STAMP_WINDOW_TITLE)
    })?;
    let hwnd = app
        .window
        .ok_or_else(|| "у окна нет дескриптора".to_owned())?;
    let content = ctx
        .automation
        .await_element(&window, wait::WINDOW_TIMEOUT, &|e: &Element| text_field(e))
        .ok_or_else(|| "элемент ввода не найден".to_owned())?;

    shell::activate_window(app.pid, Some(hwnd))?;
    layout::ensure(hwnd, layout::US, Duration::from_secs(5))?;

    Ok((input::Target { pid: app.pid, hwnd }, content))
}

/// Empties the field **without pressing a single modifier** — `Backspace` and nothing else.
///
/// ⭐ Not tidiness, and not a stylistic preference. `Ctrl+A` ends in a `Ctrl` release, and
/// `hook::is_layout_probe` fires the layout probe of T-03-3c on **every modifier release**; a
/// field cleared that way refreshes the stamp and destroys the state the round is creating.
/// The first run of this experiment cleared with `Ctrl+A`, `Delete` and watched `layout_probes`
/// climb by one and the stamp catch up on every round. `Backspace` is in no combination at all
/// and is the one key of the FR-10 table that neither flushes nor probes — it pops one stroke,
/// which is exactly the bookkeeping the deletion deserves.
///
/// ⚠ **`VK_BACK` as a key, never `'\u{8}'` through [`input::type_text`].** That function has no
/// virtual key for `\u{8}` and falls back to `KEYEVENTF_UNICODE`, which puts the value in
/// `wScan` with `wVk = 0` — and scan code `0x08` is the **`7` key**. The product's hook decodes
/// exactly that: the second run of this experiment filled the field with `77777777` on every
/// press, eight sevens for eight "backspaces", because the buffer had faithfully recorded eight
/// presses of the `7` key. The bench's own instrument, poisoning its own measurement.
fn clear_field(target: &input::Target, content: &Element) -> Result<(), String> {
    for _ in 0..2 {
        let Some(text) = read_field(content) else {
            return Err("поле не читается при очистке".to_owned());
        };
        if text.is_empty() {
            return Ok(());
        }

        // One `Backspace` per character, plus two of margin: the count is what the field says it
        // holds, and extra presses on an empty field do nothing.
        for _ in 0..text.chars().count() + 2 {
            input::tap(VK_BACK.0, target).map_err(|error| format!("очистка поля: {error}"))?;
        }

        wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(content).filter(String::is_empty)
        });
    }

    match read_field(content) {
        Some(text) if text.is_empty() => Ok(()),
        Some(text) => Err(format!("поле не очистилось, в нём {text:?}")),
        None => Err("поле не читается при очистке".to_owned()),
    }
}

/// How a round switches the layout of the window that already has the focus.
///
/// Two forms, alternated, because they answer two different questions and the task asks for
/// both: whether the household gesture reproduces the defect, and whether the *mechanism* named
/// in the diagnosis — an event that outruns the switch — is really what does it.
#[derive(Debug, Clone, Copy)]
enum Stimulus {
    /// **The household gesture**: a synthetic `Alt+Shift`, the switcher of this session, sent
    /// into the window the bench is typing in. Windows performs the switch after the callback
    /// that saw the release has returned; the product's probe rides that same release.
    AltShift,
    /// **The mechanism, made deterministic**: `WM_INPUTLANGCHANGEREQUEST` is *posted* and not
    /// waited for, and a bare `Shift` tap is sent immediately behind it. The target thread has
    /// not pumped the request yet, so the probe the `Shift` release fires reads the layout that
    /// is on its way out — the event has outrun the switch, by construction and not by luck.
    RequestThenRelease,
}

/// The rotation of [`Stimulus`] the rounds walk through.
const STIMULI: [Stimulus; 2] = [Stimulus::AltShift, Stimulus::RequestThenRelease];

impl Stimulus {
    fn name(self) -> &'static str {
        match self {
            Self::AltShift => "синтетический Alt+Shift",
            Self::RequestThenRelease => "WM_INPUTLANGCHANGEREQUEST без ожидания + отпускание Shift",
        }
    }

    /// Asks the window to move to `to`. Returns as soon as the request is on its way — the
    /// caller waits on the condition, never on this.
    fn fire(self, target: &input::Target, to: u32) -> Result<(), String> {
        match self {
            Self::AltShift => {
                println!("  стимул: Alt+Shift в переднее окно, фокус не трогали");
                input::chord(&[VK_MENU.0], VK_SHIFT.0, target)
                    .map_err(|error| format!("Alt+Shift: {error}"))
            }
            Self::RequestThenRelease => {
                let Some(wanted) = layout::handle_for(to) else {
                    return Err(format!(
                        "раскладка {} не подключена в этом сеансе",
                        layout::describe(to)
                    ));
                };

                // SAFETY: `target.hwnd` is the window of a process in registry A, adopted by
                // `adopt_stamp_window`. `PostMessageW` copies the message into that thread's
                // queue and dereferences nothing of ours; `wanted` is a layout handle taken from
                // the system's own list. NFR-13: the result is examined below.
                let posted = unsafe {
                    PostMessageW(
                        Some(target.hwnd),
                        WM_INPUTLANGCHANGEREQUEST,
                        WPARAM(0),
                        LPARAM(wanted.0 as isize),
                    )
                };
                posted
                    .map_err(|error| format!("PostMessage(WM_INPUTLANGCHANGEREQUEST): {error}"))?;

                println!(
                    "  стимул: WM_INPUTLANGCHANGEREQUEST → {} ПОСЛАНО и не дождано; \
                     сразу за ним отпускание Shift",
                    layout::describe(to)
                );

                // ⚠ Nothing is waited for between the post and this tap. The request is sitting
                // in the target thread's queue; the `Shift` release goes through the hook first
                // because `SendInput` reaches the low-level hook chain before the target thread
                // is next scheduled to pump.
                input::tap(VK_SHIFT.0, target).map_err(|error| format!("Shift: {error}"))
            }
        }
    }
}

/// `WM_INPUTLANGCHANGEREQUEST` — the same constant `layout::ensure` uses.
const WM_INPUTLANGCHANGEREQUEST: u32 = 0x0050;

/// Waits until the product's stamp agrees with the real layout of `hwnd`, and says what it saw.
fn await_agreement(hwnd: HWND) -> (bool, StampSample) {
    let agreed = wait::until(STAMP_CONVERGENCE_BOUND, || {
        let sample = StampSample::take(hwnd);
        (sample.diverged() == Some(false)).then_some(sample)
    });

    match agreed {
        Some(sample) => (true, sample),
        None => (false, StampSample::take(hwnd)),
    }
}

/// ⭐ **Task T-10-14** — the divergence between the layout stamp and the real layout, produced
/// **without putting the machine to sleep**, and the press that follows it.
///
/// # What is being reproduced
///
/// The defect is not in sleeping. The stamp of FR-04 is read on an *event* —
/// `watchdog::WM_APP_LAYOUT`, posted from a focus or foreground change — and used much later,
/// at the moment strokes are recorded. A switch the event does not cover, or covers too early,
/// leaves the stamp naming a layout the user is no longer typing in; from then on FR-26 takes
/// its direction from that value **and** `Recorder::lookup` decodes the strokes through that
/// layout's map, so the product converts what it believes was typed into what is already on the
/// screen. The user sees nothing happen.
///
/// # The stimulus, and why it needs no sleep
///
/// `WM_INPUTLANGCHANGEREQUEST` to the window that already has the focus — `layout::ensure`, the
/// bench's existing precondition tool. Nothing is activated, no window is created, no focus
/// moves, the machine is not suspended. It is the case `app::refresh_layout_and_cache` writes
/// down as its own known limit in as many words: «a user who switches layout with `Alt+Shift`
/// **without leaving the window they are typing in** changes no foreground and moves no focus».
///
/// # ⚠ The instrument
///
/// The verdict of every press is `after != before` — [`PressOutcome::changed`] — and never a
/// comparison with `привет`, because under the Russian layout the six keys of `ghbdtn` already
/// read `привет` while they are being typed. The sample is printed alongside as a description
/// and never as the test. That trap is what the controller reported «работает» on.
pub fn experiment_stamp(ctx: &Context, rounds: usize) -> std::process::ExitCode {
    println!("--- ОПЫТ T-10-14: штамп раскладки против настоящей, БЕЗ сна ---\n");
    println!(
        "Прибор: сравнение поля ДО и ПОСЛЕ нажатия. Совпадение с образцом {EXPECTED:?} \
         сообщается рядом и НИКОГДА не является вердиктом.\n\
         Стимул: WM_INPUTLANGCHANGEREQUEST в окно, которое УЖЕ переднее — ни фокус, ни переднее \
         окно не меняются. ⛔ Машина не усыпляется и усыплена быть не может: sleep во всём стенде \
         один, это интервал опроса wait::POLL.\n"
    );

    let already = crate::sut::any_running();
    if !already.is_empty() {
        eprintln!(
            "⛔ продукт уже запущен: {already:?}. Показания канала были бы неизвестно чьими. \
             Опыт не ставится."
        );
        return std::process::ExitCode::from(1);
    }

    let _clipboard = clip::Guard::capture();

    let mut product = match crate::sut::Sut::launch_with(600) {
        Ok(product) => product,
        Err(error) => {
            eprintln!("продукт не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };
    let Some(ready) = product.await_ready(Duration::from_secs(30)) else {
        eprintln!("продукт не сообщил о готовности через канал SEC-04a за 30 с");
        let _ = product.stop();
        return std::process::ExitCode::from(1);
    };
    println!(
        "Продукт PID {}, hook_installed={}\n",
        product.pid,
        ready.get("hook_installed").unwrap_or("?")
    );

    let mut field = match launch_stamp_window() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("окно стенда не открылось: {error}");
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    };

    let outcome = (|| -> Result<(usize, usize), String> {
        let (target, content) = adopt_stamp_window(ctx, &mut field)?;

        // ---- the instrument, shown working on a case that must succeed --------------------
        println!("=== ПЛЕЧО A: штамп свеж — положительный контроль прибора ===");
        let (agreed, sample) = await_agreement(target.hwnd);
        println!("  {}", sample.describe());
        if !agreed {
            return Err(format!(
                "штамп не сошёлся с настоящей раскладкой за {} с ещё до всякого стимула — \
                 плечо A недостоверно",
                STAMP_CONVERGENCE_BOUND.as_secs()
            ));
        }

        let fresh = press_once(ctx, &target, &content, "A", layout::US)?;
        println!("  {}", fresh.describe());
        if !fresh.changed() {
            return Err(
                "плечо A не изменило текст — прибор не годится, всё ниже недостоверно".to_owned(),
            );
        }

        let mut stale_rounds = 0usize;
        let mut unchanged_rounds = 0usize;

        for round in 1..=rounds {
            let stimulus = STIMULI[(round - 1) % STIMULI.len()];
            println!(
                "\n=== ПЛЕЧО B, круг {round}: стимул «{}» ===",
                stimulus.name()
            );

            // Step 1 — empty the field, **without a single modifier**. ⭐ This is not tidiness.
            // `Ctrl+A` ends in a `Ctrl` release, and a modifier release is exactly what
            // `hook::is_layout_probe` fires the probe of T-03-3c on; clearing the field that way
            // would refresh the stamp and destroy the very state the round is trying to create.
            // Measured: the first run of this experiment cleared with `Ctrl+A`, `Delete` and
            // watched `layout_probes` climb and the stamp catch up every single time.
            clear_field(&target, &content)?;

            // Step 2 — **the healthy precondition, built and verified, not assumed.** The round
            // always starts from en-US and always switches into ru-RU, because the direction is
            // not symmetric: measured over two runs, the probe of T-03-3c wins the race against
            // an activation of en-US every time and loses it against ru-RU every time. A round
            // that took whatever layout the previous press left behind measured the direction,
            // not the defect.
            //
            // The `Shift` tap is the setup and is deliberate: it is the probe of T-03-3c itself,
            // fired **after** the switch has already settled, which is exactly the case in which
            // it is supposed to work. It is what makes «до» a healthy state rather than a hope.
            let from = layout::ensure(target.hwnd, layout::US, Duration::from_secs(5))?;
            input::tap(VK_SHIFT.0, &target)
                .map_err(|error| format!("Shift подготовки: {error}"))?;
            let (started_agreed, start) = await_agreement(target.hwnd);
            println!(
                "  до переключения (сошлись: {started_agreed}): {}",
                start.describe()
            );
            if !started_agreed {
                println!("  ⚠ штамп не сошёлся ещё до стимула — круг пропущен");
                continue;
            }

            // Step 3 — the stimulus. Both forms switch the layout of the window that already has
            // the focus: no foreground change, no focus move, nothing suspended.
            let to = other_layout(from);
            stimulus.fire(&target, to)?;

            let settled = wait::until(Duration::from_secs(5), || {
                layout::of_window(target.hwnd)
                    .map(layout::id_of)
                    .filter(|id| id & 0xFFFF == to & 0xFFFF)
            });
            let Some(settled) = settled else {
                println!(
                    "  ⚠ окно не перешло в {} за 5 с — круг пропущен",
                    layout::describe(to)
                );
                continue;
            };
            println!(
                "  окно перешло {} → {}",
                layout::describe(from),
                layout::describe(settled)
            );

            // Step 4 — do the two converge on their own? The wait is on the condition, and
            // «не сошлись за три секунды» is the finding.
            let (converged, after_switch) = await_agreement(target.hwnd);
            println!("  после переключения: {}", after_switch.describe());
            println!(
                "  штамп догнал настоящую раскладку сам: {}",
                if converged { "да" } else { "⛔ НЕТ" }
            );
            if !converged {
                stale_rounds += 1;
            }

            // Step 5 — type under the layout the window really has, and press. The six keys are
            // the same whichever layout it is; what changes is what the field shows for them.
            // ⚠ Not one of them is a modifier, so nothing between here and the hotkey can fire
            // the probe — which is precisely the household case: a person switches the layout
            // and types a word.
            let shown = shown_under(settled);

            // ⚠ Read once more **immediately before the first key**: the defect is about what
            // the stamp said when the strokes were recorded, not a second earlier.
            let at_typing = StampSample::take(target.hwnd);
            println!("  в момент набора: {}", at_typing.describe());

            type_paced_as(TYPED, shown, &target, &content, "")?;

            let before =
                read_field(&content).ok_or_else(|| "поле не читается перед нажатием".to_owned())?;
            println!("{}", watched(&format!("круг {round}: набрано, до нажатия")));

            input::tap(ctx.hotkey_vk, &target)
                .map_err(|error| format!("горячая клавиша: {error}"))?;

            let after = wait::until(SERIES_STEP_TIMEOUT, || {
                read_field(&content).filter(|text| text != &before)
            })
            .or_else(|| read_field(&content))
            .unwrap_or_else(|| "<чтение не удалось>".to_owned());

            let press = PressOutcome {
                before,
                after,
                wanted: shown_under(other_layout(settled)).to_owned(),
            };
            println!("{}", watched(&format!("круг {round}: после нажатия")));
            println!(
                "  ⭐ круг {round}: набирали под {}, штамп говорил {} — {}",
                layout::describe(settled),
                at_typing
                    .stamp
                    .map_or_else(|| "<нет>".to_owned(), layout::describe),
                press.describe()
            );

            if !press.changed() {
                unchanged_rounds += 1;
            }
        }

        println!("\n=== ИТОГ ===");
        println!("  кругов: {rounds}, стимулы чередуются: {:?}", STIMULI);
        println!("  штамп сам не догнал раскладку: {stale_rounds}/{rounds}");
        println!("  ⛔ нажатие НЕ изменило текст: {unchanged_rounds}/{rounds}");

        Ok((stale_rounds, unchanged_rounds))
    })();

    println!("{}", field.close());
    match product.stop() {
        Ok(code) => println!("продукт остановлен, код {code}"),
        Err(error) => println!("⚠ {error}"),
    }

    match outcome {
        // The exit code **is** the detector: green when every press changed the field, red when
        // any press did nothing at all.
        Ok((_, 0)) => {
            println!("\nВЕРДИКТ: все нажатия изменили текст — ЗЕЛЁНЫЙ");
            std::process::ExitCode::SUCCESS
        }
        Ok((_, unchanged)) => {
            println!("\nВЕРДИКТ: нажатий, ничего не изменивших: {unchanged} — ⛔ КРАСНЫЙ");
            std::process::ExitCode::from(1)
        }
        Err(reason) => {
            eprintln!("\nопыт не доведён: {reason}");
            std::process::ExitCode::from(2)
        }
    }
}

// ---------------------------------------------------------------------------------------
// Опыт T-10-14 — задержка callback, критерий 12
// ---------------------------------------------------------------------------------------

/// ⭐ **Task T-10-14** — `callback_p50_ns` and `callback_p99_ns` under a volley of the shape
/// that actually exercises the repair.
///
/// # Why this is not position 23
///
/// It is position 23's measurement, in position 23's terms, on a window of the bench's own —
/// and it has to be, twice over.
///
/// 1. Position 23 drives Notepad, and on this machine a `Notepad.exe` the bench did not start
///    is running (§2.1 of the report of this task), so requirement B refuses the window.
/// 2. ⭐ **Position 23 types `ghbdtn` with no boundary key at all**, so the ring never empties
///    after the first stroke and the repair's branch — the first stroke of a **new word** —
///    runs exactly once in ten thousand presses. Its percentiles therefore cannot say what
///    that branch costs; they can only say it is not on the common path, which is worth
///    knowing and is not what criterion 12 asks.
///
/// `--words` types `ghbdtn ` instead: the space is a boundary key of FR-10, the ring is
/// emptied, and the next letter is a first stroke of a new word. One press in seven, that is
/// **one callback sample in fourteen** — comfortably inside the p99 the criterion is about.
///
/// The histogram of `hook::profile` is global and is never reset, so one run measures one
/// shape. The mode is meant to be run twice, once each way, against a fresh product.
pub fn experiment_latency(ctx: &Context, presses: usize, words: bool) -> std::process::ExitCode {
    /// Presses spent before the first reading, so one-time costs are paid outside it.
    const WARMUP: usize = 300;
    /// Presses per `SendInput` call — position 23's number, and for its reason.
    const BATCH: usize = 300;

    let chunk_text = if words {
        format!("{TYPED} ")
    } else {
        TYPED.to_owned()
    };

    println!("--- ОПЫТ T-10-14: задержка callback (критерий 12) ---\n");
    println!(
        "Форма залпа: {chunk_text:?} — {}\n",
        if words {
            "с границей слова, ветка починки работает на каждом седьмом нажатии"
        } else {
            "без границ слова, как в позиции 23: ветка починки срабатывает один раз за залп"
        }
    );

    let already = crate::sut::any_running();
    if !already.is_empty() {
        eprintln!("⛔ продукт уже запущен: {already:?}. Опыт не ставится.");
        return std::process::ExitCode::from(1);
    }

    let _clipboard = clip::Guard::capture();

    let mut product = match crate::sut::Sut::launch_with(600) {
        Ok(product) => product,
        Err(error) => {
            eprintln!("продукт не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };
    if product.await_ready(Duration::from_secs(30)).is_none() {
        eprintln!("продукт не сообщил о готовности за 30 с");
        let _ = product.stop();
        return std::process::ExitCode::from(1);
    }
    println!("Продукт PID {}\n", product.pid);

    let mut field = match launch_stamp_window() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("окно стенда не открылось: {error}");
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    };

    let outcome = (|| -> Result<(), String> {
        let (target, _content) = adopt_stamp_window(ctx, &mut field)?;

        let warm = chunk_text.repeat(WARMUP / chunk_text.chars().count());
        let (before_warmup, ..) = callback_reading()?;
        input::type_text(&warm, &target).map_err(|error| format!("разминка: {error}"))?;
        let warm_target = before_warmup + 2 * warm.chars().count() as u64;
        if wait::until(Duration::from_secs(20), || {
            callback_reading()
                .ok()
                .filter(|(samples, ..)| *samples >= warm_target)
        })
        .is_none()
        {
            return Err("разминка не дошла до callback за 20 с".to_owned());
        }

        let (samples_before, p50_before, p99_before, max_before) = callback_reading()?;
        println!(
            "  до залпа:  выборка {samples_before}, p50 {p50_before} нс, p99 {p99_before} нс, \
             максимум {max_before} нс"
        );

        let batch = chunk_text.repeat(BATCH / chunk_text.chars().count());
        let batch_len = batch.chars().count();
        let batches = presses.div_ceil(batch_len);
        let started = Instant::now();
        for _ in 0..batches {
            input::type_text(&batch, &target).map_err(|error| format!("залп: {error}"))?;
        }
        let injection = started.elapsed();

        // Requirement 1 of §11.5: the wait is on «выборка накрыла залп», never on a clock.
        let volley_target = samples_before + 2 * (batches * batch_len) as u64;
        let Some((samples, p50, p99, max)) = wait::until(Duration::from_secs(40), || {
            callback_reading()
                .ok()
                .filter(|(samples, ..)| *samples >= volley_target)
        }) else {
            return Err(format!(
                "выборка не набралась: нужно ≥ {volley_target}, канал показывает {:?}",
                callback_reading().map(|(samples, ..)| samples)
            ));
        };

        println!("  после:     выборка {samples}, p50 {p50} нс, p99 {p99} нс, максимум {max} нс");
        println!(
            "\n  залп {} нажатий пакетами по {batch_len}, инжекция {} мс; \
             вызовов за залп {}",
            batches * batch_len,
            injection.as_millis(),
            samples - samples_before
        );
        println!(
            "  NFR-01 (p99 < 100 000 нс): {}   NFR-02 (максимум < 1 000 000 нс): {}",
            if p99 < 100_000 { "да" } else { "⛔ НЕТ" },
            if max < 1_000_000 {
                "да"
            } else {
                "⛔ НЕТ"
            }
        );

        Ok(())
    })();

    println!("{}", field.close());
    match product.stop() {
        Ok(code) => println!("продукт остановлен, код {code}"),
        Err(error) => println!("⚠ {error}"),
    }

    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("\nопыт не доведён: {reason}");
            std::process::ExitCode::from(2)
        }
    }
}

// ---------------------------------------------------------------------------------------
// Опыт T-10-15 — какой из пяти исходов перештамповки происходит на самом деле
// ---------------------------------------------------------------------------------------

/// The five outcome keys of task **T-10-15**, in the order `control::KEYS` lists them.
const RESTAMP_KEYS: [&str; 5] = [
    "restamp_skips",
    "restamp_no_probe",
    "restamp_unchanged",
    "restamp_uncached",
    "restamp_accepted",
];

/// One reading of the five outcome counters of `buffer::Recorder::restamp`.
///
/// ⚠ **A reading is only worth something as a difference.** Every one of the five is a monotone
/// count over the whole life of the process, so the number that answers "what did *this* press
/// do" is `after - before` around that press and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RestampCounts([u64; RESTAMP_KEYS.len()]);

impl RestampCounts {
    /// Reads all five, or `None` when the channel could not answer or a key is missing — which is
    /// what a product built before this task would look like, and is a finding rather than a zero.
    fn take() -> Option<Self> {
        let snapshot = crate::channel::read().ok()?;
        let mut values = [0u64; RESTAMP_KEYS.len()];

        for (slot, key) in values.iter_mut().zip(RESTAMP_KEYS) {
            *slot = snapshot.get(key)?.parse::<u64>().ok()?;
        }

        Some(Self(values))
    }

    /// The difference from an earlier reading.
    fn since(self, before: Self) -> [i64; RESTAMP_KEYS.len()] {
        core::array::from_fn(|index| self.0[index] as i64 - before.0[index] as i64)
    }
}

/// One line naming what the five counters did between `before` and `after`.
fn describe_restamp(before: Option<RestampCounts>, after: Option<RestampCounts>) -> String {
    let (Some(before), Some(after)) = (before, after) else {
        return "  исходы restamp: <канал не ответил или ключей нет — сборка без T-10-15?>"
            .to_owned();
    };

    let delta = after.since(before);
    let pairs: Vec<String> = RESTAMP_KEYS
        .iter()
        .zip(delta)
        .map(|(key, value)| format!("{key}{value:+}"))
        .collect();

    format!("  исходы restamp за шаг: {}", pairs.join(" "))
}

/// Which single outcome a step took, when exactly one of the four *calls* moved.
///
/// `restamp_skips` is excluded on purpose: it moves on every stroke after the first of a word, so
/// it is never the answer to "what did the first stroke do" and would mask the one that is.
fn sole_outcome(before: RestampCounts, after: RestampCounts) -> Option<&'static str> {
    let delta = after.since(before);
    let moved: Vec<&'static str> = RESTAMP_KEYS
        .iter()
        .zip(delta)
        .skip(1)
        .filter_map(|(key, value)| (value > 0).then_some(*key))
        .collect();

    (moved.len() == 1).then(|| moved[0])
}

/// ⭐ **The experiment of task T-10-15** — which of the five outcomes of `Recorder::restamp`
/// really happens, measured on the **installed** copy.
///
/// # What this is, and what it deliberately is not
///
/// It is not a fourth repair and not a fourth guess. Task T-10-14 reproduced the divergence of the
/// stamp and cured it six times out of six — **on a build the bench itself started**, with a
/// synthetic `Alt+Shift`, and with no departure of the system anywhere in it. The person's defect
/// survives on the **installed** copy after **real** sleep and a real lock. This mode removes the
/// first of those two differences entirely and says plainly what it could not do about the second.
///
/// # The arms
///
/// | Arm | What it establishes |
/// |---|---|
/// | 0 | ⛔ **negative control of the instrument**: the hotkey on an empty field must change nothing. An instrument that cannot report "не изменилось" cannot report anything |
/// | A | positive control: a fresh stamp converts, and the five counters say `restamp_unchanged` or `restamp_accepted` |
/// | B | the household case of T-10-14 — `Alt+Shift` into the window that already has the focus — now on the installed copy and with the outcome named by number |
/// | C | ⭐ **positive control of `restamp_uncached`**: a third layout attached for the length of the arm, the window moved into it with no focus change, so the cache of FR-20 provably has no map for what the probe reads |
///
/// Arm C is what makes a zero in arm B mean anything. Without it, `restamp_uncached=0` would be
/// indistinguishable from a counter that never fires — the exact class of mistake that has cost
/// this project three repairs, and the reason the instrument is checked before its readings.
///
/// # ⛔ What no arm here does
///
/// The machine is not suspended, not locked, and no desktop is switched: `LockWorkStation` would
/// take the controller's session down with it and `Ctrl+Alt+Del` cannot be sent at all. The one
/// remaining route to a **real** `EVENT_SYSTEM_DESKTOPSWITCH` without a person is the secure
/// desktop of a UAC prompt, and whether this machine has one is a question of measurement, not of
/// argument — it is measured in the report of this task and not here, because raising an elevated
/// process would put a window this bench does not own in front of every arm that follows it.
pub fn experiment_away(ctx: &Context, rounds: usize) -> std::process::ExitCode {
    println!("--- ОПЫТ T-10-15: какой из пяти исходов перештамповки происходит ---\n");
    println!(
        "Прибор: сравнение поля ДО и ПОСЛЕ нажатия. Совпадение с образцом {EXPECTED:?} \
         сообщается рядом и НИКОГДА не является вердиктом — под русской раскладкой ghbdtn \
         даёт привет уже при наборе.\n\
         Продукт: УСТАНОВЛЕННАЯ подписанная сборка из %ProgramFiles% (uiAccess=1), поднятая \
         ShellExecuteEx, стендом НЕ порождённая.\n\
         ⛔ Машина не усыпляется, не блокируется и рабочий стол не переключается.\n"
    );

    protocol(&format!(
        "\n\n=========== ОПЫТ T-10-15 {} ===========\nкругов бытового случая: {rounds}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    ));
    println!(
        "Полный протокол каждого снимка: {}",
        protocol_path().display()
    );

    let already = crate::sut::any_running();
    if !already.is_empty() {
        eprintln!(
            "⛔ продукт уже запущен: {already:?}. Синтетическая FR-96 сняла бы чужой экземпляр, \
             а показания канала были бы неизвестно чьими. Опыт не ставится."
        );
        return std::process::ExitCode::from(1);
    }

    let _clipboard = clip::Guard::capture();

    let mut product = match crate::sut::Installed::launch() {
        Ok(installed) => installed,
        Err(error) => {
            eprintln!("установленный продукт не запустился: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let Some(ready) = product.await_ready(Duration::from_secs(30)) else {
        eprintln!("установленный продукт не сообщил о готовности через канал SEC-04a за 30 с");
        let _ = product.stop();
        return std::process::ExitCode::from(1);
    };

    println!(
        "Продукт PID {}, hook_installed={}, ключей в снимке {}",
        product.pid,
        ready.get("hook_installed").unwrap_or("?"),
        ready.present_keys().len()
    );
    println!(
        "Снимок канала целиком, сразу после готовности:\n{}",
        ready.raw
    );
    protocol(&format!(
        "[готовность] PID {}\n{}",
        product.pid,
        ready.raw.trim_end()
    ));

    // ⛔ The five keys have to be there, or every reading below is a fabricated zero.
    for key in RESTAMP_KEYS {
        if ready.get(key).is_none() {
            eprintln!(
                "⛔ установленная сборка не публикует {key}: это сборка без счётчиков T-10-15, \
                 и все показания ниже были бы выдуманными нулями. Опыт не ставится."
            );
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    }

    let mut field = match launch_stamp_window() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("окно стенда не открылось: {error}");
            let _ = product.stop();
            return std::process::ExitCode::from(1);
        }
    };

    let outcome = (|| -> Result<(usize, bool), String> {
        let (target, content) = adopt_stamp_window(ctx, &mut field)?;

        // ---- arm 0: the negative control of the instrument itself --------------------------
        //
        // The hotkey on an **empty** field. The product is running, the hook is up, the press
        // really reaches it — and there is nothing in the buffer to convert, so the field must
        // read the same afterwards. An instrument that reports «изменилось» here is measuring
        // something other than what it claims, and everything below it would be worthless.
        println!("=== ПЛЕЧО 0: отрицательный контроль прибора — нажатие на пустом поле ===");
        clear_field(&target, &content)?;
        let empty_before =
            read_field(&content).ok_or_else(|| "поле не читается перед нажатием".to_owned())?;
        println!("{}", watched("плечо 0: поле пусто, до нажатия"));
        input::tap(ctx.hotkey_vk, &target).map_err(|error| format!("горячая клавиша: {error}"))?;
        let empty_after = wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(&content).filter(|text| text != &empty_before)
        })
        .or_else(|| read_field(&content))
        .unwrap_or_else(|| "<чтение не удалось>".to_owned());
        println!("{}", watched("плечо 0: после нажатия"));

        let idle = PressOutcome {
            before: empty_before,
            after: empty_after,
            wanted: String::new(),
        };
        println!("  {}", idle.describe());
        if idle.changed() {
            return Err(format!(
                "⛔ ПРИБОР НЕИСПРАВЕН: нажатие на пустом поле изменило текст ({})",
                idle.describe()
            ));
        }
        println!("  ✔ прибор умеет сказать «не изменилось» — ему можно верить\n");

        // ---- arm A: the positive control ---------------------------------------------------
        println!("=== ПЛЕЧО A: штамп свеж — положительный контроль ===");
        let (agreed, sample) = await_agreement(target.hwnd);
        println!("  {}", sample.describe());
        if !agreed {
            return Err(format!(
                "штамп не сошёлся с настоящей раскладкой за {} с ещё до всякого стимула — \
                 плечо A недостоверно",
                STAMP_CONVERGENCE_BOUND.as_secs()
            ));
        }

        let before = RestampCounts::take();
        let fresh = press_once(ctx, &target, &content, "A", layout::US)?;
        let after = RestampCounts::take();
        println!("  {}", fresh.describe());
        println!("{}", describe_restamp(before, after));
        if !fresh.changed() {
            return Err(
                "плечо A не изменило текст — прибор не годится, всё ниже недостоверно".to_owned(),
            );
        }
        println!();

        // ---- arm B: the household case, on the installed copy -------------------------------
        let mut unchanged_rounds = 0usize;

        for round in 1..=rounds {
            println!("=== ПЛЕЧО B, круг {round}: Alt+Shift в окно, которое уже переднее ===");

            // Cleared with `Backspace` alone: `Ctrl+A` ends in a modifier release, and a modifier
            // release **is** the probe of T-03-3c — the round would refresh the very stamp it is
            // trying to leave stale. Measured in task T-10-14 and inherited here.
            clear_field(&target, &content)?;

            // The healthy precondition, built and verified rather than assumed — and the
            // direction is fixed for the reason T-10-14 measured: the probe wins the race against
            // an activation of `en-US` every time and loses it against `ru-RU` every time, so a
            // round that took whatever the previous press left behind would measure the direction
            // and not the defect.
            let from = layout::ensure(target.hwnd, layout::US, Duration::from_secs(5))?;
            input::tap(VK_SHIFT.0, &target)
                .map_err(|error| format!("Shift подготовки: {error}"))?;
            let (started, start) = await_agreement(target.hwnd);
            println!(
                "  до переключения (сошлись: {started}): {}",
                start.describe()
            );
            if !started {
                println!("  ⚠ штамп не сошёлся ещё до стимула — круг пропущен");
                continue;
            }

            let to = other_layout(from);
            println!("  стимул: Alt+Shift в переднее окно, фокус не трогали");
            input::chord(&[VK_MENU.0], VK_SHIFT.0, &target)
                .map_err(|error| format!("Alt+Shift: {error}"))?;

            let settled = wait::until(Duration::from_secs(5), || {
                layout::of_window(target.hwnd)
                    .map(layout::id_of)
                    .filter(|id| id & 0xFFFF == to & 0xFFFF)
            });
            let Some(settled) = settled else {
                println!(
                    "  ⚠ окно не перешло в {} за 5 с — круг пропущен",
                    layout::describe(to)
                );
                continue;
            };
            println!(
                "  окно перешло {} → {}",
                layout::describe(from),
                layout::describe(settled)
            );

            let (converged, after_switch) = await_agreement(target.hwnd);
            println!("  после переключения: {}", after_switch.describe());
            println!(
                "  штамп догнал настоящую раскладку сам: {}",
                if converged { "да" } else { "⛔ НЕТ" }
            );

            let shown = shown_under(settled);
            let at_typing = StampSample::take(target.hwnd);
            println!("  в момент набора: {}", at_typing.describe());

            // ⭐ The reading that this whole task exists for: the counters **around the first
            // stroke of the word** and nothing wider. `type_paced_as` types six keys, of which
            // exactly one — the first — reaches `restamp` at all.
            let before = RestampCounts::take();
            type_paced_as(TYPED, shown, &target, &content, "")?;
            let after = RestampCounts::take();

            let typed_before =
                read_field(&content).ok_or_else(|| "поле не читается перед нажатием".to_owned())?;
            println!("{}", watched(&format!("круг {round}: набрано, до нажатия")));
            println!("{}", describe_restamp(before, after));
            if let (Some(before), Some(after)) = (before, after)
                && let Some(outcome) = sole_outcome(before, after)
            {
                println!("  ⭐ исход первого штриха слова: {outcome}");
            }

            input::tap(ctx.hotkey_vk, &target)
                .map_err(|error| format!("горячая клавиша: {error}"))?;

            let text_after = wait::until(SERIES_STEP_TIMEOUT, || {
                read_field(&content).filter(|text| text != &typed_before)
            })
            .or_else(|| read_field(&content))
            .unwrap_or_else(|| "<чтение не удалось>".to_owned());

            let press = PressOutcome {
                before: typed_before,
                after: text_after,
                wanted: shown_under(other_layout(settled)).to_owned(),
            };
            println!("{}", watched(&format!("круг {round}: после нажатия")));
            println!(
                "  ⭐ круг {round}: набирали под {}, штамп говорил {} — {}",
                layout::describe(settled),
                at_typing
                    .stamp
                    .map_or_else(|| "<нет>".to_owned(), layout::describe),
                press.describe()
            );

            if !press.changed() {
                unchanged_rounds += 1;
            }
            println!();
        }

        // ---- arm C: the positive control of `restamp_uncached` ------------------------------
        let uncached_fires = arm_uncached(&target, &content)?;

        println!("\n=== ИТОГ ===");
        println!("  кругов бытового случая: {rounds}");
        println!("  ⛔ нажатие НЕ изменило текст: {unchanged_rounds}/{rounds}");
        println!(
            "  счётчик restamp_uncached срабатывает, когда карты нет в кэше: {}",
            if uncached_fires { "ДА" } else { "⛔ НЕТ" }
        );

        Ok((unchanged_rounds, uncached_fires))
    })();

    println!("{}", field.close());
    match product.stop() {
        Ok(()) => println!("продукт остановлен через FR-96"),
        Err(error) => println!("⚠ {error}"),
    }

    match outcome {
        // The exit code is the detector of arm B, exactly as it is in the mode of T-10-14: green
        // when every press changed the field, red when any press did nothing at all.
        Ok((0, _)) => {
            println!("\nВЕРДИКТ: все нажатия изменили текст — ЗЕЛЁНЫЙ");
            std::process::ExitCode::SUCCESS
        }
        Ok((unchanged, _)) => {
            println!("\nВЕРДИКТ: нажатий, ничего не изменивших: {unchanged} — ⛔ КРАСНЫЙ");
            std::process::ExitCode::from(1)
        }
        Err(reason) => {
            eprintln!("\nопыт не доведён: {reason}");
            std::process::ExitCode::from(2)
        }
    }
}

/// ⭐ **Arm C — the positive control of `restamp_uncached`**, and the only arm that can produce
/// the silent refusal on purpose.
///
/// # Why a third layout
///
/// The refusal happens when the probe reads a real layout the cache of FR-20 has **no map for**.
/// With the two layouts of this session that state is unreachable: both were in
/// `GetKeyboardLayoutList` when the product built its cache. A layout attached *after* the build,
/// and made current with no focus change and no foreground change, is that state by construction —
/// `app::refresh_layout_and_cache` is the only thing that rebuilds the cache and it runs on
/// exactly the two events this arm avoids.
///
/// This is not the person's configuration and is not claimed to be. It is what makes a zero
/// reading of `restamp_uncached` in arm B *mean* something: a counter that has been seen to fire.
///
/// # ⛔ The layout is given back on every path out
///
/// `layout::Temporary` detaches on success, on failure and on an unwind, and leaves a note on
/// disk that the next run acts on if this process is killed between the two. Footnote 3 of §11.3
/// and the rule it is built on: never take away, or leave behind, something on somebody's machine.
fn arm_uncached(target: &input::Target, content: &Element) -> Result<bool, String> {
    println!("=== ПЛЕЧО C: положительный контроль счётчика restamp_uncached ===");
    println!(
        "  Третья раскладка подключается на время плеча и снимается на любом пути выхода \
         (сноска 3 §11.3)."
    );

    let mut stash = match layout::Temporary::attach() {
        Ok(stash) => stash,
        Err(error) => {
            println!("  ⚠ третья раскладка не подключилась: {error}");
            println!("  плечо C не поставлено — контроля счётчика нет");
            return Ok(false);
        }
    };

    let measured = (|| -> Result<bool, String> {
        println!(
            "  третья раскладка подключена: {}",
            layout::describe(stash.handle())
        );
        println!("{}", watched("плечо C: третья раскладка подключена"));

        clear_field(target, content)?;

        // The window moves into the third layout by the same message the household `Alt+Shift`
        // ends in — a request to the window that already has the focus. Nothing is activated,
        // no focus moves, so nothing asks the product to rebuild its cache.
        layout::ensure(target.hwnd, layout::THIRD, Duration::from_secs(5))?;
        println!(
            "  окно переведено в {} без смены фокуса",
            layout::describe(layout::THIRD)
        );

        let before = RestampCounts::take();
        // One key, and it is the first stroke of a new word — the only stroke that reaches
        // `restamp` at all. What it puts in the field is of no interest here; the counter is.
        input::type_text("g", target)
            .map_err(|error| format!("ввод под третьей раскладкой: {error}"))?;
        wait::until(SERIES_STEP_TIMEOUT, || {
            read_field(content).filter(|text| !text.is_empty())
        });
        let after = RestampCounts::take();

        println!("{}", watched("плечо C: штрих под третьей раскладкой"));
        println!("{}", describe_restamp(before, after));

        let fired = match (before, after) {
            (Some(before), Some(after)) => after.since(before)[3] > 0,
            _ => false,
        };

        println!(
            "  ⭐ молчаливый отказ виден числом: {}",
            if fired {
                "ДА — restamp_uncached вырос"
            } else {
                "⛔ НЕТ — счётчик не двинулся; см. cache_builds выше"
            }
        );

        clear_field(target, content)?;
        Ok(fired)
    })();

    println!("  {}", stash.detach());
    // Back to the layout every other arm assumes, before anything else runs.
    let _ = layout::ensure(target.hwnd, layout::US, Duration::from_secs(5));

    measured
}
