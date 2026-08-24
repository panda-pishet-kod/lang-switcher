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

use std::sync::{Mutex, MutexGuard, PoisonError};

use lang_switcher::layouts::{
    Configured, Cycle, KeyMapping, KeyPress, LayoutCache, LayoutError, LayoutId, LayoutMap,
    LayoutMapBuilder, LayoutSpec, MAX_CYCLE, MappingKind, Mods, REBUILD_MESSAGES, SelectionError,
    cycle_for, enumerate, enumerate_all, needs_rebuild, published, selection_failures,
};
use lang_switcher::settings::{LayoutMode, Layouts};

/// Russian, slot 1 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const RUSSIAN: LayoutId = LayoutId::from_raw(0x0419_0419);

/// US, slot 2 of `HKCU\Keyboard Layout\Preload` — TOOLCHAIN.md section 4.
const US: LayoutId = LayoutId::from_raw(0x0409_0409);

/// Scan code of the `A` key on a set 1 keyboard. Used as a cross-check only: every test
/// below finds the key through the cache rather than through this constant.
const SCAN_A: u16 = 0x1E;

/// Scan code shared by the `/?` key of the main block and the `/` key of the keypad, the
/// pair task T-02-1a separated.
const SCAN_SLASH: u16 = 0x35;

/// The extended flag of FR-05 for a key of the main block, and for one of the keypad.
const MAIN_BLOCK: bool = false;
/// See [`MAIN_BLOCK`].
const KEYPAD: bool = true;

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
    assert!(!press.extended, "the A key is not an extended key");
    press.scan
}

