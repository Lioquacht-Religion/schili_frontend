use egui::{Color32, ComboBox, ahash::{HashMap, HashMapExt}};
use schili_api::api::{self, SensorTrait};

use crate::{model::{DataSearchError, IntervalUnit, SensorData, SensorDataSearchForm, SensorMeasureData, SensorsDisplayData, WaitForDataState, get_sensor_type_short}, send_requests::send_measurements_get_request};

pub fn display_get_sensor_data_ui(
    ui: &mut egui::Ui,
    form: &mut SensorDataSearchForm,
    sensor_data: &mut SensorsDisplayData,
) {
    ui.horizontal(|ui| {
        ui.label("Sensor reference");
        ui.text_edit_singleline(&mut form.sensor_ref);
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
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::NoInterval,
                    IntervalUnit::NoInterval.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Milliseconds,
                    IntervalUnit::Milliseconds.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Seconds,
                    IntervalUnit::Seconds.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Minutes,
                    IntervalUnit::Minutes.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Hours,
                    IntervalUnit::Hours.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Days,
                    IntervalUnit::Days.to_str(),
                );
                ui.selectable_value(
                    &mut form.time_interval_unit,
                    IntervalUnit::Years,
                    IntervalUnit::Years.to_str(),
                );
            });
    });
    if ui.button("Fetch measurement data").clicked() {
        for s in sensor_data.sensors.iter_mut() {
            for st in s.sensor.sensor_types.iter() {
                let m = s
                    .measurements
                    .entry(*st)
                    .or_insert(SensorMeasureData::default());
                if m.is_active{
                    send_measurements_get_request(form, *st, &mut m.wait_for_measurements);
                }
            }
        }
    }
    if let Some(e) = &form.error{
        ui.label(format!("Error: status: '{}'; text: '{}'", e.error_code, e.error_text));
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Row(usize);

pub fn display_sensors(
    ui: &mut egui::Ui,
    sensors: &mut Vec<SensorData>,
    data: &mut WaitForDataState<Result<Vec<api::SensorWithTypes>, DataSearchError>>,
    error: &mut Option<DataSearchError>,
    auto_shrink: bool,
) {
    if data.waiting_for_data {
        if let Ok(new_sensors) = data.sc.try_recv() {
            data.waiting_for_data = false;
            if let Some(new_sensors) = new_sensors{
                sensors.clear();
                match new_sensors{
                    Ok(new_sensors) => {
                        for s in new_sensors.into_iter(){
                            sensors.push(
                                SensorData::new(s, HashMap::new()));
                        }
                    }
                    Err(e) => *error = Some(e),
                }
            }

        } else {
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

        let textstyle = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&textstyle);
        let row_num = sensors.len();

        let frame = egui::Frame::default().corner_radius(5).inner_margin(4.0);

        let mut from = None;
        let mut to = None;

        //TODO: only allow sensors to be dragged with right mose button
        // so that draggable ui can still be interacted with otherwise
        let (_, dropped_payload) = ui.dnd_drop_zone::<Row, ()>(frame, |ui| {
            ui.set_min_size(egui::vec2(64., 100.));
            egui::ScrollArea::vertical()
                .auto_shrink(auto_shrink)
                .show_rows(ui, row_height, row_num, |ui, row_range| {
                    for row in row_range {
                        let item_id = egui::Id::new(("drag_and_drop_sensor", row));
                        let item_row = Row(row);
                        let sensor_data = &mut sensors[row];
                        let sensor = &mut sensor_data.sensor;
                        let measurements = &mut sensor_data.measurements;

                        let response = egui::Frame::default()
                            .fill(Color32::from_rgb(40, 40, 70))
                            .corner_radius(5)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                let response = ui.dnd_drag_source(item_id, item_row, |ui| {
                                    egui::Frame::default()
                                        .fill(Color32::from_rgb(50, 50, 80))
                                        .corner_radius(5)
                                        .inner_margin(4.0)
                                        .show(ui, |ui| {
                                            ui.label(format!(
                                                "Reference: {} | Name: {}",
                                                sensor.get_reference(), sensor.get_name()
                                            ));
                                        })
                                });
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("Sensor types:");
                                    for sensor_type in sensor.sensor_types.iter() {
                                        let m = measurements
                                            .entry(*sensor_type)
                                            .or_insert(
                                                SensorMeasureData::new(
                                                    false, Vec::new(), Vec::new()
                                                    )
                                            );
                                        ui
                                            .toggle_value(
                                                &mut m.is_active,
                                                get_sensor_type_short(*sensor_type),
                                            );
                                        }
                                });
                                response
                            })
                            .response;

                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<Row>(),
                        ) {
                            let rect = response.rect;
                            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
                            let insert_row_idx = if *hovered_payload == item_row {
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                row
                            } else if pointer.y < rect.center().y {
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                row
                            } else {
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                row + 1
                            };
                            if let Some(released_payload) = response.dnd_release_payload::<Row>() {
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


