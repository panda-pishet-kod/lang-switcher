// Stage E120, task T-120-12: the drawings of res/art, read at build time.
//
// The windows of 1.8.0 draw the panda with plain Direct2D geometry, and that is why this file
// exists: the system reader of SVG documents arrived with Windows 10 1703, and the program runs
// from 1607 (decision 208.170). So the build reads the drawings once and hands the program a list
// of simple commands -- an ellipse here, a line there -- that draw the same on every Windows 10.
//
// One file, three readers. build.rs turns every res/art/**/*.svg into OUT_DIR/art.rs; the tests
// (tests/ui_art_parse.rs, tests/ui_art_data.rs) hold the reader to known answers; src/ui/art.rs
// takes the types of the data it draws. Standard library only: a build dependency would be a
// crate added to the closed list of SPEC section 3.2.
//
// The subset is exactly the one measured in the eight drawings of 1.8.0, not the SVG of the
// specification. Anything outside it -- an element, an attribute, a path command, a value --
// stops the build with the file and the name of what was not understood: drawn in silence, it
// would be a picture that differs from the one its author made, and nobody would see why.
//
// The output names nothing of Direct2D: the size of the drawing, an optional circle clip, and the
// shapes in document order, each with its folded 3x2 matrix, its geometry in absolute
// coordinates, its fill and its stroke. Every fill is nonzero, the SVG default -- `fill-rule` is
// outside the subset.

/// A point, in the coordinates of its own shape (before the shape's matrix).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A 3x2 matrix in the order of SVG's `matrix(a b c d e f)`: the point (x, y) goes to
/// (a·x + c·y + e, b·x + d·y + f). The chain of every `transform` above a shape, folded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix(pub [f32; 6]);

/// An opaque colour, `#RRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// The end of an open line: SVG's default `butt`, or `round`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cap {
    Butt,
    Round,
}

/// The corner of a stroke: SVG's default `miter` (limit 4), or `round`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Join {
    Miter,
    Round,
}

/// The stroke of a shape. `width` and `dash` are in the units of the shape, as SVG writes them;
/// an empty `dash` is a solid line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub color: Rgb,
    pub width: f32,
    pub opacity: f32,
    pub cap: Cap,
    pub join: Join,
    pub dash: &'static [f32],
}

/// One step of a path, every point absolute. A figure always begins with `Move`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Quad(Point, Point),
    /// SVG's elliptical arc to `to`. `rotation` is in degrees; `large` and `sweep` are SVG's
    /// flags -- `sweep` true is the positive angle, clockwise on a screen whose y goes down.
    Arc {
        to: Point,
        rx: f32,
        ry: f32,
        rotation: f32,
        large: bool,
        sweep: bool,
    },
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry {
    /// `circle` and `ellipse`.
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
    /// `rect`; `rx` and `ry` zero for square corners.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rx: f32,
        ry: f32,
    },
    Path(&'static [Segment]),
}

/// One shape: filled first, then stroked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub geometry: Geometry,
    pub matrix: Matrix,
    pub fill: Option<Rgb>,
    pub stroke: Option<Stroke>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

/// A drawing: its field (`viewBox`, origin at zero), the circle every shape is clipped to, if
/// any, and its shapes in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawing {
    pub width: f32,
    pub height: f32,
    pub clip: Option<Circle>,
    pub shapes: &'static [Shape],
}

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

// The build script and the tests read each drawing once and keep it to the end of the process, so
// its lists are leaked into `&'static` -- the type of the program's data, which `to_rust` writes as
// static slices. One type for both is what lets tests/ui_art_data.rs compare them with `==`.
fn leak<T>(list: Vec<T>) -> &'static [T] {
    Vec::leak(list)
}

fn point(x: f64, y: f64) -> Point {
    Point {
        x: x as f32,
        y: y as f32,
    }
}

// ---- numbers ----