/// The single character a key produces, or a failure naming the coordinates.
fn char_at(map: &LayoutMap, scan: u16, extended: bool, mods: Mods) -> char {
    let mapping = map.lookup(scan, extended, mods);
    mapping.single_char().unwrap_or_else(|| {
        panic!(
            "layout {} key 0x{:03X} mods 0b{:03b} produced {:?}, not a single character",
            map.layout(),
            usize::from(scan & 0xFF) | usize::from(extended) << 8,
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
    assert_eq!(
        char_at(map_of(&cache, RUSSIAN), scan, MAIN_BLOCK, Mods::NONE),
        'ф'
    );
}

// -- criterion 12 ------------------------------------------------------------------------

#[test]
fn the_a_key_unmodified_produces_latin_a_in_us() {
    let cache = cache();
    let scan = scan_of_a(&cache);
    assert_eq!(
        char_at(map_of(&cache, US), scan, MAIN_BLOCK, Mods::NONE),
        'a'
    );
}

// -- criterion 13 ------------------------------------------------------------------------

#[test]
fn shift_on_the_a_key_produces_capitals_in_both_layouts() {
    let cache = cache();
    let scan = scan_of_a(&cache);
    assert_eq!(
        char_at(map_of(&cache, US), scan, MAIN_BLOCK, Mods::SHIFT),
        'A'
    );
    assert_eq!(
        char_at(map_of(&cache, RUSSIAN), scan, MAIN_BLOCK, Mods::SHIFT),
        'Ф'
    );
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
        char_at(map_of(&cache, US), digit_two.scan, MAIN_BLOCK, Mods::SHIFT),
        '@'
    );
    assert_eq!(
        char_at(
            map_of(&cache, RUSSIAN),
            digit_two.scan,
            MAIN_BLOCK,
            Mods::SHIFT
        ),
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
        char_at(russian, press.scan, press.extended, press.mods),
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
            assert_eq!(char_at(map, press.scan, press.extended, press.mods), ch);
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

// -- task T-02-1a, the extended key in the cache key ---------------------------------------

#[test]
fn the_main_block_and_the_keypad_slash_keys_occupy_different_slots() {
    // Both keys report the scan code 0x35 and `MapVirtualKeyExW(MAPVK_VK_TO_VSC_EX)` tells
    // them apart by the `E0` prefix: 0x0035 for the `/?` key, 0xE035 for the keypad `/`.
    // Before task T-02-1a they shared one slot and the keypad won it in both layouts, which
    // is why the Russian map read `/` where the layout says `.`.
    let cache = cache();

    for (layout, plain, shifted) in [(US, '/', '?'), (RUSSIAN, '.', ',')] {
        let map = map_of(&cache, layout);

        // The main block key, with its own reading in each layout.
        assert_eq!(char_at(map, SCAN_SLASH, MAIN_BLOCK, Mods::NONE), plain);
        assert_eq!(char_at(map, SCAN_SLASH, MAIN_BLOCK, Mods::SHIFT), shifted);

        // And the keypad key, still there, reading `/` whatever the layout and whatever the
        // Shift state: two keys, two slots, both reachable.
        assert_eq!(char_at(map, SCAN_SLASH, KEYPAD, Mods::NONE), '/');
        assert_eq!(char_at(map, SCAN_SLASH, KEYPAD, Mods::SHIFT), '/');
    }

    // The Russian map is where the two used to collapse into one answer.
    let russian = map_of(&cache, RUSSIAN);
    assert_ne!(
        russian.lookup(SCAN_SLASH, MAIN_BLOCK, Mods::NONE),
        russian.lookup(SCAN_SLASH, KEYPAD, Mods::NONE)
    );
}

#[test]
fn the_reverse_index_carries_the_extended_flag() {
    // A synthetic layout, because no character of the RU/EN pair is carried by a keypad key
    // alone: the reverse index keeps the first writer and the main block is written first,
    // so the live cache cannot show the `true` case. Here the two keys carry different
    // characters and each is found under its own flag.
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0xF001_0409));
    builder.set(
        SCAN_SLASH,
        MAIN_BLOCK,
        Mods::NONE,
        KeyMapping::from_char('/'),
    );
    builder.set(
        SCAN_SLASH,
        KEYPAD,
        Mods::NONE,
        KeyMapping::from_char('\u{00F7}'),
    );
    let map = builder.finish();

    assert_eq!(
        map.find_key('/'),
        Some(KeyPress {
            scan: SCAN_SLASH,
            extended: false,
            mods: Mods::NONE,
        })
    );
    assert_eq!(
        map.find_key('\u{00F7}'),
        Some(KeyPress {
            scan: SCAN_SLASH,
            extended: true,
            mods: Mods::NONE,
        })
    );

    // On the live cache both `/` keys carry the same character, and the answer is the main
    // block one — the key a user pressed, and the key task T-07-2 has to replay.
    let cache = cache();
    let press = map_of(&cache, US)
        .find_key('/')
        .expect("the US layout must have a key that produces '/'");
    assert_eq!(press.scan, SCAN_SLASH);
    assert!(
        !press.extended,
        "'/' must be found on the main block key, not on the keypad"
    );
    assert_eq!(press.mods, Mods::NONE);

    // The reverse index of the Russian layout gained a key it did not have before: `.` sits
    // on the very slot the keypad used to take.
    let full_stop = map_of(&cache, RUSSIAN)
        .find_key('.')
        .expect("the Russian layout must have a key that produces '.'");
    assert_eq!(full_stop.scan, SCAN_SLASH);
    assert!(!full_stop.extended);
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
    builder.set(0x10, MAIN_BLOCK, Mods::NONE, dead);
    builder.set(0x11, MAIN_BLOCK, Mods::NONE, ligature);
    builder.set(0x12, MAIN_BLOCK, Mods::NONE, outside_bmp);
    let map = builder.finish();

    let stored_dead = map.lookup(0x10, MAIN_BLOCK, Mods::NONE);
    assert_eq!(stored_dead.kind(), MappingKind::Dead);
    assert_eq!(stored_dead.units(), [0x0300]);
    assert!(stored_dead.is_dead());

    let stored_ligature = map.lookup(0x11, MAIN_BLOCK, Mods::NONE);
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
    assert_eq!(map.lookup(0x12, MAIN_BLOCK, Mods::NONE).units().len(), 2);
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

/// **FR-24: a dead key the OS did not actually write is nothing, not a dead `NUL`.**
///
/// Written for task T-13-21 after the audit of 2026-08-24 ("страховка мёртвой клавиши в
/// `KeyMapping::from_to_unicode` проверяет не то и потому мертва", `src\layouts.rs:439`). The
/// guarantee is reached through the seam the module already offers: `from_to_unicode` takes the
/// raw return value and the buffer separately, so an answer the OS is not supposed to give can be
/// handed to it by hand, without a keyboard and without a layout.
///
/// The point of the case is what the buffer looks like on the *product* path. `decode` always
/// passes a full scratch slice of `DECODE_UNITS` zeroes, so a call that returned a dead key
/// without writing the spacing character its documentation promises "if possible" leaves a zero
/// in the first unit — never an empty slice. A dead mapping carrying `\0` would then be carried
/// over unchanged by FR-24 and handed to `SendInput` as a character.
#[test]
fn a_negative_return_that_wrote_nothing_reads_as_empty_and_not_as_a_dead_nul() {
    // What `decode` really hands over: the scratch buffer, untouched.
    let untouched = [0u16; 16];
    let broken_contract = KeyMapping::from_to_unicode(-1, &untouched);

    assert_eq!(broken_contract.kind(), MappingKind::None);
    assert!(
        broken_contract.is_empty(),
        "nothing, as the comment promises"
    );
    assert!(
        !broken_contract.is_dead(),
        "a dead key with no dead character is not a dead key"
    );
    assert!(
        broken_contract.units().is_empty(),
        "and nothing reaches SendInput"
    );
    assert_eq!(broken_contract.single_char(), None);

    // A short slice is the same answer, so the guarantee does not rest on the length either.
    assert!(KeyMapping::from_to_unicode(-1, &[]).is_empty());

    // And the dead key the OS does write is unchanged: the first unit is the dead character,
    // the entry is one unit long whatever else the scratch buffer still holds.
    let mut written = [0u16; 16];
    written[0] = 0x0300;
    let dead = KeyMapping::from_to_unicode(-1, &written);

    assert_eq!(dead.kind(), MappingKind::Dead);
    assert!(dead.is_dead());
    assert_eq!(dead.units(), [0x0300]);
    assert_eq!(dead, KeyMapping::dead('\u{0300}'));
}

#[test]
fn a_key_with_no_character_reads_as_empty_rather_than_as_a_failure() {
    let cache = cache();
    let us = map_of(&cache, US);
    // Scan code 0 is not a key: nothing may be filed under it.
    assert!(us.lookup(0x00, MAIN_BLOCK, Mods::NONE).is_empty());
    assert_eq!(
        us.lookup(0x00, MAIN_BLOCK, Mods::NONE).kind(),
        MappingKind::None
    );
    // FR-23 rests on this: an absent mapping is an ordinary answer, and the caller carries
    // the character over unchanged instead of aborting the conversion.
    assert!(us.lookup(0xFF, MAIN_BLOCK, Mods::ALTGR).is_empty());

    // The index is bounded by construction, not by the caller: a scan code that still has
    // the `E0` prefix in it, or any other value a hook could report, is masked into the
    // table rather than read past its end.
    assert!(us.lookup(0xE000, KEYPAD, Mods::ALTGR).is_empty());
    assert!(
        us.lookup(u16::MAX, KEYPAD, Mods::from_bits_truncate(0xFF))
            .is_empty()
    );
    assert_eq!(
        us.lookup(0xE01E, MAIN_BLOCK, Mods::NONE),
        us.lookup(SCAN_A, MAIN_BLOCK, Mods::NONE),
        "only the low byte of a scan code selects a key"
    );
}

// -------------------------------------------------------------------------------------
// §4.4 — choosing the target layout. Task T-05-2.
//
// Everything below is arithmetic over layout identifiers and touches neither the OS nor the
// session's layout list: the lists are written out by hand, exactly as FR-30 and FR-31 let a
// configuration write them, so the answers are the same on any machine.
// -------------------------------------------------------------------------------------

/// Greek, the third layout of position 17 of the matrix — synthetic, nothing is attached here.
const GREEK: LayoutId = LayoutId::from_raw(0x0408_0408);

/// Chinese Simplified, Microsoft Pinyin — an **IME**, excluded by FR-35.
///
/// The top nibble of the device handle is `0xE`, which is what `LayoutId::is_ime` reads.
const PINYIN: LayoutId = LayoutId::from_raw(0xE020_0804);

/// `[layouts]` of section 7, as a hand-edited file would carry it.
fn settings(mode: LayoutMode, pair: [&str; 2], cycle: &[&str]) -> Configured {
    Configured::from_settings(&Layouts {
        mode,
        pair_source: pair[0].to_owned(),
        pair_target: pair[1].to_owned(),
        cycle: cycle.iter().map(|entry| (*entry).to_owned()).collect(),
    })
}

/// The turn of the tests that **raise or measure** the `no_layouts` counter of
/// [`selection_failures`].
///
/// The three counters are process-wide, and the tests of one binary run side by side, so a test
/// asserting that a counter rose by exactly one has to be the only test raising it while it runs.
///
/// ⚠ **The turn is taken by every test that moves the counter, not only by those that read it —
/// task T-13-30.** This comment used to say "the two tests below that touch `no_layouts`", and a
/// third one did: `cycle_mode_walks_the_list_in_order_and_comes_back_to_the_start` raises the
/// refusal once, at its last assertion, and said nothing about the count. A test that raises a
/// counter **outside** the turn is exactly as damaging as one that reads it outside — it is what
/// makes the reader's `before + 1` come out as `before + 2` — and it is the reason
/// `a_cycle_list_that_outlives_its_layouts_refuses_instead_of_walking_the_session` was seen red
/// under load by the executor of T-13-17 while being green five times out of five in isolation. A
/// battery that flickers is a battery nobody trusts, and this stage does not get to leave one
/// behind.
///
/// The three that take it, and what each does to the counter:
///
/// * `cycle_mode_walks_the_list_in_order_and_comes_back_to_the_start` — **raises** it once and
///   asserts nothing about it;
/// * `a_cycle_list_that_outlives_its_layouts_refuses_instead_of_walking_the_session` — raises it
///   twice and asserts **exactly** `before + 1` and then `before + 2`;
/// * `a_cycle_is_bounded_and_holds_no_repetitions` — raises it three times and asserts a bound.
///
/// Nothing else in the file can move `no_layouts`: the only two places that raise it are
/// `Cycle::from_layouts` when fewer than two distinct layouts are left and `Cycle::target` on a
/// cycle shorter than two, and every call of either that can refuse is inside one of the three.
/// The other two counters — `ime_layout` and `origin_outside` — are asserted as bounds wherever
/// they appear and need no turn.
static REFUSAL_COUNTERS: Mutex<()> = Mutex::new(());

/// See [`REFUSAL_COUNTERS`]. A poisoned turn is still a turn: what it guards is a counter, and a
/// panic in another test does not make it unreadable.
fn refusal_counters() -> MutexGuard<'static, ()> {
    REFUSAL_COUNTERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// The turn of the tests that touch the **process-wide** published `[layouts]`.
///
/// `layouts::publish` and `layouts::published` talk through statics, so two tests that both write
/// them, or one that writes while another reads, cannot mean anything running side by side. The
/// same device as [`REFUSAL_COUNTERS`] and for the same reason; the two tests below this line are
/// the only ones in the file that need it.
static PUBLICATION: Mutex<()> = Mutex::new(());

/// See [`PUBLICATION`]. A poisoned turn is still a turn.
fn publication() -> MutexGuard<'static, ()> {
    PUBLICATION.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The pair of decision 19, in mode `pair`.
fn the_pair_of_decision_19() -> Configured {
    settings(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    )
}

// -------------------------------------------------------------------------------------
// Point 13 — FR-30: exactly two layouts, and no configuration at all
// -------------------------------------------------------------------------------------

#[test]
fn two_layouts_in_the_session_choose_the_target_without_any_configuration() {
    // "Если в системе установлены ровно две раскладки — работа полностью автоматическая,
    // настройка не требуется: целевая раскладка есть вторая из двух." Nothing has been
    // published and nothing is read: `Configured::default` is what the input thread sees
    // before the UI thread has said anything at all.
    let session = [US, RUSSIAN];
    let cycle = cycle_for(Configured::default(), &session).expect("two layouts are a pair");

    assert_eq!(cycle.layouts(), session);
    assert_eq!(
        cycle.target(US, 1).expect("the second of the two"),
        RUSSIAN,
        "FR-30: the target is the other one of the two"
    );
    assert_eq!(
        cycle.target(RUSSIAN, 1).expect("the second of the two"),
        US,
        "and it is symmetric — either layout may be the one typed under"
    );

    // The same answer with a `config.toml` that names layouts this session does not have. A
    // machine in the state decision 19 fixes has to work whatever the file says, and this is
    // the case the first bullet of FR-30 is written for.
    let nonsense = settings(
        LayoutMode::Pair,
        ["0x0000040C", "0x00000407"],
        &["0x0000040C"],
    );
    let cycle = cycle_for(nonsense, &session).expect("two layouts are a pair regardless");

    assert_eq!(cycle.layouts(), session);
    assert_eq!(cycle.target(US, 1).expect("the other one"), RUSSIAN);
}

// -------------------------------------------------------------------------------------
// Point 14 — FR-30: three or more, mode `pair`
// -------------------------------------------------------------------------------------

#[test]
fn three_layouts_in_pair_mode_walk_the_named_pair_and_ignore_the_rest() {
    let session = [US, RUSSIAN, GREEK];

    // "Пользователь явно задаёт рабочую пару «источник ↔ цель». Остальные раскладки
    // игнорируются."
    let cycle = cycle_for(the_pair_of_decision_19(), &session).expect("the named pair");

    assert_eq!(cycle.layouts(), [US, RUSSIAN]);
    assert!(!cycle.contains(GREEK), "the third layout takes no part");
    assert_eq!(cycle.target(US, 1).expect("the pair"), RUSSIAN);
    assert_eq!(cycle.target(RUSSIAN, 1).expect("the pair"), US);

    // Text typed under the layout that takes no part is not converted at all: the direction
    // it would be converted in is one the user did not give.
    let before = selection_failures().origin_outside;

    assert_eq!(cycle.target(GREEK, 1), Err(SelectionError::OriginOutside));
    assert!(
        selection_failures().origin_outside > before,
        "the refusal is counted"
    );

    // The pair the user names is the pair that is walked, whichever two of the three it is.
    let other_pair = settings(
        LayoutMode::Pair,
        ["0x00000419", "0x00000408"],
        &["0x00000409", "0x00000419"],
    );
    let cycle = cycle_for(other_pair, &session).expect("the named pair");

    assert_eq!(cycle.layouts(), [RUSSIAN, GREEK]);
    assert_eq!(cycle.target(RUSSIAN, 1).expect("the pair"), GREEK);

    // A pair naming a layout this session does not have falls back to the rule FR-30 gives
    // for the dialog — the first two of the system list — rather than stopping the program.
    let absent = settings(
        LayoutMode::Pair,
        ["0x0000040C", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );
    let cycle = cycle_for(absent, &session).expect("the prefill of FR-30");

    assert_eq!(cycle.layouts(), [US, RUSSIAN]);
}

// -------------------------------------------------------------------------------------
// Point 15 — FR-31: variant 2, then variant 3, then round
// -------------------------------------------------------------------------------------

#[test]
fn cycle_mode_walks_the_list_in_order_and_comes_back_to_the_start() {
    // The third test that moves `no_layouts` — see [`REFUSAL_COUNTERS`]. It asserts nothing about
    // the counter itself, and takes the turn for the sake of the two that do: the refusal at the
    // foot of this test is what used to land between their `before` and their `after`.
    let _serialised = refusal_counters();

    let session = [US, RUSSIAN, GREEK];
    let three = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419", "0x00000408"],
    );

    let cycle = cycle_for(three, &session).expect("the configured list");

    assert_eq!(cycle.layouts(), session);

    // "первое нажатие — вариант 2, второе — вариант 3, и так далее по кругу с возвратом
    // к исходному варианту."
    assert_eq!(cycle.target(US, 1).expect("variant 2"), RUSSIAN);
    assert_eq!(cycle.target(US, 2).expect("variant 3"), GREEK);
    assert_eq!(cycle.target(US, 3).expect("back to the start"), US);
    assert_eq!(cycle.target(US, 4).expect("round again"), RUSSIAN);

    // The order is the file's, not the system's.
    let reversed = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000408", "0x00000419"],
    );
    let cycle = cycle_for(reversed, &session).expect("the configured list");

    assert_eq!(cycle.layouts(), [US, GREEK, RUSSIAN]);
    assert_eq!(cycle.target(US, 1).expect("variant 2"), GREEK);

    // A list mentioning layouts this session does not have is the list of the others: two or
    // more survivors are walked exactly as before, in the file's order.
    let partly_absent = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x0000040C", "0x00000419", "0x00000409"],
    );
    assert_eq!(
        cycle_for(partly_absent, &session)
            .expect("what is left of the list")
            .layouts(),
        [RUSSIAN, US]
    );

    // Fewer than two survivors is a refusal and not the session's own layouts — the test below
    // is the one that states why.
    let all_absent = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x0000040C"],
    );
    assert_eq!(
        cycle_for(all_absent, &session),
        Err(SelectionError::NoLayouts)
    );
}

