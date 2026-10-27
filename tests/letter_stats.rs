//! The statistics of letters — решение 196.1 (2), stage Э105: the model the selection path reads an
//! ambiguous division by, on a synthetic table here and on the table of English the image carries.

use lang_switcher::layouts::LayoutId;
use lang_switcher::letter_stats::{self, TABLE_LEN, Trigrams};

/// The index of the trigram `a b c` of symbols (0 the boundary, 1…26 the letters).
fn at(a: usize, b: usize, c: usize) -> usize {
    (a * 27 + b) * 27 + c
}

/// The symbol of a Latin letter.
fn s(letter: char) -> usize {
    usize::from(letter as u8 - b'a') + 1
}

/// The mean cost of `text` under `table`.
fn cost(table: &Trigrams, text: &str) -> f32 {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut reading = table.reading();
    reading.feed(&units);
    reading.cost()
}

/// A table of 40 everywhere but on the trigrams of `cheap`, which cost 1.
fn table_with_cheap(cheap: &[(usize, usize, usize)]) -> Trigrams {
    let mut costs = vec![40u8; TABLE_LEN];
    for &(a, b, c) in cheap {
        costs[at(a, b, c)] = 1;
    }
    Trigrams::from_costs(Box::leak(costs.into_boxed_slice())).expect("a table of the right length")
}

/// **A table is exactly 27³ bytes**, and nothing else is one.
#[test]
fn a_table_is_twenty_seven_cubed_bytes() {
    assert_eq!(TABLE_LEN, 19_683);
    assert!(Trigrams::from_costs(&[]).is_none());
    assert!(Trigrams::from_costs(Box::leak(vec![0u8; TABLE_LEN - 1].into_boxed_slice())).is_none());
}

/// **The reading is the mean cost per symbol, the boundaries of the word included** — the
/// probable text is the cheaper one, whatever its length: `be` (three trigrams with its end) is
/// cheap where `ghe` is not, on a table that says so.
#[test]
fn the_cheaper_reading_is_the_one_the_table_makes_probable() {
    let table = table_with_cheap(&[(0, 0, s('b')), (0, s('b'), s('e')), (s('b'), s('e'), 0)]);

    assert!(cost(&table, "be") < cost(&table, "ghe"));
    assert!(
        (cost(&table, "be") - 1.0).abs() < f32::EPSILON,
        "three cheap trigrams"
    );
    assert!(cost(&table, "BE") < cost(&table, "ghe"), "case is folded");
    assert_eq!(cost(&table, ""), 40.0, "nothing but the end of a word");
}

/// **Only English carries a table in stage Э105** (ТЗ §0 п. 10) — a layout of another language
/// gets none, and the selection path then divides by single keys (196.1 (3)).
#[test]
fn only_english_has_a_table() {
    assert!(letter_stats::for_layout(LayoutId::from_raw(0x0419_0419)).is_none());
    assert!(letter_stats::for_layout(LayoutId::from_raw(0x0401_0401)).is_none());
    assert!(letter_stats::for_layout(LayoutId::from_raw(0x0409_0409)).is_some());
    assert!(
        letter_stats::for_layout(LayoutId::from_raw(0x0809_0809)).is_some(),
        "every English layout: the primary language decides"
    );
}

/// **Посылка П6 — the table of English reads the two words of decision 196.1 the way English is
/// written:** `لاث` divided as `B`, `E` is `be` and as `G`, `H`, `E` is `ghe`; `مهلاف` divided as `L`,
/// `I`, `G`, `H`, `T` is `light` and as `L`, `I`, `B`, `T` is `libt`. The table is the image's own,
/// made from the five books of the report.
#[test]
fn the_table_of_english_prefers_be_and_light() {
    let english = letter_stats::for_layout(LayoutId::from_raw(0x0409_0409)).expect("the table");

    let (be, ghe) = (cost(&english, "be"), cost(&english, "ghe"));
    let (light, libt) = (cost(&english, "light"), cost(&english, "libt"));
    println!("be {be:.2} ghe {ghe:.2} light {light:.2} libt {libt:.2}");

    assert!(
        be < ghe,
        "be ({be}) is more probable English than ghe ({ghe})"
    );
    assert!(
        light < libt,
        "light ({light}) is more probable English than libt ({libt})"
    );
}
