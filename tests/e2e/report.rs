//! Verdicts and the machine-readable report — requirement 6 of §11.5 and decision Р-30.
//!
//! # Three verdicts, and why `pending` is not a shade of `pass`
//!
//! The scenario of §11.3 makes **two** assertions per position: the text became `привет`, and
//! the active layout of the window became RU. **Both are real verdicts today.** The layout row
//! was `pending` while the switch of §4.6 did not exist; task T-05-1 wrote the chain of FR-50 to
//! FR-52 (commit `c8e463f`) and task T-05-2 gave it the target of §4.4 (commit `7932eb2`), so
//! the bench reads the window's layout and says `pass` or `fail` by what it reads.
//!
//! `pending` is undiminished by that and keeps the two jobs decision Р-30 gave it: the positions
//! marked **П**, which §11.6 hands to a person because the bench may not drive them at all, and
//! the requirements no task has written yet. "Cannot be tested" is a third outcome and not a
//! quiet success — Р-30 requires it counted separately and forbids a run with a non-zero
//! `pending` from calling itself successful.
//!
//! Every [`Pending`] carries the task that owns the missing requirement, so the summary says
//! not only that something is untested but who will make it testable.
//!
//! # SEC-01 and SEC-07, and why `ghbdtn` may be printed here
//!
//! SEC-01 and SEC-07 forbid the **product** from putting a character or a scan code into its
//! own log. Nothing in this file is the product's log. The strings below are the bench's own
//! test vector — a constant chosen by whoever wrote the matrix, typed by the bench, and
//! compared by the bench — and printing an expected value is what a test report is for. The
//! difference is the origin: `ghbdtn` here was never intercepted from anybody's keyboard, and
//! the bench never reads the product's buffer contents, only its length through SEC-04a.

use std::fmt;

/// The three outcomes of decision Р-30.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Checked, and it holds.
    Pass,
    /// Checked, and it does not hold.
    Fail,
    /// Not checkable: the requirement is not implemented yet.
    Pending,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Pending => "pending",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which of the two halves of the §11.3 scenario a row is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assertion {
    /// The text became `привет`.
    Text,
    /// The active layout of the window became RU.
    Layout,
    /// Something a particular position asserts instead of, or beside, the two above.
    Other(&'static str),
}

impl Assertion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "текст",
            Self::Layout => "раскладка",
            Self::Other(name) => name,
        }
    }
}

/// The `утверждение` of a row left by [`Row::infrastructure_failure`]: the position never got as
/// far as the text or the layout, so what it asserts is the one thing that did not happen.
const READINESS: &str = "готовность продукта";

/// The `ожидаемое` of the same row.
const READY: &str = "готовность";

/// The `приложение` of the same row. The position was stopped before its application was ever
/// opened, so naming one would invent a fact; the column says where the failure was instead.
const NO_APPLICATION: &str = "инфраструктура стенда";

/// The `примечание` of the same row — the one field that says in words what the presence of the
/// row says in structure: this position **was** asked for. A position nobody asked for has no row
/// at all, and the two must never read alike (task T-13-19).
const NOT_RUN: &str =
    "позиция была запрошена и не выполнялась: сбой инфраструктуры стенда до сценария";

/// The `утверждение` of a row left by [`Row::position_without_a_scenario`]. The bench cannot know
/// what such a position asserts — it has no scenario for it — so what the row asserts is the
/// existence of the scenario itself.
const SCENARIO: &str = "сценарий позиции";

/// The `ожидаемое` of the same row.
const SCENARIO_RUN: &str = "позиция выполняется стендом";

/// The `приложение` of the same row. Naming an application would again invent a fact: with no
/// scenario the bench does not know which application the position means. Deliberately **not**
/// [`NO_APPLICATION`] — the two causes must not read alike.
const OUTSIDE_THE_LIST: &str = "вне перечня стенда";

/// The `фактическое` of the same row.
const NO_SCENARIO: &str = "стенд не выполняет эту позицию: сценария для неё нет";