/// **FR-31 has no prefill, and a tick taken off stays off.**
///
/// Written knowingly for task T-13-21, in place of the assertion
/// `cycle_mode_walks_the_list_in_order_and_comes_back_to_the_start` used to carry — that a cycle
/// list resolving to fewer than two layouts is answered with the session's own layouts. The audit
/// of 2026-08-24 struck that fallback out ("фолбэк режима «Несколько» шагает по всем раскладкам
/// сессии, включая явно исключённые пользователем", `src\layouts.rs:1609`).
///
/// The behaviour changed because the list of FR-31 is the "список раскладок с галочками участия"
/// of FR-92: a layout outside it is a layout the user excluded by hand, and section 7 keeps only
/// the participants, so a list that resolves to one layout is indistinguishable from no list at
/// all. Walking the session in that case walks precisely what was excluded. FR-31 licenses no
/// fallback to lean on — the prefill sentence of FR-30 is written for the pair alone (which is
/// why the test below still finds it there) — so the honest answer is the refusal that already
/// exists and is already counted.
#[test]
fn a_cycle_list_that_outlives_its_layouts_refuses_instead_of_walking_the_session() {
    let _serialised = refusal_counters();

    // The user ticked EN and RU out of a session of EN, RU and EL. While both are present the
    // list is walked as it always was, and the layout left unticked takes no part.
    let ticked = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );
    let cycle = cycle_for(ticked, &[US, RUSSIAN, GREEK]).expect("both participants are present");

    assert_eq!(cycle.layouts(), [US, RUSSIAN]);
    assert!(
        !cycle.contains(GREEK),
        "the layout left unticked takes no part"
    );

    // Then RU is removed from the system. One participant is left, and the answer is a refusal:
    // before this task the hotkey started converting into EL — the one layout the user had
    // explicitly excluded.
    let before = selection_failures().no_layouts;

    assert_eq!(
        cycle_for(ticked, &[US, GREEK]),
        Err(SelectionError::NoLayouts),
        "one live participant is nothing to switch between, not a licence to walk the session"
    );

    assert_eq!(
        selection_failures().no_layouts,
        before + 1,
        "the refusal is counted, and counted once"
    );

    // A list that resolves to nothing at all is the same answer, counted the same way.
    let none_left = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x0000040C", "0x00000407"],
    );

    assert_eq!(
        cycle_for(none_left, &[US, GREEK]),
        Err(SelectionError::NoLayouts)
    );

    assert_eq!(selection_failures().no_layouts, before + 2);
}