/// A cursor over one attribute value: its numbers and the flags of an arc.
struct Scan<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Scan<'a> {
    fn new(text: &'a str) -> Self {
        Scan { text, at: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    /// SVG's comma-wsp: spaces, at most one comma, spaces.
    fn skip_separator(&mut self) {
        self.skip_space();
        if self.peek() == Some(b',') {
            self.at += 1;
            self.skip_space();
        }
    }

    /// The word at `start`, for an error: up to a space or a comma, at least one character.
    fn word(&self, start: usize) -> &'a str {
        let rest = &self.text[start..];
        let end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == ',')
            .unwrap_or(rest.len());
        let first = rest.chars().next().map_or(0, char::len_utf8);
        &rest[..end.max(first)]
    }

    /// One number at the cursor: a minus or none, digits, a fraction or none -- `.6` and `5.`
    /// count. It ends where its grammar ends, so `1.2.8` is 1.2 and then .8, and `10-10` is 10 and
    /// then -10. No plus, no exponent: neither is in the drawings.
    fn number(&mut self) -> Result<f64, String> {
        let start = self.at;
        let bytes = self.text.as_bytes();
        let digits = |mut i: usize| {
            let from = i;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            (i, i - from)
        };
        let mut i = start + usize::from(bytes.get(start) == Some(&b'-'));
        let (end, whole) = digits(i);
        i = end;
        let mut fraction = 0;
        if bytes.get(i) == Some(&b'.') {
            (i, fraction) = digits(i + 1);
        }
        if whole + fraction == 0 {
            return Err(match self.word(start) {
                "" => "a number is missing".to_owned(),
                word => format!("{word:?} is not a number"),
            });
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            return Err(format!(
                "{:?}: an exponent is not in the subset",
                self.word(start)
            ));
        }
        self.at = i;
        self.text[start..i]
            .parse()
            .map_err(|_| format!("{:?} is not a number", &self.text[start..i]))
    }

    /// `N` numbers of a path command; the first may follow its letter without a separator.
    fn args<const N: usize>(
        &mut self,
        command: char,
        after_letter: bool,
    ) -> Result<[f64; N], String> {
        let mut out = [0.0; N];
        for (index, slot) in out.iter_mut().enumerate() {
            if index == 0 && after_letter {
                self.skip_space();
            } else {
                self.skip_separator();
            }
            if !self
                .peek()
                .is_some_and(|byte| byte == b'-' || byte == b'.' || byte.is_ascii_digit())
            {
                return Err(format!(
                    "the path command '{command}' is short of numbers at byte {}",
                    self.at
                ));
            }
            *slot = self.number()?;
        }
        Ok(out)
    }

    /// An arc flag: one character, `0` or `1`.
    fn flag(&mut self, command: char) -> Result<bool, String> {
        self.skip_separator();
        let flag = match self.peek() {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => {
                return Err(format!(
                    "the arc '{command}' wants a flag, 0 or 1, at byte {}",
                    self.at
                ));
            }
        };
        self.at += 1;
        Ok(flag)
    }
}

/// The numbers of a list: `.6 1.2.8 -3 10-10` is six of them.
pub fn numbers(text: &str) -> Result<Vec<f64>, String> {
    let mut scan = Scan::new(text);
    let mut out = Vec::new();
    scan.skip_space();
    while scan.peek().is_some() {
        out.push(scan.number()?);
        scan.skip_separator();
    }
    Ok(out)
}

/// The one number of an attribute.
fn number(key: &str, value: &str) -> Result<f64, String> {
    match numbers(value) {
        Ok(list) if list.len() == 1 => Ok(list[0]),
        Ok(_) => Err(format!("{key}=\"{value}\" is not one number")),
        Err(error) => Err(format!("{key}=\"{value}\": {error}")),
    }
}

// ---- path ----

