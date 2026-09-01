//! Pure transcoding logic. Does not depend on Win32, fully covered by unit tests.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-22, FR-23, FR-24, FR-25, FR-26 — implemented here by
//! task T-02-2.
//! Moved out to match the backlog (decision R-17): FR-32 (cycle accuracy, the position
//! counter) to module `layouts`, task T-05-2. What FR-32 needs from *this* module is a
//! property rather than code, and it is stated below.
//! Implemented by backlog tasks: T-02-2 (done), T-02-1a (done).
//!
//! # What conversion is
//!
//! The user typed `ghbdtn` with the English layout active and pressed the hotkey. Every
//! keystroke was recorded by its **physical key** — scan code plus modifier combination —
//! together with the characters it produced at the time. Conversion asks one question per
//! stroke: *what does that same physical key produce in the target layout?* The answer is
//! `привет`.
//!
//! FR-22 forbids the obvious alternative, a character-to-character correspondence table. A
//! table has to be written per layout pair and silently produces nothing for a pair nobody
//! wrote it for, whereas the physical key is a coordinate every layout answers in. That is
//! why the stroke of FR-04 carries a scan code at all, and why nothing below ever looks at a
//! character to decide what to emit.
//!
//! The neighbouring temptation is [`LayoutMap::find_key`], the reverse index of module
//! `layouts`: with it one could write "character to key to character" and get a conversion
//! that looks equivalent. On the buffer path it is not. The stroke is already there, with
//! the key that produced it; going through the character throws that away and then guesses
//! it back, and the guess differs whenever two keys produce the same character. The reverse
//! index exists for the *selection* path of FR-61, where the strokes are genuinely gone and
//! only text remains — a different operation, and the business of task T-07-2.
//!
//! # Direction — FR-26
//!
//! Direction is not discovered here, it is given. [`convert_strokes`] takes the target
//! layout as a parameter, and the choice of that parameter belongs to task T-05-2 (modes
//! "pair" and "cycle", FR-30 to FR-34) working from the layout recorded with each stroke —
//! [`Keystroke::layout`] — and from the settings of section 4.4. FR-26 forbids deciding it
//! from the text: there is no script detection here, no "this looks Russian" heuristic, and
//! no place where the characters being converted influence anything but their own
//! replacement.
//!
//! # Shape of the result — FR-32, NFR-03
//!
//! FR-32 forbids replacing the buffer contents with converted text, because a buffer that
//! has been converted once is no longer a record of what the user typed and every further
//! pass compounds the distortion. Conversion is therefore a **pure function** of a stroke
//! sequence and a target layout: it takes `&[Keystroke]`, never `&mut`, and the same
//! strokes rendered into the same layout give the same units forever. Cycling through `N`
//! layouts and arriving back at the first one reproduces the original text bit for bit, and
//! it does so because the source was never touched, not because the round trip happens to
//! cancel out.
//!
//! NFR-03 sets the second half of the shape: the hotkey is handled on the input thread, so
//! conversion allocates nothing. The result is written into a buffer the caller owns
//! ([`convert_strokes`]), and a result that does not fit is an error the caller can act on
//! ([`ConvertError::BufferTooSmall`]) rather than a panic or a quiet truncation. A ligature
//! is written whole or not at all: half a ligature is not a shorter answer, it is a wrong
//! one.
//!
//! # Privacy
//!
//! SEC-01 and SEC-07: nothing here is written to a log, to a file or to a panic message.
//! The module has no `println!`, no formatting of decoded characters, and its only error
//! type reports a buffer size. Unit tests below do compare and print characters — they
//! contain no user input, only string constants written into the test itself.

use core::fmt;

use crate::layouts::{
    KeyMapping, LayoutCache, LayoutId, LayoutMap, LayoutMapBuilder, MAX_UNITS, Mods,
};

// ---------------------------------------------------------------------------------------
// One recorded keystroke
// ---------------------------------------------------------------------------------------

/// One recorded keystroke, in the coordinates FR-22 converts in.
///
/// This is the part of the stroke of FR-04 that conversion needs: the physical key — scan
/// code *and* extended flag — the modifier combination, the layout that was active when the
/// key went down, and what the key produced back then. The ring buffer of task T-03-2 owns
/// the full structure — virtual key, timestamp — and narrows it to this type on the way in;
/// nothing here needs the rest.
///
/// Plain `Copy` data. Building one and converting it allocates nothing (NFR-03).
///
/// Note that [`Mods`] carries only the three modifiers that change which character a key
/// produces. The extended flag of FR-05 is not among them, and it is not a modifier here
/// either: it is the ninth bit of the key itself, because the keypad repeats the scan codes
/// of the main block and the `E0` prefix is all that tells the two apart. Its source in the
/// product is `LLKHF_EXTENDED` in `KBDLLHOOKSTRUCT::flags`, read by the hook of tasks T-03-1
/// and T-03-2; the same flag is replayed by the injection of FR-40.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keystroke {
    layout: LayoutId,
    scan: u16,
    extended: bool,
    mods: Mods,
    produced: KeyMapping,
}