/// **The pair keeps its prefill — FR-30, unchanged by task T-13-21.**
///
/// The refusal above is the `cycle` branch only. FR-30 says in as many words that the fields
/// "предзаполняются первыми двумя раскладками системного списка", so a pair naming a layout this
/// session does not have falls back to the first two of the session, and a session of exactly two
/// does not consult the configuration at all. Both are asserted here against the very
/// configuration that the test above refuses, so that the two branches cannot quietly be made to
/// agree.
#[test]
fn the_pair_still_prefills_from_the_session_where_the_cycle_refuses() {
    /// German — a third layout of this session, so that the prefill of FR-30 has something to
    /// prefill *from* and the answer is not simply "the whole session".
    const GERMAN: LayoutId = LayoutId::from_raw(0x0407_0407);

    let session = [US, GREEK];

    // The same participants that leave one survivor, read in mode `pair`: the prefill of FR-30
    // answers the first two of the session instead of refusing.
    let as_pair = settings(
        LayoutMode::Pair,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0x00000419"],
    );

    // The target of the named pair is not in this session, so the named pair is unusable.
    let cycle = cycle_for(as_pair, &[US, GREEK, GERMAN]).expect("the prefill of FR-30");

    assert_eq!(cycle.layouts(), [US, GREEK]);
    assert!(!cycle.contains(GERMAN), "the first two, as FR-30 words it");

    // And with exactly two layouts in the session the configuration is not consulted at all —
    // the first bullet of FR-30, which no change to the cycle branch may touch.
    let cycle = cycle_for(as_pair, &session).expect("two layouts are a pair regardless");

    assert_eq!(cycle.layouts(), session);
    assert_eq!(cycle.target(US, 1).expect("the other one"), GREEK);
}

