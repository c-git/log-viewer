#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 2: Add search
// TODO 2: Add filter by and let user pick like ID or date or something like that
// TODO 2: Add checkbox to filter by current request id
// TODO 2: Add search
// TODO 2: Add support for navigating using arrow keys Implement hot keys https://github.com/c-git/egui/blob/34db001db14940c948eb03d3fe87f2af2c45daba/crates/egui_demo_lib/src/demo/demo_app_windows.rs#L323
// TODO 3: Add button to show/hide either the top or bottom
// TODO 5: Support auto reload (look into watching for changes)
