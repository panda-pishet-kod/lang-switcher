//! The reader of the panda drawings — task **T-120-12**, part A, decision 208.170.
//!
//! `build/svg.rs` reads `res/art/**/*.svg` at build time into plain drawing commands, because the
//! system reader of SVG exists only from Windows 10 1703 and the program runs from 1607. This file
//! holds that reader to known answers: the numbers, every command of a path, the chain of a
//! transform, the defaults of SVG, the refusal of anything outside the measured subset, and the
//! count of shapes in each of the eight drawings against the census of the stage.

// The same file build.rs reads, so the code under test is the code of the build. This test crate
// takes the reader and leaves the writer of OUT_DIR/art.rs to tests/ui_art_data.rs.
#[allow(dead_code)]
#[path = "../build/svg.rs"]
mod svg;

use svg::{Cap, Geometry, Ink, Join, Matrix, Point, Rgb, Segment, Shape};

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn drawing(body: &str) -> Result<svg::Drawing, String> {
    svg::parse(
        "t.svg",
        &format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">{body}</svg>"#),
    )
}

fn shapes(body: &str) -> Vec<Shape> {
    match drawing(body) {
        Ok(drawing) => drawing.shapes.to_vec(),
        Err(error) => panic!("{body}: {error}"),
    }
}

fn one(body: &str) -> Shape {
    let shapes = shapes(body);
    assert_eq!(shapes.len(), 1, "{body}");
    shapes[0]
}

fn path(d: &str) -> Vec<Segment> {
    svg::path(d).unwrap_or_else(|error| panic!("{d}: {error}"))
}

/// The error of a drawing that must not be read, with the file and `named` in it.
fn refused(body: &str, named: &str) {
    match drawing(body) {
        Ok(_) => panic!("{body}: read, and it is outside the subset"),
        Err(error) => {
            assert!(error.starts_with("t.svg"), "{error}: the file is not named");
            assert!(error.contains(named), "{error}: «{named}» is not named");
        }
    }
}

fn apply(m: Matrix, x: f32, y: f32) -> (f32, f32) {
    let [a, b, c, d, e, f] = m.0;
    (a * x + c * y + e, b * x + d * y + f)
}

// ---- numbers ----

#[test]
fn numbers_leading_dot_run_together_negative() {
    assert_eq!(
        svg::numbers(".6 1.2.8 -3 10-10").unwrap(),
        [0.6, 1.2, 0.8, -3.0, 10.0, -10.0]
    );
}

#[test]
fn numbers_refuse_what_is_not_one() {
    for text in ["1e3", "5px", "+1", "1 x", "--1", "."] {
        assert!(svg::numbers(text).is_err(), "{text}");
    }
}

// ---- path commands ----

#[test]
fn path_m_and_l() {
    assert_eq!(
        path("M1 2L3 4"),
        [Segment::Move(p(1.0, 2.0)), Segment::Line(p(3.0, 4.0))]
    );
}

#[test]
fn path_h_and_v_keep_the_other_coordinate() {
    assert_eq!(
        path("M1 2H5V7"),
        [
            Segment::Move(p(1.0, 2.0)),
            Segment::Line(p(5.0, 2.0)),
            Segment::Line(p(5.0, 7.0))
        ]
    );
}

#[test]
fn path_v_relative() {
    assert_eq!(
        path("M1 2v3"),
        [Segment::Move(p(1.0, 2.0)), Segment::Line(p(1.0, 5.0))]
    );
}

#[test]
fn path_c_absolute() {
    assert_eq!(
        path("M0 0C1 2 3 4 5 6"),
        [
            Segment::Move(p(0.0, 0.0)),
            Segment::Cubic(p(1.0, 2.0), p(3.0, 4.0), p(5.0, 6.0))
        ]
    );
}

#[test]
fn path_c_relative_every_point_from_the_start() {
    assert_eq!(
        path("M10 10c1 2 3 4 5 6"),
        [
            Segment::Move(p(10.0, 10.0)),
            Segment::Cubic(p(11.0, 12.0), p(13.0, 14.0), p(15.0, 16.0))
        ]
    );
}

