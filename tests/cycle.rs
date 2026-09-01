//! Integration tests for task T-05-2 — the cycle of §4.4 driven end to end.
//!
//! # What is assembled here and why it is a file of its own
//!
//! The requirements this file is about are **FR-32** and **FR-33**, and neither of them is a
//! property of one module. FR-32 says the buffer keeps the original strokes and the position
//! counter moves instead of them; the strokes live in `buffer`, the counter lives beside them,
//! the list of layouts and the step along it live in `layouts`, and the rendering lives in
//! `convert`. A test that stayed inside any one of the three could show that the piece works and
//! not that the cycle closes.
//!
//! So [`press`] below is the product's own hotkey path with exactly one thing left out: the
//! `SendInput` of FR-41. Everything that decides *what would be sent* is the same code in the
//! same order as [`lang_switcher::inject::on_hotkey`] — read the strokes, read the live cache,
//! build the cycle, take the step, render, advance the counter — and what it answers is the text
//! the replacement would have typed. The injection itself is `tests\inject.rs`'s and needs a
//! hook; the arithmetic of the cycle needs neither.
//!
//! # Printing
//!
//! `Stroke` has no `Debug` on purpose (SEC-01, SEC-07), so the strokes are compared with `==`
//! and a message rather than with `assert_eq!`. The strings that do appear in messages are the
//! constants of this file: synthetic layouts built by hand, never anybody's typing.
//!
//! # The behavioural run at the bottom of the file
//!
//! One test — points 23, 24 and 25 of the task — starts the **real product** and types into a
//! window of its own. It carries `#[ignore]`, because it runs a program with a live global
//! keyboard hook and switches the layout of the machine somebody is sitting at, and it is run by
//! name:
//!
//! ```text
//! cargo test --test cycle -- --ignored --nocapture --test-threads=1
//! ```
//!
//! ⚠ **Decision Р-42 throughout.** The only process it ends is the one it started, by the handle
//! `Command::spawn` answered with; the only window it drives is the one it created; nothing is
//! sent until `GetForegroundWindow` has answered with that window, and if it never does the run
//! says so and sends nothing. The layout the machine was on is put back however the test ends.

use lang_switcher::buffer::{Recorder, Stroke};
use lang_switcher::convert::{Keystroke, convert_strokes, max_units};
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::layouts::{
    Configured, KeyMapping, LayoutCache, LayoutId, LayoutMap, LayoutMapBuilder, MAX_CYCLE, Mods,
    Session, cycle_for,
};
use lang_switcher::settings::{LayoutMode, Layouts};

// -------------------------------------------------------------------------------------
// Three synthetic layouts on the six keys of the §11.3 scenario
// -------------------------------------------------------------------------------------

/// US, `00000409` — the layout the strokes below are recorded under.
const EN: LayoutId = LayoutId::from_raw(0x0409_0409);
/// Russian, `00000419` — the second layout of the pair (decision 19).
const RU: LayoutId = LayoutId::from_raw(0x0419_0419);
/// Greek, `00000408` — the third layout of position 17 of the matrix, footnote 3 of §11.3.
///
/// Synthetic here: the tests of this file attach nothing to the session and ask the OS nothing.
const EL: LayoutId = LayoutId::from_raw(0x0408_0408);

/// The six keys of the §11.3 scenario, in the order the position types them: `ghbdtn`.
///
/// Scan codes of the main block of a US keyboard: `G`, `H`, `B`, `D`, `T`, `N`.
const KEYS: [(u16, u16); 6] = [
    (0x47, 0x22),
    (0x48, 0x23),
    (0x42, 0x30),
    (0x44, 0x20),
    (0x54, 0x14),
    (0x4E, 0x31),
];

/// What those six keys give in each of the three layouts, key for key.
///
/// The English row is the scenario's `ghbdtn` and the Russian row is its `привет`, which is the
/// pair the matrix of §11.3 is written in. The Greek row is a third alphabet, distinct from both,
/// so that a step taken in the wrong direction is visible in the result rather than plausible.
const IN_ENGLISH: [char; 6] = ['g', 'h', 'b', 'd', 't', 'n'];
const IN_RUSSIAN: [char; 6] = ['п', 'р', 'и', 'в', 'е', 'т'];
const IN_GREEK: [char; 6] = ['γ', 'ρ', 'β', 'δ', 'τ', 'ν'];

/// The text of the scenario, in each of the three layouts.
const TYPED: &str = "ghbdtn";
const CONVERTED: &str = "привет";
const IN_THE_THIRD: &str = "γρβδτν";

/// A key of the main block: the `LLKHF_EXTENDED` of FR-05 is clear.
const MAIN_BLOCK: bool = false;

/// The space bar — task **Т-24-3**, the soft boundary of FR-10.
///
/// `(virtual key, scan code)`, in the same shape as [`KEYS`]. It is not one of the six: the six
/// are the word, and this is what follows it.
const SPACE: (u16, u16) = (0x20, 0x39);

/// Builds the map of one layout over the six keys — and over the space bar.
///
/// The space is in **every** layout with the same character, which is not decoration: FR-22
/// converts by the physical key, so the tail of the soft boundary is carried through a
/// conversion by the ordinary rule and not by an exception. A synthetic layout that left the
/// space bar blank would have made the whole of task Т-24-3 vacuous.
fn map_of(layout: LayoutId, characters: [char; 6]) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(layout);

    for (&(_, scan), &character) in KEYS.iter().zip(characters.iter()) {
        builder.set(
            scan,
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::from_char(character),
        );
    }

    builder.set(SPACE.1, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char(' '));

    builder.finish()
}

/// The cache of FR-20 over the layouts `layouts` names, in that order.
fn cache_of(layouts: &[LayoutId]) -> LayoutCache {
    let maps = layouts
        .iter()
        .map(|&layout| match layout {
            RU => map_of(RU, IN_RUSSIAN),
            EL => map_of(EL, IN_GREEK),
            other => map_of(other, IN_ENGLISH),
        })
        .collect();

    LayoutCache::from_maps(maps).expect("non-empty maps")
}

