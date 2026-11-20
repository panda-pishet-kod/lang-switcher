//! The panda pictures. Knows nothing of windows.
//!
//! Stage E120, task T-120-12 (decision 208.170). The pictures of `res\art` are read at build time
//! by `build\svg.rs` into plain drawing commands — the system reader of SVG exists only from
//! Windows 10 1703, and the program runs from 1607 — and this file draws those commands with
//! Direct2D geometry: ellipses, rounded rectangles and paths, under one 3×2 matrix per shape.
//!
//! The shell asks one thing of it: draw the picture with this name into this circle
//! ([`Art::draw`]). A finished picture is kept as a bitmap of its size in pixels and redrawn when
//! the size or the scale changes, so a window repaints a picture by copying it.
//!
//! Three things differ between SVG and Direct2D, and each is held here by name:
//!
//! - **the fill rule.** SVG fills by `nonzero`; a path of Direct2D fills by `ALTERNATE` unless told
//!   otherwise, so every path sets `D2D1_FILL_MODE_WINDING`;
//! - **the dashes.** SVG measures a dash in units of the drawing, Direct2D in widths of the line:
//!   `1 16` at a width of 7 is 1/7 and 16/7;
//! - **the clip.** The circle of a picture clips through a layer with a geometric mask, smoothed
//!   like any other edge; a clip by a rectangle would leave a staircase along the rim.

use std::mem::ManuallyDrop;

use ::windows::Win32::Foundation::E_INVALIDARG;
use ::windows::Win32::Graphics::Direct2D::Common::*;
use ::windows::Win32::Graphics::Direct2D::*;
use ::windows::core::{Error, Interface, Result};

// The types of the pictures and the reader of SVG are one file with the build script, so the data
// the build writes and the types the program draws cannot part. The program takes the types only;
// the reader half of the file runs at build time.
#[allow(dead_code)]
#[path = "../../build/svg.rs"]
mod svg;

use super::tokens;
use svg::{Cap, Drawing, Geometry, Ink, Join, Rgb, Segment, Shape, Stroke};

/// `DRAWINGS`, written by build.rs out of `res\art`.
mod data {
    use super::svg::*;
    include!(concat!(env!("OUT_DIR"), "/art.rs"));
}

// windows 0.62 takes the point and the matrix of the crate `windows_numerics` and does not
// re-export it, and the list of crates is closed (SPEC section 3.2). So the two types are reached
// through a field of a Direct2D structure that holds one, and filled in by name: a macro, because
// a function would have to name its return type.
macro_rules! vector {
    ($x:expr, $y:expr) => {{
        let mut vector = D2D1_ELLIPSE::default().point;
        vector.X = $x;
        vector.Y = $y;
        vector
    }};
}

/// The matrix of `[a, b, c, d, e, f]` in the order of SVG's `matrix(…)`, which is the order of
/// Direct2D's fields: a point (x, y) goes to (a·x + c·y + e, b·x + d·y + f).
macro_rules! matrix {
    ($six:expr) => {{
        let [a, b, c, d, e, f]: [f32; 6] = $six;
        let mut matrix = D2D1_BRUSH_PROPERTIES::default().transform;
        matrix.M11 = a;
        matrix.M12 = b;
        matrix.M21 = c;
        matrix.M22 = d;
        matrix.M31 = e;
        matrix.M32 = f;
        matrix
    }};
}

const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// A device-independent pixel: Direct2D's unit is 1/96 inch.
const DIPS_PER_INCH: f32 = 96.0;

/// SVG's `stroke-miterlimit` is 4 widths of the line. Direct2D measures the miter against half the
/// width (ID2D1StrokeStyle::GetMiterLimit), so the same limit is 8 there.
const MITER_LIMIT: f32 = 8.0;

/// The folder of the menu icons under `res\art`; their names here go without it.
const ICONS: &str = "icons/";

/// The named place `place` of the picture `name` (T-120-25: `anchor-screen` and `anchor-key` of
/// «Привет»), as shares of the field of the picture — left, top, width, height, each 0 to 1: the
/// window lays it over the circle it draws the picture in.
pub fn anchor(name: &str, place: &str) -> Option<[f32; 4]> {
    let (_, drawing) = find(name).ok()?;
    let found = drawing.anchors.iter().find(|anchor| anchor.name == place)?;
    Some([
        found.x / drawing.width,
        found.y / drawing.height,
        found.width / drawing.width,
        found.height / drawing.height,
    ])
}

