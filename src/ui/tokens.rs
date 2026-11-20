//! The style table of the interface shell: §1–§5 of the passport of stage Э120, and nothing else.
//!
//! Every colour, type size, spacing, radius and line of `src\ui\` lives here and only here;
//! `tests\ui_guard.rs` fails if a number of the look turns up anywhere else in the shell. A
//! different look for one button or one window does not exist: a new look is a new entry here,
//! and it acts everywhere at once.
//!
//! `tools\export-tokens.ps1` reads this file **as text** and prints from it the passport tables
//! and the CSS of the mock-ups, so the layout of the entries is part of the contract: one entry
//! per line, the `///` line right above it says what it is in the words of the passport (a `◆`
//! marks a value the owner approves from the element bench), and `// §N.` lines open the
//! sections. An entry in a form the script does not know stops it with the line number.
//!
//! Units: [`Pt`] is a point at 100 % scale (96 DPI); [`Em`] is a share of the window's type size
//! B (§2). The canvas applies the screen scale, in one place.
//!
//! Knows nothing of windows or of Direct2D.

/// The theme of a window: the two columns of §1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

/// A colour as the passport writes it, `#RRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// The colour of `0xRRGGBB`.
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: (hex >> 16) as u8,
            g: (hex >> 8) as u8,
            b: hex as u8,
        }
    }

    /// The colour as `0xRRGGBB`.
    pub const fn hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    /// This colour with `share` of `other` mixed in, channel by channel, rounded to the nearest:
    /// how §5 darkens the main button under the mouse and when pressed.
    pub fn mix(self, other: Rgb, share: f32) -> Rgb {
        let channel = |own: u8, theirs: u8| {
            let value = f32::from(own) * (1.0 - share) + f32::from(theirs) * share;
            value.round().clamp(0.0, 255.0) as u8
        };

        Rgb {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
        }
    }
}

/// The two values of one colour name, §1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pub light: Rgb,
    pub dark: Rgb,
}

impl Pair {
    const fn new(light: u32, dark: u32) -> Self {
        Self {
            light: Rgb::from_hex(light),
            dark: Rgb::from_hex(dark),
        }
    }
}

// §1. Цвета

/// A colour name of §1. The `///` line of each is the «Где» column of the passport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// фон окна
    Bg,
    /// карточка, поле, обычная кнопка, список
    Surface,
    /// панель с рисунком, вложенная карточка, наведение
    Surface2,
    /// разделители, рамка того, что не нажимается
    Line,
    /// точки узора панели, невыполненная часть хода мастера
    Line2,
    /// ◆ рамка всего, что нажимается или заполняется; дорожка выключенного выключателя
    Edge,
    /// заголовки, текст кнопок, рамка «выбрано»
    Ink,
    /// основной текст
    Text,
    /// второй текст: подписи, подсказки, сноски
    Ink2,
    /// только недоступное и подсказка внутри пустого поля
    Ink3,
    /// надпись окна, ссылка, «включено», номера списка, ход мастера
    Accent,
    /// заливка главной кнопки — и больше ничего
    Button,
    /// текст главной кнопки
    ButtonFg,
    /// фон замечания
    Note,
    /// рамка замечания
    NoteLine,
    /// полоса заголовка окна (Windows 11)
    Caption,
    /// точка «нужен перезапуск»
    Warn,
    /// кружок выключателя
    Knob,
}

impl Color {
    /// Every name, in the order of the passport.
    pub const ALL: [Color; 18] = [
        Color::Bg,
        Color::Surface,
        Color::Surface2,
        Color::Line,
        Color::Line2,
        Color::Edge,
        Color::Ink,
        Color::Text,
        Color::Ink2,
        Color::Ink3,
        Color::Accent,
        Color::Button,
        Color::ButtonFg,
        Color::Note,
        Color::NoteLine,
        Color::Caption,
        Color::Warn,
        Color::Knob,
    ];