/// A recorder holding `ghbdtn`, typed under English, with `layouts` in its cache.
fn typed_in_english(layouts: &[LayoutId]) -> Recorder {
    let mut recorder = Recorder::with_capacity(64);
    recorder.set_cache(cache_of(layouts));
    recorder.set_active_layout(EN);

    for &(vk, scan) in &KEYS {
        recorder.record(KeyEvent {
            vk,
            edge: Edge::Down,
            extra_info: 0,
            scan,
            flags: 0,
            time: 1_000,
        });
    }

    recorder
}

/// The same recorder, with `spaces` presses of the space bar after the word — task **Т-24-3**.
///
/// This is the state the soft boundary of FR-10 leaves behind: «слово + хвост». The strokes go
/// in through [`Recorder::record`] like every other, which is the point — the tail is the ring's
/// own content and not a second buffer beside it.
fn typed_in_english_with_tail(layouts: &[LayoutId], spaces: usize) -> Recorder {
    let mut recorder = typed_in_english(layouts);

    for _ in 0..spaces {
        recorder.record(KeyEvent {
            vk: SPACE.0,
            edge: Edge::Down,
            extra_info: 0,
            scan: SPACE.1,
            flags: 0,
            time: 1_000,
        });
    }

    recorder
}

/// The configuration of `[layouts]`, as section 7 writes it.
fn configured(mode: LayoutMode, pair: [&str; 2], cycle: &[&str]) -> Configured {
    Configured::from_settings(&Layouts {
        mode,
        pair_source: pair[0].to_owned(),
        pair_target: pair[1].to_owned(),
        cycle: cycle.iter().map(|entry| (*entry).to_owned()).collect(),
    })
}

/// The strokes the recorder is holding, copied out for comparison.
fn strokes_of(recorder: &Recorder) -> Vec<Stroke> {
    (0..recorder.len())
        .map(|index| recorder.stroke(index).expect("index below the length"))
        .collect()
}

/// What the strokes in the buffer produced when they were made — the text as typed.
///
/// A convenience over data this test file put in itself; the product has no such function and
/// could not have one (SEC-07).
fn recorded_text(recorder: &Recorder) -> String {
    let units: Vec<u16> = strokes_of(recorder)
        .iter()
        .flat_map(|stroke| stroke.units().to_vec())
        .collect();

    String::from_utf16(&units).expect("the synthetic layouts carry valid text")
}

// -------------------------------------------------------------------------------------
// One press of the hotkey — the path of `inject::on_hotkey`, without the `SendInput`
// -------------------------------------------------------------------------------------

/// What one press of the hotkey would type, and the counter moved on by one.
///
/// Line for line the same decisions as `inject::take_press` and the conversion inside
/// `inject::replace_in_with`, in the same order:
///
/// 1. the strokes are **read** out of the buffer and the buffer is not touched — FR-32;
/// 2. the participating layouts come from the live cache — FR-35, NFR-09;
/// 3. the cycle comes from `[layouts]` and the session — FR-30, FR-31;
/// 4. the step is the position counter plus one, and the target is `origin + step` — FR-33;
/// 5. the original strokes are rendered into that layout — FR-22;
/// 6. the counter advances, and only then — FR-32.
fn press(recorder: &mut Recorder, settings: Configured) -> String {
    let (text, length) = {
        let cache = recorder.cache().expect("a cache was published");

        let mut available = [LayoutId::default(); MAX_CYCLE];
        let count = cache.layouts(&mut available);

        let cycle = cycle_for(
            settings,
            &available[..count],
            Session::of(count, cache.len()),
        )
        .expect("a usable cycle");

        let mut strokes = vec![Keystroke::default(); recorder.len()];
        let live = recorder.keystrokes(&mut strokes).expect("sized from len()");

        // FR-26: the direction is the HKL recorded with the stroke, not the layout that is
        // active now — which, after the first press, is the previous target.
        let origin = strokes
            .first()
            .map_or_else(|| recorder.active_layout(), |stroke| stroke.layout());
        let step = recorder.cycle_position() + 1;
        let target = cycle
            .target(origin, step)
            .expect("a target for the layout the strokes were typed in");
        let map = cache.get(target).expect("the target is in the cache");

        let mut units = vec![0u16; max_units(live)];
        let written =
            convert_strokes(&strokes[..live], map, &mut units).expect("sized by max_units");

        (
            String::from_utf16(&units[..written]).expect("the synthetic layouts carry text"),
            cycle.len(),
        )
    };

    recorder.advance_cycle(length);

    text
}

// -------------------------------------------------------------------------------------
// Point 9 — FR-32: the buffer is never written back
// -------------------------------------------------------------------------------------

/// The strokes after any number of conversions are the strokes the user made.
///
/// This is the requirement the whole task turns on, and it is checked on the *strokes* and not
/// on the text: a buffer overwritten with the conversion would still answer `привет` on the
/// first press, and only the second or the third would show the damage. Here the recording
/// itself is compared, field for field, after three presses in a cycle of three.
#[test]
fn the_buffer_holds_the_original_strokes_after_every_press() {
    let mut recorder = typed_in_english(&[EN, RU, EL]);
    let settings = configured(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419", "0x00000408"],
    );

    let before = strokes_of(&recorder);

    for _ in 0..3 {
        press(&mut recorder, settings);

        let now = strokes_of(&recorder);

        assert_eq!(
            now.len(),
            before.len(),
            "FR-32: a conversion adds and removes nothing"
        );
        assert!(
            now.iter().zip(&before).all(|(now, before)| now == before),
            "FR-32: the buffer holds the original keystrokes, unchanged"
        );
    }
}

// -------------------------------------------------------------------------------------
// Point 10 — FR-32 and position 17: three passes over a cycle of three
// -------------------------------------------------------------------------------------

/// `N` conversions round a cycle of `N` layouts give back the original text, code unit for code
/// unit — §11.1, and position 17 of the matrix of §11.3.
#[test]
fn three_presses_round_a_cycle_of_three_restore_the_original_text() {
    let mut recorder = typed_in_english(&[EN, RU, EL]);
    let settings = configured(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419", "0x00000408"],
    );

    // FR-31: "первое нажатие — вариант 2, второе — вариант 3, и так далее по кругу с возвратом
    // к исходному варианту".
    assert_eq!(press(&mut recorder, settings), CONVERTED);
    assert_eq!(press(&mut recorder, settings), IN_THE_THIRD);
    assert_eq!(press(&mut recorder, settings), TYPED);

    // And it keeps closing: the fourth press is the first one again.
    assert_eq!(press(&mut recorder, settings), CONVERTED);

    // Bit for bit, not merely "looks the same": the code units of the restored text are the
    // code units of what was typed.
    let restored: Vec<u16> = TYPED.encode_utf16().collect();
    let round_trip: Vec<u16> = {
        press(&mut recorder, settings);
        press(&mut recorder, settings).encode_utf16().collect()
    };

    assert_eq!(round_trip, restored, "FR-32: побитово точный возврат");
}

