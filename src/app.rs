// TODO 3: Add search
// TODO 3: Add filter by and let user pick like ID or date or something like that
//
// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct LogViewerApp {
    selected_row: Option<usize>,
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

        // Make table clickable
        table = table.sense(egui::Sense::click());
        let data: Vec<i32> = (1..=5).collect();
        let num_rows = data.len();

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
                    row.col(|ui| {
                        ui.label(format!("{}", data[row_index]));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", data[row_index]));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", data[row_index]));
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", data[row_index]));
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
            use egui_extras::{Size, StripBuilder};
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
