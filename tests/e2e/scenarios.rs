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

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Accessibility::{
    UIA_ButtonControlTypeId, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_A, VK_CONTROL, VK_DELETE, VK_HOME, VK_SHIFT, VK_TAB,
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

/// All three rows of position 16 when the scenario could not reach the second press.
///
/// Its own helper and not [`both_failed`] because position 16 asserts three things and expects
/// different values for two of them: the *original* text, and the *source* layout. A row that
/// failed while claiming to have expected `привет` would misreport what the position is for.
fn rollback_failed(position: u8, app: &str, reason: &str) -> Vec<Row> {
    vec![
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
    ]
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

    vec![
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
    ]
}

/// The key of the SEC-04a channel this position rests on — task T-05-2a.
const CYCLE_KEY: &str = "cycle_position";

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
        || {
            let child = Command::new("notepad.exe")
                .spawn()
                .map_err(|error| error.to_string())?;
            // Terminating is safe here and it is not rake 5: unlike Word, Notepad keeps no
            // recovery state that a kill would poison, and the document is the bench's own six
            // characters. `WM_CLOSE` would raise a save prompt and leave a modal window behind.
            Ok(launched("Блокнот", child, CloseWith::Terminate, None))
        },
    )
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

/// Every position that cannot be run yet, with the task that owns it.
///
/// Positions 19 and 21 are **П** — locking the session and sleeping the machine are done by a
/// person in the acceptance session of §11.6, and the task is explicit that no stub is to be
/// written for them. They appear here so the summary counts them, and nowhere else.
pub fn pending_positions() -> Vec<Row> {
    let scenario = format!("текст {EXPECTED:?} и раскладка RU");

    vec![
        Row::pending(
            9,
            "Блокнот от администратора",
            Assertion::Other("работа поверх элевированного окна"),
            "T-09-1",
            &scenario,
        )
        .with_note("механизм uiAccess (§8.1) не реализован"),
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
        Row::pending(
            18,
            "Переключение рабочего стола",
            Assertion::Other("хук восстановлен"),
            "T-06-2",
            "хук восстановлен",
        )
        .with_note("сторож хука (§4.9) не реализован"),
        Row::pending(
            19,
            "Блокировка и разблокировка сеанса",
            Assertion::Other("хук восстановлен"),
            "П",
            "хук восстановлен",
        )
        .with_note("П — приёмочная сессия §11.6; заготовка не пишется по указанию задания"),
        Row::pending(
            20,
            "Перезапуск explorer.exe",
            Assertion::Other("иконка в трее восстановлена"),
            "T-06-2",
            "иконка восстановлена",
        )
        .with_note("сторож (§4.9) не реализован"),
        Row::pending(
            21,
            "Сон и пробуждение",
            Assertion::Other("хук восстановлен"),
            "П",
            "хук восстановлен",
        )
        .with_note("П — приёмочная сессия §11.6; заготовка не пишется по указанию задания"),
        Row::pending(
            23,
            "Производительность, 10 000 нажатий",
            Assertion::Other("p50/p99/максимум в пределах NFR-01, NFR-02"),
            "T-10-1",
            "в пределах NFR-01, NFR-02",
        )
        .with_note("профилирование задержки хука — задача T-10-1"),
    ]
}
