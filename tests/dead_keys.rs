//! Dead keys in the conversion — решение 195.1, task T-105-5 (stage Э105).
//!
//! Cases 10 and 11 of замер 187 on Test_VM: `être` typed in US with the keys of French AZERTY
//! (`[etre`) stayed `[etre` after the hotkey, and `ê` typed in French as `^` + `e` came back into
//! US as `^e` instead of `[e`. The two rules of the decision: (а) a dead stroke of the source is
//! converted as an ordinary one — the character of the same key in the target; (б) a key that is
//! dead in the target composes with the stroke after it, by the target's own table, as Windows
//! does, and with nothing to compose with puts the accent and the letter side by side (`^t`).
//! Decision 75 stays: a dead stroke typed by the user counts nothing on the screen.
//!
//! The layouts here are synthetic and state only the keys they use, by the scan codes of the
//! physical keys of a US keyboard; the compositions are those of the layouts' own files, and the
//! last test reads one of them (`KBDFR.DLL`) off the machine to show they are.

use lang_switcher::convert::{self, Keystroke};
use lang_switcher::inject::OnScreen;
use lang_switcher::layouts::{
    self, DeadTransition, KeyMapping, LayoutId, LayoutMap, LayoutMapBuilder, Mods,
};
use lang_switcher::selection;

const SCAN_BRACKET: u16 = 0x1A;
const SCAN_QUOTE: u16 = 0x28;
const SCAN_BACKSLASH: u16 = 0x2B;
const SCAN_SPACE: u16 = 0x39;

/// The letter keys these tests type, by the US letter they carry.
const LETTERS: [(char, u16); 9] = [
    ('a', 0x1E),
    ('c', 0x2E),
    ('e', 0x12),
    ('i', 0x17),
    ('n', 0x31),
    ('o', 0x18),
    ('r', 0x13),
    ('t', 0x14),
    ('x', 0x2D),
];

/// A layout with the letters of [`LETTERS`] and the space bar, `extra` keys, and `transitions`.
fn layout(
    raw: usize,
    extra: &[(u16, KeyMapping)],
    transitions: &[(char, char, char)],
) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(raw));

    for (letter, scan) in LETTERS {
        builder.set(scan, false, Mods::NONE, KeyMapping::from_char(letter));
    }

    builder.set(SCAN_SPACE, false, Mods::NONE, KeyMapping::from_char(' '));

    for &(scan, mapping) in extra {
        builder.set(scan, false, Mods::NONE, mapping);
    }

    for &(accent, base, composed) in transitions {
        builder.add_dead_transition(accent, base, composed);
    }

    builder.finish()
}

/// US: `[`, `'` and `\` are ordinary keys; no dead key.
fn us() -> LayoutMap {
    layout(
        0x0409_0409,
        &[
            (SCAN_BRACKET, KeyMapping::from_char('[')),
            (SCAN_QUOTE, KeyMapping::from_char('\'')),
            (SCAN_BACKSLASH, KeyMapping::from_char('\\')),
        ],
        &[],
    )
}

/// French AZERTY: the key of US `[` is the dead circumflex.
fn french() -> LayoutMap {
    layout(
        0x040C_040C,
        &[(SCAN_BRACKET, KeyMapping::dead('^'))],
        &[('^', 'e', 'ê'), ('^', 'a', 'â'), ('^', ' ', '^')],
    )
}

/// Spanish: the key of US `'` is the dead acute.
fn spanish() -> LayoutMap {
    layout(
        0x040A_040A,
        &[(SCAN_QUOTE, KeyMapping::dead('\u{00B4}'))],
        &[('\u{00B4}', 'o', 'ó'), ('\u{00B4}', 'a', 'á')],
    )
}

/// Portuguese: the key of US `\` is the dead tilde.
fn portuguese() -> LayoutMap {
    layout(
        0x0816_0816,
        &[(SCAN_BACKSLASH, KeyMapping::dead('~'))],
        &[('~', 'a', 'ã'), ('~', 'o', 'õ')],
    )
}

/// The strokes of `text` typed in `map`, one key per character, by the keys `map` has for them.
fn typed(map: &LayoutMap, keys: &[u16]) -> Vec<Keystroke> {
    keys.iter()
        .map(|&scan| Keystroke::recorded_in(map, scan, false, Mods::NONE))
        .collect()
}

/// The scan codes of `text` on a US keyboard.
fn keys_of(text: &str) -> Vec<u16> {
    text.chars()
        .map(|ch| match ch {
            '[' => SCAN_BRACKET,
            '\'' => SCAN_QUOTE,
            '\\' => SCAN_BACKSLASH,
            ' ' => SCAN_SPACE,
            letter => {
                LETTERS
                    .iter()
                    .find(|(known, _)| *known == letter)
                    .expect("a letter of the test")
                    .1
            }
        })
        .collect()
}

/// What `strokes` become in `target`, as text.
fn rendered(strokes: &[Keystroke], target: &LayoutMap) -> String {
    let mut out = vec![0u16; convert::max_units(strokes.len())];
    let written = convert::convert_strokes(strokes, target, &mut out).expect("fits");
    String::from_utf16(&out[..written]).expect("valid text")
}

