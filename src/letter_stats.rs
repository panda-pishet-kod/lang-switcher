//! **Statistics of the letters of a language** — решение 196.1 (2), stage Э105; the languages
//! besides English — решение 198.7, task T-106-5, stage Э106.
//!
//! Text selected in a layout with an ambiguous ligature (Arabic 101: `لا` is the key `B`, and it
//! is `G` then `H`) can be read two ways, and when the short memory of the typing does not have
//! it, the reading is chosen by how probable each is **in the language of the target layout**:
//! `لاث` is `be` and not `ghe`, `مهلاف` is `light` and not `libt`. The probability is a model of
//! letter trigrams — the chance of a letter after the two before it, word boundaries included —
//! counted once from a corpus of the language with a clear licence (the origin and the count are
//! in the reports of the stages) and carried in the image as a table of costs.
//!
//! # The alphabets
//!
//! Every language has its own: `res\letter-stats-alphabets.txt`, one line per language, read here
//! and by `tools\make-letter-stats.ps1` alike — the generator counts by the very classes the
//! program reads a candidate by, so the two cannot part. A class is one symbol: the lower-case
//! letters of it, a letter with a sign above or below folded into its base letter (ТЗ Э106 п. 8 —
//! a table of 64 KB at most); symbol 0 is the boundary, anything outside every class.
//!
//! # The table
//!
//! `S × S × S` bytes for `S` symbols (the boundary and the classes), and the byte at
//! `(a × S + b) × S + c` is the cost of `c` after `a b` — the negative binary logarithm of its
//! probability, in quarters of a bit, capped at 255. Lower is more probable. English and French
//! are 27 symbols (19 683 bytes), Spanish 28 (21 952), Russian 33 (35 937); since stage Э107
//! Arabic 31 (29 791), Persian 34 (39 304), Indonesian 27 (19 683), Hindi 40 (64 000), Tamil 37
//! (50 653) and Sinhala 36 (46 656) — under the 64 KB of one table and the 1 MB of all of them
//! (ТЗ Э106 п. 8): 347 342 bytes together.
//!
//! # The marks read as nothing
//!
//! A token `~…` of an alphabet lists the marks that are neither a letter nor a boundary — the
//! joiners of Sinhala, Hindi and Persian, the vowel marks and the tatweel of Arabic, the nukta of
//! Devanagari (task T-107-5). The generator and [`Reading`] both skip them, so a word written with
//! a joiner is the same word as without it, and no word is cut in two at one.
//!
//! # What it reads, and what it keeps
//!
//! A [`Reading`] is fed the code units of a candidate as the conversion writes them, and keeps two
//! symbols and two sums — no copy of the text is made (SEC-01). It runs on the UI thread in the
//! selection path, never in the hook.

use std::sync::OnceLock;

use crate::layouts::LayoutId;

/// The symbols of the English model: the boundary and the 26 letters.
const SYMBOLS: usize = 27;

/// The length of a table of 27 symbols — English, and every table [`Trigrams::from_costs`] makes.
pub const TABLE_LEN: usize = SYMBOLS * SYMBOLS * SYMBOLS;

/// The alphabets of every language that carries a table — see the module documentation.
const ALPHABETS: &str = include_str!("../res/letter-stats-alphabets.txt");

