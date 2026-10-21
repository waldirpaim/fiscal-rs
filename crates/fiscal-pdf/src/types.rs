//! Public value types shared by the engine API.

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Taller than wide.
    #[default]
    Portrait,
    /// Wider than tall.
    Landscape,
    /// Use the orientation the document was created with.
    Default,
}

/// Clockwise page rotation applied by the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    /// No rotation.
    #[default]
    R0,
    /// 90 degrees.
    R90,
    /// 180 degrees.
    R180,
    /// 270 degrees.
    R270,
}

impl Rotation {
    pub(crate) fn degrees(self) -> i32 {
        match self {
            Self::R0 => 0,
            Self::R90 => 90,
            Self::R180 => 180,
            Self::R270 => 270,
        }
    }
}

/// User unit of the document coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    /// Point (1/72 inch).
    Pt,
    /// Millimetre.
    #[default]
    Mm,
    /// Centimetre.
    Cm,
    /// Inch.
    In,
    /// Pixel at 96 dpi.
    Px,
}

impl Unit {
    /// Number of points in one user unit.
    pub(crate) fn scale(self) -> f64 {
        match self {
            Self::Pt => 1.0,
            Self::Mm => 72.0 / 25.4,
            Self::Cm => 72.0 / 2.54,
            Self::In => 72.0,
            Self::Px => 0.75,
        }
    }
}

/// Standard page formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageFormat {
    /// 297 x 420 mm.
    A3,
    /// 210 x 297 mm.
    #[default]
    A4,
    /// 148 x 210 mm.
    A5,
    /// 8.5 x 11 in.
    Letter,
    /// 8.5 x 14 in.
    Legal,
}

impl PageFormat {
    /// Page size in points.
    pub(crate) fn size_pt(self) -> PageSize {
        match self {
            Self::A3 => PageSize {
                w: 841.89,
                h: 1190.55,
            },
            Self::A4 => PageSize {
                w: 595.28,
                h: 841.89,
            },
            Self::A5 => PageSize {
                w: 420.94,
                h: 595.28,
            },
            Self::Letter => PageSize { w: 612.0, h: 792.0 },
            Self::Legal => PageSize {
                w: 612.0,
                h: 1008.0,
            },
        }
    }
}

/// Page width and height (in user units unless stated otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PageSize {
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
}

/// The 17 named colours of the original library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum NamedColor {
    Black,
    Silver,
    Gray,
    White,
    Maroon,
    Red,
    Purple,
    Fuchsia,
    Green,
    Lime,
    Olive,
    Yellow,
    Navy,
    Blue,
    Teal,
    Aqua,
    LightGrey,
}

impl NamedColor {
    /// The RGB triple of the colour.
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Black => (0, 0, 0),
            Self::Silver => (192, 192, 192),
            Self::Gray => (128, 128, 128),
            Self::White => (255, 255, 255),
            Self::Maroon => (128, 0, 0),
            Self::Red => (255, 0, 0),
            Self::Purple => (128, 0, 128),
            Self::Fuchsia => (255, 0, 255),
            Self::Green => (0, 128, 0),
            Self::Lime => (0, 255, 0),
            Self::Olive => (128, 128, 0),
            Self::Yellow => (255, 255, 0),
            Self::Navy => (0, 0, 128),
            Self::Blue => (0, 0, 255),
            Self::Teal => (0, 128, 128),
            Self::Aqua => (0, 255, 255),
            Self::LightGrey => (220, 220, 220),
        }
    }
}

/// A colour for drawing, filling or text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Grey level, `0` (black) to `255` (white).
    Gray(u8),
    /// Red, green and blue components.
    Rgb(u8, u8, u8),
    /// One of the named colours.
    Named(NamedColor),
}

impl Color {
    /// Black.
    pub const BLACK: Self = Self::Gray(0);
    /// White.
    pub const WHITE: Self = Self::Gray(255);

    /// `true` when the colour is written with the single-operand grey operators.
    pub(crate) fn is_gray(self) -> bool {
        match self {
            Self::Gray(_) => true,
            Self::Rgb(r, g, b) => r == 0 && g == 0 && b == 0,
            Self::Named(n) => n.rgb() == (0, 0, 0),
        }
    }

