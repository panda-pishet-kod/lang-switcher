//! The flag of the layout on the tray icon — FR-90б, task T-119-2 (решения 208.88, 208.89).
//!
//! What is checked here is the drawing and the choice of the face, by reading the pixels back:
//! the table of flags is sane, a flag lands where the mock-up puts it (16 × 12 in the middle of the
//! square), the frame is dark on a light taskbar and light on a dark one, the pause greys, the
//! letter puts its dot in the corner, a country without a flag gets a plate — and **every layout
//! in the registry of Windows gets a flag or a plate**, never nothing.

use lang_switcher::flags::{self, FLAGS, Face, Look};

const LIGHT: Look = Look {
    size: 16,
    paused: false,
    unread: false,
    dark_bar: false,
};

/// The pixel `(x, y)` as `(r, g, b, a)`.
fn at(pixels: &[[u8; 4]], size: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let p = pixels[(y * size + x) as usize];
    (p[2], p[1], p[0], p[3])
}

fn flag(country: &str) -> Face {
    Face::Flag(flags::flag_of(country).unwrap_or_else(|| panic!("the table has {country}")))
}

/// The table: every code two capital letters, every code once, every figure inside a sane box.
#[test]
fn the_table_of_flags_is_sane() {
    let mut seen = std::collections::BTreeSet::new();

    for one in FLAGS {
        assert!(
            one.country.len() == 2 && one.country.chars().all(|c| c.is_ascii_uppercase()),
            "a code of two capitals: {}",
            one.country
        );
        assert!(seen.insert(one.country), "{} appears twice", one.country);
        assert!(!one.figures.is_empty(), "{} has figures", one.country);
    }

    println!("{} flags", FLAGS.len());
    assert!(
        FLAGS.len() >= 80,
        "the countries of the languages of the interface and their neighbours"
    );

    for code in [
        "RU", "US", "GB", "DE", "FR", "UA", "KZ", "BY", "TR", "IL", "JP", "CN", "IN", "NP",
    ] {
        assert!(flags::flag_of(code).is_some(), "{code} has a flag");
    }
}

/// Russia at 100 %: two rows of air, white, blue, red, two rows of air; the frame darkens the edge.
#[test]
fn a_flag_lands_in_the_middle_of_the_square_in_its_colours() {
    let pixels = flags::render(&flag("RU"), LIGHT);

    for x in 0..16 {
        assert_eq!(at(&pixels, 16, x, 0).3, 0, "row 0 is air");
        assert_eq!(at(&pixels, 16, x, 1).3, 0, "row 1 is air");
        assert_eq!(at(&pixels, 16, x, 14).3, 0, "row 14 is air");
        assert_eq!(at(&pixels, 16, x, 15).3, 0, "row 15 is air");
    }

    assert_eq!(
        at(&pixels, 16, 8, 4),
        (255, 255, 255, 255),
        "the white stripe"
    );
    assert_eq!(at(&pixels, 16, 8, 8), (0, 57, 166, 255), "the blue stripe");
    assert_eq!(at(&pixels, 16, 8, 12), (213, 43, 30, 255), "the red stripe");

    let edge = at(&pixels, 16, 0, 8);
    assert!(
        edge.2 < 166 && edge.3 == 255,
        "the frame darkens the edge: {edge:?}"
    );
}

/// The frame follows the taskbar: dark on a light one, light on a dark one — on the white stripe
/// of Russia the dark frame shows and the light one does not.
#[test]
fn the_frame_is_dark_on_a_light_bar_and_light_on_a_dark_one() {
    let light = flags::render(&flag("RU"), LIGHT);
    let dark = flags::render(
        &flag("RU"),
        Look {
            dark_bar: true,
            ..LIGHT
        },
    );

    assert!(
        at(&light, 16, 0, 4).0 < 200,
        "a dark frame on the white stripe"
    );
    assert_eq!(at(&dark, 16, 0, 4).0, 255, "no dark line on a dark bar");
    assert!(
        at(&dark, 16, 0, 12).0 > 213,
        "a light frame lifts the red edge"
    );
}

/// Every size the shell asks for, 100 % to 200 %, draws the flag in the same place.
#[test]
fn every_size_from_100_to_200_percent_draws_the_flag() {
    for size in [16_u32, 20, 24, 28, 32] {
        let pixels = flags::render(&flag("US"), Look { size, ..LIGHT });
        assert_eq!(pixels.len(), (size * size) as usize);
        assert_eq!(
            at(&pixels, size, size / 2, 0).3,
            0,
            "{size}: the top row is air"
        );
        assert_eq!(
            at(&pixels, size, size / 2, size / 2).3,
            255,
            "{size}: the middle is flag"
        );
    }
}

