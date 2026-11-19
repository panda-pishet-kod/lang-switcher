//! The canvas of the interface shell, `src\ui\canvas.rs` (stage Э120, T-120-3), drawn in memory:
//! colours come out as the table writes them, edges are smoothed and flat areas are not, the scale
//! of the screen is applied by the canvas, the plain look draws the same calls without rounding,
//! the factory goes with the last canvas, and a refused target turns a canvas to the plain look.

use lang_switcher::diag;
use lang_switcher::ui::canvas::{self, Canvas, Image, Mode, Rect};
use lang_switcher::ui::text::{Align, Direction, Style, Text, Wrap};
use lang_switcher::ui::tokens::{Color, RADIUS_MEDIUM, Rgb, Role, Theme};
use windows::Win32::Foundation::HWND;

/// The test picture: 100 × 60 points of `bg`, a rectangle of 80 × 40 points at 10, 10 rounded
/// by the medium radius in `fill`.
const AREA: (f32, f32) = (100.0, 60.0);
const SHAPE: Rect = Rect::new(10.0, 10.0, 80.0, 40.0);

fn draw(scale: f32, mode: Mode, theme: Theme, fill: Color) -> Image {
    let mut canvas = Canvas::in_memory(AREA.0, AREA.1, scale, mode).expect("a canvas in memory");
    let drawn = canvas.frame(|painter| {
        painter.clear(Color::Bg.get(theme));
        painter.fill_rect(SHAPE, RADIUS_MEDIUM.0, fill.get(theme));
    });
    assert_eq!(
        drawn, mode,
        "the frame was drawn the way the canvas was made"
    );
    canvas.image().expect("a canvas in memory gives its pixels")
}

/// The pixel at a point given in points, on an image drawn at `scale`.
fn at(image: &Image, scale: f32, x: f32, y: f32) -> Rgb {
    image.at((x * scale) as u32, (y * scale) as u32)
}

/// How many pixels of the middle row of the shape are exactly `fill`.
fn run_across(image: &Image, scale: f32, fill: Rgb) -> u32 {
    let row = ((SHAPE.y + SHAPE.h / 2.0) * scale) as u32;
    (0..image.width)
        .filter(|x| image.at(*x, row) == fill)
        .count() as u32
}

#[test]
fn the_fill_at_the_centre_of_a_shape_is_the_colour_of_the_table_in_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        for fill in [Color::Surface2, Color::Accent, Color::Button] {
            let image = draw(1.0, Mode::Direct2D, theme, fill);

            assert_eq!(
                at(&image, 1.0, 50.0, 30.0),
                fill.get(theme),
                "{theme:?} {}",
                fill.name()
            );
            assert_eq!(
                at(&image, 1.0, 2.0, 2.0),
                Color::Bg.get(theme),
                "{theme:?} background"
            );
        }
    }
}

#[test]
fn a_rounded_edge_is_smoothed_and_the_centre_and_the_background_are_not() {
    let (bg, fill) = (Color::Bg.get(Theme::Light), Color::Accent.get(Theme::Light));
    let image = draw(1.0, Mode::Direct2D, Theme::Light, Color::Accent);

    // The corner of the shape is cut by the radius: its pixels on the diagonal run from the
    // background through mixed values into the fill.
    let diagonal: Vec<Rgb> = (0..12)
        .map(|step| at(&image, 1.0, SHAPE.x + step as f32, SHAPE.y + step as f32))
        .collect();
    let mixed = diagonal
        .iter()
        .filter(|pixel| **pixel != bg && **pixel != fill)
        .count();
    assert!(mixed > 0, "no smoothed pixel on the corner: {diagonal:?}");
    assert_eq!(diagonal[0], bg, "the very corner is cut away by the radius");

    // Flat areas carry no mixed value at all.
    let flat = [
        at(&image, 1.0, 50.0, 30.0),
        at(&image, 1.0, 30.0, 20.0),
        at(&image, 1.0, 1.0, 1.0),
        at(&image, 1.0, 98.0, 58.0),
    ];
    assert_eq!(flat, [fill, fill, bg, bg]);
}

#[test]
fn at_two_hundred_percent_a_shape_is_twice_as_large_in_screen_pixels() {
    let fill = Color::Accent.get(Theme::Light);
    let one = run_across(
        &draw(1.0, Mode::Direct2D, Theme::Light, Color::Accent),
        1.0,
        fill,
    );
    let two = run_across(
        &draw(2.0, Mode::Direct2D, Theme::Light, Color::Accent),
        2.0,
        fill,
    );

    assert_eq!(
        (one, two),
        (80, 160),
        "the shape of {} points is {one} pixels across at 100 % and {two} at 200 %: the canvas \
         must apply the scale of the screen",
        SHAPE.w
    );
}

