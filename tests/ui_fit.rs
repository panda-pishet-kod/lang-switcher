//! The instrument of fitting (stage Э120, T-120-11): every window of the shell
//! (`ui::windows::catalogue`), in every state, in the 66 languages, in the three classes of window
//! (passport §2: narrow 620 × 500, ordinary 900 × 540, large 1280 × 690) and, for the languages read
//! right to left, mirrored — measured by DirectWrite with both main faces. A window fails when it
//! scrolls, cuts a line, sets type under 12 points or is not an exact mirror.
//!
//! The same pass makes the table of k (`fit::TABLE`): for a language and class, the smallest factor
//! every window needs with both faces, such that the content still fits at a type 3 % larger. With
//! `LANGSW_UI_FIT_WRITE` set the table is printed into `src\ui\fit.rs`, and with
//! `LANGSW_UI_FIT_REPORT` naming a file the report is written there (`reports\FIT-E120.md`).
//!
//! A string a language lacks is stood in for by the longest string of that language in the same
//! role, 30 % longer; the report counts them. The strings are the program's own, read from the
//! product binary as a data file, as `tests\letters.rs` does.

use lang_switcher::settings::{self, INTERFACE_STRINGS, Language};
use lang_switcher::ui::fit::{self, K};
use lang_switcher::ui::layout::{Class, Layout, Part, WindowSize, client_size, lay_out};
use lang_switcher::ui::scene::{ButtonKind, Role, Window};
use lang_switcher::ui::text::{Direction, MAIN_FACES, MONO_FACES, Measure, Style, Text, Wrap};
use lang_switcher::ui::tokens::{BUTTON_PAD, BUTTON_PRIMARY_WEIGHT, TYPE_FLOOR, window_b};
use lang_switcher::ui::windows::catalogue;
use std::collections::BTreeMap;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
use windows::core::PCWSTR;

/// The classes of window, each the tightest window of its layout.
const CLASSES: [(Class, f32, f32); 3] = [
    (Class::Narrow, 620.0, 500.0),
    (Class::Ordinary, 900.0, 540.0),
    (Class::Large, 1280.0, 690.0),
];
/// The windows at the border of the layout: the widest one column, the tightest two.
const BORDERS: [(f32, f32); 2] = [(879.0, 500.0), (880.0, 500.0)];
/// The margin of the table: the content fits at a type this much larger.
const MARGIN: f32 = 1.03;
/// A missing string is stood in for by the longest of its role, this much longer.
const STAND_IN: f32 = 1.3;
/// Halvings of the search for k.
const STEPS: u32 = 10;

fn with_product_strings() {
    static PRODUCT: OnceLock<usize> = OnceLock::new();
    let raw = *PRODUCT.get_or_init(|| {
        let exe = std::env::current_exe()
            .expect("the test executable has a path")
            .parent()
            .and_then(Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");
        let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated path owned by this frame; mapped for its resources only.
        let module =
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
                .expect("the product binary opens as a data file");
        module.0 as usize
    });
    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
}

/// The names of the string identifiers, read from `src\settings.rs`: `IDS_X: u16 = N`.
fn string_names() -> BTreeMap<u16, String> {
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/settings.rs"))
            .expect("src\\settings.rs");
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub const IDS_")?;
            let (name, value) = rest.split_once(": u16 = ")?;
            let value = value.trim_end_matches(';').parse().ok()?;
            Some((value, format!("IDS_{name}")))
        })
        .collect()
}

fn faces() -> Vec<&'static str> {
    let probe = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    MAIN_FACES
        .into_iter()
        .filter(|face| probe.has_face(face))
        .collect()
}

/// The windows of `language`, missing strings stood in for; answers them and how many stand-ins.
fn windows_of(language: Language) -> (Vec<(String, Window<u16>)>, usize) {
    let mut windows = catalogue(language);
    let mut longest: BTreeMap<String, String> = BTreeMap::new();
    for (_, window) in &mut windows {
        window.texts_mut(&mut |role, text| {
            let entry = longest.entry(format!("{role:?}")).or_default();
            if text.chars().count() > entry.chars().count() {
                *entry = text.clone();
            }
        });
    }
    let mut stand_ins = 0;
    for (_, window) in &mut windows {
        window.texts_mut(&mut |role, text| {
            if text.is_empty() {
                let model = longest
                    .get(&format!("{role:?}"))
                    .cloned()
                    .unwrap_or_else(|| "Lang Switcher".to_owned());
                let length = (model.chars().count() as f32 * STAND_IN).ceil() as usize;
                *text = model.chars().cycle().take(length).collect();
                stand_ins += 1;
            }
        });
    }
    (windows, stand_ins)
}

