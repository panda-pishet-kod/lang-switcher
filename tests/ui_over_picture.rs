//! What lies over the picture of the panel (stage Э120, T-120-25 p. 3): the word and the keys of the
//! example of «Привет» in the named places of `hello.svg` — where the mock-up puts them
//! (`helloOverlayReal` of `letters.html`, its numbers written here by hand), at two sizes of the
//! picture, at 100 % and 200 %, not mirrored in a window that reads right to left; in one column the
//! row stands in the flow instead.

use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::layout::{Class, Layout, Part, WindowSize, lay_out};
use lang_switcher::ui::scene::{Art, Node, OnPicture, Window, WindowKind};
use lang_switcher::ui::text::FixedAdvance;
use lang_switcher::ui::tokens::{ART_RING, Role, TYPE_FLOOR};

/// Every character is half the type size wide.
const MEASURE: FixedAdvance = FixedAdvance(0.5);

// The mock-up, in the field of hello.svg (400 × 400): HELLO_REAL of letters.html.
const FIELD: f32 = 400.0;
const SCREEN_MIDDLE: f32 = 154.0 + 94.0 / 2.0;
const WORD_X: f32 = 204.0;
const WORD_SIZE: f32 = 24.0;
const KEY_CENTRE: f32 = 268.0;
const KEY_Y: f32 = 282.0;
const KEY_H: f32 = 30.0;
const KEY_LABEL: f32 = 16.0;
const KEY_GAP: f32 = 10.0;

