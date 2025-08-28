use crossterm::event::MouseEvent;
use oxker_core::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, Copy)]
pub enum InputMessages {
    ButtonPress((KeyCode, KeyModifiers)),
    MouseEvent((MouseEvent, KeyModifiers)),
}