#[test]
fn the_plain_look_draws_the_same_calls_in_the_same_places_without_rounding() {
    let theme = Theme::Light;
    let (bg, fill) = (Color::Bg.get(theme), Color::Accent.get(theme));

    for scale in [1.0, 2.0] {
        let image = draw(scale, Mode::Gdi, theme, Color::Accent);

        assert_eq!(at(&image, scale, 50.0, 30.0), fill, "centre at {scale}");
        assert_eq!(at(&image, scale, 2.0, 2.0), bg, "background at {scale}");
        // No rounding: the very corner pixel is the fill, where Direct2D cuts it away.
        assert_eq!(
            at(&image, scale, SHAPE.x, SHAPE.y),
            fill,
            "corner at {scale}"
        );
        assert_eq!(
            run_across(&image, scale, fill),
            (SHAPE.w * scale) as u32,
            "width at {scale}"
        );
        // No smoothing either: two colours and nothing between.
        assert!(
            image
                .pixels()
                .iter()
                .all(|pixel| *pixel == bg || *pixel == fill),
            "the plain look mixed colours at {scale}"
        );
    }
}

#[test]
fn a_circle_clip_keeps_what_follows_inside_the_circle() {
    let theme = Theme::Dark;
    let (bg, fill) = (Color::Bg.get(theme), Color::Surface2.get(theme));
    let mut canvas = Canvas::in_memory(AREA.0, AREA.1, 1.0, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(bg);
        painter.clip_circle(50.0, 30.0, 20.0);
        painter.fill_rect(Rect::new(0.0, 0.0, AREA.0, AREA.1), 0.0, fill);
        painter.unclip();
    });
    let image = canvas.image().expect("pixels");

    assert_eq!(image.at(50, 30), fill, "the centre of the circle");
    assert_eq!(
        image.at(33, 13),
        bg,
        "the corner of its square, outside the circle"
    );
    assert_eq!(image.at(5, 5), bg, "far outside");
}

#[test]
fn a_frame_and_a_line_are_drawn_inside_and_on_their_place() {
    let theme = Theme::Light;
    let (bg, edge) = (Color::Bg.get(theme), Color::Edge.get(theme));
    let mut canvas = Canvas::in_memory(AREA.0, AREA.1, 1.0, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(bg);
        painter.stroke_rect(Rect::new(10.0, 10.0, 30.0, 20.0), 0.0, 2.0, edge);
        painter.line(50.0, 40.5, 90.0, 40.5, 1.0, edge);
    });
    let image = canvas.image().expect("pixels");

    // A frame of two points inside the rectangle: its outer pixels and its inner ones.
    assert_eq!(
        [image.at(10, 20), image.at(11, 20), image.at(12, 20)],
        [edge, edge, bg]
    );
    assert_eq!(image.at(9, 20), bg, "nothing outside the rectangle");
    // A line of one point on a half-pixel row fills exactly that row.
    assert_eq!(
        [image.at(70, 39), image.at(70, 40), image.at(70, 41)],
        [bg, edge, bg]
    );
}

#[test]
fn the_pattern_of_a_panel_puts_a_dot_at_the_centre_of_every_cell_and_the_plain_look_none() {
    let theme = Theme::Light;
    let (bg, dot) = (Color::Surface2.get(theme), Color::Line2.get(theme));

    for (mode, expected) in [(Mode::Direct2D, true), (Mode::Gdi, false)] {
        let mut canvas = Canvas::in_memory(72.0, 36.0, 1.0, mode).expect("a canvas");
        canvas.frame(|painter| {
            painter.clear(bg);
            painter.dots(Rect::new(0.0, 0.0, 72.0, 36.0), 4.0, 18.0, dot);
        });
        let image = canvas.image().expect("pixels");

        for (x, y) in [(9, 9), (27, 9), (63, 27)] {
            assert_eq!(
                image.at(x, y) != bg,
                expected,
                "{mode:?}: the cell centred on {x}, {y}"
            );
        }
        assert_eq!(image.at(0, 0), bg, "{mode:?}: between the dots");
    }
}

#[test]
fn the_factory_of_a_thread_goes_with_its_last_canvas() {
    assert!(
        !canvas::factory_is_held(),
        "a fresh thread holds no factory"
    );

    let first = Canvas::in_memory(10.0, 10.0, 1.0, Mode::Direct2D).expect("a canvas");
    let second = Canvas::in_memory(10.0, 10.0, 1.0, Mode::Direct2D).expect("a canvas");
    assert!(canvas::factory_is_held());

    drop(first);
    assert!(
        canvas::factory_is_held(),
        "the second canvas still holds it"
    );
    drop(second);
    assert!(!canvas::factory_is_held(), "released with the last canvas");

    let plain = Canvas::in_memory(10.0, 10.0, 1.0, Mode::Gdi).expect("a canvas");
    assert!(
        !canvas::factory_is_held(),
        "the plain look needs no factory"
    );
    drop(plain);
}

