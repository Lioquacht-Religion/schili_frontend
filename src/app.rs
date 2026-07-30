use std::{fmt::Debug, ops::RangeInclusive, sync::mpsc};

use chrono::{Utc};
use egui::{Color32, ComboBox};
use egui_plot::{GridMark, Line, PlotPoint};
use log::info;
use schili_api::api::{self, Sensor, SensorSimpleMeasurements, SensorType};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::model::IntervalUnit;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SchiliApp {
    sensor_search_form: SensorDataSearchForm,
    sensor_data: WaitForDataState<Vec<Sensor>>,
    #[serde(skip)]
    sensor_temps_plot_points: Vec<PlotPoint>,
    temp_data: WaitForDataState<api::SensorSimpleMeasurements>
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct WaitForDataState<D>{
    data: Option<D>,
    #[serde(skip)]
    sc: mpsc::Receiver<Option<D>>,
    #[serde(skip)]
    mp: mpsc::Sender<Option<D>>,
    waiting_for_data: bool,
}

impl<D> Default for WaitForDataState<D>{
    fn default() -> Self {
        let (mp, sc) = mpsc::channel();
        Self {
            data: None,
            sc,
            mp,
            waiting_for_data: false,
        }
    }
}

impl Default for SchiliApp {
    fn default() -> Self {
        Self {
            sensor_search_form: SensorDataSearchForm::new(),
            sensor_data: Default::default(),
            sensor_temps_plot_points: Vec::new(),
            temp_data: Default::default()        
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
                .show(ctx, |ui|{
                    ui.vertical_centered(|ui| ui.heading("Sensors"));
                    display_sensors(ui, &mut self.sensor_data, true)
                });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("eframe template");

            display_get_sensor_data_ui(
                ui, &mut self.sensor_search_form, &mut self.temp_data,
                &mut self.sensor_data
            );

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

#[derive(Debug, Deserialize, Serialize)]
struct SensorDataSearchForm {
    sensor_ref: String,
    sensor_type: SensorType,
    start_datetime_str: String,
    end_datetime_str: String,
    time_interval: String,
    time_interval_unit: IntervalUnit,
}

impl SensorDataSearchForm {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Default for SensorDataSearchForm {
    fn default() -> Self {
        Self {
            sensor_ref: String::new(),
            sensor_type: SensorType::Temperature,
            start_datetime_str: String::new(),
            end_datetime_str: String::new(),
            time_interval: String::new(),
            time_interval_unit: IntervalUnit::Minutes,
        }
    }
}

fn display_get_sensor_data_ui(
    ui: &mut egui::Ui,
    form: &mut SensorDataSearchForm,
    temp_data: &mut WaitForDataState<SensorSimpleMeasurements>,
    sensor_data: &mut WaitForDataState<Vec<Sensor>>
) {
    ui.horizontal(|ui| {
        ui.label("Sensor reference");
        ui.text_edit_singleline(&mut form.sensor_ref);
    });
    ui.horizontal(|ui| {
        ui.label("Sensor type");
        ComboBox::from_label("select sensor type")
            .selected_text(form.sensor_type.to_str())
            .show_ui(ui, |ui|{
                ui.selectable_value(&mut form.sensor_type, SensorType::Temperature, SensorType::Temperature.to_str());
                ui.selectable_value(&mut form.sensor_type, SensorType::Humidity, SensorType::Humidity.to_str());
                ui.selectable_value(&mut form.sensor_type, SensorType::Airpressure, SensorType::Airpressure.to_str());
                ui.selectable_value(&mut form.sensor_type, SensorType::Co2, SensorType::Co2.to_str());
                ui.selectable_value(&mut form.sensor_type, SensorType::BatteryVoltage, SensorType::BatteryVoltage.to_str());
                ui.selectable_value(&mut form.sensor_type, SensorType::ChipTemperature, SensorType::ChipTemperature.to_str());
            });
    });
    ui.horizontal(|ui| {
        ui.label("Start datetime");
        ui.text_edit_singleline(&mut form.start_datetime_str);
    });
    ui.horizontal(|ui| {
        ui.label("End datetime");
        ui.text_edit_singleline(&mut form.end_datetime_str);
    });
    ui.horizontal(|ui| {
        ui.label("Time interval in ms");
        ui.text_edit_singleline(&mut form.time_interval);
        ComboBox::from_label("select unit")
            .selected_text(form.time_interval_unit.to_str())
            .show_ui(ui, |ui|{
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Milliseconds, IntervalUnit::Milliseconds.to_str());
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Seconds, IntervalUnit::Seconds.to_str());
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Minutes, IntervalUnit::Minutes.to_str());
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Hours, IntervalUnit::Hours.to_str());
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Days, IntervalUnit::Days.to_str());
                ui.selectable_value(&mut form.time_interval_unit, IntervalUnit::Years, IntervalUnit::Years.to_str());
            });
    });
    if ui.button("Fetch measurement data").clicked() {
        send_measurements_get_request(form, temp_data);
    }
    if ui.button("Fetch sensor data").clicked() {
        get_all_sensors(sensor_data);
    }
}