/// What fails in `window` at `size`, `k` and direction: a scroll, a cut line, type under 12.
fn faults(window: &Window<u16>, size: WindowSize, k: f32, rtl: bool, text: &Text) -> Vec<String> {
    let layout: Layout<u16> = lay_out(window, size, k, rtl, text);
    let mut found = Vec::new();
    if !layout.shortfall.fits() {
        found.push(format!(
            "does not fit: {:.1} across, {:.1} down",
            layout.shortfall.w, layout.shortfall.h
        ));
    }
    if layout.b < TYPE_FLOOR.0 {
        found.push(format!("B {:.2} under 12", layout.b));
    }
    for item in &layout.items {
        let (shown, needed) = match &item.part {
            Part::Text {
                role,
                text: line,
                wrap: Wrap::OneLine,
                ..
            } => (
                line,
                text.measure(
                    line,
                    Style::of(*role, layout.b, size.h),
                    Wrap::OneLine,
                    f32::MAX,
                )
                .width,
            ),
            Part::Button { kind, label } => {
                let mut style = Style::of(Role::Button, layout.b, size.h);
                if *kind == ButtonKind::Primary {
                    style = style.with_weight(BUTTON_PRIMARY_WEIGHT);
                }
                (
                    label,
                    text.measure(label, style, Wrap::OneLine, f32::MAX).width
                        + BUTTON_PAD.h.at(layout.b) * 2.0,
                )
            }
            _ => continue,
        };
        if needed > item.rect.w + 0.5 {
            found.push(format!(
                "cut: «{shown}» needs {needed:.1}, has {:.1}",
                item.rect.w
            ));
        }
    }
    found
}

/// Whether the mirrored layout is the exact reflection of the straight one; the first part that is
/// not.
fn mirror_fault(window: &Window<u16>, size: WindowSize, k: f32, text: &Text) -> Option<String> {
    let straight = lay_out(window, size, k, false, text);
    let mirrored = lay_out(window, size, k, true, text);
    straight
        .items
        .iter()
        .zip(&mirrored.items)
        .find_map(|(left, right)| {
            let x = size.w - left.rect.x - left.rect.w;
            ((right.rect.x - x).abs() > 0.01 || (right.rect.y - left.rect.y).abs() > 0.01)
                .then(|| format!("not mirrored: {}", left.part.name()))
        })
}

/// The largest k ≤ 1 at which `window` still fits at a type 3 % larger; none when it does not fit
/// even at 12 points.
fn k_needed(window: &Window<u16>, size: WindowSize, rtl: bool, text: &Text) -> Option<f32> {
    let fits = |k: f32| faults(window, size, k * MARGIN, rtl, text).is_empty();
    if fits(1.0) {
        return Some(1.0);
    }
    let floor = (TYPE_FLOOR.0 / window_b(size.w, size.h, 1.0)).min(1.0);
    if !fits(floor / MARGIN) {
        return None;
    }
    let (mut good, mut bad) = (floor / MARGIN, 1.0_f32);
    for _ in 0..STEPS {
        let middle = (good + bad) / 2.0;
        if fits(middle) {
            good = middle;
        } else {
            bad = middle;
        }
    }
    Some(good)
}

/// The string of `window` whose removal makes it fit — named by its identifier.
fn culprit(
    language: Language,
    window: &Window<u16>,
    size: WindowSize,
    k: f32,
    rtl: bool,
    text: &Text,
    names: &BTreeMap<u16, String>,
) -> String {
    let mut strings = Vec::new();
    window
        .clone()
        .texts_mut(&mut |_, line| strings.push(line.clone()));
    for (index, line) in strings.iter().enumerate() {
        let mut without = window.clone();
        let mut at = 0;
        without.texts_mut(&mut |_, other| {
            if at == index {
                other.clear();
            }
            at += 1;
        });
        if faults(&without, size, k, rtl, text).is_empty() {
            let id = INTERFACE_STRINGS.iter().find(|id| {
                let own = settings::text_in(language, **id);
                !own.is_empty() && (own == *line || own.replace("{0}", "Pause") == *line)
            });
            return match id {
                Some(id) => format!(
                    "{} «{line}»",
                    names.get(id).cloned().unwrap_or_else(|| id.to_string())
                ),
                None => format!("«{line}» (a string of no identifier)"),
            };
        }
    }
    "no single string".to_owned()
}