    pub(crate) fn components(self) -> (u8, u8, u8) {
        match self {
            Self::Gray(g) => (g, g, g),
            Self::Rgb(r, g, b) => (r, g, b),
            Self::Named(n) => n.rgb(),
        }
    }
}

impl From<NamedColor> for Color {
    fn from(value: NamedColor) -> Self {
        Self::Named(value)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::Rgb(r, g, b)
    }
}

/// Which sides of a cell get a border.
///
/// Built from the same strings the original API used: `"1"` draws a full frame, otherwise any
/// combination of the letters `L`, `T`, `R` and `B` (`"0"` or `""` draw nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    /// Draw a full frame with a single rectangle (`"1"`).
    pub frame: bool,
    /// Left side.
    pub left: bool,
    /// Top side.
    pub top: bool,
    /// Right side.
    pub right: bool,
    /// Bottom side.
    pub bottom: bool,
}

impl Border {
    /// No border.
    pub const NONE: Self = Self {
        frame: false,
        left: false,
        top: false,
        right: false,
        bottom: false,
    };
    /// Full frame drawn as one rectangle.
    pub const FRAME: Self = Self {
        frame: true,
        left: false,
        top: false,
        right: false,
        bottom: false,
    };

    /// Parses the string form (`"1"`, `"LTRB"`, `"LR"`, ...).
    pub fn parse(spec: &str) -> Self {
        Self {
            frame: spec == "1",
            left: spec.contains('L'),
            top: spec.contains('T'),
            right: spec.contains('R'),
            bottom: spec.contains('B'),
        }
    }

    /// `true` when anything at all is requested.
    pub(crate) fn any(self) -> bool {
        self.frame || self.left || self.top || self.right || self.bottom
    }
}

impl From<&str> for Border {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<bool> for Border {
    fn from(value: bool) -> Self {
        if value { Self::FRAME } else { Self::NONE }
    }
}

/// Horizontal text alignment inside a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Left (`"L"` or empty).
    #[default]
    Left,
    /// Centred (`"C"`).
    Center,
    /// Right (`"R"`).
    Right,
    /// Justified (`"J"`, only meaningful for [`Fpdf::multi_cell`](crate::Fpdf::multi_cell)).
    Justify,
}

impl Align {
    /// Parses the single-letter form.
    pub fn parse(spec: &str) -> Self {
        match spec {
            "R" => Self::Right,
            "C" => Self::Center,
            "J" => Self::Justify,
            _ => Self::Left,
        }
    }
}

impl From<&str> for Align {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

/// Where the cursor goes after a cell is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ln {
    /// To the right of the cell (`0`).
    #[default]
    Right,
    /// To the beginning of the next line (`1`).
    NextLine,
    /// Below the cell, keeping the x position (`2`).
    Below,
}

/// Options of [`Fpdf::cell_with`](crate::Fpdf::cell_with).
#[derive(Debug, Clone, Default)]
pub struct CellOpts {
    /// Border sides.
    pub border: Border,
    /// Cursor movement after the cell.
    pub ln: Ln,
    /// Text alignment.
    pub align: Align,
    /// Fill the background with the current fill colour.
    pub fill: bool,
    /// Optional link attached to the text.
    pub link: Option<Link>,
}

/// Target of a link annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// Internal destination created by [`Fpdf::add_link`](crate::Fpdf::add_link).
    Internal(usize),
    /// External URI.
    Uri(String),
}

/// Initial zoom of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomMode {
    /// Viewer default.
    #[default]
    Default,
    /// Fit the whole page.
    FullPage,
    /// Fit the page width.
    FullWidth,
    /// 100%.
    Real,
    /// Use the explicit zoom factor (percent).
    Custom,
}

/// Page layout of the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    /// Viewer default.
    #[default]
    Default,
    /// One page at a time.
    Single,
    /// Continuous scrolling.
    Continuous,
    /// Two pages side by side.
    Two,
}

/// Civil date and time stamped into the document information dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    /// Year (four digits).
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u32,
    /// Day of month, 1 to 31.
    pub day: u32,
    /// Hour, 0 to 23.
    pub hour: u32,
    /// Minute, 0 to 59.
    pub minute: u32,
    /// Second, 0 to 59.
    pub second: u32,
}