#[test]
fn path_q_absolute() {
    assert_eq!(
        path("M0 0Q1 2 3 4"),
        [
            Segment::Move(p(0.0, 0.0)),
            Segment::Quad(p(1.0, 2.0), p(3.0, 4.0))
        ]
    );
}

#[test]
fn path_q_relative() {
    assert_eq!(
        path("M10 10q1 2 3 4"),
        [
            Segment::Move(p(10.0, 10.0)),
            Segment::Quad(p(11.0, 12.0), p(13.0, 14.0))
        ]
    );
}

#[test]
fn path_z_returns_to_the_start_of_the_figure() {
    // After Z the pen is at the start, and the next figure begins there.
    assert_eq!(
        path("M1 1L5 5Zv2"),
        [
            Segment::Move(p(1.0, 1.0)),
            Segment::Line(p(5.0, 5.0)),
            Segment::Close,
            Segment::Move(p(1.0, 1.0)),
            Segment::Line(p(1.0, 3.0))
        ]
    );
}

#[test]
fn path_both_arcs_of_news() {
    // The tab of the envelope in news.svg, the only arcs of the eight drawings.
    let arc = |x, y| Segment::Arc {
        to: p(x, y),
        rx: 10.0,
        ry: 10.0,
        rotation: 0.0,
        large: false,
        sweep: true,
    };
    assert_eq!(
        path("M318 200a10 10 0 0 1 10 -10H498a10 10 0 0 1 10 10V242H318Z"),
        [
            Segment::Move(p(318.0, 200.0)),
            arc(328.0, 190.0),
            Segment::Line(p(498.0, 190.0)),
            arc(508.0, 200.0),
            Segment::Line(p(508.0, 242.0)),
            Segment::Line(p(318.0, 242.0)),
            Segment::Close
        ]
    );
}

#[test]
fn path_arc_flags_and_zero_radius() {
    assert_eq!(
        path("M0 0a5 6 30 1 0 2 3"),
        [
            Segment::Move(p(0.0, 0.0)),
            Segment::Arc {
                to: p(2.0, 3.0),
                rx: 5.0,
                ry: 6.0,
                rotation: 30.0,
                large: true,
                sweep: false
            }
        ]
    );
    // SVG draws an arc with a zero radius as a straight line.
    assert_eq!(
        path("M1 1a0 4 0 0 1 2 2"),
        [Segment::Move(p(1.0, 1.0)), Segment::Line(p(3.0, 3.0))]
    );
}

#[test]
fn path_refuses_what_is_outside_the_subset() {
    for (d, named) in [
        ("m1 1", "'m'"),
        ("M1 1l2 2", "'l'"),
        ("M1 1S1 2 3 4", "'S'"),
        ("M1 1A1 1 0 0 1 2 2", "'A'"),
        ("M1 1 2 2", "M"),
        ("L1 1", "M"),
        ("M1 1L2", "L"),
        ("M0 0a1 1 0 2 1 2 2", "flag"),
    ] {
        match svg::path(d) {
            Ok(segments) => panic!("{d}: read as {segments:?}"),
            Err(error) => assert!(error.contains(named), "{d}: {error}"),
        }
    }
}

// ---- transform ----

