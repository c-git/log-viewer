#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 3: Fix bug where multiline messages bleed into the lines below them
// TODO 3: Add support for navigating using arrow keys
// TODO 3: Add button to show/hide either the top or bottom
// TODO 4: Add a open most recent log button
// TODO 4: Add a reload log button
// TODO 5: Support auto reload (look into watching for changes)
