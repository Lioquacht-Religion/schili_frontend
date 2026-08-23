use std::sync::mpsc;

use chrono::TimeDelta;
use egui::{Color32, ahash::HashMap};
use egui_plot::PlotPoint;
use schili_api::api::{Sensor, SensorSimpleMeasurements, SensorType, SimpleMeasurement};
use serde::{Deserialize, Serialize};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WaitForDataState<D> {
    pub data: Option<D>,
    #[serde(skip)]
    pub sc: mpsc::Receiver<Option<D>>,
    #[serde(skip)]
    pub mp: mpsc::Sender<Option<D>>,
    pub waiting_for_data: bool,
}

impl<D> Default for WaitForDataState<D> {
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSearchError{
    pub error_code: u16,
    pub error_text: String,
}

impl DataSearchError{
    pub fn new(error_code: u16, error_text: String) -> Self{
        Self { error_code, error_text }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorDataSearchForm {
    pub sensor_ref: String,
    pub sensor_type: SensorType,
    pub start_datetime_str: String,
    pub end_datetime_str: String,
    pub time_interval: String,
    pub time_interval_unit: IntervalUnit,
    pub error: Option<DataSearchError>,
}

impl SensorDataSearchForm {
    pub fn new() -> Self {
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
            error: None,
        }
    }
}


#[derive(serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
pub struct SensorRef(pub String);
#[derive(serde::Deserialize, serde::Serialize)]
pub struct SensorIndex(pub usize);

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct SensorsDisplayData {
    pub sensors: Vec<SensorData>,
    pub sens_ref_map: HashMap<SensorRef, SensorIndex>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SensorData {
    pub sensor: Sensor,
    pub measurements: HashMap<SensorType, SensorMeasureData>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SensorMeasureData {
    pub is_active: bool,
    pub wait_for_measurements: WaitForDataState<Result<SensorSimpleMeasurements, DataSearchError>>,
    pub measurements: Vec<SimpleMeasurement>,
    #[serde(skip)]
    pub plot_points: Vec<PlotPoint>,
}

impl SensorData {
    pub fn new(sensor: Sensor, measurements: HashMap<SensorType, SensorMeasureData>) -> Self {
        Self {
            sensor,
            measurements,
        }
    }
}

impl SensorMeasureData {
    pub fn new(
        is_active: bool,
        measurements: Vec<SimpleMeasurement>,
        plot_points: Vec<PlotPoint>,
    ) -> Self {
        Self {
            is_active,
            wait_for_measurements: Default::default(),
            measurements,
            plot_points,
        }
    }
}

impl Default for SensorMeasureData {
    fn default() -> Self {
        Self {
            is_active: false,
            wait_for_measurements: Default::default(),
            measurements: Vec::default(),
            plot_points: Vec::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum IntervalUnit {
    NoInterval,
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Months,
    Years,
}

impl IntervalUnit {
    pub fn to_str(&self) -> &'static str {
        match self {
            IntervalUnit::NoInterval => "No interval",
            IntervalUnit::Milliseconds => "Milliseconds",
            IntervalUnit::Seconds => "Seconds",
            IntervalUnit::Minutes => "Minutes",
            IntervalUnit::Hours => "Hours",
            IntervalUnit::Days => "Days",
            IntervalUnit::Months => "Months",
            IntervalUnit::Years => "Years",
        }
    }

    pub fn to_timedelta(&self, interval: i64) -> Option<TimeDelta> {
        match self {
            IntervalUnit::NoInterval => None,
            IntervalUnit::Milliseconds => Some(TimeDelta::milliseconds(interval)),
            IntervalUnit::Seconds => Some(TimeDelta::seconds(interval)),
            IntervalUnit::Minutes => Some(TimeDelta::minutes(interval)),
            IntervalUnit::Hours => Some(TimeDelta::hours(interval)),
            IntervalUnit::Days => Some(TimeDelta::days(interval)),
            //TODO: probably does not make sense to provide these as units
            // they do not have consistent conversion amounts
            IntervalUnit::Months => Some(TimeDelta::days(interval * 30)),
            IntervalUnit::Years => Some(TimeDelta::days(interval * 365)),
        }
    }
}

pub fn get_sensor_type_short(sensor_type: SensorType) -> &'static str {
    match sensor_type {
        SensorType::Temperature => "T",
        SensorType::Humidity => "H",
        SensorType::Airpressure => "AP",
        SensorType::LightIntensity => "LI",
        SensorType::BatteryVoltage => "BV",
        SensorType::ChipTemperature => "CT",
        SensorType::Co2 => "CO2",
    }
}

pub fn get_sensor_type_color(sensor_type: SensorType) -> Color32 {
    match sensor_type {
        SensorType::Temperature => Color32::from_rgb(200, 20, 20),
        SensorType::Humidity => Color32::from_rgb(20, 20, 200),
        SensorType::Airpressure => Color32::from_rgb(100, 140, 100),
        SensorType::LightIntensity => Color32::from_rgb(180, 240, 10),
        SensorType::BatteryVoltage => Color32::from_rgb(200, 200, 20),
        SensorType::ChipTemperature => Color32::from_rgb(160, 80, 80),
        SensorType::Co2 => Color32::from_rgb(200, 20, 200),
    }
}

impl WaitForDataState<Result<SensorSimpleMeasurements, DataSearchError>> {
    pub fn wait_for_data_init_plot(
        &mut self,
        ui: &mut egui::Ui,
        measurements: &mut Vec<SimpleMeasurement>,
        plot_points: &mut Vec<PlotPoint>,
        search_error: &mut Option<DataSearchError>
    ) {
        if self.waiting_for_data {
            if let Ok(temps) = self.sc.try_recv() {
                self.data = None;
                self.waiting_for_data = false;

                if let Some(res_measurements) = temps{
                    match res_measurements{
                        Ok(res_measurements) => { 
                            *measurements = res_measurements.measurements;
                            fill_measurements_plot_points(measurements, plot_points)
                        }
                        Err(e) => *search_error = Some(e),
                    }

                }
            } else {
                ui.spinner();
            }
        }
    }
}

fn fill_measurements_plot_points(
    measurements: &mut Vec<SimpleMeasurement>,
    plot_points: &mut Vec<PlotPoint>,
){
                    measurements
                        .sort_by(|t1, t2| t1.measure_time.cmp(&t2.measure_time).reverse());
                    plot_points.clear();
                    measurements
                        .iter()
                        .map(|t| {
                            PlotPoint::new(
                                (t.measure_time - chrono::DateTime::UNIX_EPOCH).num_seconds()
                                    as f64,
                                bigdecimal::ToPrimitive::to_f64(&t.measurement).unwrap(),
                            )
                        })
                        .for_each(|t| {
                            plot_points.push(t);
                        });
}


