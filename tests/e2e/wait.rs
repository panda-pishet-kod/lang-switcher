//! The one place in the bench that sleeps **waiting for a result**, and the ten that sleep for
//! reasons of their own.
//!
//! Requirement 1 of §11.5 of SPEC forbids deciding that a window is ready by waiting a fixed
//! amount of time. The distinction this module exists to make plain is between two things a
//! grep for `sleep` cannot tell apart on its own:
//!
//! * a **fixed delay** — "wait three seconds, then assume Word is up". Forbidden. It is either
//!   slower than it needs to be or wrong, and which of the two it is changes between runs.
//! * a **poll interval** — how often a condition that has already been asked about is asked
//!   again. Unavoidable in any wait that is not event-driven, and invisible in the result: the
//!   wait ends at the first poll that succeeds, so the observable behaviour is "as soon as the
//!   condition holds", never "after N milliseconds".
//!
//! Every wait **for a result** in the bench is expressed as [`until`] over a condition, and the
//! sleep on the single line inside it is the poll interval of that wait. It is not, however, the
//! only `std::thread::sleep` in `tests\e2e\`: there are **eleven** call sites, in three files. The
//! other ten are intervals belonging to a scenario or to a measurement — an interval whose
//! elapsing *is* the stimulus, or *is* the quantity under study — and not one of them ends in a
//! decision that something is ready. A grep for `sleep` therefore has eleven hits, and this is
//! what each of them is:
//!
//! * [`until`] (`wait.rs`) — **the poll interval**, and the only sleep a verdict ever waits
//!   behind: the wait ends at the first successful poll, so its length is invisible in the
//!   result.
//! * `console_park` (`e2e.rs`) — the idle loop of the `--console-park` helper mode after
//!   `CONIN$` failed to open: there is nothing left to read, the process only stays alive until
//!   its window is closed, and the interval bounds nothing at all.
//! * `idle_watch` (`scenarios.rs`) — the spacing between samples of the rest measurement: every
//!   number it prints is a sample taken at a stated moment, never a «ready».
//! * `slow_press` (`scenarios.rs`) — the 200 ms hesitation between keystrokes that the
//!   acceptance session described; it is the **stimulus** of the position, so shortening it
//!   would change what is being measured rather than speed it up.
//! * `empty_press` (`scenarios.rs`) — two seconds for the state to settle after a press on an
//!   empty buffer: nothing is converted and the screen does not change, so there is no result a
//!   condition could ask about — only the state the path leaves behind, read afterwards.
//! * `console_empty_press` (`scenarios.rs`) — the same two seconds for the same reason on the
//!   console, where П-3 says the press must produce nothing at all.
//! * `experiment_explorer` (`scenarios.rs`) — a four-second dwell that is the scenario's own
//!   «отошёл на некоторое время»: what is given time is the shell's own settling, and there is
//!   no condition on our side that says it is over.
//! * `run_sequence` (`scenarios.rs`) — the same four-second dwell, for the same reason, in the
//!   sequence that drives `Win+E`.
//! * `race_pause` (`scenarios.rs`) — the pause `D` of the race experiment: it is the
//!   **independent variable itself**, and waiting on a condition instead would destroy the
//!   measurement rather than improve it.
//! * `hand_pause` (`scenarios.rs`) — task T-95-3: the interval of a hand in the experiment of the
//!   double press of `Shift` — between two taps, two `Alt+Shift`, `Shift`, a click and the release.
//!   The rule of the pair is a rule of times (`hook::Taps`), so the interval is the **stimulus** —
//!   120 ms is a pair, 650 ms is not — and no condition can stand in for it.
//! * `quiet_round` (`scenarios.rs`) — task T-95-3: the 1.2 s after a round that must convert
//!   **nothing** — as in `empty_press`, nothing changes on the screen, so there is no result a
//!   condition could ask about, only the counters read afterwards.
//!
//! ⚠ **The list is checked, not asserted.** It used to read «exactly once», which had been
//! false for eight of the nine, and a reader who believes such a sentence never goes to look —
//! a false invariant in code is worse than no invariant at all. The test
//! `the_bench_sleeps_only_in_the_eleven_places_this_module_lists` at the foot of this file
//! re-derives the list from the source of `tests\e2e\` on every `cargo test --features testing`:
//! a twelfth call site, or one of these eleven moving into another function, turns it red.

