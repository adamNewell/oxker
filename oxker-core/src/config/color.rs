/// Color abstraction that doesn't depend on any UI framework
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Reset to the default color.
    Reset,
    /// Black color.
    Black,
    /// Red color.
    Red,
    /// Green color.
    Green,
    /// Yellow color.
    Yellow,
    /// Blue color.
    Blue,
    /// Magenta color.
    Magenta,
    /// Cyan color.
    Cyan,
    /// Gray color.
    Gray,
    /// Dark gray color.
    DarkGray,
    /// Light red color.
    LightRed,
    /// Light green color.
    LightGreen,
    /// Light yellow color.
    LightYellow,
    /// Light blue color.
    LightBlue,
    /// Light magenta color.
    LightMagenta,
    /// Light cyan color.
    LightCyan,
    /// White color.
    White,
    /// RGB color with specific red, green, and blue values.
    Rgb(u8, u8, u8),
    /// Indexed color from the 256-color palette.
    Indexed(u8),
}

impl Default for Color {
    fn default() -> Self {
        Self::Reset
    }
}
