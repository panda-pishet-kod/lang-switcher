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
//! | [`wait`] | the poll interval — the only sleep a verdict waits behind, of the nine listed there |
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
    // ⚠ **Before the banner and before every net**, deliberately. This mode is not a run of the
    // bench: it is the program the console item of `--experiment-sequence` hosts inside a
    // `conhost.exe`, and everything below — the safety net, the config recovery, the layout
    // recovery — belongs to a run that measures something. A parked console must touch nothing.
    if arguments.first().map(String::as_str) == Some("--console-park") {
        return console_park();
    }

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
        Some("--experiment-explorer") => experiment_explorer(arguments),
        Some("--experiment-sequence") => experiment_sequence(arguments),
        Some("--experiment-stamp") => experiment_stamp(arguments),
        Some("--experiment-latency") => experiment_latency(arguments),
        Some("--experiment-away") => experiment_away(arguments),
        Some("--experiment-race") => experiment_race(arguments),
        Some("--experiment-voice") => experiment_voice(arguments),
        Some("--experiment-phase") => experiment_phase(arguments),
        Some("--experiment-threads") => experiment_threads(arguments),
        Some("--experiment-belief") => experiment_belief(arguments),
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

/// **Task T-10-9** — `Win+E` and what conversion does after it.
///
/// The arm is here and the experiment is in `scenarios.rs`, the split every mode of this file
/// follows. `--experiment-explorer [кругов]` — the optional argument is how many rounds are made
/// after the shell window is opened; the task asks for ten, and a smaller number is what a first
/// pass over a new instrument wants.
fn experiment_explorer(arguments: &[String]) -> std::process::ExitCode {
    let rounds = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);
    // ⚠ `--installed` runs the **signed copy in `%ProgramFiles%`** instead of the one built
    // beside this bench, and it is the only configuration the defect was ever seen in: Release
    // manifest, `uiAccess="true"`, signed, out of `%ProgramFiles%`, no FR-97 deadline.
    //
    // ⭐ **Task T-10-10 made it readable.** T-10-9 could reach this копия only as a Release
    // build with no channel, and could not start it at all (`CreateProcess` → 740). The
    // installed copy is now built `--release --features testing` and signed with the same
    // certificate — the manifest comes from `PROFILE`, the channel from the feature — and it is
    // raised by `ShellExecuteEx`, so the bench reads its channel without fathering it.
    let installed = arguments.iter().any(|value| value == "--installed");

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

    scenarios::experiment_explorer(&context, rounds, installed)
}

/// **Task T-10-11** — the человек's whole sequence, and `Win+E` only after all of it.
///
/// The same split as every mode of this file: the arm builds the context, the experiment lives in
/// `scenarios.rs`. `--experiment-sequence [кругов] [--installed]`; the task asks for at least ten
/// verification rounds after the shell window, so ten is the default and a smaller number is what
/// a first pass over a new instrument wants.
fn experiment_sequence(arguments: &[String]) -> std::process::ExitCode {
    let rounds = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10);
    let installed = arguments.iter().any(|value| value == "--installed");

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

    scenarios::experiment_sequence(&context, rounds, installed)
}

/// ⭐ **Task T-10-14** — the layout stamp against the real layout, produced without sleep.
///
/// The same split as every mode of this file: the arm builds the context, the experiment lives
/// in `scenarios.rs`. `--experiment-stamp [кругов]` — each round switches the window's layout
/// with no focus change, asks whether the stamp catches up on its own, and then types and
/// presses. See [`scenarios::experiment_stamp`].
fn experiment_stamp(arguments: &[String]) -> std::process::ExitCode {
    let rounds = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4);

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

    scenarios::experiment_stamp(&context, rounds)
}

/// ⭐ **Task T-10-15** — which of the five outcomes of `Recorder::restamp` really happens, on the
/// **installed** copy.
///
/// The same split as every mode of this file: the arm builds the context, the experiment lives in
/// `scenarios.rs`. `--experiment-away [кругов]`. There is no `--installed` switch, and that is
/// deliberate — the whole subject of the task is the configuration the person's defect lives in,
/// so the installed copy is the only product this mode will run. See
/// [`scenarios::experiment_away`].
fn experiment_away(arguments: &[String]) -> std::process::ExitCode {
    let rounds = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4);

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

    scenarios::experiment_away(&context, rounds)
}