use std::time::{Duration, Instant};

/// How often a pending condition is re-examined.
///
/// Small enough that the granularity it adds to a measurement is below the reaction time of
/// every application the matrix drives, and large enough that a minute-long wait costs a
/// negligible number of cross-process UI Automation round trips.
pub const POLL: Duration = Duration::from_millis(50);

/// Waits until `probe` produces a value, and answers with the first one it produced.
///
/// `None` means the timeout elapsed with `probe` never succeeding — a fact the caller turns
/// into a verdict, never into an assumption that the thing happened anyway.
///
/// `probe` is asked **before** the first sleep, so a condition that already holds costs
/// nothing at all.
pub fn until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(value) = probe() {
            return Some(value);
        }

        if Instant::now() >= deadline {
            return None;
        }

        // The poll interval, and the only sleep in the bench a verdict waits behind. Not a
        // fixed delay: see the module comment, which lists the other eight and what they are.
        std::thread::sleep(POLL);
    }
}

/// [`until`] for a condition that yields no value, only a yes or a no.
pub fn until_true(timeout: Duration, mut probe: impl FnMut() -> bool) -> bool {
    until(timeout, || probe().then_some(())).is_some()
}

/// How long to wait for an application window to come up.
///
/// Word is the slow one; the rest are up in a fraction of this. Because the wait ends at the
/// first successful poll, a generous bound costs nothing on the applications that are quick.
pub const WINDOW_TIMEOUT: Duration = Duration::from_secs(60);

/// How long to wait for typed text to appear in the element that was read back.
pub const TEXT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for an application to disappear after it was asked to close.
pub const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------------------
// The self-audits of the bench, as tests rather than as a grep somebody once ran
// ---------------------------------------------------------------------------------------

/// Sweeps over the source of `tests\e2e\`, shared by the two self-audits of the bench.
///
/// Two modules state an invariant about their own source: this one about `std::thread::sleep`,
/// and `input` about `SendInput`. Both sentences were true when they were written and both had
/// quietly stopped being true — which is why this module exists instead of a note asking the
/// next reader to run a grep.
///
/// ⚠ **Two things a sweep of this kind must get right, both of them learned in this tree.**
///
/// * **Line endings.** `.gitattributes` keeps the working tree in CRLF while `cargo fmt` writes
///   LF, so one commit is one text after a checkout and another after a format. A sweep whose
///   needle or whose split contains a newline answers differently for the two — this tree has
///   carried exactly such a sweep, green only because it never matched anything. Everything
///   here goes through `str::lines`, which splits on `\n` and drops a trailing `\r`, so a line
///   is the same line either way, and `the_sweep_gives_the_same_answer_at_lf_and_at_crlf` proves
///   it by running both forms of the real files rather than by arguing it.
/// * **Prose against code.** The bench discusses sleeping and sending at length in its own
///   comments, and a sweep that counted sentences would be one nobody could keep green. Comment
///   lines are dropped, and every needle carries its opening parenthesis.
///
/// Nothing here falls back to a guess: a directory that will not list, a file that will not read
/// and a call site with no function around it are all panics. A sweep that quietly reads nothing
/// is the one outcome that would leave these tests worse than absent.
#[cfg(test)]
pub(crate) mod sweep {
    use std::path::PathBuf;

    /// A call of `std::thread::sleep`.
    ///
    /// Assembled from pieces on purpose: spelled out whole, this very line would be a hit and
    /// the sweep would find its own source.
    pub(crate) const SLEEP_CALL: &str = concat!("thread::", "sleep", "(");

    /// A call of `SendInput` — assembled for the same reason as [`SLEEP_CALL`].
    pub(crate) const SEND_INPUT_CALL: &str = concat!("Send", "Input", "(");

