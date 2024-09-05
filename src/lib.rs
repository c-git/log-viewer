#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 2: Add filter by and let user pick like ID or date or something like that
// TODO 2: Add button to set to current value if a field is selected
// TODO 3: Support auto reload (look into watching for changes) https://watchexec.github.io/
