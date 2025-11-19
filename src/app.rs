use std::sync::mpsc;

use chrono::Utc;
use egui_plot::{Line, PlotPoint};
use log::info;
use schili_api::api;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    sensor_ref: String,
    sensor_temps: Option<api::SensorTempMeasurements>,
    #[serde(skip)]
    sensor_temps_plot_points: Vec<PlotPoint>,
    #[serde(skip)]
    sensor_temps_sc: mpsc::Receiver<Option<api::SensorTempMeasurements>>,
    #[serde(skip)]
    sensor_temps_mp: mpsc::Sender<Option<api::SensorTempMeasurements>>,
    waiting_for_data: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (mp, sc) = mpsc::channel();
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            sensor_ref: String::from("bme280_1"),
            sensor_temps: None,
            sensor_temps_plot_points: Vec::new(),
            sensor_temps_sc: sc,
            sensor_temps_mp: mp,
            waiting_for_data: false,
        }
    }
}

impl TemplateApp {
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

impl eframe::App for TemplateApp {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            display_get_sensor_data_ui(ui, self);

            ui.separator();

            ui.columns(2, |cols |{
                cols[0].vertical(|ui| {
                        display_sensor_data(ui, self, false);
                    });
                cols[1].vertical(|ui| {
                        display_sensor_data_plot(ui, self);
                    });
            });
        });

        let content_rect = ctx.input(|i| i.content_rect());
        egui::TopBottomPanel::bottom("footer-bottom")
            .min_height(content_rect.height() * 0.3)
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

fn display_get_sensor_data_ui(ui: &mut egui::Ui, app: &mut TemplateApp) {
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut app.sensor_ref);
        if ui.button("Fetch Temperature Data").clicked() {
            send_temps_get_request(app);
        }
    });
}

fn send_temps_get_request(app: &mut TemplateApp) {
    let request = ehttp::Request::get(format!(
        "http://localhost:8080/sensor/temperature/{}",
        &app.sensor_ref
    ));
    let mp = app.sensor_temps_mp.clone();
    app.waiting_for_data = true; 
    ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
        info!("fetching temp data");
        if let Ok(result) = result {
            info!("Status code: {:?}", &result.status);
            if result.status == 200 {
                if let Ok(temps) = serde_json::from_slice(&result.bytes[..]) {
                    let _ = mp.send(Some(temps));
                } else {
                    let _ = mp.send(None);
                }
            } else {
                let _ = mp.send(None);
            }
        }
    });
}

fn display_sensor_data(ui: &mut egui::Ui, app: &mut TemplateApp, auto_shrink: bool) {
    if app.waiting_for_data {
        if let Ok(temps) = app.sensor_temps_sc.try_recv() {
            app.sensor_temps = temps;
            app.waiting_for_data = false; 

            if let Some(temps) = &mut app.sensor_temps{
                temps.temp_measurements.sort_by(
                    |t1, t2| 
                    t1.measure_time.cmp(&t2.measure_time).reverse());
            }
        }
        else{
            ui.spinner();
        }
    }

    if let Some(temps) = &mut app.sensor_temps {
        let textstyle = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&textstyle);
        let row_num = temps.temp_measurements.len();
        egui::ScrollArea::vertical()
            .auto_shrink(auto_shrink)
            .show_rows(ui, row_height, row_num, |ui, row_range| {
                for row in row_range {
                    let temp = &temps.temp_measurements[row];
                    ui.label(format!(
                        "temp: {} °C | time: {}",
                        temp.temp_celsius, temp.measure_time
                    ));
                }
            });
    }
}

fn display_sensor_data_plot(ui: &mut egui::Ui, app: &mut TemplateApp){
    if let Some(temps) = &app.sensor_temps{
        app.sensor_temps_plot_points.clear();
        temps.temp_measurements.iter()
            .map(|t| PlotPoint::new(
                    (t.measure_time - chrono::DateTime::UNIX_EPOCH).num_seconds() as f64,
                    bigdecimal::ToPrimitive::to_f64( &t.temp_celsius).unwrap(), 
            ))
            .for_each(|t|{
                app.sensor_temps_plot_points.push(t);
            });
    }
    egui_plot::Plot::new("sensor_temps_plot")
        .data_aspect(10.0)
        .show(ui, |plot_ui|{
            plot_ui.line(Line::new("sensor_temps_plot_lines", &app.sensor_temps_plot_points[..])
                .name("sensor_temps_plot_lines"));
        });
}
