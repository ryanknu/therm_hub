use chrono::{Date, DateTime, TimeZone, Timelike, Utc};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
struct ApiResponse {
    properties: ApiWrapper,
}

#[derive(Deserialize, Debug)]
struct ApiWrapper {
    updated: String,
    periods: Vec<ApiCondition>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiCondition {
    pub start_time: DateTime<Utc>,
    pub temperature: i32,
    pub temperature_unit: String,
    pub detailed_forecast: String,
    pub short_forecast: String,
}

impl Into<DailyCondition> for ApiCondition {
    fn into(self) -> DailyCondition {
        DailyCondition {
            date: self.start_time,
            condition: self.short_forecast,
            day_temperature: -1000,
            night_temperature: -1000,
        }
    }
}

impl Into<HourlyCondition> for ApiCondition {
    fn into(self) -> HourlyCondition {
        HourlyCondition {
            date: self.start_time,
            condition: self.detailed_forecast,
            temperature: self.temperature,
        }
    }
}

#[derive(Clone)]
pub struct Forecast<T> {
    stale_time: DateTime<Utc>,
    pub conditions: Vec<T>,
}

#[derive(Serialize, Clone)]
pub struct DailyCondition {
    date: DateTime<Utc>,
    condition: String,
    day_temperature: i32,
    night_temperature: i32,
}

#[derive(Serialize, Clone)]
pub struct HourlyCondition {
    pub date: DateTime<Utc>,
    pub condition: String,
    pub temperature: i32,
}

/// # Forecast From Therms
/// Turns ApiConditions into a Forecast. Under the hood, it uses a HashMap
/// to arrange ApiConditions by date, and combine multiples (daily and
/// nightly) into single elements.
impl Into<Forecast<DailyCondition>> for Vec<ApiCondition> {
    fn into(self) -> Forecast<DailyCondition> {
        let mut map: HashMap<Date<Utc>, DailyCondition> = HashMap::new();
        let mut max = Utc.timestamp(0, 0);
        for condition in self {
            let key = condition.start_time.date();
            let hour = condition.start_time.hour();
            let temp = condition.temperature * 10;
            if condition.start_time.timestamp() > max.timestamp() {
                max = condition.start_time;
            }
            let mut condition = map.entry(key).or_insert_with(|| condition.into());
            if hour < 12 {
                condition.day_temperature = temp;
            } else {
                condition.night_temperature = temp;
            }
        }
        Forecast {
            stale_time: max,
            conditions: map.values().cloned().collect(),
        }
    }
}

impl Into<Forecast<HourlyCondition>> for Vec<ApiCondition> {
    fn into(self) -> Forecast<HourlyCondition> {
        Forecast {
            stale_time: Utc::now(),
            conditions: self
                .into_iter()
                .map(|condition| -> HourlyCondition { condition.into() })
                .collect(),
        }
    }
}

/// # Fetch forecast
/// Fetches (and stores) the coming forecast for the upcoming week.
/// Should only fetch when the forecast on hand is stale. Return None
/// if no change.
#[cfg(any(test, feature = "offline"))]
pub fn daily_forecast() -> Option<Forecast<DailyCondition>> {
    Some(Forecast {
        stale_time: Utc.timestamp(0, 0),
        conditions: vec![
            DailyCondition {
                date: Utc.timestamp(1595203200, 0),
                condition: String::from("Sunny"),
                day_temperature: 800,
                night_temperature: 710,
            },
            DailyCondition {
                date: Utc.timestamp(1595289600, 0),
                condition: String::from("Sunny"),
                day_temperature: 780,
                night_temperature: 700,
            },
            DailyCondition {
                date: Utc.timestamp(1595376000, 0),
                condition: String::from("Partly Sunny"),
                day_temperature: 810,
                night_temperature: 710,
            },
            DailyCondition {
                date: Utc.timestamp(1595462400, 0),
                condition: String::from("Raining"),
                day_temperature: 750,
                night_temperature: 680,
            },
            DailyCondition {
                date: Utc.timestamp(1595548800, 0),
                condition: String::from("Thunder Storms"),
                day_temperature: 720,
                night_temperature: 670,
            },
        ],
    })
}

#[cfg(not(any(test, feature = "offline")))]
pub fn daily_forecast() -> Option<Forecast<DailyCondition>> {
    match weather_request_retry_wrapper(false) {
        Ok(response) => Some(response.into()),
        Err(err) => {
            eprintln!("Failed getting weather! {:?}", err);
            None
        }
    }
}

//////////////////////////////////
#[cfg(any(test, feature = "offline"))]
pub fn hourly_forecast() -> Option<Forecast<HourlyCondition>> {
    Some(Forecast {
        stale_time: Utc.timestamp(0, 0),
        conditions: vec![
            HourlyCondition {
                date: Utc.timestamp(1595232000, 0),
                condition: String::from("Sunny"),
                temperature: 800,
            },
            HourlyCondition {
                date: Utc.timestamp(1595235600, 0),
                condition: String::from("Sunny"),
                temperature: 780,
            },
            HourlyCondition {
                date: Utc.timestamp(1595239200, 0),
                condition: String::from("Partly Sunny"),
                temperature: 810,
            },
            HourlyCondition {
                date: Utc.timestamp(1595242800, 0),
                condition: String::from("Raining"),
                temperature: 750,
            },
            HourlyCondition {
                date: Utc.timestamp(1595246400, 0),
                condition: String::from("Thunder Storms"),
                temperature: 720,
            },
            // TODO: return however many the weather.gov api returns
        ],
    })
}
#[cfg(not(any(test, feature = "offline")))]
pub fn hourly_forecast() -> Option<Forecast<HourlyCondition>> {
    match weather_request_retry_wrapper(true) {
        Ok(response) => Some(response.into()),
        Err(err) => {
            eprintln!("Failed getting weather! {:?}", err);
            None
        }
    }
}
/// //////////////////////////////

/// # Weather Request Retry Wrapper
/// Calls `weather_request` but implements up to 5 retries.
#[cfg(not(any(test, feature = "offline")))]
fn weather_request_retry_wrapper(hourly: bool) -> anyhow::Result<Vec<ApiCondition>> {
    for _ in 1..5 {
        if let Ok(result) = weather_request(hourly) {
            return Ok(result);
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    weather_request(hourly)
}

/// # Weather Request
/// Gets either hourly or daily weather (based on boolean input var) from weather.gov
/// and returns a vector of ApiCondition
#[cfg(not(any(test, feature = "offline")))]
#[tokio::main]
async fn weather_request(hourly: bool) -> anyhow::Result<Vec<ApiCondition>> {
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
    match serde_json::from_str::<ApiResponse>(&body) {
        Ok(data) => Ok(data.properties.periods),
        Err(_) => Ok(Vec::new()),
    }
}
