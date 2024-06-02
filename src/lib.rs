#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::LogViewerApp;

// TODO 1: Add a reload log button
// TODO 1: Add a open most recent log button
// TODO 2: Add search
// TODO 2: Figure out how to let the user know that the text is selectable. It's not obvious at first glance (requires you to wait with the mouse in one place then you can select)
// TODO 3: Add button to show/hide either the top or bottom
// TODO 3: Add checkbox to filter by current request id
// TODO 3: Add filter by and let user pick like ID or date or something like that
// TODO 3: Add search
// TODO 3: Add support for navigating using arrow keys
// TODO 5: Support auto reload (look into watching for changes)