impl Keystroke {
    /// Assembles a stroke from its recorded parts.
    ///
    /// `produced` is what the key gave when it was pressed, as `ToUnicodeEx` reported it
    /// then — the `chars` and `len` fields of FR-04, and the dead key flag of FR-24.
    /// `extended` is the `LLKHF_EXTENDED` flag of FR-05.
    pub const fn new(
        layout: LayoutId,
        scan: u16,
        extended: bool,
        mods: Mods,
        produced: KeyMapping,
    ) -> Self {
        Self {
            layout,
            scan,
            extended,
            mods,
            produced,
        }
    }

    /// The stroke the hook would have recorded for the key `(scan, extended)` under `mods`
    /// while `map`'s layout was active.
    ///
    /// The decoding step of FR-06 is already done and cached by module `layouts`, so this
    /// reads the cache instead of asking the OS again, and works in a unit test with a
    /// synthetic layout exactly as it works on the live cache.
    pub fn recorded_in(map: &LayoutMap, scan: u16, extended: bool, mods: Mods) -> Self {
        Self {
            layout: map.layout(),
            scan,
            extended,
            mods,
            produced: map.lookup(scan, extended, mods),
        }
    }

    /// The layout that was active when the key went down — the `hkl` field of FR-04.
    ///
    /// The one input FR-26 allows the direction of conversion to be decided from. Reading it
    /// is the business of task T-05-2; this module only carries it.
    pub const fn layout(self) -> LayoutId {
        self.layout
    }

    /// Scan code of the physical key, low byte only.
    pub const fn scan(self) -> u16 {
        self.scan
    }

    /// Whether the key was an extended one — the `LLKHF_EXTENDED` flag of FR-05.
    ///
    /// The other half of the key: `false` names the `/?` key of the main block, `true` the
    /// `/` of the keypad, and both report the scan code `0x35`.
    pub const fn extended(self) -> bool {
        self.extended
    }

    /// Modifier combination held when the key went down.
    pub const fn mods(self) -> Mods {
        self.mods
    }

    /// What the key produced in the layout it was typed under.
    pub const fn produced(self) -> KeyMapping {
        self.produced
    }

    /// Whether the stroke was a dead key — FR-24.
    pub const fn is_dead(self) -> bool {
        self.produced.is_dead()
    }
}

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Why a conversion could not deliver its result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertError {
    /// The caller's buffer is shorter than the result — NFR-03.
    ///
    /// `needed` is the exact length of the whole result in UTF-16 code units, so the caller
    /// can size a buffer and call again rather than probe. Nothing is promised about what is
    /// left in the buffer after this error: the answer is `needed`, not the prefix that
    /// happened to fit, and a caller that reads the prefix is reading a truncation this type
    /// exists to prevent.
    BufferTooSmall {
        /// Length of the complete result, in UTF-16 code units.
        needed: usize,
    },
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // A count of code units, never the units themselves — SEC-07.
            Self::BufferTooSmall { needed } => {
                write!(f, "the result needs {needed} UTF-16 code units")
            }
        }
    }
}

impl std::error::Error for ConvertError {}

// ---------------------------------------------------------------------------------------
// Conversion — FR-22, FR-23, FR-24
// ---------------------------------------------------------------------------------------

/// What one stroke becomes in `target` — FR-22, FR-23, FR-24.
///
/// The whole engine, in four branches:
///
/// 1. a dead stroke is carried through unchanged (FR-24). Its own characters are the answer,
///    and the target layout is not even consulted: a dead key is a half-finished
///    composition, and re-deciding half a composition in another layout is not a
///    translation of it;
/// 2. otherwise the target layout is asked what the same physical key gives (FR-22);
/// 3. if it gives nothing, the stroke keeps its own characters and conversion goes on
///    (FR-23). Absence is an ordinary answer here, not a failure and not a reason to drop a
///    character;
/// 4. if the same key is a *dead* key in the target layout, the stroke also keeps its own
///    characters. Emitting the dead character as if it were a letter would start a
///    composition in the user's application instead of inserting text, so as far as this
///    stroke is concerned the target layout offers no character — case 3 again. Module
///    `layouts` keeps dead keys out of its reverse index for the same reason.
///
/// Returns a [`KeyMapping`] by value: ten bytes of `Copy` data, no allocation, no borrow of
/// either argument.
pub fn convert_stroke(stroke: Keystroke, target: &LayoutMap) -> KeyMapping {
    if stroke.produced.is_dead() {
        return stroke.produced;
    }
    let candidate = target.lookup(stroke.scan, stroke.extended, stroke.mods);
    if candidate.is_empty() || candidate.is_dead() {
        return stroke.produced;
    }
    candidate
}