// -------------------------------------------------------------------------------------
// Point 11 — FR-33 and position 16: two presses in mode `pair`
// -------------------------------------------------------------------------------------

/// Two presses of the hotkey in a row give back what was typed — the rollback of FR-33, which
/// position 16 of the matrix of §11.3 asserts.
#[test]
fn two_presses_in_pair_mode_restore_the_original_text() {
    let mut recorder = typed_in_english(&[EN, RU]);
    let settings = configured(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );

    assert_eq!(press(&mut recorder, settings), CONVERTED);
    assert_eq!(press(&mut recorder, settings), TYPED);

    // A pair is a cycle of two and nothing else, so it goes on alternating for as long as the
    // buffer lives.
    assert_eq!(press(&mut recorder, settings), CONVERTED);
    assert_eq!(press(&mut recorder, settings), TYPED);

    // And the strokes are still the strokes — FR-32 is what makes the rollback exact rather
    // than approximate.
    assert_eq!(recorded_text(&recorder), TYPED);
}

/// The rollback survives step 5 of FR-40 — the switch the *first* press performs.
///
/// ⚠ The trap this test exists for. Step 5 switches the foreground window to the layout the
/// replacement was rendered into, and the product learns of it (FR-21) and republishes it as the
/// active layout. A cycle counted from "the layout in use" would therefore start the second
/// press from the target of the first one and answer that same layout again — the rollback of
/// FR-33 would stop working, silently, from the second press onwards. FR-26 settles it: the
/// direction comes from **the HKL recorded with the stroke**, and FR-32 keeps that recording
/// intact for as long as the buffer lives.
#[test]
fn the_rollback_does_not_depend_on_the_layout_the_switch_left_behind() {
    let mut recorder = typed_in_english(&[EN, RU]);
    let settings = configured(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );

    assert_eq!(press(&mut recorder, settings), CONVERTED);

    // Step 5 of FR-40 has just switched the window to Russian, and `app` publishes it — FR-11,
    // which flushes nothing, so the strokes and the counter are still here.
    recorder.set_active_layout(RU);

    assert_eq!(recorder.len(), 6);
    assert_eq!(recorder.cycle_position(), 1);

    // The second press still rolls back, because the strokes still say what they were typed in.
    assert_eq!(press(&mut recorder, settings), TYPED);

    recorder.set_active_layout(EN);

    assert_eq!(press(&mut recorder, settings), CONVERTED);
}

// -------------------------------------------------------------------------------------
// Task Т-24-3 — the soft boundary of FR-10: the word converts with its tail
// -------------------------------------------------------------------------------------

/// **The whole of what the user asked for, at the level where the text is made.**
///
/// The complaint, word for word (question 82): «часто набираю слово, потом ставлю пробел, а
/// только потом вижу что выбрана не та раскладка и нажимаю `Pause`, но переключения уже не
/// происходит». Since task Т-24-2 the space leaves «слово + хвост» in the ring instead of an
/// empty buffer, and this is what the hotkey then makes of it: the word is converted and the
/// tail is carried over untouched, because FR-22 converts by the physical key and the space bar
/// gives a space in the target layout too.
#[test]
fn the_tail_of_the_soft_boundary_is_converted_with_the_word() {
    let mut recorder = typed_in_english_with_tail(&[EN, RU], 1);
    let settings = configured(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );

    assert_eq!(recorder.len(), 7, "six keys of the word and the tail");
    assert_eq!(recorded_text(&recorder), "ghbdtn ");

    assert_eq!(
        press(&mut recorder, settings),
        "привет ",
        "the word is converted and the trailing space is still there"
    );
}

/// **The rollback of FR-32 works with the tail exactly as it works without one.**
///
/// The second press gives back the word **and** the spaces, code unit for code unit, and the
/// buffer is still holding the strokes the user made — which is the property the whole cycle
/// rests on. Position 26 of the matrix of §11.3 is this test with a human's hands.
#[test]
fn the_rollback_gives_the_word_and_its_tail_back_bit_for_bit() {
    let mut recorder = typed_in_english_with_tail(&[EN, RU], 1);
    let settings = configured(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );

    let before = strokes_of(&recorder);

    assert_eq!(press(&mut recorder, settings), "привет ");
    assert_eq!(press(&mut recorder, settings), "ghbdtn ");

    // Bit for bit, and not merely "looks the same".
    let restored: Vec<u16> = "ghbdtn ".encode_utf16().collect();
    let round_trip: Vec<u16> = {
        press(&mut recorder, settings);
        press(&mut recorder, settings).encode_utf16().collect()
    };
    assert_eq!(
        round_trip, restored,
        "FR-32: побитово точный возврат с хвостом"
    );

    // FR-32 again, from the other end: the ring still holds what was typed, tail included.
    let now = strokes_of(&recorder);
    assert_eq!(now.len(), before.len());
    assert!(
        now.iter().zip(&before).all(|(now, before)| now == before),
        "FR-32: a conversion adds and removes nothing, and the tail is no exception"
    );
    assert_eq!(recorded_text(&recorder), "ghbdtn ");
}

/// **A tail of several spaces goes round the whole cycle, and every step keeps its length.**
///
/// Rule 2 of the soft boundary: the spaces pile up. Three layouts, three presses, and the tail
/// is two characters wide at every one of them — which is also what keeps the erasure of FR-41
/// exact, since the count of what stands on the screen is a count of these same units.
#[test]
fn a_tail_of_several_spaces_is_carried_round_the_whole_cycle() {
    let mut recorder = typed_in_english_with_tail(&[EN, RU, EL], 2);
    let settings = configured(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419", "0x00000408"],
    );

    assert_eq!(
        recorder.len(),
        8,
        "six keys of the word and two of the tail"
    );

    for expected in ["привет  ", "γρβδτν  ", "ghbdtn  "] {
        let shown = press(&mut recorder, settings);

        assert_eq!(shown, expected);
        assert_eq!(
            shown.chars().count(),
            8,
            "every step of the cycle stands eight characters wide: {shown:?}"
        );
    }
}