/// **The tables the image carries**, by the tag of their line in [`ALPHABETS`] — made by
/// `tools\make-letter-stats.ps1` out of books of Project Gutenberg in the public domain of the USA:
///
/// - `en` — stage Э105, решение 197.10а: «Pride and Prejudice», «The Adventures of Sherlock
///   Holmes», «A Tale of Two Cities», «Frankenstein», «Alice's Adventures in Wonderland»; 475 673
///   words, 2 020 716 letters;
/// - `fr`, `es`, `ru` — stage Э106, решения 198.3, 198.7: five books of French, five of Spanish,
///   three of Russian (all Gutenberg has in modern spelling); the books, their sizes and SHA-256
///   are in `reports\ITOG-E106.md`.
///
/// - `ar`, `fa`, `id`, `hi`, `ta`, `si` — stage Э107, task T-107-5, решения 198.22а и 198.22г: the
///   six target languages Gutenberg had no corpus of, out of the Leipzig Corpora Collection
///   (Universität Leipzig, CC BY; the Sinhala corpus is of Wikipedia, CC BY-SA) —
///   `ara_news_2005-2009_300K`, `fas_news_2019_300K`, `ind_mixed_2013_300K`, `hin_mixed_2019_300K`,
///   `tam_newscrawl_2011_300K`, `sin_wikipedia_2021_100K`; their sizes, SHA-256 and counts are in
///   `reports\ITOG-E107.md`.
///
/// The books and the corpora are not in the tree and not in the image — the tables are.
const TABLES: [(&str, &[u8]); 10] = [
    ("en", include_bytes!("../res/letter-stats-en.bin")),
    ("fr", include_bytes!("../res/letter-stats-fr.bin")),
    ("es", include_bytes!("../res/letter-stats-es.bin")),
    ("ru", include_bytes!("../res/letter-stats-ru.bin")),
    ("ar", include_bytes!("../res/letter-stats-ar.bin")),
    ("fa", include_bytes!("../res/letter-stats-fa.bin")),
    ("id", include_bytes!("../res/letter-stats-id.bin")),
    ("hi", include_bytes!("../res/letter-stats-hi.bin")),
    ("ta", include_bytes!("../res/letter-stats-ta.bin")),
    ("si", include_bytes!("../res/letter-stats-si.bin")),
];

/// **The letters of one language** — one line of [`ALPHABETS`].
#[derive(Debug)]
pub struct Alphabet {
    tag: &'static str,
    primary: usize,
    symbols: usize,
    /// Every lower-case letter and its symbol, sorted by the letter.
    letters: Vec<(char, u8)>,
    /// The marks that are **neither a letter nor a boundary** — the token `~…` of the line, task
    /// T-107-5: the joiners of Sinhala, Hindi and Persian (U+200C, U+200D), the vowel marks and the
    /// tatweel of Arabic, the nukta of Devanagari. Read as nothing at all: the word goes on across
    /// them. Sorted.
    skip: Vec<char>,
}

impl Alphabet {
    /// Whether `ch` is one of the marks this alphabet reads as nothing — see [`Alphabet::skip`].
    fn skips(&self, ch: char) -> bool {
        self.skip.binary_search(&ch).is_ok()
    }

    /// The symbol of `ch`: its class, after lower-casing, or 0 — the boundary.
    fn symbol(&self, ch: char) -> usize {
        let mut lower = ch.to_lowercase();
        let ch = match (lower.next(), lower.next()) {
            (Some(single), None) => single,
            _ => ch,
        };

        self.letters
            .binary_search_by_key(&ch, |&(letter, _)| letter)
            .map_or(0, |at| usize::from(self.letters[at].1))
    }
}

/// The alphabets of [`ALPHABETS`], read once.
fn alphabets() -> &'static [Alphabet] {
    static READ: OnceLock<Vec<Alphabet>> = OnceLock::new();

    READ.get_or_init(|| {
        ALPHABETS
            .lines()
            .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let tag = fields.next()?;
                let primary = usize::from_str_radix(fields.next()?, 16).ok()?;
                let mut letters = Vec::new();
                let mut skip = Vec::new();
                let mut symbols = 1;

                for class in fields {
                    // `~` — the marks read as nothing (task T-107-5); no symbol of their own.
                    if let Some(marks) = class.strip_prefix('~') {
                        for hex in marks.split('+') {
                            skip.push(char::from_u32(u32::from_str_radix(hex, 16).ok()?)?);
                        }

                        continue;
                    }

                    let symbol = u8::try_from(symbols).ok()?;

                    for hex in class.split('+') {
                        letters.push((char::from_u32(u32::from_str_radix(hex, 16).ok()?)?, symbol));
                    }

                    symbols += 1;
                }

                letters.sort_unstable();
                skip.sort_unstable();

                Some(Alphabet {
                    tag,
                    primary,
                    symbols,
                    letters,
                    skip,
                })
            })
            .collect()
    })
}

/// The alphabet of `tag`, if [`ALPHABETS`] has a line for it.
fn alphabet(tag: &str) -> Option<&'static Alphabet> {
    alphabets().iter().find(|alphabet| alphabet.tag == tag)
}

