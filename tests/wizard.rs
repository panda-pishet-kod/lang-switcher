//! The wizard «Написать автору» — **FR-104**, task Т-32-8: the text it writes and the shape of
//! the road it walks.
//!
//! What is checkable without a window is checked here, and that is deliberately most of it:
//! [`letters::report::build`] is a pure function of two records, so every branch of an appeal —
//! the two roads, the four boxes, the empty answers — can be walked without a registry, without
//! a clipboard and without a screen. What needs a window is checked by the stand
//! (`scratchpad-Э32\stand32`) and by the eye.

use lang_switcher::letters::report::{Attach, Draft, Facts, Field, Repeat, Trouble, build};
use lang_switcher::letters::{self, Step};
use lang_switcher::settings;

/// The interface strings of the product, so that the labels of an appeal are the real ones.
///
/// The same helper the letters tests use: without it `settings::text` answers with the strings
/// of whatever module ran first, and an appeal would be checked against blanks.
fn with_product_strings() {
    settings::set_ui_language(settings::Language::Ru);
}

/// A draft with something in every field — the one an appeal is fullest from.
fn full_draft() -> Draft {
    Draft {
        trouble: Trouble::WrongResult,
        program: "notepad.exe · Notepad".to_owned(),
        field: Field::Normal,
        layout: "English (United States)".to_owned(),
        hotkey: "Pause".to_owned(),
        expected: "привет".to_owned(),
        got: "ghbdtn".to_owned(),
        repeat: Repeat::Always,
        idea: String::new(),
        helps: String::new(),
        attach: Attach::default(),
    }
}

/// Facts with something in every field.
fn full_facts() -> Facts {
    Facts {
        version: "0.41.0".to_owned(),
        windows: "Windows 11 Pro 26100".to_owned(),
        scale: "100 %".to_owned(),
        monitors: "1".to_owned(),
        layouts: "English (United States), Русский (Россия)".to_owned(),
        settings: "Pause · 0x0409 → 0x0419 · auto".to_owned(),
        journal: "journal started\njournal written".to_owned(),
        journal_entries: 2,
    }
}

// =========================================================================================
// The road
// =========================================================================================

/// **Five steps for a trouble and three for an idea** — FR-104, and the first answer is what
/// decides.
#[test]
fn the_road_of_an_idea_is_shorter_than_the_road_of_a_trouble() {
    assert_eq!(Trouble::WrongResult.steps(), 5);
    assert_eq!(Trouble::NothingHappened.steps(), 5);
    assert_eq!(Trouble::Idea.steps(), 3);

    assert!(Trouble::Idea.is_idea());
    assert!(!Trouble::WrongResult.is_idea());
    assert!(!Trouble::NothingHappened.is_idea());
}

/// **Every step shows something, and no two steps share a slot** — the table
/// [`letters::controls_of`] is what hides the rest of the pool, so a slot in two steps at once
/// would be a control that never goes away.
#[test]
fn no_slot_of_the_pool_belongs_to_two_steps() {
    const STEPS: [Step; 6] = [
        Step::What,
        Step::Where,
        Step::Did,
        Step::Idea,
        Step::Attach,
        Step::Preview,
    ];

    let mut seen: Vec<i32> = Vec::new();

    for step in STEPS {
        let controls = letters::controls_of(step);

        assert!(!controls.is_empty(), "{step:?} shows nothing at all");

        for control in controls {
            assert!(
                !seen.contains(control),
                "{control} is shown by {step:?} and by an earlier step as well"
            );

            seen.push(*control);
        }
    }

    // And the pool is walked whole: every slot the window has belongs to exactly one step.
    assert_eq!(
        seen.len(),
        letters::wizard_slot_count(),
        "every slot of the template belongs to one step"
    );
}

// =========================================================================================
// The text of an appeal
// =========================================================================================

