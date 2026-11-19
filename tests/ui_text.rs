//! Text of the interface shell, `src\ui\text.rs` (stage Э120, T-120-4): every language of the
//! registry measures, words wrap and a single line ends in an ellipsis, capitals follow the
//! language, names inside a right-to-left line stand in the written order, and the tall scripts
//! are not cut by a button — with each main face Windows has.
//!
//! The instrument of the mixed lines takes from DirectWrite where every character stands
//! (`HitTestTextPosition`) and compares the order of the pieces with the order written down. Its
//! own control is a line that **is** turned round without embeddings: the instrument must say so,
//! or it would confirm anything (lesson of stage E108).

use lang_switcher::theme::embedded_name;
use lang_switcher::ui::text::{
    Align, Direction, FixedAdvance, Laid, MAIN_FACES, MONO_FACES, Measure, Style, Text, Wrap,
};
use lang_switcher::ui::tokens::{BUTTON_MIN_HEIGHT, BUTTON_PRIMARY_WEIGHT, Role, window_b};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Registry {
    language: Vec<Language>,
}

#[derive(Deserialize)]
struct Language {
    tag: String,
    locale: String,
    endonym: String,
    rtl: bool,
}

fn registry() -> Vec<Language> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("res/lang/languages.toml");
    let text = std::fs::read_to_string(path).expect("languages.toml");
    toml::from_str::<Registry>(&text)
        .expect("the registry parses")
        .language
}

/// The string `id` of `res\lang\<tag>.rc`.
fn rc(tag: &str, id: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("res/lang/{tag}.rc"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{tag}.rc"));
    let line = text
        .lines()
        .find(|line| line.split_whitespace().next() == Some(id))
        .unwrap_or_else(|| panic!("{id} in {tag}.rc"));
    let (start, end) = (
        line.find('"').expect("a quote"),
        line.rfind('"').expect("a quote"),
    );
    line[start + 1..end].replace("\"\"", "\"")
}

fn direction(rtl: bool) -> Direction {
    if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    }
}

/// The main faces this Windows has: both on Windows 11, «Segoe UI» alone on Windows 10.
fn faces() -> Vec<&'static str> {
    let probe = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    MAIN_FACES
        .into_iter()
        .filter(|face| probe.has_face(face))
        .collect()
}

/// The type size of the usual window, 911 × 512.
fn usual_b() -> f32 {
    window_b(911.0, 512.0, 1.0)
}

#[test]
fn the_faces_are_the_systems_and_chosen_in_order() {
    let text = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    let faces = faces();

    assert!(
        faces.contains(&"Segoe UI"),
        "Segoe UI is on every Windows this program runs on"
    );
    assert_eq!(
        text.main_face(),
        faces[0],
        "the first of the passport's order that exists"
    );
    assert!(MONO_FACES.contains(&text.mono_face()));
}

#[test]
fn every_language_of_the_registry_measures_a_line_of_its_own() {
    let languages = registry();
    assert_eq!(languages.len(), 66, "the registry has 66 languages");

    for face in faces() {
        let mut report = Vec::new();
        for language in &languages {
            let text = Text::with_faces(
                face,
                MONO_FACES[1],
                &language.locale,
                direction(language.rtl),
            )
            .expect("DirectWrite");
            let line = format!("{} — {}", language.endonym, rc(&language.tag, "IDS_CLOSE"));
            let laid = text
                .lay(
                    &line,
                    Style::of(Role::Body, usual_b(), 512.0),
                    Wrap::Words,
                    Align::Start,
                    600.0,
                )
                .unwrap_or_else(|error| panic!("{}: {error}", language.tag));
            let size = laid.size();

            assert!(
                size.width > 0.0 && size.lines == 1,
                "{} in {face}: {size:?}",
                language.tag
            );
            report.push(format!(
                "TEXT|{face}|{}|{:.1}|{:.1}",
                language.tag, size.width, size.height
            ));
        }
        println!("{}", report.join("\n"));
    }
}

#[test]
fn words_wrap_within_the_width_and_one_line_ends_in_an_ellipsis() {
    let text = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    let style = Style::of(Role::Body, usual_b(), 512.0);
    let sentence = "Lang Switcher corrects text typed in the wrong keyboard layout, word by word.";

    let wrapped = text
        .lay(sentence, style, Wrap::Words, Align::Start, 200.0)
        .expect("a layout");
    let size = wrapped.size();
    assert!(size.lines > 1 && size.width <= 200.0, "{size:?}");
    assert!(
        (size.height - style.line_height() * size.lines as f32).abs() < 0.01,
        "every line is one line height of the role: {size:?}"
    );

    let cut = text
        .lay(sentence, style, Wrap::OneLine, Align::Start, 200.0)
        .expect("a layout");
    let size = cut.size();
    let last = sentence.encode_utf16().count() as u32 - 1;
    assert_eq!(size.lines, 1);
    assert!(size.width <= 200.0, "{size:?}");
    assert!(
        cut.is_trimmed(last),
        "the end of the line is cut away by the ellipsis"
    );
}

