//! Integration tests for module `layouts`, task T-02-1.
//!
//! The expected values are not guessed. The layout list and the two anchor characters come
//! from section 4 of TOOLCHAIN.md, which records what probe H-9 actually observed on this
//! machine: exactly two layouts, `0x04190419` Russian and `0x04090409` US, with the `A` key
//! producing `ф` under the first and `a` under the second.
//!
//! The tests read the system layout list and decode characters from it; they change
//! nothing. Loading, activating or unloading a layout is forbidden to this task, and the
//! last test here checks that the list came out of the run as it went in.
//!
//! Printing decoded characters is allowed in this file and only in this file: there is no
//! user input here, only a sweep of the keyboard. SEC-01 and SEC-07 govern the product, and
//! the product never prints them.

use lang_switcher::layouts::{
    KeyMapping, LayoutCache, LayoutError, LayoutId, LayoutMap, LayoutMapBuilder, MappingKind, Mods,
    REBUILD_MESSAGES, enumerate, enumerate_all, needs_rebuild,
};

/// Russian, slot 1 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const RUSSIAN: LayoutId = LayoutId::from_raw(0x0419_0419);

/// US, slot 2 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const US: LayoutId = LayoutId::from_raw(0x0409_0409);

/// Scan code of the `A` key on a set 1 keyboard. Used as a cross-check only: every test
/// below finds the key through the cache rather than through this constant.
const SCAN_A: u16 = 0x1E;

/// Builds the cache, failing the test with the reason if it cannot be built.
fn cache() -> LayoutCache {
    LayoutCache::build().expect("the mapping cache must build on this machine")
}

/// The map of `layout`, or a failure naming the layout that is missing.
fn map_of(cache: &LayoutCache, layout: LayoutId) -> &LayoutMap {
    cache
        .get(layout)
        .unwrap_or_else(|| panic!("layout {layout} is not in the cache"))
}

/// The scan code of the physical key that produces `a` in the US layout — the `A` key,
/// identified by what it does rather than by a hardwired number.
fn scan_of_a(cache: &LayoutCache) -> u16 {
    let press = map_of(cache, US)
        .find_key('a')
        .expect("the US layout must have a key that produces 'a'");
    assert_eq!(
        press.mods,
        Mods::NONE,
        "'a' must be the unmodified reading of its key"
    );
    assert_eq!(
        press.scan, SCAN_A,
        "the A key is scan code 0x1E on a set 1 keyboard"
    );
    press.scan
}

/// The single character a key produces, or a failure naming the coordinates.
fn char_at(map: &LayoutMap, scan: u16, mods: Mods) -> char {
    let mapping = map.lookup(scan, mods);
    mapping.single_char().unwrap_or_else(|| {
        panic!(
            "layout {} scan 0x{scan:02X} mods 0b{:03b} produced {:?}, not a single character",
            map.layout(),
            mods.bits(),
            mapping.kind()
        )
    })
}

// -- criterion 9 -------------------------------------------------------------------------

#[test]
fn enumeration_reports_exactly_two_layouts() {
    let layouts = enumerate().expect("the system layout list must be readable");
    assert_eq!(
        layouts.len(),
        2,
        "TOOLCHAIN.md section 4: this machine has exactly two layouts, got {layouts:?}"
    );
}

// -- criterion 10 ------------------------------------------------------------------------

#[test]
fn enumeration_contains_the_russian_and_us_layouts() {
    let layouts = enumerate().expect("the system layout list must be readable");
    assert!(
        layouts.contains(&RUSSIAN),
        "expected {RUSSIAN} among {layouts:?}"
    );
    assert!(layouts.contains(&US), "expected {US} among {layouts:?}");
}

// -- criterion 11 ------------------------------------------------------------------------

#[test]
fn the_a_key_unmodified_produces_cyrillic_ef_in_russian() {
    let cache = cache();
    let scan = scan_of_a(&cache);
    assert_eq!(char_at(map_of(&cache, RUSSIAN), scan, Mods::NONE), 'ф');
}

// -- criterion 12 ------------------------------------------------------------------------

#[test]
fn the_a_key_unmodified_produces_latin_a_in_us() {
    let cache = cache();
    let scan = scan_of_a(&cache);
    assert_eq!(char_at(map_of(&cache, US), scan, Mods::NONE), 'a');
}

// -- criterion 13 ------------------------------------------------------------------------

#[test]
fn shift_on_the_a_key_produces_capitals_in_both_layouts() {
    let cache = cache();
    let scan = scan_of_a(&cache);
    assert_eq!(char_at(map_of(&cache, US), scan, Mods::SHIFT), 'A');
    assert_eq!(char_at(map_of(&cache, RUSSIAN), scan, Mods::SHIFT), 'Ф');
}

// -- criterion 14 ------------------------------------------------------------------------

