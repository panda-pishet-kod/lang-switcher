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

/// **Windows 11 closes the list without hiding its window** (stand T-119-4, 2026-11-20, 26300): at
/// every close — `Enter` after the arrows, a click on a row, `Esc` — `Shell_InputSwitchTopLevelWindow`
/// gets a focus event and nothing else, while the overlay the shell puts behind the list,
/// `Shell_InputSwitchDismissOverlay`, is hidden and destroyed (4 closes of 4, `evmon` logs). The
/// choice by the mouse was caught by the release of the button; the choice by the arrows and
/// `Enter` by nothing — the flag kept the old layout 12 times of 12. The overlay's hide is the
/// sign of the list closing there; its birth and its showing are not.
#[test]
fn the_list_closes_on_windows_11_by_its_overlay() {
    use flags::{LIST_OVERLAY_CLASS, closes_the_list};

    const CREATE: u32 = 0x8000;
    const DESTROY: u32 = 0x8001;
    const SHOW: u32 = 0x8002;
    const HIDE: u32 = 0x8003;

    assert_eq!(LIST_OVERLAY_CLASS, "Shell_InputSwitchDismissOverlay");
    assert!(closes_the_list(HIDE, 0, 0, LIST_OVERLAY_CLASS));
    assert!(closes_the_list(DESTROY, 0, 0, LIST_OVERLAY_CLASS));
    assert!(
        !closes_the_list(SHOW, 0, 0, LIST_OVERLAY_CLASS),
        "the list opening"
    );
    assert!(
        !closes_the_list(CREATE, 0, 0, LIST_OVERLAY_CLASS),
        "the list opening"
    );
    assert!(
        !closes_the_list(HIDE, -4, 0, LIST_OVERLAY_CLASS),
        "a part of it, not the window"
    );
    assert!(
        !closes_the_list(
            HIDE,
            0,
            0,
            "Windows.UI.Composition.DesktopWindowContentBridge"
        ),
        "a window of XAML — every flyout of explorer has one"
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

/// **The flag itself is never taken for the input indicator** (task T-119-3). Since T-119-2 the
/// tooltip of the flag — the name of its button on the taskbar of Windows 11 — begins with the
/// language of the layout, «Русский (Россия)», and every button of the tray there has the same
/// automation id: on a Windows whose interface speaks the program's language the search for the
/// indicator would find the flag first (it stands left of РУС/ENG) — the console would read the
/// flag's own layout back, and a click on the flag would press the flag again.
#[test]
fn the_flag_is_not_taken_for_the_indicator() {
    use lang_switcher::layouts::LayoutId;

    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let names = |layout: LayoutId| -> Option<String> {
        Some(match layout.language_id() {
            0x0409 => "Английский".to_owned(),
            0x0419 => "Русский".to_owned(),
            _ => return None,
        })
    };
    let session = [us, ru];

    assert_eq!(
        flags::layout_named_in(
            "Русский (Россия)\nLang Switcher — работает",
            &session,
            names
        ),
        None,
        "the flag's own tooltip, whatever language it opens with"
    );
}

/// **After a list the program opened has closed — what to give the window** (task T-119-3,
/// решение 208.164): the layout chosen in the list, only where it did not arrive by itself.
/// Edge does not get a layout chosen in a list the program opened (0 of 20 on each Windows);
/// Блокнот and cmd do (20 of 20) — there nothing is switched a second time, nothing sounds.
#[test]
fn the_window_gets_the_chosen_layout_only_where_it_did_not_arrive() {
    use lang_switcher::layouts::LayoutId;

    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let none = LayoutId::from_raw(0);

    assert_eq!(
        flags::layout_to_give(us, ru, ru, false),
        None,
        "arrived by itself (Блокнот)"
    );
    assert_eq!(
        flags::layout_to_give(us, ru, us, false),
        Some(ru),
        "did not arrive (Edge): the program's own switch"
    );
    assert_eq!(
        flags::layout_to_give(us, us, us, false),
        None,
        "Esc, a click aside, the same row: nothing was chosen"
    );
    assert_eq!(
        flags::layout_to_give(us, ru, us, true),
        None,
        "a console — the reading is blind, and the choice arrives there by itself"
    );
    assert_eq!(
        flags::layout_to_give(us, none, us, false),
        None,
        "the choice could not be read: nothing is guessed"
    );
    assert_eq!(
        flags::layout_to_give(us, ru, none, false),
        None,
        "the window could not be read: nothing is guessed"
    );
}

/// **The window the focus goes back to is the person's, never a part of the shell** (task
/// T-119-3): a click on the flag puts the taskbar in front, and the list itself is a window of
/// explorer — neither is where the person was typing.
#[test]
fn the_shell_is_not_the_window_the_person_typed_in() {
    for shell in [
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "NotifyIconOverflowWindow",
        "TopLevelWindowForOverflowXamlIsland",
        "Shell_InputSwitchTopLevelWindow",
        "Shell_InputSwitchDismissOverlay",
        "Xaml_WindowedPopupClass",
        "Windows.UI.Core.CoreWindow",
        "#32768",
        "Progman",
        "WorkerW",
    ] {
        assert!(flags::is_shell_window_class(shell), "{shell} is the shell");
    }

    for person in [
        "Notepad",
        "Chrome_WidgetWin_1",
        "ConsoleWindowClass",
        "CASCADIA_HOSTING_WINDOW_CLASS",
        "#32770",
    ] {
        assert!(
            !flags::is_shell_window_class(person),
            "{person} is a window a person types in"
        );
    }
}

/// **The list of the program's own (T-119-3, part Б): what its choice gives the window.** Nothing of
/// it reaches the window by itself — the chosen layout is given unless the window is in it, and a
/// console is given it too (its reading is blind; the choice of Windows' list reached it by itself,
/// this one does not).
#[test]
fn the_choice_in_the_own_list_is_given_by_the_program() {
    use lang_switcher::layouts::LayoutId;

    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let none = LayoutId::from_raw(0);

    assert_eq!(flags::own_layout_to_give(ru, us, false), Some(ru));
    assert_eq!(
        flags::own_layout_to_give(ru, ru, false),
        None,
        "already in it: no second switch"
    );
    assert_eq!(
        flags::own_layout_to_give(ru, none, true),
        Some(ru),
        "a console: given"
    );
    assert_eq!(
        flags::own_layout_to_give(none, us, false),
        None,
        "nothing chosen"
    );
}

/// ⭐ **After a choice in the own list, a console shows the layout given** — defect 3 of the stand
/// (2026-11-20, Windows 11, cmd): the flag stayed on the old layout (row 0 of 4, keys 0 of 3) — a
/// console reads blind and РУС/ENG, whose name would tell it, is hidden whenever this list is open.
#[test]
fn a_console_shows_the_layout_the_own_list_gave_it() {
    use lang_switcher::switch::{Outcome, SwitchError};

    assert!(
        flags::flag_shows_the_given(true, true, &Ok(Outcome::Sent)),
        "a console of the own list: the flag shows what was sent"
    );
    assert!(flags::flag_shows_the_given(
        true,
        true,
        &Ok(Outcome::Switched)
    ));
    assert!(
        !flags::flag_shows_the_given(true, false, &Ok(Outcome::Switched)),
        "a window that reads is read back as before"
    );
    assert!(
        !flags::flag_shows_the_given(false, true, &Ok(Outcome::Sent)),
        "the list of Windows: the indicator is there to read"
    );
    assert!(
        !flags::flag_shows_the_given(true, true, &Err(SwitchError::NoForeground)),
        "nothing sent, nothing shown"
    );
    assert!(!flags::flag_shows_the_given(
        true,
        true,
        &Ok(Outcome::AlreadyActive)
    ));
}

/// **The geometry of the own list — the mock-up's numbers at 96 DPI, scaled with the monitor.**
#[test]
fn the_own_list_has_the_size_of_the_mock_up_and_scales() {
    let at_100 = flags::own_geometry(2, 96);

    assert_eq!(at_100.width, 300, "the mock-up's width");
    assert_eq!(
        at_100.row, 52,
        "two lines: the language and the keyboard layout"
    );
    assert_eq!(at_100.code, 44);
    assert_eq!(
        at_100.height,
        6 + 25 + 2 * 52 + 9 + 32 + 6,
        "padding, title, two rows, separator, link, padding"
    );

    let at_150 = flags::own_geometry(2, 144);

    assert_eq!(at_150.width, 450);
    assert_eq!(at_150.row, 78);
    assert!(
        flags::own_geometry(5, 96).height > at_100.height,
        "more layouts, a taller list"
    );
    assert_eq!(flags::own_geometry(0, 96), flags::own_geometry(1, 96));
}

/// **Where the own list stands: beside the flag, inside the work area, on every side of the
/// taskbar** — the requirement «у значка на любой стороне панели задач … не выходя за рабочую
/// область».
#[test]
fn the_own_list_stands_beside_the_flag_inside_the_work_area() {
    use windows::Win32::Foundation::RECT;

    let rect = |left, top, right, bottom| RECT {
        left,
        top,
        right,
        bottom,
    };
    let size = (300, 160);
    let inside = |(x, y): (i32, i32), work: RECT| {
        x >= work.left && y >= work.top && x + size.0 <= work.right && y + size.1 <= work.bottom
    };

    // The taskbar below (1280×800, the bar 48 high): above the icon, the right edges together.
    let work = rect(0, 0, 1280, 752);
    let icon = rect(1054, 752, 1086, 800);
    let at = flags::own_place(icon, work, size, 8);
    assert_eq!(at, (1086 - 300, 752 - 8 - 160));
    assert!(inside(at, work));

    // An icon at the very right: pulled inside.
    let at = flags::own_place(rect(1250, 752, 1280, 800), work, size, 8);
    assert_eq!(at.0, 1280 - 300);
    assert!(inside(at, work));

    // The taskbar at the top: below the icon.
    let work = rect(0, 48, 1280, 800);
    let at = flags::own_place(rect(1054, 0, 1086, 48), work, size, 8);
    assert_eq!(at.1, 48 + 8);
    assert!(inside(at, work));

    // On the right: left of the icon.
    let work = rect(0, 0, 1232, 800);
    let at = flags::own_place(rect(1232, 700, 1280, 732), work, size, 8);
    assert_eq!(at.0, 1232 - 8 - 300);
    assert!(inside(at, work));

    // On the left: right of the icon.
    let work = rect(48, 0, 1280, 800);
    let at = flags::own_place(rect(0, 700, 48, 732), work, size, 8);
    assert_eq!(at.0, 48 + 8);
    assert!(inside(at, work));

    // An icon at the top of a short screen: never above the work area.
    let work = rect(0, 0, 1280, 120);
    let at = flags::own_place(rect(1054, 120, 1086, 168), work, size, 8);
    assert!(at.1 >= 0, "the top stays on the screen");
}

/// **The keyboard in the own list — as in the list of Windows:** arrows, Home and End, no wrapping.
#[test]
fn the_arrows_move_the_mark_without_wrapping() {
    const UP: u16 = 0x26;
    const DOWN: u16 = 0x28;
    const HOME: u16 = 0x24;
    const END: u16 = 0x23;

    assert_eq!(flags::own_step(0, 3, DOWN), 1);
    assert_eq!(flags::own_step(2, 3, DOWN), 2, "no wrapping at the end");
    assert_eq!(flags::own_step(0, 3, UP), 0, "no wrapping at the start");
    assert_eq!(flags::own_step(2, 3, UP), 1);
    assert_eq!(flags::own_step(1, 3, HOME), 0);
    assert_eq!(flags::own_step(0, 3, END), 2);
    assert_eq!(
        flags::own_step(1, 3, 0x41),
        1,
        "another key leaves the mark"
    );
}

/// **A point of the own list falls on a row, on the link, or on nothing.**
#[test]
fn a_point_of_the_own_list_falls_on_a_row_or_the_link() {
    let geometry = flags::own_geometry(2, 96);
    let first = 6 + 25 + 5;
    let second = 6 + 25 + 52 + 5;
    let link = 6 + 25 + 2 * 52 + 9 + 5;

    assert_eq!(flags::own_hit(2, &geometry, 100, first), Some(0));
    assert_eq!(flags::own_hit(2, &geometry, 100, second), Some(1));
    assert_eq!(flags::own_hit(2, &geometry, 100, link), Some(usize::MAX));
    assert_eq!(flags::own_hit(2, &geometry, 100, 10), None, "the title");
    assert_eq!(flags::own_hit(2, &geometry, 2, first), None, "the padding");
}

/// **The rows of the own list are the layouts of the session in their order, coded by language.**
#[test]
fn the_rows_of_the_own_list_are_the_layouts_of_the_session() {
    use lang_switcher::layouts::LayoutId;

    let us = LayoutId::from_raw(0x0409_0409);
    let ru = LayoutId::from_raw(0x0419_0419);
    let rows = flags::own_rows(
        &[us, ru],
        &[
            "English (United States)".to_owned(),
            "Русский (Россия)".to_owned(),
        ],
        flags::language_code,
        |layout| lang_switcher::layouts::display_name(layout).unwrap_or_default(),
    );

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].layout, us);
    assert_eq!(rows[0].code, "ENG");
    assert_eq!(rows[1].code, "RUS");
    assert_eq!(rows[1].name, "Русский (Россия)");

    // The second line is the name Windows gives the keyboard layout (решение по вопросу 82,
    // 2026-11-20): every Windows has US and Russian, in the language of its interface.
    println!(
        "keyboard layouts: {:?} / {:?}",
        rows[0].layout_name, rows[1].layout_name
    );
    assert!(
        !rows[0].layout_name.is_empty() && !rows[1].layout_name.is_empty(),
        "Windows names both keyboard layouts"
    );
    assert_ne!(
        rows[0].layout_name, rows[1].layout_name,
        "and tells them apart"
    );
}