    /// The name as the passport spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Color::Bg => "bg",
            Color::Surface => "surface",
            Color::Surface2 => "surface2",
            Color::Line => "line",
            Color::Line2 => "line2",
            Color::Edge => "edge",
            Color::Ink => "ink",
            Color::Text => "text",
            Color::Ink2 => "ink2",
            Color::Ink3 => "ink3",
            Color::Accent => "accent",
            Color::Button => "button",
            Color::ButtonFg => "button_fg",
            Color::Note => "note",
            Color::NoteLine => "note_line",
            Color::Caption => "caption",
            Color::Warn => "warn",
            Color::Knob => "knob",
        }
    }

    /// Light and dark. `knob` is `bg` by reference, not a number of its own: the knob of a switch
    /// is the window's background showing through, in either theme.
    pub const fn pair(self) -> Pair {
        match self {
            Color::Bg => Pair::new(0xFAF9F6, 0x23272F),
            Color::Surface => Pair::new(0xFDFCFA, 0x2B303A),
            Color::Surface2 => Pair::new(0xF1EFE9, 0x323845),
            Color::Line => Pair::new(0xE6E3DC, 0x363C47),
            Color::Line2 => Pair::new(0xD3CFC6, 0x46505E),
            Color::Edge => Pair::new(0x8C8880, 0x7A8290),
            Color::Ink => Pair::new(0x1C1B18, 0xE6E9EE),
            Color::Text => Pair::new(0x2B2A26, 0xD6DAE0),
            Color::Ink2 => Pair::new(0x5B5955, 0xA2A8B3),
            Color::Ink3 => Pair::new(0x75726C, 0x8A909B),
            Color::Accent => Pair::new(0x1F7A4A, 0x6CC894),
            Color::Button => Pair::new(0x1C1B18, 0xD6DAE0),
            Color::ButtonFg => Pair::new(0xFFFFFF, 0x23272F),
            Color::Note => Pair::new(0xFBF1DC, 0x3A3322),
            Color::NoteLine => Pair::new(0xECD3A2, 0x5C4C2A),
            Color::Caption => Pair::new(0xF3F2EE, 0x1F232A),
            Color::Warn => Pair::new(0xC98A1B, 0xC98A1B),
            Color::Knob => Color::Bg.pair(),
        }
    }

    /// The value in `theme`.
    pub const fn get(self, theme: Theme) -> Rgb {
        let pair = self.pair();

        match theme {
            Theme::Light => pair.light,
            Theme::Dark => pair.dark,
        }
    }
}

/// The colours of what lies over the picture of the panel (T-120-25, the mock-up's
/// `helloOverlayReal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PictureColours {
    /// The word being typed.
    pub word: Rgb,
    /// The word fixed.
    pub fixed: Rgb,
    /// The frame and the name of a key.
    pub ink: Rgb,
    /// A key.
    pub key: Rgb,
    /// A key pressed.
    pub lit: Rgb,
}

/// The picture is the same in both themes, and so is what lies on it: the light values of §1 —
/// and the pressed key in the accent of the dark theme, as in the mock-up.
pub const fn picture_colours() -> PictureColours {
    PictureColours {
        word: Color::Ink2.pair().light,
        fixed: Color::Accent.pair().light,
        ink: Color::Ink.pair().light,
        key: Color::ButtonFg.pair().light,
        lit: Color::Accent.pair().dark,
    }
}

// §2. Кегль и шрифт

/// A length in points at 100 % scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt(pub f32);

/// A length as a share of the window's type size B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Em(pub f32);

impl Em {
    /// The length in points for the type size `b`.
    pub fn at(self, b: f32) -> f32 {
        self.0 * b
    }
}

/// Пол кегля: ни окно, ни роль, ни окно в углу, ни меню значка не мельче.
pub const TYPE_FLOOR: Pt = Pt(12.0);
/// Кегль окна B — нижняя граница до множителя k.
pub const B_MIN: Pt = Pt(13.0);
/// Кегль окна B — верхняя граница до множителя k.
pub const B_MAX: Pt = Pt(19.0);
/// Кегль окна B по ширине — начало.
pub const B_BY_WIDTH_BASE: Pt = Pt(9.5);
/// Кегль окна B по ширине — прибавка на точку ширины.
pub const B_BY_WIDTH_SLOPE: f32 = 0.005;
/// Кегль окна B по высоте — начало.
pub const B_BY_HEIGHT_BASE: Pt = Pt(6.3);
/// Кегль окна B по высоте — прибавка на точку высоты.
pub const B_BY_HEIGHT_SLOPE: f32 = 0.016;

/// The window's type size B of §2, for the client area `w` × `h` in points at 100 % and the
/// factor `k` of the language and window class:
/// `B = max(12; k × clamp(13; min(9,5 + 0,005·W; 6,3 + 0,016·H); 19))`.
pub fn window_b(w: f32, h: f32, k: f32) -> f32 {
    let by_width = B_BY_WIDTH_BASE.0 + B_BY_WIDTH_SLOPE * w;
    let by_height = B_BY_HEIGHT_BASE.0 + B_BY_HEIGHT_SLOPE * h;

    (k * by_width.min(by_height).clamp(B_MIN.0, B_MAX.0)).max(TYPE_FLOOR.0)
}

/// Подпись под рисунком — доля высоты окна.
pub const BRAND_BY_HEIGHT: f32 = 0.017;
/// Подпись под рисунком — не мельче.
pub const BRAND_MIN: Pt = Pt(13.0);
/// Подпись под рисунком — не крупнее.
pub const BRAND_MAX: Pt = Pt(18.0);

/// The face of a text role: the system's main face, or its monospace one (§2: Segoe UI Variable
/// Text or Segoe UI; Cascadia Mono or Consolas — chosen by `text.rs`, not here).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Main,
    Mono,
}