/// **(б) A key dead in the target composes with the stroke after it** — case 10 of 187 (`[etre`
/// typed in US is `être` in French), and the Spanish and Portuguese words of the decision.
#[test]
fn a_dead_key_of_the_target_composes_with_the_next_stroke_as_windows_does() {
    let us = us();

    assert_eq!(
        rendered(&typed(&us, &keys_of("[etre")), &french()),
        "être",
        "case 10"
    );
    assert_eq!(
        rendered(&typed(&us, &keys_of("canci'on")), &spanish()),
        "canción"
    );
    assert_eq!(
        rendered(&typed(&us, &keys_of("n\\ao")), &portuguese()),
        "não"
    );
}

/// **Nothing to compose with: the accent and the letter side by side** — `^` + `t` is `^t`, as
/// Windows puts it; a dead key at the end of the run goes out as its accent, and `^` + space is
/// the accent by the layout's own table.
#[test]
fn a_dead_key_with_nothing_to_compose_puts_the_accent_and_the_letter() {
    let us = us();
    let french = french();

    assert_eq!(
        rendered(&typed(&us, &keys_of("[t")), &french),
        "^t",
        "the negative control"
    );
    assert_eq!(
        rendered(&typed(&us, &keys_of("x[")), &french),
        "x^",
        "at the end"
    );
    assert_eq!(
        rendered(&typed(&us, &keys_of("[ x")), &french),
        "^x",
        "with the space"
    );
}

/// **(а) A dead stroke of the source is converted as an ordinary one** — case 11 of 187: `ê`
/// typed in French as `^` + `e` is the keys `[` and `e` of US, `[e`; until Э105 the dead stroke was
/// carried over as `^`. One character stood on the screen, and one is erased (decision 75).
#[test]
fn a_dead_stroke_of_the_source_becomes_the_character_of_its_key_in_the_target() {
    let french = french();
    let strokes = typed(&french, &keys_of("[e"));

    assert!(
        strokes[0].is_dead(),
        "the circumflex is a dead stroke of French"
    );
    assert_eq!(rendered(&strokes, &us()), "[e", "case 11");
    assert_eq!(
        OnScreen::as_typed(&strokes).characters(),
        1,
        "ê — one character on the screen, one erased"
    );

    // Decision 75 stands: a dead stroke that composed with nothing (`^t` on the screen) is still
    // counted as nothing — short by one, never one too many.
    let uncomposed = typed(&french, &keys_of("[t"));
    assert_eq!(OnScreen::as_typed(&uncomposed).characters(), 1);
}

/// **The second press erases what the first one really put on the screen** — `injected_chars`
/// over the composed text: `[etre` injected into French is `être`, four characters, not five.
#[test]
fn the_press_after_a_composition_erases_the_composed_text() {
    let strokes = typed(&us(), &keys_of("[etre"));

    assert_eq!(OnScreen::as_injected(&strokes, &french()).characters(), 4);
    assert_eq!(OnScreen::as_injected(&strokes, &french()).letters(), 4);
    assert_eq!(OnScreen::as_injected(&strokes, &us()).characters(), 5);
}

/// **The selection path takes the composed character back to the two keys it was typed with** —
/// решение 195.1 and 196.1: `ê` of French is the dead `^` and `e`, without any memory, by the
/// layout's own table; and the way into French composes as the typing path does.
#[test]
fn the_selection_decomposes_by_the_table_and_composes_on_the_way_in() {
    let us = us();
    let french = french();

    assert_eq!(selection::recode("ê", &french, &us).text(), "[e");
    assert_eq!(selection::recode("être", &french, &us).text(), "[etre");
    assert_eq!(selection::recode("[etre", &us, &french).text(), "être");
    assert_eq!(
        selection::recode("canción", &spanish(), &us).text(),
        "canci'on"
    );
    assert_eq!(selection::recode("não", &portuguese(), &us).text(), "n\\ao");
}

/// **The table is the layout's own, read off its file** — посылка П3 (197.11) as a test:
/// `KBDFR.DLL` of this machine composes `^` + `e` into `ê`, and the check of the sweep believes it
/// only when it covers the accents found. The file is read, not loaded as a layout: the session's
/// list of layouts is not touched. A name with a path in it is refused.
#[test]
fn the_table_of_a_layout_file_is_read_and_believed_only_when_it_matches() {
    let unit = |ch: char| u16::try_from(u32::from(ch)).expect("a BMP character");

    let french = layouts::read_dead_table("KBDFR.DLL");
    assert!(
        french.contains(&DeadTransition {
            accent: unit('^'),
            base: unit('e'),
            composed: unit('ê'),
        }),
        "KBDFR.DLL composes ^ + e into ê"
    );

    assert!(
        layouts::read_dead_table("KBDUS.DLL").is_empty(),
        "US has no dead key"
    );
    assert!(
        layouts::read_dead_table(r"..\KBDFR.DLL").is_empty(),
        "a path is refused"
    );

    assert!(!layouts::believed_against(french.clone(), &[unit('^')]).is_empty());
    assert!(
        layouts::believed_against(french, &[unit('^'), unit('\u{2603}')]).is_empty(),
        "an accent the file does not have: another layout's file"
    );
}
