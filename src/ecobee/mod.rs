use chrono::NaiveDateTime;
use diesel::PgConnection;
pub use reading::Reading;
pub use token::Token;

pub mod install;
pub mod reading;
pub mod token;

pub fn current_token(db: &PgConnection) -> Option<Token> {
    match token::get_token(db) {
        None => None,
        Some(token) => {
            match token.is_expired() {
                false => Some(token.clone()),
                true => {
                    let result = token::get_from_remote_blocking(&token.refresh_token, token::GrantType::RefreshToken);
                    match result {
                        None => None,
                        Some(response) => {
                            token::save_token(&response.to_token(), db);
                            Some(response.to_token())
                        }
                    }
                },
            }
        },
    }
}

pub fn read(token: &Token) -> Vec<Reading> {
    // URL:  https://api.ecobee.com/1/thermostat?json=%7B%22selection%22%3A%7B%22selectionType%22%3A%22registered%22%2C%22selectionMatch%22%3A%22%22%2C%22includeRuntime%22%3A%22true%22%2C%22includeSensors%22%3A%22true%22%7D%7D
    // Head: Content-Type: application/json
    //       Authorization: Bearer UOd0oboKvQYCbYiBM32tXkJsrqynWWPl
    // Body: {"selectionType": "thermostats","selectionMatch": "", "includeSettings": "true"}
    if crate::is_offline() {
        return vec![
            Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline outside"),    relative_humidity: 0,  temperature: 77 },
            Reading { id: 0, is_hygrostat: true,  time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline thermostat"), relative_humidity: 65, temperature: 73 },
            Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline fridge"),     relative_humidity: 0,  temperature: 42 },
        ];
    }

    reading::read(&token.access_token)
}
