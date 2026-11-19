//! The guard of the interface shell, `src\ui\` (stage Э120, T-120-2): the numbers of the look live
//! in `tokens.rs` and nowhere else, and Direct2D and DirectWrite stay in the drawing files.
//!
//! Four rules, each its own test:
//!
//! 1. no colour is written anywhere in `src\ui\` but `tokens.rs` — neither as a six- or
//!    eight-digit hexadecimal literal nor as a group of three or four numbers (`(r, g, b)`,
//!    `[r, g, b, a]`, `r: …, g: …, b: …`);
//! 2. a window description, `src\ui\windows\`, carries no number with a fractional part, no integer
//!    above 1, and does not name `tokens`, `canvas` or `text`: a window says *what* it shows,
//!    never how big or in what colour;
//! 3. the names of Direct2D and DirectWrite occur only in `canvas.rs`, `text.rs` and `art.rs`;
//! 4. `elements.rs`, `layout.rs`, `scene.rs` and `host.rs` use no number but 0, 1, 2 and 0.5 —
//!    every other one is a value of the look and comes from `tokens`.
//!
//! The sources are read as text and **cut into tokens**: comments, strings and character literals
//! are skipped by the lexer, not by a substring search over the file, so a sentence about
//! `0xFAF9F6` or `ID2D1Factory` is not a violation and a literal in code is one. The lexer has its
//! own tests at the end of this file.

use std::fs;
use std::path::{Path, PathBuf};

/// A token of Rust source, as far as the guard needs one.
#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Number(Number),
    Punct(char),
}

/// A numeric literal. `digits` counts the digits of a hexadecimal literal, without the prefix, the
/// separators and the suffix; `float` is a literal with a fractional part, an exponent or a float
/// suffix.
#[derive(Clone, Debug, PartialEq)]
struct Number {
    text: String,
    value: f64,
    float: bool,
    hex_digits: Option<usize>,
}

/// A token and the line it starts on, counted from 1.
#[derive(Clone, Debug, PartialEq)]
struct Token {
    tok: Tok,
    line: usize,
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The source cut into tokens; comments, strings and character literals leave nothing behind.
fn lex(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut at = 0;

    // Skips a quoted body that starts after the opening quote at `at`, honouring backslash
    // escapes; returns the index after the closing quote.
    let skip_quoted = |mut at: usize, quote: char, line: &mut usize| -> usize {
        while at < chars.len() {
            match chars[at] {
                '\\' => {
                    if chars.get(at + 1) == Some(&'\n') {
                        *line += 1;
                    }
                    at += 2;
                }
                '\n' => {
                    *line += 1;
                    at += 1;
                }
                c if c == quote => return at + 1,
                _ => at += 1,
            }
        }
        at
    };

    // Skips a raw string whose `r` is at `at - 1` and whose hashes start at `at`.
    let skip_raw = |mut at: usize, line: &mut usize| -> usize {
        let mut hashes = 0;
        while chars.get(at) == Some(&'#') {
            hashes += 1;
            at += 1;
        }
        at += 1; // the opening quote
        while at < chars.len() {
            if chars[at] == '\n' {
                *line += 1;
            }
            if chars[at] == '"' && (1..=hashes).all(|k| chars.get(at + k) == Some(&'#')) {
                return at + 1 + hashes;
            }
            at += 1;
        }
        at
    };

    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();

        if c == '\n' {
            line += 1;
            at += 1;
        } else if c.is_whitespace() {
            at += 1;
        } else if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[at] == '\n' {
                        line += 1;
                    }
                    at += 1;
                }
            }
        } else if c == '"' {
            at = skip_quoted(at + 1, '"', &mut line);
        } else if c == '\'' {
            // A character literal, or a lifetime: `'x'` and `'\n'` are literals, `'a` is not.
            if next == Some('\\') {
                at = skip_quoted(at + 1, '\'', &mut line);
            } else if chars.get(at + 2) == Some(&'\'') {
                at += 3;
            } else {
                tokens.push(Token {
                    tok: Tok::Punct('\''),
                    line,
                });
                at += 1;
            }
        } else if c.is_ascii_digit() {
            let after_dot = matches!(
                tokens.last(),
                Some(Token {
                    tok: Tok::Punct('.'),
                    ..
                })
            );
            let (number, end) = lex_number(&chars, at, after_dot);
            tokens.push(Token {
                tok: Tok::Number(number),
                line,
            });
            at = end;
        } else if is_ident_start(c) {
            let start = at;
            while at < chars.len() && is_ident_continue(chars[at]) {
                at += 1;
            }
            let word: String = chars[start..at].iter().collect();
            let here = chars.get(at).copied();

            match (word.as_str(), here) {
                ("r" | "br" | "cr", Some('"' | '#'))
                    if here == Some('"') || matches!(chars.get(at + 1), Some('"' | '#')) =>
                {
                    at = skip_raw(at, &mut line);
                }
                ("r", Some('#')) => {
                    // A raw identifier, `r#type`: the identifier is what follows.
                    at += 1;
                }
                ("b" | "c", Some('"')) => at = skip_quoted(at + 1, '"', &mut line),
                ("b", Some('\'')) => at = skip_quoted(at + 1, '\'', &mut line),
                _ => tokens.push(Token {
                    tok: Tok::Ident(word),
                    line,
                }),
            }
        } else {
            tokens.push(Token {
                tok: Tok::Punct(c),
                line,
            });
            at += 1;
        }
    }

    tokens
}

