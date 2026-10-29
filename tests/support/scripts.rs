//! **The script of a sign** — stage E108, решение 202.8: which of the scripts the languages of the
//! program write in a letter or a sign belongs to. Shared by the guard of `tests\l10n.rs` and the
//! sweep of the installer (`tests\installer_languages.rs`), included by `#[path]` as
//! `resource_script.rs` is.

#![allow(dead_code)]

/// The script (ISO 15924) of a letter or sign of the blocks the languages of the program write
/// in, other than Latin; `None` for Latin, digits, punctuation and everything else. The dandas
/// (U+0964, U+0965) stand in the Devanagari block but are shared by the Indic scripts, so they
/// are no script's here.
pub fn script_of_char(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{0964}' | '\u{0965}' => return None,
        '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}' => "Grek",
        '\u{0400}'..='\u{052F}' => "Cyrl",
        '\u{0530}'..='\u{058F}' => "Armn",
        '\u{0590}'..='\u{05FF}' | '\u{FB1D}'..='\u{FB4F}' => "Hebr",
        '\u{0600}'..='\u{06FF}'
        | '\u{0750}'..='\u{077F}'
        | '\u{08A0}'..='\u{08FF}'
        | '\u{FB50}'..='\u{FDFF}'
        | '\u{FE70}'..='\u{FEFF}' => "Arab",
        '\u{0900}'..='\u{097F}' | '\u{A8E0}'..='\u{A8FF}' => "Deva",
        '\u{0980}'..='\u{09FF}' => "Beng",
        '\u{0A00}'..='\u{0A7F}' => "Guru",
        '\u{0A80}'..='\u{0AFF}' => "Gujr",
        '\u{0B00}'..='\u{0B7F}' => "Orya",
        '\u{0B80}'..='\u{0BFF}' => "Taml",
        '\u{0C00}'..='\u{0C7F}' => "Telu",
        '\u{0C80}'..='\u{0CFF}' => "Knda",
        '\u{0D00}'..='\u{0D7F}' => "Mlym",
        '\u{0E00}'..='\u{0E7F}' => "Thai",
        '\u{0E80}'..='\u{0EFF}' => "Laoo",
        '\u{10A0}'..='\u{10FF}' | '\u{1C90}'..='\u{1CBF}' => "Geor",
        '\u{1780}'..='\u{17FF}' | '\u{19E0}'..='\u{19FF}' => "Khmr",
        _ => return None,
    })
}