/// A text role of §2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// заголовок окна
    Title,
    /// вводный абзац
    Lead,
    /// текст, название раздела настроек
    Body,
    /// заголовок карточки, выбранный раздел
    Strong,
    /// текст кнопки
    Button,
    /// подпись поля
    FieldLabel,
    /// ссылка
    Link,
    /// подсказка, сноска
    Small,
    /// надпись над заголовком
    Eyebrow,
    /// метка раздела
    Label,
    /// путь, имя файла, клавиша, текст обращения
    Code,
    /// подпись под рисунком
    Brand,
    /// окно в углу — шапка: о чём окно (постоянный кегль, §2)
    CornerHeader,
    /// окно в углу — заголовок
    CornerTitle,
    /// окно в углу — текст
    CornerText,
    /// окно в углу — подпись флажка
    CornerCheck,
    /// меню значка — выделенный пункт: полужирный, `accent`
    MenuEmphasis,
}

/// How one role sets its text. `size` is a share of B; `line` is the line height as a multiple of
/// the role's own size; `tracking` is letter spacing as a share of the role's own size; `color`
/// is `None` where the element decides (a button's text follows the button).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoleStyle {
    pub size: Em,
    pub line: f32,
    pub weight: u16,
    pub face: Face,
    pub caps: bool,
    pub tracking: f32,
    pub color: Option<Color>,
}

impl RoleStyle {
    const fn main(size: f32, line: f32, weight: u16, color: Option<Color>) -> Self {
        Self {
            size: Em(size),
            line,
            weight,
            face: Face::Main,
            caps: false,
            tracking: 0.0,
            color,
        }
    }

    const fn mono(size: f32, line: f32, weight: u16, color: Option<Color>) -> Self {
        Self {
            face: Face::Mono,
            ..Self::main(size, line, weight, color)
        }
    }

    const fn caps(self, tracking: f32) -> Self {
        Self {
            caps: true,
            tracking,
            ..self
        }
    }
}

impl Role {
    /// Every role, in the order of the passport.
    pub const ALL: [Role; 12] = [
        Role::Title,
        Role::Lead,
        Role::Body,
        Role::Strong,
        Role::Button,
        Role::FieldLabel,
        Role::Link,
        Role::Small,
        Role::Eyebrow,
        Role::Label,
        Role::Code,
        Role::Brand,
    ];

    /// The name as the passport spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Role::Title => "title",
            Role::Lead => "lead",
            Role::Body => "body",
            Role::Strong => "strong",
            Role::Button => "button",
            Role::FieldLabel => "field_label",
            Role::Link => "link",
            Role::Small => "small",
            Role::Eyebrow => "eyebrow",
            Role::Label => "label",
            Role::Code => "code",
            Role::Brand => "brand",
            Role::CornerHeader => "corner_header",
            Role::CornerTitle => "corner_title",
            Role::CornerText => "corner_text",
            Role::CornerCheck => "corner_check",
            Role::MenuEmphasis => "menu_emphasis",
        }
    }

    /// The table of §2. `brand` is sized by the window's height, not by B — see [`role_size`];
    /// its `size` here is unused and zero.
    pub const fn style(self) -> RoleStyle {
        match self {
            Role::Title => RoleStyle::main(1.6, 1.2, 600, Some(Color::Ink)),
            Role::Lead => RoleStyle::main(1.1, 1.5, 400, Some(Color::Ink)),
            Role::Body => RoleStyle::main(1.0, 1.5, 400, Some(Color::Text)),
            Role::Strong => RoleStyle::main(1.0, 1.5, 600, Some(Color::Ink)),
            Role::Button => RoleStyle::main(0.95, 1.3, 500, None),
            Role::FieldLabel => RoleStyle::main(0.9, 1.5, 400, Some(Color::Ink2)),
            Role::Link => RoleStyle::main(0.92, 1.5, 400, Some(Color::Accent)),
            Role::Small => RoleStyle::main(0.86, 1.4, 400, Some(Color::Ink2)),
            Role::Eyebrow => RoleStyle::mono(0.74, 1.5, 600, Some(Color::Accent)).caps(0.08),
            Role::Label => RoleStyle::mono(0.72, 1.5, 600, Some(Color::Ink2)).caps(0.08),
            Role::Code => RoleStyle::mono(0.86, 1.45, 400, Some(Color::Ink)),
            Role::Brand => RoleStyle::mono(0.0, 1.5, 400, Some(Color::Ink2)),
            Role::CornerHeader => RoleStyle::mono(0.0, 1.0, 600, Some(Color::Accent)).caps(0.08),
            Role::CornerTitle => RoleStyle::main(0.0, 1.3, 600, Some(Color::Ink)),
            Role::CornerText => RoleStyle::main(0.0, 1.45, 400, Some(Color::Ink2)),
            Role::CornerCheck => RoleStyle::main(0.0, 1.45, 400, Some(Color::Ink)),
            Role::MenuEmphasis => RoleStyle::main(1.0, 1.5, 600, Some(Color::Accent)),
        }
    }
}

/// Главная кнопка — начертание текста.
pub const BUTTON_PRIMARY_WEIGHT: u16 = 600;
/// Путь в роли `code` — цвет.
pub const CODE_PATH_COLOR: Color = Color::Ink2;
/// Имя в подписи под рисунком — начертание основного шрифта.
pub const BRAND_NAME_WEIGHT: u16 = 600;
/// Имя в подписи под рисунком — цвет.
pub const BRAND_NAME_COLOR: Color = Color::Ink;

