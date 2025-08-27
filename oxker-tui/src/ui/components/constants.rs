//! Shared constants for UI components

// Application metadata
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const REPO: &str = env!("CARGO_PKG_REPOSITORY");

// ASCII art for the application name
pub const NAME_TEXT: &str = r#"
                          88                               
                          88                               
                          88                               
 ,adPPYba,   8b,     ,d8  88   ,d8    ,adPPYba,  8b,dPPYba,
a8"     "8a   `Y8, ,8P'   88 ,a8"    a8P_____88  88P'   "Y8
8b       d8     )888(     8888[      8PP"""""""  88        
"8a,   ,a8"   ,d8" "8b,   88`"Yba,   "8b,   ,aa  88        
 `"YbbdP"'   8P'     `Y8  88   `Y8a   `"Ybbd8"'  88        "#;

// Utility functions
/// Calculate maximum line width for text
#[must_use]
pub fn max_line_width(input: &str) -> usize {
    input.lines().map(|x| x.chars().count()).max().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_line_width() {
        assert_eq!(max_line_width("hello"), 5);
        assert_eq!(max_line_width("hello\nworld!"), 6);
        assert_eq!(max_line_width(""), 1);
        assert_eq!(max_line_width("\n\n"), 0);
    }
}