/// The pause: grey and paler (решение 208.89 п. 5).
#[test]
fn the_pause_greys_the_flag() {
    let pixels = flags::render(
        &flag("UA"),
        Look {
            paused: true,
            ..LIGHT
        },
    );
    let (r, g, b, a) = at(&pixels, 16, 8, 5);

    assert!(r == g && g == b, "grey: {r} {g} {b}");
    assert!(a <= 153, "paler: alpha {a}");
}

/// The letter: the light dot in its dark ring in the bottom right corner, inside the square.
#[test]
fn an_unread_letter_puts_the_dot_in_the_corner() {
    let without = flags::render(&flag("DE"), LIGHT);
    let with = flags::render(
        &flag("DE"),
        Look {
            unread: true,
            ..LIGHT
        },
    );

    assert_eq!(
        at(&with, 16, 12, 12),
        (245, 246, 247, 255),
        "the fill of the dot"
    );
    assert_ne!(at(&with, 16, 15, 12), at(&without, 16, 15, 12), "the ring");
    assert_eq!(
        at(&with, 16, 2, 4),
        at(&without, 16, 2, 4),
        "the rest of the flag is untouched"
    );
}

/// A country without a flag: a plate with the letters of the language; nothing known: «?».
#[test]
fn a_country_without_a_flag_gets_a_plate_with_letters() {
    assert_eq!(flags::face_from(Some("US"), Some("en")), flag("US"));
    assert_eq!(
        flags::face_from(Some("XX"), Some("eo")),
        Face::Plate("EO".to_owned())
    );
    assert_eq!(
        flags::face_from(None, Some("haw")),
        Face::Plate("HA".to_owned())
    );
    assert_eq!(flags::face_from(None, None), Face::Plate("?".to_owned()));

    let plate = flags::render(&Face::Plate("EO".to_owned()), LIGHT);
    let ink = plate.iter().filter(|p| p[3] == 255 && p[2] > 200).count();
    let ground = plate.iter().filter(|p| p[3] == 255 && p[2] < 80).count();

    assert!(
        ink > 10 && ground > 40,
        "letters on the plate: ink {ink}, ground {ground}"
    );
}

/// **The guard of the task: every layout of the registry of Windows gives a flag or a plate.**
/// The keys of `HKLM\SYSTEM\CurrentControlSet\Control\Keyboard Layouts` are the layouts Windows
/// knows; the language of each is the low word of its identifier.
#[test]
fn every_layout_in_the_registry_of_windows_gives_a_flag_or_a_plate() {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
    };
    use windows::core::{PWSTR, w};

    let mut key = HKEY::default();
    // SAFETY: a constant path, a key handle of this frame closed below.
    unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            w!("SYSTEM\\CurrentControlSet\\Control\\Keyboard Layouts"),
            None,
            KEY_READ,
            &mut key,
        )
    }
    .ok()
    .expect("the registry of layouts opens");

    let mut languages = std::collections::BTreeSet::new();
    let mut index = 0;

    loop {
        let mut name = [0u16; 64];
        let mut len = name.len() as u32;
        // SAFETY: `name` is a live buffer whose length is passed; the key is open.
        let read = unsafe {
            RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut len,
                None,
                None,
                None,
                None,
            )
        };

        if read.is_err() {
            break;
        }

        let klid = String::from_utf16_lossy(&name[..len as usize]);
        if let Ok(value) = u32::from_str_radix(&klid, 16) {
            languages.insert((value & 0xFFFF) as u16);
        }

        index += 1;
    }

    // SAFETY: the key opened above.
    let _ = unsafe { RegCloseKey(key) };

    assert!(
        languages.len() > 50,
        "the registry lists the languages of Windows: {}",
        languages.len()
    );

    let mut with_flag = 0;
    let mut plates = Vec::new();

    for language in &languages {
        match flags::face_of_language(*language) {
            Face::Flag(_) => with_flag += 1,
            Face::Plate(letters) => {
                assert!(!letters.is_empty(), "{language:#06X}: a plate has letters");
                plates.push(format!("{language:04X}:{letters}"));
            }
        }

        // And every face draws.
        let pixels = flags::render(&flags::face_of_language(*language), LIGHT);
        assert!(
            pixels.iter().any(|p| p[3] > 0),
            "{language:#06X}: something is drawn"
        );
    }

    println!(
        "{} languages: {with_flag} flags, {} plates: {}",
        languages.len(),
        plates.len(),
        plates.join(" ")
    );
    assert!(with_flag > languages.len() / 2, "most layouts get a flag");
}

