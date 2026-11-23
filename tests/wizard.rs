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
use lang_switcher::ui::windows::wizard;

/// The interface strings of the product, so that the labels of an appeal are the real ones.
///
/// The same helper the letters tests use: without it `settings::text` answers with the strings
/// of whatever module ran first, and an appeal would be checked against blanks.
/// The strings of the product, opened as data, and Russian as the language of the program: every
/// test of this file reads the same words (one of them opening the strings while another builds an
/// appeal without them raced, AUDIT-180 Z8).
fn with_product_strings() {
    with_product_texts();
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
        translation: None,
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
    assert_eq!(Trouble::Idea.steps(), 3);

    assert!(Trouble::Idea.is_idea());
    assert!(!Trouble::WrongResult.is_idea());
}

// =========================================================================================
// Where an appeal goes — решения 199.1, 199.3, 199.10, 199.11, задача T-107-3
// =========================================================================================

/// The page `name` of `letters::wizard_pages`, described in Russian as the window of the shell
/// shows it (T-120-19), and the identifiers of its elements and of its row.
fn page(name: &str) -> (Vec<wizard::Id>, Vec<wizard::Id>) {
    use lang_switcher::ui::host::Content;

    let (_, wizard) = letters::wizard_pages()
        .into_iter()
        .find(|(page, _)| *page == name)
        .unwrap_or_else(|| panic!("{name} is a page of the wizard"));
    let window = wizard::Page {
        wizard,
        language: settings::Language::Ru,
        theme: lang_switcher::theme::ThemeSetting::Light,
    }
    .describe();
    let ids = |node: String| -> Vec<wizard::Id> {
        wizard::ALL
            .iter()
            .copied()
            .filter(|id| node.contains(&format!("id: {id:?},")))
            .collect()
    };
    (
        ids(format!("{:?}", window.body)),
        ids(format!("{:?}", window.buttons)),
    )
}

/// **Задача T-107-3, решения 199.3 и 199.11 — every appeal goes to the site, and the letter is the
/// way back.** Ready and on its way, the last step shows the text, the contact under it and
/// «Сохранить в папку журнала», and its button says «Отправить», then «Отправляется…»; once the
/// site did not take it, it says «Не удалось отправить», shows the address and the two buttons of
/// the letter, and its button says «Повторить». The same for a trouble and for an idea, because
/// neither the words nor the shape ask which (until 1.2.0 `destination` sent a trouble by letter
/// and an idea to the group or GitHub Issues — решения 181.1 и 181.6).
///
/// ⚠ Control: a shape that offers the letter before the site was asked is caught.
#[test]
fn every_appeal_goes_to_the_site_and_the_letter_is_the_way_back() {
    use letters::{Post, StepWords, step_words};

    let ready = step_words(Step::Preview, Post::Ready);
    let sending = step_words(Step::Preview, Post::Sending(7));
    let failed = step_words(Step::Preview, Post::Failed);

    assert_eq!(
        ready,
        StepWords {
            title: settings::IDS_WIZARD_PREVIEW_TITLE,
            note: Some(settings::IDS_WIZARD_PREVIEW_NOTE),
            next: settings::IDS_WIZARD_SEND,
        }
    );
    assert_eq!(sending.next, settings::IDS_WIZARD_SENDING);
    assert_eq!((sending.title, sending.note), (ready.title, ready.note));
    assert_eq!(
        failed,
        StepWords {
            title: settings::IDS_WIZARD_FAILED_TITLE,
            note: Some(settings::IDS_WIZARD_FAILED_NOTE),
            next: settings::IDS_WIZARD_RETRY,
        }
    );

    // The shapes, asked of the window of the shell (T-120-19): ready — the text, the contact and
    // the two quiet buttons; not taken — the address and the three buttons of the letter.
    let (ready_body, ready_row) = page("wizard preview");
    let (failed_body, failed_row) = page("wizard failed");
    println!("ready {ready_body:?} {ready_row:?}; failed {failed_body:?} {failed_row:?}");
    let with_contact = [
        wizard::Id::Appeal,
        wizard::Id::Contact,
        wizard::Id::CopyText,
        wizard::Id::Save,
    ];
    let with_letter = [
        wizard::Id::CopyText,
        wizard::Id::CopyAddress,
        wizard::Id::Save,
    ];

    assert_eq!(ready_body, with_contact, "the last step, ready");
    assert_eq!(failed_body, with_letter, "«Не удалось отправить»");
    assert!(
        ready_row.contains(&wizard::Id::Back) && !failed_row.contains(&wizard::Id::Back),
        "«Назад» on the last step, not on the failure: {ready_row:?} {failed_row:?}"
    );

    // The control: the letter offered before the site was asked.
    assert_ne!(
        ready_body, with_letter,
        "the control: the letter before the site was asked must be caught"
    );
}