/// ⭐ **Task T-10-16** — the race window, looked for by a **series with the timing varied**.
///
/// `--experiment-race <ступень> [повторов]`, where the stage is one of `lag`, `truth`, `grid`,
/// `control`. The same split as every mode of this file: the arm builds the context, the
/// experiment lives in `scenarios.rs`. See [`scenarios::experiment_race`] for what each stage
/// answers and why they are separate invocations.
fn experiment_race(arguments: &[String]) -> std::process::ExitCode {
    let stage = arguments.get(1).map_or("grid", String::as_str);
    let reps = arguments
        .get(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20);

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

    scenarios::experiment_race(&context, stage, reps)
}

/// ⭐ **Task T-10-17** — the replacement that returns what it took, given a voice.
///
/// `--experiment-voice <ступень> [повторов]`, where the stage is one of `check`, `blink`, `pair`.
/// The same split as every mode of this file: the arm builds the context, the experiment lives in
/// `scenarios.rs`. See [`scenarios::experiment_voice`] for what each stage answers, why the
/// product here is the one this build produced rather than the installed copy, and what the `pair`
/// stage borrows and gives back.
fn experiment_voice(arguments: &[String]) -> std::process::ExitCode {
    let stage = arguments.get(1).map_or("check", String::as_str);
    let reps = arguments
        .get(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);

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

    scenarios::experiment_voice(&context, stage, reps)
}

/// ⭐ **Task T-10-18** — does the phase of the cycle survive an absence?
///
/// `--experiment-phase <ступень> [кругов]`, where the stage is one of `state`, `park`, `power`,
/// `session`, `control`. The same split as every mode of this file: the arm builds the context,
/// the experiment lives in `scenarios.rs`. See [`scenarios::experiment_phase`] for what each stage
/// answers, why `control` is not optional and what the forged messages are.
///
/// ⛔ The machine is not suspended and not locked by any of them.
fn experiment_phase(arguments: &[String]) -> std::process::ExitCode {
    let stage = arguments.get(1).map_or("state", String::as_str);
    let reps = arguments
        .get(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20);

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

    scenarios::experiment_phase(&context, stage, reps)
}

/// ⭐ **Task T-10-19** — two threads, two layouts: is the layout read from the *right* thread?
///
/// `--experiment-threads <ступень> [кругов]`, where the stage is one of `probe`, `pairs`,
/// `control`. The same split as every mode of this file: the arm builds the context, the
/// experiment lives in `scenarios.rs`. See [`scenarios::experiment_threads`] for what each stage
/// answers and why the product is deliberately not launched.
///
/// ⛔ A task of **measurement**: `src\` is not touched. The machine is not suspended, not locked,
/// and no desktop is switched.
fn experiment_threads(arguments: &[String]) -> std::process::ExitCode {
    let stage = arguments.get(1).map_or("pairs", String::as_str);
    let reps = arguments
        .get(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(50);

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

    scenarios::experiment_threads(&context, stage, reps)
}

/// ⭐ **Task T-10-14** — the callback percentiles under a volley that exercises the repair.
///
/// `--experiment-latency [нажатий] [--words]`. See [`scenarios::experiment_latency`] for why
/// this is position 23's measurement on a window of the bench's own and what `--words` changes.
fn experiment_latency(arguments: &[String]) -> std::process::ExitCode {
    let presses = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_200);
    let words = arguments.iter().any(|value| value == "--words");

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

    scenarios::experiment_latency(&context, presses, words)
}

/// ⭐ **Task T-10-20** — the detector of defect E, on the mechanism itself.
///
/// `--experiment-belief [кругов]`, where the number is circles **per side**: the mode runs both
/// directions and both arms, so `60` is 120 circles of the experiment and 120 of the negative
/// control, per application. The same split as every mode of this file: the arm builds the
/// context, the detector lives in `scenarios.rs`. See [`scenarios::experiment_belief`] for what
/// the two arms and the two applications each prove.
fn experiment_belief(arguments: &[String]) -> std::process::ExitCode {
    let reps = arguments
        .get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(60);

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

    scenarios::experiment_belief(&context, reps)
}

