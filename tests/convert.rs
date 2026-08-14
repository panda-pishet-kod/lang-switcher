//! Integration tests for module `convert`, task T-02-2.
//!
//! The unit tests of section 11.1 live in `src\convert.rs` and run on synthetic layouts, so
//! they depend on neither Win32 nor the layouts installed on the machine. These are the other
//! half: the same conversion driven by the **live** mapping cache of module `layouts`, on the
//! RU/EN pair section 4 of TOOLCHAIN.md records for this machine — `0x04190419` Russian and
//! `0x04090409` US.
//!
//! Two things are checked here that a synthetic layout cannot check: that the full RU↔EN
//! traversal of section 11.1 gives the expected characters when the layouts come from the OS
//! rather than from a table written by hand, and that the hardwired fallback of FR-25 answers
//! exactly as the live cache does — which is what makes it a fallback rather than a second,
//! quietly different program.
//!
//! The tests read the system layout list and the layouts themselves; they change nothing, and
//! the last test checks that the list came out of the run as it went in.
//!
//! Printing decoded characters is allowed in a test file and only there: there is no user
//! input here, only string constants. SEC-01 and SEC-07 govern the product, and the product
//! never prints them.

use lang_switcher::convert::{Keystroke, convert_stroke, convert_strokes, fallback_map, max_units};
use lang_switcher::layouts::{LayoutCache, LayoutId, LayoutMap, Mods, enumerate_all};

/// Russian, slot 1 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const RUSSIAN: LayoutId = LayoutId::from_raw(0x0419_0419);

/// US, slot 2 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const US: LayoutId = LayoutId::from_raw(0x0409_0409);

/// `Shift` and `CapsLock` together.
const SHIFT_CAPS: Mods = Mods::new(true, true, false);

/// The four modifier combinations the RU and EN layouts put characters on.
///
/// `AltGr` is left out on purpose: neither layout carries characters on it, and with `Ctrl`
/// held `ToUnicodeEx` reports the control characters of the C0 block, which are keystrokes to
/// be passed through rather than text to be converted.
const TEXT_MODS: [Mods; 4] = [Mods::NONE, Mods::SHIFT, Mods::CAPS, SHIFT_CAPS];

/// Scan code shared by the main block `/?` key and the keypad `/` key.
///
/// The two are told apart by the extended flag of FR-05, which is the ninth bit of the cache
/// key since task T-02-1a. Before that task the keypad won the slot in both layouts, EN `/`
/// converted to RU `/` instead of RU `.`, and the traversal constants below had to leave the
/// key out; they carry it again.
const SHARED_WITH_KEYPAD: u16 = 0x35;

/// The extended flag of FR-05 for a key of the main block, and for one of the keypad.
const MAIN_BLOCK: bool = false;
/// See [`MAIN_BLOCK`].
const KEYPAD: bool = true;

/// The 47 keys of the main block, unshifted in the English layout.
///
/// Aligned character by character with [`RU_LOWER`]: the *n*-th character is what the *n*-th
/// of the other becomes when the same physical key is read in the other layout.
const EN_LOWER: &str = "`1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./";
/// The same keys in the Russian layout, unshifted.
const RU_LOWER: &str = "ё1234567890-=йцукенгшщзхъ\\фывапролджэячсмитьбю.";
/// The same keys in the English layout, with `Shift`.
const EN_UPPER: &str = "~!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>?";
/// The same keys in the Russian layout, with `Shift`.
const RU_UPPER: &str = "Ё!\"№;%:?*()_+ЙЦУКЕНГШЩЗХЪ/ФЫВАПРОЛДЖЭЯЧСМИТЬБЮ,";

/// The live mapping cache, or a failure naming the reason.
fn cache() -> LayoutCache {
    LayoutCache::build().expect("the mapping cache must build on this machine")
}

/// The map of `layout`, or a failure naming the layout that is missing.
fn map_of(cache: &LayoutCache, layout: LayoutId) -> &LayoutMap {
    cache
        .get(layout)
        .unwrap_or_else(|| panic!("layout {layout} is not in the cache"))
}

/// The strokes the hook would have recorded had the user typed `text` under `map`.
///
/// The typing, not the conversion. In the product the scan codes arrive from the hook (task
/// T-03-2); a test has no keyboard, so the key carrying each character is found through the
/// reverse index of module `layouts`. Nothing under test calls `find_key`.
fn type_text(map: &LayoutMap, text: &str) -> Vec<Keystroke> {
    text.chars()
        .map(|ch| {
            let press = map
                .find_key(ch)
                .unwrap_or_else(|| panic!("layout {} has no key for {ch:?}", map.layout()));
            Keystroke::recorded_in(map, press.scan, press.extended, press.mods)
        })
        .collect()
}