#[test]
fn capitals_follow_the_language_and_caps_are_spaced() {
    let eyebrow = Style::of(Role::Eyebrow, usual_b(), 512.0);
    let turkish = Text::new("tr-TR", Direction::LeftToRight).expect("DirectWrite");
    let english = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");

    let lay = |text: &Text, style: Style| {
        text.lay("bilgi", style, Wrap::OneLine, Align::Start, 400.0)
            .expect("a layout")
    };
    assert_eq!(lay(&turkish, eyebrow).shown(), "BİLGİ");
    assert_eq!(lay(&english, eyebrow).shown(), "BILGI");

    let unspaced = Style {
        tracking: 0.0,
        ..eyebrow
    };
    assert!(
        lay(&english, eyebrow).size().width > lay(&english, unspaced).size().width,
        "the tracking of the role widens the line"
    );
}

#[test]
fn the_stand_in_measure_counts_characters_and_breaks_between_words() {
    let measure = FixedAdvance(0.5);
    let style = Style::of(Role::Body, 20.0, 512.0);
    let advance = 0.5 * style.size;

    let one = measure.measure("abcd efgh", style, Wrap::Words, 1000.0);
    assert_eq!((one.width, one.lines), (9.0 * advance, 1));

    let two = measure.measure("abcd efgh", style, Wrap::Words, 6.0 * advance);
    assert_eq!((two.width, two.lines), (4.0 * advance, 2));
    assert_eq!(two.height, 2.0 * style.line_height());
}

// ---- The mixed lines ------------------------------------------------------------------------