fn send_measurements_get_request(form: &mut SensorDataSearchForm, temp_data: &mut WaitForDataState<SensorSimpleMeasurements>) {
    let sensor_type = form.sensor_type;
    let start_dt = chrono::NaiveDateTime::parse_from_str(
        &form.start_datetime_str,
        "%Y-%m-%d %H:%M:%S",
    );
    let end_dt = chrono::NaiveDateTime::parse_from_str(
        &form.end_datetime_str,
        "%Y-%m-%d %H:%M:%S",
    );
    let time_interval = form.time_interval.parse::<i64>();
    match (start_dt, end_dt, time_interval) {
        (
            Ok(start_dt), Ok(end_dt),
            Ok(time_interval)
        ) =>  {
            get_measurements_by_interval_in_range(
                temp_data,
                &form.sensor_ref,
                 sensor_type, 
                start_dt.and_utc(), end_dt.and_utc(), 
                form.time_interval_unit.to_timedelta(time_interval)
            );
        }
        (st, end, time_interval) => {
            if let Err(st) = st{
                println!("error start datetime parsing: {st}");
            }
            if let Err(end) = end{
                println!("error end datetime parsing: {end}");
            }
            if let Err(time_interval) = time_interval{
                println!("error time interval parsing: {time_interval}");
            }
        }
    }
}

const API_HOST : &'static str = "http://192.168.2.226:8080";
//"http://localhost:8080";

fn get_all_sensors(
    data: &mut WaitForDataState<Vec<api::Sensor>>,
) {
    //#[get("/sensor/all")]
    fetch_data_with_http(data, &format!("{API_HOST}/sensor/all"));
}

fn get_measurements_by_interval_in_range(
    data: &mut WaitForDataState<api::SensorSimpleMeasurements>,
    sensor_ref: &str,
    measurement_kind: SensorType,
    start_dt: chrono::DateTime<Utc>,
    end_dt: chrono::DateTime<Utc>,
    interval: chrono::TimeDelta,
) {
   //#[get("/sensor/measurement/avg/interval/range/{measurement_kind}/{sensor_reference}/{start_datetime}/{end_datetime}/{interval}")]
    let url = format!(
        "{API_HOST}/sensor/measurement/avg/interval/range/{}/{}/{}/{}/{}",
        <&SensorType as Into<&str>>::into(&measurement_kind),
        sensor_ref,
        start_dt.timestamp(),
        end_dt.timestamp(),
        interval.num_milliseconds(),
    );
    fetch_data_with_http(data, &url);
}

fn fetch_data_with_http<D: DeserializeOwned + Send + 'static>(
    data: &mut WaitForDataState<D>,
    url: &str,
) {
    let request = ehttp::Request::get(url);
    data.waiting_for_data = true;
    let mp = data.mp.clone();
    ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
        info!("fetching data");
        match result {
            Ok(result) => {
                info!("Status code: {:?}", &result.status);
                if result.status == 200 {
                    match serde_json::from_slice(&result.bytes) {
                        Ok(data) => {
                            info!("data rec");
                            let _ = mp.send(Some(data));
                        }
                        Err(e) => {
                            log::error!("data parse error: {e}");
                            let _ = mp.send(None);
                        }
                    }
                } else {
                    log::error!("http error status: {}", result.status);
                    let _ = mp.send(None);
                }
            }
            Err(e) => log::error!("data receive error: {e}"),
        }
    });
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Row(usize);

