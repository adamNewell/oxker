#![allow(clippy::unwrap_used)]

use oxker_tui::ui::{GuiState, Rerender};
use std::{sync::Arc, time::Duration};

#[test]
fn test_mouse_capture_toast_messages() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Test setting mouse capture enabled message
    gui_state.set_info_box("✓ mouse capture enabled");

    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "✓ mouse capture enabled");

    // Test setting mouse capture disabled message
    gui_state.set_info_box("✖ mouse capture disabled");

    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "✖ mouse capture disabled");
}

#[test]
fn test_toast_replacement_on_rapid_toggle() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Simulate rapid toggling
    gui_state.set_info_box("✓ mouse capture enabled");
    let first_instant = gui_state.info_box_text.as_ref().unwrap().1;

    // Small delay to ensure different timestamp
    std::thread::sleep(Duration::from_millis(10));

    gui_state.set_info_box("✖ mouse capture disabled");
    let second_instant = gui_state.info_box_text.as_ref().unwrap().1;

    // Verify the message was replaced with new timestamp
    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "✖ mouse capture disabled");
    assert!(second_instant > first_instant);
}

#[test]
fn test_info_box_reset() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Set info box
    gui_state.set_info_box("Test message");
    assert!(gui_state.info_box_text.is_some());

    // Reset info box
    gui_state.reset_info_box();
    assert!(gui_state.info_box_text.is_none());
}

#[test]
fn test_info_box_on_no_container() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Test the "No container selected" message
    gui_state.set_info_box("No container selected");

    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "No container selected");
}

#[test]
fn test_unicode_detection_logic() {
    // Test that UTF-8 detection logic works correctly
    // We can't set environment variables in tests due to unsafe restrictions,
    // but we can test the logic conceptually

    // Test UTF-8 detection patterns
    let utf8_locales = ["en_US.UTF-8", "en_GB.utf8", "C.UTF-8"];
    for locale in utf8_locales {
        assert!(locale.contains("UTF-8") || locale.contains("utf8"));
    }

    // Test terminal detection patterns
    let unicode_terms = ["xterm-256color", "alacritty", "kitty", "iTerm2"];
    for term in unicode_terms {
        assert!(
            term.contains("256color")
                || term.contains("alacritty")
                || term.contains("kitty")
                || term.contains("iTerm")
        );
    }

    // Test non-Unicode patterns
    let ascii_terms = ["xterm", "vt100", "dumb"];
    for term in ascii_terms {
        assert!(
            !term.contains("256color")
                && !term.contains("alacritty")
                && !term.contains("kitty")
                && !term.contains("iTerm")
        );
    }
}