/// **A trigram table of one language** — see the module documentation.
#[derive(Clone, Copy, Debug)]
pub struct Trigrams {
    costs: &'static [u8],
    alphabet: &'static Alphabet,
}

impl Trigrams {
    /// A table of the English alphabet out of `costs`, or `None` when it is not [`TABLE_LEN`]
    /// bytes long.
    pub fn from_costs(costs: &'static [u8]) -> Option<Self> {
        Self::of(alphabet("en")?, costs)
    }

    /// A table of `alphabet` out of `costs`, or `None` when it is not `S³` bytes long.
    fn of(alphabet: &'static Alphabet, costs: &'static [u8]) -> Option<Self> {
        (costs.len() == alphabet.symbols.pow(3)).then_some(Self { costs, alphabet })
    }

    /// The tag of the language of the table — the first field of its line in the alphabets.
    pub fn language(&self) -> &'static str {
        self.alphabet.tag
    }

    /// How many symbols the model has: the boundary and the classes of the alphabet.
    pub fn symbols(&self) -> usize {
        self.alphabet.symbols
    }

    /// A reading of one candidate text against this table, at its start.
    pub const fn reading(&self) -> Reading<'_> {
        Reading {
            table: self,
            before: 0,
            last: 0,
            cost: 0,
            count: 0,
        }
    }

    /// The cost of `symbol` after `before` and `last`.
    fn cost_of(&self, before: usize, last: usize, symbol: usize) -> u32 {
        let symbols = self.alphabet.symbols;

        u32::from(
            self.costs
                .get((before * symbols + last) * symbols + symbol)
                .copied()
                .unwrap_or(u8::MAX),
        )
    }
}

/// **Every table the image carries**, each with its alphabet — a table whose length does not
/// match its alphabet is left out, and the sweep of `tests\letter_stats.rs` counts them.
pub fn tables() -> Vec<Trigrams> {
    TABLES
        .iter()
        .filter_map(|&(tag, costs)| Trigrams::of(alphabet(tag)?, costs))
        .collect()
}

/// **The table of the language of `layout`**, if this program carries one: the primary language
/// of the layout decides (every English layout reads by the table of English). English, French,
/// Spanish and Russian since stage Э106; Arabic, Persian, Indonesian, Hindi, Tamil and Sinhala
/// since stage Э107; any other language gets none, and the selection path then divides by single
/// keys (196.1 (3)).
pub fn for_layout(layout: LayoutId) -> Option<Trigrams> {
    let language = layout.raw() & 0x3FF;

    let alphabet = alphabets()
        .iter()
        .find(|alphabet| alphabet.primary == language)?;
    let &(_, costs) = TABLES.iter().find(|&&(tag, _)| tag == alphabet.tag)?;

    Trigrams::of(alphabet, costs)
}

/// **One candidate text, read symbol by symbol** — see the module documentation.
#[derive(Clone, Copy, Debug)]
pub struct Reading<'table> {
    table: &'table Trigrams,
    before: usize,
    last: usize,
    cost: u32,
    count: u32,
}

impl Reading<'_> {
    /// Reads the next code units of the candidate.
    pub fn feed(&mut self, units: &[u16]) {
        for ch in char::decode_utf16(units.iter().copied()) {
            // A joiner or a mark of the alphabet's `~` is not a step at all (task T-107-5).
            if ch.as_ref().is_ok_and(|&ch| self.table.alphabet.skips(ch)) {
                continue;
            }

            let symbol = ch.map_or(0, |ch| self.table.alphabet.symbol(ch));

            self.step(symbol);
        }
    }

    /// The mean cost per symbol of the candidate read so far, its end included — lower is more
    /// probable.
    pub fn cost(&self) -> f32 {
        let mut ended = *self;
        ended.step(0);

        ended.cost as f32 / ended.count as f32
    }

    fn step(&mut self, symbol: usize) {
        self.cost = self
            .cost
            .saturating_add(self.table.cost_of(self.before, self.last, symbol));
        self.count = self.count.saturating_add(1);
        self.before = self.last;
        self.last = symbol;
    }
}