// -------------------------------------------------------------------------------------
// Point 12 — FR-33: "Пара" and "Цикл" are one mechanism
// -------------------------------------------------------------------------------------

/// The two modes of §4.4 are the same code with a different list, and this is what "без
/// отдельной реализации" means in practice.
///
/// Three statements, and all three are about the *code* rather than about the outcome:
///
/// 1. both modes are produced by the one function `layouts::cycle_for`, which is the only public
///    constructor of a `Cycle` from a configuration — the assertion below compares what it
///    answers for the two modes over the same two layouts, and they are the **same value**;
/// 2. a `Cycle` has no mode in it: [`lang_switcher::layouts::Cycle`] carries a list and nothing
///    else, so there is nothing below this line that could branch on one;
/// 3. the stepping is `Cycle::target` for both, so the sequence of presses is identical.
#[test]
fn pair_and_cycle_are_the_same_mechanism_with_a_different_list() {
    let available = [EN, RU];

    let as_pair = cycle_for(
        configured(
            LayoutMode::Pair,
            ["0x00000409", "0x00000419"],
            &["0x00000409", "0x00000419"],
        ),
        &available,
        Session::Whole,
    )
    .expect("the pair of decision 19");

    let as_cycle = cycle_for(
        configured(
            LayoutMode::Cycle,
            ["0x00000409", "0x00000419"],
            &["0x00000409", "0x00000419"],
        ),
        &available,
        Session::Whole,
    )
    .expect("the same two layouts as a cycle");

    assert_eq!(
        as_pair, as_cycle,
        "FR-33: mode `pair` is a cycle of two and not a second implementation"
    );
    assert_eq!(as_pair.len(), 2);

    // The same conclusion from the other end: driven through the whole path, press by press,
    // the two modes produce the same text at every step.
    let mut in_pair = typed_in_english(&available);
    let mut in_cycle = typed_in_english(&available);

    for _ in 0..4 {
        let from_pair = press(
            &mut in_pair,
            configured(
                LayoutMode::Pair,
                ["0x00000409", "0x00000419"],
                &["0x00000409", "0x00000419"],
            ),
        );
        let from_cycle = press(
            &mut in_cycle,
            configured(
                LayoutMode::Cycle,
                ["0x00000409", "0x00000419"],
                &["0x00000409", "0x00000419"],
            ),
        );

        assert_eq!(from_pair, from_cycle);
    }
}

// -------------------------------------------------------------------------------------
// Behavioural — points 23, 24 and 25, against the real product, in our own window
// -------------------------------------------------------------------------------------

/// **Points 23, 24 and 25.** `ghbdtn` → `привет` → `ghbdtn` → `привет` → `ghbdtn`, in a window
/// this test creates, against the product this test starts.
///
/// This is the scenario of §11.3 with position 16 of the matrix on top of it: the hotkey pressed
/// four times in a row, with the text and the layout of the window read back after each press.
/// The second press is the one the task is about — the rollback of FR-33 — and the text it
/// leaves has to be the **original** one, code unit for code unit, which is FR-32.
///
/// ⚠ `#[ignore]`: it starts a program with a live global keyboard hook (with the deadline of
/// FR-97 set to 45 seconds) and changes the keyboard layout of the machine. Run by name.
#[test]
#[ignore = "starts the product with a live global hook and switches the layout; run with --ignored"]
fn behavioural_four_presses_alternate_and_restore_the_original_text() {
    use own_window::{Bench, Product, Restore};

    let session = lang_switcher::layouts::enumerate().expect("the session has usable layouts");
    let names: Vec<String> = session.iter().map(ToString::to_string).collect();
    println!("layouts in this session: {names:?}");

    let us = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0409);
    let ru = session
        .iter()
        .copied()
        .find(|layout| layout.language_id() == 0x0419);

    let (Some(us), Some(ru)) = (us, ru) else {
        println!("SKIPPED: this session does not carry both US and Russian");
        return;
    };

    // The product first: its hook must be up before anything is typed.
    let product = Product::start();
    println!("product started, pid {}", product.id());

    let Some(bench) = Bench::open() else {
        println!("SKIPPED: our own window did not reach the foreground; nothing was typed");
        product.stop();
        return;
    };

    let started_on = lang_switcher::switch::current();
    println!("layout of our window before anything: {started_on}");
    let _restore = Restore::to(started_on, bench.handle());

    // The scenario of §11.3 starts in the English layout.
    let outcome = lang_switcher::switch::to(us);
    println!("window set to US: {outcome:?}");
    assert_eq!(lang_switcher::switch::current(), us);

    // The product's own layout probe — task T-03-3c. Without it the product still believes the
    // window is on the layout it had when it first saw it; a person typing `ghbdtn` in English
    // has already given the product such an event, and a test has to give it one on purpose.
    own_window::tap_shift();

    own_window::type_ascii(TYPED);
    let before = bench.wait_for_text(|text| text == TYPED);
    println!("after typing: {before:?}");
    assert_eq!(
        before.as_deref(),
        Some(TYPED),
        "the run starts from `ghbdtn`"
    );

    // ---- point 23: the first press ------------------------------------------------------
    own_window::press_hotkey();
    let after = bench.wait_for_text(|text| text == CONVERTED);
    let layout = lang_switcher::switch::current();
    println!("point 23: press 1 -> text {after:?}, layout {layout}");

    assert_eq!(after.as_deref(), Some(CONVERTED), "point 23: the text half");
    assert_eq!(layout, ru, "point 23: the layout half");

    // ---- point 24: the second press, the rollback of FR-33 -------------------------------
    own_window::press_hotkey();
    let after = bench.wait_for_text(|text| text == TYPED);
    let layout = lang_switcher::switch::current();
    println!("point 24: press 2 -> text {after:?}, layout {layout}");

    assert_eq!(
        after.as_deref(),
        Some(TYPED),
        "point 24: FR-33 — the second press gives back what was typed"
    );
    assert_eq!(
        after.map(|text| text.encode_utf16().collect::<Vec<u16>>()),
        Some(TYPED.encode_utf16().collect::<Vec<u16>>()),
        "point 24: FR-32 — code unit for code unit"
    );
    assert_eq!(layout, us, "point 24: and the layout went back with it");

    // ---- point 25: the third and the fourth ----------------------------------------------
    own_window::press_hotkey();
    let third = bench.wait_for_text(|text| text == CONVERTED);
    let third_layout = lang_switcher::switch::current();
    println!("point 25: press 3 -> text {third:?}, layout {third_layout}");

    own_window::press_hotkey();
    let fourth = bench.wait_for_text(|text| text == TYPED);
    let fourth_layout = lang_switcher::switch::current();
    println!("point 25: press 4 -> text {fourth:?}, layout {fourth_layout}");

    assert_eq!(third.as_deref(), Some(CONVERTED), "point 25: press 3");
    assert_eq!(third_layout, ru);
    assert_eq!(fourth.as_deref(), Some(TYPED), "point 25: press 4");
    assert_eq!(fourth_layout, us);

    // ---- put everything back --------------------------------------------------------------
    let outcome = lang_switcher::switch::to(started_on);
    println!("restoring {started_on} -> {outcome:?}");
    assert_eq!(lang_switcher::switch::current(), started_on);

    product.stop();
}

