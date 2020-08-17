use chrono::{Date, DateTime, TimeZone, Timelike, Utc};
use std::collections::HashMap;

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
    pub start_time: DateTime<Utc>,
    pub temperature: i32,
    pub temperature_unit: String,
    pub detailed_forecast: String,
}

impl Into<Condition> for ForecastTherm {
    fn into(self) -> Condition {
        Condition {
            date: format!("{}", self.start_time.date().format("%Y-%m-%d")),
            condition: self.detailed_forecast,
            day_temp: -1000,
            night_temp: -1000,
        }
    }
}

#[derive(Clone)]
pub struct Forecast {
    stale_time: DateTime<Utc>,
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
            stale_time: Utc.timestamp(0, 0),
            conditions: vec![],
        }
    }
}

impl From<Vec<ForecastTherm>> for Forecast {
    /// # Forecast From Therms
    /// Turns ForecastTherms into a Forecast. Under the hood, it uses a HashMap
    /// to arrange ForecastTherms by date, and combine multiples (daily and
    /// nightly) into single elements.
    fn from(therms: Vec<ForecastTherm>) -> Forecast {
        let mut map: HashMap<Date<Utc>, Condition> = HashMap::new();
        let mut max = Utc.timestamp(0, 0);
        for therm in therms {
            let key = therm.start_time.date();
            let hour = therm.start_time.hour();
            let temp = therm.temperature * 10;
            if therm.start_time.timestamp() > max.timestamp() {
                max = therm.start_time;
            }
            let mut condition = map.entry(key).or_insert_with(|| therm.into());
            if hour < 12 {
                condition.day_temp = temp;
            } else {
                condition.night_temp = temp;
            }
        }
        Forecast {
            stale_time: max,
            conditions: map.values().cloned().collect(),
        }
    }
}

/// # Current Weather
/// Returns a single forecast therm for what the weather is currently like.
#[cfg(any(test, feature = "offline"))]
pub fn current() -> Option<ForecastTherm> {
    Some(ForecastTherm {
        start_time: DateTime::parse_from_rfc3339("2020-01-01T00:00:00-05:00")
            .unwrap()
            .with_timezone(&Utc),
        temperature: 770,
        temperature_unit: String::from("F"),
        detailed_forecast: String::from("Slight Chance Showers And Thunderstorms"),
    })
}

#[cfg(not(any(test, feature = "offline")))]
pub fn current() -> Option<ForecastTherm> {
    match weather_request(true) {
        Ok(response) => match most_applicable(response) {
            Some(therm) => Some(ForecastTherm {
                detailed_forecast: therm.detailed_forecast,
                start_time: therm.start_time,
                temperature: therm.temperature * 10,
                temperature_unit: therm.temperature_unit,
            }),
            None => None,
        },
        Err(err) => {
            eprintln!("Failed getting weather! {:?}", err);
            None
        }
    }
}

/// # Fetch forecast
/// Fetches (and stores) the coming forecast for the upcoming week.
/// Should only fetch when the forecast on hand is stale. Return None
/// if no change.
#[cfg(any(test, feature = "offline"))]
pub fn forecast() -> Option<Forecast> {
    Some(Forecast {
        stale_time: Utc.timestamp(0, 0),
        conditions: vec![
            Condition {
                date: String::from("2020-07-20"),
                condition: String::from("Sunny"),
                day_temp: 800,
                night_temp: 710,
            },
            Condition {
                date: String::from("2020-07-21"),
                condition: String::from("Sunny"),
                day_temp: 780,
                night_temp: 700,
            },
            Condition {
                date: String::from("2020-07-22"),
                condition: String::from("Partly Sunny"),
                day_temp: 810,
                night_temp: 710,
            },
            Condition {
                date: String::from("2020-07-23"),
                condition: String::from("Raining"),
                day_temp: 750,
                night_temp: 680,
            },
            Condition {
                date: String::from("2020-07-24"),
                condition: String::from("Thunder Storms"),
                day_temp: 720,
                night_temp: 670,
            },
        ],
    })
}

#[cfg(not(any(test, feature = "offline")))]
pub fn forecast() -> Option<Forecast> {
    match weather_request(false) {
        Ok(response) => Some(Forecast::from(response)),
        Err(err) => {
            eprintln!("Failed getting weather! {:?}", err);
            None
        }
    }
}

/// # Most Applicable
/// Searches for and returns the closest ForecastTherm in a vector whose
/// start_time is closest to the current time. I admit this is not the proper
/// way to consume this data (the system *should* return the ForecastTherm
/// which the curren time is between) but this was a fun exercise to implement
/// closest.
#[cfg(not(any(test, feature = "offline")))]
fn most_applicable(therms: Vec<ForecastTherm>) -> Option<ForecastTherm> {
    let mut index = 0;
    let mut min_difference = 999999999;
    let now = Utc::now();
    for (i, therm) in therms.iter().enumerate() {
        let difference = now.timestamp() - therm.start_time.timestamp();
        let difference = difference.abs();
        if difference < min_difference {
            min_difference = difference;
            index = i;
        }
    }
    match therms.get(index) {
        Some(therm) => Some(therm.clone()),
        _ => None,
    }
}

/// # Weather Request
/// Gets either hourly or daily weather (based on boolean input var) from weather.gov
/// and returns a vector of ForecastTherm
#[cfg(not(any(test, feature = "offline")))]
#[tokio::main]
async fn weather_request(hourly: bool) -> Result<Vec<ForecastTherm>, crate::error::Error> {
    println!(
        "[worker] Getting {} weather",
        if hourly { "hourly" } else { "daily" }
    );

    let weather_url = std::env::var(if hourly {
        "WEATHER_URL_HOURLY"
    } else {
        "WEATHER_URL_DAILY"
    })
    .unwrap();
    let client = reqwest::Client::new();
    let body = client
        .get(&weather_url)
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .send()
        .await?
        .text()
        .await?;

    match serde_json::from_str::<ForecastResponse>(&body) {
        Ok(data) => Ok(data.properties.periods),
        Err(_) => Ok(Vec::new()),
    }
}
