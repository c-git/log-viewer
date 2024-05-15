#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 3: Fix bug where multiline messages bleed into the lines below them
// TODO 4: Add a open most recent log button
// TODO 4: Add a reload log button
