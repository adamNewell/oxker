use oxker_core::config::Color as CoreColor;
use ratatui::style::Color as RatatuiColor;

/// Convert from oxker-core Color to ratatui Color
pub trait IntoRatatuiColor {
    fn into_ratatui_color(self) -> RatatuiColor;
}

impl IntoRatatuiColor for CoreColor {
    fn into_ratatui_color(self) -> RatatuiColor {
        match self {
            Self::Reset => RatatuiColor::Reset,
            Self::Black => RatatuiColor::Black,
            Self::Red => RatatuiColor::Red,
            Self::Green => RatatuiColor::Green,
            Self::Yellow => RatatuiColor::Yellow,
            Self::Blue => RatatuiColor::Blue,
            Self::Magenta => RatatuiColor::Magenta,
            Self::Cyan => RatatuiColor::Cyan,
            Self::Gray => RatatuiColor::Gray,
            Self::DarkGray => RatatuiColor::DarkGray,
            Self::LightRed => RatatuiColor::LightRed,
            Self::LightGreen => RatatuiColor::LightGreen,
            Self::LightYellow => RatatuiColor::LightYellow,
            Self::LightBlue => RatatuiColor::LightBlue,
            Self::LightMagenta => RatatuiColor::LightMagenta,
            Self::LightCyan => RatatuiColor::LightCyan,
            Self::White => RatatuiColor::White,
            Self::Rgb(r, g, b) => RatatuiColor::Rgb(r, g, b),
            Self::Indexed(index) => RatatuiColor::Indexed(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_conversion() {
        assert_eq!(CoreColor::Reset.into_ratatui_color(), RatatuiColor::Reset);
        assert_eq!(CoreColor::Black.into_ratatui_color(), RatatuiColor::Black);
        assert_eq!(CoreColor::Red.into_ratatui_color(), RatatuiColor::Red);
        assert_eq!(CoreColor::Green.into_ratatui_color(), RatatuiColor::Green);
        assert_eq!(CoreColor::Yellow.into_ratatui_color(), RatatuiColor::Yellow);
        assert_eq!(CoreColor::Blue.into_ratatui_color(), RatatuiColor::Blue);
        assert_eq!(
            CoreColor::Magenta.into_ratatui_color(),
            RatatuiColor::Magenta
        );
        assert_eq!(CoreColor::Cyan.into_ratatui_color(), RatatuiColor::Cyan);
        assert_eq!(CoreColor::Gray.into_ratatui_color(), RatatuiColor::Gray);
        assert_eq!(
            CoreColor::DarkGray.into_ratatui_color(),
            RatatuiColor::DarkGray
        );
        assert_eq!(
            CoreColor::LightRed.into_ratatui_color(),
            RatatuiColor::LightRed
        );
        assert_eq!(
            CoreColor::LightGreen.into_ratatui_color(),
            RatatuiColor::LightGreen
        );
        assert_eq!(
            CoreColor::LightYellow.into_ratatui_color(),
            RatatuiColor::LightYellow
        );
        assert_eq!(
            CoreColor::LightBlue.into_ratatui_color(),
            RatatuiColor::LightBlue
        );
        assert_eq!(
            CoreColor::LightMagenta.into_ratatui_color(),
            RatatuiColor::LightMagenta
        );
        assert_eq!(
            CoreColor::LightCyan.into_ratatui_color(),
            RatatuiColor::LightCyan
        );
        assert_eq!(CoreColor::White.into_ratatui_color(), RatatuiColor::White);
        assert_eq!(
            CoreColor::Rgb(255, 178, 36).into_ratatui_color(),
            RatatuiColor::Rgb(255, 178, 36)
        );
        assert_eq!(
            CoreColor::Indexed(42).into_ratatui_color(),
            RatatuiColor::Indexed(42)
        );
    }
}
