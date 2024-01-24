use egui_extras::{Size, StripBuilder};
use std::fmt::Debug;

use self::data::Data;

// TODO 3: Add search
// TODO 3: Add filter by and let user pick like ID or date or something like that

mod data;
mod loading;
type LoadingType = Option<anyhow::Result<String>>;
type LoadingPromise = poll_promise::Promise<LoadingType>;
type LoadingPromiseOpt = Option<LoadingPromise>;
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    selected_row: Option<usize>,
    data: Option<Data>,

    #[serde(skip)]
    loading_status: LoadingPromiseOpt,
}

impl Debug for LogViewerApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogViewerApp")
            .field("selected_row", &self.selected_row)
            .field("data", &self.data)
            .field("loading_status.is_some", &self.loading_status.is_some())
            .finish()
    }
}

impl LogViewerApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn show_log_lines(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};

        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        let num_rows = match &self.data {
            Some(data) => data.rows().len(),
            None => 0,
        };

        // Make table clickable
        table = table.sense(egui::Sense::click());

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Time");
                });
                header.col(|ui| {
                    ui.strong("request_id");
                });
                header.col(|ui| {
                    ui.strong("otel.name");
                });
                header.col(|ui| {
                    ui.strong("msg");
                });
            })
            .body(|body| {
                body.rows(text_height, num_rows, |mut row| {
                    let row_index = row.index();
                    if let Some(selected_row) = self.selected_row {
                        row.set_selected(selected_row == row_index);
                    }
                    let log_row = &self
                        .data
                        .as_ref()
                        .expect("Should only run if there are rows")
                        .rows()[row_index];
                    row.col(|ui| {
                        ui.label(log_row.time());
                    });
                    row.col(|ui| {
                        ui.label(log_row.request_id());
                    });
                    row.col(|ui| {
                        ui.label(log_row.otel_name());
                    });
                    row.col(|ui| {
                        ui.label(log_row.msg());
                    });

                    self.toggle_row_selection(row_index, &row.response());
                });
            });
    }

    fn show_log_details(&self, ui: &mut egui::Ui) {
        // TODO 1: Log Details
        ui.label("Hi");
    }

    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if Some(row_index) == self.selected_row {
                self.selected_row = None;
            } else {
                self.selected_row = Some(row_index);
            }
        }
    }

    fn ui_loading(&mut self, ui: &mut egui::Ui) {
        if let Some(promise) = &self.loading_status {
            if let Some(result_opt) = promise.ready() {
                match result_opt {
                    Some(result) => match result {
                        Ok(data) => {
                            dbg!(data);
                            self.loading_status = None;
                        } // TODO: Load data
                        Err(e) => {
                            if ui
                                .button(format!("Click to clear. Load Failed: {e:?}"))
                                .clicked()
                            {
                                self.loading_status = None;
                            }
                        }
                    },
                    None => self.loading_status = None, // User aborted
                }
            } else {
                ui.spinner();
            }
        } else {
            if ui.button("📂 Open log file...").clicked() {
                let ctx = ui.ctx().clone();
                self.loading_status = self.initiate_loading(ctx);
            }
            if ui.button("Clear Data").clicked() {
                self.data = None;
            }
        }
    }

    fn initiate_loading(&self, ctx: egui::Context) -> LoadingPromiseOpt {
        Some(execute(async move {
            let result = load_file().await;
            ctx.request_repaint();
            result
        }))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute(f: impl std::future::Future<Output = LoadingType> + 'static + Send) -> LoadingPromise {
    poll_promise::Promise::spawn_async(f)
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

impl eframe::App for LogViewerApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
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
            ui.horizontal(|ui| {
                ui.heading("Log Viewer");
                ui.separator();
                self.ui_loading(ui);
            });
            StripBuilder::new(ui)
                .size(Size::remainder().at_least(100.0)) // for the table
                .size(Size::exact(100.0)) // for the details
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            self.show_log_lines(ui);
                        });
                    });
                    strip.cell(|ui| {
                        self.show_log_details(ui);
                    });
                });
        });
    }
}

async fn load_file() -> LoadingType {
    let file = rfd::AsyncFileDialog::new().pick_file().await?;
    let text = file.read().await;
    Some(match String::from_utf8(text) {
        Ok(s) => Ok(s),
        Err(e) => Err(e.into()),
    })
}
