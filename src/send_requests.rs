use chrono::Utc;
use log::info;
use schili_api::api::{self, SensorSimpleMeasurements, SensorType};
use serde::de::DeserializeOwned;

use crate::model::{DataSearchError, SensorDataSearchForm, WaitForDataState};


pub fn send_measurements_get_request(
    form: &mut SensorDataSearchForm,
    sensor_type: SensorType,
    temp_data: &mut WaitForDataState<Result<SensorSimpleMeasurements, DataSearchError>>,
) {
    let start_dt =
        chrono::NaiveDateTime::parse_from_str(&form.start_datetime_str, "%Y-%m-%d %H:%M:%S");
    let end_dt = chrono::NaiveDateTime::parse_from_str(&form.end_datetime_str, "%Y-%m-%d %H:%M:%S");
    let time_interval = form.time_interval.parse::<i64>();
    match (start_dt, end_dt, time_interval) {
        (Ok(start_dt), Ok(end_dt), Ok(time_interval)) => {
            if let Some(time_interval) = form.time_interval_unit.to_timedelta(time_interval){
                get_measurements_by_interval_in_range(
                    temp_data, &form.sensor_ref,sensor_type,
                    start_dt.and_utc(), end_dt.and_utc(), time_interval
                );
            }
            else{
                get_measurements_in_range(
                    temp_data, &form.sensor_ref,sensor_type,
                    start_dt.and_utc(), end_dt.and_utc()
                );
            }
        }
        (st, end, time_interval) => {
            if let Err(st) = st {
                println!("error start datetime parsing: {st}");
            }
            if let Err(end) = end {
                println!("error end datetime parsing: {end}");
            }
            if let Err(time_interval) = time_interval {
                println!("error time interval parsing: {time_interval}");
            }
        }
    }
}

//const API_HOST: &'static str = "http://192.168.2.117:8080";
const API_HOST: &'static str = "http://192.168.2.226:8080";
//"http://localhost:8080";

pub fn get_all_sensors(data: &mut WaitForDataState<Result<Vec<api::SensorWithTypes>, DataSearchError>>) {
    //#[get("/sensor/all")]
    fetch_data_with_http(data, &format!("{API_HOST}/sensor/all"));
}

pub fn get_measurements_in_range(
    data: &mut WaitForDataState<Result<api::SensorSimpleMeasurements, DataSearchError>>,
    sensor_ref: &str,
    measurement_kind: SensorType,
    start_dt: chrono::DateTime<Utc>,
    end_dt: chrono::DateTime<Utc>,
) {
    //#[get("/sensor/measurement/range/{measurement_kind}/{sensor_reference}/{start_datetime}/{end_datetime}")]
    let url = format!(
        "{API_HOST}/sensor/measurement/range/{}/{}/{}/{}",
        <&SensorType as Into<&str>>::into(&measurement_kind),
        sensor_ref,
        start_dt.timestamp(),
        end_dt.timestamp(),
    );
    fetch_data_with_http(data, &url);
}

pub fn get_measurements_by_interval_in_range(
    data: &mut WaitForDataState<Result<api::SensorSimpleMeasurements, DataSearchError>>,
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

pub fn fetch_data_with_http<D: DeserializeOwned + Send + 'static>(
    data: &mut WaitForDataState<Result<D, DataSearchError>>,
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
                    let parsed_response : Result<D, _> = serde_json::from_slice(&result.bytes);
                    match parsed_response {
                        Ok(data) => {
                            info!("data rec");
                            let _ = mp.send(Some(Ok(data)));
                        }
                        Err(e) => {
                            log::error!("data parse error: {e}");
                            let _ = mp.send(Some(Err(
                                        DataSearchError::new(
                                            result.status, e.to_string()
                                            )
                                        )));
                        }
                    }
                } else {
                    log::error!("http error status: {}", result.status);
                    let _ = mp.send(Some(Err(
                                    DataSearchError::new(
                                        result.status, 
                                        result.text().unwrap_or(&result.status_text).to_string()
                                        )
                                    )));
                }
            }
            Err(e) => log::error!("data receive error: {e}"),
        }
    });
}
