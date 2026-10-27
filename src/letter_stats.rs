//! **Statistics of the letters of a language** — решение 196.1 (2), stage Э105.
//!
//! Text selected in a layout with an ambiguous ligature (Arabic 101: `لا` is the key `B`, and it
//! is `G` then `H`) can be read two ways, and when the short memory of the typing does not have
//! it, the reading is chosen by how probable each is **in the language of the target layout**:
//! `لاث` is `be` and not `ghe`, `مهلاف` is `light` and not `libt`. The probability is a model of
//! letter trigrams — the chance of a letter after the two before it, word boundaries included —
//! counted once from a corpus of the language with a clear licence (the origin and the count are
//! in the report of the stage) and carried in the image as a table of costs.
//!
//! # The table
//!
//! `27 × 27 × 27` bytes: the symbols are the boundary (anything that is not a Latin letter) and
//! `a`…`z`, and the byte at `(a × 27 + b) × 27 + c` is the cost of `c` after `a b` — the negative
//! binary logarithm of its probability, in quarters of a bit, capped at 255. Lower is more
//! probable. 19 683 bytes per language, under the 64 KB the ТЗ allows (§0 п. 10).
//!
//! # What it reads, and what it keeps
//!
//! A [`Reading`] is fed the code units of a candidate as the conversion writes them, and keeps two
//! symbols and two sums — no copy of the text is made (SEC-01). It runs on the UI thread in the
//! selection path, never in the hook.

use crate::layouts::LayoutId;

/// The symbols of the model: the boundary and the 26 letters.
const SYMBOLS: usize = 27;

/// The length of a table of one language.
pub const TABLE_LEN: usize = SYMBOLS * SYMBOLS * SYMBOLS;

/// `LANG_ENGLISH`, the primary language of every English layout.
const LANG_ENGLISH: usize = 0x09;

/// The table of English — counted from the corpus of the report; empty until it is in the tree.
const ENGLISH: &[u8] = &[];

/// **A trigram table of one language** — see the module documentation.
#[derive(Clone, Copy, Debug)]
pub struct Trigrams {
    costs: &'static [u8],
}

impl Trigrams {
    /// A table out of `costs`, or `None` when it is not [`TABLE_LEN`] bytes long.
    pub const fn from_costs(costs: &'static [u8]) -> Option<Self> {
        if costs.len() == TABLE_LEN {
            Some(Self { costs })
        } else {
            None
        }
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
        u32::from(
            self.costs
                .get((before * SYMBOLS + last) * SYMBOLS + symbol)
                .copied()
                .unwrap_or(u8::MAX),
        )
    }
}

/// **The table of the language of `layout`**, if this program carries one — English only in
/// stage Э105 (ТЗ §0 п. 10: other languages are a proposal of the report, not this stage).
pub fn for_layout(layout: LayoutId) -> Option<Trigrams> {
    let language = layout.raw() & 0x3FF;

    if language == LANG_ENGLISH {
        Trigrams::from_costs(ENGLISH)
    } else {
        None
    }
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
            let symbol = match ch {
                Ok(ch) if ch.is_ascii_alphabetic() => {
                    usize::from(ch.to_ascii_lowercase() as u8 - b'a') + 1
                }
                _ => 0,
            };

            self.step(symbol);
        }
    }

    /// The mean cost per symbol of the candidate read so far, its end included — lower is more
    /// probable. A candidate of no symbol at all costs the most.
    pub fn cost(&self) -> f32 {
        let mut ended = *self;
        ended.step(0);

        if ended.count == 0 {
            return f32::MAX;
        }

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