/// **The appeal carries what was filled in and nothing else** — and the empty answers leave no
/// empty lines behind them.
#[test]
fn an_appeal_carries_the_answers_and_leaves_out_the_blanks() {
    with_product_strings();

    let appeal = build(&full_draft(), &full_facts());

    for wanted in [
        "Lang Switcher 0.41.0",
        "notepad.exe · Notepad",
        "English (United States)",
        "привет",
        "ghbdtn",
        "Windows 11 Pro 26100",
        "100 %",
        "journal written",
    ] {
        assert!(
            appeal.contains(wanted),
            "the appeal must carry «{wanted}»:\n{appeal}"
        );
    }

    // Nothing about the idea road is in a trouble's appeal.
    assert!(!appeal.contains("Что предлагаете"));

    // An empty answer leaves no line: the two fields of the idea road are empty here, and so
    // is every «label:» that would have named one.
    for line in appeal.lines() {
        assert!(
            !line.trim_end().ends_with(':'),
            "a label with nothing after it: «{line}»"
        );
    }
}

/// **The idea road writes the idea and not the trouble** — the other half of the same rule.
#[test]
fn an_idea_carries_two_answers_and_no_layout() {
    with_product_strings();

    let draft = Draft {
        trouble: Trouble::Idea,
        idea: "Хочу тёмную тему в мастере".to_owned(),
        helps: "Ночью глаза целее".to_owned(),
        ..full_draft()
    };

    let appeal = build(&draft, &full_facts());

    assert!(appeal.contains("Хочу тёмную тему в мастере"));
    assert!(appeal.contains("Ночью глаза целее"));

    // The three answers of a trouble are not asked on this road and must not be written.
    for unwanted in ["notepad.exe", "привет", "ghbdtn"] {
        assert!(
            !appeal.contains(unwanted),
            "an idea must not carry «{unwanted}»:\n{appeal}"
        );
    }
}

/// **Every box that is off takes its own line out and no other** — the promise the step makes.
#[test]
fn each_box_of_the_attachment_step_answers_for_its_own_line() {
    with_product_strings();

    let facts = full_facts();

    let all = build(&full_draft(), &facts);
    assert!(all.contains(&facts.windows));
    assert!(all.contains(&facts.layouts));
    assert!(all.contains(&facts.settings));
    assert!(all.contains("journal written"));

    let none = build(
        &Draft {
            attach: Attach {
                machine: false,
                layouts: false,
                settings: false,
                journal: false,
            },
            ..full_draft()
        },
        &facts,
    );

    assert!(!none.contains(&facts.windows), "the machine line is gone");
    assert!(!none.contains(&facts.layouts), "the layouts line is gone");
    assert!(!none.contains(&facts.settings), "the settings line is gone");
    assert!(!none.contains("journal written"), "the journal is gone");

    // And what the person wrote is untouched by any of it.
    assert!(none.contains("notepad.exe · Notepad"));
    assert!(none.contains("привет"));

    // One box at a time, so that a line cannot be taken out by the wrong box.
    for (which, needle) in [
        (
            Attach {
                machine: false,
                ..Attach::default()
            },
            facts.windows.as_str(),
        ),
        (
            Attach {
                layouts: false,
                ..Attach::default()
            },
            facts.layouts.as_str(),
        ),
        (
            Attach {
                settings: false,
                ..Attach::default()
            },
            facts.settings.as_str(),
        ),
        (
            Attach {
                journal: false,
                ..Attach::default()
            },
            "journal written",
        ),
    ] {
        let appeal = build(
            &Draft {
                attach: which,
                ..full_draft()
            },
            &facts,
        );

        assert!(
            !appeal.contains(needle),
            "the box that was taken off left «{needle}» in"
        );
    }
}

/// **A machine that answers nothing still writes an appeal** — NFR-13, and the appeal is the
/// person's words with no «unknown» in it.
#[test]
fn an_appeal_survives_a_machine_that_says_nothing_about_itself() {
    with_product_strings();

    let appeal = build(&full_draft(), &Facts::default());

    assert!(appeal.contains("notepad.exe · Notepad"));
    assert!(appeal.contains("привет"));
    assert!(!appeal.contains("··"), "no empty joins:\n{appeal}");
    assert!(
        !appeal.to_lowercase().contains("unknown"),
        "a fact that could not be read leaves no word behind:\n{appeal}"
    );
}

