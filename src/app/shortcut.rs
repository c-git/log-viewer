use egui::{KeyboardShortcut, Modifiers};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Shortcuts {
    pub prev: KeyboardShortcut,
    pub next: KeyboardShortcut,
    pub first: KeyboardShortcut,
    pub last: KeyboardShortcut,
    pub unfilter: KeyboardShortcut,
    pub open: KeyboardShortcut,
    pub reload: KeyboardShortcut,
    pub load_latest: KeyboardShortcut,
    pub apply_filter: KeyboardShortcut,
    pub search: KeyboardShortcut,
    pub auto_scroll: KeyboardShortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            prev: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowUp),
            next: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowDown),
            first: KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Home),
            last: KeyboardShortcut::new(Modifiers::CTRL, egui::Key::End),
            unfilter: KeyboardShortcut::new(Modifiers::NONE, egui::Key::Escape),
            open: KeyboardShortcut::new(Modifiers::CTRL, egui::Key::O),
            reload: KeyboardShortcut::new(Modifiers::NONE, egui::Key::F5),
            load_latest: KeyboardShortcut::new(Modifiers::NONE, egui::Key::F6),
            apply_filter: KeyboardShortcut::new(Modifiers::NONE, egui::Key::F7),
            search: KeyboardShortcut::new(Modifiers::CTRL, egui::Key::F),
            auto_scroll: KeyboardShortcut::new(Modifiers::NONE, egui::Key::F8),
        }
    }
}
