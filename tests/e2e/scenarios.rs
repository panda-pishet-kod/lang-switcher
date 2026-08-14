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
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_A, VK_CONTROL, VK_DELETE, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

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
    } = scene;

    // Step 1 — forward. Rake 3, for every application and not only for Word. The window handle
    // goes too: the window is in registry A, so Win32 may be used on it directly.
    if let Err(error) = shell::activate_window(pid, Some(window)) {
        return both_failed(position, app_name, &format!("вывод окна вперёд: {error}"));
    }

    let target = input::Target { pid, hwnd: window };

    // Step 2 — the precondition the scenario states: the English layout.
    let source_layout = match layout::ensure(window, layout::US, Duration::from_secs(5)) {
        Ok(id) => id,
        Err(error) => {
            return both_failed(position, app_name, &format!("исходная раскладка: {error}"));
        }
    };

    // Step 3 — empty the field, so the reading afterwards is only this run's doing.
    if clear_first {
        if let Err(error) = input::chord(&[VK_CONTROL.0], VK_A.0, &target) {
            return both_failed(position, app_name, &format!("очистка поля: {error}"));
        }
        if let Err(error) = input::tap(VK_DELETE.0, &target) {
            return both_failed(position, app_name, &format!("очистка поля: {error}"));
        }
    }

    // Step 4 — type. The foreground check is inside `type_text`; a refusal ends the position
    // here rather than typing into somebody else's window.
    if let Err(error) = input::type_text(TYPED, &target) {
        return both_failed(position, app_name, &format!("ввод {TYPED:?}: {error}"));
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
        return both_failed(
            position,
            app_name,
            &format!(
                "введённое не дошло до приложения: прочитано {:?}, ожидалось содержащее {TYPED:?}",
                actual.unwrap_or_else(|| "<чтение не удалось>".to_owned())
            ),
        );
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
        return both_failed(position, app_name, &format!("горячая клавиша: {error}"));
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

    let mut app = match launch() {
        Ok(app) => app,
        Err(error) => return both_failed(plan.number, plan.name, &format!("запуск: {error}")),
    };

    let mut rows = match adopt_window(ctx.automation, &mut app, plan.window_is) {
        Err(reason) => both_failed(plan.number, plan.name, &reason),
        Ok(window) => match (app.window, (plan.content_in)(ctx.automation, &window)) {
            (None, _) => both_failed(plan.number, plan.name, "у окна нет дескриптора"),
            (_, None) => both_failed(
                plan.number,
                plan.name,
                "элемент ввода не найден в дереве UI Automation",
            ),
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
    notepad_position(ctx, 1, "Блокнот", &[])
}

/// Position 22 is the same application with `Shift` held over the hotkey.
pub fn position_22(ctx: &Context) -> Vec<Row> {
    notepad_position(
        ctx,
        22,
        "Модификаторы: Shift + горячая клавиша (Блокнот)",
        &[VK_SHIFT.0],
    )
}

fn notepad_position(ctx: &Context, number: u8, name: &str, modifiers: &[u16]) -> Vec<Row> {
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

    let child = match Command::new("rundll32.exe").arg("shell32.dll,#61").spawn() {
        Ok(child) => child,
        Err(error) => return both_failed(11, "Диалог «Выполнить»", &format!("запуск: {error}")),
    };

    let mut app = launched("Диалог «Выполнить»", child, CloseWith::WmClose, None);

    let window = adopt_window(ctx.automation, &mut app, &|element: &Element| {
        element.class() == "#32770"
            && (element.name().contains("Выполнить") || element.name().contains("Run"))
    });

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
            "{}; закрытие: {closed}; буфер обмена не изменился: {}",
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

/// Position 14 — a password field, answered through the SEC-04a channel.
///
/// ⚠ This is the one position where the result is invisible from outside — requirement 4 of
/// §11.5 names it. The assertion is "the buffer stays empty", and the only witness is the
/// product's own `buffer_len`, published on the channel.
///
/// The stub is wired to the channel **now**, and it survives the absence of `password_field`:
/// that key arrives with T-06-1 and is listed in `control::RESERVED_KEYS` today. What the stub
/// does is read the snapshot, report `buffer_len` as an observation, and give the verdict
/// `pending` on T-06-1 — because until password-field detection exists there is nothing that
/// could make the buffer stay empty, and a `pass` here would be an accident, not a check.
pub fn position_14(_ctx: &Context) -> Vec<Row> {
    let snapshot = crate::channel::read();

    let (actual, note) = match &snapshot {
        Ok(snapshot) => {
            let buffer_len = snapshot
                .get("buffer_len")
                .map_or("ключ отсутствует".to_owned(), str::to_owned);
            let password = snapshot.get("password_field").map_or_else(
                || "ключ password_field отсутствует, как и ожидается до T-06-1".to_owned(),
                |value| format!("password_field={value}"),
            );
            (
                format!("buffer_len={buffer_len}"),
                format!(
                    "канал опрошен, присутствуют ключи: {}; {password}",
                    snapshot.present_keys().join(", ")
                ),
            )
        }
        Err(error) => (
            format!("канал недоступен: {error}"),
            "канал SEC-04a не ответил".to_owned(),
        ),
    };

    vec![
        Row::pending(
            14,
            "Любое поле пароля",
            Assertion::Other("буфер остаётся пустым"),
            "T-06-1",
            "buffer_len=0 при вводе в поле пароля",
        )
        .with_note(format!("{note}; наблюдение: {actual}")),
    ]
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
        Row::pending(
            15,
            "Путь выделения",
            Assertion::Other("фраза конвертирована, буфер обмена восстановлен"),
            "T-07-2",
            "конвертированная фраза",
        )
        .with_note("путь выделения §4.7 не реализован"),
        Row::pending(
            16,
            "Откат двойным нажатием",
            Assertion::Other("исходный текст восстановлен побитово"),
            "T-05-2",
            format!("{TYPED:?} побитово"),
        )
        .with_note("выбор целевой раскладки по режимам «Пара»/«Цикл» (FR-30…FR-34) не реализован"),
        Row::pending(
            17,
            "Цикл при трёх раскладках",
            Assertion::Other("три нажатия возвращают исходный текст"),
            "T-05-2",
            format!("{TYPED:?} побитово"),
        )
        .with_note(
            "режим «Цикл» не реализован; временная третья раскладка стендом не подключалась",
        ),
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
