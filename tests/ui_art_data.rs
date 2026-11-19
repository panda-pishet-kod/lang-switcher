//! The drawings as the program gets them — task **T-120-12**, part A, decision 208.170.
//!
//! build.rs reads `res/art/**/*.svg` through `build/svg.rs` and writes `OUT_DIR/art.rs`: the static
//! `DRAWINGS` the windows draw from. Included here with the types of that same file, it must
//! compile, it must hold the eight drawings of 1.8.0, and every one of them must equal the reader's
//! own reading of its file — so nothing is lost or bent between the reading and the program.

// The same file build.rs reads; this test crate takes its writer and its reader.
#[allow(dead_code)]
#[path = "../build/svg.rs"]
mod svg;

mod built {
    use super::svg::*;
    include!(concat!(env!("OUT_DIR"), "/art.rs"));
}

const NAMES: [&str; 8] = [
    "about", "avatar", "hello", "news", "settings", "thanks", "update", "wizard",
];

fn read(name: &str) -> svg::Drawing {
    let file = format!("{}/res/art/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|error| panic!("{file}: {error}"));
    svg::parse(&file, &text).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn the_eight_drawings_in_the_order_of_their_names() {
    let names: Vec<&str> = built::DRAWINGS.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, NAMES);
}

#[test]
fn every_drawing_equals_the_reading_of_its_file() {
    for (name, drawing) in &built::DRAWINGS {
        assert_eq!(*drawing, read(name), "{name}");
    }
}

#[test]
fn the_writer_writes_the_file_of_the_build() {
    let drawings: Vec<(String, svg::Drawing)> = NAMES
        .iter()
        .map(|name| ((*name).to_owned(), read(name)))
        .collect();
    let built = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/art.rs"))
        .expect("build.rs writes OUT_DIR/art.rs");
    assert!(
        svg::to_rust(&drawings) == built,
        "to_rust differs from OUT_DIR/art.rs"
    );
}