/// Converts `strokes` into `target` and decodes the result for comparison.
fn render(strokes: &[Keystroke], target: &LayoutMap) -> String {
    let mut out = [0u16; max_units(64)];
    let written =
        convert_strokes(strokes, target, &mut out).expect("the result must fit the buffer");
    String::from_utf16(&out[..written]).expect("the result must be valid UTF-16")
}

// -- section 11.1, position 1, on the live cache ------------------------------------------

#[test]
fn a_full_ru_en_traversal_on_the_live_cache_converts_every_key_in_both_cases() {
    let cache = cache();
    let (us, ru) = (map_of(&cache, US), map_of(&cache, RUSSIAN));

    // Every letter of both alphabets in both cases, the digit row with and without `Shift`,
    // the punctuation keys and `ё`/`~` — both directions, on layouts decoded from the OS.
    assert_eq!(render(&type_text(us, EN_LOWER), ru), RU_LOWER);
    assert_eq!(render(&type_text(ru, RU_LOWER), us), EN_LOWER);
    assert_eq!(render(&type_text(us, EN_UPPER), ru), RU_UPPER);
    assert_eq!(render(&type_text(ru, RU_UPPER), us), EN_UPPER);

    // The constants are aligned key by key, so the assertions above are the claim they look
    // like and not an accident of two strings of unequal length. All 47 keys of the main
    // block, the `/?` key included since task T-02-1a.
    assert_eq!(EN_LOWER.chars().count(), 47);
    assert_eq!(EN_LOWER.chars().count(), RU_LOWER.chars().count());
    assert_eq!(EN_UPPER.chars().count(), RU_UPPER.chars().count());
    assert_eq!(EN_LOWER.chars().count(), EN_UPPER.chars().count());
}

// -- section 11.1, position 2, on the live cache ------------------------------------------

#[test]
fn the_same_physical_key_gives_at_in_us_and_a_quote_in_russian_on_the_live_cache() {
    let cache = cache();
    let (us, ru) = (map_of(&cache, US), map_of(&cache, RUSSIAN));
    let digit_two = us.find_key('2').expect("the US layout must carry '2'");
    assert_eq!(digit_two.mods, Mods::NONE);

    let typed_in_us = Keystroke::recorded_in(us, digit_two.scan, MAIN_BLOCK, Mods::SHIFT);
    assert_eq!(typed_in_us.produced().single_char(), Some('@'));
    assert_eq!(convert_stroke(typed_in_us, ru).single_char(), Some('"'));

    let typed_in_russian = Keystroke::recorded_in(ru, digit_two.scan, MAIN_BLOCK, Mods::SHIFT);
    assert_eq!(typed_in_russian.produced().single_char(), Some('"'));
    assert_eq!(
        convert_stroke(typed_in_russian, us).single_char(),
        Some('@')
    );
}

// -- position 7 of the task, section 11.3 of SPEC -----------------------------------------

#[test]
fn ghbdtn_becomes_privet_on_the_live_cache() {
    let cache = cache();
    let strokes = type_text(map_of(&cache, US), "ghbdtn");
    assert_eq!(render(&strokes, map_of(&cache, RUSSIAN)), "привет");
}

// -- position 8 of the task ----------------------------------------------------------------

#[test]
fn privet_becomes_ghbdtn_on_the_live_cache() {
    let cache = cache();
    let strokes = type_text(map_of(&cache, RUSSIAN), "привет");
    assert_eq!(render(&strokes, map_of(&cache, US)), "ghbdtn");
}

// -- position 9 of the task, FR-25 ---------------------------------------------------------

