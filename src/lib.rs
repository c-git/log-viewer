#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 2: Figure out how to let the user know that the text is selectable. It's not obvious at first glance (requires you to wait with the mouse in one place then you can select)
// TODO 2: Add search
// TODO 3: Add support for navigating using arrow keys
// TODO 3: Add button to show/hide either the top or bottom
// TODO 4: Add a open most recent log button
// TODO 4: Add a reload log button
// TODO 5: Support auto reload (look into watching for changes)