/// **Not a check — the pixels of every flag for the eye of the controller (C3 of TZ E119).**
/// Writes `E119_FLAGS_OUT` (a JSON file): every flag and two plates at 16, 20, 24 and 32 on a
/// light and a dark bar, and the states of three flags. Run by hand:
/// `cargo test --test flags dump_the_flags_for_review -- --ignored`.
#[test]
#[ignore = "writes a file for the eye, not a check"]
fn dump_the_flags_for_review() {
    let Ok(out) = std::env::var("E119_FLAGS_OUT") else {
        return;
    };

    fn base64(bytes: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    s.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }

    let mut rows = Vec::new();
    let mut push = |name: &str, face: &Face, look: Look, state: &str| {
        let rgba: Vec<u8> = flags::render(face, look)
            .iter()
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect();
        rows.push(format!(
            "{{\"c\":\"{name}\",\"s\":{},\"bar\":\"{}\",\"st\":\"{state}\",\"px\":\"{}\"}}",
            look.size,
            if look.dark_bar { "dark" } else { "light" },
            base64(&rgba)
        ));
    };

    let mut faces: Vec<(String, Face)> = FLAGS
        .iter()
        .enumerate()
        .map(|(i, f)| (f.country.to_owned(), Face::Flag(i)))
        .collect();
    faces.push(("plate EO".to_owned(), Face::Plate("EO".to_owned())));
    faces.push(("plate HA".to_owned(), Face::Plate("HA".to_owned())));

    for (name, face) in &faces {
        for size in [16, 20, 24, 32] {
            for dark_bar in [false, true] {
                push(
                    name,
                    face,
                    Look {
                        size,
                        paused: false,
                        unread: false,
                        dark_bar,
                    },
                    "plain",
                );
            }
        }
    }

    for code in ["RU", "US", "DE"] {
        let face = flag(code);
        for size in [16, 20, 24, 32] {
            for dark_bar in [false, true] {
                push(
                    code,
                    &face,
                    Look {
                        size,
                        paused: true,
                        unread: false,
                        dark_bar,
                    },
                    "paused",
                );
                push(
                    code,
                    &face,
                    Look {
                        size,
                        paused: false,
                        unread: true,
                        dark_bar,
                    },
                    "unread",
                );
                push(
                    code,
                    &face,
                    Look {
                        size,
                        paused: true,
                        unread: true,
                        dark_bar,
                    },
                    "paused+unread",
                );
            }
        }
    }

    std::fs::write(&out, format!("[{}]", rows.join(",\n"))).expect("the dump is written");
    println!("{} images into {out}", rows.len());
}

// =========================================================================================
// T-119-4 — knowing the layout without a timer
// =========================================================================================

/// **The keys of Windows that switch a layout, and only they, make the flag re-read** — the rule
/// of `flags::ends_a_switch_by_keys`. Red before: the function did not exist.
#[test]
fn only_the_keys_that_switch_a_layout_make_the_flag_re_read() {
    use flags::ends_a_switch_by_keys as ends;

    // Alt+Shift and Ctrl+Shift, released in either order; Win+Space; Win alone.
    assert!(
        ends(0xA0, true, false, false, false),
        "Shift let go with Alt held"
    );
    assert!(
        ends(0xA1, false, true, false, false),
        "right Shift let go with Ctrl held"
    );
    assert!(
        ends(0xA4, false, false, true, false),
        "Alt let go with Shift held"
    );
    assert!(
        ends(0xA3, false, false, true, false),
        "Ctrl let go with Shift held"
    );
    assert!(
        ends(0x20, false, false, false, true),
        "Space let go with Win held"
    );
    assert!(ends(0x5B, false, false, false, false), "Win let go");

    // Typing.
    assert!(
        !ends(0xA0, false, false, false, false),
        "a capital letter's Shift"
    );
    assert!(
        !ends(0x20, false, false, false, false),
        "a space between words"
    );
    assert!(
        !ends(0x41, true, true, true, true),
        "a letter, whatever is held"
    );
    assert!(
        !ends(0xA4, false, true, false, false),
        "Alt let go with Ctrl but no Shift"
    );
}