/// The size of `role` in points, for the type size `b` of a window whose client area is `h`
/// points high. **The floor of 12 is here**, inside the one function every role is sized by, so
/// no role of any window can come out smaller (§2).
pub fn role_size(role: Role, b: f32, h: f32) -> f32 {
    let size = match role {
        Role::Brand => (BRAND_BY_HEIGHT * h).clamp(BRAND_MIN.0, BRAND_MAX.0),
        // The window in the corner keeps its sizes whatever the screen (§2).
        Role::CornerHeader => CORNER_HEADER.0,
        Role::CornerTitle => CORNER_TITLE.0,
        Role::CornerText => CORNER_TEXT.0,
        Role::CornerCheck => CORNER_CHECK_LABEL.0,
        _ => role.style().size.at(b),
    };

    size.max(TYPE_FLOOR.0)
}

/// Окно в углу — шапка (моноширинный капс 600, `accent`).
pub const CORNER_HEADER: Pt = Pt(12.0);
/// Окно в углу — заголовок.
pub const CORNER_TITLE: Pt = Pt(15.0);
/// Окно в углу — крупный заголовок (1,3 / 600 / `ink`).
pub const CORNER_TITLE_LARGE: Pt = Pt(17.0);
/// Окно в углу — текст (1,45 / `ink2`).
pub const CORNER_TEXT: Pt = Pt(13.5);
/// Окно в углу — дата и «было» (`ink2`).
pub const CORNER_DATE: Pt = Pt(12.5);
/// Окно в углу — ссылка.
pub const CORNER_LINK: Pt = Pt(13.0);
/// Окно в углу — кнопка.
pub const CORNER_BUTTON: Pt = Pt(13.5);
/// Окно в углу — подпись флажка.
pub const CORNER_CHECK_LABEL: Pt = Pt(13.0);
/// Меню значка — строка.
pub const MENU_ROW: Pt = Pt(14.0);
/// Меню значка — клавиша у пункта и состояние.
pub const MENU_KEY: Pt = Pt(12.0);

/// Every fixed size of the corner window and of the tray menu, for the floor test.
pub const FIXED_SIZES: [Pt; 10] = [
    CORNER_HEADER,
    CORNER_TITLE,
    CORNER_TITLE_LARGE,
    CORNER_TEXT,
    CORNER_DATE,
    CORNER_LINK,
    CORNER_BUTTON,
    CORNER_CHECK_LABEL,
    MENU_ROW,
    MENU_KEY,
];

// §3. Отступы и размеры

/// Inner padding: top and bottom × left and right, as the passport writes «поля 0,45 × 1,2 B».
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    pub v: Em,
    pub h: Em,
}

impl Pad {
    const fn em(v: f32, h: f32) -> Self {
        Self { v: Em(v), h: Em(h) }
    }
}

/// Four sides in points, in the passport's order: top, right, bottom, left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sides {
    pub top: Pt,
    pub right: Pt,
    pub bottom: Pt,
    pub left: Pt,
}

impl Sides {
    const fn pt(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top: Pt(top),
            right: Pt(right),
            bottom: Pt(bottom),
            left: Pt(left),
        }
    }
}

/// A width × height in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizePt {
    pub w: Pt,
    pub h: Pt,
}

impl SizePt {
    const fn pt(w: f32, h: f32) -> Self {
        Self { w: Pt(w), h: Pt(h) }
    }
}

/// A width × height in shares of B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeEm {
    pub w: Em,
    pub h: Em,
}

impl SizeEm {
    const fn em(w: f32, h: f32) -> Self {
        Self { w: Em(w), h: Em(h) }
    }
}

/// A share of something the entry names: the work area, the window's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Share(pub f32);

/// `clamp(min; val·B; max)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clamp {
    pub min: Pt,
    pub val: Em,
    pub max: Pt,
}

impl Clamp {
    const fn new(min: Pt, val: Em, max: Pt) -> Self {
        Self { min, val, max }
    }

    /// The length in points for the type size `b`.
    pub fn at(self, b: f32) -> f32 {
        self.val.at(b).clamp(self.min.0, self.max.0)
    }
}

/// The diameter of a picture circle: `min(w·W; h·H; max)` for a window of W × H points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub w: Share,
    pub h: Share,
    pub max: Pt,
}

impl Circle {
    const fn new(w: f32, h: f32, max: Pt) -> Self {
        Self {
            w: Share(w),
            h: Share(h),
            max,
        }
    }

    /// The diameter in points for a window of `w` × `h` points.
    pub fn at(self, w: f32, h: f32) -> f32 {
        (self.w.0 * w).min(self.h.0 * h).min(self.max.0)
    }
}

