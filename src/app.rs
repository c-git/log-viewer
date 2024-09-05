use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{bail, Context};
use egui::{Align, KeyboardShortcut, Modifiers};
use egui_extras::{Column, TableBuilder};
use log::info;

use self::{data::Data, data_display_options::DataDisplayOptions};

mod data;
mod data_display_options;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    data: Option<Data>,
    data_display_options: DataDisplayOptions,
    start_open_path: Arc<Mutex<Option<PathBuf>>>,
    last_filename: Arc<Mutex<Option<PathBuf>>>,
    show_last_filename: bool,
    track_item_align: Option<Align>,
    shortcut_prev: KeyboardShortcut,
    shortcut_next: KeyboardShortcut,
    shortcut_first: KeyboardShortcut,
    shortcut_last: KeyboardShortcut,

    #[serde(skip)]
    should_scroll: bool,
    #[serde(skip)]
    loading_status: LoadingStatus,
}

impl Default for LogViewerApp {
    fn default() -> Self {
        Self {
            data: Default::default(),
            data_display_options: Default::default(),
            start_open_path: Default::default(),
            loading_status: Default::default(),
            last_filename: Default::default(),
            track_item_align: Default::default(),
            shortcut_prev: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowUp),
            shortcut_next: KeyboardShortcut::new(Modifiers::NONE, egui::Key::ArrowDown),
            shortcut_first: KeyboardShortcut::new(Modifiers::NONE, egui::Key::Home),
            shortcut_last: KeyboardShortcut::new(Modifiers::NONE, egui::Key::End),
            should_scroll: Default::default(),
            show_last_filename: true,
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
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let mut table_builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT));

        let n = self.data_display_options.main_list_fields().len();
        for _ in self
            .data_display_options
            .main_list_fields()
            .iter()
            .take(n - 1)
        {
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
                // TODO 3: Figure out if calculating these values only once is worth it.
                // TODO 4: Remove hard coded "msg"
                let heights: Vec<f32> = data
                    .rows()
                    .iter()
                    .map(|x| {
                        (1f32).max(x.field_value("msg").display().lines().count() as f32)
                            * text_height
                    })
                    .collect();
                body.heterogeneous_rows(heights.into_iter(), |mut row| {
                    let row_index = row.index();
                    let log_row = &data.rows()[row_index];

                    let emphasis_info = if let Some(selected_row) = data.selected_row {
                        row.set_selected(selected_row == row_index);
                        if let Some(emphasis_field_idx) =
                            *self.data_display_options.emphasize_if_matching_field_idx()
                        {
                            let field_name =
                                &self.data_display_options.main_list_fields()[emphasis_field_idx];
                            Some((
                                emphasis_field_idx,
                                data.rows()[selected_row].field_value(field_name),
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
                                ui.label(field_value.display());
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
        let Some(data) = self.data.as_mut() else {
            ui.label("No data");
            return;
        };

        let Some(selected_values) =
            data.selected_row_data_as_slice(self.data_display_options.common_fields())
        else {
            ui.label("No row Selected");
            return;
        };

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
            // TODO 3: Figure out if calculating these values only once is worth it.
            let heights: Vec<f32> = selected_values
                .iter()
                .map(|x| (1f32).max(x.1.lines().count() as f32) * text_height)
                .collect();
            body.heterogeneous_rows(heights.iter().cloned(), |mut row| {
                let row_index = row.index();
                let (title, value) = &selected_values[row_index];
                row.col(|ui| {
                    ui.label(title);
                });
                row.col(|ui| {
                    ui.label(value.to_string());
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
                let msg = msg.replace(r"\n", "\n");
                let msg = msg.replace(r#"\""#, "\"");
                if ui.button("Clear Error Status").clicked() {
                    self.loading_status = LoadingStatus::NotInProgress;
                }
                ui.label(msg);
            }
            LoadingStatus::Success(data) => {
                self.loading_status = match Data::try_from(&data[..]) {
                    Ok(data) => {
                        self.data = Some(data);
                        LoadingStatus::NotInProgress
                    }
                    Err(e) => LoadingStatus::Failed(format!("{e:?}")),
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
            ui.horizontal(|ui| {
                ui.label(
                    "Text is selectable just hover over it for a short time if you want to copy",
                );
            });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Attempts to read the contents of the last loaded file and return it in a loading status otherwise returns an error loading status
    fn reload_file(&self) -> LoadingStatus {
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

    fn check_shortcuts(&mut self, ui: &mut egui::Ui) {
        if ui.input_mut(|i| i.consume_shortcut(&self.shortcut_prev)) {
            self.move_selected_prev();
        }

        if ui.input_mut(|i| i.consume_shortcut(&self.shortcut_next)) {
            self.move_selected_next();
        }

        if ui.input_mut(|i| i.consume_shortcut(&self.shortcut_first)) {
            self.move_selected_first();
        }

        if ui.input_mut(|i| i.consume_shortcut(&self.shortcut_last)) {
            self.move_selected_last();
        }
    }

    fn navigation_and_filtering_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Nav:");
            if ui
                .button("⏪")
                .on_hover_text(format!(
                    "First ({})",
                    ui.ctx().format_shortcut(&self.shortcut_first)
                ))
                .clicked()
            {
                self.move_selected_first();
            }
            if ui
                .button("⬆")
                .on_hover_text(format!(
                    "Previous ({})",
                    ui.ctx().format_shortcut(&self.shortcut_prev)
                ))
                .clicked()
            {
                self.move_selected_prev();
            }
            if ui
                .button("⬇")
                .on_hover_text(format!(
                    "Next ({})",
                    ui.ctx().format_shortcut(&self.shortcut_next)
                ))
                .clicked()
            {
                self.move_selected_next();
            }
            if ui
                .button("⏩")
                .on_hover_text(format!(
                    "Last ({})",
                    ui.ctx().format_shortcut(&self.shortcut_last)
                ))
                .clicked()
            {
                self.move_selected_last();
            }
        });
    }
    fn data_load_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("📂 Open log file...").clicked() {
                let ctx = ui.ctx().clone();
                self.loading_status = self.initiate_loading(ctx);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if ui.button("Reload").clicked() {
                    self.loading_status = self.reload_file();
                }
                if ui.button("Load Most Recent File").clicked() {
                    self.loading_status = self.load_most_recent_file();
                }
            }
            if ui.button("Clear Data").clicked() {
                self.data = None;
            }

            if self.show_last_filename {
                if let Some(filename) = self.last_filename.lock().unwrap().as_ref() {
                    ui.label(format!("Filename: {}", filename.display()));
                }
            }
        });
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
        info!("Saving data");
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            self.check_shortcuts(ui);

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

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Log Viewer");
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
                    ui.vertical_centered(|ui| {
                        ui.heading("Details");
                    });
                    egui::ScrollArea::horizontal()
                        .id_source("details area")
                        .show(ui, |ui| {
                            ui.push_id("table details", |ui| self.show_log_details(ui));
                        });
                    if ui.available_height() > 0.0 {
                        ui.allocate_space(ui.available_size());
                    }
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_source("log lines")
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
