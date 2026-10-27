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

/// **The four languages of decision 198.7 carry a table, and every one is the table of its own
/// language** — English since stage Э105, French, Spanish and Russian since Э106 (task T-106-5).
/// The primary language of a layout decides, so every layout of a language reads by its table; a
/// language without one (Arabic: Gutenberg has no corpus of it) gets none, and the selection path
/// then divides by single keys (196.1 (3)).
#[test]
fn every_language_of_decision_198_7_reads_by_its_own_table() {
    for (layout, language) in [
        (0x0409_0409, "en"),
        (0x0809_0809, "en"),
        (0x040C_040C, "fr"),
        (0x080C_080C, "fr"),
        (0x100C_100C, "fr"),
        (0x040A_040A, "es"),
        (0x0C0A_0C0A, "es"),
        (0x080A_080A, "es"),
        (0x0419_0419, "ru"),
        // Stage E107, task T-107-5 (решения 198.22а, 198.22г): the six of the Leipzig corpora —
        // Arabic of the East and of the Maghreb, Persian, Indonesian on the US keys, Hindi, Tamil,
        // Sinhala.
        (0x0401_0401, "ar"),
        (0x1401_1401, "ar"),
        (0x0429_0429, "fa"),
        (0x0409_0421, "id"),
        (0x0439_0439, "hi"),
        (0x0449_0449, "ta"),
        (0x045B_045B, "si"),
    ] {
        let table = letter_stats::for_layout(LayoutId::from_raw(layout));

        assert_eq!(
            table.map(|table| table.language()),
            Some(language),
            "layout {layout:#010x} must read by the table of {language}"
        );
    }

    for layout in [0x0407_0407, 0x0422_0422, 0x041E_041E] {
        assert!(
            letter_stats::for_layout(LayoutId::from_raw(layout)).is_none(),
            "layout {layout:#010x}: no table is carried for its language"
        );
    }
}

