#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;

use chrono::Duration as ChronoDuration;
use chrono::Utc;
use diesel::prelude::*;
use diesel_migrations::*;
use dotenv::dotenv;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

mod ecobee;
mod http;
mod image;
mod schema;
mod therm;
mod weather;
use therm::Thermostat;
use weather::Condition;

static VERSION: u32 = 20200818;

/// Set up in-memory data cache for web server. We want to keep track of:
/// 1. The entire string repsonse for "/now" requests, since it only changes
///    when the data model changes, and allows crazy fast response times.
///    We need to keep additional data in memory to construct this from partial
///    responses.
/// 2. A vector of thermostat readings. This is kept in cache to allow the
///    weather virtual thermostat to be pushed to the collection independently
///    of Ecobee thermostat readings.
/// 3. A daily weather forecast. Because we only query this at most once a day,
///    we need to cache it for building the NOW_RES.
/// This utilizes the lazy_static crate to enable a simpler syntax for creating
/// static variables that require runtime initialization, for example calling
/// the new function.
type StsString = Arc<RwLock<String>>;
type StsNowResponse = Arc<RwLock<NowResponse>>;
lazy_static! {
    pub static ref NOW_STR: StsString = Arc::new(RwLock::new(String::new()));
    static ref NOW_RES: StsNowResponse = Arc::new(RwLock::new(NowResponse::default()));
}

#[derive(Serialize)]
struct NowResponse {
    forecast: Vec<Condition>,
    thermostats: Vec<Thermostat>,
}

impl Default for NowResponse {
    fn default() -> Self {
        Self {
            forecast: vec![],
            thermostats: vec![],
        }
    }
}

/// # Therm Hub
/// A backend system to bring my Ecobee thermostat readings together. Also,
/// my first Rust project.
///
/// It implements a background worker thread that retrieves data from remote
/// API's and stores them in a database. It also runs an HTTP server for
/// getting that data back. It utilizes some global static variables for
/// moving data from the background thread to the main thread, for faster
/// response times.
///
/// I believe this is a good starter project because it covers a breadth of
/// topics, from thread management, to ownership/borrowing, hyper HTTP server,
/// serializing/deserializing structured data, etc.
///
/// Crates Used:
/// 1. Lazy Static - for initializing complex types as statics.
/// 2. Serde - for turning objects into JSON and vice versa.
/// 3. Diesel - for storing data in the database and retrieving it.
/// 4. Hyper - for HTTP server implementation.
/// 5. Chrono - for date and time operations.
#[tokio::main]
async fn main() {
    if cfg!(feature = "offline") {
        println!("Starting in offline mode...");
    }
    if check_env() && run_migrations() && start_fetching_backgrounds() {
        // todo: worker::start() maybe?
        // TODO: stop if you can't get initial readings
        start_worker();
        http::start().await;
    }
}

/// # Check Environment
/// Checks to see if all environment variables are set before starting
/// the server so that way we don't have some threads panic and some not.
/// Panics the main thread if any environment vars are missing.
fn check_env() -> bool {
    println!("Checking env...");
    dotenv().ok();
    env::var("LISTEN_PORT").expect("LISTEN_PORT must be set");
    env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::var("WEATHER_URL_DAILY").expect("WEATHER_URL_DAILY must be set");
    env::var("WEATHER_URL_HOURLY").expect("WEATHER_URL_HOURLY must be set");
    env::var("CORS_HOST").expect("CORS_HOST must be set");
    env::var("ECOBEE_CLIENT_ID").expect("ECOBEE_CLIENT_ID must be set");
    env::var("SHARED_ALBUM_ID").expect("SHARED_ALBUM_ID must be set");
    env::var("PHOTO_CACHE_DIR").expect("PHOTO_CACHE_DIR must be set");
    env::var("SHARED_SECRET").expect("SHARED_SECRET must be set");
    true
}

/// # Run Database Migrations
/// Updates the database with the latest table defintions. Returns `true`
/// if it worked and `false` if it failed.
fn run_migrations() -> bool {
    println!("Running migrations...");
    embed_migrations!();
    let connection = establish_connection();
    match embedded_migrations::run(&connection) {
        Ok(_) => true,
        Err(message) => {
            println!("Migrations failed! {:?}", message);
            false
        }
    }
}

fn start_fetching_backgrounds() -> bool {
    thread::spawn(|| {
        println!("Starting update of backgrounds...");
        let _ = image::scrape_webstream();
    });
    true
}

/// # Start Worker Thread
/// The worker thread is a background program that retrieves information
/// from Internet services every 5 minutes. It will put historical entries
/// in the database, and emits the current detail on a channel
// TODO: shorten thread::sleep duration to 30 seconds and check the time
fn start_worker() {
    thread::spawn(|| {
        let mut last_timestamp = Utc::now()
            .checked_sub_signed(ChronoDuration::seconds(1000))
            .unwrap();
        loop {
            let now = Utc::now();
            if now - last_timestamp > ChronoDuration::seconds(300) {
                last_timestamp = Utc::now();
                let mut therms: Vec<Thermostat> = Vec::new();
                // TODO: error handling, clean up var names
                if let Some(weather) = weather::current() {
                    therms.push(Thermostat::new(
                        String::from("weather.gov"),
                        weather.start_time,
                        weather.temperature,
                    ));
                }

                if let Some(forecast) = weather::forecast() {
                    let now_res = Arc::clone(&NOW_RES);
                    let mut now_res = now_res.write().unwrap();
                    *now_res = NowResponse {
                        forecast: forecast.conditions,
                        thermostats: now_res.thermostats.clone(),
                    };
                    drop(now_res);
                }

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
                let now_res = Arc::clone(&NOW_RES);
                let mut now_res = now_res.write().unwrap();
                *now_res = NowResponse {
                    forecast: now_res.forecast.clone(),
                    thermostats: therms,
                };
                drop(now_res);

                // Do a one-time JSON encoding; store result static
                let now_res = Arc::clone(&NOW_RES);
                let now_res = now_res.read().unwrap();
                let now_str = Arc::clone(&NOW_STR);
                let mut now_str = now_str.write().unwrap();
                *now_str = serde_json::to_string(&*now_res).unwrap();
                drop(now_res);
                drop(now_str);
            }

            thread::sleep(Duration::from_secs(4));
        }
    });
    println!("Worker thread spawned");
}

/// # Establish Connection
/// Returns a database connection from a connection string in an
/// environment variable. Diesel crate boilerplate code.
// TODO: Remove `except`, we don't want the software to panic if
//       there's a temporary problem with the database.
fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

/// # Parse JSON
/// Parses JSON into the type you want using serde.
pub fn parse<'de, T>(data: &'de str) -> anyhow::Result<T>
where
    T: serde::Deserialize<'de>,
{
    let result: Result<T, serde_json::error::Error> = serde_json::from_str(&data);
    match result {
        Ok(data) => Ok(data),
        Err(err) => {
            eprintln!("[ json ] JSON: {}", data);
            eprintln!("[ json ] JSON error: {}", err);
            Err(err.into())
        }
    }
}