/// Renders `strokes` into `target` and writes the result into `out` — FR-22, NFR-03.
///
/// Returns the number of UTF-16 code units written. `strokes` is borrowed immutably and is
/// still the original recording afterwards, which is what FR-32 rests on: rendering the same
/// strokes into the layout they were typed under reproduces the original text bit for bit,
/// however many other layouts were rendered in between.
///
/// Allocates nothing. Every stroke contributes at most [`MAX_UNITS`] code units, so
/// [`max_units`] gives a buffer size that can never be too small, and [`converted_len`]
/// gives the exact one.
///
/// A result that does not fit is [`ConvertError::BufferTooSmall`] carrying the length the
/// whole result would have had. Nothing is written for a stroke that does not fit whole, so
/// a ligature can never be cut in half.
pub fn convert_strokes(
    strokes: &[Keystroke],
    target: &LayoutMap,
    out: &mut [u16],
) -> Result<usize, ConvertError> {
    let mut needed = 0usize;
    let mut written = 0usize;

    for &stroke in strokes {
        let mapping = convert_stroke(stroke, target);
        let units = mapping.units();
        needed += units.len();
        // `needed` only grows, so this is false for every later stroke once one has
        // overflowed: `written` cannot fall behind and then be used as a start index.
        if needed <= out.len() {
            out[written..needed].copy_from_slice(units);
            written = needed;
        }
    }

    if needed > out.len() {
        return Err(ConvertError::BufferTooSmall { needed });
    }
    Ok(written)
}

/// Length of the result of [`convert_strokes`], in UTF-16 code units.
///
/// The same walk without the writing, for a caller that wants to size a buffer exactly.
/// Allocates nothing.
pub fn converted_len(strokes: &[Keystroke], target: &LayoutMap) -> usize {
    strokes
        .iter()
        .map(|&stroke| convert_stroke(stroke, target).units().len())
        .sum()
}

/// Buffer length that fits the conversion of `strokes` strokes whatever the target layout.
///
/// `const`, so a caller can size a fixed array with it: the ring buffer of FR-07 holds 256
/// strokes, and `max_units(256)` is the buffer that always fits their conversion.
pub const fn max_units(strokes: usize) -> usize {
    strokes * MAX_UNITS
}

// ---------------------------------------------------------------------------------------
// The hardwired fallback — FR-25
// ---------------------------------------------------------------------------------------

/// US, `00000409` — the identifier the fallback table of FR-25 files its English half under.
pub const FALLBACK_US: LayoutId = LayoutId::from_raw(0x0409_0409);

/// Russian, `00000419` — the identifier the fallback table files its Russian half under.
pub const FALLBACK_RUSSIAN: LayoutId = LayoutId::from_raw(0x0419_0419);

/// `Shift` and `CapsLock` together, the fourth combination the fallback table fills.
const SHIFT_CAPS: Mods = Mods::new(true, true, false);

/// The extended flag of FR-05 for a key of the main block: never set.
///
/// Every row of [`FALLBACK_KEYS`] is a main block key, so the flag is a constant here. It is
/// named rather than written as a bare `false` at the call sites of `set` below, where a
/// stray boolean would say nothing about which key it selects.
const MAIN_BLOCK: bool = false;

/// One row of [`FALLBACK_KEYS`]: the scan code of a physical key, then what that key produces
/// in US and in Russian as `[unshifted, with Shift]`.
type FallbackRow = (u16, [char; 2], [char; 2]);