    /// An import of `std::thread` in any of its forms, which the bench does not have.
    ///
    /// That absence is what makes [`SLEEP_CALL`] the only spelling a sleep can have here: an
    /// import would allow a bare `sleep(POLL)`, and the sweep would not see it.
    pub(crate) const THREAD_IMPORT: &str = concat!("use std::", "thread");

    /// Qualifiers a function header may carry in front of `fn`.
    const QUALIFIERS: [&str; 6] = [
        "pub",
        "pub(crate)",
        "pub(super)",
        "const",
        "async",
        "unsafe",
    ];

    /// Every `.rs` file of `tests\e2e\`, as (file name, text).
    pub(crate) fn sources() -> Vec<(String, String)> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("e2e");

        let listing = std::fs::read_dir(&root)
            .unwrap_or_else(|error| panic!("{} must list: {error}", root.display()));

        let mut out = Vec::new();

        for entry in listing {
            let path = entry.expect("a readable directory entry").path();

            if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("a source file of the bench has a name")
                    .to_owned();
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("{} must read: {error}", path.display()));

                out.push((name, text));
            }
        }

        assert!(
            out.len() >= 14,
            "the whole bench was read and not a truncated listing: {} files",
            out.len()
        );

        out
    }

    /// Line numbers, counted from one, of the lines of `text` that **call** `needle`.
    pub(crate) fn call_lines(text: &str, needle: &str) -> Vec<usize> {
        text.lines()
            .enumerate()
            .filter(|(_, line)| {
                let code = line.trim_start();

                !code.starts_with("//") && code.contains(needle)
            })
            .map(|(index, _)| index + 1)
            .collect()
    }

    /// The name of the function that `line` of `text` is inside.
    ///
    /// The nearest function header at or above the line, whatever its indentation, so that a
    /// call inside a closure answers with the function the closure lives in. **Panics** when
    /// there is none: a call site outside every function is a fact this sweep must not paper
    /// over with a file name or an empty string.
    pub(crate) fn function_at(text: &str, line: usize) -> String {
        let above: Vec<&str> = text.lines().take(line).collect();

        above
            .iter()
            .rev()
            .find_map(|candidate| header_name(candidate))
            .unwrap_or_else(|| panic!("line {line} is inside some function"))
            .to_owned()
    }

    /// The name `line` declares if `line` is a function header, `None` otherwise.
    ///
    /// Qualifiers are stripped word by word, so every form in [`QUALIFIERS`] is recognised. A
    /// header spelled in some way this does not know makes [`function_at`] answer with an
    /// earlier function — which shows up at once as a wrong name in an inventory below, never as
    /// a silent pass.
    fn header_name(line: &str) -> Option<&str> {
        let mut rest = line.trim_start();

        loop {
            if let Some(tail) = rest.strip_prefix("fn ") {
                let end = tail.find(|c: char| !c.is_alphanumeric() && c != '_')?;

                return Some(&tail[..end]);
            }

            let (word, tail) = rest.split_once(' ')?;

            if !QUALIFIERS.contains(&word) {
                return None;
            }

            rest = tail;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sweep;

    /// The eleven call sites of `std::thread::sleep`: the file, and the function each is inside.
    ///
    /// One entry per bullet of the module comment, in the same order. Changing one of the two
    /// without the other is exactly the drift this table exists to stop. Nine until task T-95-3,
    /// which brought the two intervals of the double press of `Shift`.
    const SLEEPS: [(&str, &str); 11] = [
        ("wait.rs", "until"),
        ("e2e.rs", "console_park"),
        ("scenarios.rs", "idle_watch"),
        ("scenarios.rs", "slow_press"),
        ("scenarios.rs", "empty_press"),
        ("scenarios.rs", "console_empty_press"),
        ("scenarios.rs", "experiment_explorer"),
        ("scenarios.rs", "run_sequence"),
        ("scenarios.rs", "race_pause"),
        ("scenarios.rs", "hand_pause"),
        ("scenarios.rs", "quiet_round"),
    ];

    /// **Requirement 1 of §11.5, as a test rather than as a sentence about a grep.**
    ///
    /// The inventory in the module comment is re-derived from the source every run, so a
    /// twelfth call site or an eleventh that moved makes the comment red instead of stale.
    #[test]
    fn the_bench_sleeps_only_in_the_eleven_places_this_module_lists() {
        let mut found: Vec<(String, String)> = Vec::new();

        for (name, text) in sweep::sources() {
            assert!(
                sweep::call_lines(&text, sweep::THREAD_IMPORT).is_empty(),
                "{name}: the bench imports no part of `std::thread`, so that every sleep is \
                 written in full and this sweep cannot miss one"
            );

            for line in sweep::call_lines(&text, sweep::SLEEP_CALL) {
                found.push((name.clone(), sweep::function_at(&text, line)));
            }
        }

        let mut expected: Vec<(String, String)> = SLEEPS
            .iter()
            .map(|(file, owner)| ((*file).to_owned(), (*owner).to_owned()))
            .collect();

        found.sort();
        expected.sort();

        assert_eq!(
            found, expected,
            "the inventory at the top of `wait.rs` no longer matches the source. A new interval \
             needs a bullet there saying what guarantees it and a line here; a wait for a result \
             needs neither, it needs `until`."
        );
    }

    /// ⚠ **Proof, by running it, that the sweep does not depend on the line endings.**
    ///
    /// `.gitattributes` holds this tree in CRLF and `cargo fmt` rewrites files in LF, so both
    /// forms are things a working copy really is. Every needle is swept over each source twice —
    /// once with `\r\n` collapsed to `\n`, once with `\n` expanded back to `\r\n` — and the two
    /// answers must agree on the line numbers and on the function names. The count at the end
    /// keeps the comparison from passing on two empty answers.
    #[test]
    fn the_sweep_gives_the_same_answer_at_lf_and_at_crlf() {
        let needles = [
            sweep::SLEEP_CALL,
            sweep::SEND_INPUT_CALL,
            sweep::THREAD_IMPORT,
        ];
        let mut hits = 0;

        for (name, text) in sweep::sources() {
            let lf = text.replace("\r\n", "\n");
            let crlf = lf.replace('\n', "\r\n");

            for needle in needles {
                let at_lf = sweep::call_lines(&lf, needle);
                let at_crlf = sweep::call_lines(&crlf, needle);

                assert_eq!(
                    at_lf, at_crlf,
                    "{name}: the lines of `{needle}`, LF against CRLF"
                );

                for line in &at_lf {
                    assert_eq!(
                        sweep::function_at(&lf, *line),
                        sweep::function_at(&crlf, *line),
                        "{name}: the function around line {line}, LF against CRLF"
                    );
                }

                hits += at_lf.len();
            }
        }

        assert!(
            hits > 0,
            "the sweep found something to compare: two empty answers would agree for the wrong \
             reason"
        );
    }

    /// A call is not a sentence about a call, and a `\r` is not part of the line.
    ///
    /// The fixture is written in both line endings and both must yield the same single call
    /// site in the same function. Its needle is deliberately none of the real ones, so that the
    /// text of this test is not itself a hit of the sweeps above.
    #[test]
    fn the_sweep_separates_a_call_from_a_sentence_about_one() {
        const LF: &str = "// nap::doze( is only talked about here\n\
                          fn napping() {\n\
                          let hold = || {\n\
                          nap::doze(POLL);\n\
                          };\n\
                          }\n";

        let crlf = LF.replace('\n', "\r\n");

        for (form, text) in [("LF", LF.to_owned()), ("CRLF", crlf)] {
            let lines = sweep::call_lines(&text, "nap::doze(");

            assert_eq!(
                lines,
                [4],
                "{form}: the comment on line 1 is not a call site"
            );
            assert_eq!(
                sweep::function_at(&text, 4),
                "napping",
                "{form}: a call inside a closure belongs to the function around it"
            );
        }
    }
}