/// The path data of a `d` attribute, every point made absolute and `H`, `V` turned into lines.
/// The commands are those of the drawings: `M L H V C Q Z` and the relative `a c q v`.
pub fn path(d: &str) -> Result<Vec<Segment>, String> {
    let mut scan = Scan::new(d);
    let mut out = Vec::new();
    let (mut x, mut y) = (0.0_f64, 0.0_f64);
    let (mut start_x, mut start_y) = (0.0_f64, 0.0_f64);
    let mut open = false;
    let mut previous = None;
    loop {
        scan.skip_space();
        let Some(byte) = scan.peek() else { break };
        if !byte.is_ascii_alphabetic() {
            return Err(match previous {
                None => "a path begins with M".to_owned(),
                Some(command) => format!(
                    "more numbers after '{command}' than it takes, at byte {}: a command repeated \
                     without its letter is not in the subset",
                    scan.at
                ),
            });
        }
        let command = char::from(byte);
        scan.at += 1;
        if previous.is_none() && command != 'M' {
            return Err(format!("a path begins with M, not '{command}'"));
        }
        // After Z the pen is back at the start of the figure, and the next figure begins there.
        if command != 'M' && !open {
            out.push(Segment::Move(point(start_x, start_y)));
            open = true;
        }
        match command {
            'M' => {
                [x, y] = scan.args(command, true)?;
                (start_x, start_y) = (x, y);
                out.push(Segment::Move(point(x, y)));
            }
            'L' => {
                [x, y] = scan.args(command, true)?;
                out.push(Segment::Line(point(x, y)));
            }
            'H' => {
                [x] = scan.args(command, true)?;
                out.push(Segment::Line(point(x, y)));
            }
            'V' => {
                [y] = scan.args(command, true)?;
                out.push(Segment::Line(point(x, y)));
            }
            'v' => {
                let [dy] = scan.args(command, true)?;
                y += dy;
                out.push(Segment::Line(point(x, y)));
            }
            'C' => {
                let [x1, y1, x2, y2, x3, y3] = scan.args(command, true)?;
                out.push(Segment::Cubic(point(x1, y1), point(x2, y2), point(x3, y3)));
                (x, y) = (x3, y3);
            }
            'c' => {
                let [x1, y1, x2, y2, x3, y3] = scan.args(command, true)?;
                out.push(Segment::Cubic(
                    point(x + x1, y + y1),
                    point(x + x2, y + y2),
                    point(x + x3, y + y3),
                ));
                (x, y) = (x + x3, y + y3);
            }
            'Q' => {
                let [x1, y1, x2, y2] = scan.args(command, true)?;
                out.push(Segment::Quad(point(x1, y1), point(x2, y2)));
                (x, y) = (x2, y2);
            }
            'q' => {
                let [x1, y1, x2, y2] = scan.args(command, true)?;
                out.push(Segment::Quad(point(x + x1, y + y1), point(x + x2, y + y2)));
                (x, y) = (x + x2, y + y2);
            }
            'a' => {
                let [rx, ry, rotation] = scan.args(command, true)?;
                let large = scan.flag(command)?;
                let sweep = scan.flag(command)?;
                let [dx, dy] = scan.args(command, false)?;
                (x, y) = (x + dx, y + dy);
                if rx < 0.0 || ry < 0.0 {
                    return Err(format!("the arc '{command}' has a negative radius"));
                }
                // SVG draws an arc with a zero radius as a straight line to its end.
                out.push(if rx == 0.0 || ry == 0.0 {
                    Segment::Line(point(x, y))
                } else {
                    Segment::Arc {
                        to: point(x, y),
                        rx: rx as f32,
                        ry: ry as f32,
                        rotation: rotation as f32,
                        large,
                        sweep,
                    }
                });
            }
            'Z' => {
                out.push(Segment::Close);
                (x, y) = (start_x, start_y);
                open = false;
            }
            _ => return Err(format!("the path command '{command}' is not in the subset")),
        }
        if command == 'M' {
            open = true;
        }
        previous = Some(command);
    }
    if out.is_empty() {
        return Err("an empty path".to_owned());
    }
    Ok(out)
}

// ---- transform ----

/// A matrix being folded, in f64 so that a chain of groups does not gather the error of f32.
#[derive(Clone, Copy, PartialEq)]
struct Affine([f64; 6]);

const IDENTITY: Affine = Affine([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

impl Affine {
    /// `self · next`: `next` acts on a point first, as the later function of a transform list does.
    fn then(self, next: Affine) -> Affine {
        let [a, b, c, d, e, f] = self.0;
        let [na, nb, nc, nd, ne, nf] = next.0;
        Affine([
            a * na + c * nb,
            b * na + d * nb,
            a * nc + c * nd,
            b * nc + d * nd,
            a * ne + c * nf + e,
            b * ne + d * nf + f,
        ])
    }

    fn matrix(self) -> Matrix {
        Matrix(self.0.map(|value| value as f32))
    }
}

/// `rotate(degrees cx cy)`: translate(cx cy) · rotate(degrees) · translate(-cx -cy).
fn rotate(degrees: f64, cx: f64, cy: f64) -> Affine {
    // A quarter turn is exact: cos 90° in floating point is 6e-17, not 0.
    let quarters = degrees / 90.0;
    let (sin, cos) = if quarters.fract() == 0.0 {
        match quarters.rem_euclid(4.0) as u8 {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        }
    } else {
        degrees.to_radians().sin_cos()
    };
    Affine([
        cos,
        sin,
        -sin,
        cos,
        cx - cos * cx + sin * cy,
        cy - sin * cx - cos * cy,
    ])
}

/// A `transform` list, folded in the order written: `translate(x y)`, `scale(s)` and
/// `rotate(a cx cy)` -- the three forms of the drawings, and no other.
fn transform(value: &str) -> Result<Affine, String> {
    let shape = || {
        format!(
            "transform=\"{value}\" is not in the subset: translate(x y), scale(s), rotate(a cx cy)"
        )
    };
    let mut folded = IDENTITY;
    let mut rest = value.trim();
    if rest.is_empty() {
        return Err(shape());
    }
    while !rest.is_empty() {
        let open = rest.find('(').ok_or_else(shape)?;
        let close = rest
            .find(')')
            .filter(|close| *close > open)
            .ok_or_else(shape)?;
        let name = rest[..open].trim_end();
        let args =
            numbers(&rest[open + 1..close]).map_err(|error| format!("{}: {error}", shape()))?;
        let step = match (name, args.as_slice()) {
            ("translate", &[tx, ty]) => Affine([1.0, 0.0, 0.0, 1.0, tx, ty]),
            ("scale", &[s]) => Affine([s, 0.0, 0.0, s, 0.0, 0.0]),
            ("rotate", &[angle, cx, cy]) => rotate(angle, cx, cy),
            _ => return Err(shape()),
        };
        folded = folded.then(step);
        rest = rest[close + 1..].trim_start();
    }
    Ok(folded)
}

// ---- paint ----

fn color(key: &str, value: &str) -> Result<Rgb, String> {
    let hex = value
        .strip_prefix('#')
        .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| format!("{key}=\"{value}\" is not in the subset: #RRGGBB or none"))?;
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or(0);
    Ok(Rgb {
        r: byte(0),
        g: byte(2),
        b: byte(4),
    })
}

fn dash(value: &str) -> Result<&'static [f32], String> {
    let refuse =
        || format!("stroke-dasharray=\"{value}\" is not in the subset: pairs of a dash and a gap");
    let list = numbers(value).map_err(|_| refuse())?;
    if list.is_empty()
        || list.len() % 2 != 0
        || list.iter().any(|length| *length < 0.0)
        || list.iter().sum::<f64>() <= 0.0
    {
        return Err(refuse());
    }
    Ok(leak(list.into_iter().map(|length| length as f32).collect()))
}