/// Окно — доля рабочей области монитора.
pub const WINDOW_SHARE: Share = Share(2.0 / 3.0);
/// Малое окно (`host::Content::least`) — его размер и его наименьшая рамка; и мера класса малого
/// экрана в приборе влезания.
pub const WINDOW_MIN: SizePt = SizePt::pt(620.0, 500.0);
/// ◆ Рамка обычного окна оболочки — не меньше (208.198): эталон, в котором окно открывается. Меньше
/// рабочая область (малый экран) — окно во всю неё, и только тогда тело прокручивается.
pub const FRAME_MIN: SizePt = SizePt::pt(900.0, 540.0);
/// ◆ Окно — не больше; рамку можно тянуть в этих пределах, «развернуть» нет.
pub const WINDOW_MAX: SizePt = SizePt::pt(1280.0, 720.0);
/// ◆ Две колонки с панелью — с этой ширины окна (208.168: 880, не 680).
pub const TWO_COLUMNS_FROM: Pt = Pt(880.0);
/// ◆ Окно при открытии — не меньше, пока рабочая область позволяет (208.168).
pub const WINDOW_OPEN_MIN: SizePt = SizePt::pt(900.0, 540.0);
/// ◆ Класс кегля «обычное» — с этого размера окна (208.168).
pub const CLASS_ORDINARY_FROM: SizePt = SizePt::pt(900.0, 540.0);
/// ◆ Класс кегля «большое» — с этого размера окна (208.168).
pub const CLASS_LARGE_FROM: SizePt = SizePt::pt(1280.0, 690.0);
/// Две колонки — панель, доля ширины окна.
pub const PANEL_SHARE: Share = Share(0.38);
/// Колонка содержимого — не шире.
pub const CONTENT_MAX_WIDTH: Em = Em(40.0);
/// Колонка содержимого — поля.
pub const CONTENT_PAD: Pad = Pad::em(2.2, 2.6);
/// Узкое окно — поля.
pub const NARROW_PAD: Pad = Pad::em(1.5, 1.8);
/// Узкое окно — кружок с рисунком рядом с заголовком.
pub const NARROW_ART: Em = Em(4.6);
/// ◆ Пара в ряду — половина не уже, иначе одна под другой (208.168: 15 B; пара — в колонке текста).
pub const PAIR_MIN_HALF: Em = Em(15.0);
/// Между блоками.
pub const BLOCK_GAP: Em = Em(1.0);
/// Панель — точка узора (`line2`).
pub const PANEL_DOT: Pt = Pt(1.2);
/// Панель — шаг узора.
pub const PANEL_DOT_STEP: Pt = Pt(18.0);
/// Рисунок в панели — диаметр круга.
pub const PANEL_ART: Circle = Circle::new(0.29, 0.64, Pt(520.0));
/// Рисунок в окне настроек — диаметр круга.
pub const SETTINGS_ART: Circle = Circle::new(0.17, 0.30, Pt(240.0));
/// ◆ Окно настроек, панель — поля сверху и по бокам (макет `authors.html`, `.panel.setp`).
pub const SETTINGS_PANEL_PAD: Pad = Pad::em(1.8, 1.4);
/// ◆ Окно настроек, панель — от рисунка до столбца разделов.
pub const SETTINGS_PANEL_GAP: Em = Em(1.2);
/// Рисунок — кольцо цвета `bg`.
pub const ART_RING: Pt = Pt(6.0);
/// Подпись под рисунком — от края.
pub const BRAND_FROM_EDGE: Em = Em(1.6);
/// Подпись под рисунком — от низа.
pub const BRAND_FROM_BOTTOM: Em = Em(1.2);
/// ◆ Подпись под рисунком — между именем и адресом, доля кегля подписи (из макета).
pub const BRAND_GAP: Share = Share(0.5);
/// ◆ Поле в ряду с кнопкой — не уже; уже — ряд переносится (в макете `.inrow` минимума нет).
pub const ROW_FIELD_MIN: Em = Em(8.0);
/// ◆ Метка «новое» — от надписи (макет: 0,6 кегля метки).
pub const BADGE_GAP: Em = Em(0.41);
/// ◆ Поверх рисунка — слово: кегль, доля высоты якоря (T-120-25, макет `helloOverlayReal`: 24 из 94).
pub const ON_PICTURE_WORD: Share = Share(24.0 / 94.0);
/// ◆ Поверх рисунка — слово: от левого края якоря, доля его высоты (макет: 16 из 94).
pub const ON_PICTURE_WORD_INSET: Share = Share(16.0 / 94.0);
/// ◆ Поверх рисунка — клавиша: высота, доля высоты якоря (макет: 30 из 42).
pub const ON_PICTURE_KEY: Share = Share(30.0 / 42.0);
/// ◆ Поверх рисунка — клавиша: кегль имени, доля высоты якоря (макет: 16 из 42).
pub const ON_PICTURE_KEY_LABEL: Share = Share(16.0 / 42.0);
/// ◆ Поверх рисунка — клавиша: не уже, доля высоты якоря (макет: 76 из 42).
pub const ON_PICTURE_KEY_MIN: Share = Share(76.0 / 42.0);
/// ◆ Поверх рисунка — клавиша: поле у имени с каждой стороны, доля высоты якоря (макет: 11 из 42).
pub const ON_PICTURE_KEY_PAD: Share = Share(11.0 / 42.0);
/// ◆ Поверх рисунка — между клавишами, доля высоты якоря (макет: 10 из 42).
pub const ON_PICTURE_KEY_GAP: Share = Share(10.0 / 42.0);
/// ◆ Поверх рисунка — клавиша: скругление, доля её высоты (макет: 7 из 30).
pub const ON_PICTURE_KEY_RADIUS: Share = Share(7.0 / 30.0);
/// ◆ Поверх рисунка — клавиша: рамка, доля её высоты (макет: 4 из 30).
pub const ON_PICTURE_KEY_FRAME: Share = Share(4.0 / 30.0);
/// ◆ Поверх рисунка — начертание слова и имени клавиши (моноширинный).
pub const ON_PICTURE_WEIGHT: u16 = 700;
/// Блок — поля.
pub const BLOCK_PAD: Pad = Pad::em(0.8, 1.0);
/// Блок — между строками.
pub const BLOCK_ROW_GAP: Em = Em(0.5);
/// Вложенная карточка — поля.
pub const NESTED_PAD: Pad = Pad::em(0.8, 0.95);
/// Замечание — поля.
pub const NOTE_PAD: Pad = Pad::em(0.65, 0.85);
/// Ряд кнопок окна — отступ над ним, под линией.
pub const BUTTON_ROW_ABOVE: Em = Em(1.0);
/// Ряд кнопок мастера — отступ над ним, под линией.
pub const WIZARD_BUTTON_ROW_ABOVE: Em = Em(0.6);
/// Ряд кнопок — между кнопками.
pub const BUTTON_GAP: Em = Em(0.6);
/// Кнопка — высота не меньше.
pub const BUTTON_MIN_HEIGHT: Em = Em(2.5);
/// Кнопка — поля.
pub const BUTTON_PAD: Pad = Pad::em(0.45, 1.2);
/// Поле — поля.
pub const FIELD_PAD: Pad = Pad::em(0.45, 0.7);
/// Многострочное поле — поля.
pub const MULTILINE_PAD: Pad = Pad::em(0.55, 0.75);
/// Сегмент — поля ячейки.
pub const SEGMENT_PAD: Pad = Pad::em(0.35, 0.9);
/// Группа строк — поля.
pub const ROW_GROUP_PAD: Pad = Pad::em(0.15, 1.0);
/// Группа строк — строка, сверху и снизу; между строками линия `line`.
pub const ROW_GROUP_ROW: Em = Em(0.6);
/// Выключатель — размер.
pub const TOGGLE: SizeEm = SizeEm::em(2.3, 1.3);
/// Выключатель — не меньше.
pub const TOGGLE_MIN: SizePt = SizePt::pt(32.0, 18.0);
/// Выключатель — кружок.
pub const TOGGLE_KNOB: Em = Em(0.9);
/// Выключатель — зазор кружка.
pub const TOGGLE_GAP: Em = Em(0.2);
/// Флажок и радиокнопка — размер.
pub const CHECK: Clamp = Clamp::new(Pt(16.0), Em(1.2), Pt(18.0));
/// Флажок и радиокнопка — рамка.
pub const CHECK_BORDER: Pt = Pt(1.5);
/// Карточка выбора — поля.
pub const CHOICE_PAD: Pad = Pad::em(0.7, 1.0);
/// Карточка выбора в списке вариантов — поля.
pub const CHOICE_IN_LIST_PAD: Pad = Pad::em(0.4, 0.8);
/// Список — поля.
pub const LIST_PAD: Em = Em(0.3);
/// Список — строка, поля.
pub const LIST_ROW_PAD: Pad = Pad::em(0.3, 0.6);
/// Раздел настроек — поля.
pub const SECTION_PAD: Pad = Pad::em(0.5, 0.85);
/// Раздел настроек — между пунктами.
pub const SECTION_GAP: Em = Em(0.25);
/// Ход мастера — высота полоски.
pub const WIZARD_STEP: Em = Em(0.3);
/// Ход мастера — зазор между полосками.
pub const WIZARD_STEP_GAP: Em = Em(0.35);
/// Клавиша — рамка снизу (`edge`; остальные стороны — линия).
pub const KEY_BOTTOM: Pt = Pt(3.0);
/// Клавиша — поля.
pub const KEY_PAD: Pad = Pad::em(0.1, 0.5);
/// Точка состояния — диаметр.
pub const STATUS_DOT: Em = Em(0.55);
/// ◆ Шапка окна — заголовок под надписью (из макета: 0,2 кегля заголовка).
pub const HEADER_TITLE_GAP: Em = Em(0.32);
/// ◆ Шапка окна — подпись под заголовком (из макета: 0,25 кегля подписи).
pub const HEADER_SUBTITLE_GAP: Em = Em(0.215);
/// ◆ Флажок, радиокнопка, выключатель — от подписи (из макета).
pub const CONTROL_GAP: Em = Em(0.6);
/// ◆ Поле — подпись над полем, отступ (из макета: 0,3 кегля).
pub const FIELD_LABEL_GAP: Em = Em(0.3);
/// ◆ Многострочное поле — строк видно.
pub const FIELD_TEXT_LINES: Share = Share(4.0);
/// ◆ Список — строк видно, когда его высоту не задаёт окно.
pub const LIST_VISIBLE_ROWS: Share = Share(6.0);
/// ◆ Список — ширина своей тонкой полосы прокрутки.
pub const LIST_BAR: Pt = Pt(4.0);
/// ◆ Список — строк за один щелчок колеса мыши.
pub const LIST_WHEEL_ROWS: Share = Share(3.0);
/// Окно в углу — ширина.
pub const CORNER_WIDTH: Pt = Pt(364.0);
/// Окно в углу — поля.
pub const CORNER_PAD: Sides = Sides::pt(10.0, 12.0, 16.0, 16.0);
/// Окно в углу — между частями.
pub const CORNER_GAP: Pt = Pt(12.0);
/// Окно в углу — кружок с пандой.
pub const CORNER_ART: Pt = Pt(44.0);
/// ◆ Окно в углу — высота строки шапки (макет: 28).
pub const CORNER_HEADER_ROW: Pt = Pt(28.0);
/// ◆ Окно в углу — значок программы в шапке.
pub const CORNER_ICON: Pt = Pt(16.0);
/// ◆ Окно в углу — между значком, названием и крестиком.
pub const CORNER_ICON_GAP: Pt = Pt(8.0);
/// ◆ Окно в углу — между заголовком и текстом.
pub const CORNER_TITLE_GAP: Pt = Pt(3.0);
/// ◆ Окно в углу — между кнопками.
pub const CORNER_BUTTON_GAP: Pt = Pt(8.0);
/// ◆ Окно в углу — доля второй кнопки ряда (слева).
pub const CORNER_SECOND_SHARE: Share = Share(1.0);
/// ◆ Окно в углу — доля главной кнопки ряда (справа).
pub const CORNER_MAIN_SHARE: Share = Share(2.0);
/// Окно в углу — высота картинки панели задач в «Обновлено» (окно Э119: 150).
pub const CORNER_PICTURE: Pt = Pt(150.0);
/// ◆ Окно в углу — знак крестика (макет: 13 пт шрифта, знак около 10).
pub const CORNER_CROSS: Pt = Pt(10.0);
/// Окно в углу — кнопка не ниже.
pub const CORNER_BUTTON_MIN_HEIGHT: Pt = Pt(40.0);
/// Меню значка — ширина от.
pub const MENU_MIN_WIDTH: Pt = Pt(320.0);
/// Меню значка — строка не ниже.
pub const MENU_ROW_MIN_HEIGHT: Pt = Pt(36.0);
/// Меню значка — значок.
pub const MENU_ICON: Pt = Pt(18.0);
/// Меню значка — линия значка.
pub const MENU_ICON_LINE: Pt = Pt(1.6);
/// ◆ Меню значка — поля окна (макет `menu.html`: 6).
pub const MENU_PAD: Pt = Pt(6.0);
/// ◆ Меню значка — шапка: поля по вертикали и по горизонтали (8, 10).
pub const MENU_HEAD_PAD: SizePt = SizePt::pt(10.0, 8.0);
/// ◆ Меню значка — шапка: точка состояния (7).
pub const MENU_STATE_DOT: Pt = Pt(7.0);
/// ◆ Меню значка — шапка: зазор от точки до слов (6).
pub const MENU_STATE_GAP: Pt = Pt(6.0);
/// ◆ Меню значка — пункт: поля по горизонтали (10).
pub const MENU_ITEM_PAD: Pt = Pt(10.0);
/// ◆ Меню значка — пункт: столбец значка (20).
pub const MENU_ICON_COLUMN: Pt = Pt(20.0);
/// ◆ Меню значка — пункт: зазоры между значком, подписью и концом строки (10).
pub const MENU_ITEM_GAP: Pt = Pt(10.0);
/// ◆ Меню значка — точка нового у пункта (8).
pub const MENU_DOT: Pt = Pt(8.0);
/// ◆ Меню значка — линия между группами: поля по вертикали и по горизонтали (5, 8).
pub const MENU_RULE_PAD: SizePt = SizePt::pt(8.0, 5.0);
/// ◆ Меню значка — кегль, от которого считаются его роли (строка — 14 т., §2).
pub const MENU_B: Pt = Pt(14.0);