fn letter(over: Vec<(&'static str, OnPicture)>) -> Window<u8> {
    Window {
        kind: WindowKind::Ordinary,
        art: Some(Art::Hello),
        brand: None,
        body: Node::Column(vec![
            Node::Text(Role::Title, "Привет".to_owned()),
            Node::OverPicture {
                over,
                otherwise: Box::new(Node::Example {
                    typed: "ghbdtn".to_owned(),
                    key: "Pause".to_owned(),
                    fixed: "привет".to_owned(),
                }),
            },
            Node::Text(Role::Body, "The text of the letter".to_owned()),
        ]),
        buttons: Vec::new(),
        disabled: Vec::new(),
    }
}

fn example(
    word: &str,
    keys: &[&str],
    fixed: bool,
    pressed: bool,
) -> Vec<(&'static str, OnPicture)> {
    vec![
        (
            "screen",
            OnPicture::Word {
                word: word.to_owned(),
                fixed,
            },
        ),
        (
            "key",
            OnPicture::Keys {
                labels: keys.iter().map(|key| (*key).to_owned()).collect(),
                pressed,
            },
        ),
    ]
}

/// The usual window and a large one: two sizes of the picture.
fn sizes() -> [WindowSize; 2] {
    [
        WindowSize {
            w: 900.0,
            h: 540.0,
            class: Class::Ordinary,
            scrolls: false,
        },
        WindowSize {
            w: 1400.0,
            h: 800.0,
            class: Class::Large,
            scrolls: false,
        },
    ]
}

fn picture(layout: &Layout<u8>) -> Rect {
    layout
        .items
        .iter()
        .find(|item| matches!(item.part, Part::Art(_)))
        .expect("the picture of the panel")
        .rect
}

/// The top left corner of the field of the picture in the window and the points of one unit of it:
/// the field fitted to the square around the circle inside the ring, as the canvas draws it.
fn field(circle: Rect, ring: f32) -> (f32, f32, f32) {
    let radius = circle.w / 2.0 - ring;
    (
        circle.x + circle.w / 2.0 - radius,
        circle.y + circle.h / 2.0 - radius,
        radius * 2.0 / FIELD,
    )
}

fn over(layout: &Layout<u8>) -> Vec<(Part, Rect)> {
    layout
        .items
        .iter()
        .filter(|item| item.part.lies_over_picture())
        .map(|item| (item.part.clone(), item.rect))
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// **The word and the key stand where the mock-up puts them, at both sizes of the picture** — the
/// word from x 204 of the field, in the middle of the screen, at 24 units of it; the key centred on
/// x 268, from y 282, 30 high, its name at 16 units — never under the floor of 12 points.
#[test]
fn the_word_and_the_key_stand_where_the_mock_up_puts_them() {
    for size in sizes() {
        let layout = lay_out(
            &letter(example("ghbdtn", &["Pause"], false, false)),
            size,
            1.0,
            false,
            &MEASURE,
        );
        let (left, top, unit) = field(picture(&layout), ART_RING.0);
        let placed = over(&layout);
        assert_eq!(placed.len(), 2, "{}: {placed:?}", size.w);

        let (
            Part::PictureWord {
                word,
                size: word_size,
                fixed,
            },
            at,
        ) = &placed[0]
        else {
            panic!("the word first: {placed:?}");
        };
        assert_eq!((word.as_str(), *fixed), ("ghbdtn", false));
        assert!(
            close(at.x, left + WORD_X * unit),
            "{}: the word at {at:?}",
            size.w
        );
        assert!(
            close(at.y + at.h / 2.0, top + SCREEN_MIDDLE * unit),
            "{}: the word not in the middle of the screen: {at:?}",
            size.w
        );
        assert!(close(*word_size, (WORD_SIZE * unit).max(TYPE_FLOOR.0)));

        let (
            Part::PictureKey {
                label,
                size: label_size,
                pressed,
            },
            key,
        ) = &placed[1]
        else {
            panic!("the key second: {placed:?}");
        };
        assert_eq!((label.as_str(), *pressed), ("Pause", false));
        assert!(
            close(key.x + key.w / 2.0, left + KEY_CENTRE * unit),
            "{}: {key:?}",
            size.w
        );
        assert!(close(key.y, top + KEY_Y * unit), "{}: {key:?}", size.w);
        assert!(close(key.h, KEY_H * unit), "{}: {key:?}", size.w);
        assert!(close(*label_size, (KEY_LABEL * unit).max(TYPE_FLOOR.0)));
    }
}

/// Two keys («Shift, Shift») stand side by side, alike, 10 units apart, the pair centred on the key
/// of the picture; the states the window gives are what is drawn.
#[test]
fn two_keys_stand_side_by_side_in_the_middle() {
    let size = sizes()[1];
    let layout = lay_out(
        &letter(example("привет", &["Shift", "Shift"], true, true)),
        size,
        1.0,
        false,
        &MEASURE,
    );
    let (left, _, unit) = field(picture(&layout), ART_RING.0);
    let placed = over(&layout);
    let keys: Vec<Rect> = placed
        .iter()
        .filter_map(|(part, rect)| {
            matches!(part, Part::PictureKey { pressed: true, .. }).then_some(*rect)
        })
        .collect();

    assert!(matches!(placed[0].0, Part::PictureWord { fixed: true, .. }));
    assert_eq!(keys.len(), 2, "{placed:?}");
    assert!(close(keys[0].w, keys[1].w));
    assert!(
        close(keys[1].x - keys[0].right(), KEY_GAP * unit),
        "{keys:?}"
    );
    assert!(
        close(
            (keys[0].x + keys[1].right()) / 2.0,
            left + KEY_CENTRE * unit
        ),
        "{keys:?}"
    );
}

/// **Neither the picture nor what lies on it is mirrored**: in a window that reads right to left
/// the picture moves to the other side with its panel, and the word and the key stay where they are
/// on it — the same distance from its left edge as in the straight window.
#[test]
fn what_lies_on_the_picture_is_not_mirrored() {
    let window = letter(example("ghbdtn", &["Ctrl", "Shift"], false, false));
    for size in sizes() {
        let straight = lay_out(&window, size, 1.0, false, &MEASURE);
        let mirrored = lay_out(&window, size, 1.0, true, &MEASURE);
        let (from, to) = (picture(&straight), picture(&mirrored));
        assert!(
            close(to.x, size.w - from.x - from.w),
            "the picture moves with its panel"
        );

        let (before, after) = (over(&straight), over(&mirrored));
        assert_eq!(before.len(), after.len());
        for ((part, a), (mirrored_part, b)) in before.iter().zip(&after) {
            assert_eq!(part, mirrored_part);
            assert!(
                close(b.x - to.x, a.x - from.x) && close(b.y, a.y),
                "{} at {}: {b:?}, straight {a:?}",
                part.name(),
                size.w
            );
        }
        // The keys keep their order on the picture.
        let ctrl = after.iter().position(
            |(part, _)| matches!(part, Part::PictureKey { label, .. } if label == "Ctrl"),
        );
        let shift = after.iter().position(
            |(part, _)| matches!(part, Part::PictureKey { label, .. } if label == "Shift"),
        );
        let (ctrl, shift) = (ctrl.expect("Ctrl"), shift.expect("Shift"));
        assert!(
            after[ctrl].1.x < after[shift].1.x,
            "Ctrl stays left of Shift"
        );
    }
}

/// **Over the picture it takes no room**: the text after it stands where it stands without it. In
/// one column there is no panel — the row of the example stands in the flow instead, and nothing
/// lies over the circle beside the header.
#[test]
fn over_the_picture_it_takes_no_room_and_in_one_column_it_is_a_row() {
    let with = letter(example("ghbdtn", &["Pause"], false, false));
    let mut without = with.clone();
    if let Node::Column(nodes) = &mut without.body {
        nodes.remove(1);
    }
    let texts = |layout: &Layout<u8>| -> Vec<Rect> {
        layout
            .items
            .iter()
            .filter(|item| matches!(item.part, Part::Text { .. }))
            .map(|item| item.rect)
            .collect()
    };

    for size in sizes() {
        let (a, b) = (
            lay_out(&with, size, 1.0, false, &MEASURE),
            lay_out(&without, size, 1.0, false, &MEASURE),
        );
        assert_eq!(texts(&a), texts(&b), "{}: the flow moved", size.w);
    }

    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let layout = lay_out(&with, narrow, 1.0, false, &MEASURE);
    assert!(
        over(&layout).is_empty(),
        "nothing over the circle: {:?}",
        over(&layout)
    );
    assert!(
        layout.items.iter().any(|item| item.part
            == Part::Key {
                label: "Pause".to_owned()
            }),
        "the row of the example in the flow"
    );
}

#[cfg(feature = "testing")]
mod live {
    use super::{FIELD, KEY_CENTRE, WORD_X, example, letter};
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::canvas::Rect;
    use lang_switcher::ui::host::{self, Content, Reply};
    use lang_switcher::ui::scene::{Event, Window};
    use lang_switcher::ui::tokens::ART_RING;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SendMessageW, WM_CLOSE, WM_DPICHANGED, WM_PAINT,
    };

    struct Hello;

    impl Content for Hello {
        type Id = u8;

        fn describe(&self) -> Window<u8> {
            letter(example("ghbdtn", &["Pause"], false, false))
        }

        fn event(&mut self, _: Event<u8>) -> Reply {
            Reply::Stay
        }

        fn title(&self) -> String {
            "Hello".to_owned()
        }

        fn language(&self) -> (String, bool) {
            ("en-US".to_owned(), false)
        }

        fn theme(&self) -> ThemeSetting {
            ThemeSetting::Light
        }
    }

    fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) {
        // SAFETY: a window of this thread; the message goes straight to its procedure.
        unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))) };
    }

    /// **At 100 % and at 200 % the word and the key are drawn on their places of the picture** — in
    /// the pixels of the window the host lays out and paints (the drawing of both parts runs).
    #[test]
    fn the_places_hold_at_100_and_200_percent() {
        let hwnd = host::create(Hello, None).expect("the window of the shell");
        send(hwnd, WM_PAINT, 0, 0);
        let mut first = RECT::default();
        // SAFETY: the out parameter is a local.
        unsafe { GetWindowRect(hwnd, &mut first) }.expect("the window's rectangle");

        for (dpi, scale) in [(96usize, 1_i32), (192, 2)] {
            // The rectangle Windows suggests: the same size in points, so the panel stays.
            let window = RECT {
                right: first.left + (first.right - first.left) * scale,
                bottom: first.top + (first.bottom - first.top) * scale,
                ..first
            };
            let scale = scale as f32;
            send(
                hwnd,
                WM_DPICHANGED,
                dpi | (dpi << 16),
                (&raw const window) as isize,
            );
            send(hwnd, WM_PAINT, 0, 0);
            let probe = host::probe(hwnd).expect("alive");
            assert_eq!(probe.scale, scale);

            let find = |kind: &str| -> Rect {
                probe
                    .items
                    .iter()
                    .find(|(part, _, _)| part.starts_with(kind))
                    .unwrap_or_else(|| panic!("no {kind} at {dpi}: {:?}", probe.items))
                    .2
            };
            let circle = find("Art(Hello)");
            let radius = circle.w / 2.0 - ART_RING.0 * scale;
            let (left, unit) = (circle.x + circle.w / 2.0 - radius, radius * 2.0 / FIELD);
            let (word, key) = (find("PictureWord"), find("PictureKey"));

            assert!(
                (word.x - (left + WORD_X * unit)).abs() < 0.05,
                "{dpi}: {word:?}"
            );
            assert!(
                (key.x + key.w / 2.0 - (left + KEY_CENTRE * unit)).abs() < 0.05,
                "{dpi}: {key:?}"
            );
        }
        send(hwnd, WM_CLOSE, 0, 0);
    }
}
