//! The panda pictures on the screen — task **T-120-12**, part B, decision 208.170.
//!
//! `src\ui\art.rs` draws with Direct2D the commands build.rs read out of `res\art`. Here it draws
//! into memory — a WIC bitmap under a software render target, no window — and the result is held
//! to two things: the snapshots of the same SVG files from a browser (part A), and known answers
//! for the three places where SVG and Direct2D part ways — the fill rule, the dashes, the clip.
//!
//! The snapshots are not in the repository: the folder is named by the environment variable
//! `LANGSW_ART_REF` (`tools\local.ps1` sets it on the author's machine). Without it the two tests
//! that read it print `SKIPPED_NO_ART_REF` and pass, as tests/feed.rs does with its file. The
//! pictures this file draws are written to the test's temporary folder, `ui_art\`, to be looked at.

use std::path::{Path, PathBuf};

use lang_switcher::ui::art::{self, Art};
use lang_switcher::ui::tokens::Rgb;
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;
use windows::core::HSTRING;

// The reader of build.rs, for the geometry of the dotted line of wizard.svg.
#[allow(dead_code)]
#[path = "../build/svg.rs"]
mod svg;

/// What the task allows between two engines that smooth edges each its own way: the mean
/// difference of a channel, out of 255, and the share of pixels with a channel more than `FAR`
/// apart.
const MEAN_LIMIT: f64 = 3.0;
const FAR: u8 = 48;
const FAR_SHARE_LIMIT: f64 = 0.01;

/// The sizes of the snapshots, in pixels across.
const SIZES: [(&str, [u32; 2]); 8] = [
    ("about", [240, 520]),
    ("avatar", [44, 88]),
    ("hello", [240, 520]),
    ("news", [240, 520]),
    ("settings", [240, 520]),
    ("thanks", [240, 520]),
    ("update", [240, 520]),
    ("wizard", [240, 520]),
];

fn wic() -> IWICImagingFactory {
    // SAFETY: COM for this thread, then the WIC factory; S_FALSE (already initialised) is fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
            .expect("the WIC factory")
    }
}

/// A square of `size` pixels in memory, premultiplied BGRA, with a software render target at
/// `dpi`.
struct Canvas {
    bitmap: IWICBitmap,
    target: ID2D1RenderTarget,
    size: u32,
}

impl Canvas {
    fn new(size: u32, dpi: f32) -> Canvas {
        let wic = wic();
        // SAFETY: plain WIC and Direct2D calls; the structures live across them.
        unsafe {
            let bitmap = wic
                .CreateBitmap(
                    size,
                    size,
                    &GUID_WICPixelFormat32bppPBGRA,
                    WICBitmapCacheOnLoad,
                )
                .expect("a WIC bitmap");
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).expect("Direct2D");
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: dpi,
                dpiY: dpi,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let target = factory
                .CreateWicBitmapRenderTarget(&bitmap, &properties)
                .expect("a render target on the bitmap");
            Canvas {
                bitmap,
                target,
                size,
            }
        }
    }

    /// One frame: cleared to transparency, `paint`, ended.
    fn frame(&self, paint: impl FnOnce(&ID2D1RenderTarget)) {
        // SAFETY: plain Direct2D calls on a live target.
        unsafe {
            self.target.BeginDraw();
            self.target.Clear(Some(&D2D1_COLOR_F::default()));
            paint(&self.target);
            self.target.EndDraw(None, None).expect("EndDraw");
        }
    }

    fn pixels(&self) -> Vec<u8> {
        let mut pixels = vec![0; (self.size * self.size * 4) as usize];
        // SAFETY: the buffer holds the whole bitmap at this stride.
        unsafe {
            self.bitmap
                .CopyPixels(std::ptr::null(), self.size * 4, &mut pixels)
                .expect("the pixels of the canvas");
        }
        pixels
    }

    /// Writes the canvas as PNG to the temporary folder of the tests, `ui_art\<name>.png`, to be
    /// looked at.
    fn save(&self, name: &str) {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ui_art");
        std::fs::create_dir_all(&folder).expect("the folder of the pictures");
        let file = folder.join(format!("{name}.png"));
        std::fs::write(&file, png(self.size, &self.pixels()))
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
    }
}