// -------------------------------------------------------------------------------------
// Point 18 — FR-35: an IME in the pair or in the cycle list
// -------------------------------------------------------------------------------------

#[test]
fn an_ime_layout_named_as_a_participant_is_refused_and_counted() {
    assert!(PINYIN.is_ime(), "the fixture is an IME to begin with");
    assert!(!US.is_ime() && !RUSSIAN.is_ime() && !GREEK.is_ime());

    // In the pair: the session has exactly two layouts and one of them is an IME. FR-35
    // excludes it from the participating set, so there is no pair to switch between and the
    // answer is a refusal — "обработать как отказ и сосчитать, а не переключаться".
    let before = selection_failures().ime_layout;

    assert_eq!(
        cycle_for(the_pair_of_decision_19(), &[US, PINYIN]),
        Err(SelectionError::ImeLayout)
    );

    // In the cycle list, named by hand in a damaged `config.toml`.
    let with_an_ime = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000409", "0xE0200804", "0x00000419"],
    );

    assert_eq!(
        cycle_for(with_an_ime, &[US, PINYIN, RUSSIAN]),
        Err(SelectionError::ImeLayout)
    );

    // And through the composition the product makes — `cycle_for` and then `Cycle::target`,
    // which is what `inject::take_press` walks on the hotkey path — where a refusal turns into
    // "no replacement and no switch". Until task T-13-21 this line went through a wrapper
    // `layouts::target_for`, which no product code called; the wrapper is gone and the test now
    // exercises the same two steps in the same order as the product.
    assert_eq!(
        cycle_for(the_pair_of_decision_19(), &[US, PINYIN]).and_then(|cycle| cycle.target(US, 1)),
        Err(SelectionError::ImeLayout)
    );

    let after = selection_failures().ime_layout;

    assert!(
        after >= before + 3,
        "FR-35: every refusal is counted — {before} then {after}"
    );

    // An IME merely *present* in the session is not an error: FR-35 excludes it from the
    // participants, and a pair naming the other two is a pair.
    let cycle = cycle_for(the_pair_of_decision_19(), &[US, PINYIN, RUSSIAN])
        .expect("the named pair does not include the IME");

    assert_eq!(cycle.layouts(), [US, RUSSIAN]);
}