/// The hardwired RU/EN correspondence of FR-25, by physical key.
///
/// Every row is one key of the main block of a set 1 keyboard, in the order the keys sit on
/// it: the number row, then the three letter rows, then the space bar under them. The two
/// pairs are the two readings of that one key, so the table is a correspondence between
/// *keys*, not between characters — FR-22 holds for the fallback exactly as it holds for the
/// live cache, and adding a third layout would mean adding a column, not writing a new table.
///
/// Coverage matches what section 11.1 of SPEC asks conversion to be tested on: every letter
/// of both alphabets in both cases, the digit row with and without `Shift`, the punctuation
/// keys, and `ё`/`~`.
///
/// The numeric keypad is deliberately absent, and since task T-02-1a that is a choice rather
/// than a constraint: the extended flag of FR-05 is now part of the key, so a keypad row
/// would land in its own slot instead of colliding with the main block key of the same scan
/// code. It stays absent because FR-25 is an emergency reserve for typing RU/EN text, and
/// the keypad reads alike in both layouts — a row for it would add nothing a conversion
/// could use. Rule 3 of [`convert_stroke`] covers those keys: absent from the target, the
/// stroke keeps its own character.
///
/// ⭐ **The space bar is the one key that argument does not cover — task Т-25-1, finding
/// м-Э24-1, the user's decision 84.1.** It too reads alike in both layouts, so as long as a
/// row was worth having only for what it converts, it was rightly absent. Task Т-20-1 changed
/// the terms: FR-41 counts the display units of the recorded strokes to decide how many
/// `Backspace` a replacement sends, and task Т-24-2 put the space bar into the ring as the
/// soft boundary of FR-10. A key that carries no character is not counted, so on the fallback
/// path «ghbdtn » came out six units wide instead of seven and the tail survived the erase. A
/// row is needed here not because it converts anything but because it **counts**.
const FALLBACK_KEYS: [FallbackRow; 48] = [
    (0x29, ['`', '~'], ['ё', 'Ё']),
    (0x02, ['1', '!'], ['1', '!']),
    (0x03, ['2', '@'], ['2', '"']),
    (0x04, ['3', '#'], ['3', '№']),
    (0x05, ['4', '$'], ['4', ';']),
    (0x06, ['5', '%'], ['5', '%']),
    (0x07, ['6', '^'], ['6', ':']),
    (0x08, ['7', '&'], ['7', '?']),
    (0x09, ['8', '*'], ['8', '*']),
    (0x0A, ['9', '('], ['9', '(']),
    (0x0B, ['0', ')'], ['0', ')']),
    (0x0C, ['-', '_'], ['-', '_']),
    (0x0D, ['=', '+'], ['=', '+']),
    (0x10, ['q', 'Q'], ['й', 'Й']),
    (0x11, ['w', 'W'], ['ц', 'Ц']),
    (0x12, ['e', 'E'], ['у', 'У']),
    (0x13, ['r', 'R'], ['к', 'К']),
    (0x14, ['t', 'T'], ['е', 'Е']),
    (0x15, ['y', 'Y'], ['н', 'Н']),
    (0x16, ['u', 'U'], ['г', 'Г']),
    (0x17, ['i', 'I'], ['ш', 'Ш']),
    (0x18, ['o', 'O'], ['щ', 'Щ']),
    (0x19, ['p', 'P'], ['з', 'З']),
    (0x1A, ['[', '{'], ['х', 'Х']),
    (0x1B, [']', '}'], ['ъ', 'Ъ']),
    (0x2B, ['\\', '|'], ['\\', '/']),
    (0x1E, ['a', 'A'], ['ф', 'Ф']),
    (0x1F, ['s', 'S'], ['ы', 'Ы']),
    (0x20, ['d', 'D'], ['в', 'В']),
    (0x21, ['f', 'F'], ['а', 'А']),
    (0x22, ['g', 'G'], ['п', 'П']),
    (0x23, ['h', 'H'], ['р', 'Р']),
    (0x24, ['j', 'J'], ['о', 'О']),
    (0x25, ['k', 'K'], ['л', 'Л']),
    (0x26, ['l', 'L'], ['д', 'Д']),
    (0x27, [';', ':'], ['ж', 'Ж']),
    (0x28, ['\'', '"'], ['э', 'Э']),
    (0x2C, ['z', 'Z'], ['я', 'Я']),
    (0x2D, ['x', 'X'], ['ч', 'Ч']),
    (0x2E, ['c', 'C'], ['с', 'С']),
    (0x2F, ['v', 'V'], ['м', 'М']),
    (0x30, ['b', 'B'], ['и', 'И']),
    (0x31, ['n', 'N'], ['т', 'Т']),
    (0x32, ['m', 'M'], ['ь', 'Ь']),
    (0x33, [',', '<'], ['б', 'Б']),
    (0x34, ['.', '>'], ['ю', 'Ю']),
    (0x35, ['/', '?'], ['.', ',']),
    // The space bar, under the three letter rows and last for that reason. Both readings are
    // the same character in both layouts, which is exactly what the live cache of FR-20
    // reports for it — measured on this machine by
    // `tests\layouts.rs::the_live_cache_of_fr20_carries_the_space_bar`, `Shift` included.
    (0x39, [' ', ' '], [' ', ' ']),
];

/// Builds one half of the fallback table.
///
/// `russian` picks the column; the identifier is passed in so that the map answers to the
/// same [`LayoutId`] the live cache would have used, and a caller that fell back cannot tell
/// the difference by looking at the map.
fn build_fallback(layout: LayoutId, russian: bool) -> LayoutMap {
    let mut builder = LayoutMapBuilder::new(layout);
    for &(scan, us, ru) in &FALLBACK_KEYS {
        let [plain, shifted] = if russian { ru } else { us };

        // `CapsLock` swaps the two readings of a letter key and leaves every other key
        // alone. That is the whole of its effect in both of these layouts, which is why the
        // table stores two readings per key and derives four combinations from them rather
        // than listing four.
        let (caps, shift_caps) = if plain.is_alphabetic() {
            (shifted, plain)
        } else {
            (plain, shifted)
        };

        builder.set(scan, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char(plain));
        builder.set(
            scan,
            MAIN_BLOCK,
            Mods::SHIFT,
            KeyMapping::from_char(shifted),
        );
        builder.set(scan, MAIN_BLOCK, Mods::CAPS, KeyMapping::from_char(caps));
        builder.set(
            scan,
            MAIN_BLOCK,
            SHIFT_CAPS,
            KeyMapping::from_char(shift_caps),
        );
    }
    builder.finish()
}

/// The hardwired map of one of the two layouts of FR-25, or `None` for any other layout.
///
/// `None` is the honest answer for a third layout: the fallback is a last resort for the
/// RU/EN pair, not a claim to know layouts the OS refused to describe.
pub fn fallback_map(layout: LayoutId) -> Option<LayoutMap> {
    match layout {
        FALLBACK_RUSSIAN => Some(build_fallback(layout, true)),
        FALLBACK_US => Some(build_fallback(layout, false)),
        _ => None,
    }
}