/// ⭐ **The program the console item of the sequence hosts** — task T-10-11.
///
/// # Why the bench hosts itself rather than a shell
///
/// Measured before it was written. A `cmd.exe` or a `powershell.exe` started directly on this
/// machine is handed to a **COM-activated `WindowsTerminal.exe` under `svchost.exe`** — not kin to
/// the bench at all, so requirement A refuses it before requirement C ever gets a say; and
/// `conhost.exe cmd.exe` gives the right window class but hands the window to a `cmd.exe`, a
/// protected name that was not spawned **directly**, which `own::claim_window_process` refuses
/// before it walks any ancestry. Relaxing requirement C for a proved descendant is the open
/// question §11.3 position 6 leaves to the controller, and this task does not answer it for them.
///
/// So the console the sequence drives is a real `conhost.exe` — spawned directly, therefore inside
/// requirement C — hosting **this binary**, whose name is not on the protected list and which
/// descends from a process the bench started. The window is a genuine `ConsoleWindowClass`, which
/// is what FR-42а resolves to the `Backspace` path.
///
/// # What it does, and the whole of it
///
/// Reads lines from standard input until the console goes away. That is not a placeholder: a
/// program blocked in `read_line` is what puts a console into **cooked line-input mode with echo**,
/// which is the mode the `Backspace` path of FR-42а exists for and the one positions 6–7 were
/// confirmed in. Nothing is printed but the two lines below, nothing is parsed, and no line is ever
/// acted on — the typing is there to be looked at by UI Automation, not to be obeyed.
/// ⚠ **`CONIN$` and not `stdin`, and the first pass measured why.** `Command::spawn` hands the
/// child the parent's standard handles, and the parent of this process is a bench whose own stdin
/// is a pipe or the null device. Reading `stdin` therefore hit end-of-file at once, the program
/// exited, `conhost.exe` closed with it and the console window the item needs never existed long
/// enough to be found — the run reported «окно приложения не появилось» and nothing else.
/// `CONIN$` is the console input buffer of **this** process's console whatever the inherited
/// handles point at, which is the thing that has to be read for the console to be in cooked mode.
fn console_park() -> std::process::ExitCode {
    use std::io::{BufRead, BufReader};

    println!("langsw-e2e --console-park: окно консоли для пунктов 4-5 опыта T-10-11.");
    println!("Строки читаются и никак не исполняются. Закройте окно, чтобы завершить.");

    // Read **and** write: a console input handle opened read-only cannot be used to put the
    // console into the mode a line read needs. NFR-13: the failure is examined, and it parks
    // rather than exiting, because a window that vanishes is worse to diagnose than one that sits.
    let console = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("CONIN$");

    let Ok(console) = console else {
        println!("⚠ CONIN$ не открылся — окно остаётся, но набор читаться не будет");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    };

    let mut reader = BufReader::new(console);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            // End of input — the console has gone, and so does this.
            Ok(0) | Err(_) => return std::process::ExitCode::SUCCESS,
            Ok(_) => {}
        }
    }
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

    // The positions §11.6 hands to a live person. They have no arm in the `match` below either,
    // and they are **not** a miss: `pending_positions` puts them in the report of every run with
    // the verdict Р-30 gives them. Read from the same list that writes them, so the two cannot
    // drift apart — task T-13-19, addition ordered by the controller.
    let handed_to_a_person: Vec<u8> = scenarios::pending_positions()
        .iter()
        .map(|row| row.position)
        .collect();

    for position in &wanted {
        println!("\n--- позиция {position} ---");
        own::forget_refusals();

        // ⚠ **A fresh copy of the product for every position.** The task caps
        // `LANGSW_DEBUG_TIMEOUT_SEC` at 30 to 60 seconds, and a whole matrix does not fit into
        // one minute — the first trial run had the product end itself on its own FR-97 deadline
        // half-way through, with the remaining positions then testing nothing. Restarting per
        // position keeps the cap, keeps the hook alive for the whole of each scenario, and has
        // the side benefit that every position starts against an empty buffer.
        //
        // ⛔ **Both failures of this launch are rows, not only stderr — task T-13-19.** A position
        // that was asked for and could not be run leaves `Row::infrastructure_failure` in the
        // report, so requirement 6 of §11.5 keeps its four fields and the `fail` counter moves.
        // Before that, an `eprintln!` and a bare `continue` left the position **absent** from the
        // machine-readable block: a consumer checking `fail == 0` — the ordinary CI gate — read
        // the run as a success while the position had never been checked. A position nobody asked
        // for is the other case and stays absent, because it genuinely did not run.
        let mut product = match sut::Sut::launch() {
            Ok(product) => product,
            Err(error) => {
                eprintln!("  продукт не запустился: {error}");
                report.push(report::Row::infrastructure_failure(
                    *position,
                    format!("продукт не запустился: {error}"),
                ));
                continue;
            }
        };

        let Some(ready) = product.await_ready(Duration::from_secs(30)) else {
            eprintln!("  продукт не сообщил о готовности через канал SEC-04a за 30 с");
            report.push(report::Row::infrastructure_failure(
                *position,
                "канал молчит: продукт не сообщил о готовности через SEC-04a за 30 с",
            ));
            // The product itself is not leaked by the `continue`: `Sut` takes itself down in
            // `Drop` — requirement 5 of §11.5 — and nothing of the position ran, so the ambient
            // layout the loop restores at its end was never touched.
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

        // ⛔ **`None` is the position the bench has no arm for — task T-13-19, addition ordered by
        // the controller.** It used to be an empty `Vec`, which lost the position from the report
        // exactly as the two `continue`s above did: `langsw-e2e --positions 99` printed a line to
        // stdout and left the machine-readable block without a trace of it. `selected_positions`
        // does not check the number against `RUNNABLE`, so the arm is reachable from the command
        // line and not only in theory.
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
                Some(scenarios::position_17(&context))
            }
            1 => Some(scenarios::position_1(&context)),
            2 => {
                let (rows, protocol) = scenarios::position_2(&context);
                write_word_protocol(&protocol, &rows);
                Some(rows)
            }
            3 => Some(scenarios::position_3(&context)),
            4 => Some(scenarios::position_4(&context)),
            5 => Some(scenarios::position_5(&context)),
            6 => Some(scenarios::position_6(&context)),
            7 => Some(scenarios::position_7(&context)),
            8 => Some(scenarios::position_8(&context)),
            10 => Some(scenarios::position_10(&context)),
            11 => Some(scenarios::position_11(&context)),
            12 => Some(scenarios::position_12(&context)),
            15 => Some(scenarios::position_15(&context)),
            16 => Some(scenarios::position_16(&context)),
            14 => Some(scenarios::position_14(&context)),
            22 => Some(scenarios::position_22(&context)),
            23 => Some(scenarios::position_23(&context)),
            24 => Some(scenarios::position_24(&context)),
            other => {
                // The line stays: a person at the console reads it, and the report row below is
                // for the consumer of the block. One does not replace the other.
                println!("  позиция {other} не выполняется стендом");
                None
            }
        };

        let rows = rows_of_a_position(rows, *position, &handed_to_a_person);

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

    // Р-30: a non-zero `pending` is not a success, and neither is a `fail`. The rule itself sits
    // in `Report::exit_code` beside the counters, where a unit test can reach it — task T-13-19.
    std::process::ExitCode::from(report.exit_code())
}

