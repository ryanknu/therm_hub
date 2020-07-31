use chrono::NaiveDateTime;
use crate::http_client::parse;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Reading {
    pub id: i32,
    pub name: String,
    pub time: NaiveDateTime,
    pub is_hygrostat: bool,
    pub temperature: i32,
    pub relative_humidity: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadResult {
    thermostat_list: Vec<ReadThermostats>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadThermostats {
    thermostat_time: String,
    remote_sensors: Vec<ReadSensors>,
}

#[derive(Debug, Deserialize)]
struct ReadSensors {
    name: String,
    capability: Vec<ReadSensorCapability>,
}

#[derive(Debug, Deserialize)]
struct ReadSensorCapability {
    r#type: String,
    value: String,
}

pub fn read(bearer_token: &str) -> Vec<Reading> {
    if crate::is_offline() {
        return vec![
            Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline outside"),    relative_humidity: 0,  temperature: 77 },
            Reading { id: 0, is_hygrostat: true,  time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline thermostat"), relative_humidity: 65, temperature: 73 },
            Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline fridge"),     relative_humidity: 0,  temperature: 42 },
        ];
    }

    let mut readings: HashMap<String, Reading> = HashMap::new();
    if let Ok(result) = http_request(bearer_token) {
        if let Some(result) = parse::<ReadResult>(&result) {
            for read_result in result.thermostat_list {
                let time_str = read_result.thermostat_time;
                for sensor in read_result.remote_sensors {
                    let key = sensor.name;
                    for capability in sensor.capability {
                        if capability.r#type.eq("humidity") || capability.r#type.eq("temperature") {
                            let value: i32 = capability.value.parse().unwrap();
                            let is_hygrostat = capability.r#type.eq("humidity");
                            if let Some(reading) = readings.get_mut(&key) {
                                if is_hygrostat {
                                    reading.is_hygrostat = true;
                                    reading.relative_humidity = value;
                                }
                            } else {
                                readings.insert(key.clone(), Reading {
                                    id: 0,
                                    name: key.clone(),
                                    time: NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S").unwrap(),
                                    is_hygrostat,
                                    temperature: if is_hygrostat { -10000 } else { value },
                                    relative_humidity: if is_hygrostat { value } else { 0 },
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    readings.values().cloned().collect()
}


#[tokio::main]
async fn http_request(bearer_token: &str) -> Result<String, reqwest::Error> {
    println!("[ecobee] HTTP request read ecobee w/ token {}", bearer_token);

    let client = reqwest::Client::new();
    let body = client.get("https://api.ecobee.com/1/thermostat?json=%7B%22selection%22%3A%7B%22selectionType%22%3A%22registered%22%2C%22selectionMatch%22%3A%22%22%2C%22includeRuntime%22%3A%22true%22%2C%22includeSensors%22%3A%22true%22%7D%7D")
        .header("User-Agent", "github.com/ryanknu/therm_hub")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", bearer_token))
        .body("%7B%22selectionType%22%3A%22thermostats%22%2C%22selectionMatch%22%3A%22%22%2C%22includeSettings%22%3A%22true%22%7D")
        .send()
        .await?
        .text()
        .await?;

    Ok(body)
}