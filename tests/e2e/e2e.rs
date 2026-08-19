//! Lang_Switcher — the end-to-end acceptance bench of §11.5 of SPEC. Task T-04-3.
//!
//! ⚠ **This program adds the product nothing.** It is the instrument the rest of the tasks are
//! checked with, and every position it closes is one subtracted from the acceptance session
//! with a live person.
//!
//! # Why the entry point is `e2e.rs` and not `main.rs`
//!
//! cargo auto-discovers `tests\*\main.rs` as an integration test. Named that way, this bench
//! would be built and **run** by every `cargo test` — starting Microsoft Word each time. The
//! name `e2e.rs` is not subject to that discovery, and the single `[[bin]]` section in
//! `Cargo.toml` with `required-features = ["testing"]` is the only way it is ever built.
//!
//! # The shape of the program
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`own`] | ⛔ the registry of processes the bench may touch — requirements A to E |
//! | [`wait`] | the one place the bench sleeps, and only as a poll interval |
//! | [`input`] | `SendInput` with the bench's own signature; the foreground guard |
//! | [`uia`] | UI Automation: window readiness, `ValuePattern`, `TextPattern` |
//! | [`layout`] | FR-52 layout of a window; the bench's position on footnote 3 |
//! | [`clip`] | the user's clipboard, saved and restored |
//! | [`config`] | ⚠ the user's `config.toml`, borrowed and given back byte for byte |
//! | [`channel`] | client of the SEC-04a debug channel |
//! | [`sut`] | the product under test: job object, FR-96, the safety net |
//! | [`shell`] | `AppActivate` — rake 3, for every application |
//! | [`word`] | Microsoft Word and the five rakes |
//! | [`report`] | three verdicts and the machine-readable output |
//! | [`scenarios`] | the positions of the matrix |
//!
//! # SEC-01 and SEC-07
//!
//! Neither a character nor a scan code reaches the **product's** log. This is not the
//! product's log: `ghbdtn` and `привет` below are the bench's own test vector, constants of
//! the matrix, printed as the expected and actual values of a check. Nothing here was
//! intercepted from anybody's keyboard, and the bench never reads the contents of the
//! product's buffer — only its length, through SEC-04a.

mod channel;
mod clip;
mod config;
mod input;
mod layout;
mod own;
mod report;
mod scenarios;
mod shell;
mod sut;
mod uia;
mod wait;
mod word;

use std::time::Duration;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

use report::Report;

/// Where the protocol of position 2 is written.
const WORD_PROTOCOL: &str = r"<dev>\control\Lang_Switcher\reports\T-04-3-позиция-2.md";

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    // SAFETY: `CoInitializeEx` initialises COM for this thread. A single-threaded apartment is
    // what UI Automation's client wants and what `probe-word.ps1` proved out. NFR-13: the
    // `HRESULT` is examined below — `S_FALSE` means "already initialised", which is a success.
    let hresult = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hresult.is_err() {
        eprintln!("CoInitializeEx не удалась: {hresult:?}");
        return std::process::ExitCode::from(1);
    }

    let code = run(&arguments);

    // SAFETY: matched with the successful `CoInitializeEx` above, on the same thread.
    unsafe { CoUninitialize() };

    code
}

fn run(arguments: &[String]) -> std::process::ExitCode {
    banner();

    // Requirement 7, first half: the job object and the panic hook. Everything below this line
    // runs with both nets under it.
    if let Err(error) = sut::install_safety_net() {
        eprintln!("не удалось поставить страховку снятия хука: {error}");
        return std::process::ExitCode::from(1);
    }

    // ⚠ The path no code of a killed run could take. If a previous run was ended from outside
    // between borrowing `config.toml` and giving it back, its copy is still on disk and this is
    // where it goes home. Before any mode, because every mode may start the product, and the
    // product reads that file.
    if let Some(said) = config::recover() {
        println!("{said}\n");
    }
    if let Some(said) = layout::recover_temporary() {
        println!("{said}\n");
    }

    match arguments.first().map(String::as_str) {
        Some("--experiment-panic") => experiment_panic(),
        Some("--experiment-kill") => experiment_kill(),
        Some("--channel-only") => channel_only(),
        Some("--experiment-foreign") => experiment_foreign(arguments),
        Some("--classify") => scenarios::classify(&|| uia::Automation::new().ok()),
        Some("--experiment-unicode") => experiment_unicode(),
        Some("--experiment-modes") => experiment_modes(),
        Some("--measure-layout") => measure_layout(arguments.get(1).map(String::as_str)),
        _ => full_run(arguments),
    }
}