/// The attributes of one element, taken one by one; what is left over is outside the subset.
struct Attrs<'a> {
    element: &'a str,
    at: usize,
    list: Vec<(&'a str, &'a str)>,
}

impl<'a> Attrs<'a> {
    fn fail(&self, what: impl std::fmt::Display) -> String {
        format!("byte {}: <{}>: {what}", self.at, self.element)
    }

    fn take(&mut self, key: &str) -> Option<&'a str> {
        let index = self.list.iter().position(|(name, _)| *name == key)?;
        Some(self.list.remove(index).1)
    }

    fn number(&mut self, key: &str) -> Result<Option<f64>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(value) => number(key, value)
                .map(Some)
                .map_err(|error| self.fail(error)),
        }
    }

    fn positive(&mut self, key: &str) -> Result<f64, String> {
        self.positive_or(key, None)
    }

    /// A number above zero; `default` when the attribute is not written, or an error without one.
    fn positive_or(&mut self, key: &str, default: Option<f64>) -> Result<f64, String> {
        let Some(value) = self.take(key) else {
            return default.ok_or_else(|| self.fail(format!("{key} is missing")));
        };
        match number(key, value) {
            Ok(number) if number > 0.0 => Ok(number),
            Ok(_) => Err(self.fail(format!("{key}=\"{value}\" must be above zero"))),
            Err(error) => Err(self.fail(error)),
        }
    }

    /// Fill and stroke with the defaults of SVG: no `fill` is black, no `stroke` is none, no
    /// `stroke-width` is 1, the ends `butt`, the corners `miter`, the line solid and opaque.
    fn paint(&mut self) -> Result<(Option<Rgb>, Option<Stroke>), String> {
        let fill = match self.take("fill") {
            None => Some(BLACK),
            Some("none") => None,
            Some(value) => Some(color("fill", value).map_err(|error| self.fail(error))?),
        };
        let stroke = match self.take("stroke") {
            None | Some("none") => None,
            Some(value) => Some(color("stroke", value).map_err(|error| self.fail(error))?),
        };
        let width = self.positive_or("stroke-width", Some(1.0))?;
        let opacity = match self.number("stroke-opacity")? {
            None => 1.0,
            Some(opacity) if (0.0..=1.0).contains(&opacity) => opacity,
            Some(opacity) => {
                return Err(self.fail(format!("stroke-opacity=\"{opacity}\" is outside 0..1")));
            }
        };
        let cap = match self.take("stroke-linecap") {
            None => Cap::Butt,
            Some("round") => Cap::Round,
            Some(value) => {
                return Err(self.fail(format!(
                    "stroke-linecap=\"{value}\" is not in the subset: round"
                )));
            }
        };
        let join = match self.take("stroke-linejoin") {
            None => Join::Miter,
            Some("round") => Join::Round,
            Some(value) => {
                return Err(self.fail(format!(
                    "stroke-linejoin=\"{value}\" is not in the subset: round"
                )));
            }
        };
        let dash = match self.take("stroke-dasharray") {
            None => &[][..],
            Some(value) => dash(value).map_err(|error| self.fail(error))?,
        };
        Ok((
            fill,
            stroke.map(|color| Stroke {
                color,
                width: width as f32,
                opacity: opacity as f32,
                cap,
                join,
                dash,
            }),
        ))
    }

    fn done(self) -> Result<(), String> {
        match self.list.first() {
            None => Ok(()),
            Some((key, value)) => Err(self.fail(format!(
                "the attribute {key}=\"{value}\" is not in the subset"
            ))),
        }
    }
}

