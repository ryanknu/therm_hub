#[cfg(not(any(test, feature="offline")))]
use chrono::{DateTime, Utc};
use std::collections::HashMap;
#[cfg(not(any(test, feature="offline")))]
use std::env;
#[cfg(not(any(test, feature="offline")))]
use serde_json::error::Error as SerdeJsonError;

#[derive(Deserialize, Debug)]
struct ForecastResponse {
    properties: ForecastProperties,
}

#[derive(Deserialize, Debug)]
struct ForecastProperties {
    updated: String,
    periods: Vec<ForecastTherm>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ForecastTherm {
    pub start_time: String,
    pub temperature: i32,
    pub temperature_unit: String,
    pub detailed_forecast: String,
}

#[derive(Clone)]
pub struct Forecast {
    stale_time: u64,
    pub conditions: Vec<Condition>,
}

#[derive(Serialize, Clone)]
pub struct Condition {
    date: String,
    condition: String,
    day_temp: i32,
    night_temp: i32,
}

impl Forecast {
    pub fn new() -> Forecast {
        Forecast {
            stale_time: 0,
            conditions: vec![],
        }
    }

    /// # Forecast From Therms
    /// Turns ForecastTherms into a Forecast. Under the hood, it uses a HashMap
    /// to arrange ForecastTherms by date, and combine multiples (daily and
    /// nightly) into single elements.
    pub fn from(therms: Vec<ForecastTherm>) -> Forecast {
        let mut map: HashMap<String, Condition> = HashMap::new();
        for (_i, therm) in therms.iter().enumerate() {
            let key = &therm.start_time[0..10];
            let hour = &therm.start_time[11..13];
            let hour: u8 = hour.parse().unwrap();
            if map.contains_key(key) {
                let mut condition = map.get_mut(key).unwrap();
                if hour < 12 {
                    condition.day_temp = therm.temperature * 10;
                } else {
                    condition.night_temp = therm.temperature * 10;
                }
            } else {
                if hour < 12 {
                    map.insert(key.to_string(), Condition {
                        date: String::from(key), 
                        condition: therm.detailed_forecast.clone(), 
                        day_temp: therm.temperature * 10, 
                        night_temp: -1000,
                    });
                } else {
                    map.insert(key.to_string(), Condition {
                        date: String::from(key), 
                        condition: therm.detailed_forecast.clone(), 
                        day_temp: -1000, 
                        night_temp: therm.temperature * 10,
                    });
                }
            }
        }
        Forecast {
            stale_time: 0, // TODO
            conditions: map.values().cloned().collect(),
        }
    }
}

/// Returns the current weather. 
/// ---------
/// TODO: store the current hourly forecast in a static variable
/// TODO: find the nearest and return it as a thermostat reading
/// TODO: figure out whe we're out of readings and request more
#[cfg(any(test, feature="offline"))]
pub fn current() -> Option<ForecastTherm> {
    Some(ForecastTherm {
        start_time: String::from("2020-01-01T00:00:00-05:00"),
        temperature: 770,
        temperature_unit: String::from("F"),
        detailed_forecast: String::from("Slight Chance Showers And Thunderstorms"),
    })
}

#[cfg(not(any(test, feature="offline")))]
pub fn current() -> Option<ForecastTherm> {
    let runtime = tokio::runtime::Runtime::new();
    if let Ok(mut runtime) = runtime {
        let response = runtime.block_on(request(true));
        if let Ok(response) = response {
            // print!("response: {}", response);
            if let Some(therm) = most_applicable(parse_vec(&response)) {
                return Some(ForecastTherm {
                    detailed_forecast: therm.detailed_forecast,
                    start_time: therm.start_time,
                    temperature: therm.temperature * 10,
                    temperature_unit: therm.temperature_unit,
                })
            }
        }
    }
    None
}

/// # Fetch forecast
/// Fetches (and stores) the coming forecast for the upcoming week.
/// Should only fetch when the forecast on hand is stale. Return None
/// if no change.
#[cfg(any(test, feature="offline"))]
pub fn forecast() -> Option<Forecast> {
    Some(Forecast {
        stale_time: 0,
        conditions: vec![
            Condition { date: String::from("2020-07-20"), condition: String::from("Sunny"), day_temp: 800, night_temp: 710 },
            Condition { date: String::from("2020-07-21"), condition: String::from("Sunny"), day_temp: 780, night_temp: 700 },
            Condition { date: String::from("2020-07-22"), condition: String::from("Partly Sunny"), day_temp: 810, night_temp: 710 },
            Condition { date: String::from("2020-07-23"), condition: String::from("Raining"), day_temp: 750, night_temp: 680 },
            Condition { date: String::from("2020-07-24"), condition: String::from("Thunder Storms"), day_temp: 720, night_temp: 670 },
        ],
    })
}

#[cfg(not(any(test, feature="offline")))]
pub fn forecast() -> Option<Forecast> {
    let runtime = tokio::runtime::Runtime::new();
    if let Ok(mut runtime) = runtime {
        let response = runtime.block_on(request(false));
        if let Ok(response) = response {
            // print!("response: {}", response);
            return Some(Forecast::from(parse_vec(&response)));
        }
    }
    None
}

/// # Most Applicable
/// Searches for and returns the closest ForecastTherm in a vector whose
/// start_time is closest to the current time. I admit this is not the proper
/// way to consume this data (the system *should* return the ForecastTherm
/// which the curren time is between) but this was a fun exercise to implement
/// closest.
#[cfg(not(any(test, feature="offline")))]
fn most_applicable(therms: Vec<ForecastTherm>) -> Option<ForecastTherm> {
    let mut index = 0;
    let mut last_difference = 999999999;
    let now = Utc::now();
    for (i, therm) in therms.iter().enumerate() {
        let therm_date = DateTime::parse_from_rfc3339(&therm.start_time);
        if let Ok(therm_date) = therm_date {
            let difference = now.timestamp() - therm_date.timestamp();
            let difference = difference.abs();
            if difference < last_difference {
                last_difference = difference;
                index = i;
            }
        }
    }
    match therms.iter().nth(index) {
        Some(therm) => Some(therm.clone()),
        _ => None,
    }
}

/// # Parse Vector of Weather Data
/// Parses a weather.gov API response into a vector
#[cfg(not(any(test, feature="offline")))]
fn parse_vec(data: &str) -> Vec<ForecastTherm> {
    let mut therms: Vec<ForecastTherm> = Vec::new();
    let data: Result<ForecastResponse, SerdeJsonError> = serde_json::from_str(&data);
    if let Ok(data) = data {
        for (_i, period) in data.properties.periods.iter().enumerate() {
            therms.push(period.clone());
        }
    }
    therms
}

// TODO: return my error, not reqwest
#[cfg(not(any(test, feature="offline")))]
async fn request(hourly: bool) -> Result<String, reqwest::Error> {
    println!("[worker] Getting {} weather", if hourly {"hourly"} else {"daily"});
    
    let weather_url = env::var(if hourly {"WEATHER_URL_HOURLY"} else {"WEATHER_URL_DAILY"}).unwrap();

    let client = reqwest::Client::new();
    let body = client.get(&weather_url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;

    Ok(body)
}