/// **Every table the image carries matches its alphabet, and all of them fit the limits of the
/// stage** — ТЗ Э106 п. 8: one table 64 KB at most, all of them 1 MB at most (развилка C5).
#[test]
fn every_table_matches_its_alphabet_and_the_limits() {
    let tables = letter_stats::tables();
    let languages: Vec<&str> = tables.iter().map(|table| table.language()).collect();

    assert_eq!(
        languages,
        ["en", "fr", "es", "ru", "ar", "fa", "id", "hi", "ta", "si"],
        "a table whose length does not match its alphabet is left out"
    );

    let mut total = 0;

    for table in &tables {
        let bytes = table.symbols().pow(3);
        println!(
            "{}: {} symbols, {bytes} bytes",
            table.language(),
            table.symbols()
        );

        assert!(bytes <= 64 * 1024, "{}: {bytes} bytes", table.language());
        total += bytes;
    }

    println!("all tables: {total} bytes");
    assert!(total <= 1024 * 1024, "all tables: {total} bytes");
    assert_eq!(
        tables.iter().map(Trigrams::symbols).collect::<Vec<_>>(),
        [27, 27, 28, 33, 31, 34, 27, 40, 37, 36],
        "the alphabets of decision 198.7: English and French 26 classes, Spanish 27, Russian 32; \
         of stage E107: Arabic 30, Persian 33, Indonesian 26, Hindi 39, Tamil 36, Sinhala 35"
    );
    assert_eq!(total, 347_342, "the ten tables together");
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

/// The table of the language of `layout`, which must be there.
fn table_of(layout: usize, what: &str) -> Trigrams {
    letter_stats::for_layout(LayoutId::from_raw(layout))
        .unwrap_or_else(|| panic!("решение 198.7: {what} ({layout:#010x}) carries no table"))
}

/// Every pair `(word, other)` must read `word` as the more probable text of `table`.
fn assert_prefers(table: &Trigrams, language: &str, pairs: &[(&str, &str)]) {
    let mut wrong = Vec::new();

    for &(word, other) in pairs {
        let (good, bad) = (cost(table, word), cost(table, other));
        println!("{language}: {word} {good:.2} — {other} {bad:.2}");

        if good >= bad {
            wrong.push(format!("{word} ({good:.2}) not below {other} ({bad:.2})"));
        }
    }

    assert!(
        wrong.is_empty(),
        "решение 198.7: the table of {language} reads the less probable text: {wrong:#?}"
    );
}

/// **Task T-106-5, решение 198.7 — the table of French reads the two divisions the way French is
/// written.** The Arabic layouts of the Maghreb sit beside the French one (Arabic (102) AZERTY is
/// made for it): `لا` is the physical `B`, and `ل`, `ا` are `G`, `H` — so a French word with `b`
/// can come back as `gh` and the other way round, and the table must prefer the word.
#[test]
fn the_table_of_french_prefers_the_french_word() {
    let french = table_of(0x040C_040C, "French");

    assert_prefers(
        &french,
        "French",
        &[
            ("bien", "ghien"),
            ("table", "taghle"),
            ("arbre", "arghre"),
            ("bonjour", "ghonjour"),
        ],
    );
}

/// **Task T-106-5 — the table of Spanish**, the same two divisions on the Spanish layout.
#[test]
fn the_table_of_spanish_prefers_the_spanish_word() {
    let spanish = table_of(0x0C0A_0C0A, "Spanish");

    assert_prefers(
        &spanish,
        "Spanish",
        &[
            ("bien", "ghien"),
            ("hablar", "haghlar"),
            ("libro", "lighro"),
            ("buenos", "ghuenos"),
        ],
    );
}

/// **Task T-106-5 — the table of Russian.** On the Russian layout the physical `B` is `и` and `G`,
/// `H` are `п`, `р`: `и` can come back as `пр`, and `пр` as `и`.
#[test]
fn the_table_of_russian_prefers_the_russian_word() {
    let russian = table_of(0x0419_0419, "Russian");

    assert_prefers(
        &russian,
        "Russian",
        &[
            ("мир", "мпрр"),
            ("кино", "кпрно"),
            ("три", "трпр"),
            ("прост", "иост"),
            ("правда", "иавда"),
        ],
    );
}

/// **Task T-106-5 — every table is the table of its own language**, measured on a sentence of each:
/// a sentence costs least under the table of its language. A table put under another language
/// (the mutant of the task) reads the sentence of its own language and loses here.
#[test]
fn a_sentence_costs_least_under_the_table_of_its_language() {
    let samples = [
        (
            0x0409_0409,
            "English",
            "the children play in the garden while their parents talk about life",
        ),
        (
            0x040C_040C,
            "French",
            "les enfants jouent dans le jardin pendant que leurs parents parlent de la vie",
        ),
        (
            0x0C0A_0C0A,
            "Spanish",
            "los niños juegan en el jardín mientras sus padres hablan de la vida",
        ),
        (
            0x0419_0419,
            "Russian",
            "дети играют в саду пока родители говорят о жизни",
        ),
    ];

    let mut wrong = Vec::new();

    for &(own, language, sentence) in &samples {
        let mine = cost(&table_of(own, language), sentence);

        for &(other, other_language, _) in &samples {
            if other == own {
                continue;
            }

            let theirs = cost(&table_of(other, other_language), sentence);
            println!("{language} sentence: own {mine:.2}, {other_language} {theirs:.2}");

            if mine >= theirs {
                wrong.push(format!(
                    "the {language} sentence: own table {mine:.2}, {other_language} {theirs:.2}"
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "решение 198.7: a table is not the table of its language: {wrong:#?}"
    );
}

// =========================================================================================
// Stage E107, task T-107-5 — the six target languages of the Leipzig corpora (198.22а, 198.22г)
// =========================================================================================

/// The layouts of the six, one each, and their names.
const SIX: [(usize, &str); 6] = [
    (0x0401_0401, "Arabic"),
    (0x0429_0429, "Persian"),
    (0x0409_0421, "Indonesian"),
    (0x0439_0439, "Hindi"),
    (0x0449_0449, "Tamil"),
    (0x045B_045B, "Sinhala"),
];

/// **The marks of the `~` of an alphabet are read as nothing** — task T-107-5: a word written with
/// a joiner, a nukta or a vowel mark is the same word as without it, under its own table. The
/// Sinhala corpus holds 111 802 zero-width joiners (`ශ්‍රී`), and a mark read as a boundary would cut
/// every such word in two.
///
/// ⚠ Control: under the table of English, which skips nothing, the joiner is a boundary and the
/// two costs part — the check of «the same» can fail.
#[test]
fn the_marks_of_the_tilde_are_read_as_nothing() {
    for (layout, language, with, without) in [
        (0x045B_045B, "Sinhala", "ශ්\u{200D}රී ලංකාව", "ශ්රී ලංකාව"),
        (0x0439_0439, "Hindi", "ज\u{093C}रूर", "जरूर"),
        (0x0429_0429, "Persian", "می\u{200C}روم", "میروم"),
        (0x0401_0401, "Arabic", "كِتَابٌ", "كتاب"),
    ] {
        let table = table_of(layout, language);
        let (marked, plain) = (cost(&table, with), cost(&table, without));

        println!("{language}: {with} {marked:.2} — {without} {plain:.2}");
        assert!(
            (marked - plain).abs() < f32::EPSILON,
            "{language}: the marks of «{with}» must be read as nothing"
        );
    }

    let english = table_of(0x0409_0409, "English");

    assert!(
        (cost(&english, "ab\u{200D}cd") - cost(&english, "abcd")).abs() > 0.5,
        "the control: a table that skips nothing reads the joiner as a boundary"
    );
}

/// **Task T-107-5 — the table of each of the six prefers a word of its language to the word spoilt
/// by one letter.** Words of everyday writing; the spoilt letter makes a sequence the language does
/// not write.
#[test]
fn the_tables_of_the_six_prefer_the_word_of_their_language() {
    /// A layout, the name of its language, and its pairs `(word, spoilt)`.
    type Pairs = (usize, &'static str, &'static [(&'static str, &'static str)]);

    let pairs: [Pairs; 6] = [
        (
            0x0401_0401,
            "Arabic",
            &[("مدرسة", "مدرثظ"), ("الكتاب", "الكتظب")],
        ),
        (
            0x0429_0429,
            "Persian",
            &[("کتاب", "کتظب"), ("خانه", "خانظ")],
        ),
        (
            0x0409_0421,
            "Indonesian",
            &[("terima", "terxma"), ("sekolah", "sekqlah")],
        ),
        (0x0439_0439, "Hindi", &[("नमस्ते", "नमस्ठे"), ("भारत", "भाढत")]),
        (
            0x0449_0449,
            "Tamil",
            &[("வணக்கம்", "வணக்ஙம்"), ("தமிழ்", "தமிஞ்")],
        ),
        // ⚠ Not «ආයුබෝඩන්»: `ඩ` there is as probable as `ව` — the two sums are one, 45 and 45
        // (scratchpad-E107\stats\probe-cost.pl); a pair that cannot tell is no pair.
        (
            0x045B_045B,
            "Sinhala",
            &[("ආයුබෝවන්", "ආයුබෝඤන්"), ("ලංකාව", "ලංකාඤ")],
        ),
    ];

    for (layout, language, words) in pairs {
        assert_prefers(&table_of(layout, language), language, words);
    }
}

/// **Task T-107-5 — every table of the six is the table of its own language**, measured on a
/// sentence of each, as `a_sentence_costs_least_under_the_table_of_its_language` measures the four:
/// the sentence costs least under its own table. Arabic and Persian share a script, Indonesian
/// shares its letters with English — the two places a table put under another language would hide.
#[test]
fn a_sentence_of_the_six_costs_least_under_its_own_table() {
    let samples = [
        (
            0x0401_0401,
            "Arabic",
            "ذهب الأطفال إلى المدرسة صباحا مع أصدقائهم",
        ),
        (
            0x0429_0429,
            "Persian",
            "بچه‌ها صبح با دوستانشان به مدرسه رفتند",
        ),
        (
            0x0409_0421,
            "Indonesian",
            "anak anak pergi ke sekolah bersama teman mereka pagi ini",
        ),
        (0x0439_0439, "Hindi", "बच्चे सुबह अपने दोस्तों के साथ स्कूल गए"),
        (
            0x0449_0449,
            "Tamil",
            "குழந்தைகள் காலையில் நண்பர்களுடன் பள்ளிக்குச் சென்றனர்",
        ),
        (0x045B_045B, "Sinhala", "ළමයි උදේ යාළුවන් එක්ක පාසලට ගියා"),
    ];
    let others = SIX
        .iter()
        .copied()
        .chain([(0x0409_0409, "English"), (0x040C_040C, "French")]);

    let mut wrong = Vec::new();

    for &(own, language, sentence) in &samples {
        let mine = cost(&table_of(own, language), sentence);

        for (other, other_language) in others.clone() {
            if other == own {
                continue;
            }

            let theirs = cost(&table_of(other, other_language), sentence);

            if mine >= theirs {
                wrong.push(format!(
                    "the {language} sentence: own table {mine:.2}, {other_language} {theirs:.2}"
                ));
            }
        }

        println!("{language} sentence: own {mine:.2}");
    }

    assert!(
        wrong.is_empty(),
        "T-107-5: a table is not the table of its language: {wrong:#?}"
    );
}
