#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 2: Add support for navigating using arrow keys Implement hot keys https://github.com/c-git/egui/blob/34db001db14940c948eb03d3fe87f2af2c45daba/crates/egui_demo_lib/src/demo/demo_app_windows.rs#L323
// TODO 2: Add filter by and let user pick like ID or date or something like that
// TODO 2: Add button to set to current value if a field is selected
// TODO 3: Support auto reload (look into watching for changes) https://watchexec.github.io/