/// The two signatures, side by side. Printed first, on every run, whatever else it does.
///
/// ⚠ Requirement 2 of §11.5. If these two were equal the product would filter the bench's
/// input as its own and every position would report success without a single real check. The
/// equality is already impossible — `input.rs` has a `const` assertion that fails the build —
/// and this prints the values so a reader of the output can see it rather than trust it.
fn banner() {
    println!("=== Lang_Switcher — сквозной стенд приёмки §11.5 (T-04-3) ===\n");
    println!("Сигнатуры dwExtraInfo (§11.5 п. 2, FR-03):");
    println!(
        "  hook::INJECTED_SIGNATURE (рабочая, продукта) = 0x{:016X}",
        lang_switcher::hook::INJECTED_SIGNATURE
    );
    println!(
        "  input::BENCH_SIGNATURE   (стенда)            = 0x{:016X}",
        input::BENCH_SIGNATURE
    );
    println!(
        "  различны: {}  (проверено на этапе компиляции: const-assert в tests\\e2e\\input.rs)\n",
        lang_switcher::hook::INJECTED_SIGNATURE != input::BENCH_SIGNATURE
    );
}

/// Requirement 7, the panic half: the hook must go even when the bench falls over.
fn experiment_panic() -> std::process::ExitCode {
    println!("--- ОПЫТ 1: стенд падает с паникой посреди прогона ---\n");

    let sut = sut::Sut::launch().expect("продукт не запустился");
    println!("Продукт запущен, PID {}", sut.pid);

    let ready = sut.await_ready(Duration::from_secs(30));
    println!(
        "Канал SEC-04a: hook_installed={}",
        ready
            .as_ref()
            .and_then(|s| s.get("hook_installed"))
            .unwrap_or("нет ответа")
    );

    println!("\nСейчас будет паника. Хук обязан быть снят панической страховкой (§11.5 п. 7).");
    panic!("намеренная паника опыта 1 — проверка требования 7 §11.5");
}

/// Requirement 7, the external-kill half: the bench is killed and runs no code of its own.
///
/// Prints the two process ids and parks. The operator kills **this** process from outside; the
/// product must go with it, killed by the job object rather than by anything the bench does —
/// there is no code here that could run.
fn experiment_kill() -> std::process::ExitCode {
    println!("--- ОПЫТ 2: стенд убивают снаружи ---\n");

    let sut = sut::Sut::launch().expect("продукт не запустился");
    let ready = sut.await_ready(Duration::from_secs(30));

    println!("СТЕНД PID = {}", std::process::id());
    println!("ПРОДУКТ PID = {}", sut.pid);
    println!(
        "Канал SEC-04a: hook_installed={}",
        ready
            .as_ref()
            .and_then(|s| s.get("hook_installed"))
            .unwrap_or("нет ответа")
    );
    println!("\nГОТОВ. Убивайте стенд снаружи; продукт обязан умереть вместе с ним.");

    // Parked, not spinning: the point of the experiment is that nothing of ours runs.
    loop {
        std::thread::park();
    }
}

