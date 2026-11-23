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
use lang_switcher::ui::layout::{
    Class, Concession, Layout, Part, WindowSize, client_size, lay_out, lay_out_with,
};
use lang_switcher::ui::scene::{ButtonKind, Role, Window, WindowKind};
use lang_switcher::ui::text::{Direction, MAIN_FACES, MONO_FACES, Measure, Style, Text, Wrap};
use lang_switcher::ui::tokens::{
    BUTTON_PAD, BUTTON_PRIMARY_WEIGHT, CORNER_WIDTH, TYPE_FLOOR, window_b,
};
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
/// The step down of a factor of the table at which not every window fits.
const DOWN: f32 = 0.005;
#[path = "support/excused.rs"]
mod excused;
use excused::{
    Category, EXCUSED, UNTRANSLATED, excuse_line, excused, uncategorised, uncategorised_strings,
    unsettled, unsettled_strings,
};

/// The number of a step of the order of giving way in the passport (§3а).
fn step_number(step: Concession) -> usize {
    Concession::ALL
        .iter()
        .position(|known| *known == step)
        .expect("a step of the order")
        + 1
}

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
    let stand_ins = stand_in(&mut windows);
    (windows, stand_ins)
}

/// ⭐ **The places of the texts empty in English too** — empty on purpose, no string missing
/// there (окно правок 208.235): the label of a segment that has none («Одна клавиша | Две
/// клавиши», «Пара | Несколько раскладок»), the hint of a field that shows a value. By the name of
/// the window, whether each of its texts, in the order [`Window::texts_mut`] walks them, is empty in
/// English.
fn empty_in_english() -> &'static BTreeMap<String, Vec<bool>> {
    static EMPTY: OnceLock<BTreeMap<String, Vec<bool>>> = OnceLock::new();
    EMPTY.get_or_init(|| {
        with_product_strings();
        catalogue(Language::En)
            .into_iter()
            .map(|(name, mut window)| {
                let mut empty = Vec::new();
                window.texts_mut(&mut |_, text| empty.push(text.is_empty()));
                (name, empty)
            })
            .collect()
    })
}