/// **The appeal is written with CRLF** — the one thing all three of its destinations agree on.
///
/// A multi-line `EDIT` shows a lone `\n` as no break at all, `CF_UNICODETEXT` is defined with
/// CRLF, and a file with lone newlines opens in Notepad as one long line. The stand found this
/// in the first screenshot it took of the last step: the whole appeal stood in one run.
#[test]
fn the_appeal_breaks_its_lines_the_way_windows_reads_them() {
    with_product_strings();

    let appeal = build(&full_draft(), &full_facts());

    assert!(appeal.contains("\r\n"), "the appeal breaks lines at all");

    let lone = appeal
        .char_indices()
        .filter(|(index, character)| {
            *character == '\n' && (*index == 0 || appeal.as_bytes()[index - 1] != b'\r')
        })
        .count();

    assert_eq!(lone, 0, "not one lone \\n survives:\n{appeal:?}");

    // And no `\r\r\n` either — the conversion must not double a break that was already right.
    assert!(!appeal.contains("\r\r"), "a break is converted once");
}

/// **A text of several lines is indented under its label** rather than run together with it.
#[test]
fn a_multi_line_answer_keeps_its_lines() {
    with_product_strings();

    let appeal = build(
        &Draft {
            trouble: Trouble::Idea,
            idea: "Первая строка\nВторая строка".to_owned(),
            ..full_draft()
        },
        &full_facts(),
    );

    assert!(
        appeal.contains("\n    Вторая строка"),
        "the second line is indented under the first:\n{appeal}"
    );
}

/// **The three answers of the radio rows reach the appeal** — every value of both closed sets.
#[test]
fn every_value_of_both_radio_rows_is_written_out() {
    with_product_strings();

    for field in [Field::Normal, Field::Password, Field::Unknown] {
        for repeat in [Repeat::Always, Repeat::Sometimes, Repeat::Once] {
            let appeal = build(
                &Draft {
                    field,
                    repeat,
                    ..full_draft()
                },
                &full_facts(),
            );

            let field_word = settings::text(match field {
                Field::Normal => settings::IDS_WIZARD_FIELD_NORMAL,
                Field::Password => settings::IDS_WIZARD_FIELD_PASSWORD,
                Field::Unknown => settings::IDS_WIZARD_FIELD_UNKNOWN,
            });
            let repeat_word = settings::text(match repeat {
                Repeat::Always => settings::IDS_WIZARD_REPEAT_ALWAYS,
                Repeat::Sometimes => settings::IDS_WIZARD_REPEAT_SOMETIMES,
                Repeat::Once => settings::IDS_WIZARD_REPEAT_ONCE,
            });

            assert!(
                appeal.contains(&field_word),
                "{field:?} must be written out as «{field_word}»"
            );
            assert!(
                appeal.contains(&repeat_word),
                "{repeat:?} must be written out as «{repeat_word}»"
            );
        }
    }
}

/// **The name of the saved file carries the day and no more** — FR-104, and the shape does not
/// change with the interface language.
#[test]
fn the_saved_file_is_named_by_the_day() {
    with_product_strings();

    let day = letters::Date::from_ymd(2026, 10, 14).expect("a day");
    let name = letters::report::file_name(day);

    assert!(name.ends_with("-2026-10-14.txt"), "{name}");
    assert!(
        !name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']),
        "a file name and not a path: {name}"
    );

    // The stem follows the interface language, the date never does.
    settings::set_ui_language(settings::Language::En);
    let english = letters::report::file_name(day);
    settings::set_ui_language(settings::Language::Ru);

    assert!(english.ends_with("-2026-10-14.txt"), "{english}");
    assert_ne!(
        english, name,
        "the stem is written in the interface language"
    );
}

