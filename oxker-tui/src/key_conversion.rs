//! Conversion utilities between crossterm key types and oxker-core key types

use crossterm::event::{KeyCode as CrosstermKeyCode, KeyModifiers as CrosstermKeyModifiers};
use oxker_core::{KeyCode, KeyModifiers};

/// Convert a crossterm KeyCode to oxker-core KeyCode
#[must_use]
pub const fn from_crossterm_keycode(key: CrosstermKeyCode) -> Option<KeyCode> {
    match key {
        CrosstermKeyCode::Backspace => Some(KeyCode::Backspace),
        CrosstermKeyCode::Enter => Some(KeyCode::Enter),
        CrosstermKeyCode::Left => Some(KeyCode::Left),
        CrosstermKeyCode::Right => Some(KeyCode::Right),
        CrosstermKeyCode::Up => Some(KeyCode::Up),
        CrosstermKeyCode::Down => Some(KeyCode::Down),
        CrosstermKeyCode::Home => Some(KeyCode::Home),
        CrosstermKeyCode::End => Some(KeyCode::End),
        CrosstermKeyCode::PageUp => Some(KeyCode::PageUp),
        CrosstermKeyCode::PageDown => Some(KeyCode::PageDown),
        CrosstermKeyCode::Tab => Some(KeyCode::Tab),
        CrosstermKeyCode::BackTab => Some(KeyCode::BackTab),
        CrosstermKeyCode::Delete => Some(KeyCode::Delete),
        CrosstermKeyCode::Insert => Some(KeyCode::Insert),
        CrosstermKeyCode::F(n) => Some(KeyCode::F(n)),
        CrosstermKeyCode::Char(c) => Some(KeyCode::Char(c)),
        CrosstermKeyCode::Esc => Some(KeyCode::Esc),
        CrosstermKeyCode::Null => Some(KeyCode::Null),
        CrosstermKeyCode::CapsLock => Some(KeyCode::CapsLock),
        // Crossterm-specific keys that don't map to our abstraction
        CrosstermKeyCode::ScrollLock
        | CrosstermKeyCode::NumLock
        | CrosstermKeyCode::PrintScreen
        | CrosstermKeyCode::Pause
        | CrosstermKeyCode::Menu
        | CrosstermKeyCode::KeypadBegin
        | CrosstermKeyCode::Media(_)
        | CrosstermKeyCode::Modifier(_) => None,
    }
}

/// Convert crossterm KeyModifiers to oxker-core KeyModifiers
#[must_use]
pub const fn from_crossterm_modifiers(modifiers: CrosstermKeyModifiers) -> KeyModifiers {
    let mut result = KeyModifiers::NONE;

    if modifiers.contains(CrosstermKeyModifiers::SHIFT) {
        result = KeyModifiers(result.0 | KeyModifiers::SHIFT.0);
    }
    if modifiers.contains(CrosstermKeyModifiers::CONTROL) {
        result = KeyModifiers(result.0 | KeyModifiers::CONTROL.0);
    }
    if modifiers.contains(CrosstermKeyModifiers::ALT) {
        result = KeyModifiers(result.0 | KeyModifiers::ALT.0);
    }
    if modifiers.contains(CrosstermKeyModifiers::SUPER) {
        result = KeyModifiers(result.0 | KeyModifiers::SUPER.0);
    }
    if modifiers.contains(CrosstermKeyModifiers::HYPER) {
        result = KeyModifiers(result.0 | KeyModifiers::HYPER.0);
    }
    if modifiers.contains(CrosstermKeyModifiers::META) {
        result = KeyModifiers(result.0 | KeyModifiers::META.0);
    }

    result
}

/// Convert oxker-core KeyCode to crossterm KeyCode (for display purposes)
#[must_use]
pub const fn to_crossterm_keycode(key: KeyCode) -> CrosstermKeyCode {
    match key {
        KeyCode::Backspace => CrosstermKeyCode::Backspace,
        KeyCode::Enter => CrosstermKeyCode::Enter,
        KeyCode::Left => CrosstermKeyCode::Left,
        KeyCode::Right => CrosstermKeyCode::Right,
        KeyCode::Up => CrosstermKeyCode::Up,
        KeyCode::Down => CrosstermKeyCode::Down,
        KeyCode::Home => CrosstermKeyCode::Home,
        KeyCode::End => CrosstermKeyCode::End,
        KeyCode::PageUp => CrosstermKeyCode::PageUp,
        KeyCode::PageDown => CrosstermKeyCode::PageDown,
        KeyCode::Tab => CrosstermKeyCode::Tab,
        KeyCode::BackTab => CrosstermKeyCode::BackTab,
        KeyCode::Delete => CrosstermKeyCode::Delete,
        KeyCode::Insert => CrosstermKeyCode::Insert,
        KeyCode::F(n) => CrosstermKeyCode::F(n),
        KeyCode::Char(c) => CrosstermKeyCode::Char(c),
        KeyCode::Esc => CrosstermKeyCode::Esc,
        KeyCode::Null => CrosstermKeyCode::Null,
        KeyCode::CapsLock => CrosstermKeyCode::CapsLock,
    }
}

/// Convert oxker-core KeyModifiers to crossterm KeyModifiers (for display purposes)
#[must_use]
pub fn to_crossterm_modifiers(modifiers: KeyModifiers) -> CrosstermKeyModifiers {
    let mut result = CrosstermKeyModifiers::empty();

    if modifiers.contains_shift() {
        result.insert(CrosstermKeyModifiers::SHIFT);
    }
    if modifiers.contains_control() {
        result.insert(CrosstermKeyModifiers::CONTROL);
    }
    if modifiers.contains_alt() {
        result.insert(CrosstermKeyModifiers::ALT);
    }
    if modifiers.contains_super() {
        result.insert(CrosstermKeyModifiers::SUPER);
    }
    if modifiers.contains_hyper() {
        result.insert(CrosstermKeyModifiers::HYPER);
    }
    if modifiers.contains_meta() {
        result.insert(CrosstermKeyModifiers::META);
    }

    result
}
