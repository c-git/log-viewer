use egui_extras::{Size, StripBuilder};

use self::data::Data;

// TODO 3: Add search
// TODO 3: Add filter by and let user pick like ID or date or something like that

mod data;
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    selected_row: Option<usize>,
    data: Option<Data>,

    #[serde(skip)]
    loading_status: LoadingStatus,
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