/// Stands in for the missing strings of `windows`: a text empty here and not in English is a string
/// the language lacks, and takes the longest of its role, [`STAND_IN`] times longer. A text empty
/// in English too is empty on purpose and stays so — ⛔ standing in for it put a paragraph of ≈ 990
/// characters (the warning about a single `Ctrl`, the longest `Body` of ta) over the segment of
/// «Горячая клавиша», and the instrument said 14.9 points short where 114.7 were free (окно правок
/// 208.235). A window whose texts are not as many as in English is stood in for everywhere, as
/// before. Answers how many stand-ins.
fn stand_in(windows: &mut [(String, Window<u16>)]) -> usize {
    let mut longest: BTreeMap<String, String> = BTreeMap::new();
    for (_, window) in windows.iter_mut() {
        window.texts_mut(&mut |role, text| {
            let entry = longest.entry(format!("{role:?}")).or_default();
            if text.chars().count() > entry.chars().count() {
                *entry = text.clone();
            }
        });
    }
    let mut stand_ins = 0;
    for (name, window) in windows.iter_mut() {
        let mut count = 0;
        window.texts_mut(&mut |_, _| count += 1);
        let english = empty_in_english()
            .get(name.as_str())
            .filter(|empty| empty.len() == count);
        let mut at = 0;
        window.texts_mut(&mut |role, text| {
            let on_purpose = english.is_some_and(|empty| empty[at]);
            at += 1;
            if text.is_empty() && !on_purpose {
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
    stand_ins
}

/// What fails in `window` at `size`, `k` and direction, every step of giving way (§3а) allowed: a
/// scroll, a cut line, type under 12.
fn faults(window: &Window<u16>, size: WindowSize, k: f32, rtl: bool, text: &Text) -> Vec<String> {
    faults_with(window, size, k, rtl, text, Concession::ALL)
}

/// [`faults`] with only the steps `allowed`.
fn faults_with(
    window: &Window<u16>,
    size: WindowSize,
    k: f32,
    rtl: bool,
    text: &Text,
    allowed: &[Concession],
) -> Vec<String> {
    // The window in the corner has its own width and takes the height its content needs: only
    // what is cut across counts (T-120-14).
    let size = if window.kind == WindowKind::Corner {
        let first = lay_out_with(
            window,
            client_size(CORNER_WIDTH.0, size.h),
            k,
            rtl,
            text,
            allowed,
        );
        client_size(CORNER_WIDTH.0, size.h + first.shortfall.h.max(0.0))
    } else {
        size
    };
    let layout: Layout<u16> = lay_out_with(window, size, k, rtl, text, allowed);
    let mut found = Vec::new();
    if !layout.shortfall.fits() {
        found.push(format!(
            "does not fit: {:.1} across, {:.1} down",
            layout.shortfall.w, layout.shortfall.h
        ));
    }
    // 208.198: on a small screen a body that still lacks height after the steps of giving way
    // scrolls — no fault while every element of it can be reached, that is while its room holds a
    // line of it at least.
    if let Some(body) = layout.scroll {
        let line = Style::of(Role::Body, layout.b, size.h).line_height();
        if body.viewport.h < line {
            found.push(format!(
                "the room of the scrolled body is {:.1}, under a line ({line:.1}): not reachable",
                body.viewport.h
            ));
        }
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
                let (pad_h, pad_v) = (BUTTON_PAD.h.at(layout.b), BUTTON_PAD.v.at(layout.b));
                let one_line = text.measure(label, style, Wrap::OneLine, f32::MAX).width;
                // A label on more lines (§3а, step 5): every line in the button, the lines in its
                // height.
                let inner = item.rect.w - pad_h * 2.0;
                if one_line > inner + 0.5 {
                    let lines = text.measure(label, style, Wrap::Words, inner);
                    if lines.height + pad_v * 2.0 <= item.rect.h + 0.5 {
                        (label, lines.width + pad_h * 2.0)
                    } else {
                        (label, one_line + pad_h * 2.0)
                    }
                } else {
                    (label, one_line + pad_h * 2.0)
                }
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

/// Whether the mirrored layout is the exact reflection of the straight one (what lies over the
/// picture of the panel moves with the picture, T-120-25); the first part that is not.
fn mirror_fault(window: &Window<u16>, size: WindowSize, k: f32, text: &Text) -> Option<String> {
    let straight = lay_out(window, size, k, false, text);
    let mirrored = lay_out(window, size, k, true, text);
    straight
        .items
        .iter()
        .zip(&mirrored.items)
        .find_map(|(left, right)| {
            let x = mirrored.reflection_of(&straight, left).x;
            ((right.rect.x - x).abs() > 0.01 || (right.rect.y - left.rect.y).abs() > 0.01)
                .then(|| format!("not mirrored: {}", left.part.name()))
        })
}

/// The largest k ≤ 1 at which `window` still fits at a type 3 % larger: the type first — measured
/// with the steps of the table only ([`Concession::TABLE`]) — and only a window that does not fit
/// so even at 12 points with every step of giving way (§3а); none when it does not fit even then.
fn k_needed(window: &Window<u16>, size: WindowSize, rtl: bool, text: &Text) -> Option<f32> {
    k_needed_with(window, size, rtl, text, Concession::TABLE)
        .or_else(|| k_needed_with(window, size, rtl, text, Concession::ALL))
}

fn k_needed_with(
    window: &Window<u16>,
    size: WindowSize,
    rtl: bool,
    text: &Text,
    allowed: &[Concession],
) -> Option<f32> {
    let fits = |k: f32| faults_with(window, size, k * MARGIN, rtl, text, allowed).is_empty();
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
    // The window with the string `index` changed by `change`.
    let with = |index: usize, change: &dyn Fn(&mut String)| {
        let mut changed = window.clone();
        let mut at = 0;
        changed.texts_mut(&mut |_, other| {
            if at == index {
                change(other);
            }
            at += 1;
        });
        changed
    };
    let across = |window: &Window<u16>| {
        lay_out_with(window, size, k, rtl, text, Concession::ALL)
            .shortfall
            .w
    };
    let name = |line: &str| {
        let id = INTERFACE_STRINGS.iter().find(|id| {
            let own = settings::text_in(language, **id);
            !own.is_empty() && (own == line || own.replace("{0}", "Pause") == line)
        });
        match id {
            Some(id) => format!(
                "{} «{line}»",
                names.get(id).cloned().unwrap_or_else(|| id.to_string())
            ),
            None => format!("«{line}» (a string of no identifier)"),
        }
    };

    // Across: the run of one line wider than its room — a string that, made longer, runs over
    // further, and taken away leaves nothing over. A paragraph that wraps is not it, even when
    // taking it away changes the steps of giving way so that the window fits (ml «Спасибо»: the
    // lead was named for the label of the button of a card, 208.193).
    let over = across(window);
    if over > 0.5 {
        for (index, line) in strings.iter().enumerate() {
            let longer = with(index, &|s: &mut String| *s = s.repeat(2));
            if across(&longer) <= over + 0.5 {
                continue;
            }
            if across(&with(index, &|s: &mut String| s.clear())) <= 0.5 {
                return name(line);
            }
        }
    }

    for (index, line) in strings.iter().enumerate() {
        if faults(
            &with(index, &|s: &mut String| s.clear()),
            size,
            k,
            rtl,
            text,
        )
        .is_empty()
        {
            return name(line);
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
    /// The windows that gave way (§3а) at the factor of the table: language, window, class, face,
    /// the step they came to.
    steps: Vec<String>,
    /// The windows of the wave of translations (208.175): what is short at 12 points, and which
    /// string — the list for the translator.
    wave: Vec<String>,
    /// The failures excused, each with its reason.
    excused: Vec<String>,
    /// The excuses this pass leaned on: window, class, language — not stale while it does (Пс2,
    /// 23.11: at the factor of the table a window fits, with the margin of 3 % it does not even at
    /// 12 points; the excuse is needed by this pass and excused nothing at the table).
    leaned_on: Vec<(String, Class, String)>,
    stand_ins: usize,
}

fn pass() -> &'static Pass {
    static PASS: OnceLock<Pass> = OnceLock::new();
    PASS.get_or_init(|| {
        with_product_strings();
        let names = string_names();
        let faces = faces();
        let mut pass = Pass { table: Vec::new(), failures: Vec::new(), borders: Vec::new(), tightest: Vec::new(), steps: Vec::new(), wave: Vec::new(), excused: Vec::new(), leaned_on: Vec::new(), stand_ins: 0 };

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
            // 208.200: the factor of the narrow class is the ordinary one — of the reference window —
            // no higher than the width allows: the type does not jump with the size of the screen,
            // and on a small screen the height is the scroll's. So the ordinary class goes first.
            for slot in [1, 0, 2] {
                let (class, w, h) = &CLASSES[slot];
                let size = client_size(*w, *h);
                assert_eq!(size.class, *class, "the class of {w}×{h}");
                let mut least: Option<(f32, String)> = None;
                // The windows that do not fit even at 12 points, and the excused: they do not hold the
                // factor down.
                let mut beyond: Vec<&str> = Vec::new();
                for (name, window) in &windows {
                    for (face, text) in &texts {
                        match k_needed(window, size, rtl, text) {
                            Some(k) => {
                                if least.as_ref().is_none_or(|(known, _)| k < *known) {
                                    least = Some((k, format!("{name} ({face})")));
                                }
                            }
                            None => {
                                let line = format!(
                                    "{}: {name}, {class:?}, {face}: does not fit even at 12 points — {}",
                                    language.tag(),
                                    culprit(language, window, size, TYPE_FLOOR.0 / window_b(*w, *h, 1.0), rtl, text, &names)
                                );
                                beyond.push(name);
                                match excused(name, *class, language.tag()) {
                                    Some(excuse) => {
                                        pass.excused.push(format!("{line} — ИСКЛЮЧЕНИЕ: {}", excuse_line(excuse)));
                                        pass.leaned_on.push((name.to_string(), *class, language.tag().to_owned()));
                                    }
                                    None => pass.failures.push(line),
                                }
                            }
                        }
                    }
                }
                let (k, mut tightest) = least.unwrap_or((1.0, "—".to_owned()));
                let mut k = (k * 1000.0).floor() / 1000.0;
                if *class == Class::Narrow && ks[1] < k {
                    k = ks[1];
                    tightest = format!("the ceiling of Ordinary (208.200); across: {tightest}");
                }
                // Each window was measured alone, and the rule of the pair is not monotone in k: at a
                // smaller type a pair may stand side by side and come out taller. The factor of the
                // table goes down further while any window does not fit at it.
                let floor = (TYPE_FLOOR.0 / window_b(*w, *h, 1.0)).min(1.0);
                let all_fit = |k: f32| {
                    windows.iter().filter(|(name, _)| !beyond.contains(&name.as_str()) && excused(name, *class, language.tag()).is_none()).all(|(_, window)| {
                        texts.iter().all(|(_, text)| faults(window, size, k, rtl, text).is_empty())
                    })
                };
                while k > floor && !all_fit(k) {
                    k = ((k - DOWN) * 1000.0).floor() / 1000.0;
                }
                ks[slot] = k;
                if k < 1.0 {
                    pass.tightest.push(format!("{} {class:?}: k {:.3} — {tightest}", language.tag(), ks[slot]));
                }
            }
            let k = K { narrow: ks[0], ordinary: ks[1], large: ks[2] };
            pass.table.push((language.tag().to_owned(), k));

            // How far each window gave way (§3а) at the factor of the table.
            for (class, w, h) in CLASSES {
                let size = client_size(w, h);
                for (name, window) in &windows {
                    for (face, text) in &texts {
                        if let Some(step) = lay_out(window, size, k.of(class), rtl, text).step {
                            pass.steps.push(format!(
                                "{}: {name}, {class:?}, {face}: шаг {} ({step:?})",
                                language.tag(),
                                step_number(step)
                            ));
                        }
                        let wave = excused(name, class, language.tag())
                            .is_some_and(|excuse| excuse.category == Some(Category::Wave));
                        if wave {
                            // At 12 points: the emergency road of the window goes down to there.
                            let floor = (TYPE_FLOOR.0 / window_b(w, h, 1.0)).min(1.0);
                            let short = lay_out(window, size, floor, rtl, text).shortfall;
                            // Only what is short: a window that fits at 12 asks nothing of the translator.
                            if !short.fits() {
                                pass.wave.push(format!(
                                    "| {name} | {} | {class:?} {w}×{h} | {face} | {:.1} | {:.1} | {} |",
                                    language.tag(),
                                    short.w.max(0.0),
                                    short.h.max(0.0),
                                    culprit(language, window, size, floor, rtl, text, &names)
                                ));
                            }
                        }
                    }
                }
            }

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
    let mut excused_found = Vec::new();
    let mut matched: Vec<(String, Class, String)> = Vec::new();
    // 208.198: who scrolls on a small screen — printed, not counted.
    let mut scrolled: Vec<String> = Vec::new();

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
                    if size.scrolls
                        && lay_out_with(window, size, k, rtl, &text, Concession::ALL)
                            .scroll
                            .is_some()
                    {
                        let who = format!("{}: {name}, {class:?}", language.tag());
                        if !scrolled.contains(&who) {
                            scrolled.push(who);
                        }
                    }
                    if !faults.is_empty() {
                        let who = culprit(language, window, size, k, rtl, &text, &names);
                        let line = format!(
                            "{}: {name}, {class:?}, {face}, k {k}: {} — {who}",
                            language.tag(),
                            faults.join("; ")
                        );
                        match excused(name, class, language.tag()) {
                            Some(excuse) => {
                                excused_found
                                    .push(format!("{line} — ИСКЛЮЧЕНИЕ: {}", excuse_line(excuse)));
                                matched.push((name.clone(), class, language.tag().to_owned()));
                            }
                            None => found.push(line),
                        }
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

    // An excuse that excused nothing is taken away (208.175): it would hide a fault to come.
    // An excuse the pass of the table leans on (C6, with its margin of 3 %) is not stale either.
    matched.extend(pass().leaned_on.iter().cloned());
    let stale = excused::stale(&matched);
    for line in &stale {
        found.push(format!("an excuse no longer needed — take it away: {line}"));
    }

    // The excused, each with its reason — seen in the output, not counted.
    let mut excused_windows: Vec<String> = Vec::new();
    for line in &excused_found {
        let key = line.split(", k ").next().unwrap_or(line).to_owned();
        if !excused_windows.contains(&key) {
            excused_windows.push(key);
        }
    }
    println!(
        "excused ({} cases, {} windows × languages × faces):",
        excused_found.len(),
        excused_windows.len()
    );
    for excuse in EXCUSED {
        println!("- {}", excuse_line(excuse));
    }
    println!(
        "scrolls on a small screen (208.198, 620 × 500): {}",
        scrolled.len()
    );
    for line in &scrolled {
        println!("- {line}");
    }
    // How far the windows gave way (§3а) at the factors of the table.
    let steps = &pass().steps;
    println!("gave way (§3а): {}", steps.len());
    for line in steps {
        println!("- {line}");
    }
    assert!(
        found.is_empty(),
        "{} failures:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The guard of the table: computed again on this machine, no factor comes out smaller than the
/// table says — or the strings have grown and the table is to be printed again. What the narrow
/// class needs is the ordinary factor at most (208.200): a narrow factor above it is red here too.
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

/// 208.200: the type does not jump with the size of the screen — no language has a factor of the
/// narrow class above the one of the ordinary class, the reference window.
#[test]
fn the_narrow_factor_is_not_above_the_ordinary_one() {
    let found: Vec<String> = fit::TABLE
        .iter()
        .filter(|(_, k)| k.narrow > k.ordinary)
        .map(|(tag, k)| format!("{tag}: narrow {} above ordinary {}", k.narrow, k.ordinary))
        .collect();
    assert!(
        found.is_empty(),
        "208.200, k Narrow ≤ k Ordinary:\n{}",
        found.join("\n")
    );
}

/// ⭐ **A missing string is stood in for, a text empty on purpose is not** (окно правок 208.235).
/// In «settings Hotkey ctrl set» of ta the texts empty in English too — the label of the segment
/// of the mode, the hints of the fields that show their keys — stay empty; the note under the
/// second field, emptied as a language that lacks the string would have it, takes a stand-in.
#[test]
fn a_missing_string_is_stood_in_for_and_a_text_empty_on_purpose_is_not() {
    with_product_strings();
    let mut windows = catalogue(Language::Ta);
    let at = windows
        .iter()
        .position(|(name, _)| name == "settings Hotkey ctrl set")
        .expect("the state of the tap set");
    let count_empty = |window: &mut Window<u16>| {
        let mut empty = 0;
        window.texts_mut(&mut |_, text| empty += usize::from(text.is_empty()));
        empty
    };
    let on_purpose = count_empty(&mut windows[at].1);
    assert!(on_purpose > 0, "the state has texts empty on purpose");

    let note = settings::text_in(Language::Ta, settings::IDS_CTRL_WARN_TEXT);
    let (mut index, mut emptied) = (0, None);
    windows[at].1.texts_mut(&mut |_, text| {
        if *text == note {
            text.clear();
            emptied = Some(index);
        }
        index += 1;
    });
    let emptied = emptied.expect("the note of the tap is in the state");

    let stand_ins = stand_in(&mut windows);
    let (mut index, mut stood_in) = (0, String::new());
    windows[at].1.texts_mut(&mut |_, text| {
        if index == emptied {
            stood_in = text.clone();
        }
        index += 1;
    });
    assert!(
        stand_ins >= 1 && !stood_in.is_empty(),
        "the missing note took no stand-in"
    );
    assert_eq!(
        count_empty(&mut windows[at].1),
        on_purpose,
        "a text empty on purpose took a stand-in"
    );
}

/// Prints the table into `src\ui\fit.rs` and the report, when asked to.
#[test]
fn print_the_table_and_the_report() {
    let write = std::env::var_os("LANGSW_UI_FIT_WRITE").is_some();
    let report = std::env::var_os("LANGSW_UI_FIT_REPORT");
    let wave = std::env::var_os("LANGSW_UI_FIT_WAVE");
    if !write && report.is_none() && wave.is_none() {
        return;
    }
    let pass = pass();

    // The list for the translator (208.175): `l10n\E120-fit-wave.md`.
    if let Some(wave) = wave {
        let mut text = String::new();
        text.push_str(
            "# E120-fit-wave — что укоротить в волне переводов (T-120-24, 208.175)\r\n\r\n",
        );
        text.push_str(
            "Окна, которые не помещаются даже при кегле 12 после всех уступок §3а, — по языкам. \
             «Поперёк» и «вниз» — сколько точек не хватает при 12; «строка» — та, без которой окно \
             помещается (временный образец ×1,3 — помечен как строка без идентификатора). Печатает \
             прибор `tests\\ui_fit.rs` (`LANGSW_UI_FIT_WAVE`).\r\n\r\n",
        );
        text.push_str("| Окно | Язык | Класс | Гарнитура | Поперёк, т. | Вниз, т. | Строка |\r\n|---|---|---|---|---|---|---|\r\n");
        for line in &pass.wave {
            text.push_str(line);
            text.push_str("\r\n");
        }
        if pass.wave.is_empty() {
            text.push_str("\r\nПусто: окон волны нет.\r\n");
        }
        std::fs::write(wave, text).expect("the list of the wave written");
    }
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
        text.push_str("\r\n## Уступки §3а при кегле из таблицы — до какого шага дошло\r\n\r\n");
        for line in &pass.steps {
            text.push_str(&format!("- {line}\r\n"));
        }
        if pass.steps.is_empty() {
            text.push_str("Нет: все окна помещаются без уступок.\r\n");
        }
        text.push_str("\r\n## Исключения прибора (с причиной)\r\n\r\n");
        for excuse in EXCUSED {
            text.push_str(&format!("- {}\r\n", excuse_line(excuse)));
        }
        for line in &pass.excused {
            text.push_str(&format!("  - {line}\r\n"));
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

/// **The instrument fails where it must** — FR-92's positive control, from the test of the dialog
/// templates that left with them (T-120-19): English with every string 40 % longer no longer fits
/// at its factor of the table, in some window and class. Without it the green of
/// [`every_window_fits_every_language_at_the_factor_of_the_table`] says nothing.
#[test]
fn english_forty_percent_longer_does_not_fit_at_its_factor() {
    with_product_strings();
    let (mut windows, _) = windows_of(Language::En);
    for (_, window) in &mut windows {
        window.texts_mut(&mut |_, text| {
            let length = (text.chars().count() as f32 * 1.4).ceil() as usize;
            *text = text.chars().cycle().take(length).collect();
        });
    }
    let face = faces()[0];
    let text = Text::with_faces(
        face,
        MONO_FACES[1],
        Language::En.tag(),
        Direction::LeftToRight,
    )
    .expect("DirectWrite");
    let mut red = Vec::new();
    for (class, w, h) in CLASSES {
        let size = client_size(w, h);
        let k = fit::k_for(Language::En.tag(), class);
        for (name, window) in &windows {
            if !faults(window, size, k, false, &text).is_empty() {
                red.push(format!("{name}, {class:?}"));
            }
        }
    }
    assert!(
        !red.is_empty(),
        "English 40 % longer fits everywhere at its factor: the instrument cannot fail"
    );
}

/// **Every excuse has a category and a term** (208.175): a record without one is refused.
#[test]
fn every_excuse_has_a_category_and_a_term() {
    let mut refused = uncategorised(EXCUSED);
    // 208.236 п. 6: the strings excused from translation obey the same rule.
    refused.extend(uncategorised_strings(UNTRANSLATED));
    assert!(
        refused.is_empty(),
        "excuses without a category or a term (208.175):\n{}",
        refused.join("\n")
    );
}

/// **The gate of a release** (208.175, T-120-24): with `LANGSW_UI_FIT_RELEASE` set, as the build of
/// a release sets it, no excuse of the wave of translations is left and every excuse of the owner
/// names its decision. Before a release the instrument prints what is still open. Since 208.236
/// п. 6 the strings excused from translation as well: no English stand-in waiting for its wave
/// goes out in a release.
#[test]
fn at_a_release_every_excuse_is_settled() {
    let mut open = unsettled(EXCUSED);
    open.extend(unsettled_strings(UNTRANSLATED));
    if std::env::var_os("LANGSW_UI_FIT_RELEASE").is_some() {
        assert!(
            open.is_empty(),
            "a release with excuses not settled (208.175):\n{}",
            open.join("\n")
        );
    } else {
        println!("excuses to settle before a release: {}", open.len());
        for line in &open {
            println!("- {line}");
        }
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

/// **A button whose label is wider than its room is named with its key** (the C6 of ml
/// «Спасибо», 208.193): the label of the button of the card «Поддержать автора» ran over, and the
/// instrument named the lead — taking the lead away changed the steps of giving way so that the
/// window fitted. Here the label in English is made long — another string of the program, not in
/// this window — and the instrument must name its key, not a paragraph. It does not set the trap of the
/// case — a paragraph whose removal also makes the window fit — and does not catch the rule
/// «across» taken away; the next test does.
#[test]
fn a_button_too_wide_for_its_room_is_named_with_its_key() {
    with_product_strings();
    let names = string_names();
    let support = settings::text_in(Language::En, settings::IDS_SUPPORT_OPEN);
    let long = settings::text_in(Language::En, settings::IDS_HELLO_LEAD);
    let (windows, _) = windows_of(Language::En);
    let (_, thanks) = windows
        .iter()
        .find(|(name, _)| name == "letter thanks")
        .expect("«Спасибо» among the windows");
    let mut window = thanks.clone();
    let mut replaced = 0;
    window.texts_mut(&mut |role, s| {
        if matches!(role, Role::Button) && *s == support {
            *s = long.clone();
            replaced += 1;
        }
    });
    assert_eq!(replaced, 1, "the label of the button of the card");

    let face = faces()[0];
    let text = Text::with_faces(face, MONO_FACES[1], "en-US", Direction::LeftToRight)
        .expect("DirectWrite");
    let (class, w, h) = CLASSES[0];
    let size = client_size(w, h);
    let k = fit::k_for("en-US", class);
    assert!(
        !faults(&window, size, k, false, &text).is_empty(),
        "the long label runs over"
    );
    let named = culprit(Language::En, &window, size, k, false, &text, &names);
    assert!(
        named.starts_with("IDS_HELLO_LEAD"),
        "the label of the button is named with its key: {named}"
    );
}

/// **The case itself** (ml «Спасибо», 20.11): the label of the button of the card ran over, and
/// taking the lead away also made the window fit — the steps of giving way changed — so the
/// instrument named the lead. The label of that day is kept here, so that the case outlives the
/// translator's shorter label; if the window changes so that taking the lead away no longer makes
/// it fit, the case is gone and this test says so, not passes for nothing.
///
/// ⚠ Built anew on 22.11 (208.209): the action card took the frame of a block, and in the narrow
/// window the label of that day ran over the card with the lead or without it (60.7 points across).
/// The same label, lead and factor reproduce the case in the large window, whose column is wider.
/// The factor moved again on 23.11 (Пс2), see `K`.
#[test]
fn a_button_is_named_even_when_taking_a_paragraph_away_also_fits() {
    const LABEL: &str = "നിർമ്മാതാവിനെ പിന്തുണയ്ക്കുക";
    // The lead and the factor of ml Narrow of that day: the translator shortened the lead and the
    // table moved since, and the case lives on these (the wave of translations, 20.11).
    const LEAD: &str = "നമസ്കാരം, ഇത് വീണ്ടും പാണ്ട. ഒരു മാസം മുമ്പ് നിങ്ങൾ Lang Switcher ഇൻസ്റ്റാൾ ചെയ്തു, ഇത് ഉപകാരപ്പെടും എന്ന് കരുതുന്നു: പ്രോഗ്രാമിന്റെ ഭാരം കുറയ്ക്കാനും നന്നായി ഓടാനും ഞാൻ ഏറെ ശ്രമിച്ചു.";
    // ⚠ 0.914 until 23.11: the link of «Спасибо» went into its lead (208.217, Пс2), the letter
    // lost a line, and the case lives at 0.936…1.000 of the large window — the middle of it.
    const K: f32 = 0.95;
    with_product_strings();
    let names = string_names();
    let support = settings::text_in(Language::Ml, settings::IDS_SUPPORT_OPEN);
    let lead = settings::text_in(Language::Ml, settings::IDS_THANKS_LEAD);
    let (windows, _) = windows_of(Language::Ml);
    let (_, thanks) = windows
        .iter()
        .find(|(name, _)| name == "letter thanks")
        .expect("«Спасибо» among the windows");
    let mut window = thanks.clone();
    window.texts_mut(&mut |role, s| {
        if matches!(role, Role::Button) && *s == support {
            *s = LABEL.to_owned();
        // The lead and the sentence with the link after it (208.217, Пс2) — the lead of that day
        // alone, as the case was.
        } else if *s == lead || s.starts_with(&format!("{lead} ")) {
            *s = LEAD.to_owned();
        }
    });
    let face = faces()[0];
    let text = Text::with_faces(
        face,
        MONO_FACES[1],
        Language::Ml.tag(),
        Direction::LeftToRight,
    )
    .expect("DirectWrite");
    // The large window: the case of the narrow one went with the frames of the cards (see above).
    let (class, w, h) = CLASSES[2];
    let size = client_size(w, h);
    let _ = class;
    let k = K;
    assert!(
        !faults(&window, size, k, false, &text).is_empty(),
        "the label runs over"
    );
    let mut without_lead = window.clone();
    without_lead.texts_mut(&mut |_, s| {
        if *s == LEAD {
            s.clear();
        }
    });
    assert!(
        faults(&without_lead, size, k, false, &text).is_empty(),
        "the case is gone: taking the lead away no longer makes the window fit — build it anew: {:?}",
        faults(&without_lead, size, k, false, &text)
    );
    let named = culprit(Language::Ml, &window, size, k, false, &text, &names);
    assert!(
        named.contains(LABEL),
        "the label of the button is named: {named}"
    );
}

/// **The entry of the feed is the one a person sees** (word of the controller on the C6 of ml and ta,
/// 208.193): the instrument measures «Обновление» and «Новость» with the update of the last real
/// feed in the language of the window — its own entry, else English, else Russian, as the product's
/// `feed::parse_body` picks; the feed is read in the language of the program, not in Russian.
#[cfg(feature = "testing")]
#[test]
fn the_sample_of_the_feed_is_in_the_language_of_the_window() {
    use lang_switcher::letters;

    with_product_strings();
    let in_script = |s: &str, from: char, to: char| s.chars().any(|c| (from..=to).contains(&c));
    let ml = letters::sample_entry_in(Language::Ml);
    let ta = letters::sample_entry_in(Language::Ta);
    let ru = letters::sample_entry_in(Language::Ru);
    let en = letters::sample_entry_in(Language::En);
    assert!(
        in_script(&ml.title, '\u{0D00}', '\u{0D7F}'),
        "Malayalam: {}",
        ml.title
    );
    assert!(
        in_script(&ta.title, '\u{0B80}', '\u{0BFF}'),
        "Tamil: {}",
        ta.title
    );
    assert!(in_script(&ru.title, 'А', 'я'), "Russian: {}", ru.title);
    assert!(en.title.is_ascii(), "English: {}", en.title);
}

/// The letters that show an entry of the feed — «Новость» and «Обновление» — built in `language`
/// with `item`.
#[cfg(feature = "testing")]
fn feed_letters(
    language: Language,
    item: &lang_switcher::letters::FeedItem,
) -> Vec<(String, Window<u16>)> {
    use lang_switcher::ui::windows::letter;
    letter::states_with(language, item)
        .into_iter()
        .filter(|(name, _)| name == "letter news" || name == "letter update")
        .map(|(name, window)| (name, window.map(&letter::Id::number)))
        .collect()
}

/// Whether an entry of `kind` with a text of `points` code points fits its letter — «Обновление» for
/// an update, «Новость» for a news item (word of the owner, 2026-11-20) — in the
/// window of the reference (decision 208.197: the size it opens in — Ordinary 900 × 540, since
/// 208.227 on every work area that holds it; a smaller window scrolls) at the factor of the table, on every
/// main face, with the steps of
/// giving way the table is counted with (`Concession::TABLE`, as [`k_needed`] first tries): an entry
/// that fitted only with more steps would ask the table a lower factor.
#[cfg(feature = "testing")]
fn entry_fits(
    language: Language,
    points: usize,
    faces: &[&str],
    kind: lang_switcher::letters::FeedKind,
) -> bool {
    use lang_switcher::letters;
    let mut item = letters::sample_entry_in(language);
    item.text = letters::sample_text(language, points);
    let rtl = language.is_rtl();
    let direction = if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    };
    let letter = format!("letter {}", kind.name());
    let windows: Vec<_> = feed_letters(language, &item)
        .into_iter()
        .filter(|(name, _)| *name == letter)
        .collect();
    assert_eq!(windows.len(), 1, "the letter of {letter}");
    faces.iter().all(|face| {
        let text =
            Text::with_faces(face, MONO_FACES[1], language.tag(), direction).expect("DirectWrite");
        [CLASSES[1]].iter().all(|&(class, w, h)| {
            let size = client_size(w, h);
            let k = fit::k_for(language.tag(), class);
            // At the edge of its cap an entry may fit at the factor of the table and fail a step
            // under it — the lines break otherwise — and the search of the factor, which halves its
            // range, then lands under the table (measured: «Обновление» ru, fits at 0,916, the
            // search found 0,897). So the entry fits only where the search agrees. The search of the table
            // floors the factor to thousandths, and so does this: an entry that needed 0,8776 fitted
            // here and asked the table 0,877 (208.198, the merge of 4c3318d).
            windows.iter().all(|(_, window)| {
                faults_with(window, size, k, rtl, &text, Concession::TABLE).is_empty()
                    && k_needed(window, size, rtl, &text)
                        .is_some_and(|need| (need * 1000.0).floor() / 1000.0 + 0.0005 >= k)
            })
        })
    })
}

/// The cap of the text of an entry of `kind` in `language`: the longest that fits its letter
/// ([`entry_fits`]); 0 — the letter does not fit even without a text.
#[cfg(feature = "testing")]
fn cap_of(language: Language, faces: &[&str], kind: lang_switcher::letters::FeedKind) -> usize {
    const MOST: usize = 4000;
    if !entry_fits(language, 0, faces, kind) {
        return 0;
    }
    if entry_fits(language, MOST, faces, kind) {
        return MOST;
    }
    let (mut fits, mut not) = (0, MOST);
    while not - fits > 1 {
        let middle = (fits + not) / 2;
        if entry_fits(language, middle, faces, kind) {
            fits = middle;
        } else {
            not = middle;
        }
    }
    fits
}

/// **The caps of the feed, printed** (word of the controller on the C6 of ru and en, 208.193): the
/// factor of the table stays, and every entry of the feed is held to the length that fits its
/// letters in its language at that factor. With `LANGSW_FEED_CAPS_WRITE` set the caps are written to
/// `tools\feed-caps.tsv`, which `sign-news.ps1` reads; without it this test does nothing.
#[cfg(feature = "testing")]
#[test]
fn print_the_caps_of_the_feed() {
    if std::env::var_os("LANGSW_FEED_CAPS_WRITE").is_none() {
        return;
    }
    with_product_strings();
    let faces = faces();
    let mut text = String::from(
        "# The cap of the text of an entry of the feed (FR-102), by language and by the kind of the \
         entry:\r\n# tag<TAB>update<TAB>news. Code points, line breaks not counted. An update is \
         measured by the letter\r\n# \"Obnovlenie\", a news item by \"Novost\" -- the only windows \
         that show the whole text. Printed by the\r\n# instrument of fitting (tests/ui_fit.rs, \
         print_the_caps_of_the_feed with LANGSW_FEED_CAPS_WRITE);\r\n# read by \
         tools/sign-news.ps1, which refuses a longer text of that kind. 0 -- the letter does not\r\n# \
         fit even without a text in that language yet.\r\n",
    );
    for language in Language::ALL {
        let [update, news] =
            lang_switcher::letters::FeedKind::ALL.map(|kind| cap_of(language, &faces, kind));
        println!("{}\t{update}\t{news}", language.tag());
        text.push_str(&format!("{}\t{update}\t{news}\r\n", language.tag()));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join("feed-caps.tsv");
    std::fs::write(path, text).expect("the caps written");
}

/// **An entry of its cap fits its letter, in every language and of both kinds** — the gate of the caps
/// of the feed: the instrument measures «Обновление» with an update and «Новость» with a news item
/// as long as `sign-news.ps1` lets through for that kind, so a longer entry cannot lower the factor of the table. A language with no cap,
/// or whose entry of its cap no longer fits, asks the caps to be printed again.
#[cfg(feature = "testing")]
#[test]
fn every_entry_of_its_cap_fits_its_letter() {
    use lang_switcher::letters;
    with_product_strings();
    let faces = faces();
    let mut wrong = Vec::new();
    for (language, kind) in Language::ALL
        .into_iter()
        .flat_map(|language| letters::FeedKind::ALL.map(|kind| (language, kind)))
    {
        match letters::feed_cap(language, kind) {
            None => wrong.push(format!("{} {}: no cap", language.tag(), kind.name())),
            Some(0) => {}
            Some(cap) => {
                // Fitting is not monotone at the edge (the lines break otherwise), and the feed
                // lets through every length up to the cap: a few of them are tried, not the cap
                // alone.
                for points in [cap, cap * 3 / 4, cap / 2, cap / 4] {
                    if !entry_fits(language, points, &faces, kind) {
                        wrong.push(format!(
                            "{} {}: an entry of {points} points (the cap {cap}) does not fit",
                            language.tag(),
                            kind.name()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "print the caps again (LANGSW_FEED_CAPS_WRITE):\n{}",
        wrong.join("\n")
    );
}

/// **Every window of the catalogue draws in the plain look, in every state of its elements** (Z15,
/// 208.210): the look a machine falls back to when Direct2D does not come up is never seen on the
/// stand, and an empty field or label in it brought the process down. The windows as the product
/// gives them — an empty string stays empty, no stand-in — in a straight language and the mirror,
/// in the narrow and the ordinary class, laid out as the product lays them out.
#[test]
fn every_window_draws_in_the_plain_look_in_every_state() {
    use lang_switcher::ui::canvas::{Canvas, Mode};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::tokens::{Color, Theme};

    with_product_strings();
    for language in [Language::Ru, Language::He] {
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("DirectWrite");
        for (name, window) in catalogue(language) {
            for (class, w, h) in &CLASSES[..2] {
                let size = client_size(*w, *h);
                let layout = fit::emergency(
                    &window,
                    size,
                    fit::k_for(language.tag(), *class),
                    rtl,
                    &text,
                );
                let look = Look {
                    theme: Theme::Light,
                    rtl,
                    b: layout.b,
                    h: size.h,
                    text: &text,
                };
                let mut canvas =
                    Canvas::in_memory(size.w, size.h, 1.0, Mode::Gdi).expect("a canvas");
                let drawn = canvas.frame(|painter| {
                    painter.clear(Color::Bg.get(Theme::Light));
                    for item in &layout.items {
                        for state in State::ALL {
                            elements::draw(painter, &look, &item.part, item.rect, *state);
                        }
                    }
                });
                assert_eq!(
                    drawn,
                    Mode::Gdi,
                    "{name} {}: the plain look",
                    language.tag()
                );
            }
        }
    }
}

/// **The brand line of the plain look: two whole lines, over nothing** (решение 208.220, слово
/// владельца «можно сделать в 2 строчки, и тогда влезет»). GDI sets a face wider than DirectWrite
/// measures it, and on one line the plain look drew «Panda K…» and cut the address. In every window
/// with the brand line, in ru and he, in the two classes with a panel, at 100, 125 and 150 %: the
/// name stands over the address; no other part of the window crosses either; GDI draws each in its
/// place pixel for pixel as in a box as wide as the window — nothing cut, no ellipsis — and its ink
/// stays inside its own line.
#[test]
fn the_brand_line_of_the_plain_look_is_two_whole_lines_over_nothing() {
    use lang_switcher::ui::canvas::{Canvas, Image, Mode, Rect};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::tokens::{Color, Theme};

    with_product_strings();
    let theme = Theme::Light;
    let bg = Color::Bg.get(theme);
    let crosses = |a: Rect, b: Rect| {
        let e = 0.01;
        a.x < b.right() - e && b.x < a.right() - e && a.y < b.bottom() - e && b.y < a.bottom() - e
    };
    let ink = |image: &Image| {
        let mut found: Option<(u32, u32, u32, u32)> = None;
        for y in 0..image.height {
            for x in 0..image.width {
                if image.at(x, y) != bg {
                    let (l, t, r, b) = found.unwrap_or((x, y, x, y));
                    found = Some((l.min(x), t.min(y), r.max(x), b.max(y)));
                }
            }
        }
        found
    };
    let mut seen = 0;
    for language in [Language::Ru, Language::He] {
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("DirectWrite");
        for (name, window) in catalogue(language) {
            if window.brand.is_none() {
                continue;
            }
            for (class, w, h) in &CLASSES[1..] {
                let size = client_size(*w, *h);
                let k = fit::k_for(language.tag(), *class);
                let layout = fit::emergency_for(&window, size, k, rtl, &text, true);
                let tag = language.tag();
                let brand: Vec<usize> = (0..layout.items.len())
                    .filter(|&i| match &layout.items[i].part {
                        Part::BrandName { .. } => true,
                        Part::Link { role, .. } => *role == Role::Brand,
                        _ => false,
                    })
                    .collect();
                assert_eq!(
                    brand.len(),
                    2,
                    "{name} {tag} {class:?}: the name and the address"
                );
                let (name_r, address_r) =
                    (layout.items[brand[0]].rect, layout.items[brand[1]].rect);
                assert!(
                    (name_r.bottom() - address_r.y).abs() < 0.01
                        && (name_r.x - address_r.x).abs() < 0.01
                        && (name_r.w - address_r.w).abs() < 0.01,
                    "{name} {tag} {class:?}: the name over the address, one column: {name_r:?} \
                     {address_r:?}"
                );
                for (index, item) in layout.items.iter().enumerate() {
                    if brand.contains(&index) || matches!(item.part, Part::Panel) {
                        continue;
                    }
                    for line in [name_r, address_r] {
                        assert!(
                            !crosses(item.rect, line),
                            "{name} {tag} {class:?}: {:?} at {:?} crosses the brand line {line:?}",
                            item.part,
                            item.rect
                        );
                    }
                }

                let look = Look {
                    theme,
                    rtl,
                    b: layout.b,
                    h: size.h,
                    text: &text,
                };
                for scale in [1.0_f32, 1.25, 1.5] {
                    for &index in &brand {
                        let (part, rect) = (&layout.items[index].part, layout.items[index].rect);
                        let drawn = |rect: Rect| {
                            let mut canvas = Canvas::in_memory(size.w, size.h, scale, Mode::Gdi)
                                .expect("a canvas");
                            canvas.frame(|painter| {
                                painter.clear(bg);
                                elements::draw(painter, &look, part, rect, State::NORMAL);
                            });
                            canvas.image().expect("pixels")
                        };
                        // As wide as the window, from the edge it starts at.
                        let wide = if rtl {
                            Rect::new(rect.right() - size.w, rect.y, size.w, rect.h)
                        } else {
                            Rect::new(rect.x, rect.y, size.w, rect.h)
                        };
                        let (own, free) = (drawn(rect), drawn(wide));
                        let at = |x: u32, y: u32| own.at(x, y) == free.at(x, y);
                        let same = (0..own.height).all(|y| (0..own.width).all(|x| at(x, y)));
                        assert!(
                            same,
                            "{name} {tag} {class:?} {}: {part:?} is cut in its line {rect:?}",
                            scale * 100.0
                        );
                        let (l, t, r, b) = ink(&own).expect("ink");
                        let px = |v: f32| v * scale;
                        assert!(
                            l as f32 >= px(rect.x) - 1.0
                                && r as f32 <= px(rect.right()) + 1.0
                                && t as f32 >= px(rect.y) - 1.0
                                && b as f32 <= px(rect.bottom()) + 1.0,
                            "{name} {tag} {class:?} {}: the ink of {part:?} ({l}, {t})…({r}, {b}) \
                             leaves its line {rect:?}",
                            scale * 100.0
                        );
                    }
                }
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "no window with the brand line");
    println!("the brand line in the plain look: {seen} windows and classes");
}