#[test]
fn the_fallback_table_converts_exactly_as_the_live_cache_does() {
    let cache = cache();
    let live_us = map_of(&cache, US);
    let live_ru = map_of(&cache, RUSSIAN);
    let table_us = fallback_map(US).expect("the fallback table must carry US");
    let table_ru = fallback_map(RUSSIAN).expect("the fallback table must carry Russian");

    let mut compared = 0usize;
    for scan in 0..=u16::from(u8::MAX) {
        for mods in TEXT_MODS {
            // Only where the hardwired table claims to know a key: the live cache also
            // describes the keypad, the function keys and the control keys, and FR-25 never
            // promised those. Every row of the table is a main block key.
            if table_us.lookup(scan, MAIN_BLOCK, mods).is_empty() {
                continue;
            }

            // EN to RU and RU to EN, once through the table and once through the OS.
            assert_eq!(
                convert_stroke(
                    Keystroke::recorded_in(&table_us, scan, MAIN_BLOCK, mods),
                    &table_ru
                ),
                convert_stroke(
                    Keystroke::recorded_in(live_us, scan, MAIN_BLOCK, mods),
                    live_ru
                ),
                "EN to RU differs at scan 0x{scan:02X} mods 0b{:03b}",
                mods.bits()
            );
            assert_eq!(
                convert_stroke(
                    Keystroke::recorded_in(&table_ru, scan, MAIN_BLOCK, mods),
                    &table_us
                ),
                convert_stroke(
                    Keystroke::recorded_in(live_ru, scan, MAIN_BLOCK, mods),
                    live_us
                ),
                "RU to EN differs at scan 0x{scan:02X} mods 0b{:03b}",
                mods.bits()
            );
            compared += 1;
        }
    }

    // All 47 keys of the table, four modifier combinations each. Stated as a number so that a
    // table that quietly lost half its rows cannot pass this test by comparing nothing. It
    // was 46 before task T-02-1a: the key of `SHARED_WITH_KEYPAD` had to be skipped because
    // the live cache answered it with the keypad.
    assert_eq!(compared, 47 * 4);

    // And the letters and digits section 11.1 names, end to end through both.
    assert_eq!(
        render(&type_text(&table_us, "ghbdtn123"), &table_ru),
        render(&type_text(live_us, "ghbdtn123"), live_ru)
    );
}

// -- the defect of task T-02-1a, on the live cache -----------------------------------------

/// The `/?` key of the main block reads `/` in US and `.` in Russian, so EN `/` has to
/// convert to RU `.`. Before task T-02-1a it came out as `/`: the keypad `/` shares the scan
/// code `0x35`, the cache was keyed by the scan code alone, and `VK_DIVIDE` reached the slot
/// first. It is a punctuation mark, and the traversal of section 11.1 of SPEC covers those.
///
/// The body of this test is what failed before the fix, character for character; only the
/// shared `type_text` helper above learned to pass the extended flag on.
#[test]
fn the_main_block_slash_key_converts_to_a_russian_full_stop() {
    let cache = cache();
    let (us, ru) = (map_of(&cache, US), map_of(&cache, RUSSIAN));
    assert_eq!(render(&type_text(us, "/"), ru), ".");
}

/// The other direction. Before the fix the Russian map had no key that produced `.` at all,
/// because its only one was the slot the keypad had taken.
#[test]
fn the_russian_full_stop_converts_back_to_an_english_slash() {
    let cache = cache();
    let (us, ru) = (map_of(&cache, US), map_of(&cache, RUSSIAN));
    assert_eq!(render(&type_text(ru, "."), us), "/");
}

/// And the keypad key itself is still there, in its own slot, reading `/` in both layouts —
/// the repair separated the two keys rather than replacing one with the other.
#[test]
fn the_keypad_slash_key_keeps_its_own_slot_in_both_layouts() {
    let cache = cache();
    let (us, ru) = (map_of(&cache, US), map_of(&cache, RUSSIAN));

    for (map, other) in [(us, ru), (ru, us)] {
        let keypad = Keystroke::recorded_in(map, SHARED_WITH_KEYPAD, KEYPAD, Mods::NONE);
        assert_eq!(keypad.produced().single_char(), Some('/'));
        assert_eq!(convert_stroke(keypad, other).single_char(), Some('/'));
    }

    // The same scan code, the other key: two different characters out of one scan code is
    // the whole point of the ninth bit.
    let main_block = Keystroke::recorded_in(us, SHARED_WITH_KEYPAD, MAIN_BLOCK, Mods::NONE);
    let keypad = Keystroke::recorded_in(us, SHARED_WITH_KEYPAD, KEYPAD, Mods::NONE);
    assert_eq!(main_block.produced(), keypad.produced());
    assert_eq!(convert_stroke(main_block, ru).single_char(), Some('.'));
    assert_eq!(convert_stroke(keypad, ru).single_char(), Some('/'));
}

// -- hygiene -------------------------------------------------------------------------------

#[test]
fn converting_leaves_the_system_layout_list_untouched() {
    // This task may not change the set of installed layouts. Conversion never calls Win32 at
    // all, and building the cache only reads; this is the check that both stayed that way.
    let before = enumerate_all().expect("the system layout list must be readable");
    let cache = cache();
    let _ = render(
        &type_text(map_of(&cache, US), "ghbdtn"),
        map_of(&cache, RUSSIAN),
    );
    let after = enumerate_all().expect("the system layout list must be readable");
    assert_eq!(before, after, "the layout list must survive a conversion");
}
