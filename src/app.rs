use schili_api::api::Sensor;

use crate::{measurements_plot_ui::display_sensor_data_plot, model::{DataSearchError, SensorDataSearchForm, SensorsDisplayData, WaitForDataState}, send_requests::get_all_sensors, sensors_ui::{display_get_sensor_data_ui, display_sensors}};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SchiliApp {
    pub sensor_search_form: SensorDataSearchForm,
    pub sensor_data: SensorsDisplayData,
    pub wait_sensor_data: WaitForDataState<Result<Vec<Sensor>, DataSearchError>>,
}

impl Default for SchiliApp {
    fn default() -> Self {
        Self {
            sensor_search_form: SensorDataSearchForm::new(),
            sensor_data: Default::default(),
            wait_sensor_data: Default::default(),
        }
    }
}

impl SchiliApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for SchiliApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
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

        let content_rect = ctx.input(|i| i.content_rect());
        egui::TopBottomPanel::bottom("footer-bottom")
            .min_height(content_rect.height() * 0.1)
            .max_height(content_rect.height() * 0.15)
            .show(ctx, |ui| {
                ui.separator();

                ui.add(egui::github_link_file!(
                    "https://github.com/emilk/eframe_template/blob/main/",
                    "Source code."
                ));

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    powered_by_egui_and_eframe(ui);
                    egui::warn_if_debug_build(ui);
                });
            });

        egui::SidePanel::left("left-current-sensors")
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| ui.heading("Sensors"));
                if ui.button("Fetch sensor data").clicked() {
                    get_all_sensors(&mut self.wait_sensor_data);
                }
                display_sensors(
                    ui, 
                    &mut self.sensor_data.sensors, 
                    &mut self.wait_sensor_data, 
                    &mut self.sensor_search_form.error,
                    true
                )
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eframe template");

            display_get_sensor_data_ui(ui, &mut self.sensor_search_form, &mut self.sensor_data);

            ui.separator();

            display_sensor_data_plot(ui, self);
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