/// **The thanks is one for a trouble and for an idea** — решение 199.10: one heading, no line of
/// explanation, and its button closes the window, as it did. The words themselves — the owner's,
/// with no number of the appeal — are read out of the built binary by `tests\settings.rs`
/// (`FR_94_STRINGS` and `the_thanks_of_the_wizard_carries_no_number_of_the_appeal`).
#[test]
fn the_thanks_is_one_for_both_kinds_of_appeal() {
    use letters::{Post, step_words};

    for post in [Post::Ready, Post::Failed] {
        let words = step_words(Step::Done, post);

        assert_eq!(words.title, settings::IDS_WIZARD_SENT_TITLE);
        assert_eq!(words.note, None);
        assert_eq!(words.next, settings::IDS_CLOSE);
    }
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
/// **Решение 181.28 — an idea carries what the person wrote and the version in its heading, and
/// nothing of the machine.** An idea goes to an open discussion (решение 181.1) and its road has no
/// step «Что приложить?», so the four boxes it never saw must not fill it: no line of Windows, of
/// the layouts, of the settings, no journal — with every box of the draft at its default, which is
/// ticked. A trouble, which goes by letter (решение 181.6), carries all four as before.
///
/// ⚠ Control: the same draft and facts as a trouble carry the journal — the facts are not empty.
#[test]
fn an_idea_carries_what_was_written_and_the_version_and_nothing_of_the_machine() {
    with_product_strings();

    let idea = Draft {
        trouble: Trouble::Idea,
        idea: "Хочу тёмную тему в мастере".to_owned(),
        helps: "Ночью глаза целее".to_owned(),
        ..full_draft()
    };
    let facts = full_facts();

    assert_eq!(idea.attach, Attach::default(), "every box at its default");
    assert!(
        idea.attach.machine && idea.attach.journal,
        "and the default is ticked"
    );

    let appeal = build(&idea, &facts);

    assert!(
        appeal.starts_with("Lang Switcher 0.41.0\r\n"),
        "the version is in the heading:\n{appeal}"
    );
    assert!(appeal.contains("Хочу тёмную тему в мастере") && appeal.contains("Ночью глаза целее"));

    // The needles are the facts themselves and not the labels before them: this binary reads no
    // string table, and an empty label would be «found» in any text at all.
    for unwanted in [
        facts.windows.as_str(),
        facts.layouts.as_str(),
        facts.settings.as_str(),
        "journal started",
    ] {
        assert!(
            !appeal.contains(unwanted),
            "181.28: an idea must not carry «{unwanted}»:\n{appeal}"
        );
    }

    let trouble = build(&full_draft(), &facts);

    assert!(
        trouble.contains("journal started") && trouble.contains(facts.windows.as_str()),
        "the control: a trouble carries the journal and the machine:\n{trouble}"
    );
}

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

#[test]
fn the_thanks_is_a_step_of_the_wizard_and_no_road_counts_it() {
    // The two roads are unchanged: the thanks is not a step of either.
    assert_eq!(Trouble::WrongResult.steps(), 5);
    assert_eq!(Trouble::Idea.steps(), 3);

    // The page of the thanks (T-120-19): the words of the owner and «Открыть канал» in it, and
    // «Закрыть» alone in the row — the main button, where «Далее» stands on the steps.
    let (body, row) = page("wizard sent");
    println!("{body:?} {row:?}");
    assert_eq!(
        body,
        [wizard::Id::Channel],
        "the thanks carries «Открыть канал»"
    );
    assert_eq!(row, [wizard::Id::Next], "and «Закрыть» alone in its row");
}

/// **T-120-19: «Получено сайтом автора» stands on the thanks and nowhere else** (3313, the condition
/// of the controller: the line says the site took the appeal, so it is shown only on the page that
/// stands after the site answered with success — `Wizard::take_answer` — and not on the last step
/// or on «Не удалось отправить»).
#[test]
fn the_line_of_the_site_stands_only_on_the_thanks() {
    use lang_switcher::ui::host::Content;

    with_product_strings();

    for (name, wizard) in letters::wizard_pages() {
        let window = wizard::Page {
            wizard,
            language: settings::Language::Ru,
            theme: lang_switcher::theme::ThemeSetting::Light,
        }
        .describe();
        // Its words are held by the canon of the strings (`tests\settings.rs`); here — the line itself.
        let carries = format!("{:?}", window.body).contains("Status {");
        println!("{name}: {carries}");
        assert_eq!(
            carries,
            name == "wizard sent",
            "{name}: the line «Получено сайтом автора» belongs to the thanks alone"
        );
    }
}

/// **T-120-19, the C3 of the controller: the text of the appeal reads in the direction of its own
/// language, not of the window.** The appeal is data for the author — «Что случилось:», «Lang
/// Switcher 1.8.0», the names of programs — in the language it was built in. In a window in Hebrew
/// a Russian appeal goes left to right (`FieldKind::Code { rtl: false }`); an appeal whose labels
/// are Hebrew goes right to left.
///
/// ⚠ Control: the window itself is mirrored in both cases — the direction of the field is not the
/// window's.
#[test]
fn the_text_of_the_appeal_reads_in_the_direction_of_its_own_language() {
    use lang_switcher::ui::host::Content;

    let field_of = |wizard: letters::Wizard| -> String {
        let window = wizard::Page {
            wizard,
            language: settings::Language::He,
            theme: lang_switcher::theme::ThemeSetting::Light,
        }
        .describe();
        let body = format!("{:?}", window.body);
        let at = body
            .find("kind: Code {")
            .expect("the last step carries the field of the appeal");
        body[at..].chars().take(26).collect()
    };
    let preview = || {
        letters::wizard_pages()
            .into_iter()
            .find(|(name, _)| *name == "wizard preview")
            .map(|(_, wizard)| wizard)
            .expect("the last step")
    };

    // The pages are built with Russian answers: a Russian appeal in a Hebrew window.
    let russian = field_of(preview());

    // The same step with the labels of the report in Hebrew.
    let mut hebrew = preview();
    hebrew
        .edit_appeal("Lang Switcher 1.8.0\n==========\n\nמה קרה: התוכנה עשתה משהו אחר".to_owned());
    let hebrew = field_of(hebrew);

    // T-120-22: a report about the translation has its mark above the name, skipped with the name.
    let mut marked = letters::Wizard::with(
        Draft {
            trouble: Trouble::Idea,
            translation: Some(settings::Language::He),
            ..Draft::default()
        },
        Facts::default(),
        Vec::new(),
    );
    let _ = marked.forward();
    let _ = marked.forward();
    marked.edit_appeal(
        "טעות בתרגום: he\nLang Switcher 1.8.0\n==========\n\nמה קרה: רעיון".to_owned(),
    );
    let marked = field_of(marked);

    println!("ru in he: {russian}; he in he: {hebrew}; marked he in he: {marked}");

    assert!(
        russian.starts_with("kind: Code { rtl: false }"),
        "a Russian appeal goes left to right in a Hebrew window: {russian}"
    );
    assert!(
        hebrew.starts_with("kind: Code { rtl: true }"),
        "an appeal in Hebrew goes right to left: {hebrew}"
    );
    assert!(
        marked.starts_with("kind: Code { rtl: true }"),
        "a report about the translation in Hebrew goes right to left: {marked}"
    );
}

/// The window of the page `name` of the wizard in Russian.
fn window_of(name: &str) -> lang_switcher::ui::scene::Window<wizard::Id> {
    use lang_switcher::ui::host::Content;

    let (_, wizard) = letters::wizard_pages()
        .into_iter()
        .find(|(page, _)| *page == name)
        .unwrap_or_else(|| panic!("{name} is a page of the wizard"));
    wizard::Page {
        wizard,
        language: settings::Language::Ru,
        theme: lang_switcher::theme::ThemeSetting::Light,
    }
    .describe()
}

/// **AUDIT-180 Z8, решения 208.131, 208.132 — the first step.** Two cards — «Программа сделала не
/// то», whose line says «…или ничего не произошло», and «Хочу предложить улучшение» — each with its
/// icon and the line of where it leads.
#[test]
fn the_first_step_offers_two_cards_with_their_icons_and_where_they_lead() {
    with_product_texts();
    let body = format!("{:?}", window_of("wizard what").body);
    let next = |id| {
        format!(
            "next: Some({:?})",
            settings::text_in(settings::Language::Ru, id)
        )
    };
    assert_eq!(body.matches("ChoiceCard").count(), 2, "two cards: {body}");
    for (icon, id) in [
        ("alert", settings::IDS_WIZARD_CARD_WRONG_NEXT),
        ("bulb", settings::IDS_WIZARD_CARD_IDEA_NEXT),
    ] {
        assert!(
            body.contains(&format!("icon: Some({icon:?})")) && body.contains(&next(id)),
            "the card of {icon} with where it leads: {body}"
        );
    }
    assert!(
        settings::text_in(settings::Language::Ru, settings::IDS_WIZARD_CARD_WRONG_SUB)
            .ends_with("или ничего не произошло"),
        "the line of the first card names what the second card used to"
    );
}

/// **AUDIT-180 Z8, решения 208.133, 208.136 п. 8 — the steps as decided.** «Где»: the kinds of field
/// are cards, no radio buttons; «Что делали»: «Ожидали» and «Получили» are fields of many lines;
/// «Что приложить»: the list stands in one group of rows.
#[test]
fn the_steps_of_the_wizard_are_cards_fields_of_many_lines_and_a_group() {
    with_product_texts();
    let place = format!("{:?}", window_of("wizard where").body);
    assert!(
        place.matches("ChoiceCard").count() == 3 && !place.contains("Radio"),
        "«Где»: three cards and no radio button: {place}"
    );
    let did = format!("{:?}", window_of("wizard did").body);
    for id in ["Expected", "Got"] {
        let at = did
            .find(&format!("id: {id},"))
            .unwrap_or_else(|| panic!("«Что делали»: the field {id}"));
        let kind = did[at..].find("kind: ").map(|from| &did[at + from..]);
        assert!(
            kind.is_some_and(|kind| kind.starts_with("kind: Text")),
            "«Что делали»: {id} is a field of many lines"
        );
    }
    let attach = format!("{:?}", window_of("wizard attach").body);
    assert!(
        attach.contains("Group([Check"),
        "«Что приложить»: the list in a group of rows: {attach}"
    );
}

/// **AUDIT-180 Z8, решение 208.133 — the bar of the steps at one height, a block down to the
/// buttons.** On every step of both roads, laid out in Russian in the window of 900 × 540 and in the
/// large one, the progress stands at one height, and the content reaches as close to the row of
/// buttons on every step: one block of each step takes what the step leaves.
#[test]
fn the_bar_of_the_steps_stands_at_one_height_and_a_block_reaches_the_buttons() {
    use lang_switcher::ui::fit;
    use lang_switcher::ui::layout::{Part, client_size};
    use lang_switcher::ui::text::{Direction, Text};

    with_product_texts();
    let language = settings::Language::Ru;
    let text = Text::new(language.tag(), Direction::LeftToRight).expect("DirectWrite");
    for size in [client_size(900.0, 540.0), client_size(1280.0, 720.0)] {
        let mut bars = Vec::new();
        let mut gaps = Vec::new();
        for (name, _) in letters::wizard_pages() {
            if matches!(name, "wizard sent" | "wizard failed") {
                continue;
            }
            let window = window_of(name);
            let layout = fit::emergency(
                &window,
                size,
                fit::k_for(language.tag(), size.class),
                false,
                &text,
            );
            let bar = layout
                .items
                .iter()
                .find(|item| matches!(item.part, Part::Progress { .. }))
                .map(|item| item.rect.y)
                .unwrap_or_else(|| panic!("{name}: the bar of the steps"));
            let rule = layout
                .items
                .iter()
                .find(|item| matches!(item.part, Part::ButtonRule))
                .map(|item| item.rect.y)
                .unwrap_or_else(|| panic!("{name}: the rule over the buttons"));
            let lowest = layout
                .items
                .iter()
                .filter(|item| {
                    item.rect.y < rule && !matches!(item.part, Part::Panel | Part::Art(_))
                })
                .map(|item| item.rect.y + item.rect.h)
                .fold(0.0_f32, f32::max);
            bars.push((name, bar));
            gaps.push((name, rule - lowest));
        }
        let first = bars[0].1;
        assert!(
            bars.iter().all(|(_, y)| (y - first).abs() < 0.5),
            "{size:?}: the bar at one height on every step: {bars:?}"
        );
        let gap = gaps[0].1;
        assert!(
            gaps.iter().all(|(_, g)| (g - gap).abs() < 1.0),
            "{size:?}: one distance to the buttons on every step: {gaps:?}"
        );
    }
}

/// The strings of the product binary, opened as data — the tests of the window and of its layout
/// read the words the program shows (`with_product_strings` of this file only sets the language).
fn with_product_texts() {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
    use windows::core::PCWSTR;

    static PRODUCT: OnceLock<usize> = OnceLock::new();
    let raw = *PRODUCT.get_or_init(|| {
        let exe = std::env::current_exe()
            .expect("the test executable has a path")
            .parent()
            .and_then(Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");
        let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: a path ending in a zero; the module is kept open for the process.
        let module =
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
                .expect("the product binary opens as a data file");
        module.0 as usize
    });
    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
    settings::set_ui_language(settings::Language::Ru);
}