#[test]
fn one_physical_key_gives_at_in_us_and_a_quote_in_russian() {
    // Section 11.1 of SPEC names this pair as the proof that conversion follows the
    // physical key and not a character table: the same scan code carries two characters
    // that have nothing to do with each other.
    let cache = cache();
    let digit_two = map_of(&cache, US)
        .find_key('2')
        .expect("the US layout must have a key that produces '2'");
    assert_eq!(digit_two.mods, Mods::NONE);

    assert_eq!(
        char_at(map_of(&cache, US), digit_two.scan, Mods::SHIFT),
        '@'
    );
    assert_eq!(
        char_at(map_of(&cache, RUSSIAN), digit_two.scan, Mods::SHIFT),
        '"'
    );
}

// -- criterion 15 ------------------------------------------------------------------------

#[test]
fn the_cache_covers_both_layouts_and_neither_map_is_empty() {
    let cache = cache();
    assert_eq!(cache.len(), 2, "expected one map per layout");
    for layout in [RUSSIAN, US] {
        let map = map_of(&cache, layout);
        assert!(!map.is_empty(), "layout {layout} produced no characters");
        // A latin or cyrillic alphabet plus the digit row and punctuation, times the
        // modifier combinations: far more than a hundred filled entries.
        assert!(
            map.len() > 100,
            "layout {layout} produced only {} mappings",
            map.len()
        );
    }
}

// -- criterion 16 ------------------------------------------------------------------------

#[test]
fn the_reverse_index_finds_the_key_that_produces_the_character() {
    let cache = cache();
    let russian = map_of(&cache, RUSSIAN);

    let press = russian
        .find_key('ф')
        .expect("the Russian layout must have a key that produces 'ф'");
    assert_eq!(
        press.scan,
        scan_of_a(&cache),
        "'ф' must come from the same physical key that carries 'a' in US"
    );
    assert_eq!(
        char_at(russian, press.scan, press.mods),
        'ф',
        "the two directions must agree"
    );

    // The whole alphabet, both cases, both layouts: every character the forward table
    // reports must be findable backwards, and must lead back to the same character.
    for (map, alphabet) in [
        (russian, "фисвуапршолдьтщзйкыегмцчня"),
        (map_of(&cache, US), "abcdefghijklmnopqrstuvwxyz"),
    ] {
        for ch in alphabet.chars() {
            let press = map
                .find_key(ch)
                .unwrap_or_else(|| panic!("{ch:?} not found in layout {}", map.layout()));
            assert_eq!(char_at(map, press.scan, press.mods), ch);
        }
    }

    // A character that belongs to neither layout is simply absent, not a wrong answer.
    assert_eq!(russian.find_key('\u{05D0}'), None);
}

// -- criterion 17 ------------------------------------------------------------------------

#[test]
fn rebuilding_yields_a_cache_equal_to_the_original() {
    let mut cache = cache();
    let original = cache.clone();

    cache.rebuild().expect("a rebuild must succeed");
    assert_eq!(cache, original, "FR-21: a rebuild must be idempotent");

    // Twice more, because FR-21 fires on every WM_INPUTLANGCHANGE and there can be many.
    cache.rebuild().expect("a rebuild must succeed");
    cache.rebuild().expect("a rebuild must succeed");
    assert_eq!(cache, original);
}

// -- criterion 18 ------------------------------------------------------------------------

#[test]
fn the_ime_filter_rejects_ime_handles_and_keeps_ordinary_layouts() {
    // Synthetic handles: the filter is a property of the HKL value, so it is tested without
    // asking the system for anything. This machine has no IME installed (TOOLCHAIN.md
    // section 4), which is exactly why the positive case has to be built by hand.
    let ime = [
        (0xE020_0804usize, "Chinese Simplified, Microsoft Pinyin"),
        (0xE001_0411, "Japanese, Microsoft IME"),
        (0xE012_0412, "Korean, Microsoft IME"),
        (0xE000_0409, "a text service claiming the US language"),
    ];
    for (raw, what) in ime {
        assert!(
            LayoutId::from_raw(raw).is_ime(),
            "0x{raw:08X} ({what}) must be excluded by FR-35"
        );
    }

    let ordinary = [
        (0x0409_0409usize, "US"),
        (0x0419_0419, "Russian"),
        (0x0407_0407, "German"),
        (0xF001_0409, "US Dvorak, an explicit layout identifier"),
        (
            0xF002_0409,
            "US International, an explicit layout identifier",
        ),
        (0x0804_0804, "Chinese Simplified, a plain keyboard layout"),
    ];
    for (raw, what) in ordinary {
        assert!(
            !LayoutId::from_raw(raw).is_ime(),
            "0x{raw:08X} ({what}) must not be excluded by FR-35"
        );
    }

    // And the filter is actually applied to the system list.
    let participating = enumerate().expect("the system layout list must be readable");
    assert!(participating.iter().all(|layout| !layout.is_ime()));
}

