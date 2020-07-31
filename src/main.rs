#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;

use diesel::prelude::*;
use diesel_migrations::*;
use dotenv::dotenv;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

mod ecobee;
// mod error;
mod http;
mod http_client;
mod therm;
mod weather;
mod schema;
use weather::{Condition, Forecast};
use therm::Thermostat;

static VERSION: u32 = 20200722;

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
type StaticThreadSafeString = Arc<RwLock<String>>;
type StaticThreadSafeTherms = Arc<RwLock<Vec<Thermostat>>>;
type StaticThreadSafeForecast = Arc<RwLock<Forecast>>;
lazy_static! {
    pub static ref NOW_RES: StaticThreadSafeString = Arc::new(RwLock::new(String::new()));
    pub static ref THERMS: StaticThreadSafeTherms = Arc::new(RwLock::new(vec![]));
    pub static ref FORECAST: StaticThreadSafeForecast = Arc::new(RwLock::new(Forecast::new()));
}

#[derive(Serialize)]
struct NowResponse {
    forecast: Vec<Condition>,
    thermostats: Vec<Thermostat>,
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
    if check_env() && run_migrations() {
        if is_offline() {
            println!("[ main ] Starting in offline mode!");
        }
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
        },
    }
}

/// # Start Worker Thread
/// The worker thread is a background program that retrieves information
/// from Internet services every 5 minutes. It will put historical entries
/// in the database, and emits the current detail on a channel
// TODO: shorten thread::sleep duration to 30 seconds and check the time
fn start_worker() {
    thread::spawn(|| {
        loop {
            let mut therms: Vec<Thermostat> = Vec::new();
            
            // TODO: error handling, clean up var names
            if let Some(weather) = weather::current(is_offline()) {
                therms.push(Thermostat::new(String::from("weather.gov"), weather.start_time.clone(), weather.temperature));
                println!("[worker] Got weather: {:?}", therms);
            }

            if let Some(forecast) = weather::forecast(is_offline()) {
                let static_forecast = Arc::clone(&FORECAST);
                let mut static_forecast = static_forecast.write().unwrap();
                *static_forecast = forecast;
                drop(static_forecast);
            }

            // Write thermostats to db
            // TODO: don't write duplicates :P
            let db = establish_connection();
            if let Some(token) = ecobee::current_token(&db) {
                for reading in ecobee::read(&token) {
                    therms.push(Thermostat::new2(reading.name, reading.time, reading.is_hygrostat, reading.temperature, reading.relative_humidity));
                }
            }

            for (_i, therm) in therms.iter().enumerate() {
                therm.insert(&db);
            }
            drop(db);

            let writable_therms = Arc::clone(&THERMS);
            let mut writable_therms = writable_therms.write().unwrap();
            *writable_therms = therms;
            drop(writable_therms);

            set_now_response();
            
            thread::sleep(Duration::from_secs(300));
        }
    });
    println!("[ main ] Worker thread spawned");
}

/// # Set Now Response
/// Sets the static NOW_RES from other static data points. Most of this method
/// is just locking and unlocking data points for thread safety, to make sure
/// we cause any memory corruption.
fn set_now_response() {
    let forecast = Arc::clone(&FORECAST);
    let forecast = forecast.read().unwrap();
    let therms = Arc::clone(&THERMS);
    let therms = therms.read().unwrap();
    let now_res = NowResponse {
        forecast: (*forecast).conditions.clone(),
        thermostats: (*therms).clone(),
    };
    drop(forecast);
    drop(therms);
    let writable_now = Arc::clone(&NOW_RES);
    let mut writable_now = writable_now.write().unwrap();
    *writable_now = serde_json::to_string(&now_res).unwrap().clone();
    drop(writable_now);
}

/// # Is Off-line
/// Returns if the server should connect to Internet-based services.
/// Set via "OFFLINE" environment variable. If offline, parts of the 
/// application that would connect to Internet-based services will
/// return stubbed data instead.
fn is_offline() -> bool {
    let offline = env::var("OFFLINE");
    if let Ok(offline) = offline {
        return offline == "yes";
    }
    false
}

/// # Establish Connection
/// Returns a database connection from a connection string in an
/// environment variable. Diesel crate boilerplate code.
// TODO: Remove `except`, we don't want the software to panic if
//       there's a temporary problem with the database.
fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
