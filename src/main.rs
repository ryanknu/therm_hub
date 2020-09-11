#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel_migrations::*;
use dotenv::dotenv;
use lazy_static::lazy_static;
use serde::Serialize;
use std::env;
use std::sync::{Arc, RwLock};
use std::thread;

mod ecobee;
mod http;
mod image;
mod schema;
mod therm;
mod weather;
mod worker;
use therm::Thermostat;
use weather::{DailyCondition, HourlyCondition};

static VERSION: u32 = 20200822;

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
type StsBool = Arc<RwLock<bool>>;
lazy_static! {
    static ref FETCHING_PHOTOS: StsBool = Arc::new(RwLock::new(false));
    pub static ref NOW_STR: StsString = Arc::new(RwLock::new(String::new()));
    static ref NOW_RES: StsNowResponse = Arc::new(RwLock::new(NowResponse::default()));
    pub static ref REQWEST: reqwest::Client = reqwest::Client::new();
}

#[derive(Serialize)]
struct NowResponse {
    forecast_daily: Vec<DailyCondition>,
    forecast_hourly: Vec<HourlyCondition>,
    thermostats: Vec<Thermostat>,
}

impl Default for NowResponse {
    fn default() -> Self {
        Self {
            forecast_daily: vec![],
            forecast_hourly: vec![],
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
fn main() {
    if cfg!(feature = "offline") {
        log_message("Starting in offline mode...");
    }
    if check_env() && run_migrations() && start_fetching_backgrounds() && worker::check() {
        worker::start();
        http::start();
    }
}

/// # Check Environment
/// Checks to see if all environment variables are set before starting
/// the server so that way we don't have some threads panic and some not.
/// Panics the main thread if any environment vars are missing.
fn check_env() -> bool {
    log_message("Checking env...");
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
    log_message("Running migrations...");
    embed_migrations!();
    let connection = establish_connection();
    match embedded_migrations::run(&connection) {
        Ok(_) => true,
        Err(message) => {
            log_error(&format!("Migrations failed! {:?}", message));
            false
        }
    }
}

/// # Start Fetching Backgrounds
/// Creates a thread that will populate the backgrounds directory in the
/// background.
fn start_fetching_backgrounds() -> bool {
    thread::spawn(|| {
        log_message("Starting update of backgrounds...");
        let fetching_photos = Arc::clone(&FETCHING_PHOTOS);
        let fetching_photos = fetching_photos.write();
        if let Ok(mut fetching_photos) = fetching_photos {
            if *fetching_photos {
                log_message("Already fetching photos; stopped");
            } else {
                *fetching_photos = true;
                match image::scrape_webstream() {
                    Ok(_) => log_message("Completed update of backgrounds"),
                    Err(err) => log_message(&format!("Error updating backgrounds: {:?}", err)),
                };
                // TODO: this could be made faster (but more complex) by dropping the lock before scrape_webstream() and then re-establishing the lock here
                *fetching_photos = false;
            }
        }
    });
    true
}

/// # Establish Connection
/// Returns a database connection from a connection string in an
/// environment variable. Diesel crate boilerplate code.
fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    for _ in 1..5 {
        let connection_result = PgConnection::establish(&database_url);
        if let Ok(connection) = connection_result {
            return connection;
        } else if let Err(err) = connection_result {
            log_error(&format!("{:?}", err));
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    panic!("Error connecting to {}", database_url);
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
            log_error(data);
            log_error(&format!("{}", err));
            Err(err.into())
        }
    }
}

/// # Log Message
/// Logs a message to the journal.
pub fn log_message(message: &str) {
    println!("msg [{}] {}", chrono::Utc::now(), message);
}

/// # Log Error
/// Just a convenience method to make all error logs go through one method. This makes all log
/// entries have the date prepended to them.
pub fn log_error(message: &str) {
    eprintln!("err [{}] {}", chrono::Utc::now(), message);
}