// -------------------------------------------------------------------------------------
// The identifiers of section 7, and the bounds of the list
// -------------------------------------------------------------------------------------

#[test]
fn the_identifiers_of_section_7_are_read_in_both_forms() {
    // The form section 7 prints: the language in the low word, the high word zero. It names
    // the language and matches whatever handle this session gave it.
    let us = LayoutSpec::parse("0x00000409");

    assert!(us.matches(US), "0x00000409 names the US layout");
    assert!(
        us.matches(LayoutId::from_raw(0xF001_0409)),
        "and any handle serving it"
    );
    assert!(!us.matches(RUSSIAN));

    // A handle written out in full is compared exactly, so that two layouts of one language
    // can still be told apart.
    let handle = LayoutSpec::parse("0x04090409");

    assert!(handle.matches(US));
    assert!(!handle.matches(LayoutId::from_raw(0xF001_0409)));

    // Without the prefix, which is how the registry writes a keyboard layout identifier.
    assert!(LayoutSpec::parse("00000419").matches(RUSSIAN));
    assert!(LayoutSpec::parse("  0x00000419  ").matches(RUSSIAN));

    // A field this cannot read names nothing and matches nothing — a typo leaves the program
    // on its defaults rather than selecting something at random.
    assert!(LayoutSpec::parse("ru-RU").is_none());
    assert!(LayoutSpec::parse("").is_none());
    assert!(!LayoutSpec::parse("zzz").matches(US));

    // Resolution against the session is the same rule, applied to a list.
    assert_eq!(us.resolve(&[RUSSIAN, US]), Some(US));
    assert_eq!(
        LayoutSpec::parse("0x0000040C").resolve(&[RUSSIAN, US]),
        None
    );
}

