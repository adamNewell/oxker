//! UI-independent key types for oxker-core
//!
//! This module provides key types that don't depend on any specific UI framework,
//! allowing oxker-core to remain UI-agnostic.

use std::fmt;

/// Represents a keyboard key code without UI framework dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Backspace key
    Backspace,
    /// Enter key (also known as Return on some keyboards)
    Enter,
    /// Left arrow key  
    Left,
    /// Right arrow key
    Right,
    /// Up arrow key
    Up,
    /// Down arrow key
    Down,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up key
    PageUp,
    /// Page down key
    PageDown,
    /// Tab key
    Tab,
    /// Shift + Tab (Backtab)
    BackTab,
    /// Delete key
    Delete,
    /// Insert key
    Insert,
    /// F key (F1-F12)
    F(u8),
    /// A character key
    Char(char),
    /// Escape key
    Esc,
    /// Null byte
    Null,
    /// CapsLock key (not supported by all terminals)
    CapsLock,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyModifiers(pub u8);

impl KeyModifiers {
    /// No modifiers
    pub const NONE: Self = Self(0);
    /// Shift modifier
    pub const SHIFT: Self = Self(1);
    /// Control modifier
    pub const CONTROL: Self = Self(2);
    /// Alt modifier (also known as Option on Mac)
    pub const ALT: Self = Self(4);
    /// Super/Windows/Command key modifier (not supported by all terminals)
    pub const SUPER: Self = Self(8);
    /// Hyper key modifier (not supported by all terminals)
    pub const HYPER: Self = Self(16);
    /// Meta key modifier (not supported by all terminals)
    pub const META: Self = Self(32);

    /// Check if shift is pressed
    #[must_use]
    pub const fn contains_shift(&self) -> bool {
        self.0 & Self::SHIFT.0 != 0
    }

    /// Check if control is pressed
    #[must_use]
    pub const fn contains_control(&self) -> bool {
        self.0 & Self::CONTROL.0 != 0
    }

    /// Check if alt is pressed
    #[must_use]
    pub const fn contains_alt(&self) -> bool {
        self.0 & Self::ALT.0 != 0
    }

    /// Check if super is pressed
    #[must_use]
    pub const fn contains_super(&self) -> bool {
        self.0 & Self::SUPER.0 != 0
    }

    /// Check if hyper is pressed
    #[must_use]
    pub const fn contains_hyper(&self) -> bool {
        self.0 & Self::HYPER.0 != 0
    }

    /// Check if meta is pressed
    #[must_use]
    pub const fn contains_meta(&self) -> bool {
        self.0 & Self::META.0 != 0
    }

    /// Check if this set of modifiers contains all of the given modifiers
    #[must_use]
    pub const fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backspace => write!(f, "Backspace"),
            Self::Enter => write!(f, "Enter"),
            Self::Left => write!(f, "Left"),
            Self::Right => write!(f, "Right"),
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Home => write!(f, "Home"),
            Self::End => write!(f, "End"),
            Self::PageUp => write!(f, "PageUp"),
            Self::PageDown => write!(f, "PageDown"),
            Self::Tab => write!(f, "Tab"),
            Self::BackTab => write!(f, "BackTab"),
            Self::Delete => write!(f, "Delete"),
            Self::Insert => write!(f, "Insert"),
            Self::F(n) => write!(f, "F{n}"),
            Self::Char(c) => write!(f, "{c}"),
            Self::Esc => write!(f, "Esc"),
            Self::Null => write!(f, "Null"),
            Self::CapsLock => write!(f, "CapsLock"),
        }
    }
}

impl fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut modifiers = Vec::new();
        if self.contains_control() {
            modifiers.push("Ctrl");
        }
        if self.contains_alt() {
            modifiers.push("Alt");
        }
        if self.contains_shift() {
            modifiers.push("Shift");
        }
        if self.contains_super() {
            modifiers.push("Super");
        }
        if self.contains_hyper() {
            modifiers.push("Hyper");
        }
        if self.contains_meta() {
            modifiers.push("Meta");
        }

        if modifiers.is_empty() {
            write!(f, "None")
        } else {
            write!(f, "{}", modifiers.join("+"))
        }
    }
}