/// The rows of one position of the loop: what its scenario produced, or — when the `match` of
/// [`full_run`] has no arm for the position — the single `fail` row that keeps it in the report.
/// Task T-13-19, addition ordered by the controller.
///
/// The one exception is a position §11.6 hands to a live person. Those have no arm either, and
/// they are **not** a miss: `scenarios::pending_positions` puts them in the block of every run as
/// `pending` with owner `П`. A `fail` beside that `pending` would be a false finding about the
/// bench — the position is in the report, not missing from it — so they leave no row here.
fn rows_of_a_position(
    produced: Option<Vec<report::Row>>,
    position: u8,
    handed_to_a_person: &[u8],
) -> Vec<report::Row> {
    if let Some(rows) = produced {
        return rows;
    }
    if handed_to_a_person.contains(&position) {
        return Vec::new();
    }
    vec![report::Row::position_without_a_scenario(position)]
}

/// The positions the `match` of [`full_run`] answers, and what a run with no `--positions` asks
/// for. Every other number reaches the `other` arm above.
///
/// The task, section 4: the positions runnable after T-04-1 and T-03-4, plus 16 and 17 —
/// task T-04-3-3, decision Р-52. Position 17 is deliberately **last**: it is the only one
/// that rewrites the user's `config.toml`, and the shorter that file spends replaced the
/// fewer ways a run can end with it still replaced.
/// Position 23 arrived with task T-10-1 and stands between 22 and 24: it is a Notepad
/// position like its neighbours, and it stays ahead of 17 for the same reason everything
/// does — 17 rewrites the user's `config.toml` and goes last.
const RUNNABLE: [u8; 18] = [
    1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 14, 15, 16, 22, 23, 24, 17,
];