/// The names of the pictures, in the order of `res\art`.
pub fn names() -> impl Iterator<Item = &'static str> {
    data::DRAWINGS
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !name.starts_with(ICONS))
}

/// The names of the menu icons, the files of `res\art\icons`, in their order.
pub fn icon_names() -> impl Iterator<Item = &'static str> {
    data::DRAWINGS
        .iter()
        .filter_map(|(name, _)| name.strip_prefix(ICONS))
}

fn find(name: &str) -> Result<(&'static str, &'static Drawing)> {
    data::DRAWINGS
        .iter()
        .find(|(known, _)| *known == name && !known.starts_with(ICONS))
        .map(|(known, drawing)| (*known, drawing))
        .ok_or_else(|| Error::new(E_INVALIDARG, format!("no picture «{name}» in res\\art")))
}

/// Draws the menu icon `name`, a file of `res\art\icons`, in `color` into the square with this
/// top-left `corner` and `side`, in the device-independent pixels of `target`, between its
/// `BeginDraw` and `EndDraw`. The corner lands on a whole pixel. An icon is a few lines and is
/// drawn afresh each time: its colour changes with the state of its item.
pub fn draw_icon(
    target: &ID2D1RenderTarget,
    name: &str,
    color: tokens::Rgb,
    corner: (f32, f32),
    side: f32,
) -> Result<()> {
    let drawing = data::DRAWINGS
        .iter()
        .find(|(known, _)| known.strip_prefix(ICONS) == Some(name))
        .map(|(_, drawing)| drawing)
        .ok_or_else(|| Error::new(E_INVALIDARG, format!("no icon «{name}» in res\\art\\icons")))?;
    let scale = scale_of(target);
    let left = (corner.0 * scale).round() / scale;
    let top = (corner.1 * scale).round() / scale;
    let half = side / 2.0;
    paint(
        target,
        drawing,
        (left + half, top + half),
        half,
        channels(color.r, color.g, color.b),
    )
}

/// Device pixels per device-independent pixel of `target`.
fn scale_of(target: &ID2D1RenderTarget) -> f32 {
    let (mut dpi_x, mut dpi_y) = (0.0, 0.0);
    // SAFETY: two writable floats for the two values the call fills in.
    unsafe { target.GetDpi(&mut dpi_x, &mut dpi_y) };
    dpi_x / DIPS_PER_INCH
}

/// One finished picture: drawn for `target` at `pixels` across.
struct Finished {
    name: &'static str,
    pixels: u32,
    target: ID2D1RenderTarget,
    bitmap: ID2D1Bitmap,
}

/// The pictures of one window, kept as bitmaps of the size they were last drawn at.
///
/// A bitmap belongs to the render target that made it: one that was made for another target, or
/// for another size in pixels, is drawn again. A window keeps one `Art` for its life.
#[derive(Default)]
pub struct Art {
    finished: Vec<Finished>,
    /// How many times a picture was drawn into a new bitmap — for the tests that a repeated
    /// frame copies the finished picture and does not draw it again (T-120-8). Feature
    /// `testing`, absent from the Release configuration (SEC-04a).
    #[cfg(feature = "testing")]
    drawn_afresh: usize,
}

impl Art {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many times this `Art` drew a picture into a new bitmap: once per picture, and again
    /// only when its size in pixels or its render target changed.
    #[cfg(feature = "testing")]
    pub fn drawn_afresh(&self) -> usize {
        self.drawn_afresh
    }