fn display_sensors(ui: &mut egui::Ui, data: &mut WaitForDataState<Vec<api::Sensor>>, auto_shrink: bool) {
    if data.waiting_for_data {
        if let Ok(sensors) = data.sc.try_recv() {
            data.data = sensors;
            data.waiting_for_data = false;
        } else {
            ui.vertical_centered(|ui| 
                ui.spinner());
        }
    }

    if let Some(sensors) = &mut data.data {
        let textstyle = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&textstyle);
        let row_num = sensors.len();

        let frame = egui::Frame::default()
            .corner_radius(5)
            .inner_margin(4.0);

        let mut from = None;
        let mut to = None;

        let (_, dropped_payload) = ui.dnd_drop_zone::<Row, ()>(frame, |ui| {
            ui.set_min_size(egui::vec2(64., 100.));
            egui::ScrollArea::vertical()
                .auto_shrink(auto_shrink)
                .show_rows(ui, row_height, row_num, |ui, row_range| {
                    for row in row_range {
                        let item_id = egui::Id::new(("drag_and_drop_sensor", row));
                        let item_row = Row(row);
                        let sensor = &sensors[row];

                        let response = egui::Frame::default()
                            .fill(Color32::from_rgb(40, 40, 70))
                            .corner_radius(5)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                        let response = ui.dnd_drag_source(item_id, item_row, |ui|{
                            egui::Frame::default()
                                .fill(Color32::from_rgb(50, 50, 80))
                                .corner_radius(5)
                                .inner_margin(4.0)
                                .show(ui, |ui| {
                                ui.label(format!(
                                    "Reference: {} | Name: {}", sensor.reference, sensor.name
                                ));
                            })
                        });
                        ui.horizontal_wrapped(|ui|{
                            ui.label("Sensor types:");
                            for sensor_type in sensor.sensor_types.iter(){
                                if ui.toggle_value(&mut false, get_sensor_type_short(*sensor_type)).clicked(){
                                    //TODO:
                                }
                            }
                        });
                        response
                            }).response;

                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<Row>()
                        ){
                            let rect = response.rect;
                            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
                            let insert_row_idx = if *hovered_payload == item_row {
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                row
                            }
                            else if pointer.y < rect.center().y{
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                row
                            }
                            else {
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                row + 1
                            };
                            if let Some(released_payload) = response.dnd_release_payload::<Row>(){
                                from = Some(released_payload);
                                to = Some(Row(insert_row_idx));
                            }
                        }
                    }
                });
        });

        if let Some(dragged_payload) = dropped_payload {
            from = Some(dragged_payload);
            to = Some(Row(usize::MAX));
        }

        if let (Some(from), Some(mut to)) = (from, to) {
            let item = sensors.remove(from.0);
            to.0 = to.0.min(sensors.len());
            sensors.insert(to.0, item);
        }
    }
}

fn get_sensor_type_short(sensor_type: SensorType) -> &'static str{
    match sensor_type {
        SensorType::Temperature => "T",
        SensorType::Humidity => "H",
        SensorType::Airpressure => "AP",
        SensorType::BatteryVoltage => "BV",
        SensorType::ChipTemperature => "CT",
        SensorType::Co2 => "CO2",
    }
}

fn display_sensor_data_plot(ui: &mut egui::Ui, app: &mut SchiliApp) {
    if app.temp_data.waiting_for_data {
        if let Ok(temps) = app.temp_data.sc.try_recv() {
            app.temp_data.data = temps;
            app.temp_data.waiting_for_data = false;

            if let Some(temps) = &mut app.temp_data.data {
                temps
                    .measurements
                    .sort_by(|t1, t2| t1.measure_time.cmp(&t2.measure_time).reverse());
            }
        if let Some(temps) = &app.temp_data.data{
            app.sensor_temps_plot_points.clear();
            temps
                .measurements
                .iter()
                .map(|t| {
                    PlotPoint::new(
                        (t.measure_time - chrono::DateTime::UNIX_EPOCH).num_seconds() as f64 / 100_000.0,
                        bigdecimal::ToPrimitive::to_f64(&t.measurement).unwrap(),
                    )
                })
                .for_each(|t| {
                    app.sensor_temps_plot_points.push(t);
                });
        }

        } else {
            ui.spinner();
        }
    }

    let label_fmt = |_text: &str, point: &PlotPoint|{
        let ts_seconds = 
            point.x as i64 * 100_000;
        let ts = chrono::DateTime::<Utc>::from_timestamp_secs(ts_seconds).unwrap();
        //let ts = chrono::DateTime::UNIX_EPOCH + (chrono::DateTime::UNIX_EPOCH - ts);
        format!(
        "
        Temperature: {} grad celsius
        timestamp: {}
        ",
        point.y,
        ts.format("%Y-%m-%d %H:%M:%S").to_string())
    };

    let time_formatter = |mark: GridMark, _range: &RangeInclusive<f64>|{
        let ts_seconds = (mark.value * 100_000.) as i64;
        let ts = chrono::DateTime::<Utc>::from_timestamp_secs(ts_seconds).unwrap();
        ts.format("%Y-%m-%d %H:%M:%S").to_string()
    };

    egui_plot::Plot::new("sensor_temps_plot")
        .x_axis_label("Time")
        .y_axis_label("Measurement")
        .x_axis_formatter(time_formatter)
        .label_formatter(label_fmt)
        .data_aspect(10.0)
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new("sensor_temps_plot_lines", &app.sensor_temps_plot_points[..])
                    .name("sensor_temps_plot_lines"),
            );
        });
}
