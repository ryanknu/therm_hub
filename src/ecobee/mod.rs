#[cfg(any(test, feature="offline"))]
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

// TODO: this method seems to be an unecessary wrapper. replace with pub use.
#[allow(unused_variables)]
#[cfg(any(test, feature="offline"))]
pub fn read(token: &Token) -> Vec<Reading> {
    vec![
        Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline outside"),    relative_humidity: 0,  temperature: 77 },
        Reading { id: 0, is_hygrostat: true,  time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline thermostat"), relative_humidity: 65, temperature: 73 },
        Reading { id: 0, is_hygrostat: false, time: NaiveDateTime::from_timestamp(1595382655, 0), name: String::from("offline fridge"),     relative_humidity: 0,  temperature: 42 },
    ]
}

#[cfg(not(any(test, feature="offline")))]
pub fn read(token: &Token) -> Vec<Reading> {
    reading::read(&token.access_token)
}