// §4. Скругления, линии, тени

/// Скругление малое: поле, сегментный переключатель, строка списка, тихая кнопка.
pub const RADIUS_SMALL: Pt = Pt(6.0);
/// Скругление среднее: кнопка, блок, карточки, замечание, список, группа строк, раздел, строка меню.
pub const RADIUS_MEDIUM: Pt = Pt(8.0);
/// Скругление большое: окно в углу, меню значка, всплывающий список.
pub const RADIUS_LARGE: Pt = Pt(12.0);
/// Скругление флажка и клавиши.
pub const RADIUS_CHECK: Pt = Pt(4.0);
/// Линия: разделители, рамки, граница панели.
pub const LINE: Pt = Pt(1.0);
/// Рамка «выбрано» (`ink`).
pub const SELECTED_BORDER: Pt = Pt(2.0);
/// Полоса текущего раздела (`accent`).
pub const CURRENT_STRIPE: Pt = Pt(3.0);
/// ◆ Рамка клавиатуры — кольцо (`accent`).
pub const FOCUS_RING: Pt = Pt(2.0);
/// ◆ Рамка клавиатуры — зазор.
pub const FOCUS_GAP: Pt = Pt(2.0);
/// Ссылка — подчёркивание ниже базовой линии.
pub const LINK_UNDERLINE: Pt = Pt(2.0);