// ---- the document ----

enum Tag<'a> {
    Open {
        name: &'a str,
        attrs: Vec<(&'a str, &'a str)>,
        empty: bool,
        at: usize,
    },
    Close {
        name: &'a str,
        at: usize,
    },
    End,
}

fn attributes(mut rest: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut list: Vec<(&str, &str)> = Vec::new();
    loop {
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            return Ok(list);
        }
        if trimmed.len() == rest.len() {
            return Err(format!("{trimmed:?}: attributes are parted by spaces"));
        }
        let equals = trimmed
            .find('=')
            .ok_or_else(|| format!("{trimmed:?} is not an attribute"))?;
        let key = trimmed[..equals].trim_end();
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_'))
        {
            return Err(format!("{key:?} is not an attribute name"));
        }
        let after = trimmed[equals + 1..].trim_start();
        let quote = after
            .chars()
            .next()
            .filter(|quote| matches!(quote, '"' | '\''))
            .ok_or_else(|| format!("{key}: the value is not in quotes"))?;
        let length = after[1..]
            .find(quote)
            .ok_or_else(|| format!("{key}: the quote is not closed"))?;
        let value = &after[1..1 + length];
        if value.contains(['&', '<']) {
            return Err(format!("{key}=\"{value}\": an entity is not in the subset"));
        }
        if list.iter().any(|(name, _)| *name == key) {
            return Err(format!("{key} twice"));
        }
        list.push((key, value));
        rest = &after[length + 2..];
    }
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
    clips: Vec<(&'a str, Circle)>,
    shapes: Vec<Shape>,
    clip: Option<Circle>,
    /// The clipped group is closed: nothing more may be drawn outside it.
    clip_closed: bool,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<Tag<'a>, String> {
        let rest = &self.text[self.at..];
        let lt = rest.find('<').unwrap_or(rest.len());
        let between = rest[..lt].trim();
        if !between.is_empty() {
            return Err(format!(
                "byte {}: the text {between:?} is not in the subset",
                self.at
            ));
        }
        self.at += lt;
        let at = self.at;
        if at == self.text.len() {
            return Ok(Tag::End);
        }
        let rest = &self.text[at..];
        let gt = rest
            .find('>')
            .ok_or_else(|| format!("byte {at}: a tag without its '>'"))?;
        self.at = at + gt + 1;
        let inner = &rest[1..gt];
        if inner.starts_with(['!', '?']) {
            let shown: String = rest[..=gt].chars().take(32).collect();
            return Err(format!(
                "byte {at}: {shown} -- a comment or a declaration is not in the subset"
            ));
        }
        if let Some(name) = inner.strip_prefix('/') {
            return Ok(Tag::Close {
                name: name.trim_end(),
                at,
            });
        }
        let (inner, empty) = match inner.strip_suffix('/') {
            Some(inner) => (inner, true),
            None => (inner, false),
        };
        let name_end = inner
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(inner.len());
        let name = &inner[..name_end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_'))
        {
            return Err(format!("byte {at}: <{name}> is not an element"));
        }
        let attrs = attributes(&inner[name_end..])
            .map_err(|error| format!("byte {at}: <{name}>: {error}"))?;
        Ok(Tag::Open {
            name,
            attrs,
            empty,
            at,
        })
    }

    /// The next tag must close `name`.
    fn close(&mut self, name: &str) -> Result<(), String> {
        match self.next()? {
            Tag::Close { name: closed, .. } if closed == name => Ok(()),
            Tag::Open {
                name: found, at, ..
            }
            | Tag::Close { name: found, at } => Err(format!(
                "byte {at}: <{found}> where </{name}> must be: <{name}> holds nothing else"
            )),
            Tag::End => Err(format!("</{name}> is missing")),
        }
    }

    fn drawing(&mut self) -> Result<Drawing, String> {
        let (list, empty, at) = match self.next()? {
            Tag::Open {
                name: "svg",
                attrs,
                empty,
                at,
            } => (attrs, empty, at),
            Tag::Open { name, at, .. } | Tag::Close { name, at } => {
                return Err(format!(
                    "byte {at}: a drawing begins with <svg>, not <{name}>"
                ));
            }
            Tag::End => return Err("an empty file".to_owned()),
        };
        let mut attrs = Attrs {
            element: "svg",
            at,
            list,
        };
        let (width, height) = field(&mut attrs)?;
        attrs.done()?;
        if empty {
            return Err("an <svg> without shapes".to_owned());
        }
        loop {
            match self.next()? {
                Tag::Close { name: "svg", .. } => break,
                Tag::Open {
                    name: "defs",
                    attrs,
                    empty,
                    at,
                } => {
                    Attrs {
                        element: "defs",
                        at,
                        list: attrs,
                    }
                    .done()?;
                    if !empty {
                        self.defs()?;
                    }
                }
                Tag::Open {
                    name,
                    attrs,
                    empty,
                    at,
                } => self.drawable(name, attrs, empty, at, IDENTITY, true)?,
                Tag::Close { name, at } => {
                    return Err(format!("byte {at}: </{name}> closes nothing open"));
                }
                Tag::End => return Err("</svg> is missing".to_owned()),
            }
        }
        match self.next()? {
            Tag::End => Ok(Drawing {
                width: width as f32,
                height: height as f32,
                clip: self.clip,
                shapes: leak(std::mem::take(&mut self.shapes)),
            }),
            Tag::Open { at, .. } | Tag::Close { at, .. } => {
                Err(format!("byte {at}: something after </svg>"))
            }
        }
    }

    /// `<defs>`: circles to clip to, and nothing else.
    fn defs(&mut self) -> Result<(), String> {
        loop {
            let (list, empty, at) = match self.next()? {
                Tag::Close { name: "defs", .. } => return Ok(()),
                Tag::Open {
                    name: "clipPath",
                    attrs,
                    empty,
                    at,
                } => (attrs, empty, at),
                Tag::Open { name, at, .. } => {
                    return Err(format!("byte {at}: <{name}> is not in the subset"));
                }
                Tag::Close { name, at } => {
                    return Err(format!("byte {at}: </{name}> inside <defs>"));
                }
                Tag::End => return Err("</defs> is missing".to_owned()),
            };
            let mut attrs = Attrs {
                element: "clipPath",
                at,
                list,
            };
            let id = attrs
                .take("id")
                .ok_or_else(|| attrs.fail("id is missing"))?;
            attrs.done()?;
            if empty {
                return Err(format!("byte {at}: <clipPath> holds one <circle>"));
            }
            let circle = match self.next()? {
                Tag::Open {
                    name: "circle",
                    attrs,
                    empty,
                    at,
                } => {
                    let mut attrs = Attrs {
                        element: "circle",
                        at,
                        list: attrs,
                    };
                    let circle = Circle {
                        cx: attrs.number("cx")?.unwrap_or(0.0) as f32,
                        cy: attrs.number("cy")?.unwrap_or(0.0) as f32,
                        r: attrs.positive("r")? as f32,
                    };
                    attrs.done()?;
                    if !empty {
                        self.close("circle")?;
                    }
                    circle
                }
                Tag::Open { name, at, .. } | Tag::Close { name, at } => {
                    return Err(format!(
                        "byte {at}: <clipPath> holds one <circle>, not <{name}>"
                    ));
                }
                Tag::End => return Err("</clipPath> is missing".to_owned()),
            };
            self.close("clipPath")?;
            if self.clips.iter().any(|(name, _)| *name == id) {
                return Err(format!("byte {at}: a second <clipPath id=\"{id}\">"));
            }
            self.clips.push((id, circle));
        }
    }

    /// A group or a shape, under the folded matrix `outer`; `top` is a child of `<svg>`.
    fn drawable(
        &mut self,
        name: &'a str,
        list: Vec<(&'a str, &'a str)>,
        empty: bool,
        at: usize,
        outer: Affine,
        top: bool,
    ) -> Result<(), String> {
        if top && self.clip_closed {
            return Err(format!(
                "byte {at}: <{name}> after the clipped group: in the subset the clip holds the whole drawing"
            ));
        }
        let mut attrs = Attrs {
            element: name,
            at,
            list,
        };
        let own = match attrs.take("transform") {
            None => IDENTITY,
            Some(value) => transform(value).map_err(|error| attrs.fail(error))?,
        };
        let matrix = outer.then(own);
        let geometry = match name {
            "g" => return self.group(attrs, empty, matrix, top),
            "circle" => {
                let cx = attrs.number("cx")?.unwrap_or(0.0);
                let cy = attrs.number("cy")?.unwrap_or(0.0);
                let r = attrs.positive("r")?;
                Geometry::Ellipse {
                    cx: cx as f32,
                    cy: cy as f32,
                    rx: r as f32,
                    ry: r as f32,
                }
            }
            "ellipse" => {
                let cx = attrs.number("cx")?.unwrap_or(0.0);
                let cy = attrs.number("cy")?.unwrap_or(0.0);
                let rx = attrs.positive("rx")?;
                let ry = attrs.positive("ry")?;
                Geometry::Ellipse {
                    cx: cx as f32,
                    cy: cy as f32,
                    rx: rx as f32,
                    ry: ry as f32,
                }
            }
            "rect" => {
                let x = attrs.number("x")?.unwrap_or(0.0);
                let y = attrs.number("y")?.unwrap_or(0.0);
                let width = attrs.positive("width")?;
                let height = attrs.positive("height")?;
                let rx = match attrs.number("rx")? {
                    None => 0.0,
                    Some(rx) if rx >= 0.0 => rx,
                    Some(rx) => return Err(attrs.fail(format!("rx=\"{rx}\" is negative"))),
                };
                // SVG: ry is rx when only rx is written; then each is clamped to half its side.
                Geometry::Rect {
                    x: x as f32,
                    y: y as f32,
                    width: width as f32,
                    height: height as f32,
                    rx: rx.min(width / 2.0) as f32,
                    ry: rx.min(height / 2.0) as f32,
                }
            }
            "path" => {
                let d = attrs.take("d").ok_or_else(|| attrs.fail("d is missing"))?;
                Geometry::Path(leak(path(d).map_err(|error| attrs.fail(error))?))
            }
            _ => return Err(format!("byte {at}: <{name}> is not in the subset")),
        };
        let (fill, stroke) = attrs.paint()?;
        attrs.done()?;
        if !empty {
            self.close(name)?;
        }
        self.shapes.push(Shape {
            geometry,
            matrix: matrix.matrix(),
            fill,
            stroke,
        });
        Ok(())
    }

    fn group(
        &mut self,
        mut attrs: Attrs<'a>,
        empty: bool,
        matrix: Affine,
        top: bool,
    ) -> Result<(), String> {
        let at = attrs.at;
        let clipped = match attrs.take("clip-path") {
            None => false,
            Some(value) => {
                let id = value
                    .strip_prefix("url(#")
                    .and_then(|rest| rest.strip_suffix(')'))
                    .ok_or_else(|| {
                        attrs.fail(format!(
                            "clip-path=\"{value}\" is not in the subset: url(#id)"
                        ))
                    })?;
                let circle = self
                    .clips
                    .iter()
                    .find(|(name, _)| *name == id)
                    .map(|(_, circle)| *circle)
                    .ok_or_else(|| {
                        attrs.fail(format!(
                            "clip-path=\"{value}\": no <clipPath id=\"{id}\"> before it"
                        ))
                    })?;
                if !top || !self.shapes.is_empty() || self.clip.is_some() || matrix != IDENTITY {
                    return Err(attrs.fail(
                        "clip-path is in the subset only on the first group of the drawing, \
                         untransformed and around all of it",
                    ));
                }
                self.clip = Some(circle);
                true
            }
        };
        attrs.done()?;
        if !empty {
            loop {
                match self.next()? {
                    Tag::Close { name: "g", .. } => break,
                    Tag::Open {
                        name,
                        attrs,
                        empty,
                        at,
                    } => self.drawable(name, attrs, empty, at, matrix, false)?,
                    Tag::Close { name, at } => {
                        return Err(format!("byte {at}: </{name}> inside <g>"));
                    }
                    Tag::End => return Err(format!("byte {at}: <g> is not closed")),
                }
            }
        }
        if clipped {
            self.clip_closed = true;
        }
        Ok(())
    }
}

