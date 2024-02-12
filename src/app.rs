use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use log::info;

use self::{
    data::{Data, LogRow},
    data_display_options::DataDisplayOptions,
};

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
    details_size: f32,
    data_display_options: DataDisplayOptions,

    #[serde(skip)]
    loading_status: LoadingStatus,
}

impl Default for LogViewerApp {
    fn default() -> Self {
        Self {
            data: Default::default(),
            details_size: 100.,
            data_display_options: Default::default(),
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
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
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

    fn show_log_details(&self, ui: &mut egui::Ui) {
        let Some(selected_log_row) = self.selected_row_data() else {
            ui.label("No row Selected");
            return;
        };
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let table_builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        let table = table_builder.header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Field Name");
            });
            header.col(|ui| {
                ui.strong("Field Value");
            });
        });

        let mut iter = selected_log_row.iter();
        let total_rows = selected_log_row.len();

        table.body(|body| {
            body.rows(text_height, total_rows, |mut row| {
                let (title, value) = iter.next().expect("should always match total_rows");
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
        execute(async move {
            let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                // user canceled loading
                return Box::new(LoadingStatus::NotInProgress);
            };
            let text = file.read().await;

            // Uncomment the following line to simulate taking long to load, only works on native
            // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // If not present screen will not refresh until next paint (comment out to test, works better with the sleep above to demonstrate)
            ctx.request_repaint();

            Box::new(match String::from_utf8(text) {
                Ok(val) => LoadingStatus::Success(val),
                Err(e) => LoadingStatus::Failed(format!("{e}")),
            })
        })
    }

    fn selected_row_data(&self) -> Option<&LogRow> {
        let data = self.data.as_ref()?;
        let selected_row_index = data.selected_row?;
        Some(&data.rows()[selected_row_index])
    }

    fn ui_options(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Options", |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.details_size)
                        .speed(1.)
                        .clamp_range(2. * SPACE_BETWEEN_TABLES..=f32::INFINITY)
                        .prefix("Detail Area Size "),
                );
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute(
    f: impl std::future::Future<Output = Box<LoadingStatus>> + 'static + Send,
) -> LoadingStatus {
    LoadingStatus::InProgress(poll_promise::Promise::spawn_async(f))
}

#[cfg(target_arch = "wasm32")]
fn execute<F: std::future::Future<Output = Box<LoadingStatus>> + 'static>(f: F) -> LoadingStatus {
    LoadingStatus::InProgress(poll_promise::Promise::spawn_local(f))
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
                        .size(Size::remainder().at_least(self.details_size + SPACE_BETWEEN_TABLES)) // for the log lines
                        .size(Size::exact(SPACE_BETWEEN_TABLES)) // for the log lines
                        .size(Size::exact(self.details_size)) // for the details area
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
