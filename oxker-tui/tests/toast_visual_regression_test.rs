#![allow(clippy::unwrap_used)]

use insta::assert_snapshot;
use oxker_core::AppColors;
use oxker_tui::ui::components::{
    Component,
    widgets::info_box::{InfoBox, InfoBoxProps},
};
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;

#[test]
fn test_toast_visual_snapshot_unicode() {
    // Test Unicode symbol rendering
    // Note: Can't set env vars due to unsafe restrictions, test with Unicode directly
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let info_box = InfoBox::new();
            let theme = AppColors::new();
            let props = InfoBoxProps {
                message: "✓ mouse capture enabled",
                start_time: Instant::now(),
                theme: &theme,
            };

            info_box.render(&props, frame.area(), frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // Create a string representation of the toast area (bottom-right)
    let mut output = String::new();
    // Toast is at x=48, y=20, width=30, height=3
    for y in 20..23 {
        for x in 48..78 {
            let cell = buffer.cell((x, y)).unwrap();
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }

    assert_snapshot!("toast_unicode_enabled", output);
}

#[test]
fn test_toast_visual_snapshot_ascii() {
    // Test ASCII fallback rendering
    // Note: Can't set env vars due to unsafe restrictions, test with ASCII directly
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let info_box = InfoBox::new();
            let theme = AppColors::new();
            let props = InfoBoxProps {
                message: "[+] mouse capture enabled",
                start_time: Instant::now(),
                theme: &theme,
            };

            info_box.render(&props, frame.area(), frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // Create a string representation of the toast area
    let mut output = String::new();
    for y in 20..23 {
        for x in 48..78 {
            let cell = buffer.cell((x, y)).unwrap();
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }

    assert_snapshot!("toast_ascii_enabled", output);
}

#[test]
fn test_toast_visual_position() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let info_box = InfoBox::new();
            let theme = AppColors::new();
            let props = InfoBoxProps {
                message: "Test Message",
                start_time: Instant::now(),
                theme: &theme,
            };

            info_box.render(&props, frame.area(), frame);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // The toast renders with padding, so the text is vertically centered
    // Toast is at x=48, y=20, width=30, height=3
    // With vertical centering, text should be on line y=21 (middle line)
    let mut found_text = false;
    let mut output = String::new();

    // Extract the toast area to see what's there
    for y in 20..23 {
        for x in 48..78 {
            if let Some(cell) = buffer.cell((x, y)) {
                let symbol = cell.symbol();
                output.push_str(symbol);
                if symbol.contains("Test") || symbol.contains("Message") {
                    found_text = true;
                }
            }
        }
        output.push('\n');
    }

    // The text is padded with newlines, so check if we have the message anywhere
    if output.contains("Test Message") {
        found_text = true;
    }

    assert!(
        found_text,
        "Toast text 'Test Message' should be visible in toast area. Toast content:\n{output}"
    );
}
