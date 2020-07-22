use chrono::NaiveDateTime;
use crate::therm::Thermostat;
use std::env;

pub struct InstallResponse {
    ecobee_pin: String,
    code: String,
}

pub struct Token {
    access_token: String,
    refresh_token: String,
    expires_in: i32,
}

struct ReadResult {
    thermostatList: Vec<ReadThermostats>,
}

struct ReadThermostats {
    remoteSensors: Vec<ReadSensors>,
}

struct ReadSensors {
    name: String,
    capabilities: Vec<ReadSensorCapability>,
}

struct ReadSensorCapability {
    r#type: String,
    value: String,
}

/// Install
pub fn install() -> InstallResponse {
    if crate::is_offline() {
        return InstallResponse {
            ecobee_pin: String::from("a263"),
            code: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        }
    }

    let client_id = env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!("https://api.ecobee.com/authorize?response_type=ecobeePin&client_id={}&scope=smartRead", client_id);
    // TODO

    InstallResponse {
        ecobee_pin: String::from("a263"),
        code: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
    }
}

pub fn get_token(response: InstallResponse) -> Token {
    if crate::is_offline() {
        return Token {
            access_token: String::from("GjuHR3N9sEdZ59QvB7rtX5XhzPmyCoi0"),
            refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
            expires_in: 3600,
        }
    }

    let client_id = env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!("https://api.ecobee.com/token?grant_type=ecobeePin&code={}&client_id={}", response.code, client_id);

    Token {
        access_token: String::from("GjuHR3N9sEdZ59QvB7rtX5XhzPmyCoi0"),
        refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        expires_in: 3600,
    }
}

pub fn refresh_token(token: Token) -> Token {
    if crate::is_offline() {
        return Token {
            access_token: String::from("GjuHR3N9sEdZ59QvB7rtX5XhzPmyCoi0"),
            refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
            expires_in: 3600,
        }
    }

    let client_id = env::var("ECOBEE_CLIENT_ID").unwrap();
    let url = format!("https://api.ecobee.com/token?grant_type=refresh_token&code={}&client_id={}", token.refresh_token, client_id);

    Token {
        access_token: String::from("GjuHR3N9sEdZ59QvB7rtX5XhzPmyCoi0"),
        refresh_token: String::from("czTAVXg4thWHhVosrdZPmf8wj0iiKa7A"),
        expires_in: 3600,
    }
    
}

pub fn read() -> Vec<Thermostat> {
    // URL:  https://api.ecobee.com/1/thermostat?json=%7B%22selection%22%3A%7B%22selectionType%22%3A%22registered%22%2C%22selectionMatch%22%3A%22%22%2C%22includeRuntime%22%3A%22true%22%2C%22includeSensors%22%3A%22true%22%7D%7D
    // Head: Content-Type: application/json
    //       Authorization: Bearer UOd0oboKvQYCbYiBM32tXkJsrqynWWPl
    // Body: {"selectionType": "thermostats","selectionMatch": "", "includeSettings": "true"}
    if crate::is_offline() {
        return vec![
            Thermostat { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline outside"),    relative_humidity: 0,  temperature: 77 },
            Thermostat { id: 0, is_hygrostat: true,  time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline thermostat"), relative_humidity: 65, temperature: 73 },
            Thermostat { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline fridge"),     relative_humidity: 0,  temperature: 42 },
        ];
    }

    vec![
        Thermostat { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline outside"),    relative_humidity: 0,  temperature: 77 },
        Thermostat { id: 0, is_hygrostat: true,  time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline thermostat"), relative_humidity: 65, temperature: 73 },
        Thermostat { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline fridge"),     relative_humidity: 0,  temperature: 42 },
    ]
}