/// **«Windows 10 Pro» on a build of Windows 11 is corrected to 11** — the registry value
/// Microsoft left behind, and what the live acceptance of Э32 wrote into a real appeal from a
/// machine running 26100.
#[test]
fn the_name_of_windows_is_corrected_by_its_build_number() {
    use lang_switcher::letters::report::eleven;

    assert_eq!(
        eleven("Windows 10 Pro", "26100"),
        "Windows 11 Pro",
        "twenty-two thousand and above is Windows 11, whatever the key says"
    );
    assert_eq!(
        eleven("Windows 10 Pro", "22000"),
        "Windows 11 Pro",
        "the first build of 11 is 11"
    );

    // Below the line nothing is corrected — a real Windows 10 keeps its name.
    assert_eq!(eleven("Windows 10 Pro", "19045"), "Windows 10 Pro");
    assert_eq!(eleven("Windows 10 Home", "19045"), "Windows 10 Home");

    // A name that is already right is left alone, and so is one this rule knows nothing about.
    assert_eq!(eleven("Windows 11 Pro", "26100"), "Windows 11 Pro");
    assert_eq!(
        eleven("Windows Server 2022", "20348"),
        "Windows Server 2022"
    );

    // A build that is not a number decides nothing (NFR-13).
    assert_eq!(eleven("Windows 10 Pro", ""), "Windows 10 Pro");
    assert_eq!(eleven("Windows 10 Pro", "not a number"), "Windows 10 Pro");
}

/// **What the first box of «Что приложить?» shows is what the appeal writes** — the promise of
/// that step in one assertion.
#[test]
fn the_line_beside_the_box_is_the_line_in_the_appeal() {
    with_product_strings();

    let facts = full_facts();
    let beside = letters::report::machine_line(&facts);
    let appeal = build(&full_draft(), &facts);

    assert!(!beside.is_empty());
    assert!(
        appeal.contains(&beside),
        "the appeal must carry the very sentence the box showed:\n{beside}\n---\n{appeal}"
    );
}

/// **Т-33-2, решение 103.2** — the six labels of the three cards are carriers of strings and
/// not elements of the window.
///
/// The defect this guards against was found by the eye and measured by the stand: while the
/// labels were visible statics, hovering a card wiped its two lines off the screen. The cards
/// stand **above** their labels in the z-order of the template, no control of the window
/// carries `WS_CLIPSIBLINGS`, and a repaint of the button alone therefore painted over words
/// nobody would ask to be painted again — ink inside a card measured 3030 → 0
/// (`scratchpad-Э33\причина-2-наведение.log`).
///
/// The cure is one body of drawing: `draw_card` paints the card and its two lines together,
/// and the statics keep only the words. `NOT WS_VISIBLE` is what makes that true, and it is
/// read here out of `app.rc` — the file that decides it — because a label shown again would
/// bring the whole defect back and nothing else in the battery would notice.
#[test]
fn the_six_labels_of_the_cards_are_invisible_carriers_of_their_words() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc");
    let text = std::fs::read_to_string(&script).expect("app.rc must be readable");

    for label in [
        "IDC_WZ_CARD_T1",
        "IDC_WZ_CARD_T2",
        "IDC_WZ_CARD_T3",
        "IDC_WZ_CARD_S1",
        "IDC_WZ_CARD_S2",
        "IDC_WZ_CARD_S3",
    ] {
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with("LTEXT") && line.contains(label))
            .unwrap_or_else(|| panic!("app.rc has no LTEXT for {label}"));

        println!("{line}");

        assert!(
            line.contains("NOT WS_VISIBLE"),
            "{label} is a visible static again — hovering its card will wipe it: {line}"
        );
    }

    // And the three cards themselves are still shown: a card hidden with them would be a step
    // with nothing on it.
    for card in ["IDC_WZ_CARD_1", "IDC_WZ_CARD_2", "IDC_WZ_CARD_3"] {
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with("CONTROL") && line.contains(card))
            .unwrap_or_else(|| panic!("app.rc has no CONTROL for {card}"));

        assert!(!line.contains("NOT WS_VISIBLE"), "{card} is hidden: {line}");
    }
}