/// **The list of layouts closes — the hide or the destruction of its own window, nothing else.**
#[test]
fn the_list_closes_on_the_hide_or_destruction_of_its_window_only() {
    use flags::{LIST_WINDOW_CLASS, closes_the_list};

    const DESTROY: u32 = 0x8001;
    const SHOW: u32 = 0x8002;
    const HIDE: u32 = 0x8003;
    const CLOAKED: u32 = 0x8017;

    assert!(closes_the_list(HIDE, 0, 0, LIST_WINDOW_CLASS));
    assert!(closes_the_list(DESTROY, 0, 0, LIST_WINDOW_CLASS));
    assert!(
        !closes_the_list(SHOW, 0, 0, LIST_WINDOW_CLASS),
        "the list opening"
    );
    assert!(
        !closes_the_list(CLOAKED, 0, 0, LIST_WINDOW_CLASS),
        "born cloaked too"
    );
    assert!(
        !closes_the_list(HIDE, -4, 0, LIST_WINDOW_CLASS),
        "a part of it, not the window"
    );
    assert!(
        !closes_the_list(HIDE, 0, 3, LIST_WINDOW_CLASS),
        "a child, not the window"
    );
    assert!(
        !closes_the_list(HIDE, 0, 0, "Shell_TrayWnd"),
        "another window of explorer"
    );
}

/// **The guard of «флаг выключен — нового кода нет в работе», on the source:** every post of the
/// flag's messages from the watcher, the Raw Input and the hook stands behind `flags::is_on()` in
/// the very condition that guards it.
///
/// ⚠ Control: a post with the condition taken off is found.
#[test]
fn every_post_of_the_flag_stands_behind_the_switch() {
    fn ungated(text: &str) -> Vec<String> {
        let lines: Vec<&str> = text.lines().collect();
        let mut found = Vec::new();

        for (n, line) in lines.iter().enumerate() {
            if line.contains("post_to_ui_thread(crate::flags::WM_APP_FLAG_") {
                let guard = lines[..n]
                    .iter()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&"");
                if !(guard.trim_start().starts_with("if ")
                    && guard.contains("crate::flags::is_on()"))
                {
                    found.push(format!("{}: {}", n + 1, line.trim()));
                }
            }
        }

        found
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut posts = 0;

    for file in ["watchdog.rs", "hook.rs"] {
        let text = std::fs::read_to_string(root.join(file)).expect("a source reads");
        posts += text
            .matches("post_to_ui_thread(crate::flags::WM_APP_FLAG_")
            .count();
        let bad = ungated(&text);
        assert!(
            bad.is_empty(),
            "{file}: a post of the flag with the flag off:\n{}",
            bad.join("\n")
        );
    }

    assert_eq!(
        posts, 3,
        "the window, the mouse released, the keys released"
    );

    let control = "    if edge == Edge::Up {\n        let _ = crate::app::post_to_ui_thread(crate::flags::WM_APP_FLAG_LATER);\n";
    assert_eq!(
        ungated(control).len(),
        1,
        "the control: a post with the switch taken off is found"
    );
}

/// **The console (FR-52 is blind there): the name of the input indicator names a language, and the
/// first layout of the session of that language stands for it** — `flags::layout_named_in`.
#[test]
fn the_name_of_the_indicator_finds_the_layout_of_its_language() {
    use lang_switcher::layouts::LayoutId;

    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let ru_typewriter = LayoutId::from_raw(0xF008_0419);
    let names = |layout: LayoutId| -> Option<String> {
        Some(match layout.language_id() {
            0x0409 => "Английский".to_owned(),
            0x0419 => "Русский".to_owned(),
            _ => return None,
        })
    };

    let session = [us, ru, ru_typewriter];
    assert_eq!(
        flags::layout_named_in(
            "Индикатор ввода в области уведомлений - Русский",
            &session,
            names
        ),
        Some(ru),
        "Windows 10: the first layout of the language"
    );
    assert_eq!(
        flags::layout_named_in(
            "Индикатор ввода в области уведомлений - Английский (США)",
            &session,
            names
        ),
        Some(us)
    );
    assert_eq!(
        flags::layout_named_in("Lang Switcher", &session, names),
        None,
        "another icon"
    );
}
