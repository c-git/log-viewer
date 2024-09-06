use egui::{KeyboardShortcut, Modifiers};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Shortcuts {
    pub prev: KeyboardShortcut,
    pub next: KeyboardShortcut,
    pub first: KeyboardShortcut,
    pub last: KeyboardShortcut,
    pub unfilter: KeyboardShortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            prev: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowUp),
            next: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowDown),
            first: KeyboardShortcut::new(Modifiers::NONE, egui::Key::Home),
            last: KeyboardShortcut::new(Modifiers::NONE, egui::Key::End),
            unfilter: KeyboardShortcut::new(Modifiers::NONE, egui::Key::Escape),
        }
    }
}
