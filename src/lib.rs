#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 4: Add button to set to current value if a field is selected
// TODO 4: Support auto reload (look into watching for changes) https://watchexec.github.io/