/// Which positions to run: everything the task lists, or what `--positions` names.
///
/// ⚠ **The list from the command line is not checked against [`RUNNABLE`]** — a number outside it
/// is passed through, reaches the `other` arm of [`full_run`] and comes back as a
/// `Row::position_without_a_scenario`. Filtering it here instead would put the miss back where the
/// audit found it: outside the report.
fn selected_positions(arguments: &[String]) -> Vec<u8> {
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

    RUNNABLE.to_vec()
}

/// The hotkey the running product answers to — read from its own configuration, not assumed.
fn hotkey_vk() -> u16 {
    let path = lang_switcher::settings::default_config_path();

    let name = path
        .map(|path| lang_switcher::settings::read_or_default(&path).0.hotkey.key)
        .unwrap_or_else(|| "Pause".to_owned());

    lang_switcher::hook::vk_from_name(&name).unwrap_or(lang_switcher::hook::DEFAULT_HOTKEY_VK)
}

/// Whether the user's configuration leaves the selection path of §4.7 switched on — read the
/// way [`hotkey_vk`] reads the hotkey: from the product's own `config.toml`, never assumed.
///
/// ⚠ **Н-Э13-34 is why the bench has to look.** With `[selection] enabled = false` the product
/// refuses the path at the first gate of FR-65 (`wants_selection_path`) and never sends
/// `Ctrl+C`; position 15 then waits ten seconds on the clipboard sequence number and reports
/// «приложение не ответило на Ctrl+C» — a sentence blaming the application in a run where the
/// product did exactly what §7 told it to. Three deliveries of different epochs were measured
/// to `fail` identically before the real cause — this one setting — was found. The position is
/// not failing then; it is **unverifiable in this configuration**, and `position_15` answers
/// with the third verdict of Р-30 instead of running the scenario blindly.
fn selection_path_enabled() -> bool {
    lang_switcher::settings::default_config_path()
        .map(|path| selection_enabled_at(&path))
        .unwrap_or(true)
}