#[test]
fn a_cycle_is_bounded_and_holds_no_repetitions() {
    // One of the three tests that raise `no_layouts`; see [`REFUSAL_COUNTERS`].
    let _serialised = refusal_counters();

    // A list longer than the bound keeps its first `MAX_CYCLE` entries: a hand-edited file
    // must not be able to turn into an unbounded array on the hotkey path.
    let many: Vec<LayoutId> = (0..(MAX_CYCLE as u16 + 4))
        .map(|index| LayoutId::from_raw(0x0001_0000 | usize::from(index)))
        .collect();
    let cycle = Cycle::from_layouts(&many).expect("more than two layouts");

    assert_eq!(cycle.len(), MAX_CYCLE);
    assert_eq!(cycle.layouts(), &many[..MAX_CYCLE]);

    // A layout named twice takes one place: the counter must not spend a press standing
    // still.
    let cycle = Cycle::from_layouts(&[US, RUSSIAN, US]).expect("two distinct layouts");

    assert_eq!(cycle.layouts(), [US, RUSSIAN]);

    // Fewer than two layouts is nothing to switch between, and it is counted like any other
    // refusal.
    let before = selection_failures().no_layouts;

    assert_eq!(Cycle::from_layouts(&[US]), Err(SelectionError::NoLayouts));
    assert_eq!(Cycle::from_layouts(&[]), Err(SelectionError::NoLayouts));
    assert_eq!(
        Cycle::EMPTY.target(US, 1),
        Err(SelectionError::NoLayouts),
        "and an empty cycle answers nothing"
    );

    assert!(selection_failures().no_layouts >= before + 3);

    // A step far beyond the length is reduced rather than added: a counter that has been
    // running for a while cannot overflow the sum.
    let cycle = Cycle::from_layouts(&[US, RUSSIAN, GREEK]).expect("three layouts");

    assert_eq!(
        cycle.target(US, usize::MAX).expect("reduced"),
        cycle.target(US, usize::MAX % 3).expect("reduced")
    );
    assert_eq!(cycle.target(US, 3_000).expect("reduced"), US);
}

// -------------------------------------------------------------------------------------
// Point 22 — SEC-01, SEC-07: nothing here can carry a keystroke
// -------------------------------------------------------------------------------------

#[test]
fn nothing_the_selection_can_say_carries_a_keystroke() {
    // The three refusals are the only things this half of the module ever formats, and each
    // of them is a sentence about layouts — no character, no scan code, no virtual key, and
    // not even a layout identifier.
    for error in [
        SelectionError::ImeLayout,
        SelectionError::NoLayouts,
        SelectionError::OriginOutside,
    ] {
        let text = error.to_string();

        assert!(!text.is_empty());
        assert!(text.is_ascii(), "an outcome, not a keystroke");
        assert!(!text.contains("0x"), "not even a handle");
    }

    // And what the counters report are counts.
    let counted = selection_failures();
    let _: (u32, u32, u32) = (
        counted.ime_layout,
        counted.no_layouts,
        counted.origin_outside,
    );
}

// -------------------------------------------------------------------------------------
// `[layouts]` reaches the input thread
// -------------------------------------------------------------------------------------