// -- criterion 19 ------------------------------------------------------------------------

#[test]
fn a_failed_build_is_distinguishable_from_an_empty_cache() {
    // No layouts survived the FR-35 filter: an error, not a cache with nothing in it.
    assert_eq!(
        LayoutCache::from_maps(Vec::new()).unwrap_err(),
        LayoutError::NoUsableLayouts
    );

    // Layouts were there but not one character came out of them: also an error. This is the
    // shape that would otherwise be indistinguishable from "these keys carry no
    // characters", and the one point 4 of the task is about.
    let silent = LayoutMapBuilder::new(RUSSIAN).finish();
    assert!(silent.is_empty());
    assert_eq!(
        LayoutCache::from_maps(vec![silent]).unwrap_err(),
        LayoutError::Empty
    );

    // A cache value therefore always carries at least one layout with at least one
    // character, and a caller that got one never has to guess.
    let built = LayoutCache::build().expect("the cache must build on this machine");
    assert!(!built.is_empty());
    assert!(built.maps().iter().any(|map| !map.is_empty()));
}

// -- beyond the required set --------------------------------------------------------------

#[test]
fn the_rebuild_trigger_is_the_pair_named_by_fr_21() {
    // WM_INPUTLANGCHANGE and WM_DEVICECHANGE. Stated here so that the window procedure of
    // task T-01-2 has something to fail against if the pair is ever narrowed.
    assert_eq!(REBUILD_MESSAGES, [0x0051, 0x0219]);
    assert!(needs_rebuild(0x0051));
    assert!(needs_rebuild(0x0219));
    assert!(!needs_rebuild(0x0100)); // WM_KEYDOWN
}

#[test]
fn dead_keys_and_ligatures_are_stored_whole() {
    // Neither RU nor EN has a dead key or a ligature, so the storage rules of FR-24 and of
    // section 11.1 of SPEC are checked on a map built by hand.
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0xF001_0409));
    let dead = KeyMapping::dead('\u{0300}');
    let ligature = KeyMapping::from_to_unicode(2, &[0x0066, 0x0069]);
    let outside_bmp = KeyMapping::from_char('\u{1F600}');
    builder.set(0x10, Mods::NONE, dead);
    builder.set(0x11, Mods::NONE, ligature);
    builder.set(0x12, Mods::NONE, outside_bmp);
    let map = builder.finish();

    let stored_dead = map.lookup(0x10, Mods::NONE);
    assert_eq!(stored_dead.kind(), MappingKind::Dead);
    assert_eq!(stored_dead.units(), [0x0300]);
    assert!(stored_dead.is_dead());

    let stored_ligature = map.lookup(0x11, Mods::NONE);
    assert_eq!(stored_ligature.kind(), MappingKind::Ligature);
    assert_eq!(
        stored_ligature.units(),
        [0x0066, 0x0069],
        "both units of the ligature must survive, not just the first"
    );
    assert_eq!(stored_ligature.single_char(), None);

    // A dead key is not reachable backwards: replaying it would start a composition.
    assert_eq!(map.find_key('\u{0300}'), None);
    // A surrogate pair is one character and is reachable backwards.
    assert_eq!(map.lookup(0x12, Mods::NONE).units().len(), 2);
    assert_eq!(
        map.find_key('\u{1F600}').map(|press| press.scan),
        Some(0x12)
    );

    // A result too long for a stroke of FR-04 is recorded as nothing rather than as a
    // truncated prefix that would silently differ from what the key produces.
    let too_long = KeyMapping::from_to_unicode(5, &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(too_long.kind(), MappingKind::None);
    assert!(too_long.is_empty());
}

#[test]
fn a_key_with_no_character_reads_as_empty_rather_than_as_a_failure() {
    let cache = cache();
    let us = map_of(&cache, US);
    // Scan code 0 is not a key: nothing may be filed under it.
    assert!(us.lookup(0x00, Mods::NONE).is_empty());
    assert_eq!(us.lookup(0x00, Mods::NONE).kind(), MappingKind::None);
    // FR-23 rests on this: an absent mapping is an ordinary answer, and the caller carries
    // the character over unchanged instead of aborting the conversion.
    assert!(us.lookup(0xFF, Mods::ALTGR).is_empty());
}

#[test]
fn building_the_cache_leaves_the_system_layout_list_untouched() {
    // The task forbids changing the set of installed layouts. ToUnicodeEx with flag 0x4 and
    // MapVirtualKeyExW are read-only calls, and this is the check that they stayed that
    // way.
    let before = enumerate_all().expect("the system layout list must be readable");
    let mut cache = cache();
    cache.rebuild().expect("a rebuild must succeed");
    let after = enumerate_all().expect("the system layout list must be readable");
    assert_eq!(before, after, "the layout list must survive the sweep");
}