/// The field of `<svg>`: `viewBox="0 0 width height"`, with `width` and `height`, if written, the
/// same numbers.
fn field(attrs: &mut Attrs) -> Result<(f64, f64), String> {
    match attrs.take("xmlns") {
        Some("http://www.w3.org/2000/svg") => {}
        other => return Err(attrs.fail(format!("xmlns {other:?} is not the namespace of SVG"))),
    }
    let view = attrs
        .take("viewBox")
        .ok_or_else(|| attrs.fail("viewBox is missing"))?;
    let (width, height) = match numbers(view).as_deref() {
        Ok(&[x, y, width, height]) if x == 0.0 && y == 0.0 && width > 0.0 && height > 0.0 => {
            (width, height)
        }
        _ => {
            return Err(attrs.fail(format!(
                "viewBox=\"{view}\" is not in the subset: 0 0 width height"
            )));
        }
    };
    for (key, size) in [("width", width), ("height", height)] {
        if let Some(written) = attrs.number(key)?
            && written != size
        {
            return Err(attrs.fail(format!("{key}=\"{written}\" differs from the viewBox")));
        }
    }
    Ok((width, height))
}

/// One drawing. `file` names it in every error.
pub fn parse(file: &str, text: &str) -> Result<Drawing, String> {
    Reader {
        text,
        at: 0,
        clips: Vec::new(),
        shapes: Vec::new(),
        clip: None,
        clip_closed: false,
    }
    .drawing()
    .map_err(|error| format!("{file}: {error}"))
}