    /// Draws the picture `name` into the circle with this `centre` and `radius`, in the
    /// device-independent pixels of `target`, between its `BeginDraw` and `EndDraw`. The square
    /// field of the picture fills the square around the circle.
    pub fn draw(
        &mut self,
        target: &ID2D1RenderTarget,
        name: &str,
        centre: (f32, f32),
        radius: f32,
    ) -> Result<()> {
        let (name, drawing) = find(name)?;
        let scale = scale_of(target);
        let pixels = (2.0 * radius * scale).round().max(1.0) as u32;

        let fresh = |finished: &Finished| {
            finished.name == name
                && finished.pixels == pixels
                && finished.target.as_raw() == target.as_raw()
        };
        let bitmap = match self.finished.iter().find(|finished| fresh(finished)) {
            Some(finished) => finished.bitmap.clone(),
            None => {
                let bitmap = finish(target, drawing, pixels, scale)?;
                #[cfg(feature = "testing")]
                {
                    self.drawn_afresh += 1;
                }
                self.finished.retain(|finished| finished.name != name);
                self.finished.push(Finished {
                    name,
                    pixels,
                    target: target.clone(),
                    bitmap: bitmap.clone(),
                });
                bitmap
            }
        };

        // The corner lands on a whole pixel, so the bitmap is copied pixel for pixel, unblurred.
        let size = pixels as f32 / scale;
        let left = ((centre.0 - radius) * scale).round() / scale;
        let top = ((centre.1 - radius) * scale).round() / scale;
        let place = D2D_RECT_F {
            left,
            top,
            right: left + size,
            bottom: top + size,
        };
        // SAFETY: a bitmap of this target, a rectangle that lives across the call.
        unsafe {
            target.DrawBitmap(
                &bitmap,
                Some(&place),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
        Ok(())
    }
}

/// The picture drawn once into a bitmap of `pixels` across, compatible with `target`.
fn finish(
    target: &ID2D1RenderTarget,
    drawing: &Drawing,
    pixels: u32,
    scale: f32,
) -> Result<ID2D1Bitmap> {
    let across = pixels as f32 / scale;
    // SAFETY: plain Direct2D calls on live interfaces; the sizes live across the call.
    unsafe {
        let bitmap_target = target.CreateCompatibleRenderTarget(
            Some(&D2D_SIZE_F {
                width: across,
                height: across,
            }),
            Some(&D2D_SIZE_U {
                width: pixels,
                height: pixels,
            }),
            None,
            D2D1_COMPATIBLE_RENDER_TARGET_OPTIONS_NONE,
        )?;
        bitmap_target.BeginDraw();
        bitmap_target.Clear(Some(&D2D1_COLOR_F::default()));
        let half = across / 2.0;
        let painted = paint(&bitmap_target, drawing, (half, half), half, no_parameter());
        let ended = bitmap_target.EndDraw(None, None);
        painted?;
        ended?;
        bitmap_target.GetBitmap()
    }
}

/// Paints a drawing that is not in `res\art`, read from its text now, with `color` for the colour
/// by parameter — for the tests of the traps: a fill with a self-crossing outline, which none of
/// the pictures of 1.8.0 has, and the colour of an icon.
#[doc(hidden)]
pub fn paint_svg(
    target: &ID2D1RenderTarget,
    text: &str,
    color: tokens::Rgb,
    centre: (f32, f32),
    radius: f32,
) -> Result<()> {
    let drawing = svg::parse("svg text", text).map_err(|error| Error::new(E_INVALIDARG, error))?;
    paint(
        target,
        &drawing,
        centre,
        radius,
        channels(color.r, color.g, color.b),
    )
}

/// Paints `drawing` straight onto `target`: its field fitted to the square around the circle,
/// under the transform the target already has; a stroke of the colour by parameter in `current`.
fn paint(
    target: &ID2D1RenderTarget,
    drawing: &Drawing,
    centre: (f32, f32),
    radius: f32,
    current: D2D1_COLOR_F,
) -> Result<()> {
    // SAFETY: plain Direct2D calls on live interfaces; every structure passed by pointer lives
    // across its call, and the transform of the target is put back on every way out.
    unsafe {
        let factory = target.GetFactory()?;
        let mut outer = matrix!(IDENTITY);
        target.GetTransform(&mut outer);
        let k = 2.0 * radius / drawing.width.max(drawing.height);
        let field = matrix!([k, 0.0, 0.0, k, centre.0 - radius, centre.1 - radius]) * outer;
        let brush = target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?;

        let layer = match drawing.clip {
            None => None,
            Some(clip) => {
                let circle = factory.CreateEllipseGeometry(&D2D1_ELLIPSE {
                    point: vector!(clip.cx, clip.cy),
                    radiusX: clip.r,
                    radiusY: clip.r,
                })?;
                let layer = target.CreateLayer(None)?;
                // The mask is taken in the coordinates of the drawing, under the transform of
                // the target at the moment of the push.
                target.SetTransform(&field);
                let parameters = D2D1_LAYER_PARAMETERS {
                    contentBounds: D2D_RECT_F {
                        left: -f32::MAX,
                        top: -f32::MAX,
                        right: f32::MAX,
                        bottom: f32::MAX,
                    },
                    geometricMask: ManuallyDrop::new(Some((*circle).clone())),
                    maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                    maskTransform: matrix!(IDENTITY),
                    opacity: 1.0,
                    opacityBrush: ManuallyDrop::new(None),
                    layerOptions: D2D1_LAYER_OPTIONS_NONE,
                };
                target.PushLayer(&parameters, &layer);
                drop(ManuallyDrop::into_inner(parameters.geometricMask));
                Some(layer)
            }
        };

        let mut result = Ok(());
        for shape in drawing.shapes {
            target.SetTransform(&(matrix!(shape.matrix.0) * field));
            result = paint_shape(target, &factory, &brush, shape, current);
            if result.is_err() {
                break;
            }
        }
        if layer.is_some() {
            target.PopLayer();
        }
        target.SetTransform(&outer);
        result
    }
}

fn color(rgb: Rgb) -> D2D1_COLOR_F {
    channels(rgb.r, rgb.g, rgb.b)
}

/// An opaque colour of Direct2D out of three channels of 0..=255.
fn channels(red: u8, green: u8, blue: u8) -> D2D1_COLOR_F {
    let channel = |value: u8| f32::from(value) / f32::from(u8::MAX);
    D2D1_COLOR_F {
        r: channel(red),
        g: channel(green),
        b: channel(blue),
        a: 1.0,
    }
}

/// The colour by parameter of a picture: none has one (build.rs refuses it outside the icons), so
/// this is never painted — SVG's own default, black, stands for it.
fn no_parameter() -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        a: 1.0,
        ..Default::default()
    }
}