/// One numeric literal starting at `start`. After a `.` it is a tuple index, digits only: `x.0.1`
/// is two indices, not the number 0.1.
fn lex_number(chars: &[char], start: usize, tuple_index: bool) -> (Number, usize) {
    let mut at = start;
    let radix = match (chars[at], chars.get(at + 1)) {
        ('0', Some('x' | 'X')) => 16,
        ('0', Some('o' | 'O')) => 8,
        ('0', Some('b' | 'B')) => 2,
        _ => 10,
    };

    if radix != 10 {
        at += 2;
        let digits_start = at;
        while at < chars.len() && (chars[at].is_ascii_hexdigit() || chars[at] == '_') {
            at += 1;
        }
        // As the compiler reads it: hexadecimal digits run on as long as they can (`0x1f32` is one
        // number), and what follows them is the suffix (`0xFFu8`).
        let digits: String = chars[digits_start..at]
            .iter()
            .filter(|c| **c != '_')
            .collect();
        while at < chars.len() && is_ident_continue(chars[at]) {
            at += 1;
        }
        let text: String = chars[start..at].iter().collect();
        let value = u64::from_str_radix(&digits, radix).map_or(f64::NAN, |v| v as f64);
        let hex_digits = (radix == 16).then_some(digits.len());

        return (
            Number {
                text,
                value,
                float: false,
                hex_digits,
            },
            at,
        );
    }

    let mut float = false;
    while at < chars.len() && (chars[at].is_ascii_digit() || chars[at] == '_') {
        at += 1;
    }

    if !tuple_index && chars.get(at) == Some(&'.') {
        let after = chars.get(at + 1).copied();
        // `1.5` and `1.` are floats; `1..2` is a range and `1.max(2)` a method call.
        let is_fraction = match after {
            Some(d) if d.is_ascii_digit() => true,
            Some('.') => false,
            Some(c) if is_ident_start(c) => false,
            _ => true,
        };
        if is_fraction {
            float = true;
            at += 1;
            while at < chars.len() && (chars[at].is_ascii_digit() || chars[at] == '_') {
                at += 1;
            }
        }
    }

    if !tuple_index && matches!(chars.get(at), Some('e' | 'E')) {
        let mut probe = at + 1;
        if matches!(chars.get(probe), Some('+' | '-')) {
            probe += 1;
        }
        if chars.get(probe).is_some_and(char::is_ascii_digit) {
            float = true;
            at = probe;
            while at < chars.len() && (chars[at].is_ascii_digit() || chars[at] == '_') {
                at += 1;
            }
        }
    }

    let body: String = chars[start..at].iter().filter(|c| **c != '_').collect();
    let suffix_start = at;
    while at < chars.len() && is_ident_continue(chars[at]) {
        at += 1;
    }
    let suffix: String = chars[suffix_start..at].iter().collect();
    if suffix.starts_with('f') {
        float = true;
    }

    (
        Number {
            text: chars[start..at].iter().collect(),
            value: body.parse().unwrap_or(f64::NAN),
            float,
            hex_digits: None,
        },
        at,
    )
}