#[test]
fn the_journal_has_a_name_for_each_event_of_the_canvas() {
    for name in [canvas::TARGET_REFUSED, canvas::PLAIN_LOOK] {
        assert_ne!(
            diag::Operation::from_name(name),
            diag::Operation::UNLISTED,
            "«{name}» is not in the vocabulary of src\\diag.rs"
        );
    }
}

/// A window target Direct2D cannot make — here, for no window at all — is refused twice, written
/// to the journal, and the canvas turns to the plain look; no live window is touched.
#[test]
fn a_target_refused_twice_turns_the_canvas_to_the_plain_look_and_says_so() {
    let refused = diag::Operation::from_name(canvas::TARGET_REFUSED);
    let plain = diag::Operation::from_name(canvas::PLAIN_LOOK);
    let count = |operation| {
        diag::snapshot()
            .iter()
            .filter(|event| event.operation == operation)
            .count()
    };
    let (refused_before, plain_before) = (count(refused), count(plain));

    let mut canvas = Canvas::for_window(HWND(std::ptr::null_mut()), 100, 60, 1.0);
    assert_eq!(canvas.mode(), Mode::Direct2D, "Direct2D is tried first");

    let drawn = canvas.frame(|painter| painter.clear(Color::Bg.get(Theme::Light)));

    assert_eq!(drawn, Mode::Gdi);
    assert_eq!(
        canvas.mode(),
        Mode::Gdi,
        "the canvas stays in the plain look"
    );
    assert!(
        count(refused) >= refused_before + usize::from(canvas::REFUSALS_BEFORE_PLAIN_LOOK),
        "two refusals in the journal"
    );
    assert!(count(plain) > plain_before, "the turn in the journal");
}

/// The box of every pixel that is not `bg`, in pixels: left, top, right, bottom.
fn ink(image: &Image, bg: Rgb) -> Option<(u32, u32, u32, u32)> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for y in 0..image.height {
        for x in 0..image.width {
            if image.at(x, y) != bg {
                found = Some(found.map_or((x, y, x, y), |(l, t, r, b)| {
                    (l.min(x), t.min(y), r.max(x), b.max(y))
                }));
            }
        }
    }
    found
}

fn text_drawn(mode: Mode, locale: &str, direction: Direction, line: &str) -> (u32, u32, u32, u32) {
    let theme = Theme::Light;
    let bg = Color::Bg.get(theme);
    let text = Text::new(locale, direction).expect("DirectWrite");
    let laid = text
        .lay(
            line,
            Style::of(Role::Body, 14.0, 512.0),
            Wrap::OneLine,
            Align::Start,
            180.0,
        )
        .expect("a layout");
    let mut canvas = Canvas::in_memory(200.0, 40.0, 1.0, mode).expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(bg);
        painter.text(&laid, 10.0, 10.0, Color::Text.get(theme));
    });
    ink(&canvas.image().expect("pixels"), bg).expect("some ink")
}

/// The plain look draws the same text in the same place: the ink of `DrawTextW` and of Direct2D
/// starts at the same edge of the same box, within a few pixels of each other.
#[test]
fn the_plain_look_draws_text_where_direct2d_does_and_start_is_the_right_edge_in_hebrew() {
    for (locale, direction, line) in [
        ("en-US", Direction::LeftToRight, "Lang Switcher"),
        ("he-IL", Direction::RightToLeft, "מקש קיצור"),
    ] {
        let smooth = text_drawn(Mode::Direct2D, locale, direction, line);
        let plain = text_drawn(Mode::Gdi, locale, direction, line);

        for (name, (left, top, right, bottom)) in [("Direct2D", smooth), ("GDI", plain)] {
            assert!(
                left >= 10 && right < 190 && top >= 10 && bottom < 40,
                "{locale} {name}: {smooth:?}"
            );
            match direction {
                Direction::LeftToRight => {
                    assert!(left <= 12, "{locale} {name} starts at the left: {left}")
                }
                Direction::RightToLeft => {
                    assert!(right >= 186, "{locale} {name} starts at the right: {right}")
                }
            }
        }
        let near = |a: u32, b: u32| a.abs_diff(b) <= 3;
        assert!(
            near(smooth.1, plain.1) && near(smooth.3, plain.3),
            "{locale}: the same lines, Direct2D {smooth:?} and GDI {plain:?}"
        );
    }
}