/// One pass of the instrument over every language: the table and what failed.
struct Pass {
    table: Vec<(String, K)>,
    /// Failures at the classes: language, window, class, face, what.
    failures: Vec<String>,
    /// The windows at the border of the layout that do not fit at the table's factor.
    borders: Vec<String>,
    /// For each language and class, the window that asks the smallest k.
    tightest: Vec<String>,
    stand_ins: usize,
}

fn pass() -> &'static Pass {
    static PASS: OnceLock<Pass> = OnceLock::new();
    PASS.get_or_init(|| {
        with_product_strings();
        let names = string_names();
        let faces = faces();
        let mut pass = Pass { table: Vec::new(), failures: Vec::new(), borders: Vec::new(), tightest: Vec::new(), stand_ins: 0 };

        for language in Language::ALL {
            let (windows, stand_ins) = windows_of(language);
            pass.stand_ins += stand_ins;
            let rtl = language.is_rtl();
            let direction = if rtl { Direction::RightToLeft } else { Direction::LeftToRight };
            let texts: Vec<(&str, Text)> = faces
                .iter()
                .map(|face| (*face, Text::with_faces(face, MONO_FACES[1], language.tag(), direction).expect("DirectWrite")))
                .collect();

            let mut ks = [1.0_f32; 3];
            for (slot, (class, w, h)) in CLASSES.iter().enumerate() {
                let size = client_size(*w, *h);
                assert_eq!(size.class, *class, "the class of {w}×{h}");
                let mut least: Option<(f32, String)> = None;
                for (name, window) in &windows {
                    for (face, text) in &texts {
                        match k_needed(window, size, rtl, text) {
                            Some(k) => {
                                if least.as_ref().is_none_or(|(known, _)| k < *known) {
                                    least = Some((k, format!("{name} ({face})")));
                                }
                            }
                            None => pass.failures.push(format!(
                                "{}: {name}, {class:?}, {face}: does not fit even at 12 points — {}",
                                language.tag(),
                                culprit(language, window, size, TYPE_FLOOR.0 / window_b(*w, *h, 1.0), rtl, text, &names)
                            )),
                        }
                    }
                }
                let (k, tightest) = least.unwrap_or((1.0, "—".to_owned()));
                ks[slot] = (k * 1000.0).floor() / 1000.0;
                if k < 1.0 {
                    pass.tightest.push(format!("{} {class:?}: k {:.3} — {tightest}", language.tag(), ks[slot]));
                }
            }
            let k = K { narrow: ks[0], ordinary: ks[1], large: ks[2] };
            pass.table.push((language.tag().to_owned(), k));

            for (w, h) in BORDERS {
                let size = client_size(w, h);
                for (name, window) in &windows {
                    for (face, text) in &texts {
                        let found = faults(window, size, k.of(size.class), rtl, text);
                        if !found.is_empty() {
                            pass.borders.push(format!("{}: {name}, {w}×{h}, {face}: {}", language.tag(), found.join("; ")));
                        }
                    }
                }
            }
        }
        pass
    })
}