/// `src\ui\`.
fn shell_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("ui")
}

/// Every `.rs` file under `src\ui\`, by its path relative to it with forward slashes, and its
/// tokens.
fn shell_files() -> Vec<(String, Vec<Token>)> {
    fn walk(folder: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(folder).expect("src\\ui\\ must be readable") {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }

    let root = shell_root();
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&root)
                .expect("a file under src\\ui\\")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
            (name, lex(&text))
        })
        .collect()
}

/// A violation, as the failure prints it.
fn at(file: &str, token: &Token, what: &str) -> String {
    format!(
        "src\\ui\\{}:{}: {what}",
        file.replace('/', "\\"),
        token.line
    )
}

fn number(token: &Token) -> Option<&Number> {
    match &token.tok {
        Tok::Number(number) => Some(number),
        _ => None,
    }
}

fn is_punct(token: Option<&Token>, c: char) -> bool {
    matches!(token, Some(Token { tok: Tok::Punct(p), .. }) if *p == c)
}

fn is_ident(token: Option<&Token>, name: &str) -> bool {
    matches!(token, Some(Token { tok: Tok::Ident(word), .. }) if word == name)
}

/// The colours written in `tokens` — rule 1.
fn colours_in(file: &str, tokens: &[Token]) -> Vec<String> {
    let mut found = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        // A hexadecimal colour: six digits, or eight with alpha.
        if let Some(Number {
            hex_digits: Some(6 | 8),
            text,
            ..
        }) = number(token)
        {
            found.push(at(file, token, &format!("colour {text}")));
        }

        // A bracket holding nothing but three or four numbers: `(r, g, b)`, `[r, g, b, a]`.
        if is_punct(Some(token), '(') || is_punct(Some(token), '[') {
            let close = if is_punct(Some(token), '(') { ')' } else { ']' };
            let mut count = 0;
            let mut k = i + 1;
            while tokens.get(k).and_then(number).is_some() {
                count += 1;
                k += 1;
                if is_punct(tokens.get(k), ',') {
                    k += 1;
                }
                if is_punct(tokens.get(k), close) {
                    if count == 3 || count == 4 {
                        found.push(at(file, token, &format!("{count} numbers in one group")));
                    }
                    break;
                }
            }
        }

        // A colour structure: `r: …, g: …, b: …`.
        let channel = |k: usize, name: &str| {
            is_ident(tokens.get(k), name)
                && is_punct(tokens.get(k + 1), ':')
                && tokens.get(k + 2).and_then(number).is_some()
        };
        if channel(i, "r")
            && is_punct(tokens.get(i + 3), ',')
            && channel(i + 4, "g")
            && is_punct(tokens.get(i + 7), ',')
            && channel(i + 8, "b")
        {
            found.push(at(file, token, "the channels r, g, b as numbers"));
        }
    }

    found
}

/// What a window description must not carry — rule 2.
fn window_description_faults(file: &str, tokens: &[Token]) -> Vec<String> {
    let mut found = Vec::new();

    for token in tokens {
        match &token.tok {
            Tok::Number(number) if number.float => {
                found.push(at(
                    file,
                    token,
                    &format!("fractional number {}", number.text),
                ));
            }
            Tok::Number(number) if number.value.is_nan() || number.value > 1.0 => {
                found.push(at(file, token, &format!("number {} above 1", number.text)));
            }
            Tok::Ident(word) if matches!(word.as_str(), "tokens" | "canvas" | "text") => {
                found.push(at(file, token, &format!("names `{word}`")));
            }
            _ => {}
        }
    }

    found
}

/// The files that may name Direct2D and DirectWrite — rule 3.
const DRAWING_FILES: [&str; 3] = ["canvas.rs", "text.rs", "art.rs"];

