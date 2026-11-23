use std::ops::RangeInclusive;

use chrono::Utc;
use egui_plot::{AxisHints, Line, PlotPoint};

use crate::{SchiliApp, model::get_sensor_type_color};

pub fn display_sensor_data_plot(ui: &mut egui::Ui, app: &mut SchiliApp) {
    // try to init data being waited for
    for s in app.sensor_data.sensors.iter_mut(){
        for m in s.measurements.values_mut(){
            m.wait_for_measurements.wait_for_data_init_plot(
                ui, &mut m.measurements, &mut m.plot_points,
                &mut app.sensor_search_form.error
            );
        }
    }

    let label_fmt = |_text: &str, point: &PlotPoint| {
        let ts_seconds = point.x as i64;
        match chrono::DateTime::<Utc>::from_timestamp_secs(ts_seconds){
            Some(ts) =>
                format!(
                    "
                Temperature: {} grad celsius
                timestamp: {}
                ",
                    point.y,
                    ts.format("%Y-%m-%d %H:%M:%S").to_string()
                ),
            None => "ParseError".to_string(),
        }
    };

    let time_formatter = |mark: egui_plot::GridMark, _range: &RangeInclusive<f64>| {
        let ts_seconds = mark.value as i64;
        match chrono::DateTime::<Utc>::from_timestamp_secs(ts_seconds){
            Some(ts) => ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            None => "ParseError".to_string(),
        }
    };

    let celsius_formatter = |mark: egui_plot::GridMark, _range: &RangeInclusive<f64>| {
        let celsius = mark.value;
        format!("{celsius:.0} C")
    };

    let percentage_formatter = |mark: egui_plot::GridMark, _range: &RangeInclusive<f64>| {
        let percent = mark.value;
        format!("{percent:.0}%")
    };

    egui_plot::Plot::new("sensor_temps_plot")
        .x_axis_label("Time")
        .y_axis_label("Measurement")
        .x_axis_formatter(time_formatter)
        .label_formatter(label_fmt)
        .data_aspect(300.)
        .custom_y_axes(vec![
            AxisHints::new_y()
            .label("Temperature")
            .formatter(celsius_formatter),
            AxisHints::new_y()
            .label("Humidity")
            .formatter(percentage_formatter),
            AxisHints::new_y()
            .label("Airpressure"),
            AxisHints::new_y()
            .label("Batteryvoltage"),
            AxisHints::new_y()
            .label("Chiptemperature")
            .formatter(celsius_formatter),
        ])
        .show(ui, |plot_ui| {
            for s in app.sensor_data.sensors.iter() {
                for (st, m) in s.measurements.iter() {
                    if m.is_active {
                        plot_ui
                            .line(
                            Line::new("sensor_temps_plot_lines", &m.plot_points[..])
                            .color(get_sensor_type_color(*st))
                                .name("sensor_temps_plot_lines"),
                        );
                    }
                }
            }
        });
}
