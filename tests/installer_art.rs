//! The pictures of the installer wizard — task **T-99-2**, decision 167.1 (variant B «colour and
//! bamboo» with the signature B1, chosen by the owner on the mock-up of question 167).
//!
//! `tools\make-installer-art.ps1` draws twenty PNG files into `installer\art\`: the background of
//! the wizard — the bamboo of the margins of the site at the left and the right edge, on
//! transparency — and the logo of the signature, each for the light and the dark theme at the five
//! scales of the Inno Setup help. Inno takes the file nearest to the size of its area and
//! stretches it: a picture a pixel off is blurred on every screen of that scale, and nothing else
//! would say so.
//!
//! The pixels are read by GDI+, the decoder of Windows itself, through three functions declared
//! here: the list of crates of section 3.2 is closed (SEC-03), and so is the list of features.

use std::ffi::c_void;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// The scales of the table of the Inno Setup 6.7 help (`WizardBackImageFile`, default font): the
/// name, the size of the background, and the logo — 28 logical pixels at the factor of Inno, the
/// ratio of those areas (decision 167.4 and the header of the generator).
const SCALES: [(&str, u32, u32, u32); 5] = [
    ("100", 596, 432, 28),
    ("125", 796, 576, 37),
    ("150", 994, 720, 47),
    ("175", 1193, 864, 56),
    ("200", 1272, 922, 60),
];

/// The themes and the colour of the face of the logo in each — `--bg` of `gen\site.css`.
const THEMES: [(&str, u32); 2] = [("light", 0x00FA_F9F6), ("dark", 0x0023_272F)];

fn art() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/installer/art")).to_path_buf()
}

/// Width, height, bit depth and colour type out of the header of a PNG, or `None` if the bytes do
/// not begin with the signature of PNG and its `IHDR` chunk.
fn ihdr(bytes: &[u8]) -> Option<(u32, u32, u8, u8)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 33 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((be(16), be(20), bytes[24], bytes[25]))
}

#[repr(C)]
struct StartupInput {
    version: u32,
    debug_event_callback: *const c_void,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[repr(C)]
struct GpRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[repr(C)]
struct BitmapData {
    width: u32,
    height: u32,
    stride: i32,
    pixel_format: i32,
    scan0: *mut c_void,
    reserved: usize,
}

#[link(name = "gdiplus", kind = "raw-dylib")]
unsafe extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const StartupInput, output: *mut c_void) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipCreateBitmapFromFile(file: *const u16, bitmap: *mut *mut c_void) -> i32;
    fn GdipBitmapLockBits(
        bitmap: *mut c_void,
        rect: *const GpRect,
        flags: u32,
        format: i32,
        data: *mut BitmapData,
    ) -> i32;
    fn GdipBitmapUnlockBits(bitmap: *mut c_void, data: *mut BitmapData) -> i32;
    fn GdipDisposeImage(image: *mut c_void) -> i32;
}

const IMAGE_LOCK_MODE_READ: u32 = 1;
const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;

/// The pixels of a picture as `0xAARRGGBB`, not premultiplied, row by row.
fn pixels(path: &Path, width: u32, height: u32) -> Vec<u32> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let input = StartupInput {
        version: 1,
        debug_event_callback: std::ptr::null(),
        suppress_background_thread: 0,
        suppress_external_codecs: 0,
    };
    let mut token = 0usize;
    // SAFETY: a documented call with a valid input structure; the output may be null because
    // the background thread is not suppressed.
    let status = unsafe { GdiplusStartup(&mut token, &input, std::ptr::null_mut()) };
    assert_eq!(status, 0, "GdiplusStartup answered {status}");

    let mut bitmap = std::ptr::null_mut();
    // SAFETY: `wide` is a NUL-terminated path that outlives the call.
    let status = unsafe { GdipCreateBitmapFromFile(wide.as_ptr(), &mut bitmap) };
    assert_eq!(
        status,
        0,
        "GDI+ cannot read {}: status {status}",
        path.display()
    );

    let rect = GpRect {
        x: 0,
        y: 0,
        width: width as i32,
        height: height as i32,
    };
    let mut data = BitmapData {
        width: 0,
        height: 0,
        stride: 0,
        pixel_format: 0,
        scan0: std::ptr::null_mut(),
        reserved: 0,
    };
    // SAFETY: the bitmap was made above; the rectangle is the size its header gives.
    let status = unsafe {
        GdipBitmapLockBits(
            bitmap,
            &rect,
            IMAGE_LOCK_MODE_READ,
            PIXEL_FORMAT_32BPP_ARGB,
            &mut data,
        )
    };
    assert_eq!(
        status,
        0,
        "GDI+ cannot lock {}: status {status}",
        path.display()
    );

    let mut out = Vec::with_capacity((width * height) as usize);
    for row in 0..height as isize {
        // SAFETY: GDI+ hands out `height` rows of `stride` bytes from `scan0`, each holding
        // `width` pixels of four bytes, until the bits are unlocked below.
        let line = unsafe {
            std::slice::from_raw_parts(
                (data.scan0 as *const u8).offset(row * data.stride as isize) as *const u32,
                width as usize,
            )
        };
        out.extend_from_slice(line);
    }

    // SAFETY: the bits were locked above, and the bitmap and the token are released once.
    unsafe {
        GdipBitmapUnlockBits(bitmap, &mut data);
        GdipDisposeImage(bitmap);
        GdiplusShutdown(token);
    }
    out
}