#[test]
fn transform_chain_in_the_order_written() {
    // translate(10 20) · rotate(90 about 5 5) · scale(2), worked by hand.
    let shape = one(r#"<circle r="1" transform="translate(10 20) rotate(90 5 5) scale(2)"/>"#);
    assert_eq!(shape.matrix, Matrix([0.0, 2.0, -2.0, 0.0, 20.0, 20.0]));
    assert_eq!(apply(shape.matrix, 1.0, 0.0), (20.0, 22.0));
}

#[test]
fn transform_rotate_turns_about_its_point() {
    let shape = one(r#"<circle r="1" transform="rotate(90 5 5)"/>"#);
    assert_eq!(
        apply(shape.matrix, 5.0, 5.0),
        (5.0, 5.0),
        "the centre stays"
    );
    assert_eq!(
        apply(shape.matrix, 6.0, 5.0),
        (5.0, 6.0),
        "a quarter turn clockwise on screen"
    );
}

#[test]
fn transform_of_groups_folds_into_the_shape() {
    let shape = one(
        r#"<g transform="translate(10 0)"><g transform="scale(2)"><circle r="1" transform="translate(1 1)"/></g></g>"#,
    );
    assert_eq!(shape.matrix, Matrix([2.0, 0.0, 0.0, 2.0, 12.0, 2.0]));
}

#[test]
fn transform_refuses_other_forms() {
    for (transform, named) in [
        ("matrix(1 0 0 1 0 0)", "matrix"),
        ("translate(1)", "translate"),
        ("scale(1 2)", "scale"),
        ("rotate(30)", "rotate"),
        ("skewX(10)", "skewX"),
    ] {
        refused(
            &format!(r#"<circle r="1" transform="{transform}"/>"#),
            named,
        );
    }
}

// ---- defaults of SVG ----

#[test]
fn default_fill_is_black_and_no_stroke() {
    let shape = one(r#"<circle cx="1" cy="2" r="3"/>"#);
    assert_eq!(
        shape.geometry,
        Geometry::Ellipse {
            cx: 1.0,
            cy: 2.0,
            rx: 3.0,
            ry: 3.0
        }
    );
    assert_eq!(shape.fill, Some(BLACK));
    assert_eq!(shape.stroke, None);
    assert_eq!(shape.matrix, Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));
}

#[test]
fn default_stroke_width_one_butt_miter_opaque_solid() {
    // The line of wizard.svg: a stroke and no fill attribute.
    let shape = one(r##"<path d="M42 44L90 0" stroke="#1C1B18"/>"##);
    assert_eq!(shape.fill, Some(BLACK));
    let stroke = shape.stroke.expect("a stroke");
    assert_eq!(
        stroke.color,
        Ink::Color(Rgb {
            r: 0x1C,
            g: 0x1B,
            b: 0x18
        })
    );
    assert_eq!(stroke.width, 1.0);
    assert_eq!(stroke.opacity, 1.0);
    assert_eq!((stroke.cap, stroke.join), (Cap::Butt, Join::Miter));
    assert!(stroke.dash.is_empty());
}

#[test]
fn fill_none_and_every_stroke_attribute() {
    let shape = one(
        r##"<path d="M0 0L1 1" fill="none" stroke="#FFFFFF" stroke-width="7" stroke-opacity="0.85" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="1 16"/>"##,
    );
    assert_eq!(shape.fill, None);
    let stroke = shape.stroke.expect("a stroke");
    assert_eq!(
        stroke.color,
        Ink::Color(Rgb {
            r: 255,
            g: 255,
            b: 255
        })
    );
    assert_eq!((stroke.width, stroke.opacity), (7.0, 0.85));
    assert_eq!((stroke.cap, stroke.join), (Cap::Round, Join::Round));
    assert_eq!(stroke.dash, [1.0, 16.0]);
}

#[test]
fn rect_defaults_and_rx_for_ry() {
    let square = one(r#"<rect width="400" height="300"/>"#);
    assert_eq!(
        square.geometry,
        Geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
            rx: 0.0,
            ry: 0.0
        }
    );
    let round = one(r#"<rect x="29" y="130" width="22" height="55.57" rx="6"/>"#);
    assert_eq!(
        round.geometry,
        Geometry::Rect {
            x: 29.0,
            y: 130.0,
            width: 22.0,
            height: 55.57,
            rx: 6.0,
            ry: 6.0
        }
    );
    // SVG clamps each radius to half its side: rx to 11, while ry keeps 20 of 27.785.
    let pill = one(r#"<rect width="22" height="55.57" rx="20"/>"#);
    assert_eq!(
        pill.geometry,
        Geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 22.0,
            height: 55.57,
            rx: 11.0,
            ry: 20.0
        }
    );
}

#[test]
fn shapes_in_document_order() {
    let fills: Vec<_> = shapes(
        r##"<circle r="1" fill="#000001"/><g><rect width="1" height="1" fill="#000002"/></g><path d="M0 0" fill="#000003"/>"##,
    )
    .iter()
    .map(|shape| shape.fill.map(|rgb| rgb.b))
    .collect();
    assert_eq!(fills, [Some(1), Some(2), Some(3)]);
}

// ---- the colour by parameter (part C, the menu icons) ----

const ICON_LINE: &str = r#"<path d="M4 12H20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#;

#[test]
fn stroke_current_color_is_the_colour_by_parameter() {
    let stroke = one(ICON_LINE).stroke.expect("a stroke");
    assert_eq!(stroke.color, Ink::Current);
    assert_eq!(stroke.width, 1.6);
}

#[test]
fn fill_current_color_is_refused() {
    refused(r#"<circle r="1" fill="currentColor"/>"#, "currentColor");
}

#[test]
fn an_icon_has_no_colour_of_its_own() {
    let check = |body: &str| svg::check_icon(&drawing(body).unwrap());
    assert_eq!(check(ICON_LINE), Ok(()));
    assert_eq!(
        check(&format!(
            r#"{ICON_LINE}<circle r="3" fill="none" stroke="currentColor"/>"#
        )),
        Ok(())
    );
    for (body, named) in [
        (
            r##"<path d="M0 0L1 1" fill="none" stroke="#1C1B18"/>"##,
            "#1C1B18",
        ),
        (r#"<path d="M0 0L1 1" stroke="currentColor"/>"#, "fill"),
        (r#"<path d="M0 0L1 1" fill="none"/>"#, "stroke"),
    ] {
        match check(body) {
            Ok(()) => panic!("{body}: an icon with a colour of its own"),
            Err(error) => assert!(error.contains(named), "{body}: {error}"),
        }
    }
}

#[test]
fn a_picture_has_no_colour_by_parameter() {
    let check = |body: &str| svg::check_picture(&drawing(body).unwrap());
    assert_eq!(check(r##"<path d="M0 0L1 1" stroke="#1C1B18"/>"##), Ok(()));
    match check(ICON_LINE) {
        Ok(()) => panic!("a picture with the colour by parameter"),
        Err(error) => assert!(error.contains("currentColor"), "{error}"),
    }
}

#[test]
fn fill_rule_is_nonzero_only() {
    refused(
        r#"<path d="M0 0L1 1L0 1Z" fill-rule="evenodd"/>"#,
        "fill-rule",
    );
}

// ---- clip ----

#[test]
fn clip_circle_of_the_first_group() {
    let drawing = drawing(
        r##"<defs><clipPath id="c"><circle cx="24" cy="24" r="24"/></clipPath></defs><g clip-path="url(#c)"><rect width="48" height="48" fill="#6CC894"/></g>"##,
    )
    .unwrap();
    assert_eq!(
        drawing.clip,
        Some(svg::Circle {
            cx: 24.0,
            cy: 24.0,
            r: 24.0
        })
    );
    assert_eq!(drawing.shapes.len(), 1);
}

#[test]
fn clip_outside_the_measured_form_is_refused() {
    let defs = r#"<defs><clipPath id="c"><circle cx="24" cy="24" r="24"/></clipPath></defs>"#;
    refused(
        &format!(r#"{defs}<g clip-path="url(#d)"><circle r="1"/></g>"#),
        "#d",
    );
    refused(
        &format!(r#"{defs}<circle r="1"/><g clip-path="url(#c)"/>"#),
        "clip-path",
    );
    refused(
        &format!(r#"{defs}<g clip-path="url(#c)"/><circle r="1"/>"#),
        "circle",
    );
    refused(
        &format!(r#"{defs}<g transform="scale(2)" clip-path="url(#c)"/>"#),
        "clip-path",
    );
    refused(
        r#"<defs><clipPath id="c"><rect width="1" height="1"/></clipPath></defs>"#,
        "rect",
    );
}

// ---- the subset ----

#[test]
fn unknown_element_names_the_file_and_the_element() {
    refused(
        r#"<defs><linearGradient id="g"/></defs><circle r="1"/>"#,
        "linearGradient",
    );
    refused(r#"<text>Hi</text>"#, "text");
    refused(r#"<!-- a note --><circle r="1"/>"#, "<!--");
}

#[test]
fn unknown_attribute_names_the_attribute() {
    refused(r#"<circle r="1" opacity="0.5"/>"#, "opacity");
    refused(r#"<rect width="1" height="1" rx="1" ry="1"/>"#, "ry");
    refused(r#"<g fill="red"><circle r="1"/></g>"#, "fill");
}

#[test]
fn unknown_value_names_the_value() {
    refused(r#"<circle r="1" fill="red"/>"#, "red");
    refused(r##"<circle r="1" fill="#abc"/>"##, "#abc");
    refused(r#"<path d="M0 0" stroke-linecap="square"/>"#, "square");
    refused(r#"<path d="M0 0" stroke-opacity="1.5"/>"#, "1.5");
    refused(r#"<path d="M0 0" stroke-dasharray="1 2 3"/>"#, "1 2 3");
    refused(r#"<circle r="5px"/>"#, "5px");
    refused(r#"<circle r="0"/>"#, r#"r="0""#);
}

// ---- the eight drawings of 1.8.0 ----

/// The census of scratchpad-E120-art/svg-subset.md: circles (the clip circle among them),
/// ellipses, rectangles, paths, and whether there is a clip.
const CENSUS: [(&str, usize, usize, usize, usize, bool); 8] = [
    ("hello", 7, 10, 12, 4, true),
    ("thanks", 7, 10, 9, 7, true),
    ("news", 7, 10, 12, 6, true),
    ("update", 9, 10, 9, 9, true),
    ("about", 7, 10, 10, 11, true),
    ("wizard", 7, 10, 9, 7, true),
    ("settings", 12, 10, 26, 5, true),
    ("avatar", 7, 2, 0, 1, false),
];

fn res_art(name: &str) -> svg::Drawing {
    let file = format!("{}/res/art/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|error| panic!("{file}: {error}"));
    svg::parse(&file, &text).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn eight_drawings_count_as_measured() {
    for (name, circles, ellipses, rects, paths, clipped) in CENSUS {
        let drawing = res_art(name);
        let count = |kind: fn(&Geometry) -> bool| {
            drawing
                .shapes
                .iter()
                .filter(|shape| kind(&shape.geometry))
                .count()
        };
        let clip = usize::from(drawing.clip.is_some());
        assert_eq!(drawing.clip.is_some(), clipped, "{name}: clip");
        assert_eq!(
            count(|g| matches!(g, Geometry::Ellipse { .. })) + clip,
            circles + ellipses,
            "{name}: circles and ellipses"
        );
        assert_eq!(
            count(|g| matches!(g, Geometry::Rect { .. })),
            rects,
            "{name}: rects"
        );
        assert_eq!(
            count(|g| matches!(g, Geometry::Path(_))),
            paths,
            "{name}: paths"
        );
        assert_eq!(
            drawing.shapes.len() + clip,
            circles + ellipses + rects + paths,
            "{name}"
        );
    }
}

#[test]
fn eight_drawings_field_and_clip() {
    for (name, ..) in CENSUS {
        let drawing = res_art(name);
        if name == "avatar" {
            assert_eq!(
                (drawing.width, drawing.height, drawing.clip),
                (48.0, 48.0, None)
            );
        } else {
            assert_eq!((drawing.width, drawing.height), (400.0, 400.0), "{name}");
            assert_eq!(
                drawing.clip,
                Some(svg::Circle {
                    cx: 200.0,
                    cy: 200.0,
                    r: 200.0
                }),
                "{name}"
            );
        }
    }
}

#[test]
fn wizard_dots_and_news_arcs_arrive() {
    let dashed: Vec<_> = res_art("wizard")
        .shapes
        .iter()
        .filter_map(|shape| shape.stroke)
        .filter(|stroke| !stroke.dash.is_empty())
        .collect();
    assert_eq!(dashed.len(), 1);
    assert_eq!(
        (dashed[0].dash, dashed[0].width, dashed[0].cap),
        (&[1.0, 16.0][..], 7.0, Cap::Round)
    );

    let arcs = res_art("news")
        .shapes
        .iter()
        .filter_map(|shape| match shape.geometry {
            Geometry::Path(segments) => Some(segments),
            _ => None,
        })
        .flatten()
        .filter(|segment| matches!(segment, Segment::Arc { .. }))
        .count();
    assert_eq!(arcs, 2);
}