/// Section 7 travels from a `settings::Layouts` into the atomics the hotkey path reads.
///
/// The publication is process-wide, so this test and the interleaving one below it take the turn
/// of [`PUBLICATION`] and no other test in this file touches the atomics at all; every test above
/// passes its configuration in by hand. It puts the defaults back when it is done, for the reason
/// `tests\inject.rs` states about the two atomics of `[replacement]`.
#[test]
fn the_layouts_section_of_the_configuration_reaches_the_input_thread() {
    let _turn = publication();

    let configured = settings(
        LayoutMode::Cycle,
        ["0x00000409", "0x00000419"],
        &["0x00000419", "0x00000408", "0x00000409"],
    );

    lang_switcher::layouts::publish(configured);

    let read_back = published();

    assert_eq!(read_back.mode(), LayoutMode::Cycle);
    assert!(read_back.pair()[0].matches(US));
    assert!(read_back.pair()[1].matches(RUSSIAN));
    assert_eq!(read_back.cycle().len(), 3);
    assert_eq!(
        cycle_for(read_back, &[US, RUSSIAN, GREEK])
            .expect("the published list")
            .layouts(),
        [RUSSIAN, GREEK, US]
    );

    // The defaults of section 7, back where they were.
    lang_switcher::layouts::publish(Configured::from_settings(&Layouts::default()));

    let default = published();

    assert_eq!(default.mode(), LayoutMode::Pair);
    assert_eq!(
        cycle_for(default, &[US, RUSSIAN, GREEK])
            .expect("the pair of decision 19")
            .layouts(),
        [US, RUSSIAN]
    );
}

/// The layout the **first** publication of the interleaving test names in both halves of its pair.
///
/// A keyboard layout identifier of section 7 and not a handle: `LayoutSpec::matches` reads a spec
/// whose high word is zero as a language, so this one names US whichever handle the session gave
/// it. Nothing about this test needs the layout to be loaded — the specs never leave the atomics.
#[cfg(feature = "testing")]
const FIRST_PUBLICATION: &str = "0x00000409";

/// The layout the **second** publication names in both halves. See [`FIRST_PUBLICATION`].
#[cfg(feature = "testing")]
const SECOND_PUBLICATION: &str = "0x00000419";

/// The whole publication the seam drives into the middle of a read.
///
/// A plain `fn()` because that is what the seam takes: it is stored in a `OnceLock` inside the
/// module and called from the read, and a capturing closure could not travel there.
#[cfg(feature = "testing")]
fn publish_the_second_configuration() {
    lang_switcher::layouts::publish(settings(
        LayoutMode::Pair,
        [SECOND_PUBLICATION, SECOND_PUBLICATION],
        &[],
    ));
}

/// **A publication landing between the two halves of the pair is not read in halves.**
///
/// Written for task T-13-22 after the audit of 2026-08-24 ("публикация `[layouts]` в поток ввода
/// допускает рваное чтение: россыпь атомиков без поколения, в отличие от seqlock у guard",
/// `src\layouts.rs:1496`). The audit's own example is the shape of this test: a press of the
/// hotkey that "прочитает `pair[0]` новым, а `pair[1]` старым".
///
/// The race is made an appointment rather than raced for. The seam of `layouts::seam` runs an
/// armed publication once, after `published` has loaded `PAIR[0]` and before it loads `PAIR[1]`,
/// so the two halves of the pair are taken from two different configurations **by construction**
/// and on one thread. Two threads spinning at each other would prove nothing: a green run would
/// only mean the collision did not happen that day.
///
/// What the assertions below say is therefore exactly what the generation buys and nothing more:
///
/// * the two halves agree, so they came from one publication rather than two;
/// * that publication is the **second** one — which nothing in this test body published, so a pair
///   of Russian is at the same time the proof that the seam fired at all and that the read was
///   retried instead of stitched.
///
/// Without the generation the answer is `[US, RUSSIAN]` and the first assertion fails; the
/// experiment was run, and the report of the task records it.
#[cfg(feature = "testing")]
#[test]
fn a_publication_between_the_halves_of_the_pair_is_not_read_in_halves() {
    let _turn = publication();

    // What stands before the press: US in both halves of the pair.
    lang_switcher::layouts::publish(settings(
        LayoutMode::Pair,
        [FIRST_PUBLICATION, FIRST_PUBLICATION],
        &[],
    ));

    // The appointment.
    lang_switcher::layouts::interleave_next_read(publish_the_second_configuration);

    let read = published();

    assert_eq!(
        read.pair()[0],
        read.pair()[1],
        "the two halves of the pair must come from one publication, not one from each"
    );
    assert!(
        read.pair()[0].matches(RUSSIAN),
        "and from the publication the read ended in — which only the seam performed"
    );
    assert!(
        !read.pair()[1].matches(US),
        "nothing of the configuration that was standing may survive into the answer"
    );

    // The defaults of section 7, back where they were.
    lang_switcher::layouts::publish(Configured::from_settings(&Layouts::default()));
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