/// The hardwired RU/EN cache of FR-25, for use when [`LayoutCache::build`] failed.
///
/// FR-25 makes this an emergency reserve and nothing else: it is reached only on the `Err` of
/// a build or a rebuild, never as a shortcut, because a hardwired table cannot know which
/// layouts the user actually has. Module `layouts` makes that failure distinguishable — a
/// built cache is never empty — so there is no state in which a caller has to guess whether
/// to use this.
///
/// The two maps come in the order of the system layout list this program was developed
/// against (section 4 of TOOLCHAIN.md): Russian first, US second. Building them allocates,
/// like every other cache construction; it happens once, on the start-up path, never on the
/// hook path.
pub fn fallback_cache() -> LayoutCache {
    let maps = vec![
        build_fallback(FALLBACK_RUSSIAN, true),
        build_fallback(FALLBACK_US, false),
    ];
    // `from_maps` rejects exactly two shapes, no maps at all and no characters in any of
    // them. Both maps above come from a constant table of 48 keys, so neither shape is
    // reachable; `the_fallback_cache_carries_both_layouts` pins that.
    LayoutCache::from_maps(maps).expect("the hardwired FR-25 table is never empty")
}

// ---------------------------------------------------------------------------------------
// Tests — section 11.1 of SPEC
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::MappingKind;

    /// The 48 keys of the fallback table read off an English keyboard, unshifted.
    ///
    /// Written out here as text rather than derived from [`FALLBACK_KEYS`] on purpose: a test
    /// that builds its expectation from the table it is testing proves only that the table
    /// equals itself. These four constants are the independent statement of what the RU/EN
    /// correspondence is, and each pair is aligned character by character — the *n*-th
    /// character of one is what the *n*-th of the other becomes.
    ///
    /// ⚠ **The last character of all four is a space, and it is the space bar** — task Т-25-1.
    /// It is the one character here that cannot be seen, so it is named: the four strings were
    /// extended by hand along with the table, exactly as the paragraph above requires, and none
    /// of them is one character shorter than it looks.
    const EN_LOWER: &str = "`1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./ ";
    /// The same 48 keys in the Russian layout, unshifted.
    const RU_LOWER: &str = "ё1234567890-=йцукенгшщзхъ\\фывапролджэячсмитьбю. ";
    /// The same 48 keys in the English layout, with `Shift`.
    const EN_UPPER: &str = "~!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>? ";
    /// The same 48 keys in the Russian layout, with `Shift`.
    const RU_UPPER: &str = "Ё!\"№;%:?*()_+ЙЦУКЕНГШЩЗХЪ/ФЫВАПРОЛДЖЭЯЧСМИТЬБЮ, ";

    /// Scan codes for the synthetic layouts below. Arbitrary but stable.
    const SCANS: [u16; 3] = [0x10, 0x11, 0x12];

    /// The extended flag of FR-05 for a key of the keypad — the counterpart of
    /// [`MAIN_BLOCK`].
    const KEYPAD: bool = true;

    /// The English half of the fallback table.
    fn us() -> LayoutMap {
        fallback_map(FALLBACK_US).expect("the fallback table must carry US")
    }

    /// The Russian half of the fallback table.
    fn ru() -> LayoutMap {
        fallback_map(FALLBACK_RUSSIAN).expect("the fallback table must carry Russian")
    }

    /// The strokes the hook would have recorded had the user typed `text` under `map`.
    ///
    /// This is the *typing*, not the conversion: in the product the scan codes come from the
    /// hook (task T-03-2) and there is no character to start from. A unit test has no
    /// keyboard, so the key that carries each character is found through the reverse index of
    /// module `layouts`. Nothing under test calls `find_key`, and FR-22 is about the
    /// conversion, not about how a test types.
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

    /// The exact UTF-16 units of a conversion, for the bit-for-bit comparison of FR-32.
    fn units(strokes: &[Keystroke], target: &LayoutMap) -> Vec<u16> {
        let mut out = [0u16; max_units(64)];
        let written =
            convert_strokes(strokes, target, &mut out).expect("the result must fit the buffer");
        out[..written].to_vec()
    }

    /// The units the strokes carry as they were recorded — the original text.
    fn recorded_units(strokes: &[Keystroke]) -> Vec<u16> {
        strokes
            .iter()
            .flat_map(|stroke| stroke.produced().units().to_vec())
            .collect()
    }

    /// A synthetic layout in which the three keys of [`SCANS`] carry `alphabet`.
    fn synthetic(raw: usize, alphabet: &str) -> LayoutMap {
        let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(raw));
        for (scan, ch) in SCANS.iter().zip(alphabet.chars()) {
            builder.set(*scan, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char(ch));
        }
        builder.finish()
    }

    // -- section 11.1, position 1 ---------------------------------------------------------

    #[test]
    fn a_full_ru_en_traversal_converts_every_key_in_both_cases() {
        let (us, ru) = (us(), ru());

        // Every letter of both alphabets, the digit row with and without Shift, the
        // punctuation keys, `ё`/`~` and the space bar — 48 keys times two readings, both
        // directions.
        assert_eq!(render(&type_text(&us, EN_LOWER), &ru), RU_LOWER);
        assert_eq!(render(&type_text(&ru, RU_LOWER), &us), EN_LOWER);
        assert_eq!(render(&type_text(&us, EN_UPPER), &ru), RU_UPPER);
        assert_eq!(render(&type_text(&ru, RU_UPPER), &us), EN_UPPER);

        // The two constants really are aligned key by key, so the assertions above are the
        // claim they look like and not an accident of two strings of different lengths.
        assert_eq!(EN_LOWER.chars().count(), FALLBACK_KEYS.len());
        assert_eq!(RU_LOWER.chars().count(), FALLBACK_KEYS.len());
        assert_eq!(EN_UPPER.chars().count(), FALLBACK_KEYS.len());
        assert_eq!(RU_UPPER.chars().count(), FALLBACK_KEYS.len());
    }

    // -- section 11.1, position 2 ---------------------------------------------------------

    #[test]
    fn the_same_physical_key_gives_at_in_us_and_a_quote_in_russian() {
        // The case section 11.1 names as the proof that conversion follows the key: `Shift+2`
        // carries two characters with nothing in common, so no character table could relate
        // them and a wrong implementation cannot pass by accident.
        let (us, ru) = (us(), ru());
        let digit_two = us.find_key('2').expect("US must carry '2'");
        let shifted = Keystroke::recorded_in(&us, digit_two.scan, MAIN_BLOCK, Mods::SHIFT);

        assert_eq!(shifted.produced().single_char(), Some('@'));
        assert_eq!(convert_stroke(shifted, &ru).single_char(), Some('"'));

        // And back, from the same physical key.
        let typed_in_russian = Keystroke::recorded_in(&ru, digit_two.scan, MAIN_BLOCK, Mods::SHIFT);
        assert_eq!(typed_in_russian.produced().single_char(), Some('"'));
        assert_eq!(
            convert_stroke(typed_in_russian, &us).single_char(),
            Some('@')
        );
    }

    // -- section 11.1, position 3, FR-23 --------------------------------------------------

    #[test]
    fn a_key_absent_from_the_target_keeps_its_character_and_conversion_goes_on() {
        // The source carries three keys, the target only the outer two. FR-23: the middle
        // stroke keeps its own character, and — the half that is easy to get wrong — the
        // strokes after it are still converted.
        let source = synthetic(0xF001_0409, "abc");
        let mut sparse = LayoutMapBuilder::new(LayoutId::from_raw(0xF002_0409));
        sparse.set(SCANS[0], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('х'));
        sparse.set(SCANS[2], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('ц'));
        let target = sparse.finish();

        let strokes: Vec<Keystroke> = SCANS
            .iter()
            .map(|&scan| Keystroke::recorded_in(&source, scan, MAIN_BLOCK, Mods::NONE))
            .collect();

        assert!(target.lookup(SCANS[1], MAIN_BLOCK, Mods::NONE).is_empty());
        assert_eq!(render(&strokes, &target), "хbц");

        // Not an error, not a dropped character, not a shorter result.
        assert_eq!(converted_len(&strokes, &target), strokes.len());
    }

    // -- section 11.1, position 4, FR-32 --------------------------------------------------

    #[test]
    fn a_full_cycle_through_three_layouts_restores_the_original_bit_for_bit() {
        let first = synthetic(0xF001_0409, "abc");
        let second = synthetic(0xF002_0419, "фыв");
        let third = synthetic(0xF003_0407, "χψω");

        let strokes: Vec<Keystroke> = SCANS
            .iter()
            .map(|&scan| Keystroke::recorded_in(&first, scan, MAIN_BLOCK, Mods::NONE))
            .collect();
        let original_strokes = strokes.clone();
        let original_units = recorded_units(&strokes);

        // One turn of the cycle of FR-31: three layouts, three hotkey presses, each rendering
        // the same untouched strokes into the next layout.
        assert_eq!(render(&strokes, &second), "фыв");
        assert_eq!(render(&strokes, &third), "χψω");
        assert_eq!(units(&strokes, &first), original_units);

        // FR-32 the other way round: the source is what makes the round trip exact, so it has
        // to come out of three conversions exactly as it went in.
        assert_eq!(strokes, original_strokes);

        // And the cycle is not exact merely because the layouts are the same: the three
        // renderings differ from each other.
        assert_ne!(units(&strokes, &second), units(&strokes, &third));
        assert_ne!(units(&strokes, &second), original_units);

        // Ten turns, because the hotkey can be pressed all day and FR-32 forbids drift.
        for _ in 0..10 {
            for target in [&second, &third, &first] {
                let _ = units(&strokes, target);
            }
            assert_eq!(units(&strokes, &first), original_units);
        }
    }

    // -- section 11.1, position 5 ---------------------------------------------------------

    #[test]
    fn a_ligature_is_carried_whole() {
        let source = synthetic(0xF001_0409, "ab");
        let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0xF002_0409));
        // Two code units that are two characters, and two code units that are one: both are
        // longer than a single unit, which is what section 11.1 means by a ligature.
        builder.set(
            SCANS[0],
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::from_to_unicode(2, &[0x0066, 0x0069]),
        );
        builder.set(
            SCANS[1],
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::from_char('\u{1F600}'),
        );
        let target = builder.finish();

        let strokes: Vec<Keystroke> = SCANS[..2]
            .iter()
            .map(|&scan| Keystroke::recorded_in(&source, scan, MAIN_BLOCK, Mods::NONE))
            .collect();

        assert_eq!(
            target.lookup(SCANS[0], MAIN_BLOCK, Mods::NONE).kind(),
            MappingKind::Ligature
        );
        assert_eq!(converted_len(&strokes, &target), 4);
        assert_eq!(units(&strokes, &target), [0x0066, 0x0069, 0xD83D, 0xDE00]);
        assert_eq!(render(&strokes, &target), "fi\u{1F600}");

        // Two strokes can never need more than `max_units(2)`, which is what lets a caller
        // size a fixed buffer without knowing the layouts.
        assert!(converted_len(&strokes, &target) <= max_units(strokes.len()));
    }

    // -- section 11.1, position 6, FR-24 --------------------------------------------------

    #[test]
    fn a_dead_stroke_is_carried_through_unchanged() {
        let mut source = LayoutMapBuilder::new(LayoutId::from_raw(0xF001_0409));
        source.set(
            SCANS[0],
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::dead('\u{0300}'),
        );
        source.set(SCANS[1], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('a'));
        let source = source.finish();

        // The target has an ordinary character on both keys, so a conversion that ignored the
        // dead flag would visibly replace it.
        let target = synthetic(0xF002_0419, "фы");

        let dead = Keystroke::recorded_in(&source, SCANS[0], MAIN_BLOCK, Mods::NONE);
        assert!(dead.is_dead());
        let converted = convert_stroke(dead, &target);
        assert_eq!(converted, dead.produced());
        assert_eq!(converted.kind(), MappingKind::Dead);
        assert_eq!(converted.units(), [0x0300]);

        // The stroke next to it is converted normally: FR-24 is about one stroke, not about
        // giving up on the sequence.
        let strokes = [
            dead,
            Keystroke::recorded_in(&source, SCANS[1], MAIN_BLOCK, Mods::NONE),
        ];
        assert_eq!(render(&strokes, &target), "\u{0300}ы");

        // The mirror case: an ordinary stroke whose key is a dead key in the target. Emitting
        // the dead character would start a composition in the user's application, so the
        // target offers this stroke no character and FR-23 applies.
        let mut dead_target = LayoutMapBuilder::new(LayoutId::from_raw(0xF003_0409));
        dead_target.set(
            SCANS[1],
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::dead('\u{0301}'),
        );
        let dead_target = dead_target.finish();
        let ordinary = Keystroke::recorded_in(&source, SCANS[1], MAIN_BLOCK, Mods::NONE);
        assert_eq!(convert_stroke(ordinary, &dead_target), ordinary.produced());
    }

    // -- beyond section 11.1, position 7 --------------------------------------------------

    #[test]
    fn ghbdtn_becomes_privet() {
        // The scenario of the acceptance matrix, section 11.3 of SPEC, on the hardwired table.
        assert_eq!(render(&type_text(&us(), "ghbdtn"), &ru()), "привет");
    }

    // -- beyond section 11.1, position 8 --------------------------------------------------

    #[test]
    fn privet_becomes_ghbdtn() {
        assert_eq!(render(&type_text(&ru(), "привет"), &us()), "ghbdtn");
    }

    // -- beyond section 11.1, position 10, NFR-03 -----------------------------------------

    #[test]
    fn a_result_that_does_not_fit_is_an_error_and_not_a_panic() {
        let source = synthetic(0xF001_0409, "ab");
        let mut builder = LayoutMapBuilder::new(LayoutId::from_raw(0xF002_0409));
        builder.set(SCANS[0], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('x'));
        builder.set(
            SCANS[1],
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::from_to_unicode(2, &[0x0066, 0x0069]),
        );
        let target = builder.finish();

        let strokes: Vec<Keystroke> = SCANS[..2]
            .iter()
            .map(|&scan| Keystroke::recorded_in(&source, scan, MAIN_BLOCK, Mods::NONE))
            .collect();

        // Exactly enough: three units, no error.
        let mut exact = [0u16; 3];
        assert_eq!(convert_strokes(&strokes, &target, &mut exact), Ok(3));

        // One unit short. The error names the length the caller has to provide, and the
        // ligature that did not fit was not written in part: the second unit of the buffer is
        // still the zero it was initialised to.
        let mut short = [0u16; 2];
        assert_eq!(
            convert_strokes(&strokes, &target, &mut short),
            Err(ConvertError::BufferTooSmall { needed: 3 })
        );
        assert_eq!(short[1], 0);

        // No room at all is the same ordinary error, and an empty conversion into an empty
        // buffer is not an error at all.
        let mut none = [];
        assert_eq!(
            convert_strokes(&strokes, &target, &mut none),
            Err(ConvertError::BufferTooSmall { needed: 3 })
        );
        assert_eq!(convert_strokes(&[], &target, &mut none), Ok(0));
    }

    // -- beyond the required set ----------------------------------------------------------

    #[test]
    fn the_fallback_cache_carries_both_layouts_and_converts_between_them() {
        let cache = fallback_cache();
        assert_eq!(cache.len(), 2);
        let ru = cache
            .get(FALLBACK_RUSSIAN)
            .expect("the fallback cache must carry Russian");
        let us = cache
            .get(FALLBACK_US)
            .expect("the fallback cache must carry US");
        assert!(!ru.is_empty() && !us.is_empty());

        // Four modifier combinations on 48 keys.
        assert_eq!(ru.len(), FALLBACK_KEYS.len() * 4);
        assert_eq!(us.len(), FALLBACK_KEYS.len() * 4);

        assert_eq!(render(&type_text(us, "ghbdtn"), ru), "привет");

        // Any other layout gets an honest `None` rather than a table invented for it.
        assert!(fallback_map(LayoutId::from_raw(0x0407_0407)).is_none());
    }

    #[test]
    fn caps_lock_swaps_the_two_readings_of_a_letter_key_and_leaves_the_others_alone() {
        let (us, ru) = (us(), ru());
        let letter = us.find_key('a').expect("US must carry 'a'").scan;
        let digit = us.find_key('2').expect("US must carry '2'").scan;

        for (mods, expected_en, expected_ru) in [
            (Mods::NONE, 'a', 'ф'),
            (Mods::SHIFT, 'A', 'Ф'),
            (Mods::CAPS, 'A', 'Ф'),
            (SHIFT_CAPS, 'a', 'ф'),
        ] {
            let stroke = Keystroke::recorded_in(&us, letter, MAIN_BLOCK, mods);
            assert_eq!(stroke.produced().single_char(), Some(expected_en));
            assert_eq!(convert_stroke(stroke, &ru).single_char(), Some(expected_ru));
        }

        for (mods, expected_en, expected_ru) in [
            (Mods::NONE, '2', '2'),
            (Mods::SHIFT, '@', '"'),
            (Mods::CAPS, '2', '2'),
            (SHIFT_CAPS, '@', '"'),
        ] {
            let stroke = Keystroke::recorded_in(&us, digit, MAIN_BLOCK, mods);
            assert_eq!(stroke.produced().single_char(), Some(expected_en));
            assert_eq!(convert_stroke(stroke, &ru).single_char(), Some(expected_ru));
        }
    }

    #[test]
    fn conversion_reads_the_key_and_never_the_character() {
        // Two physical keys carrying the same character in the source layout, and different
        // characters in the target. An implementation that went through the character —
        // find_key of the character, then lookup — would give the same answer for both, since
        // find_key has only one answer per character. Going through the key gives two.
        let mut source = LayoutMapBuilder::new(LayoutId::from_raw(0xF001_0409));
        source.set(SCANS[0], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('a'));
        source.set(SCANS[1], MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('a'));
        let source = source.finish();
        let target = synthetic(0xF002_0419, "фы");

        let first = Keystroke::recorded_in(&source, SCANS[0], MAIN_BLOCK, Mods::NONE);
        let second = Keystroke::recorded_in(&source, SCANS[1], MAIN_BLOCK, Mods::NONE);
        assert_eq!(first.produced(), second.produced());
        assert_eq!(convert_stroke(first, &target).single_char(), Some('ф'));
        assert_eq!(convert_stroke(second, &target).single_char(), Some('ы'));
    }

    #[test]
    fn the_extended_flag_is_part_of_the_key_and_not_a_modifier() {
        // The defect of task T-02-1a in miniature. Two keys share the scan code `0x35` and
        // are told apart only by the extended flag of FR-05; in the source they read alike,
        // in the target they do not. A cache keyed by the scan code alone would let one of
        // them take the slot of the other and both strokes would convert to the same
        // character.
        const SHARED: u16 = 0x35;

        let mut source = LayoutMapBuilder::new(LayoutId::from_raw(0xF001_0409));
        source.set(SHARED, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('/'));
        source.set(SHARED, KEYPAD, Mods::NONE, KeyMapping::from_char('/'));
        let source = source.finish();

        let mut target = LayoutMapBuilder::new(LayoutId::from_raw(0xF002_0419));
        target.set(SHARED, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char('.'));
        target.set(SHARED, KEYPAD, Mods::NONE, KeyMapping::from_char('/'));
        let target = target.finish();

        // Both keys really are in the map, and both really are filled.
        assert_eq!(source.len(), 2);
        assert_eq!(target.len(), 2);

        let main = Keystroke::recorded_in(&source, SHARED, MAIN_BLOCK, Mods::NONE);
        let keypad = Keystroke::recorded_in(&source, SHARED, KEYPAD, Mods::NONE);
        assert!(!main.extended());
        assert!(keypad.extended());
        assert_eq!(main.scan(), keypad.scan());
        assert_eq!(main.produced(), keypad.produced());
        assert_ne!(main, keypad);

        assert_eq!(convert_stroke(main, &target).single_char(), Some('.'));
        assert_eq!(convert_stroke(keypad, &target).single_char(), Some('/'));

        // The reverse index prefers the main block, which is the key a user pressed and the
        // key task T-07-2 replays.
        let press = target.find_key('.').expect("the target must carry '.'");
        assert_eq!(press.scan, SHARED);
        assert!(!press.extended);
    }

    #[test]
    fn converted_len_matches_what_conversion_writes() {
        let (us, ru) = (us(), ru());
        for text in [EN_LOWER, EN_UPPER] {
            let strokes = type_text(&us, text);
            let mut out = [0u16; max_units(64)];
            let written = convert_strokes(&strokes, &ru, &mut out).expect("must fit");
            assert_eq!(written, converted_len(&strokes, &ru));
            assert!(written <= max_units(strokes.len()));
        }
    }

    #[test]
    fn the_error_message_carries_a_count_and_no_characters() {
        // SEC-07: the only text this module can ever produce is this one, and it is a number.
        let error = ConvertError::BufferTooSmall { needed: 7 };
        assert_eq!(error.to_string(), "the result needs 7 UTF-16 code units");
    }
}
