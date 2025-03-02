use self::{data::Data, data_display_options::DataDisplayOptions};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{bail, Context};
use data::filter::{Comparator, FieldSpecifier, FilterConfig, FilterOn};
use data_display_options::SizeUnits;
use egui::{
    text::{CCursor, CCursorRange},
    Align, KeyboardShortcut, Label,
};
use egui_extras::{Column, TableBuilder};
use shortcut::Shortcuts;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};
use tracing::{debug, error, info};

mod data;
mod data_display_options;
mod shortcut;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    data: Option<Data>,
    data_display_options: DataDisplayOptions,
    start_open_path: Arc<Mutex<Option<PathBuf>>>,
    last_filename: Arc<Mutex<Option<PathBuf>>>,
    show_last_filename: bool,
    track_item_align: Option<Align>,
    shortcuts: Shortcuts,
    should_scroll_to_end_on_load: bool,
    /// Allows the user to dim the warning by clicking on it
    should_highlight_field_warning: bool,
    /// Max size in bytes of data before saving is disabled
    max_data_save_size: Option<usize>,

    #[serde(skip)]
    should_focus_search: bool,
    #[serde(skip)]
    should_scroll: bool,
    #[serde(skip)]
    loading_status: LoadingStatus,
    #[serde(skip)]
    last_save_hash: Option<u64>,
}

impl Default for LogViewerApp {
    fn default() -> Self {
        Self {
            data: Default::default(),
            data_display_options: Default::default(),
            start_open_path: Default::default(),
            loading_status: Default::default(),
            last_filename: Default::default(),
            track_item_align: Some(Align::Center),
            shortcuts: Default::default(),
            should_scroll_to_end_on_load: Default::default(),
            should_highlight_field_warning: true,
            should_focus_search: Default::default(),
            should_scroll: Default::default(),
            show_last_filename: true,
            last_save_hash: Default::default(),
            max_data_save_size: Some(Self::DEFAULT_MAX_DATA_SAVE_SIZE),
        }
    }
}

#[derive(Default)]
pub enum LoadingStatus {
    #[default]
    NotInProgress,
    InProgress(poll_promise::Promise<Box<LoadingStatus>>),
    Failed(String),
    Success(String),
}

