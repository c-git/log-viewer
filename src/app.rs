use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use log::info;

use self::{data::Data, data_display_options::DataDisplayOptions};

// TODO 3: Add search
// TODO 3: Add filter by and let user pick like ID or date or something like that
// TODO 3: Add checkbox to filter by current request id
// TODO 3: Add support for arrow keys like up and down

mod data;
mod data_display_options;

const SPACE_BETWEEN_TABLES: f32 = 10.;
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    data: Option<Data>,
    main_table_screen_proportion: f32,
    data_display_options: DataDisplayOptions,
    start_open_path: Arc<Mutex<Option<PathBuf>>>,

    #[serde(skip)]
    loading_status: LoadingStatus,
}

impl Default for LogViewerApp {
    fn default() -> Self {
        Self {
            data: Default::default(),
            main_table_screen_proportion: 0.5,
            data_display_options: Default::default(),
            start_open_path: Default::default(),
            loading_status: Default::default(),
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

        let table = table_builder.header(20.0, |mut header| {
            for field_name in self.data_display_options.main_list_fields() {
                header.col(|ui| {
                    ui.strong(field_name);
                });
            }
        });

        if let Some(data) = &mut self.data {
            table.body(|body| {
                body.rows(text_height, data.rows().len(), |mut row| {
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

        let Some(selected_values) = data.selected_row_data_as_slice() else {
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

        let table = table_builder.header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Field Name");
            });
            header.col(|ui| {
                ui.strong("Field Value");
            });
        });

        table.body(|body| {
            body.rows(text_height, selected_values.len(), |mut row| {
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
                ui.horizontal(|ui| {
                    if ui.button("📂 Open log file...").clicked() {
                        let ctx = ui.ctx().clone();
                        self.loading_status = self.initiate_loading(ctx);
                    }
                    if ui.button("Clear Data").clicked() {
                        self.data = None;
                    }
                });
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
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.main_table_screen_proportion)
                        .speed(0.01)
                        .clamp_range(0.2..=0.85)
                        .prefix("Main Area Proportion Percentage "),
                );
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute(
    f: impl std::future::Future<Output = Box<LoadingStatus>> + 'static + Send,
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
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

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

            egui::ScrollArea::vertical()
                .id_source("scroll for overflow")
                .show(ui, |ui| {
                    StripBuilder::new(ui)
                        .size(Size::relative(self.main_table_screen_proportion)) // for the log lines
                        .size(Size::exact(SPACE_BETWEEN_TABLES)) // for the log lines
                        .size(Size::remainder()) // for the details area
                        .vertical(|mut strip| {
                            strip.cell(|ui| {
                                egui::ScrollArea::horizontal().id_source("log lines").show(
                                    ui,
                                    |ui| {
                                        ui.push_id("table log lines", |ui| self.show_log_lines(ui));
                                    },
                                );
                            });
                            strip.cell(|ui| {
                                expanding_content(ui);
                            });
                            strip.cell(|ui| {
                                egui::ScrollArea::horizontal()
                                    .id_source("details area")
                                    .show(ui, |ui| {
                                        ui.push_id("table details", |ui| self.show_log_details(ui));
                                    });
                            });
                        });
                });
        });
    }
}

fn expanding_content(ui: &mut egui::Ui) {
    // Taken from https://github.com/emilk/egui/blob/15370bbea0b468cf719a75cc6d1e39eb00c420d8/crates/egui_demo_lib/src/demo/table_demo.rs#L276
    let width = ui.available_width();
    let height = ui.available_height();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    response.on_hover_text("See options to change size");
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        (2.0, ui.visuals().text_color()),
    );
}