/// Every window fits every language at its factor from the table, with both faces, in every class;
/// mirrored exactly for the languages read right to left.
#[test]
fn every_window_fits_every_language_at_the_factor_of_the_table() {
    with_product_strings();
    let names = string_names();
    let faces = faces();
    let mut found = Vec::new();

    for language in Language::ALL {
        let (windows, _) = windows_of(language);
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        for face in &faces {
            let text = Text::with_faces(face, MONO_FACES[1], language.tag(), direction)
                .expect("DirectWrite");
            for (class, w, h) in CLASSES {
                let size = client_size(w, h);
                let k = fit::k_for(language.tag(), class);
                for (name, window) in &windows {
                    let faults = faults(window, size, k, rtl, &text);
                    if !faults.is_empty() {
                        let who = culprit(language, window, size, k, rtl, &text, &names);
                        found.push(format!(
                            "{}: {name}, {class:?}, {face}, k {k}: {} — {who}",
                            language.tag(),
                            faults.join("; ")
                        ));
                    }
                    if rtl && let Some(fault) = mirror_fault(window, size, k, &text) {
                        found.push(format!(
                            "{}: {name}, {class:?}, {face}: {fault}",
                            language.tag()
                        ));
                    }
                }
            }
        }
    }

    assert!(
        found.is_empty(),
        "{} failures:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The guard of the table: computed again on this machine, no factor comes out smaller than the
/// table says — or the strings have grown and the table is to be printed again.
#[test]
fn the_table_is_not_above_what_the_windows_need_on_this_machine() {
    let pass = pass();
    let mut found = Vec::new();
    for (tag, needed) in &pass.table {
        let written = fit::TABLE
            .iter()
            .find(|(known, _)| known == tag)
            .map(|(_, k)| *k);
        let Some(written) = written else {
            found.push(format!("{tag}: not in the table"));
            continue;
        };
        for (class, _, _) in CLASSES {
            if needed.of(class) + 0.0005 < written.of(class) {
                found.push(format!(
                    "{tag} {class:?}: the table says {}, the windows need {}",
                    written.of(class),
                    needed.of(class)
                ));
            }
        }
    }
    assert!(
        found.is_empty(),
        "print the table again (LANGSW_UI_FIT_WRITE):\n{}",
        found.join("\n")
    );
    assert!(
        pass.failures.is_empty(),
        "languages that do not fit even at 12 points (C6):\n{}",
        pass.failures.join("\n")
    );
}

/// Prints the table into `src\ui\fit.rs` and the report, when asked to.
#[test]
fn print_the_table_and_the_report() {
    let write = std::env::var_os("LANGSW_UI_FIT_WRITE").is_some();
    let report = std::env::var_os("LANGSW_UI_FIT_REPORT");
    if !write && report.is_none() {
        return;
    }
    let pass = pass();
    let rows: Vec<String> = pass
        .table
        .iter()
        .map(|(tag, k)| {
            format!(
                "    (\"{tag}\", K {{ narrow: {:?}, ordinary: {:?}, large: {:?} }}),",
                k.narrow, k.ordinary, k.large
            )
        })
        .collect();

    if write {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/fit.rs");
        let source = std::fs::read_to_string(&path).expect("src\\ui\\fit.rs");
        let start = source
            .find("pub const TABLE: &[(&str, K)] = &[")
            .expect("the start of the table");
        let end = source[start..].find("];").expect("the end of the table") + start + 2;
        let table = format!(
            "pub const TABLE: &[(&str, K)] = &[\r\n{}\r\n];",
            rows.join("\r\n")
        );
        std::fs::write(
            &path,
            format!("{}{table}{}", &source[..start], &source[end..]),
        )
        .expect("fit.rs written");
    }

    if let Some(report) = report {
        let mut text = String::new();
        text.push_str("# FIT-E120 — прибор влезания и таблица кегля (T-120-11)\r\n\r\n");
        text.push_str(&format!(
            "Окон в каталоге: {}. Языков: {}. Гарнитуры: {}. Временных образцов строк: {}.\r\n\r\n",
            catalogue(Language::Ru).len(),
            pass.table.len(),
            faces().join(", "),
            pass.stand_ins
        ));
        text.push_str("## Таблица k (узкое 620×500 / обычное 900×540 / большое 1280×690)\r\n\r\n| Язык | Узкое | Обычное | Большое |\r\n|---|---|---|---|\r\n");
        for (tag, k) in &pass.table {
            text.push_str(&format!(
                "| {tag} | {} | {} | {} |\r\n",
                k.narrow, k.ordinary, k.large
            ));
        }
        text.push_str("\r\n## Окна, которые тянут свой язык вниз\r\n\r\n");
        for line in &pass.tightest {
            text.push_str(&format!("- {line}\r\n"));
        }
        if pass.tightest.is_empty() {
            text.push_str("Нет: при k = 1 с запасом 3 % помещается всё.\r\n");
        }
        text.push_str("\r\n## Не помещается даже при кегле 12 (C6)\r\n\r\n");
        for line in &pass.failures {
            text.push_str(&format!("- {line}\r\n"));
        }
        if pass.failures.is_empty() {
            text.push_str("Нет.\r\n");
        }
        text.push_str("\r\n## Окна на границе раскладки (879×500, 880×500) — не красный тест, допустим аварийный путь\r\n\r\n");
        for line in &pass.borders {
            text.push_str(&format!("- {line}\r\n"));
        }
        if pass.borders.is_empty() {
            text.push_str("Помещаются все.\r\n");
        }
        std::fs::write(report, text).expect("the report written");
    }
}

/// The table covers every language of the program: the factor is read from `fit.rs`, not picked
/// while the program runs. Red before the table was printed — it was empty.
#[test]
fn the_table_has_a_factor_for_every_language() {
    let missing: Vec<&str> = Language::ALL
        .iter()
        .map(|language| language.tag())
        .filter(|tag| !fit::TABLE.iter().any(|(known, _)| known == tag))
        .collect();
    assert!(missing.is_empty(), "not in fit::TABLE: {missing:?}");
}