/// ⛔ Requirement 15г: shows that the bench refuses a window it did not start.
///
/// # What this experiment does, and what it deliberately does not
///
/// It **launches nothing**. It takes a substring of a window title from the command line, finds
/// every existing top-level window whose title contains it — windows that belong to whatever
/// was already running on this machine — and puts each one through exactly the code path a
/// scenario would: `own::claim_window_process`, the gate of requirement B. Then it asks
/// `own::may_touch`, the gate of requirements C and D, and `own::is_ours`, the gate of
/// requirement E.
///
/// This is the honest form of the test. Making the bench "try" to close a stranger's window and
/// showing that it did not would mean writing a code path that closes strangers' windows — and
/// there is no such path any more. What can be shown is that every gate answers "no", and that
/// the gates are the only way through.
///
/// Usage: `langsw-e2e --experiment-foreign <часть заголовка окна>`
fn experiment_foreign(arguments: &[String]) -> std::process::ExitCode {
    let needle = arguments.get(1).cloned().unwrap_or_default();
    if needle.is_empty() {
        eprintln!("нужен аргумент: часть заголовка постороннего окна");
        return std::process::ExitCode::from(2);
    }

    println!("--- ОПЫТ 3: стенд отказывается трогать постороннее окно (пункты A–E) ---\n");
    println!(
        "Стенд НИЧЕГО не запускал. Реестр пункта A: {}\n",
        own::describe()
    );
    // ⛔ A belt on the experiment itself. `shell::terminate` is called below on a **live foreign
    // process id**, and the entire point is that the gate refuses it. If the registry were not
    // empty, a bug in the gate could turn this demonstration into the accident it exists to rule
    // out. An empty registry makes the refusal structural: `descends_from_ours` returns false
    // the moment it sees no roots, so `is_ours` is false for every process on the machine.
    if !own::registry_is_empty() {
        eprintln!("⛔ реестр не пуст — опыт отменён, чтобы он сам не мог ничего снять");
        return std::process::ExitCode::from(2);
    }

    println!("Ищу существующие окна, чей заголовок содержит {needle:?}.\n");

    let Ok(automation) = uia::Automation::new() else {
        eprintln!("UI Automation недоступна");
        return std::process::ExitCode::from(1);
    };

    let candidates =
        automation.top_level_of_any(&|element: &uia::Element| element.name().contains(&needle));

    if candidates.is_empty() {
        println!("Окон с таким заголовком не найдено. Что стенд видит на верхнем уровне:");
        for window in automation.top_level_of_any(&|_: &uia::Element| true) {
            let pid = window.pid().unwrap_or(0);
            println!(
                "  pid={pid:<6} {:<24} name={:?}",
                own::process_table_name(pid).unwrap_or_else(|| "<?>".to_owned()),
                window.name()
            );
        }
        return std::process::ExitCode::from(1);
    }

    let mut report = Report::default();

    for window in &candidates {
        let pid = window.pid().unwrap_or(0);
        let name = own::process_table_name(pid).unwrap_or_else(|| "<неизвестно>".to_owned());

        println!("Окно: {}", window.describe());
        println!("  процесс {pid} ({name})");
        println!(
            "  имя в запретном списке пункта C: {}",
            own::is_protected(&name)
        );

        // Requirement B — the gate a scenario would call.
        let claim = own::claim_window_process(pid);
        println!(
            "  own::claim_window_process → {}",
            match &claim {
                Ok(()) => "УСЫНОВЛЕНО (это окно принадлежит стенду)".to_owned(),
                Err(reason) => format!("ОТКАЗ: {reason}"),
            }
        );

        // Requirements C and D — the gate `shell::terminate` calls before it opens a handle.
        let touch = own::may_touch(pid);
        println!(
            "  own::may_touch → {}",
            match &touch {
                Ok(()) => "разрешено".to_owned(),
                Err(reason) => format!("ОТКАЗ: {reason}"),
            }
        );

        // Requirement D, end to end: the real function, on the real process id.
        let terminated = shell::terminate(pid);
        println!(
            "  shell::terminate → {}",
            match &terminated {
                Ok(()) => "⚠⚠ ПРОЦЕСС СНЯТ — ЗАЩИТА НЕ РАБОТАЕТ".to_owned(),
                Err(reason) => format!("ОТКАЗ, процесс не тронут: {reason}"),
            }
        );

        // Requirement E: the gate every send passes.
        println!("  own::is_ours (пункт E) → {}\n", own::is_ours(pid));

        let refused = claim.is_err() && touch.is_err() && terminated.is_err();
        report.push(
            report::Row::new(
                0,
                format!("постороннее окно {name} (PID {pid})"),
                report::Assertion::Other("стенд отказался трогать"),
                if refused {
                    report::Verdict::Pass
                } else {
                    report::Verdict::Fail
                },
                if refused {
                    "отказано всеми тремя воротами: B, C+D, E"
                } else {
                    "ХОТЯ БЫ ОДНИ ВОРОТА ПРОПУСТИЛИ"
                },
                "отказ по всем воротам",
            )
            .with_note(window.describe()),
        );
    }

    println!("{}", report.render());
    println!(
        "\nПроцесс(ы) после опыта живы: {}",
        candidates
            .iter()
            .filter_map(uia::Element::pid)
            .map(|pid| format!("{pid}={}", shell::process_is_alive(pid)))
            .collect::<Vec<_>>()
            .join(", ")
    );

    if report.count(report::Verdict::Fail) == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}

