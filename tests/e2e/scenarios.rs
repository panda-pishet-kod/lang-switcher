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
//! Step 7 produces two rows, not one: the text, and the layout. The layout row is always
//! `pending` on **T-05-1**, because switching the layout is that task's and does not exist
//! yet — decision Р-30. The bench still *reads* the layout and reports what it saw, so that
//! when T-05-1 lands the row changes from `pending` to `pass` without the bench changing.

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

/// Finds a window **among those of the process the bench spawned**, and adopts it.
///
/// ⛔ The strongest form of requirement B, and the one to prefer whenever the application keeps
/// its window in the process that was launched: the search never leaves the bench's own process
/// in the first place, so a predicate cannot nominate a stranger's window at all.
///
/// Used where it is known to work — Telegram and VS Code. Chrome needs the broader search
/// because its window can belong to a child process, and there `own::claim_window_process` is
/// what does the deciding.
fn adopt_own_window(
    automation: &Automation,
    app: &mut App,
    predicate: &dyn Fn(&Element) -> bool,
) -> Result<Element, String> {
    let pid = app.pid;

    let window = automation
        .await_window(pid, wait::WINDOW_TIMEOUT, predicate)
        .ok_or_else(|| {
            format!(
                "у запущенного стендом процесса {pid} не появилось подходящего окна; окна чужих \
                 процессов для этой позиции не рассматриваются вовсе (пункт B)"
            )
        })?;

    app.adopt(&window)?;
    Ok(window)
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

    // Step 1 — forward. Rake 3, for every application and not only for Word.
    if let Err(error) = shell::activate(pid) {
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

    // The layout assertion. Read for real, then reported as `pending`: FR-50 is task T-05-1 and
    // is not implemented, so a `fail` here would blame the product for something nobody has
    // written yet, and a `pass` would be a lie. Р-30's third verdict is exactly this case.
    let observed = layout::of_window(window).map(layout::id_of);
    let layout_row = Row::pending(
        position,
        app_name,
        Assertion::Layout,
        "T-05-1",
        layout::describe(layout::RUSSIAN),
    )
    .with_note(format!(
        "фактически наблюдалась {} — переключение раскладки (FR-50, §4.6) не реализовано",
        observed.map_or("неизвестно".to_owned(), layout::describe)
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
    if let Err(error) = shell::activate(pid) {
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

pub fn position_5(ctx: &Context) -> Vec<Row> {
    const CODE: &str = r"<dev>\tools\vscode\Code.exe";

    run_position(
        ctx,
        Plan {
            number: 5,
            name: "Visual Studio Code",
            window_is: &|element: &Element| {
                element.class() == "Chrome_WidgetWin_1"
                    && element.name().contains("Visual Studio Code")
            },
            content_in: &|automation: &Automation, window: &Element| {
                // The editor surface, by whatever name this build gives it. The first attempt
                // required the name to contain `e2e` and found nothing — the tab name is not
                // always what UI Automation reports for the editing surface — so the predicate
                // now accepts any editable text element that can be read back, which is what
                // the scenario actually needs.
                automation.await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                    e.control_type() == Some(UIA_EditControlTypeId)
                        && (e.name().contains("e2e") || e.value().is_some() || e.text().is_some())
                })
            },
            modifiers: &[],
            clear_first: true,
        },
        || {
            let scratch = scratch_dir("vscode");
            let file = scratch.join("e2e.txt");
            std::fs::write(&file, "").map_err(|error| format!("временный файл: {error}"))?;

            let child = Command::new(CODE)
                .args(["--new-window", "--disable-extensions", "--user-data-dir"])
                .arg(scratch.join("user"))
                .arg("--extensions-dir")
                .arg(scratch.join("ext"))
                .arg(&file)
                .spawn()
                .map_err(|error| error.to_string())?;

            Ok(launched(
                "Visual Studio Code",
                child,
                CloseWith::Terminate,
                Some(scratch),
            ))
        },
    )
}

// ---------------------------------------------------------------------------------------
// Positions 6 and 7 — Windows Terminal
// ---------------------------------------------------------------------------------------

pub fn position_6(ctx: &Context) -> Vec<Row> {
    terminal_position(ctx, 6, "Windows Terminal — PowerShell", "PowerShell")
}

pub fn position_7(ctx: &Context) -> Vec<Row> {
    terminal_position(ctx, 7, "Windows Terminal — cmd", "Command Prompt")
}

fn terminal_position(ctx: &Context, number: u8, name: &str, profile: &str) -> Vec<Row> {
    let profile = profile.to_owned();

    run_position(
        ctx,
        Plan {
            number,
            name,
            window_is: &|element: &Element| element.class() == "CASCADIA_HOSTING_WINDOW_CLASS",
            content_in: &|automation: &Automation, window: &Element| {
                automation.await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| {
                    e.control_type() == Some(UIA_DocumentControlTypeId) && e.text().is_some()
                })
            },
            modifiers: &[],
            // ⚠ A console is not a text field: `Ctrl+A` and `Delete` mean other things there,
            // so nothing is cleared and the reading is a `contains` over the visible buffer.
            clear_first: false,
        },
        || {
            let child = Command::new("wt.exe")
                .args(["-w", "new", "-p", &profile])
                .spawn()
                .map_err(|error| error.to_string())?;
            // `wt.exe` is a launcher that exits at once; the window belongs to
            // `WindowsTerminal.exe`, whose process is adopted from the window.
            Ok(launched(&profile, child, CloseWith::WmClose, None))
        },
    )
}

// ---------------------------------------------------------------------------------------
// Position 8 — Telegram Desktop
// ---------------------------------------------------------------------------------------

/// ⚠ **The whole of footnote 7 of §11.3, in code.**
///
/// Work happens in «Избранное» (Saved Messages) and nowhere else, `Enter` is never sent under
/// any circumstance, and the input field is emptied after the scenario. Sending a message to a
/// live contact is an external action and is not allowed in an automatic run — so the bench has
/// no code path that presses `Enter`, not even a guarded one. The only virtual keys named
/// anywhere on this position's path are `Ctrl`, `A`, `Delete` and the hotkey.
pub fn position_8(ctx: &Context) -> Vec<Row> {
    let clipboard = clip::Guard::capture();

    let telegram = match std::env::var("APPDATA") {
        Ok(appdata) => PathBuf::from(appdata)
            .join("Telegram Desktop")
            .join("Telegram.exe"),
        Err(error) => {
            return both_failed(8, "Telegram Desktop", &format!("%APPDATA%: {error}"));
        }
    };
    if !telegram.exists() {
        return both_failed(
            8,
            "Telegram Desktop",
            &format!("{} не найден", telegram.display()),
        );
    }

    let child = match Command::new(&telegram).spawn() {
        Ok(child) => child,
        Err(error) => return both_failed(8, "Telegram Desktop", &format!("запуск: {error}")),
    };

    let mut app = launched("Telegram Desktop", child, CloseWith::WmClose, None);

    // ⚠ The window is **not** identified by title. `probe-telegram.ps1` records why: the title
    // of the Telegram main window carries the name of the open chat, not the word "Telegram",
    // so a title predicate matches nothing — which is exactly how the first run of this position
    // failed. The window is found among the windows of the process the bench started, by its Qt
    // window class.
    let window = adopt_own_window(ctx.automation, &mut app, &|element: &Element| {
        element.class().contains("Qt")
    });

    let mut rows = match (window, app.window) {
        (Ok(window), Some(hwnd)) => telegram_body(ctx, app.pid, hwnd, &window),
        (Err(reason), _) => both_failed(8, "Telegram Desktop", &reason),
        (_, None) => both_failed(8, "Telegram Desktop", "у окна нет дескриптора"),
    };

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

/// Opens «Избранное» and runs the scenario in its input field.
fn telegram_body(ctx: &Context, pid: u32, hwnd: HWND, window: &Element) -> Vec<Row> {
    if let Err(error) = shell::activate(pid) {
        return both_failed(
            8,
            "Telegram Desktop",
            &format!("вывод окна вперёд: {error}"),
        );
    }

    // «Избранное» is opened by invoking its item in the chat list — never by typing into a
    // search box and pressing `Enter`, which is the shape of action footnote 7 forbids.
    let saved = ctx.automation.find(window, &|e: &Element| {
        let name = e.name();
        name == "Избранное" || name == "Saved Messages"
    });

    let Some(saved) = saved else {
        return both_failed(
            8,
            "Telegram Desktop",
            "чат «Избранное» не найден в дереве UI Automation; стенд не переходит ни в какой \
             другой чат — сноска 7 §11.3",
        );
    };

    let opened = saved.invoke() || saved.set_focus();
    if !opened {
        return both_failed(
            8,
            "Telegram Desktop",
            "не удалось открыть «Избранное» через UI Automation; стенд не пользуется \
             синтетической мышью и не переходит в другой чат",
        );
    }

    let field = ctx
        .automation
        .await_element(window, wait::WINDOW_TIMEOUT, &|e: &Element| {
            e.control_type() == Some(UIA_EditControlTypeId)
                && e.class().contains("InputField")
                && e.value().is_some()
        });

    let Some(field) = field else {
        return both_failed(8, "Telegram Desktop", "поле ввода сообщения не найдено");
    };

    let rows = replacement(
        ctx,
        Scene {
            position: 8,
            app_name: "Telegram Desktop",
            pid,
            window: hwnd,
            content: &field,
            modifiers: &[],
            clear_first: true,
        },
    );

    // Footnote 7: the input field is emptied after the scenario. `Enter` is not involved.
    let target = input::Target { pid, hwnd };
    let _ = input::chord(&[VK_CONTROL.0], VK_A.0, &target);
    let _ = input::tap(VK_DELETE.0, &target);

    rows
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
    shell_position_refused(
        10,
        "Проводник — поле поиска",
        "окно Проводника создаёт уже работающая оболочка (explorer.exe передаёт ей запрос и \
         выходит), поэтому окно принадлежит процессу, которого стенд не запускал: пункт B \
         запрещает его усыновлять, а имя explorer стоит и в списке пункта C. Позиция не \
         запускается вовсе — попытка оставляет на рабочем столе пользователя окно, закрыть \
         которое стенд не вправе",
    )
}

/// The two positions that would require driving the shell, refused without being attempted.
fn shell_position_refused(number: u8, name: &str, reason: &str) -> Vec<Row> {
    vec![
        Row::new(
            number,
            name,
            Assertion::Text,
            Verdict::Fail,
            format!("не выполнялась: {reason}"),
            EXPECTED,
        )
        .with_note("см. раздел 7.5 отчёта T-04-3 — вопрос вынесен контролеру"),
        Row::new(
            number,
            name,
            Assertion::Layout,
            Verdict::Fail,
            format!("не выполнялась: {reason}"),
            layout::describe(layout::RUSSIAN),
        ),
    ]
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
/// could not close it again: the toggle did not put it away, and `SearchHost` was left holding
/// the foreground with a visible search overlay. Every position that ran afterwards then failed
/// at `AppActivate` — the overlay took focus back within milliseconds — and positions 1 and 22,
/// which had passed before, began failing for a reason that had nothing to do with the product.
/// Neither `WM_CLOSE`, nor a targeted `Escape`, nor invoking the Start button, nor
/// `ToggleDesktop` would dismiss it; a person pressing `Esc` clears it instantly.
///
/// So the bench does not open the Start menu at all. A position it cannot finish is one thing;
/// a position that leaves the machine unable to run the others is another, and this was the
/// second.
pub fn position_12(_ctx: &Context) -> Vec<Row> {
    shell_position_refused(
        12,
        "Поиск в меню «Пуск»",
        "меню «Пуск» — окно оболочки (StartMenuExperienceHost/SearchHost), стенд его не \
         запускал, и пункт B запрещает его усыновлять. Позиция не запускается вовсе: открытое \
         через SC_TASKLIST меню не удалось закрыть обратно, оверлей поиска остался держать \
         передний план и сорвал последующие позиции",
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