// ---- the data of the program ----

fn rust_point(point: Point) -> String {
    format!("Point {{ x: {:?}, y: {:?} }}", point.x, point.y)
}

fn rust_rgb(rgb: Rgb) -> String {
    format!("Rgb {{ r: {}, g: {}, b: {} }}", rgb.r, rgb.g, rgb.b)
}

fn rust_segment(segment: &Segment) -> String {
    match *segment {
        Segment::Move(to) => format!("Segment::Move({})", rust_point(to)),
        Segment::Line(to) => format!("Segment::Line({})", rust_point(to)),
        Segment::Cubic(first, second, to) => format!(
            "Segment::Cubic({}, {}, {})",
            rust_point(first),
            rust_point(second),
            rust_point(to)
        ),
        Segment::Quad(control, to) => {
            format!("Segment::Quad({}, {})", rust_point(control), rust_point(to))
        }
        Segment::Arc {
            to,
            rx,
            ry,
            rotation,
            large,
            sweep,
        } => format!(
            "Segment::Arc {{ to: {}, rx: {rx:?}, ry: {ry:?}, rotation: {rotation:?}, large: {large}, sweep: {sweep} }}",
            rust_point(to)
        ),
        Segment::Close => "Segment::Close".to_owned(),
    }
}

fn rust_geometry(geometry: &Geometry) -> String {
    match *geometry {
        Geometry::Ellipse { cx, cy, rx, ry } => {
            format!("Geometry::Ellipse {{ cx: {cx:?}, cy: {cy:?}, rx: {rx:?}, ry: {ry:?} }}")
        }
        Geometry::Rect {
            x,
            y,
            width,
            height,
            rx,
            ry,
        } => format!(
            "Geometry::Rect {{ x: {x:?}, y: {y:?}, width: {width:?}, height: {height:?}, rx: {rx:?}, ry: {ry:?} }}"
        ),
        Geometry::Path(segments) => {
            let list: Vec<String> = segments.iter().map(rust_segment).collect();
            format!("Geometry::Path(&[{}])", list.join(", "))
        }
    }
}