/// **The pictures of the wizard are the twenty the generator draws, each of its exact size.**
///
/// Red, it names the file and what is wrong with it. The checks: `installer\art` holds the
/// background and the logo for both themes at the five scales and nothing else; each is a PNG of
/// eight-bit RGBA of exactly the size of its scale; a background has transparent pixels (the
/// colour of `WizardBackColor` shows through) and drawn ones at the left AND at the right edge,
/// and no drawn pixel between the two columns of `0.3 × height` — the page stays clear; a logo
/// is transparent in its corner and the colour of `--bg` of its theme in the middle of the face;
/// the light and the dark picture of each kind and scale are different files.
#[test]
fn the_pictures_of_the_wizard_are_the_twenty_the_generator_draws() {
    let dir = art();
    assert!(
        dir.is_dir(),
        "installer\\art is missing — tools\\make-installer-art.ps1 draws it"
    );

    let mut expected = Vec::new();
    for (theme, _) in THEMES {
        for (scale, ..) in SCALES {
            expected.push(format!("back-{theme}-{scale}.png"));
            expected.push(format!("logo-{theme}-{scale}.png"));
        }
    }
    let mut on_disk: Vec<String> = fs::read_dir(&dir)
        .expect("installer\\art must be readable")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    on_disk.sort();

    let mut problems = Vec::new();
    for name in &expected {
        if !on_disk.contains(name) {
            problems.push(format!("{name} is missing"));
        }
    }
    for name in &on_disk {
        if !expected.contains(name) {
            problems.push(format!("{name} is not a picture of the generator"));
        }
    }

    let mut checked = 0;
    for (theme, face) in THEMES {
        for (scale, width, height, logo) in SCALES {
            for (kind, w, h) in [("back", width, height), ("logo", logo, logo)] {
                let name = format!("{kind}-{theme}-{scale}.png");
                let Ok(bytes) = fs::read(dir.join(&name)) else {
                    continue;
                };
                match ihdr(&bytes) {
                    Some((pw, ph, 8, 6)) if (pw, ph) == (w, h) => {}
                    Some((pw, ph, depth, colour)) => {
                        problems.push(format!(
                            "{name}: {pw} × {ph}, depth {depth}, colour type {colour} — \
                             wanted {w} × {h}, eight-bit RGBA (type 6)"
                        ));
                        continue;
                    }
                    None => {
                        problems.push(format!("{name}: not a PNG"));
                        continue;
                    }
                }
                checked += 1;

                let argb = pixels(&dir.join(&name), w, h);
                let alpha = |x: u32, y: u32| argb[(y * w + x) as usize] >> 24;
                if kind == "back" {
                    let column = (3 * h).div_ceil(10);
                    let transparent = argb.iter().filter(|p| *p >> 24 == 0).count();
                    let drawn_in = |from: u32, to: u32| {
                        (0..h)
                            .flat_map(|y| (from..to).map(move |x| (x, y)))
                            .filter(|&(x, y)| alpha(x, y) > 0)
                            .count()
                    };
                    let (left, middle, right) = (
                        drawn_in(0, column),
                        drawn_in(column, w - column),
                        drawn_in(w - column, w),
                    );
                    if transparent == 0 || left == 0 || right == 0 || middle != 0 {
                        problems.push(format!(
                            "{name}: transparent {transparent}, drawn at the left edge {left}, \
                             between the columns {middle}, at the right edge {right} — wanted \
                             transparency, bamboo at both edges and nothing between"
                        ));
                    }
                } else {
                    // The middle of the face, (12.5, 15.5) of the 28 × 28 box: opaque and the
                    // colour of `--bg` within 24 a channel — the bicubic boxing down rings by up
                    // to 16 next to the patches in the frames of 28 and 37; the ink and the
                    // accent of either theme stand more than 100 away.
                    let middle = argb[((h * 31 / 56) * w + w * 25 / 56) as usize];
                    let near =
                        |shift: u32| (middle >> shift & 0xFF).abs_diff(face >> shift & 0xFF) <= 24;
                    if alpha(0, 0) != 0 || middle >> 24 != 0xFF || !(near(16) && near(8) && near(0))
                    {
                        problems.push(format!(
                            "{name}: the corner {:08X}, the middle of the face {middle:08X} — \
                             wanted transparency and opaque {face:06X} within 24",
                            argb[0]
                        ));
                    }
                }
            }
        }
    }

    for (scale, ..) in SCALES {
        for kind in ["back", "logo"] {
            let light = fs::read(dir.join(format!("{kind}-light-{scale}.png")));
            let dark = fs::read(dir.join(format!("{kind}-dark-{scale}.png")));
            if let (Ok(light), Ok(dark)) = (light, dark)
                && light == dark
            {
                problems.push(format!(
                    "{kind}-light-{scale}.png and the dark one are the same file"
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "{} problem(s) with the pictures of the installer ({checked} of 20 read):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
    assert_eq!(
        checked, 20,
        "only {checked} of the twenty pictures were read"
    );
}