/// The ring around a picture of the panda is `bg` and 6 points wide: 6 pixels at 100 %, 12 at
/// 200 % (passport §3).
#[test]
fn the_ring_around_a_picture_is_the_background_six_points_wide() {
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::layout::Part;
    use lang_switcher::ui::scene::Art;

    let text = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    let theme = Theme::Light;
    let look = Look {
        theme,
        rtl: false,
        b: 14.0,
        h: 512.0,
        text: &text,
    };
    let bg = Color::Bg.get(theme);

    for scale in [1.0_f32, 2.0] {
        let mut canvas = Canvas::in_memory(100.0, 100.0, scale, Mode::Direct2D).expect("a canvas");
        canvas.frame(|painter| {
            painter.clear(Color::Surface2.get(theme));
            elements::draw(
                painter,
                &look,
                &Part::Art(Art::About),
                Rect::new(10.0, 10.0, 80.0, 80.0),
                State::NORMAL,
            );
        });
        let image = canvas.image().expect("pixels");
        let (row, start) = ((50.0 * scale) as u32, (10.0 * scale) as u32);
        let ring = (start..start + (20.0 * scale) as u32)
            .filter(|x| image.at(*x, row) == bg)
            .count() as f32;

        assert!(
            (ring - 6.0 * scale).abs() <= 1.0,
            "the ring at {scale}: {ring} pixels of bg"
        );
    }
}

/// The picture of the panda through `art.rs`: not the ground of the stand-in, twice as large at
/// 200 % and as sharp as a picture drawn afresh there — the same `Art` drew it at 100 % first.
#[test]
fn a_picture_at_two_hundred_percent_is_twice_as_large_and_as_sharp_as_a_fresh_one() {
    use lang_switcher::ui::art::Art;
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    // `Art` keeps each finished picture with the render target that made it, past the canvas in
    // memory: COM must outlive both, or the release of WIC's objects after the canvas left COM
    // reads freed memory. A window's target is not WIC's; this is a property of the canvases
    // in memory, the tests' own.
    // SAFETY: the apartment of this test's thread, entered for the whole test.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

    let theme = Theme::Light;
    let ground = Color::Accent.get(Theme::Dark);
    let draw = |art: &mut Art, scale: f32| {
        let mut canvas = Canvas::in_memory(100.0, 100.0, scale, Mode::Direct2D).expect("a canvas");
        canvas.frame_with(Some(art), |painter| {
            painter.clear(Color::Bg.get(theme));
            painter.picture("about", 50.0, 50.0, 40.0);
        });
        canvas.image().expect("pixels")
    };

    let mut art = Art::new();
    let one = draw(&mut art, 1.0);
    let two = draw(&mut art, 2.0);
    let fresh = draw(&mut Art::new(), 2.0);

    assert!(
        one.pixels()
            .iter()
            .any(|pixel| *pixel != ground && *pixel != Color::Bg.get(theme)),
        "the picture, not the stand-in"
    );
    assert_eq!((two.width, two.height), (one.width * 2, one.height * 2));
    assert!(
        two == fresh,
        "at 200 % the picture is drawn for 200 %, not stretched from 100 %"
    );
}

/// The picture is kept finished (sheet 8): a repeated frame does not draw it again, a new size
/// does, and so does a new scale of the screen on the same window — the canvas gives the target its
/// DPI, and `art.rs` sees it. A window that is never shown, so its target lives through the change.
#[cfg(feature = "testing")]
#[test]
fn the_picture_is_drawn_again_for_a_new_size_or_scale_and_not_for_a_repeated_frame() {
    use lang_switcher::ui::art::Art;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_OVERLAPPED,
    };
    use windows::core::w;

    // SAFETY: a plain window of a system class, never shown, destroyed below.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!(""),
            WS_OVERLAPPED,
            0,
            0,
            200,
            200,
            None,
            None,
            None,
            None,
        )
    }
    .expect("a window");
    let mut canvas = Canvas::for_window(hwnd, 200, 200, 1.0);
    let mut art = Art::new();
    let mut frame = |canvas: &mut Canvas, radius: f32| {
        let mode = canvas.frame_with(Some(&mut art), |painter| {
            painter.picture("about", 100.0, 100.0, radius)
        });
        assert_eq!(mode, Mode::Direct2D);
        art.drawn_afresh()
    };

    assert_eq!(frame(&mut canvas, 40.0), 1, "the first frame");
    assert_eq!(
        frame(&mut canvas, 40.0),
        1,
        "a repeated frame draws nothing again"
    );
    assert_eq!(frame(&mut canvas, 60.0), 2, "a new size draws it again");
    canvas.set_scale(2.0);
    assert_eq!(
        frame(&mut canvas, 60.0),
        3,
        "a new scale of the screen draws it again"
    );

    drop(canvas);
    // SAFETY: the window made above.
    let _ = unsafe { DestroyWindow(hwnd) };
}