fn rust_stroke(stroke: &Stroke) -> String {
    let cap = match stroke.cap {
        Cap::Butt => "Butt",
        Cap::Round => "Round",
    };
    let join = match stroke.join {
        Join::Miter => "Miter",
        Join::Round => "Round",
    };
    format!(
        "Stroke {{ color: {}, width: {:?}, opacity: {:?}, cap: Cap::{cap}, join: Join::{join}, dash: &{:?} }}",
        rust_rgb(stroke.color),
        stroke.width,
        stroke.opacity,
        stroke.dash
    )
}

/// The Rust source of OUT_DIR/art.rs: `DRAWINGS`, one entry per drawing, in the order given. It
/// names the types of this file without a path: the includer brings them into scope.
pub fn to_rust(drawings: &[(String, Drawing)]) -> String {
    let mut out = String::from(
        "// Written by build.rs out of res/art, through build/svg.rs. Do not edit: edit the drawing.\n",
    );
    out.push_str(&format!(
        "pub(crate) static DRAWINGS: [(&str, Drawing); {}] = [\n",
        drawings.len()
    ));
    for (name, drawing) in drawings {
        let clip = match drawing.clip {
            None => "None".to_owned(),
            Some(Circle { cx, cy, r }) => {
                format!("Some(Circle {{ cx: {cx:?}, cy: {cy:?}, r: {r:?} }})")
            }
        };
        out.push_str(&format!(
            "    ({name:?}, Drawing {{ width: {:?}, height: {:?}, clip: {clip}, shapes: &[\n",
            drawing.width, drawing.height
        ));
        for shape in drawing.shapes {
            let fill = shape
                .fill
                .map_or("None".to_owned(), |rgb| format!("Some({})", rust_rgb(rgb)));
            let stroke = shape.stroke.map_or("None".to_owned(), |stroke| {
                format!("Some({})", rust_stroke(&stroke))
            });
            out.push_str(&format!(
                "        Shape {{ geometry: {}, matrix: Matrix({:?}), fill: {fill}, stroke: {stroke} }},\n",
                rust_geometry(&shape.geometry),
                shape.matrix.0
            ));
        }
        out.push_str("    ] }),\n");
    }
    out.push_str("];\n");
    out
}
