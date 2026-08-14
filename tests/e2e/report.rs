//! Verdicts and the machine-readable report — requirement 6 of §11.5 and decision Р-30.
//!
//! # Three verdicts, and why `pending` is not a shade of `pass`
//!
//! The scenario of §11.3 makes **two** assertions per position: the text became `привет`, and
//! the active layout of the window became RU. Layout switching is task T-05-1 and does not
//! exist yet, so the second assertion cannot be tested — and "cannot be tested" is a third
//! outcome, not a quiet success. Decision Р-30 names it `pending`, requires it counted
//! separately, and forbids a run with a non-zero `pending` from calling itself successful.
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