/// `[selection] enabled` of the configuration at `path` — default `true`, as §7 has it.
///
/// The product's own parser and nothing of this bench's: a missing file, a missing section and
/// a missing key all come back `true` through the serde defaults of
/// [`lang_switcher::settings::Config`], which is the same road the running product took when it
/// decided whether to answer the hotkey with a `Ctrl+C` at all.
fn selection_enabled_at(path: &std::path::Path) -> bool {
    lang_switcher::settings::read_or_default(path)
        .0
        .selection
        .enabled
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

#[cfg(test)]
mod tests {
    use super::*;

    use report::{Assertion, Report, Row, Verdict};

    /// What `full_run` does to the report for one position of `--positions` — the two lines the
    /// loop above runs before anything of the scenario does, and nothing else. Keeping them in a
    /// function is what lets criterion 7 of task T-13-19 be a unit test instead of a live run of
    /// the bench, which this task is forbidden to make.
    fn record(report: &mut Report, wanted: &[u8], position: u8, failure: &str) {
        if !wanted.contains(&position) {
            return; // never asked for: no launch, no row — it did not run
        }
        report.push(Row::infrastructure_failure(position, failure.to_owned()));
    }

    #[test]
    fn positions_come_from_the_switch_and_default_to_the_whole_list() {
        let arguments = ["--positions".to_owned(), "3, 8 ,14".to_owned()];
        assert_eq!(selected_positions(&arguments), vec![3, 8, 14]);
        assert_eq!(selected_positions(&[]).len(), 18);
    }

    /// **Task T-13-19, criterion 7.** Ask for position 3 only, and let the product fail to start.
    /// Position 3 is a `fail` row of the block; position 8, which nobody asked for, has no row.
    /// Neither the line nor the counter of the two coincides.
    #[test]
    fn an_unrequested_position_is_not_the_same_as_one_that_could_not_run() {
        let wanted = selected_positions(&["--positions".to_owned(), "3".to_owned()]);
        assert_eq!(wanted, vec![3]);

        let mut report = Report::default();
        record(&mut report, &wanted, 3, "продукт не запустился: os error 2");
        record(&mut report, &wanted, 8, "продукт не запустился: os error 2");

        let rendered = report.render();
        let block: Vec<&str> = rendered
            .lines()
            .skip(2)
            .take_while(|line| !line.is_empty())
            .collect();

        assert_eq!(block.len(), 1, "одна строка на одну запрошенную позицию");
        assert!(
            block[0].starts_with("3\t"),
            "строка позиции 3: {:?}",
            block[0]
        );
        assert!(
            block[0].contains("\tfail\t"),
            "вердикт fail: {:?}",
            block[0]
        );
        assert!(
            !rendered.lines().any(|line| line.starts_with("8\t")),
            "незапрошенная позиция 8 в отчёте отсутствует"
        );

        assert_eq!(
            report.count(Verdict::Fail),
            1,
            "считается только запрошенная"
        );
        assert_eq!(report.count(Verdict::Pending), 0);
        assert_ne!(report.exit_code(), 0);
    }

    /// The same selection, with the product starting fine, leaves the block empty: the rows of a
    /// position that ran come from `scenarios`, and an empty block is what "nothing failed on the
    /// way in" looks like. Without this the test above would pass on a bench that wrote a row for
    /// every position it was asked for, failed or not.
    #[test]
    fn a_position_that_started_leaves_no_infrastructure_row() {
        let wanted = selected_positions(&["--positions".to_owned(), "3,8".to_owned()]);
        let report = Report::default();

        assert_eq!(wanted, vec![3, 8]);
        assert_eq!(report.count(Verdict::Fail), 0);
        assert_eq!(report.exit_code(), 0);
    }

    /// The list the loop reads to tell a position of §11.6 from one the bench simply does not
    /// know — the same list `pending_positions` writes, so the test states what it is rather than
    /// repeating it.
    fn handed_to_a_person() -> Vec<u8> {
        scenarios::pending_positions()
            .iter()
            .map(|row| row.position)
            .collect()
    }

    /// **Task T-13-19, addition ordered by the controller.** `--positions 99` names a number the
    /// `match` of `full_run` has no arm for. It has to come back as exactly one `fail` row of the
    /// block, move the counter, and make the process return non-zero.
    #[test]
    fn a_position_the_bench_does_not_know_is_a_fail_row_and_a_non_zero_exit_code() {
        let wanted = selected_positions(&["--positions".to_owned(), "99".to_owned()]);
        assert_eq!(wanted, vec![99]);
        assert!(
            !RUNNABLE.contains(&99),
            "99 не входит в перечень выполняемых позиций"
        );

        let mut report = Report::default();
        for position in &wanted {
            report.extend(rows_of_a_position(None, *position, &handed_to_a_person()));
        }

        let rendered = report.render();
        let block: Vec<&str> = rendered
            .lines()
            .skip(2)
            .take_while(|line| !line.is_empty())
            .collect();

        assert_eq!(block.len(), 1, "ровно одна строка");
        let fields: Vec<&str> = block[0].split('\t').collect();
        assert_eq!(fields[0], "99", "позиция");
        assert_eq!(fields[1], "вне перечня стенда", "приложение");
        assert_eq!(fields[2], "сценарий позиции", "утверждение");
        assert_eq!(fields[3], "fail", "вердикт");
        assert_eq!(
            fields[4], "стенд не выполняет эту позицию: сценария для неё нет",
            "фактическое"
        );
        assert_eq!(fields[5], "позиция выполняется стендом", "ожидаемое");
        assert_eq!(fields[6], "", "владелец — как у прочих не-pending строк");
        assert!(
            fields[7].contains("запрошена"),
            "примечание: {:?}",
            fields[7]
        );

        assert_eq!(report.count(Verdict::Fail), 1, "счётчик fail вырос");
        assert_eq!(report.exit_code(), 1, "код возврата 1");
    }

    /// The safety net: **no** position of the default run produces that row. Every number of
    /// `RUNNABLE` has an arm of the `match`, so the fallback is never consulted for it — and were
    /// it consulted by mistake, none of them is a position of §11.6 either, so the row would
    /// appear and this test would say so.
    #[test]
    fn no_position_of_the_default_run_produces_a_row_of_its_own() {
        let handed = handed_to_a_person();

        for position in selected_positions(&[]) {
            assert!(
                RUNNABLE.contains(&position),
                "позиция {position} по умолчанию обязана быть в перечне выполняемых"
            );
            assert!(
                !handed.contains(&position),
                "позиция {position} не может быть одновременно выполняемой и отданной человеку"
            );
        }

        // And the ordinary path — a scenario answered — never reaches the fallback at all: the
        // rows of the position are the scenario's own, untouched.
        let scenario = vec![Row::new(
            3,
            "Блокнот",
            Assertion::Text,
            Verdict::Pass,
            "привет",
            "привет",
        )];
        let rows = rows_of_a_position(Some(scenario), 3, &handed);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].verdict, Verdict::Pass);
        assert_ne!(rows[0].application, "вне перечня стенда");
    }

    /// A position §11.6 hands to a live person has no arm either, and it is **not** a miss:
    /// `pending_positions` already carries it. A `fail` beside that `pending` would be a false
    /// finding about the bench.
    #[test]
    fn a_position_of_the_acceptance_session_gets_no_fail_beside_its_pending() {
        let handed = handed_to_a_person();
        assert_eq!(handed, vec![9, 13, 18, 19, 20, 21], "перечень §11.6");

        for position in &handed {
            assert!(
                rows_of_a_position(None, *position, &handed).is_empty(),
                "позиция {position} уже в отчёте как pending"
            );
        }

        // What such a run really produces: the pending row, and Р-30's refusal to call it a
        // success — the position is in the block, not missing from it.
        let mut report = Report::default();
        report.extend(rows_of_a_position(None, 9, &handed));
        report.extend(scenarios::pending_positions());
        assert_eq!(report.count(Verdict::Fail), 0);
        assert!(report.count(Verdict::Pending) > 0);
        assert_eq!(report.exit_code(), 1);
        assert!(report.render().lines().any(|line| line.starts_with("9\t")));
    }

    /// The three rows the bench can leave for a position it did not check must not read alike.
    #[test]
    fn the_two_causes_of_an_unchecked_position_are_not_the_same_row() {
        let infrastructure = Row::infrastructure_failure(99, "продукт не запустился: os error 2");
        let unknown = Row::position_without_a_scenario(99);

        assert_eq!(infrastructure.verdict, unknown.verdict, "оба — fail");
        assert_ne!(infrastructure.application, unknown.application);
        assert_ne!(
            infrastructure.assertion.as_str(),
            unknown.assertion.as_str()
        );
        assert_ne!(infrastructure.actual, unknown.actual);
        assert_ne!(infrastructure.expected, unknown.expected);
        assert_ne!(infrastructure.note, unknown.note);
    }

    /// **Task Т-14-1, the remainder of Н-Э13-34.** `[selection] enabled` is read the way the
    /// hotkey is — from a file, through the product's own parser, with the default of §7 —
    /// and never assumed by this bench.
    ///
    /// Four states, each a file of its own: the flag off, the flag on, a file without the
    /// section, and no file at all. The last two must both come back `true`, because that is
    /// what the running product does with them — and a bench that answered differently would
    /// be second-guessing the very configuration it claims to be reading.
    ///
    /// No product and no application anywhere near this: the files live under `%TEMP%`, in a
    /// directory of this test's own, and the user's real `config.toml` is never touched.
    #[test]
    fn the_selection_flag_is_read_from_the_toml_with_the_default_of_section_7() {
        let dir = std::env::temp_dir().join("langsw-e2e-t14-1-selection-flag");
        std::fs::create_dir_all(&dir).expect("каталог теста создаётся");

        let case = |name: &str, text: &str| -> bool {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("файл теста записывается");
            let read = selection_enabled_at(&path);
            let _ = std::fs::remove_file(&path);
            read
        };

        assert!(
            !case("off.toml", "[selection]\nenabled = false\n"),
            "выключенный путь читается как false — состояние Н-Э13-34"
        );
        assert!(
            case("on.toml", "[selection]\nenabled = true\n"),
            "включённый путь читается как true"
        );
        assert!(
            case("no-section.toml", "[hotkey]\nkey = \"Pause\"\n"),
            "файл без секции [selection] даёт умолчание §7 — true"
        );
        assert!(
            selection_enabled_at(&dir.join("нет-такого-файла.toml")),
            "отсутствие файла даёт то же умолчание — true, как у продукта при первом запуске"
        );

        let _ = std::fs::remove_dir(&dir);
    }
}