// §5. Состояния

/// ◆ Главная кнопка под мышью — доля `bg` в `button`.
pub const HOVER_MIX: Share = Share(0.12);
/// ◆ Главная кнопка нажата — доля `bg` в `button`.
pub const PRESSED_MIX: Share = Share(0.20);

/// ◆ The shapes of elements the passport draws in words and not in numbers — approved from the
/// element bench (T-120-6). Kept apart from the sections above: they are not rows of the passport
/// tables, and `tools\export-tokens.ps1` does not print them.
pub mod shape {
    use super::{Color, Em, Share};

    /// ◆ The steps of a fill: as it is, under the mouse, pressed (§5: «на ступень темнее», «ещё на
    /// ступень»). An element with no fill of its own starts before the first step. A fill that starts
    /// on `surface2` (a cell of a segmented switch) goes on to `line2`.
    pub const FILL_STEPS: [Color; 4] = [Color::Surface, Color::Surface2, Color::Line, Color::Line2];
    /// ◆ The tick of a check box: three points as shares of its side, left to right.
    pub const CHECK_TICK: [(f32, f32); 3] = [(0.24, 0.52), (0.43, 0.7), (0.76, 0.32)];
    /// ◆ The dot of a radio button, as a share of its side.
    pub const RADIO_DOT: Share = Share(0.5);
    /// ◆ The corner mark of a choice field: its width; it is half as high.
    pub const CHEVRON: Em = Em(0.6);
}