// -------------------------------------------------------------------------------------
// Task Т-25-2 — a behavioural test that falls over leaves no product behind
// -------------------------------------------------------------------------------------

/// **A panic after the product is up still takes the process with it** — finding м-Э24-2 of
/// stage Э24, the user's decision 84.2.
///
/// # What went wrong and how it was measured
///
/// The behavioural test above starts the product on its first working line and ends it on its
/// last. Every assertion in between can panic, and a panic walks past `product.stop()` exactly
/// as it walks past everything else. Stage Э24 measured what that leaves: `pid 7624`, alive
/// **358 seconds** under a deadline of 45, with the window title `Lang Switcher`. It is not the
/// deadline of FR-97 failing — a second instance never reaches it. It is FR-82: the next launch
/// finds the abandoned predecessor, puts up a modal notification and stands on it for ever,
/// because a modal message loop is not a place a timer thread can reach.
///
/// # Why this stand and not a reading of the source
///
/// `Drop` running on the unwinding path is a property of the language, so a test that asserted
/// the presence of an `impl Drop` would prove nothing about this program. This one takes the
/// real product up, panics on purpose inside [`std::panic::catch_unwind`] — which unwinds, and
/// therefore drops, exactly as a failing test does — and then asks the operating system whether
/// the process is still there. Before task Т-25-2 it was.
///
/// ⚠ The default panic hook is silenced for the length of the `catch_unwind` and put back
/// immediately: the panic here is the instrument, not a fault, and its message on the console
/// would read as one. The hook is process-wide, which is one more reason this test belongs among
/// the `#[ignore]`d ones that run one at a time.
///
/// ⚠ **It measures only when it really raised an instance, and says so otherwise.** If another
/// copy of the product already holds the mutex of FR-82 — the installed one, say — this launch
/// leaves of its own accord and there is nothing here to abandon; the stand prints `SKIPPED` and
/// claims nothing, in the idiom the behavioural test above uses for a missing layout. Run it with
/// no other copy of the product running and it measures.
#[test]
#[ignore = "starts the product with a live global hook and panics on purpose; run with --ignored"]
fn a_panic_after_the_product_is_up_leaves_no_process_behind() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    let raised = Arc::new(AtomicU32::new(0));
    let inside = Arc::clone(&raised);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(move || {
        let product = own_window::Product::start();
        let pid = product.id();
        inside.store(pid, Ordering::Release);

        // Nothing came up to be abandoned — the caller below reports it rather than panicking
        // into a measurement that would be about an already dead process.
        if !own_window::process_is_alive(pid) {
            return;
        }

        panic!("нарочно: так падает поведенческий тест на середине");
    });
    std::panic::set_hook(previous);

    let pid = raised.load(Ordering::Acquire);

    if outcome.is_ok() {
        println!(
            "SKIPPED: экземпляр (pid {pid}) не встал — мьютекс FR-82 держит другая копия \
             продукта; стенд ничего не мерил"
        );
        return;
    }

    println!("стенд: поднят pid {pid}, затем нарочная паника");

    assert_ne!(pid, 0, "экземпляр не поднялся: мерить нечего");
    assert!(
        !own_window::process_is_alive(pid),
        "pid {pid} пережил панику: брошенный экземпляр держит второй хук и файл на диске, \
         а следующий запуск встаёт на модальном окне FR-82 — находка м-Э24-2"
    );
}