/// Whether `word` is a name of Direct2D or DirectWrite: the interfaces `ID2D1…` and `IDWrite…`,
/// the types and constants `D2D1_…` and `DWRITE_…`, the functions `D2D1…` and `DWrite…`, and the
/// module names of the `windows` crate.
fn is_drawing_name(word: &str) -> bool {
    ["ID2D1", "D2D1", "IDWrite", "DWrite", "DWRITE"]
        .iter()
        .any(|prefix| word.starts_with(prefix))
        || word == "Direct2D"
        || word == "DirectWrite"
}

/// The names of Direct2D and DirectWrite outside the drawing files — rule 3.
fn drawing_names_in(file: &str, tokens: &[Token]) -> Vec<String> {
    if DRAWING_FILES.contains(&file) {
        return Vec::new();
    }

    tokens
        .iter()
        .filter_map(|token| match &token.tok {
            Tok::Ident(word) if is_drawing_name(word) => Some(at(file, token, word)),
            _ => None,
        })
        .collect()
}

/// The files of rule 4.
const LOGIC_FILES: [&str; 4] = ["elements.rs", "layout.rs", "scene.rs", "host.rs"];

/// The numbers rule 4 lets through.
const PLAIN_NUMBERS: [f64; 4] = [0.0, 1.0, 2.0, 0.5];

/// The numbers of the look in a logic file — rule 4.
fn look_numbers_in(file: &str, tokens: &[Token]) -> Vec<String> {
    if !LOGIC_FILES.contains(&file) {
        return Vec::new();
    }

    tokens
        .iter()
        .filter_map(|token| match number(token) {
            Some(number) if !PLAIN_NUMBERS.contains(&number.value) => {
                Some(at(file, token, &format!("number {}", number.text)))
            }
            _ => None,
        })
        .collect()
}

/// Runs one rule over the shell and fails with every violation it found.
fn check(rule: &str, find: fn(&str, &[Token]) -> Vec<String>) {
    let found: Vec<String> = shell_files()
        .iter()
        .flat_map(|(file, tokens)| find(file, tokens))
        .collect();

    assert!(found.is_empty(), "{rule}:\n{}", found.join("\n"));
}

#[test]
fn rule_1_a_colour_is_written_only_in_tokens() {
    check(
        "rule 1: a colour outside src\\ui\\tokens.rs",
        |file, tokens| {
            if file == "tokens.rs" {
                Vec::new()
            } else {
                colours_in(file, tokens)
            }
        },
    );
}

#[test]
fn rule_2_a_window_description_carries_no_numbers_and_no_drawing() {
    check(
        "rule 2: a window description says how it looks",
        |file, tokens| {
            if file.starts_with("windows/") {
                window_description_faults(file, tokens)
            } else {
                Vec::new()
            }
        },
    );
}

#[test]
fn rule_3_direct2d_and_directwrite_stay_in_the_drawing_files() {
    check(
        "rule 3: Direct2D or DirectWrite outside canvas.rs, text.rs, art.rs",
        drawing_names_in,
    );
}

#[test]
fn rule_4_the_logic_files_take_their_numbers_from_tokens() {
    check(
        "rule 4: a number of the look in elements.rs, layout.rs, scene.rs or host.rs",
        look_numbers_in,
    );
}

/// The rules are green on an empty folder too, so the guard must prove it read the shell: every
/// file the rules name exists and was cut into tokens.
#[test]
fn the_guard_reads_every_file_the_rules_name() {
    let files: Vec<String> = shell_files().into_iter().map(|(file, _)| file).collect();

    for name in DRAWING_FILES
        .iter()
        .chain(LOGIC_FILES.iter())
        .chain(["tokens.rs", "windows/mod.rs"].iter())
    {
        assert!(
            files.iter().any(|file| file == name),
            "src\\ui\\{name} was not read; read: {files:?}"
        );
    }
}

// The lexer.

fn numbers(source: &str) -> Vec<f64> {
    lex(source)
        .iter()
        .filter_map(|token| number(token).map(|number| number.value))
        .collect()
}

fn idents(source: &str) -> Vec<String> {
    lex(source)
        .into_iter()
        .filter_map(|token| match token.tok {
            Tok::Ident(word) => Some(word),
            _ => None,
        })
        .collect()
}

