use chrono::TimeDelta;
use egui::ahash::HashMap;
use schili_api::api::{Sensor, SensorType, SimpleMeasurement};
use serde::{Deserialize, Serialize};

pub struct SensorRef(pub String);
pub struct SensorIndex(pub usize);

pub struct SensorsDisplayData{
    pub sensors: Vec<SensorData>,
    pub sens_ref_map: HashMap<SensorRef, SensorIndex>,
}

pub struct SensorData{
    pub sensor: Sensor,
    pub measurements: HashMap<SensorType, SensorMeasureData>,
}

pub struct SensorMeasureData{
    pub is_active: bool,
    pub measurements: Vec<SimpleMeasurement>
}

impl SensorData{
    pub fn new(sensor: Sensor, measurements: HashMap<SensorType, SensorMeasureData>) -> Self{
        Self { sensor, measurements }
    }
}

impl SensorMeasureData{
    pub fn new(is_active: bool, measurements: Vec<SimpleMeasurement>) -> Self{
        Self { is_active, measurements }
    }
}

impl Default for SensorMeasureData{
    fn default() -> Self {
        Self { is_active: false, measurements: Vec::default() }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum IntervalUnit{
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Months,
    Years
}

impl IntervalUnit{
    pub fn to_str(&self) -> &'static str{
        match self{
            IntervalUnit::Milliseconds => "Milliseconds",
            IntervalUnit::Seconds => "Seconds",
            IntervalUnit::Minutes => "Minutes",
            IntervalUnit::Hours => "Hours",
            IntervalUnit::Days => "Days",
            IntervalUnit::Months => "Months",
            IntervalUnit::Years => "Years",
        }
    }

    pub fn to_timedelta(&self, interval: i64) -> TimeDelta{
        match self{
            IntervalUnit::Milliseconds => TimeDelta::milliseconds(interval),
            IntervalUnit::Seconds => TimeDelta::seconds(interval),
            IntervalUnit::Minutes => TimeDelta::minutes(interval),
            IntervalUnit::Hours => TimeDelta::hours(interval),
            IntervalUnit::Days => TimeDelta::days(interval),
            //TODO: probably does not make sense to provide these as units
            // they do not have consistent conversion amounts
            IntervalUnit::Months => TimeDelta::days(interval * 30),
            IntervalUnit::Years => TimeDelta::days(interval * 365),
        }
    }
}