/// A window this test owns, the product it starts, and the restoration of the layout it found.
///
/// The same construction `tests\switch.rs` established for the behavioural checks of task
/// T-05-1, kept here rather than shared because an integration test is a crate of its own and
/// `tests\switch.rs` is not this task's file to touch.
#[cfg(test)]
mod own_window {
    use core::ffi::c_void;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use lang_switcher::layouts::LayoutId;
    use lang_switcher::switch;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        AttachThreadInput, GetCurrentThreadId, GetExitCodeProcess, OpenProcess,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, SetFocus, VIRTUAL_KEY, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, CreateWindowExW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
        GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, SW_SHOW, SendMessageW,
        SetForegroundWindow, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_GETTEXT, WS_BORDER, WS_POPUP, WS_VISIBLE,
    };
    use windows::core::{PCWSTR, w};

    /// How long the test thread gives the bench thread to win the foreground.
    const OPEN_TIMEOUT: Duration = Duration::from_secs(8);

    /// How long the bench waits for the window to show the text it expects.
    ///
    /// Longer than the three seconds of `tests\switch.rs`: this run presses the hotkey four
    /// times, and each press is a whole replacement — erase and retype — plus the switch of
    /// FR-40 step 5 with the verification budget of `switch::VERIFY_BUDGET_MS` behind it.
    const TEXT_TIMEOUT: Duration = Duration::from_secs(5);

    /// `dwExtraInfo` of every keystroke this bench injects.
    ///
    /// Requirement 2 of §11.5: it must **differ** from `hook::INJECTED_SIGNATURE`, or the program
    /// filters this input out as its own (FR-03) and the scenario tests nothing.
    const BENCH_SIGNATURE: usize = 0x0000_0000_5431_3532;

    /// Published instead of a window handle when the bench thread could not get the foreground.
    const FAILED: usize = usize::MAX;

    /// `ES_MULTILINE | ES_AUTOVSCROLL` — the styles of a plain text box.
    const EDIT_STYLES: u32 = 0x0004 | 0x0040;

    /// How many attempts the window gets at the foreground, at 50 ms each — about five seconds.
    const FOREGROUND_ATTEMPTS: u32 = 100;

    /// A window of this test's own **on a thread of its own, with a real message loop**.
    pub struct Bench {
        handle: HWND,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Bench {
        /// Starts the bench thread and returns once its window is the foreground one.
        pub fn open() -> Option<Self> {
            let stop = Arc::new(AtomicBool::new(false));
            let published = Arc::new(AtomicUsize::new(0));

            let (mine, theirs) = (Arc::clone(&stop), Arc::clone(&published));

            let thread = std::thread::spawn(move || {
                let Some(window) = TestWindow::open_foreground() else {
                    theirs.store(FAILED, Ordering::Release);
                    return;
                };

                theirs.store(window.handle().0 as usize, Ordering::Release);

                // The message loop. Without it a posted `WM_INPUTLANGCHANGEREQUEST` is never
                // processed and method 1 of FR-50 could not work here whatever the product did.
                while !mine.load(Ordering::Acquire) {
                    pump();
                    std::thread::sleep(Duration::from_millis(1));
                }

                // Destroyed on the thread that created it, as `DestroyWindow` requires.
                drop(window);
            });

            let started = Instant::now();

            while started.elapsed() < OPEN_TIMEOUT {
                match published.load(Ordering::Acquire) {
                    0 => std::thread::sleep(Duration::from_millis(20)),
                    FAILED => break,
                    raw => {
                        return Some(Self {
                            handle: HWND(raw as *mut c_void),
                            stop,
                            thread: Some(thread),
                        });
                    }
                }
            }

            stop.store(true, Ordering::Release);
            let _ = thread.join();
            None
        }

        /// The window the bench thread owns.
        pub fn handle(&self) -> HWND {
            self.handle
        }

        /// Polls the window's text until `wanted` accepts it, or until [`TEXT_TIMEOUT`] passes.
        ///
        /// Answers the last text read either way, so a failing assertion can print what really
        /// arrived instead of nothing.
        pub fn wait_for_text(&self, wanted: impl Fn(&str) -> bool) -> Option<String> {
            let started = Instant::now();
            let mut last = None;

            while started.elapsed() < TEXT_TIMEOUT {
                last = read_text(self.handle);

                if last.as_deref().is_some_and(&wanted) {
                    return last;
                }

                std::thread::sleep(Duration::from_millis(10));
            }

            last
        }
    }

    impl Drop for Bench {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);

            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    /// Reads the text of an `EDIT` control through `WM_GETTEXT`.
    fn read_text(window: HWND) -> Option<String> {
        let mut buffer = [0u16; 256];

        // SAFETY: `WM_GETTEXT` takes the buffer length in `wparam` and a pointer to that many
        // UTF-16 units in `lparam`. `buffer` is exactly that long, lives for the whole call and
        // is the only memory the receiver may write. `SendMessageW` is synchronous and the
        // window belongs to this process, so the pointer never crosses a process boundary.
        let written = unsafe {
            SendMessageW(
                window,
                WM_GETTEXT,
                Some(WPARAM(buffer.len())),
                Some(LPARAM(buffer.as_mut_ptr() as isize)),
            )
        };

        let written = usize::try_from(written.0).ok()?.min(buffer.len());

        Some(String::from_utf16_lossy(&buffer[..written]))
    }

    /// Types `text` as real keystrokes — one `SendInput` per character, down and up.
    pub fn type_ascii(text: &str) {
        for character in text.chars() {
            let virtual_key = character.to_ascii_uppercase() as u16;

            send_key(virtual_key, false);
            send_key(virtual_key, true);
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    /// Presses and releases the hotkey of FR-02 — `Pause`, the default of section 7.
    ///
    /// The pause afterwards is not a wait on a clock in place of a condition: the condition is
    /// waited for by [`Bench::wait_for_text`]. It is there because a second press arriving while
    /// the first replacement is still going out would be a different scenario from the one
    /// position 16 describes.
    pub fn press_hotkey() {
        send_key(lang_switcher::hook::DEFAULT_HOTKEY_VK, false);
        std::thread::sleep(Duration::from_millis(8));
        send_key(lang_switcher::hook::DEFAULT_HOTKEY_VK, true);
        std::thread::sleep(Duration::from_millis(200));
    }

    /// Taps `Shift`, which is the product's own layout probe of task T-03-3c.
    pub fn tap_shift() {
        send_key(VK_SHIFT.0, false);
        std::thread::sleep(Duration::from_millis(8));
        send_key(VK_SHIFT.0, true);
        std::thread::sleep(Duration::from_millis(150));
    }

    /// One key event through `SendInput`, with a signature the product will not mistake for its
    /// own.
    fn send_key(virtual_key: u16, up: bool) {
        let flags = if up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };

        // SAFETY: takes two integers, returns one, dereferences nothing.
        let scan = unsafe { MapVirtualKeyW(u32::from(virtual_key), MAPVK_VK_TO_VSC) } as u16;

        let event = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(virtual_key),
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: BENCH_SIGNATURE,
                },
            },
        };

        // SAFETY: one fully initialised `INPUT` in a slice this frame owns, with the size of the
        // structure passed as the OS requires. `SendInput` reads the slice and writes nothing
        // back through it. The return value is examined below (NFR-13).
        let sent = unsafe { SendInput(&[event], core::mem::size_of::<INPUT>() as i32) };

        assert_eq!(sent, 1, "SendInput refused a keystroke of the scenario");
    }

    /// The product, started for the run and stopped when this value goes away.
    ///
    /// ⚠ Decision Р-42: this is the **only** process this test may end, and it ends only this
    /// one, by the handle `Command::spawn` answered with.
    ///
    /// ⭐ **Task Т-25-2, finding м-Э24-2: the stop lives in [`Drop`] and not in [`Product::stop`]
    /// alone.** A panic in the body of a behavioural test walks past every remaining line of it,
    /// `stop()` included, and used to leave the process running for ever — measured at stage Э24
    /// (`pid 7624`, alive 358 seconds under a 45-second deadline). Unwinding does run `Drop`, so
    /// this is where the guarantee belongs; `stop` stays as the explicit door for a test that
    /// ends the product in the middle of its own body.
    pub struct Product {
        child: std::process::Child,
    }

    impl Product {
        /// Starts the product with the deadline of FR-97 and waits for its hook to be up.
        pub fn start() -> Self {
            let child = std::process::Command::new(env!("CARGO_BIN_EXE_LangSwitcher"))
                // FR-97. Never started without it: the program carries a live global hook.
                .env("LANGSW_DEBUG_TIMEOUT_SEC", "45")
                .spawn()
                .expect("the product binary is built beside this test");

            // NFR-08 gives the hook fifty milliseconds from start-up; the layout sweep of FR-20
            // follows it and this is a debug build, so the wait is generous rather than tight.
            std::thread::sleep(Duration::from_millis(2_500));

            Self { child }
        }

        /// The process id, for the record in the report.
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        /// Ends the process this test started, and no other. Idempotent.
        ///
        /// `kill` on a child that has already been reaped answers `Err`, which is why the result
        /// is dropped; `wait` caches the status it read, so a second call costs nothing and
        /// cannot block. Both doors — [`Product::stop`] and [`Drop`] — come here.
        fn end(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }

        /// Ends the product where the scenario says it ends, rather than where the value dies.
        pub fn stop(mut self) {
            self.end();
        }
    }

    impl Drop for Product {
        fn drop(&mut self) {
            self.end();
        }
    }

    /// `STILL_ACTIVE` — what `GetExitCodeProcess` answers for a process that has not exited.
    const STILL_ACTIVE: u32 = 259;

    /// Whether the process with this id is still running — task Т-25-2.
    ///
    /// Asked of the operating system and of nothing else: the handle `Command::spawn` gave the
    /// bench is gone by the time this is called, which is the whole point — the question is
    /// whether the process outlived it. `OpenProcess` refuses an id nothing carries, and a
    /// process that has exited but is still held open by somebody answers `GetExitCodeProcess`
    /// with its code rather than with [`STILL_ACTIVE`]; both are "gone".
    ///
    /// The access asked for is `PROCESS_QUERY_LIMITED_INFORMATION`, which is the least that can
    /// answer this question and grants nothing that could end anything — decision Р-42 is about
    /// what this bench may kill, and this reads.
    ///
    /// ⚠ An id is only unique while its process lives, so a false "alive" would need Windows to
    /// have handed the same number to something else in the microseconds between the kill and
    /// this call. That is the known limit of asking by id, and it is the same limit every tool
    /// that reports by pid works under.
    pub fn process_is_alive(pid: u32) -> bool {
        // SAFETY: takes an access mask, an inheritance flag and an id; it dereferences nothing
        // of ours and returns either a handle this frame owns or an error.
        let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
        else {
            return false;
        };

        let mut code = 0u32;

        // SAFETY: `handle` came from the successful call above and is still open; `code` is a
        // live local this call writes one `u32` into. The result is examined below (NFR-13).
        let read = unsafe { GetExitCodeProcess(handle, &raw mut code) };

        // SAFETY: `handle` came from the successful `OpenProcess` above and is closed exactly
        // once, here, on both paths out of this function.
        let _ = unsafe { CloseHandle(handle) };

        read.is_ok() && code == STILL_ACTIVE
    }

    /// A window created by this test, destroyed when the value is dropped.
    pub struct TestWindow {
        handle: HWND,
    }

    impl TestWindow {
        /// The handle, for callers that need to prove the foreground is still ours.
        pub fn handle(&self) -> HWND {
            self.handle
        }

        /// Creates the window and waits for it to become the foreground one.
        ///
        /// `None` when it never does — decision Р-42: the caller records that and sends nothing,
        /// rather than reaching for a window it did not create.
        pub fn open_foreground() -> Option<Self> {
            // SAFETY: `EDIT` is a system window class present in every process; a null window
            // name asks for empty content, and no parent, menu, instance or creation parameter
            // is passed, which is the documented way to ask for a plain top-level window.
            let handle = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("EDIT"),
                    PCWSTR::null(),
                    WINDOW_STYLE(EDIT_STYLES) | WS_POPUP | WS_VISIBLE | WS_BORDER,
                    200,
                    200,
                    420,
                    120,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("the EDIT class is registered in every process");

            let window = Self { handle };

            // SAFETY: `handle` is the live window this frame owns; both calls take it by value
            // and touch no memory of ours. The `BOOL`s are not fatal — the loop below decides.
            unsafe {
                let _ = ShowWindow(handle, SW_SHOW);
                let _ = SetFocus(Some(handle));
            }

            for _ in 0..FOREGROUND_ATTEMPTS {
                ask_for_foreground(handle);
                pump();

                // SAFETY: takes no arguments, returns a handle by value.
                if unsafe { GetForegroundWindow() } == handle {
                    return Some(window);
                }

                std::thread::sleep(Duration::from_millis(50));
            }

            None
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
            // destroyed exactly once — this type is neither `Copy` nor `Clone`.
            let _ = unsafe { DestroyWindow(self.handle) };
        }
    }

    /// **Point 25 of task T-05-1, and the same duty here.** Puts the layout back, however the
    /// test ends — a panic included.
    pub struct Restore {
        layout: LayoutId,
        window: HWND,
    }

    impl Restore {
        /// Remembers `layout` as the one the machine was on before the test touched it.
        pub fn to(layout: LayoutId, window: HWND) -> Self {
            Self { layout, window }
        }
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            // Decision Р-42 once more, at the one moment it is easiest to forget: restoring is
            // still switching, so it only happens while our own window is the foreground one.
            // SAFETY: takes no arguments, returns a handle by value.
            if unsafe { GetForegroundWindow() } != self.window {
                println!(
                    "our window is no longer in front; the layout was NOT put back by this \
                     test, and no window of anybody else's was touched"
                );
                return;
            }

            let outcome = switch::to(self.layout);
            println!(
                "restoring {} -> {outcome:?}, now {}",
                self.layout,
                switch::current()
            );
        }
    }

    /// Asks the system to put our own window in front, the way `tests\inject.rs` established.
    fn ask_for_foreground(handle: HWND) {
        // SAFETY: takes no arguments and returns a handle by value.
        let foreground = unsafe { GetForegroundWindow() };

        // SAFETY: a null `foreground` is answered with zero, which is examined below; `None`
        // asks for the thread id alone.
        let owner = unsafe { GetWindowThreadProcessId(foreground, None) };
        // SAFETY: takes no arguments, returns this thread's id.
        let ours = unsafe { GetCurrentThreadId() };

        let attached = owner != 0 && owner != ours;

        if attached {
            // SAFETY: both ids name live threads. A failed attach is not fatal — the call below
            // is then simply the unprivileged attempt, and the caller re-reads the fact.
            unsafe {
                let _ = AttachThreadInput(ours, owner, true);
            }
        }

        // SAFETY: `handle` is the live window the caller owns; both calls take it by value.
        unsafe {
            let _ = BringWindowToTop(handle);
            let _ = SetForegroundWindow(handle);
        }

        if attached {
            // SAFETY: undoes exactly the attachment above, with the same two ids.
            unsafe {
                let _ = AttachThreadInput(ours, owner, false);
            }
        }
    }

    /// Runs this thread's message queue dry.
    fn pump() {
        let mut message = MSG::default();

        loop {
            // SAFETY: `message` is a live, aligned `MSG` owned by this frame for the whole call
            // and is the only buffer written to. `None` asks for every message of this thread.
            let taken = unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) };

            if !taken.as_bool() {
                return;
            }

            // SAFETY: `message` was just filled by `PeekMessageW` and is passed on unchanged.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