/// A piece of a mixed line.
#[derive(Clone)]
enum Piece {
    /// Text of the language, tracked.
    Native(String),
    /// A name set in as an embedding of its own direction, tracked.
    Name(&'static str, bool),
    /// A name set in bare — only for the control line.
    Bare(&'static str),
    /// Glue between pieces: spaces, arrows. Not tracked.
    Glue(&'static str),
}

struct Mixed {
    what: &'static str,
    locale: &'static str,
    pieces: Vec<Piece>,
}

/// The line and the UTF-16 ranges of its tracked pieces, in the written order.
fn assemble(pieces: &[Piece]) -> (String, Vec<(String, std::ops::Range<u32>)>) {
    let (mut line, mut tracked) = (String::new(), Vec::new());
    let at = |line: &String| line.encode_utf16().count() as u32;

    for piece in pieces {
        let (name, set) = match piece {
            Piece::Native(text) => {
                let start = at(&line);
                line.push_str(text);
                tracked.push((text.clone(), start..at(&line)));
                continue;
            }
            Piece::Glue(text) => {
                line.push_str(text);
                continue;
            }
            Piece::Name(name, rtl) => (*name, embedded_name(name, *rtl)),
            Piece::Bare(name) => (*name, (*name).to_owned()),
        };

        // The name inside its marks: the marks themselves take no place to compare.
        let before = set.find(name).expect("the name is inside");
        let start = at(&line) + set[..before].encode_utf16().count() as u32;
        line.push_str(&set);
        tracked.push((
            name.to_owned(),
            start..start + name.encode_utf16().count() as u32,
        ));
    }

    (line, tracked)
}

/// Where the instrument found the written order broken, if it did: in a right-to-left line every
/// tracked piece stands wholly to the right of the next one.
fn order_faults(text: &Text, mixed: &Mixed) -> Vec<String> {
    let (line, tracked) = assemble(&mixed.pieces);
    let laid: Laid = text
        .lay(
            &line,
            Style::of(Role::Body, usual_b(), 512.0),
            Wrap::OneLine,
            Align::Start,
            4000.0,
        )
        .expect("a layout");

    let spans: Vec<(String, f32, f32)> = tracked
        .iter()
        .map(|(name, range)| {
            let places: Vec<_> = range.clone().filter_map(|at| laid.place_of(at)).collect();
            assert_eq!(
                places.len(),
                range.len(),
                "every character of «{name}» has a place"
            );
            let left = places.iter().map(|p| p.left).fold(f32::MAX, f32::min);
            let right = places
                .iter()
                .map(|p| p.left + p.width)
                .fold(f32::MIN, f32::max);
            (name.clone(), left, right)
        })
        .collect();

    spans
        .windows(2)
        .filter(|pair| pair[1].2 > pair[0].1 + 0.5)
        .map(|pair| {
            format!(
                "{}: «{}» [{:.1}, {:.1}] does not stand right of «{}» [{:.1}, {:.1}]",
                mixed.what, pair[0].0, pair[0].1, pair[0].2, pair[1].0, pair[1].1, pair[1].2
            )
        })
        .collect()
}

fn mixed_lines() -> Vec<Mixed> {
    let he = |id| Piece::Native(rc("he", id));
    let ar = |id| Piece::Native(rc("ar", id));
    let path = r"C:\Program Files\Lang_Switcher\LangSwitcher.exe";

    vec![
        Mixed {
            what: "he: the name of the program in a sentence",
            locale: "he-IL",
            pieces: vec![
                he("IDS_GROUP_GENERAL"),
                Piece::Glue(" "),
                Piece::Name("Lang Switcher", false),
                Piece::Glue(" "),
                he("IDS_MENU_SUSPEND"),
            ],
        },
        Mixed {
            what: "he: a path",
            locale: "he-IL",
            pieces: vec![
                he("IDS_LANGUAGE_LABEL"),
                Piece::Glue(" "),
                Piece::Name(path, false),
                Piece::Glue(" "),
                he("IDS_GROUP_GENERAL"),
            ],
        },
        Mixed {
            what: "ar: a version and a key",
            locale: "ar-SA",
            pieces: vec![
                ar("IDS_GROUP_GENERAL"),
                Piece::Glue(" "),
                Piece::Name("1.8.0", false),
                Piece::Glue(" "),
                ar("IDS_MENU_SUSPEND"),
                Piece::Glue(" "),
                Piece::Name("Pause", false),
            ],
        },
        Mixed {
            what: "he: names joined by arrows",
            locale: "he-IL",
            pieces: vec![
                Piece::Name("Lang Switcher", false),
                Piece::Glue(" ← "),
                Piece::Name("Pause", false),
                Piece::Glue(" ← "),
                Piece::Name("1.8.0", false),
            ],
        },
        Mixed {
            what: "ar: a name, an arrow, a word",
            locale: "ar-SA",
            pieces: vec![
                Piece::Name("Lang Switcher", false),
                Piece::Glue(" ← "),
                ar("IDS_MENU_RESUME"),
                Piece::Glue(" ← "),
                Piece::Name("Pause", false),
            ],
        },
    ]
}

#[test]
fn names_in_a_right_to_left_line_stand_in_the_written_order() {
    for face in faces() {
        let faults: Vec<String> = mixed_lines()
            .iter()
            .flat_map(|mixed| {
                let text =
                    Text::with_faces(face, MONO_FACES[1], mixed.locale, Direction::RightToLeft)
                        .expect("DirectWrite");
                order_faults(&text, mixed)
            })
            .collect();

        assert!(faults.is_empty(), "{face}:\n{}", faults.join("\n"));
    }
}

/// The control of the instrument: three Latin names joined by arrows, **bare**, merge into one
/// left-to-right island and come out in the reverse order — the instrument must see it.
#[test]
fn the_instrument_sees_a_line_turned_round_without_embeddings() {
    let bare = Mixed {
        what: "he: names joined by arrows, bare",
        locale: "he-IL",
        pieces: vec![
            Piece::Bare("Lang Switcher"),
            Piece::Glue(" ← "),
            Piece::Bare("Pause"),
            Piece::Glue(" ← "),
            Piece::Bare("1.8.0"),
        ],
    };

    for face in faces() {
        let text = Text::with_faces(face, MONO_FACES[1], bare.locale, Direction::RightToLeft)
            .expect("DirectWrite");
        assert!(
            !order_faults(&text, &bare).is_empty(),
            "{face}: the bare line was not caught"
        );
    }
}

// ---- The tall scripts -----------------------------------------------------------------------

/// Hindi, Tamil, Sinhala, Thai and Khmer, in a button of the passport's least height (2,5 B), the
/// text in the middle: the ink stays inside the button, at the smallest and the largest B.
#[test]
fn the_tall_scripts_are_not_cut_by_a_button() {
    // Sinhala is not a language of the program: its own name stands for it.
    let lines = [
        ("hi-IN", rc("hi", "IDS_CLOSE")),
        ("ta-IN", rc("ta", "IDS_CLOSE")),
        ("si-LK", "සිංහල — සිංහල අක්ෂර".to_owned()),
        ("th-TH", rc("th", "IDS_CLOSE")),
        ("km-KH", rc("km", "IDS_CLOSE")),
    ];
    let mut report = Vec::new();

    for face in faces() {
        for b in [12.0, usual_b(), 19.0] {
            let button = BUTTON_MIN_HEIGHT.0 * b;
            for weight in [Role::Button.style().weight, BUTTON_PRIMARY_WEIGHT] {
                let style = Style::of(Role::Button, b, 512.0).with_weight(weight);
                let room = (button - style.line_height()) / 2.0;

                for (locale, line) in &lines {
                    let text =
                        Text::with_faces(face, MONO_FACES[1], locale, Direction::LeftToRight)
                            .expect("DirectWrite");
                    let laid = text
                        .lay(line, style, Wrap::OneLine, Align::Centre, 600.0)
                        .expect("a layout");
                    let ink = laid.overhang();

                    assert!(
                        ink.top <= room && ink.bottom <= room,
                        "{face}, B {b:.2}, weight {weight}, {locale} «{line}»: the ink goes {:.2} above \
                         and {:.2} below the line, the button leaves {room:.2}",
                        ink.top,
                        ink.bottom
                    );
                    report.push(format!(
                        "INK|{face}|{b:.2}|{weight}|{locale}|{:.2}|{:.2}|{room:.2}",
                        ink.top, ink.bottom
                    ));
                }
            }
        }
    }
    println!("{}", report.join("\n"));
}