/// The geometry of one shape, in the form Direct2D draws it.
enum Outline {
    Ellipse(D2D1_ELLIPSE),
    Rect(D2D_RECT_F),
    Rounded(D2D1_ROUNDED_RECT),
    Path(ID2D1PathGeometry),
}

/// One shape under the transform already set: filled first, then stroked.
///
/// # Safety
/// `target` is between `BeginDraw` and `EndDraw`, and `brush` was made by it.
unsafe fn paint_shape(
    target: &ID2D1RenderTarget,
    factory: &ID2D1Factory,
    brush: &ID2D1SolidColorBrush,
    shape: &Shape,
    current: D2D1_COLOR_F,
) -> Result<()> {
    let outline = match shape.geometry {
        Geometry::Ellipse { cx, cy, rx, ry } => Outline::Ellipse(D2D1_ELLIPSE {
            point: vector!(cx, cy),
            radiusX: rx,
            radiusY: ry,
        }),
        Geometry::Rect {
            x,
            y,
            width,
            height,
            rx,
            ry,
        } => {
            let rect = D2D_RECT_F {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            };
            if rx > 0.0 && ry > 0.0 {
                Outline::Rounded(D2D1_ROUNDED_RECT {
                    rect,
                    radiusX: rx,
                    radiusY: ry,
                })
            } else {
                Outline::Rect(rect)
            }
        }
        Geometry::Path(segments) => Outline::Path(path(factory, segments)?),
    };

    // SAFETY: the caller's promise; every structure lives across its call.
    unsafe {
        if let Some(fill) = shape.fill {
            brush.SetColor(&color(fill));
            brush.SetOpacity(1.0);
            match &outline {
                Outline::Ellipse(ellipse) => target.FillEllipse(ellipse, brush),
                Outline::Rect(rect) => target.FillRectangle(rect, brush),
                Outline::Rounded(rounded) => target.FillRoundedRectangle(rounded, brush),
                Outline::Path(path) => target.FillGeometry(path, brush, None),
            }
        }
        if let Some(stroke) = shape.stroke {
            brush.SetColor(&match stroke.color {
                Ink::Color(rgb) => color(rgb),
                Ink::Current => current,
            });
            brush.SetOpacity(stroke.opacity);
            let style = stroke_style(factory, &stroke)?;
            let width = stroke.width;
            match &outline {
                Outline::Ellipse(ellipse) => target.DrawEllipse(ellipse, brush, width, &style),
                Outline::Rect(rect) => target.DrawRectangle(rect, brush, width, &style),
                Outline::Rounded(rounded) => {
                    target.DrawRoundedRectangle(rounded, brush, width, &style)
                }
                Outline::Path(path) => target.DrawGeometry(path, brush, width, &style),
            }
        }
    }
    Ok(())
}