/// The `примечание` of the same row — as with [`NOT_RUN`], words that say what the presence of the
/// row says in structure.
const ASKED_AND_UNKNOWN: &str = "позиция была запрошена ключом --positions и не выполнялась: \
     у стенда нет для неё сценария";

/// One line of the report.
#[derive(Debug, Clone)]
pub struct Row {
    /// Position of the matrix of §11.3.
    pub position: u8,
    /// The application the position names.
    pub application: String,
    pub assertion: Assertion,
    pub verdict: Verdict,
    /// What was observed, verbatim.
    pub actual: String,
    /// What the matrix expects.
    pub expected: String,
    /// For `pending`: the task that owns the missing requirement, or `П` for the acceptance
    /// session of §11.6.
    pub owner: Option<String>,
    /// Anything a human reading the row needs beside the four fields — which pattern answered,
    /// which window was found, why a send was refused.
    pub note: String,
}

impl Row {
    pub fn new(
        position: u8,
        application: impl Into<String>,
        assertion: Assertion,
        verdict: Verdict,
        actual: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            position,
            application: application.into(),
            assertion,
            verdict,
            actual: actual.into(),
            expected: expected.into(),
            owner: None,
            note: String::new(),
        }
    }

    /// A `pending` row with the task that owns it.
    pub fn pending(
        position: u8,
        application: impl Into<String>,
        assertion: Assertion,
        owner: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            position,
            application: application.into(),
            assertion,
            verdict: Verdict::Pending,
            actual: "не проверялось".to_owned(),
            expected: expected.into(),
            owner: Some(owner.into()),
            note: String::new(),
        }
    }

    /// The row of a position that was **asked for and never ran**: the product did not start, or
    /// it started and never answered on the SEC-04a channel.
    ///
    /// # Why `fail` and not `pending`, and why a row at all
    ///
    /// Requirement 6 of §11.5 wants position, verdict, actual and expected for the run. Before
    /// task T-13-19 an infrastructure failure produced none of them: `full_run` printed to stderr
    /// and moved on, so the position was simply **absent** from the TSV block and the counters
    /// never moved. A consumer that parses the block and checks `fail == 0` — the ordinary shape
    /// of a CI gate — read that run as a success while a position of the matrix had not been
    /// checked at all.
    ///
    /// `pending` would be the wrong verdict for it. Р-30 gave `pending` one meaning: the
    /// requirement cannot be checked **yet** because no task has written it, and the row names the
    /// task that owes it. Nothing is owed here — the requirement exists, the bench was asked to
    /// check it, and it could not. Without evidence that the requirement holds, the honest verdict
    /// is `fail`.
    ///
    /// # SEC-01 and SEC-07
    ///
    /// `actual` is a sentence about the **bench's infrastructure** — a spawn error, a silent
    /// channel — and never about what was typed or what a window held. Callers pass the reason,
    /// and the reason is the product's start-up, not anybody's data.
    pub fn infrastructure_failure(position: u8, actual: impl Into<String>) -> Self {
        Self {
            position,
            application: NO_APPLICATION.to_owned(),
            assertion: Assertion::Other(READINESS),
            verdict: Verdict::Fail,
            actual: actual.into(),
            expected: READY.to_owned(),
            // As every non-`pending` row: the `владелец` column belongs to Р-30's owner of a
            // missing requirement, and an infrastructure failure has none.
            owner: None,
            note: NOT_RUN.to_owned(),
        }
    }

    /// The row of a position that was **asked for and has no scenario**: `--positions` named a
    /// number the `match` of `full_run` does not answer.
    ///
    /// # Why this is a third row and not [`Row::infrastructure_failure`]
    ///
    /// The cause is a different one and a reader has to see which. There the bench had a scenario
    /// and the machine denied it one — the product would not start, the channel stayed silent.
    /// Here nothing was denied: the bench simply does not know the position. `приложение`,
    /// `утверждение`, `фактическое` and `примечание` all differ, so neither a parser nor a person
    /// can take one for the other.
    ///
    /// # Why `fail`
    ///
    /// The same reasoning as for [`Row::infrastructure_failure`], and it is the reasoning of the
    /// controller who ordered this row: somebody asked for an assertion of §11.3 to be checked and
    /// got no evidence either way. `pending` means what Р-30 gave it — the requirement is not
    /// written yet and a named task owes it — and neither half is true of a position the bench has
    /// no arm for. Silence is what the finding of the audit was about, so the counter has to move:
    /// a consumer checking `fail == 0` must see the miss.
    ///
    /// ⚠ **A position of §11.6 is not this.** Positions handed to a live person — `pending` rows
    /// of `scenarios::pending_positions`, owner `П` — have no `match` arm either, and they are not
    /// a miss: the report already carries them, with the verdict Р-30 gives them, in every run.
    /// A `fail` beside their `pending` would be a false finding about the bench, so `full_run`
    /// keeps them out of this row. See `rows_of_a_position` in `e2e.rs`.
    pub fn position_without_a_scenario(position: u8) -> Self {
        Self {
            position,
            application: OUTSIDE_THE_LIST.to_owned(),
            assertion: Assertion::Other(SCENARIO),
            verdict: Verdict::Fail,
            actual: NO_SCENARIO.to_owned(),
            expected: SCENARIO_RUN.to_owned(),
            owner: None,
            note: ASKED_AND_UNKNOWN.to_owned(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }

    /// The machine-readable form: tab-separated, one row per line, no field containing a tab.
    fn as_tsv(&self) -> String {
        [
            self.position.to_string(),
            clean(&self.application),
            self.assertion.as_str().to_owned(),
            self.verdict.as_str().to_owned(),
            clean(&self.actual),
            clean(&self.expected),
            clean(self.owner.as_deref().unwrap_or("")),
            clean(&self.note),
        ]
        .join("\t")
    }
}

/// Removes what would break the one-row-per-line contract.
fn clean(field: &str) -> String {
    field.replace(['\t', '\r', '\n'], " ")
}

/// Everything the run produced.
#[derive(Default)]
pub struct Report {
    pub rows: Vec<Row>,
}

impl Report {
    pub fn push(&mut self, row: Row) {
        self.rows.push(row);
    }

    pub fn extend(&mut self, rows: impl IntoIterator<Item = Row>) {
        self.rows.extend(rows);
    }

    pub fn count(&self, verdict: Verdict) -> usize {
        self.rows.iter().filter(|r| r.verdict == verdict).count()
    }

    /// The process exit code the run owes its caller: `0` only when Р-30 lets the run call itself
    /// successful, `1` otherwise.
    ///
    /// It lives here, beside the counters it is a function of, so that the rule can be checked by
    /// a unit test instead of only by a run of the bench — and so that a row added on a path that
    /// used to `continue` silently is *provably* enough to make the process fail.
    pub fn exit_code(&self) -> u8 {
        if self.count(Verdict::Fail) == 0 && self.count(Verdict::Pending) == 0 {
            0
        } else {
            1
        }
    }

    /// The three numbers Р-30 requires, and the sentence that follows from them.
    pub fn summary(&self) -> String {
        let pass = self.count(Verdict::Pass);
        let fail = self.count(Verdict::Fail);
        let pending = self.count(Verdict::Pending);

        let mut out = String::new();
        out.push_str(&format!(
            "ИТОГО  утверждений: {}   pass={pass}   fail={fail}   pending={pending}\n",
            self.rows.len()
        ));

        // Р-30: a run with a non-zero `pending` has no right to call itself successful, and a
        // bench that printed "успех" beside a pending count would be the thing the decision
        // exists to prevent.
        let line = match (fail, pending) {
            (0, 0) => "ПРОГОН УСПЕШЕН: все утверждения проверены и выполнены",
            (0, _) => {
                "ПРОГОН НЕ ЯВЛЯЕТСЯ УСПЕШНЫМ: есть pending — проверка невозможна до задач-владельцев (Р-30)"
            }
            _ => "ПРОГОН НЕ УСПЕШЕН: есть fail",
        };
        out.push_str(line);
        out.push('\n');

        let mut owners: Vec<&str> = self
            .rows
            .iter()
            .filter(|r| r.verdict == Verdict::Pending)
            .filter_map(|r| r.owner.as_deref())
            .collect();
        owners.sort_unstable();
        owners.dedup();
        if !owners.is_empty() {
            out.push_str(&format!("Владельцы pending: {}\n", owners.join(", ")));
        }

        out
    }

    /// The whole report: a machine-readable block, then a human-readable table, then the
    /// summary. Requirement 6 asks for position, verdict, actual and expected; all four are in
    /// the first block, one row per line, tab-separated.
    pub fn render(&self) -> String {
        let mut out = String::new();

        out.push_str("### МАШИНОЧИТАЕМЫЙ ОТЧЁТ (TSV)\n");
        out.push_str("позиция\tприложение\tутверждение\tвердикт\tфактическое\tожидаемое\tвладелец\tпримечание\n");
        for row in &self.rows {
            out.push_str(&row.as_tsv());
            out.push('\n');
        }

        out.push_str("\n### ТО ЖЕ ТАБЛИЦЕЙ\n\n");
        out.push_str(
            "| # | Приложение | Утверждение | Вердикт | Фактическое | Ожидаемое | Владелец |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|\n");
        for row in &self.rows {
            out.push_str(&format!(
                "| {} | {} | {} | **{}** | {} | {} | {} |\n",
                row.position,
                clean(&row.application),
                row.assertion.as_str(),
                row.verdict,
                clean(&truncate(&row.actual)),
                clean(&truncate(&row.expected)),
                clean(row.owner.as_deref().unwrap_or("—")),
            ));
        }

        out.push('\n');
        out.push_str(&self.summary());
        out
    }
}

/// Keeps a table cell readable. The TSV block above always carries the full value.
fn truncate(field: &str) -> String {
    const LIMIT: usize = 70;
    if field.chars().count() <= LIMIT {
        return field.to_owned();
    }
    let head: String = field.chars().take(LIMIT).collect();
    format!("{head}…")
}

/// The machine-readable block of [`Report::render`], without its header line — what a consumer of
/// requirement 6 of §11.5 actually parses.
#[cfg(test)]
fn tsv_rows(report: &Report) -> Vec<Vec<String>> {
    report
        .render()
        .lines()
        .take_while(|line| !line.is_empty())
        .skip(2) // "### МАШИНОЧИТАЕМЫЙ ОТЧЁТ (TSV)" and the column names
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Task T-13-19, criterion 6. An infrastructure failure — the product that did not start —
    /// has to reach the machine-readable block as a `fail` row with all four fields requirement 6
    /// of §11.5 names, and it has to make the process return non-zero on its own.
    #[test]
    fn an_infrastructure_failure_is_a_fail_row_of_the_tsv_and_a_non_zero_exit_code() {
        let mut report = Report::default();
        report.push(Row::infrastructure_failure(
            3,
            "продукт не запустился: не удалось запустить D:\\...\\LangSwitcher.exe: \
             The system cannot find the file specified. (os error 2)",
        ));

        let rows = tsv_rows(&report);
        assert_eq!(rows.len(), 1, "ровно одна строка на несостоявшуюся позицию");

        let row = &rows[0];
        assert_eq!(row.len(), 8, "восемь колонок, как у всякой строки TSV");
        assert_eq!(row[0], "3", "позиция");
        assert_eq!(row[2], "готовность продукта", "утверждение");
        assert_eq!(row[3], "fail", "вердикт");
        assert!(
            row[4].starts_with("продукт не запустился: "),
            "фактическое называет причину: {:?}",
            row[4]
        );
        assert_eq!(row[5], "готовность", "ожидаемое");
        assert_eq!(row[6], "", "владелец — как у прочих не-pending строк");

        assert_eq!(report.count(Verdict::Fail), 1, "счётчик fail сдвинулся");
        assert_eq!(report.count(Verdict::Pass), 0);
        assert_eq!(report.count(Verdict::Pending), 0);
        assert_ne!(report.exit_code(), 0, "прогон не имеет права быть успешным");
        assert!(
            report.summary().contains("fail=1"),
            "сводка называет отказ: {}",
            report.summary()
        );
    }

    /// The silent SEC-04a channel is the second path that used to be a bare `continue`, and it
    /// lands in the block the same way.
    #[test]
    fn a_silent_channel_is_a_fail_row_too() {
        let mut report = Report::default();
        report.push(Row::infrastructure_failure(
            14,
            "канал молчит: продукт не сообщил о готовности через SEC-04a за 30 с",
        ));

        let rows = tsv_rows(&report);
        assert_eq!(rows[0][0], "14");
        assert_eq!(rows[0][3], "fail");
        assert!(rows[0][4].starts_with("канал молчит: "), "{:?}", rows[0][4]);
        assert_eq!(rows[0][5], "готовность");
        assert_ne!(report.exit_code(), 0);
    }

    /// Task T-13-19, criterion 7 — the report half of it. A position the run was never asked for
    /// leaves **no** row and moves **no** counter; a position that was asked for and could not run
    /// leaves a `fail` row and moves the `fail` counter. Neither the line nor the number matches.
    ///
    /// The selection half — that `--positions` really is what decides which of the two a position
    /// gets — is checked beside `selected_positions` in `e2e.rs`.
    #[test]
    fn an_unrequested_position_and_a_failed_one_match_neither_line_nor_counter() {
        // Position 3 was asked for and the product would not start. Position 6 was not asked for.
        let mut asked = Report::default();
        asked.push(Row::infrastructure_failure(
            3,
            "канал молчит: продукт не сообщил о готовности",
        ));

        let never_asked = Report::default();

        let asked_rows = tsv_rows(&asked);
        assert_eq!(asked_rows.len(), 1);
        assert_eq!(asked_rows[0][0], "3");
        assert!(
            tsv_rows(&never_asked).is_empty(),
            "незапрошенная позиция в отчёт не добавляется — она не выполнялась"
        );
        assert!(
            !asked.render().lines().any(|line| line.starts_with("6\t")),
            "позиции 6 в блоке быть не должно"
        );

        // Not the same line.
        assert_ne!(tsv_rows(&asked), tsv_rows(&never_asked));
        // Not the same counter.
        assert_eq!(asked.count(Verdict::Fail), 1);
        assert_eq!(never_asked.count(Verdict::Fail), 0);
        // And not the same exit code, which is what a consumer checking `fail == 0` reads.
        assert_ne!(asked.exit_code(), never_asked.exit_code());

        // The row says in words what its presence says in structure, so a human reading the block
        // cannot mistake the two either.
        assert!(
            asked_rows[0][7].contains("запрошена"),
            "примечание: {:?}",
            asked_rows[0][7]
        );
    }

    /// The one-row-per-line contract of the block survives a spawn error carrying a newline.
    #[test]
    fn an_infrastructure_reason_never_breaks_the_row_per_line_contract() {
        let mut report = Report::default();
        report.push(Row::infrastructure_failure(
            8,
            "продукт не запустился: строка\tс табуляцией\nи переводом строки",
        ));

        let rows = tsv_rows(&report);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 8);
        assert!(!rows[0][4].contains('\t') && !rows[0][4].contains('\n'));
    }

    /// A clean run is still a success, and adding the constructor did not move that line.
    #[test]
    fn a_run_without_findings_still_returns_zero() {
        let mut report = Report::default();
        report.push(Row::new(
            1,
            "Блокнот",
            Assertion::Text,
            Verdict::Pass,
            "привет",
            "привет",
        ));
        assert_eq!(report.exit_code(), 0);
        assert!(report.summary().contains("ПРОГОН УСПЕШЕН"));

        report.push(Row::pending(9, "Блокнот", Assertion::Layout, "П", "RU"));
        assert_ne!(report.exit_code(), 0, "Р-30: pending — не успех");
    }
}