#[test]
fn the_lexer_skips_comments_strings_and_characters() {
    let source = r##"
        // 0xFAF9F6 ID2D1Factory
        /// 14
        //! 7
        /* 3 /* nested 4 */ 5 */
        let a = "0xFAF9F6 \" 14";
        let b = r#"ID2D1Factory "quoted" 7"#;
        let c = b"14";
        let d = '7';
        let e = '\'';
        let f = b'9';
    "##;

    assert!(numbers(source).is_empty(), "{:?}", numbers(source));
    assert_eq!(
        idents(source),
        [
            "let", "a", "let", "b", "let", "c", "let", "d", "let", "e", "let", "f"
        ]
    );
}

#[test]
fn the_lexer_tells_a_lifetime_from_a_character() {
    assert_eq!(numbers("fn f<'a>(x: &'a u8) -> u8 { 7 }"), [7.0]);
    assert_eq!(
        idents("fn f<'a>(x: &'a u8)"),
        ["fn", "f", "a", "x", "a", "u8"]
    );
}

#[test]
fn the_lexer_reads_numbers_as_the_compiler_does() {
    let lexed = lex("0.5f32 1_000 0xFF_FF_FF 2. 1e3 x.0.1 1..3 1.max(2) 0b101 0x1");
    let seen: Vec<(String, f64, bool, Option<usize>)> = lexed
        .iter()
        .filter_map(number)
        .map(|n| (n.text.clone(), n.value, n.float, n.hex_digits))
        .collect();

    assert_eq!(
        seen,
        [
            ("0.5f32".into(), 0.5, true, None),
            ("1_000".into(), 1000.0, false, None),
            ("0xFF_FF_FF".into(), 16_777_215.0, false, Some(6)),
            ("2.".into(), 2.0, true, None),
            ("1e3".into(), 1000.0, true, None),
            ("0".into(), 0.0, false, None),
            ("1".into(), 1.0, false, None),
            ("1".into(), 1.0, false, None),
            ("3".into(), 3.0, false, None),
            ("1".into(), 1.0, false, None),
            ("2".into(), 2.0, false, None),
            ("0b101".into(), 5.0, false, None),
            ("0x1".into(), 1.0, false, Some(1)),
        ]
    );
}

#[test]
fn the_lexer_counts_lines_across_comments_and_strings() {
    let source = "/* one\ntwo */\nlet s = \"three\nfour\";\nlet r = r#\"five\nsix\"#;\n7";
    let last = lex(source).pop().expect("a token");

    assert_eq!(last.line, 7);
    assert_eq!(number(&last).map(|n| n.value), Some(7.0));
}

#[test]
fn each_rule_finds_its_violation_in_code_and_not_in_a_comment() {
    let colour =
        lex("let x = 0xFAF9F6; // 0x112233\nlet y = (31, 122, 74);\nC { r: 1.0, g: 0.5, b: 0.0 }");
    assert_eq!(colours_in("elements.rs", &colour).len(), 3);

    let window = lex("// 14 1.5 text\nrow(14);\ngap(1.5);\nuse tokens;");
    assert_eq!(
        window_description_faults("windows/about.rs", &window),
        [
            "src\\ui\\windows\\about.rs:2: number 14 above 1",
            "src\\ui\\windows\\about.rs:3: fractional number 1.5",
            "src\\ui\\windows\\about.rs:4: names `tokens`",
        ]
    );

    let drawing = lex(
        "// ID2D1Factory\nlet f: ID2D1Factory;\nD2D1CreateFactory();\nDWRITE_FONT_WEIGHT(600);",
    );
    assert_eq!(drawing_names_in("layout.rs", &drawing).len(), 3);
    assert!(drawing_names_in("canvas.rs", &drawing).is_empty());

    let logic = lex("// 7\nlet a = 0.0 + 1.0 * 2.0 - 0.5;\nlet b = 7;");
    assert_eq!(
        look_numbers_in("layout.rs", &logic),
        ["src\\ui\\layout.rs:3: number 7"]
    );
    assert!(look_numbers_in("canvas.rs", &logic).is_empty());
}

#[test]
fn win32_calls_with_zero_arguments_are_not_colours() {
    let call =
        lex("SetWindowPos(hwnd, None, 0, 0, 0, 0, flags); GetMessageW(&mut msg, None, 0, 0);");
    assert!(colours_in("host.rs", &call).is_empty());
}
