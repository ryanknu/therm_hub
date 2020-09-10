use crate::{
    ecobee, establish_connection,
    weather::{daily_forecast, hourly_forecast, DailyCondition, Forecast, HourlyCondition},
    NowResponse, Thermostat, NOW_RES, NOW_STR,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{sync::Arc, thread, time::Duration};

/// # Start Worker Thread
/// The worker thread is a background program that retrieves information
/// from Internet services every 5 minutes. It will put historical entries
/// in the database, and update static data.
pub fn start() {
    println!("Starting worker thread");
    thread::spawn(|| {
        let mut last_timestamp = Utc::now();
        loop {
            if throttle(&mut last_timestamp) {
                work();
            }
            thread::sleep(Duration::from_secs(4));
        }
    });
}

/// # Check
/// Allows the `work()` function to be called outside of the background
/// thread to make sure that readings can be obtained.
/// TODO: Create a pathway to return false, then it would just be `work()`
pub fn check() -> bool {
    println!("Starting check of worker");
    work();
    true
}

/// # Throttle
/// Used to control how often the thread should "do work" independent of
/// how often it "wakes up".
fn throttle(last_timestamp: &mut DateTime<Utc>) -> bool {
    let now = Utc::now();
    let decision = now - *last_timestamp > ChronoDuration::seconds(300);
    if decision {
        *last_timestamp = Utc::now();
    }
    decision
}

/// # Work
/// The unit of work that the worker thread does every time it's invoked.
fn work() {
    let mut therms: Vec<Thermostat> = Vec::new();

    // TODO: convert get_weather() to return Vec<Therm>,
    //       call most_applicable from here, call
    //       write_hourly_forecast. Remove ref. to error_handling.

    // TODO: error handling, clean up var names

    // TODO: use join

    let hourly_forecast = hourly_forecast();
    if let Some(forcast) = hourly_forecast.clone() {
        if let Some(condition) = most_applicable(forcast.conditions) {
            // TODO: we should really be calling "into" or "from" here...
            therms.push(Thermostat::new(
                String::from("weather.gov"),
                condition.date,
                condition.temperature,
            ));
        }
    }
    write_hourly_forecast(hourly_forecast);

    // Write thermostats to db
    // TODO: don't write duplicates :P
    let db = establish_connection();
    if let Some(token) = ecobee::current_token(&db) {
        for reading in ecobee::read(&token.access_token) {
            therms.push(Thermostat::new2(
                reading.name,
                reading.time,
                reading.is_hygrostat,
                reading.temperature,
                reading.relative_humidity,
            ));
        }
    }

    for therm in &therms {
        therm.insert(&db);
    }
    drop(db);

    write_daily_forecast(daily_forecast());
    write_thermostats(therms);
    serialize_now();
}

/// # Write Thermostats
/// Writes thermostats to the now response
fn write_thermostats(therms: Vec<Thermostat>) {
    let now_res = Arc::clone(&NOW_RES);
    let mut now_res = now_res.write().unwrap();
    *now_res = NowResponse {
        forecast_hourly: now_res.forecast_hourly.clone(),
        forecast_daily: now_res.forecast_daily.clone(),
        thermostats: therms,
    };
}

/// # Write Hourly Forecast
/// Writes forecast_hourly to the now response
fn write_hourly_forecast(forecast: Option<Forecast<HourlyCondition>>) {
    match forecast {
        None => (),
        Some(forecast) => {
            let now_res = Arc::clone(&NOW_RES);
            let mut now_res = now_res.write().unwrap();
            *now_res = NowResponse {
                forecast_hourly: forecast.conditions,
                forecast_daily: now_res.forecast_daily.clone(),
                thermostats: now_res.thermostats.clone(),
            };
        }
    }
}

/// # Write Daily Forecast
/// Writes forecast_daily to the now response
fn write_daily_forecast(forecast: Option<Forecast<DailyCondition>>) {
    match forecast {
        None => (),
        Some(forecast) => {
            let now_res = Arc::clone(&NOW_RES);
            let mut now_res = now_res.write().unwrap();
            *now_res = NowResponse {
                forecast_hourly: now_res.forecast_hourly.clone(),
                forecast_daily: forecast.conditions,
                thermostats: now_res.thermostats.clone(),
            };
        }
    }
}

/// # Serialize Now
/// Perform a one-time JSON encoding of NOW_RES, storing the result in static
/// NOW_STR. This gives us the world's TINIEST performance gain by repetitive
/// calls to now not having to serialize the data again.
fn serialize_now() {
    let now_res = Arc::clone(&NOW_RES);
    let now_res = now_res.read().unwrap();
    let now_str = Arc::clone(&NOW_STR);
    let mut now_str = now_str.write().unwrap();
    *now_str = serde_json::to_string(&*now_res).unwrap();
}

/// # Most Applicable
/// Searches for and returns the closest HourlyCondition in a vector whose
/// start_time is closest to the current time. I admit this is not the proper
/// way to consume this data (the system *should* return the HourlyCondition
/// which the curren time is between) but this was a fun exercise to implement
/// closest.
pub fn most_applicable(conditions: Vec<HourlyCondition>) -> Option<HourlyCondition> {
    let mut index = 0;
    let mut min_difference = i64::MAX;
    let now = Utc::now();
    for (i, condition) in conditions.iter().enumerate() {
        let difference = now.timestamp() - condition.date.timestamp();
        let difference = difference.abs();
        if difference < min_difference {
            min_difference = difference;
            index = i;
        }
    }
    match conditions.get(index) {
        Some(therm) => Some(therm.clone()),
        _ => None,
    }
}