/// A PNG of premultiplied BGRA pixels: straight RGBA, stored without compression. The encoder of
/// WIC is out of reach — its frame takes a property bag of a feature of `windows` the program does
/// not have — and a picture to look at needs no more than this.
fn png(size: u32, bgra: &[u8]) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        let mut crc = !0_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }

    let mut raw = Vec::new();
    for row in bgra.chunks(size as usize * 4) {
        raw.push(0);
        for pixel in row.chunks(4) {
            let alpha = u32::from(pixel[3]);
            let straight = |value: u8| match alpha {
                0 => 0,
                _ => ((u32::from(value) * 255 + alpha / 2) / alpha).min(255) as u8,
            };
            raw.extend_from_slice(&[straight(pixel[2]), straight(pixel[1]), straight(pixel[0])]);
            raw.push(pixel[3]);
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (index, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(index + 1 == blocks.len()));
        let length = block.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut header = Vec::new();
    header.extend_from_slice(&size.to_be_bytes());
    header.extend_from_slice(&size.to_be_bytes());
    // Eight bits, RGBA, deflate, no filter, no interlace.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The picture `name` drawn by a fresh `Art` into a canvas of `size` pixels at `dpi`, filling it.
fn picture(name: &str, size: u32, dpi: f32) -> Canvas {
    let canvas = Canvas::new(size, dpi);
    let radius = size as f32 * 96.0 / dpi / 2.0;
    let mut art = Art::new();
    canvas.frame(|target| {
        art.draw(target, name, (radius, radius), radius)
            .expect("the picture")
    });
    canvas
}

/// The snapshot folder, or `None` with the line that says why the test passes without it.
fn reference_folder() -> Option<PathBuf> {
    match std::env::var_os("LANGSW_ART_REF").filter(|value| !value.is_empty()) {
        Some(folder) => Some(PathBuf::from(folder)),
        None => {
            println!(
                "SKIPPED_NO_ART_REF: LANGSW_ART_REF is not set — no snapshots to compare with"
            );
            None
        }
    }
}

/// The browser's snapshot `<name>-<size>.png`, premultiplied BGRA like the canvas.
fn reference(folder: &Path, name: &str, size: u32) -> Vec<u8> {
    let file = folder.join(format!("{name}-{size}.png"));
    let wic = wic();
    // SAFETY: plain WIC calls; the buffer holds the whole picture at this stride.
    unsafe {
        let decoder = wic
            .CreateDecoderFromFilename(
                &HSTRING::from(file.as_os_str()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let frame = decoder.GetFrame(0).expect("the frame of the snapshot");
        let converter = wic.CreateFormatConverter().expect("a converter");
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .expect("premultiplied BGRA");
        let (mut width, mut height) = (0, 0);
        converter
            .GetSize(&mut width, &mut height)
            .expect("the size");
        assert_eq!((width, height), (size, size), "{}", file.display());
        let mut pixels = vec![0; (size * size * 4) as usize];
        converter
            .CopyPixels(std::ptr::null(), size * 4, &mut pixels)
            .expect("the pixels of the snapshot");
        pixels
    }
}

/// How far two pictures of `size` across are apart, over the pixels of `region` (left, top,
/// right, bottom; the right and bottom excluded) or over all of them.
#[derive(Debug)]
struct Difference {
    mean: f64,
    far_share: f64,
}

impl Difference {
    fn within_limits(&self) -> bool {
        self.mean <= MEAN_LIMIT && self.far_share <= FAR_SHARE_LIMIT
    }
}

fn difference(a: &[u8], b: &[u8], size: u32, region: Option<[u32; 4]>) -> Difference {
    let [left, top, right, bottom] = region.unwrap_or([0, 0, size, size]);
    let (mut sum, mut far, mut count) = (0_u64, 0_u64, 0_u64);
    for y in top..bottom {
        for x in left..right {
            let at = ((y * size + x) * 4) as usize;
            let mut farthest = 0;
            for channel in 0..4 {
                let apart = a[at + channel].abs_diff(b[at + channel]);
                sum += u64::from(apart);
                farthest = farthest.max(apart);
            }
            far += u64::from(farthest > FAR);
            count += 1;
        }
    }
    Difference {
        mean: sum as f64 / (count * 4) as f64,
        far_share: far as f64 / count as f64,
    }
}

#[test]
fn every_picture_matches_the_browser() {
    let Some(folder) = reference_folder() else {
        return;
    };
    let mut failed = Vec::new();
    for (name, sizes) in SIZES {
        for size in sizes {
            let canvas = picture(name, size, 96.0);
            canvas.save(&format!("{name}-{size}"));
            let apart = difference(
                &canvas.pixels(),
                &reference(&folder, name, size),
                size,
                None,
            );
            println!(
                "{name}-{size}: mean {:.3}, far {:.4} %",
                apart.mean,
                apart.far_share * 100.0
            );
            if !apart.within_limits() {
                failed.push(format!("{name}-{size}: {apart:?}"));
            }
        }
    }
    assert!(failed.is_empty(), "past the limits: {failed:#?}");
}

/// The pixels around the dotted line of wizard.svg, in a picture of `size` across: the box of
/// every point of its path under its matrix, widened by half the line and one pixel.
fn dotted_line_box(size: u32) -> [u32; 4] {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/res/art/wizard.svg");
    let text = std::fs::read_to_string(file).expect("wizard.svg");
    let drawing = svg::parse(file, &text).expect("wizard.svg reads");
    let shape = drawing
        .shapes
        .iter()
        .find(|shape| shape.stroke.is_some_and(|stroke| !stroke.dash.is_empty()))
        .expect("the dotted line of wizard.svg");
    let svg::Geometry::Path(segments) = shape.geometry else {
        panic!("the dotted line is a path");
    };
    let [a, b, c, d, e, f] = shape.matrix.0;
    let k = size as f32 / drawing.width;
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for point in segments.iter().flat_map(|segment| match *segment {
        svg::Segment::Move(to) | svg::Segment::Line(to) => vec![to],
        svg::Segment::Cubic(first, second, to) => vec![first, second, to],
        svg::Segment::Quad(control, to) => vec![control, to],
        svg::Segment::Arc { to, .. } => vec![to],
        svg::Segment::Close => vec![],
    }) {
        let x = (a * point.x + c * point.y + e) * k;
        let y = (b * point.x + d * point.y + f) * k;
        (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
    }
    let width = shape.stroke.expect("a stroke").width * (a * a + b * b).sqrt() * k;
    let margin = width / 2.0 + 1.0;
    let clamp = |value: f32| value.clamp(0.0, size as f32) as u32;
    [
        clamp((left - margin).floor()),
        clamp((top - margin).floor()),
        clamp((right + margin).ceil()),
        clamp((bottom + margin).ceil()),
    ]
}

#[test]
fn the_dots_of_wizard_match_the_browser() {
    let Some(folder) = reference_folder() else {
        return;
    };
    for size in [240, 520] {
        let region = dotted_line_box(size);
        let canvas = picture("wizard", size, 96.0);
        let apart = difference(
            &canvas.pixels(),
            &reference(&folder, "wizard", size),
            size,
            Some(region),
        );
        println!(
            "wizard-{size}, the dotted line {region:?}: mean {:.3}, far {:.4} %",
            apart.mean,
            apart.far_share * 100.0
        );
        assert!(
            apart.within_limits(),
            "wizard-{size}, the dotted line: {apart:?}"
        );
    }
}

#[test]
fn a_self_crossing_outline_fills_by_nonzero() {
    // A five-pointed star drawn in one stroke: its middle is wound twice, filled by SVG's nonzero
    // and left empty by Direct2D's default ALTERNATE. None of the pictures of 1.8.0 has such a
    // fill (measured: a browser draws all eight the same under evenodd), so the trap is held here.
    let star = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path d="M50 5L79 95L2 39L98 39L21 95Z"/></svg>"#;
    let canvas = Canvas::new(100, 96.0);
    canvas.frame(|target| {
        art::paint_svg(target, star, Rgb::from_hex(0), (50.0, 50.0), 50.0).expect("the star")
    });
    canvas.save("star");
    let pixels = canvas.pixels();
    let alpha = |x: u32, y: u32| pixels[((y * 100 + x) * 4 + 3) as usize];
    assert_eq!(alpha(50, 55), 255, "the middle, wound twice");
    assert_eq!(alpha(50, 15), 255, "the top point, wound once");
    assert_eq!(alpha(50, 97), 0, "between the two lower points");
}

#[test]
fn the_rim_of_the_clip_is_smooth() {
    let size = 240;
    let canvas = picture("hello", size, 96.0);
    let pixels = canvas.pixels();
    let radius = size as f32 / 2.0;
    let (mut partial, mut astray) = (0, Vec::new());
    for y in 0..size {
        for x in 0..size {
            let alpha = pixels[((y * size + x) * 4 + 3) as usize];
            if alpha == 0 || alpha == 255 {
                continue;
            }
            partial += 1;
            let distance =
                ((x as f32 + 0.5 - radius).powi(2) + (y as f32 + 0.5 - radius).powi(2)).sqrt();
            if !(radius - 2.0..=radius + 1.0).contains(&distance) {
                astray.push((x, y, alpha));
            }
        }
    }
    // An unsmoothed rim leaves every pixel either inside or outside: none in between.
    let rim = 2.0 * std::f32::consts::PI * radius;
    assert!(
        partial as f32 >= rim / 2.0,
        "{partial} pixels in between along a rim of {rim:.0}"
    );
    assert!(
        astray.is_empty(),
        "translucent pixels off the rim: {astray:?}"
    );
}

#[test]
fn at_200_percent_twice_the_pixels_and_as_sharp() {
    // 260 device-independent pixels at 192 dpi are 520 pixels: the same picture, pixel for
    // pixel, as 520 at 96 dpi — not a picture of 260 pixels stretched.
    let double = picture("hello", 520, 192.0);
    let single = picture("hello", 520, 96.0);
    double.save("hello-520-at-200");
    let apart = difference(&double.pixels(), &single.pixels(), 520, None);
    assert!(
        apart.mean <= 0.5 && apart.far_share == 0.0,
        "200 % against 100 %: {apart:?}"
    );
}

#[test]
fn a_new_size_draws_the_picture_again() {
    let canvas = Canvas::new(520, 96.0);
    let mut art = Art::new();
    canvas.frame(|target| {
        art.draw(target, "news", (120.0, 120.0), 120.0)
            .expect("small")
    });
    canvas.frame(|target| {
        art.draw(target, "news", (260.0, 260.0), 260.0)
            .expect("large")
    });
    let fresh = picture("news", 520, 96.0);
    let apart = difference(&canvas.pixels(), &fresh.pixels(), 520, None);
    assert!(
        apart.mean == 0.0,
        "the second size is the first one stretched: {apart:?}"
    );
}

#[test]
fn an_unknown_name_is_an_error() {
    let canvas = Canvas::new(10, 96.0);
    let mut art = Art::new();
    canvas.frame(|target| {
        assert!(art.draw(target, "nobody", (5.0, 5.0), 5.0).is_err());
    });
    assert_eq!(art::names().count(), 8);
    assert_eq!(art::icon_names().count(), 10);
    assert!(art::names().all(|name| !name.contains('/')));
    canvas.frame(|target| {
        assert!(art::draw_icon(target, "nobody", Rgb::from_hex(0), (0.0, 0.0), 10.0).is_err());
        assert!(art::draw_icon(target, "hello", Rgb::from_hex(0), (0.0, 0.0), 10.0).is_err());
        let mut art = Art::new();
        assert!(art.draw(target, "icons/mail", (5.0, 5.0), 5.0).is_err());
    });
}

#[test]
fn a_stroke_by_parameter_takes_the_colour_it_is_given() {
    // The colour of a menu icon is the shell's (part C): `stroke="currentColor"`, no number.
    let icon = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 12H22" fill="none" stroke="currentColor" stroke-width="4"/></svg>"#;
    let canvas = Canvas::new(24, 96.0);
    canvas.frame(|target| {
        art::paint_svg(target, icon, Rgb::from_hex(0xC8_1E_3C), (12.0, 12.0), 12.0)
            .expect("the line")
    });
    let pixels = canvas.pixels();
    let at = ((12 * 24 + 12) * 4) as usize;
    assert_eq!(
        &pixels[at..at + 4],
        &[0x3C, 0x1E, 0xC8, 0xFF],
        "BGRA in the middle of the line"
    );
    assert_eq!(pixels[3], 0, "the corner stays empty");
}

// ---- the menu icons (part C) ----

/// An icon drawn by `art::draw_icon` in black — the colour a browser gives `currentColor` in a
/// picture of its own — into a canvas of `size` pixels at 96 dpi, filling it.
fn icon(name: &str, size: u32) -> Canvas {
    let canvas = Canvas::new(size, 96.0);
    canvas.frame(|target| {
        art::draw_icon(target, name, Rgb::from_hex(0), (0.0, 0.0), size as f32).expect("the icon")
    });
    canvas
}

#[test]
fn every_icon_matches_the_browser() {
    let Some(folder) = reference_folder() else {
        return;
    };
    let folder = folder.join("icons");
    let mut failed = Vec::new();
    for name in art::icon_names() {
        for size in [18, 36] {
            let canvas = icon(name, size);
            canvas.save(&format!("icon-{name}-{size}"));
            let apart = difference(
                &canvas.pixels(),
                &reference(&folder, name, size),
                size,
                None,
            );
            println!(
                "icons/{name}-{size}: mean {:.3}, far {:.4} %",
                apart.mean,
                apart.far_share * 100.0
            );
            if !apart.within_limits() {
                failed.push(format!("icons/{name}-{size}: {apart:?}"));
            }
        }
    }
    assert!(failed.is_empty(), "past the limits: {failed:#?}");
}

#[test]
fn the_line_of_an_icon_is_1_2_points_at_18() {
    // `stroke-width="1.6"` in the frame of 24 is 1.2 points in an icon of 18 (T-120-12, part C,
    // item 3). Measured across the bottom bar of `down`, `M5 19h14`: the sum of the coverage
    // down one column that crosses the bar and nothing else is the width of the line.
    for (size, width, tolerance) in [(18_u32, 1.2_f32, 0.1_f32), (36, 2.4, 0.2)] {
        let canvas = icon("down", size);
        let pixels = canvas.pixels();
        let k = size as f32 / 24.0;
        let column = (8.0 * k) as u32;
        let rows = ((17.0 * k) as u32)..((21.0 * k).ceil() as u32);
        let measured: f32 = rows
            .map(|row| f32::from(pixels[((row * size + column) * 4 + 3) as usize]) / 255.0)
            .sum();
        println!("down-{size}: the line is {measured:.3} points");
        assert!(
            (measured - width).abs() <= tolerance,
            "at {size}: the line is {measured:.3} points, not {width} ± {tolerance}"
        );
    }
}