/// ◆ The one movement of the shell (TZ-UI-SHELL: «Движение»): a window in the corner appears from
/// the side of the screen it stands at — the mock-up's `nw-in` (letters.html: `.34s
/// cubic-bezier(.2, .9, .3, 1)`, `translateX(28px)`). Without movement when Windows has its
/// animation off. Kept apart: not a row of the passport tables.
pub mod motion {
    use super::Pt;

    /// ◆ How long the window comes in, ms.
    pub const APPEAR_MS: u32 = 340;
    /// ◆ How far it comes from.
    pub const APPEAR_SLIDE: Pt = Pt(28.0);
    /// One step of the movement, ms.
    pub const FRAME_MS: u32 = 16;
    /// One frame of the picture of the taskbar in «Обновлено», ms (the window of E119: 40).
    pub const PICTURE_FRAME_MS: u32 = 40;
    /// ◆ The window in the corner keeps this far from the edges of the work area (the mock-up:
    /// `right: 12px`, `bottom: 60px` over a taskbar of 48).
    pub const POPUP_MARGIN: Pt = Pt(12.0);
    /// ◆ The curve of the movement.
    pub const EASE: Bezier = Bezier {
        x1: 0.2,
        y1: 0.9,
        x2: 0.3,
        y2: 1.0,
    };

    /// A cubic Bézier curve from (0, 0) to (1, 1), as CSS writes it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Bezier {
        pub x1: f32,
        pub y1: f32,
        pub x2: f32,
        pub y2: f32,
    }

    impl Bezier {
        /// The progress of the curve at the share of time `time` (0…1).
        pub fn at(self, time: f32) -> f32 {
            let time = time.clamp(0.0, 1.0);
            let point = |t: f32, a: f32, b: f32| {
                let u = 1.0 - t;
                3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
            };
            // The parameter whose x is `time`, by halving: the curve's x only grows.
            let (mut low, mut high) = (0.0_f32, 1.0_f32);
            for _ in 0..24 {
                let middle = (low + high) / 2.0;
                if point(middle, self.x1, self.x2) < time {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            point((low + high) / 2.0, self.y1, self.y2)
        }
    }

    /// The share of [`APPEAR_SLIDE`] still to go `elapsed_ms` after the window appeared: 1 at the
    /// start, 0 from [`APPEAR_MS`]; 0 at once when `moves` is false (Windows' animation off).
    pub fn remaining(elapsed_ms: u32, moves: bool) -> f32 {
        if !moves || elapsed_ms >= APPEAR_MS {
            return 0.0;
        }
        1.0 - EASE.at(elapsed_ms as f32 / APPEAR_MS as f32)
    }
}

/// ◆ The list of a choice field: the most rows it shows at once (66 languages scroll).
pub const DROPDOWN_ROWS: Share = Share(10.0);
/// ◆ The list of a choice field: the gap between the field and its list.
pub const DROPDOWN_GAP: Pt = Pt(4.0);
/// The first letters typed into an open list are one word while no more than this passes between
/// two of them, ms (as the lists of Windows).
pub const TYPE_AHEAD_MS: u64 = 1000;