/// **Task T-10-6** — the delivery experiment, with no product anywhere near it.
///
/// The arm is here and the experiment is in `scenarios.rs` for the reason every position is split
/// the same way: this file owns the list of what can be run and `scenarios.rs` owns what running
/// it means. See [`scenarios::experiment_unicode`] for what it separates.
fn experiment_unicode() -> std::process::ExitCode {
    let automation = match uia::Automation::new() {
        Ok(automation) => automation,
        Err(error) => {
            eprintln!("UI Automation недоступна: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let context = scenarios::Context {
        automation: &automation,
        hotkey_vk: hotkey_vk(),
        ambient_before: layout::ambient(),
    };

    scenarios::experiment_unicode(&context)
}

/// **Task T-10-6** — the same series under three configurations of §7.
///
/// See [`scenarios::experiment_modes`]; this arm only builds the context, exactly as
/// [`experiment_unicode`] does.
fn experiment_modes() -> std::process::ExitCode {
    let automation = match uia::Automation::new() {
        Ok(automation) => automation,
        Err(error) => {
            eprintln!("UI Automation недоступна: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let context = scenarios::Context {
        automation: &automation,
        hotkey_vk: hotkey_vk(),
        ambient_before: layout::ambient(),
    };

    scenarios::experiment_modes(&context)
}

/// The measurement of position 11 — rule Р-39, and see [`scenarios::measure_layout`].
///
/// Kept as a mode of the bench rather than thrown away with the task that needed it: the
/// question it answers — what layout a newly created window of this session starts in — is a
/// property of the *machine*, and a later run on a differently configured one will need the
/// number again rather than the conclusion drawn from it here.
/// `langsw-e2e --measure-layout [ru|en]` — the optional argument is the layout the session is
/// **left** in, which is how the machine is put back into the state position 11 used to fail in.
/// Without it the measurement restores what it found, as every mode of this bench does.
fn measure_layout(leave: Option<&str>) -> std::process::ExitCode {
    let leave = match leave {
        Some("ru") => Some(layout::RUSSIAN),
        Some("en") => Some(layout::US),
        Some(other) => {
            eprintln!("не понимаю {other:?}: ожидается ru, en или ничего");
            return std::process::ExitCode::from(2);
        }
        None => None,
    };

    let Ok(automation) = uia::Automation::new() else {
        eprintln!("UI Automation недоступна");
        return std::process::ExitCode::from(1);
    };

    scenarios::measure_layout(&automation, leave);
    std::process::ExitCode::SUCCESS
}

/// Reads the channel once and prints it — used to show what SEC-04a hands over.
fn channel_only() -> std::process::ExitCode {
    let mut sut = match sut::Sut::launch() {
        Ok(sut) => sut,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::from(1);
        }
    };

    match sut.await_ready(Duration::from_secs(30)) {
        Some(snapshot) => {
            println!("Имя канала: {}", lang_switcher::control::pipe_name());
            println!("\nСнимок целиком:\n{}", snapshot.raw);
            println!(
                "Присутствуют ключи ({}): {}",
                snapshot.present_keys().len(),
                snapshot.present_keys().join(", ")
            );
            println!(
                "Зарезервированные ключи (T-06-1): {:?} — присутствуют: {:?}",
                lang_switcher::control::RESERVED_KEYS,
                snapshot.reserved_present()
            );
            match channel::write_is_refused() {
                Ok(error) => println!("Открытие канала на запись отвергнуто: {error}"),
                Err(problem) => println!("⚠ {problem}"),
            }
        }
        None => println!("канал не ответил"),
    }

    let _ = sut.stop();
    std::process::ExitCode::SUCCESS
}

/// The acceptance run.
fn full_run(arguments: &[String]) -> std::process::ExitCode {
    let wanted = selected_positions(arguments);

    // ---- what the machine looked like before ----
    let clipboard_before = clip::Guard::capture();
    let layouts_before = layout::attached();
    let stray_before = sut::any_running();

    // ⛔ Read through a window of the bench's **own**, opened and closed for the reading — a new
    // window inherits the layout of the session, so asking one *is* the reading. See the header
    // of `layout.rs` for the measurement behind that, and `layout::set_ambient` for why the
    // window is not kept.
    let ambient_before = layout::ambient();

    println!("Состояние машины до прогона:");
    println!(
        "  подключённые раскладки: {:?}",
        layouts_before
            .iter()
            .map(|id| layout::describe(*id))
            .collect::<Vec<_>>()
    );
    println!(
        "  окружающая раскладка (в ней открывается новое окно): {}",
        ambient_before.map_or("<не читается>".to_owned(), layout::describe)
    );
    println!("  буфер обмена: {}", clipboard_before.saved().describe());
    println!("  процессы LangSwitcher: {stray_before:?}");

    if !stray_before.is_empty() {
        eprintln!(
            "\n⚠ на машине уже работает LangSwitcher — прогон остановлен, чтобы не мешать чужому экземпляру"
        );
        return std::process::ExitCode::from(1);
    }

    let hotkey = hotkey_vk();
    println!("\nГорячая клавиша продукта: VK 0x{hotkey:02X}");
    println!(
        "Продукт поднимается заново на каждую позицию, с LANGSW_DEBUG_TIMEOUT_SEC={}",
        sut::DEBUG_TIMEOUT_SECS
    );

    // ---- the positions ----
    let automation = match uia::Automation::new() {
        Ok(automation) => automation,
        Err(error) => {
            eprintln!("UI Automation недоступна: {error}");
            return std::process::ExitCode::from(1);
        }
    };

    let context = scenarios::Context {
        automation: &automation,
        hotkey_vk: hotkey,
        ambient_before,
    };

    let mut report = Report::default();

    for position in &wanted {
        println!("\n--- позиция {position} ---");
        own::forget_refusals();

        // ⚠ **A fresh copy of the product for every position.** The task caps
        // `LANGSW_DEBUG_TIMEOUT_SEC` at 30 to 60 seconds, and a whole matrix does not fit into
        // one minute — the first trial run had the product end itself on its own FR-97 deadline
        // half-way through, with the remaining positions then testing nothing. Restarting per
        // position keeps the cap, keeps the hook alive for the whole of each scenario, and has
        // the side benefit that every position starts against an empty buffer.
        let mut product = match sut::Sut::launch() {
            Ok(product) => product,
            Err(error) => {
                eprintln!("  продукт не запустился: {error}");
                continue;
            }
        };

        let Some(ready) = product.await_ready(Duration::from_secs(30)) else {
            eprintln!("  продукт не сообщил о готовности через канал SEC-04a за 30 с");
            continue;
        };
        println!(
            "  продукт PID {}, hook_installed={}, ключей в снимке {}",
            product.pid,
            ready.get("hook_installed").unwrap_or("?"),
            ready.present_keys().len()
        );
        if *position == 1 {
            match channel::write_is_refused() {
                Ok(error) => println!("  канал на запись не открывается: {error}"),
                Err(problem) => println!("  ⚠ {problem}"),
            }
        }

        let rows = match position {
            // ⚠ Position 17 rewrites `config.toml` and needs the product to have **read** it, and
            // §7 says the file is read at start-up. So the copy this loop just launched goes
            // down first and the position raises its own — see `scenarios::position_17`. The
            // `product.stop()` at the end of the iteration then finds it already gone and only
            // reports the exit code.
            17 => {
                match product.stop() {
                    Ok(code) => {
                        println!("  продукт остановлен до подмены конфигурации, код {code}")
                    }
                    Err(error) => println!("  ⚠ {error}"),
                }
                scenarios::position_17(&context)
            }
            1 => scenarios::position_1(&context),
            2 => {
                let (rows, protocol) = scenarios::position_2(&context);
                write_word_protocol(&protocol, &rows);
                rows
            }
            3 => scenarios::position_3(&context),
            4 => scenarios::position_4(&context),
            5 => scenarios::position_5(&context),
            6 => scenarios::position_6(&context),
            7 => scenarios::position_7(&context),
            8 => scenarios::position_8(&context),
            10 => scenarios::position_10(&context),
            11 => scenarios::position_11(&context),
            12 => scenarios::position_12(&context),
            15 => scenarios::position_15(&context),
            16 => scenarios::position_16(&context),
            14 => scenarios::position_14(&context),
            22 => scenarios::position_22(&context),
            23 => scenarios::position_23(&context),
            24 => scenarios::position_24(&context),
            other => {
                println!("  позиция {other} не выполняется стендом");
                Vec::new()
            }
        };

        for row in &rows {
            println!(
                "  {} [{}] {} — фактически {}",
                row.position,
                row.assertion.as_str(),
                row.verdict,
                row.actual
            );
        }
        // ⛔ Requirement B, made visible per position: every window this position declined to
        // touch, and why. A refusal is a finding about the run, and it belongs in the output
        // even when the position went on to succeed with a different window.
        for refusal in own::refusals() {
            println!("  ОТКАЗ: {refusal}");
        }

        report.extend(rows);

        // The product goes down with the position that used it — requirement 5 of §11.5 and the
        // hook-removal guarantee of requirement 7 on the ordinary path.
        match product.stop() {
            Ok(code) if code == sut::EXIT_EMERGENCY => {
                println!("  продукт снят через FR-96, код возврата {code}");
            }
            Ok(code) => println!("  продукт завершён, код возврата {code}"),
            Err(error) => println!("  ⚠ {error}"),
        }

        // ⚠ **Requirement 5 of §11.5, the half that was missing.** The position has just left
        // the session in whatever layout its window ended in — RU, because that is what the
        // product does for a living. The next position's window would be *created* in it, and
        // a position whose window cannot be moved afterwards (the system dialog of position 11)
        // would then fail for a reason belonging to the position before it.
        //
        // So the ambient goes back to what the run found, after **every** position. That is what
        // makes the order of the positions stop mattering, which is the point of the exercise.
        println!("  {}", scenarios::restore_ambient(&context));
    }

    // The positions that wait for a task that does not exist yet — always, whatever was asked
    // for, because the summary has to show the whole matrix.
    report.extend(scenarios::pending_positions());

    // ---- the report ----
    println!("\n{}", report.render());

    // ---- put the machine back ----
    println!("--- восстановление состояния (§11.5 п. 5) ---");

    println!("  {}", scenarios::restore_ambient(&context));
    let ambient_after = layout::ambient();
    println!(
        "  окружающая раскладка: была {}, стала {} — совпала: {}",
        ambient_before.map_or("<не читается>".to_owned(), layout::describe),
        ambient_after.map_or("<не читается>".to_owned(), layout::describe),
        if ambient_after == ambient_before {
            "да"
        } else {
            "НЕТ"
        }
    );

    let layouts_after = layout::attached();
    println!(
        "  раскладки после: {:?}",
        layouts_after
            .iter()
            .map(|id| layout::describe(*id))
            .collect::<Vec<_>>()
    );
    println!(
        "  временные раскладки сняты: {}",
        if layouts_after == layouts_before {
            "да"
        } else {
            "НЕТ"
        }
    );
    println!(
        "  буфер обмена не изменился: {}",
        clipboard_before.unchanged()
    );
    drop(clipboard_before);

    let stray_after = sut::any_running();
    println!("  процессы LangSwitcher после: {stray_after:?}");
    println!("  Word Resiliency: {}", word::resiliency_disabled_items());

    // Р-30: a non-zero `pending` is not a success, and neither is a `fail`.
    if report.count(report::Verdict::Fail) == 0 && report.count(report::Verdict::Pending) == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}

/// Which positions to run: everything the task lists, or what `--positions` names.
fn selected_positions(arguments: &[String]) -> Vec<u8> {
    // The task, section 4: the positions runnable after T-04-1 and T-03-4, plus 16 and 17 —
    // task T-04-3-3, decision Р-52. Position 17 is deliberately **last**: it is the only one
    // that rewrites the user's `config.toml`, and the shorter that file spends replaced the
    // fewer ways a run can end with it still replaced.
    // Position 23 arrived with task T-10-1 and stands between 22 and 24: it is a Notepad
    // position like its neighbours, and it stays ahead of 17 for the same reason everything
    // does — 17 rewrites the user's `config.toml` and goes last.
    const DEFAULT: [u8; 18] = [
        1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 14, 15, 16, 22, 23, 24, 17,
    ];

    let mut iterator = arguments.iter();
    while let Some(argument) = iterator.next() {
        if argument == "--positions"
            && let Some(list) = iterator.next()
        {
            return list
                .split(',')
                .filter_map(|item| item.trim().parse().ok())
                .collect();
        }
    }

    DEFAULT.to_vec()
}

/// The hotkey the running product answers to — read from its own configuration, not assumed.
fn hotkey_vk() -> u16 {
    let path = lang_switcher::settings::default_config_path();

    let name = path
        .map(|path| lang_switcher::settings::read_or_default(&path).0.hotkey.key)
        .unwrap_or_else(|| "Pause".to_owned());

    lang_switcher::hook::vk_from_name(&name).unwrap_or(lang_switcher::hook::DEFAULT_HOTKEY_VK)
}

/// Writes the protocol of position 2.
fn write_word_protocol(protocol: &str, rows: &[report::Row]) {
    let mut text = String::new();
    text.push_str("# T-04-3, протокол позиции 2 — Microsoft Word\n\n");
    text.push_str("Файл записан самим стендом во время прогона.\n\n");
    text.push_str("## Ход сценария\n\n```\n");
    text.push_str(protocol);
    text.push_str("```\n\n## Вердикты\n\n");
    text.push_str("| Утверждение | Вердикт | Фактическое | Ожидаемое | Владелец |\n");
    text.push_str("|---|---|---|---|---|\n");
    for row in rows {
        text.push_str(&format!(
            "| {} | **{}** | {} | {} | {} |\n",
            row.assertion.as_str(),
            row.verdict,
            row.actual,
            row.expected,
            row.owner.as_deref().unwrap_or("—")
        ));
    }

    if let Err(error) = std::fs::write(WORD_PROTOCOL, text) {
        eprintln!("не удалось записать протокол позиции 2: {error}");
    } else {
        println!("  протокол позиции 2 записан в {WORD_PROTOCOL}");
    }
}