/// The ends, the corners and the dashes of a stroke.
fn stroke_style(factory: &ID2D1Factory, stroke: &Stroke) -> Result<ID2D1StrokeStyle> {
    let cap = match stroke.cap {
        Cap::Butt => D2D1_CAP_STYLE_FLAT,
        Cap::Round => D2D1_CAP_STYLE_ROUND,
    };
    let properties = D2D1_STROKE_STYLE_PROPERTIES {
        startCap: cap,
        endCap: cap,
        // SVG gives every dash the ends of the line.
        dashCap: cap,
        lineJoin: match stroke.join {
            // SVG's miter turns into a bevel past the limit, which is Direct2D's MITER_OR_BEVEL.
            Join::Miter => D2D1_LINE_JOIN_MITER_OR_BEVEL,
            Join::Round => D2D1_LINE_JOIN_ROUND,
        },
        miterLimit: MITER_LIMIT,
        dashStyle: if stroke.dash.is_empty() {
            D2D1_DASH_STYLE_SOLID
        } else {
            D2D1_DASH_STYLE_CUSTOM
        },
        dashOffset: 0.0,
    };
    // Direct2D counts a dash in widths of the line, SVG in units of the drawing.
    let dashes: Vec<f32> = stroke
        .dash
        .iter()
        .map(|length| length / stroke.width)
        .collect();
    // SAFETY: the properties and the dashes live across the call.
    unsafe {
        factory.CreateStrokeStyle(
            &properties,
            (!dashes.is_empty()).then_some(dashes.as_slice()),
        )
    }
}

/// The figures of a path. Every figure is filled — SVG fills an open one as if it were closed —
/// and the fill is nonzero, `WINDING`, where Direct2D's own default would be `ALTERNATE`.
fn path(factory: &ID2D1Factory, segments: &[Segment]) -> Result<ID2D1PathGeometry> {
    // SAFETY: plain Direct2D calls on live interfaces; every structure lives across its call.
    unsafe {
        let geometry = factory.CreatePathGeometry()?;
        let sink = geometry.Open()?;
        sink.SetFillMode(D2D1_FILL_MODE_WINDING);
        let mut open = false;
        for segment in segments {
            match *segment {
                Segment::Move(to) => {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    }
                    sink.BeginFigure(vector!(to.x, to.y), D2D1_FIGURE_BEGIN_FILLED);
                    open = true;
                }
                Segment::Line(to) => sink.AddLine(vector!(to.x, to.y)),
                Segment::Cubic(first, second, to) => sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                    point1: vector!(first.x, first.y),
                    point2: vector!(second.x, second.y),
                    point3: vector!(to.x, to.y),
                }),
                Segment::Quad(control, to) => {
                    sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT {
                        point1: vector!(control.x, control.y),
                        point2: vector!(to.x, to.y),
                    })
                }
                Segment::Arc {
                    to,
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                } => sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: vector!(to.x, to.y),
                    size: D2D_SIZE_F {
                        width: rx,
                        height: ry,
                    },
                    rotationAngle: rotation,
                    // SVG's sweep flag 1 is the positive angle: clockwise, y going down.
                    sweepDirection: if sweep {
                        D2D1_SWEEP_DIRECTION_CLOCKWISE
                    } else {
                        D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                    },
                    arcSize: if large {
                        D2D1_ARC_SIZE_LARGE
                    } else {
                        D2D1_ARC_SIZE_SMALL
                    },
                }),
                Segment::Close => {
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    open = false;
                }
            }
        }
        if open {
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }
        sink.Close()?;
        Ok(geometry)
    }
}