// -------------------------------------------------------------------------------------
// Э20 — какой шаг цикла стоит на экране. Находка №5 аудита 2026-08-31.
// -------------------------------------------------------------------------------------

/// **`inject::showing` читает счётчик на месте, а не на шаг вперёд.**
///
/// От второго нажатия на экране стоит **предыдущая** инжекция, и стирать надо её длину.
/// Позиция `p` — это шаг, который уже отработал; `p + 1` — цель нажатия, которое только
/// готовится, и её текста на экране ещё нет. Одна единица, никем больше не сторожимая:
/// на паре RU/EN она невидима (длины совпадают), а на любой раскладке с мёртвыми клавишами
/// даёт неверное число на каждом шаге цикла — Э20, инвариант И-2.
///
/// Цикл здесь целиком синтетический: три раскладки берутся из конфигурации, у машины
/// не спрашивается ничего, поэтому проверка живёт в батарее и там, где стоят две раскладки.
#[test]
fn the_screen_shows_the_step_already_taken_and_not_the_one_being_prepared() {
    let available = [EN, RU, EL];
    let cycle = cycle_for(
        configured(
            LayoutMode::Cycle,
            ["0x00000409", "0x00000419"],
            &["0x00000409", "0x00000419", "0x00000408"],
        ),
        &available,
        Session::Whole,
    )
    .expect("три раскладки — годный цикл");

    // Набрано под EN; порядок круга — EN → RU → EL → EN.
    assert_eq!(cycle.target(EN, 1).expect("шаг 1"), RU);
    assert_eq!(cycle.target(EN, 2).expect("шаг 2"), EL);
    assert_eq!(cycle.target(EN, 3).expect("шаг 3"), EN);

    // Ни одной замены ещё не было: на экране то, что набрал человек, а не чья-то инжекция.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 0, false),
        None,
        "позиция 0 — на экране набор, счёт по typed_chars"
    );

    // Счётчик на единице — первое нажатие отработало и положило на экран русский текст.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 1, true),
        Some(RU),
        "на экране инжекция шага 1, а не цель шага 2"
    );

    // И на двойке — греческий текст второго нажатия, а не то, куда идёт третье.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 2, true),
        Some(EL),
        "на экране инжекция шага 2, а не цель шага 3"
    );
}