impl LogViewerApp {
    const DEFAULT_MAX_DATA_SAVE_SIZE: usize = 2 * 1024 * 1024; // 2MB

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            info!("Storage found");
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_else(|| {
                info!("failed to load data");
                Default::default()
            });
        }

        Default::default()
    }

    fn show_log_lines(&mut self, ui: &mut egui::Ui) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("show_log_lines");
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let default_text_color = ui.visuals().text_color();

        let mut table_builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            // .stick_to_bottom(self.scroll_to_end_on_load) // Removed because it disabled scroll on move if selected
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT));

        // Set all columns but the last to auto, last should be remainder which is set after the loop
        let n = self.data_display_options.main_list_fields().len();
        for _ in 0..n - 1 {
            table_builder = table_builder.column(Column::auto());
        }
        table_builder = table_builder
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        // Make table clickable
        table_builder = table_builder.sense(egui::Sense::click());

        table_builder = match (self.should_scroll, self.data.as_ref()) {
            (true, Some(data)) => {
                self.should_scroll = false;
                if let Some(selected_row) = data.selected_row {
                    table_builder.scroll_to_row(selected_row, self.track_item_align)
                } else {
                    table_builder
                }
            }
            (true, None) | (false, _) => {
                self.should_scroll = false;
                table_builder
            }
        };

        let table = table_builder.header(text_height, |mut header| {
            for field_name in self.data_display_options.main_list_fields() {
                header.col(|ui| {
                    ui.strong(field_name);
                });
            }
        });

        if let Some(data) = &mut self.data {
            table.body(|body| {
                let heights = data.row_heights(text_height);

                body.heterogeneous_rows(heights, |mut row| {
                    let row_index = row.index();
                    let log_row = &data
                        .rows_iter()
                        .nth(row_index)
                        .expect("len was passed above should only be valid indices");

                    let emphasis_info = if let Some(selected_row) = data.selected_row {
                        row.set_selected(selected_row == row_index);
                        if let Some(emphasis_field_idx) =
                            *self.data_display_options.emphasize_if_matching_field_idx()
                        {
                            let field_name =
                                &self.data_display_options.main_list_fields()[emphasis_field_idx];
                            Some((
                                emphasis_field_idx,
                                data.rows_iter()
                                    .nth(selected_row)
                                    .expect("selected row should always be a valid index")
                                    .field_value(field_name),
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    for (field_idx, field_name) in self
                        .data_display_options
                        .main_list_fields()
                        .iter()
                        .enumerate()
                    {
                        let field_value = log_row.field_value(field_name);

                        let should_emphasize_field =
                            Some((field_idx, field_value)) == emphasis_info;

                        row.col(|ui| {
                            if should_emphasize_field {
                                ui.strong(field_value.display());
                            } else {
                                let display_value = field_value.display();
                                if let Some(coloring_rules) =
                                    self.data_display_options.colored_fields.get(field_name)
                                {
                                    let color = coloring_rules
                                        .value_color_map
                                        .get(&display_value)
                                        .unwrap_or(&default_text_color);
                                    ui.colored_label(*color, display_value);
                                } else {
                                    ui.add(Label::new(display_value).truncate());
                                }
                            }
                        });
                    }

                    // Check for click of a row
                    if row.response().clicked() {
                        if Some(row_index) == data.selected_row {
                            data.selected_row = None;
                        } else {
                            data.selected_row = Some(row_index);
                        }
                    }
                });
            });
        } else {
            // No data so empty body
            table.body(|_| {});
        }
    }

    fn show_log_details(&mut self, ui: &mut egui::Ui) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("show_log_details");
        let Some(data) = self.data.as_mut() else {
            ui.label("No data");
            return;
        };

        let Some((selected_values, fields_matching_filter)) = data
            .selected_row_data_as_slice_with_filter_matching_fields(
                self.data_display_options.common_fields(),
            )
        else {
            ui.label("No row Selected");
            return;
        };

        let color_matching_field = ui.visuals().strong_text_color();
        let color_normal_field = ui.visuals().text_color();
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let mut table_builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        // Clicks not needed but adds highlight row
        table_builder = table_builder.sense(egui::Sense::click());

        let table = table_builder.header(text_height, |mut header| {
            header.col(|ui| {
                ui.strong("Field Name");
            });
            header.col(|ui| {
                ui.strong("Field Value");
            });
        });

        table.body(|body| {
            // TODO 4: Figure out if calculating these values only once is worth it.
            let heights: Vec<f32> = selected_values
                .iter()
                .map(|x| (1f32).max(x.1.lines().count() as f32) * text_height)
                .collect();
            body.heterogeneous_rows(heights.iter().cloned(), |mut row| {
                let row_index = row.index();
                let (title, value) = &selected_values[row_index];
                let color = if fields_matching_filter.contains(&row_index) {
                    color_matching_field
                } else {
                    color_normal_field
                };
                row.col(|ui| {
                    ui.colored_label(color, title);
                });
                row.col(|ui| {
                    ui.colored_label(color, value.to_string());
                });
            });
        });
    }

    fn ui_loading(&mut self, ui: &mut egui::Ui) {
        match &self.loading_status {
            LoadingStatus::NotInProgress => {
                self.data_load_ui(ui);
                ui.separator();
                self.navigation_and_filtering_ui(ui);
            }
            LoadingStatus::InProgress(promise) => {
                if promise.ready().is_some() {
                    let mut temp = LoadingStatus::default();
                    std::mem::swap(&mut temp, &mut self.loading_status);
                    let LoadingStatus::InProgress(owned_promise) = temp else {
                        unreachable!("we are sure of this because we just did a match on this")
                    };
                    self.loading_status = *owned_promise.block_and_take(); // We know the promise is ready at this point
                } else {
                    ui.spinner();
                }
            }
            LoadingStatus::Failed(err_msg) => {
                let msg = format!("Loading failed: {err_msg}");
                if ui.button("Clear Error Status").clicked() {
                    self.loading_status = LoadingStatus::NotInProgress;
                }
                ui.colored_label(ui.visuals().error_fg_color, msg);
            }
            LoadingStatus::Success(data) => {
                self.loading_status = match Data::try_from((&self.data_display_options, &data[..]))
                {
                    Ok(mut data) => {
                        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
                        puffin::profile_scope!("swap_data_after_load");
                        if let Some(old_data) = self.data.as_mut() {
                            // Preserve settings across loads of the data
                            data.take_config(old_data, self.data_display_options.common_fields());
                        }
                        self.data = Some(data);
                        if self.should_scroll_to_end_on_load {
                            self.move_selected_last();
                        } else {
                            self.should_scroll = true;
                        }
                        LoadingStatus::NotInProgress
                    }
                    Err(e) => LoadingStatus::Failed(clean_msg(format!("{e:?}"))),
                }
            }
        }
    }

    fn initiate_loading(&self, ctx: egui::Context) -> LoadingStatus {
        let start_open_path = Arc::clone(&self.start_open_path);
        let last_filename = Arc::clone(&self.last_filename);
        LoadingStatus::InProgress(execute(async move {
            let mut dialog = rfd::AsyncFileDialog::new();
            if let Some(path) = start_open_path.lock().unwrap().as_mut() {
                dialog = dialog.set_directory(path);
            }
            let Some(file) = dialog.pick_file().await else {
                // user canceled loading
                return Box::new(LoadingStatus::NotInProgress);
            };
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(parent) = file.path().parent() {
                *start_open_path.lock().unwrap() = Some(PathBuf::from(parent));
            }
            *last_filename.lock().unwrap() = Some(PathBuf::from(file.file_name()));
            let text = file.read().await;

            // Uncomment the following line to simulate taking long to load, only works on native
            // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // If not present screen will not refresh until next paint (comment out to test, works better with the sleep above to demonstrate)
            ctx.request_repaint();

            Box::new(match String::from_utf8(text) {
                Ok(val) => LoadingStatus::Success(val),
                Err(e) => LoadingStatus::Failed(format!("{e}")),
            })
        }))
    }

    fn ui_options(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Options", |ui| {
            ui.checkbox(&mut self.show_last_filename, "Show last filename");
            ui.checkbox(
                &mut self.should_scroll_to_end_on_load,
                "Scroll to end on load",
            )
            .on_hover_text(shortcut_hint_text(
                ui,
                "Only has an effect when a new file is loaded",
                &self.shortcuts.auto_scroll,
            ));
            ui.horizontal(|ui| {
                ui.label("Item align:");
                self.should_scroll |= ui
                    .radio_value(&mut self.track_item_align, Some(Align::Min), "Top")
                    .clicked();
                self.should_scroll |= ui
                    .radio_value(&mut self.track_item_align, Some(Align::Center), "Center")
                    .clicked();
                self.should_scroll |= ui
                    .radio_value(&mut self.track_item_align, Some(Align::Max), "Bottom")
                    .clicked();
                self.should_scroll |= ui
                    .radio_value(&mut self.track_item_align, None, "None (Bring into view)")
                    .clicked();
            });
            ui.horizontal(|ui| {
                let mut show_row_size = self.data_display_options.row_size_config.is_some();
                ui.checkbox(&mut show_row_size, "Show row size");
                match (
                    show_row_size,
                    self.data_display_options.row_size_config.is_some(),
                ) {
                    (true, true) | (false, false) => {}
                    (true, false) => {
                        self.data_display_options.row_size_config = Some(Default::default())
                    }
                    (false, true) => self.data_display_options.row_size_config = None,
                }

                if let Some(row_size) = self.data_display_options.row_size_config.as_mut() {
                    ui.separator();
                    ui.label("Field Name: ");
                    ui.text_edit_singleline(&mut row_size.field_name);
                    ui.separator();
                    egui::ComboBox::from_label("Row Size Unit")
                        .selected_text(row_size.units)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut row_size.units,
                                SizeUnits::Bytes,
                                SizeUnits::Bytes,
                            );
                            ui.selectable_value(&mut row_size.units, SizeUnits::KB, SizeUnits::KB);
                            ui.selectable_value(&mut row_size.units, SizeUnits::MB, SizeUnits::MB);
                            ui.selectable_value(&mut row_size.units, SizeUnits::GB, SizeUnits::GB);
                            ui.selectable_value(&mut row_size.units, SizeUnits::TB, SizeUnits::TB);
                            ui.selectable_value(
                                &mut row_size.units,
                                SizeUnits::Auto,
                                SizeUnits::Auto,
                            );
                        });
                }
            });

            ui.horizontal(|ui| {
                let mut has_max_data_size_for_save = self.max_data_save_size.is_some();
                ui.checkbox(
                    &mut has_max_data_size_for_save,
                    "Enable Max Data Size To Save",
                );
                match (
                    has_max_data_size_for_save,
                    self.max_data_save_size.is_some(),
                ) {
                    (true, true) | (false, false) => {}
                    (true, false) => {
                        self.max_data_save_size = Some(Self::DEFAULT_MAX_DATA_SAVE_SIZE)
                    }
                    (false, true) => self.max_data_save_size = None,
                }

                if let Some(max_data_save_size) = self.max_data_save_size.as_mut() {
                    ui.label(format!(
                        "Allowed Size: {}",
                        SizeUnits::Auto.convert_trimmed(*max_data_save_size)
                    ));
                    ui.add(
                        egui::Slider::new(max_data_save_size, 0..=100 * 1024 * 1024)
                            .step_by(1024.0),
                    );
                }
            });
        });
    }

    fn move_selected_prev(&mut self) {
        if let Some(data) = self.data.as_mut() {
            data.move_selected_to_prev();
            self.should_scroll = true;
        }
    }

    fn move_selected_next(&mut self) {
        if let Some(data) = self.data.as_mut() {
            data.move_selected_to_next();
            self.should_scroll = true;
        }
    }

    fn move_selected_first(&mut self) {
        if let Some(data) = self.data.as_mut() {
            data.move_selected_to_first();
            self.should_scroll = true;
        }
    }

    fn move_selected_last(&mut self) {
        if let Some(data) = self.data.as_mut() {
            data.move_selected_to_last();
            self.should_scroll = true;
        }
    }

    fn ui_help(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Help", |ui| {
                ui.label(
                    "- Text is selectable just hover over it for a short time if you want to copy",
                );
                ui.label("- Most display settings are only applied on load and will require the data to be reloaded")
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Attempts to read the contents of the last loaded file and return it in a loading status otherwise returns an error loading status
    fn reload_file(&self) -> LoadingStatus {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("reload_file");
        // TODO 5: Determine if this should spawn a task to do the load
        let Some(folder) = self.start_open_path.lock().unwrap().clone() else {
            return LoadingStatus::Failed("no staring folder available".into());
        };
        let Some(filename) = self.last_filename.lock().unwrap().clone() else {
            return LoadingStatus::Failed("no last filename available".into());
        };
        let file_path = folder.join(filename);
        match std::fs::read_to_string(file_path) {
            Ok(val) => LoadingStatus::Success(val),
            Err(e) => LoadingStatus::Failed(format!("error loading file: {e:?}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_most_recent_file(&self) -> LoadingStatus {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("load_most_recent_file");
        // TODO 5: Determine if this should spawn a task to do the load (might be able to reuse the normal load)
        let Some(folder) = self.start_open_path.lock().unwrap().clone() else {
            return LoadingStatus::Failed("unable to find starting folder".into());
        };
        match get_most_recent_file(&folder) {
            Ok(path) => match std::fs::read_to_string(&path) {
                Ok(val) => {
                    *self.last_filename.lock().unwrap() =
                        Some(PathBuf::from(path.file_name().unwrap()));
                    LoadingStatus::Success(val)
                }
                Err(e) => LoadingStatus::Failed(format!("error loading file: {e:?}")),
            },
            Err(e) => LoadingStatus::Failed(format!(
                "unable to determine most recent file in starting directory '{}'. Error: {e}",
                folder.display()
            )),
        }
    }

    /// These shortcuts are always enabled
    fn check_global_shortcuts(&mut self, ui: &mut egui::Ui) {
        if ui.input_mut(|i| i.consume_shortcut(&self.shortcuts.search)) {
            self.focus_search_text_edit();
        }
        if ui.input_mut(|i| i.consume_shortcut(&self.shortcuts.auto_scroll)) {
            self.should_scroll_to_end_on_load = !self.should_scroll_to_end_on_load;
        }
    }

    fn navigation_and_filtering_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.navigation_ui(ui);
            ui.separator();
            self.filtering_ui(ui);
        });
        ui.horizontal(|ui| {
            self.unfilter_ui(ui);
        });
    }

    fn filtering_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(data) = self.data.as_mut() {
            ui.label("Filter:");
            let mut is_filter_enabled = data.filter.is_some();
            ui.checkbox(&mut is_filter_enabled, "");
            match (is_filter_enabled, data.filter.is_some()) {
                (false, false) | (true, true) => {} // Already match
                (true, false) => data.filter = Some(Default::default()),
                (false, true) => {
                    data.unfilter();
                    data.filter = None;
                    self.should_scroll = true;
                }
            }
            let mut should_apply_filter = false;
            if is_filter_enabled && shortcut_button(ui, "Apply", "", &self.shortcuts.apply_filter) {
                should_apply_filter = true;
            }

            if let Some(filter) = data.filter.as_mut() {
                let FilterConfig {
                    search_key,
                    filter_on,
                    is_case_sensitive,
                    comparator,
                } = filter;

                ui.label("Search Key: ");
                let mut search_key_text_edit = egui::TextEdit::singleline(search_key).show(ui);
                if self.should_focus_search {
                    self.should_focus_search = false;

                    // Set focus on edit
                    search_key_text_edit.response.request_focus();

                    // Select all text
                    search_key_text_edit
                        .state
                        .cursor
                        .set_char_range(Some(CCursorRange::two(
                            CCursor::new(0),
                            CCursor::new(search_key.len()),
                        )));

                    // Apply selection changes
                    search_key_text_edit
                        .state
                        .store(ui.ctx(), search_key_text_edit.response.id);
                }
                if search_key_text_edit.response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    should_apply_filter = true;
                }

                ui.spacing();
                ui.checkbox(is_case_sensitive, "Case Sensitive");

                ui.spacing();
                egui::ComboBox::from_label("")
                    .selected_text(format!("{}", comparator))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(comparator, Comparator::LessThan, "Less than");
                        ui.selectable_value(
                            comparator,
                            Comparator::LessThanEqual,
                            "Less than equal",
                        );
                        ui.selectable_value(comparator, Comparator::Equal, "Equal");
                        ui.selectable_value(comparator, Comparator::GreaterThan, "Greater than");
                        ui.selectable_value(
                            comparator,
                            Comparator::GreaterThanEqual,
                            "Greater than equal",
                        );
                        ui.selectable_value(comparator, Comparator::NotEqual, "Not equal");
                        ui.selectable_value(comparator, Comparator::Contains, "Contains");
                        ui.selectable_value(comparator, Comparator::NotContains, "Not contains");
                    });

                ui.spacing();
                let mut is_any = filter_on.is_any();
                ui.toggle_value(&mut is_any, "Any");
                if is_any && !filter_on.is_any() {
                    // Toggled on
                    *filter_on = FilterOn::Any;
                }

                let mut is_field = filter_on.is_field();
                ui.toggle_value(&mut is_field, "Field");
                if is_field && !filter_on.is_field() {
                    // Toggled on
                    *filter_on = FilterOn::Field(Default::default());
                }

                if let FilterOn::Field(FieldSpecifier { name }) = filter_on {
                    ui.spacing();
                    if ui
                        .add(egui::TextEdit::singleline(name).hint_text("Name"))
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        should_apply_filter = true;
                    }

                    let color = if self.should_highlight_field_warning {
                        ui.visuals().warn_fg_color
                    } else {
                        ui.visuals().text_color()
                    };
                    let hint_text = if self.should_highlight_field_warning {
                        "Click to DIM warning"
                    } else {
                        "Click to Highlight warning"
                    };
                    // TODO 4: Add an option to select how fields are filter and not only exact match
                    if ui
                        .colored_label(color, "(Field filtering enabled)")
                        .on_hover_text(hint_text)
                        .clicked()
                    {
                        self.should_highlight_field_warning = !self.should_highlight_field_warning;
                    };
                }
            }
            if should_apply_filter {
                data.apply_filter(self.data_display_options.common_fields());
            }
        }
    }

    fn navigation_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Nav:");
        if shortcut_button(ui, "⏪", "First", &self.shortcuts.first) {
            self.move_selected_first();
        }
        if shortcut_button(ui, "⬆", "Previous", &self.shortcuts.prev) {
            self.move_selected_prev();
        }
        if shortcut_button(ui, "⬇", "Next", &self.shortcuts.next) {
            self.move_selected_next();
        }
        if shortcut_button(ui, "⏩", "Last", &self.shortcuts.last) {
            self.move_selected_last();
        }
    }
    fn data_load_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if shortcut_button(ui, "📂 Open log file...", "", &self.shortcuts.open) {
                self.loading_status = self.initiate_loading(ui.ctx().clone());
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if shortcut_button(ui, "Reload", "", &self.shortcuts.reload) {
                    self.loading_status = self.reload_file();
                }
                if shortcut_button(ui, "Load Most Recent File", "", &self.shortcuts.load_latest) {
                    self.loading_status = self.load_most_recent_file();
                }
            }
            if ui.button("Clear Data").clicked() {
                self.data = None;
            }

            if self.show_last_filename {
                if let Some(filename) = self.last_filename.lock().unwrap().as_ref() {
                    let _label = ui.label(format!("Filename: {}", filename.display()));
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(folder) = self.start_open_path.lock().unwrap().as_ref() {
                        _label.on_hover_text(folder.display().to_string());
                    }
                }
            }
            if let Some(data) = self.data.as_ref() {
                let row_count_text =
                    match (data.is_filtered(), data.len(), data.total_len_unfiltered()) {
                        (true, filtered_len, total_len) => format!(
                            "{} of {}",
                            as_string_with_separators(filtered_len),
                            as_string_with_separators(total_len)
                        ),
                        (false, _, total_len) => as_string_with_separators(total_len),
                    };
                ui.label(format!("# Rows: {row_count_text}"));
                let size_text = format!("Size: {}", data.file_size_display());
                if self.does_data_exceeded_max_size() {
                    ui.colored_label(ui.visuals().warn_fg_color, size_text)
                        .on_hover_text("Save disabled because data size is over max set");
                } else {
                    ui.label(size_text);
                }
            }
        });
    }

    fn focus_search_text_edit(&mut self) {
        if let Some(data) = self.data.as_mut() {
            data.filter.get_or_insert(Default::default()); // Create filter if it doesn't exist
            self.should_focus_search = true;
        }
    }

    fn unfilter_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(data) = self.data.as_mut() {
            if data.is_filtered() {
                ui.label(format!("Applied Filter: {}", data.applied_filter_display()));
                ui.separator();
                if shortcut_button(ui, "Unfilter", "Clears Filter", &self.shortcuts.unfilter) {
                    data.unfilter();
                    self.should_scroll = true;
                }
            }
        }
    }

    fn is_changed_since_last_save(&mut self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("is_changed_since_last_save");
        let as_ron = match ron::to_string(&self) {
            Ok(s) => s,
            Err(err_msg) => {
                error!(?err_msg, "failed to serialize app data");
                return true;
            }
        };
        let mut hasher = DefaultHasher::new();
        as_ron.hash(&mut hasher);
        let new_hash = hasher.finish();
        if let Some(&old_hash) = self.last_save_hash.as_ref() {
            if old_hash == new_hash {
                return false;
            }
        }
        self.last_save_hash = Some(new_hash);
        true
    }

    fn should_save(&mut self) -> bool {
        !self.does_data_exceeded_max_size() && self.is_changed_since_last_save()
    }

    fn does_data_exceeded_max_size(&self) -> bool {
        if let (Some(data), Some(max_size)) = (self.data.as_ref(), self.max_data_save_size.as_ref())
        {
            &data.file_size_as_bytes > max_size
        } else {
            false
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_most_recent_file(folder: &PathBuf) -> anyhow::Result<PathBuf> {
    let max = std::fs::read_dir(folder)
        .context("failed to get directory listing")?
        .map(|x| Ok(x.context("failed to open read_dir path")?.path()))
        .filter(|x| x.as_ref().is_ok_and(|x| x.is_file()))
        .map(
            |x: anyhow::Result<PathBuf>| -> anyhow::Result<(std::time::SystemTime, PathBuf)> {
                let path = x?;
                Ok((
                    std::fs::metadata(&path)
                        .with_context(|| format!("failed to read file meta data. Path: {path:?}"))?
                        .modified()
                        .with_context(|| format!("failed to get modified time. Path: {path:?}"))?,
                    path,
                ))
            },
        )
        .collect::<anyhow::Result<Vec<(std::time::SystemTime, PathBuf)>>>()?
        .into_iter()
        .max();
    if let Some((_, path)) = max {
        Ok(path)
    } else {
        bail!("no files found")
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: std::future::Future<Output = Box<LoadingStatus>> + 'static + Send>(
    f: F,
) -> poll_promise::Promise<Box<LoadingStatus>> {
    poll_promise::Promise::spawn_async(f)
}

#[cfg(target_arch = "wasm32")]
fn execute<F: std::future::Future<Output = Box<LoadingStatus>> + 'static>(
    f: F,
) -> poll_promise::Promise<Box<LoadingStatus>> {
    poll_promise::Promise::spawn_local(f)
}

impl eframe::App for LogViewerApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("eframe::App::save");
        if self.should_save() {
            info!("Saving data");
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
            puffin::profile_scope!("Saving App State");
            eframe::set_value(storage, eframe::APP_KEY, self);
        } else {
            debug!("Save skipped");
        }
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
        puffin::profile_scope!("update_loop");
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
            puffin::profile_scope!("top_panel");

            self.check_global_shortcuts(ui);

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
            puffin::profile_scope!("outer_central_panel");
            static HEADING: LazyLock<&'static str> =
                LazyLock::new(|| format!("Log Viewer {}", env!("CARGO_PKG_VERSION")).leak());
            ui.heading(*HEADING);
            ui.separator();
            self.ui_loading(ui);
            ui.separator();
            self.ui_options(ui);
            ui.separator();
            self.ui_help(ui);
            ui.separator();

            const MIN_LOG_LINES_SIZE: f32 = 100.0;
            let max_details_height = ui.available_height() - MIN_LOG_LINES_SIZE;

            egui::TopBottomPanel::bottom("details_panel")
                .resizable(true)
                .default_height(200.)
                .max_height(max_details_height)
                .min_height(60.)
                .show_inside(ui, |ui| {
                    #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
                    puffin::profile_scope!("bottom_panel");
                    ui.vertical_centered(|ui| {
                        ui.heading("Details");
                    });
                    egui::ScrollArea::horizontal()
                        .id_salt("details area")
                        .show(ui, |ui| {
                            ui.push_id("table details", |ui| self.show_log_details(ui));
                        });
                    if ui.available_height() > 0.0 {
                        ui.allocate_space(ui.available_size());
                    }
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                #[cfg(all(not(target_arch = "wasm32"), feature = "profiling"))]
                puffin::profile_scope!("inner_central_panel");
                egui::ScrollArea::horizontal()
                    .id_salt("log lines")
                    .show(ui, |ui| {
                        ui.push_id("table log lines", |ui| self.show_log_lines(ui));
                    });
            });
        });
    }
}

pub fn calculate_hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// Returns true if the button is clicked or the shortcut is pressed
///
/// Note: This makes it the case that the code for both the button and the shortcut press will do the same thing and you cannot use the shortcut to bypass the button not showing
fn shortcut_button(
    ui: &mut egui::Ui,
    caption: impl Into<egui::WidgetText>,
    hint_msg: &str,
    shortcut: &KeyboardShortcut,
) -> bool {
    ui.button(caption)
        .on_hover_text(shortcut_hint_text(ui, hint_msg, shortcut))
        .clicked()
        || ui.input_mut(|i| i.consume_shortcut(shortcut))
}

fn shortcut_hint_text(ui: &mut egui::Ui, hint_msg: &str, shortcut: &KeyboardShortcut) -> String {
    let space = if hint_msg.is_empty() { "" } else { " " };
    format!("{hint_msg}{space}({})", ui.ctx().format_shortcut(shortcut))
}

fn as_string_with_separators(value: usize) -> String {
    value
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}

fn clean_msg<S: AsRef<str>>(msg: S) -> String {
    msg.as_ref().replace(r"\n", "\n").replace(r#"\""#, "\"")
}