/// **Круг замкнулся — счётчик снова ноль, но на экране НЕ то, что набрал человек.**
///
/// Дефект, найденный живым опытом пользователя 2026-08-31 уже ПОСЛЕ первого ремонта Э20:
/// «позиция 0» значит «цель круга снова исходная», а вовсе не «замен ещё не было». После
/// полного круга на экране стоит откат — те же штрихи, отрисованные в исходную раскладку, —
/// и при мёртвых клавишах он **другой длины**, чем набор: `café` набрано четырьмя символами,
/// а откат `caf'e` — пять, потому что `KEYEVENTF_UNICODE` ничего не компонует.
///
/// Пользователь снял это пятью нажатиями: на третьем и пятом (там, где счётчик заворачивался)
/// продукт стирал на один символ меньше, и перед словом копилась буква — «12345 cсфа'у»,
/// затем «12345 ccсфа'у».
///
/// Различает эти два случая **не** счётчик, а признак открытой сессии конвертации
/// `Recorder::in_conversion` — он и есть «этот набор уже заменяли».
#[test]
fn a_closed_circle_still_shows_an_injection_and_not_the_typing() {
    let available = [EN, RU];
    let cycle = cycle_for(
        configured(
            LayoutMode::Cycle,
            ["0x00000409", "0x00000419"],
            &["0x00000409", "0x00000419"],
        ),
        &available,
        Session::Whole,
    )
    .expect("две раскладки — годный цикл");

    // Замен ещё не было: на экране набор человека, счёт по typed_chars.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 0, false),
        None,
        "первое нажатие: экран держит набор"
    );

    // Одна замена позади — на экране русская инжекция.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 1, true),
        Some(RU),
        "второе нажатие: экран держит инжекцию шага 1"
    );

    // ⭐ Круг замкнулся: счётчик снова ноль, но замена БЫЛА, и на экране стоит откат —
    // те же штрихи, отрисованные в исходную раскладку. Это не набор человека.
    assert_eq!(
        lang_switcher::inject::showing(&cycle, EN, 0, true),
        Some(EN),
        "третье нажатие: счётчик ноль, но экран держит откат в исходную раскладку"
    );
}
