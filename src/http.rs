use crate::ecobee::{get_token, install, save_token, GRANT_PIN};
use crate::Thermostat;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use hyper::header::HeaderValue;
use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, StatusCode};
use serde::Deserialize;
use std::convert::Infallible;
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Deserialize)]
struct InstallTwoInput {
    code: String,
}

#[derive(Debug, Deserialize)]
struct PastInput {
    end_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
}

/// # Start Server
/// Starts the hyper HTTP server. Also contains the routing code.
pub async fn start() {
    dotenv().ok();
    let port: u16 = env::var("LISTEN_PORT").unwrap().parse().unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let server = Server::bind(&addr);
    let server = server.serve(make_service_fn(|_connection| async {
        Ok::<_, Infallible>(service_fn(|req: Request<Body>| async move {
            println!("[ hyper] Incoming: {}", req.uri().path());
            let cors_host = env::var("CORS_HOST").unwrap();
            let mut response = match req.uri().path() {
                "/now" => now(),
                "/past" => past(req),
                "/time" => time(),
                "/v" | "/version" => version(),
                "/release-notes" => release_notes(),
                "/install/1" => install_1(req).await,
                "/install/2" => install_2(req).await,
                _ => not_found(),
            };
            response.headers_mut().insert(
                "Access-Control-Allow-Origin",
                HeaderValue::from_str(&cors_host).unwrap(),
            );
            Ok::<_, Infallible>(response)
        }))
    }));

    println!("[ main ] hyper thread started");

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

/// # Now Handler
/// Returns the current conditions. It does it by reading the static
/// now response from the crate root and copying it into a request body.
///
/// Returns a `NowRepsonse` in a response body.
fn now() -> Response<Body> {
    let now = Arc::clone(&crate::NOW_RES);
    let now = now.read().unwrap();
    Response::new(Body::from((*now).clone()))
}

/// # Past Handler
/// Returns a past historical report. This queries data from the database
/// based on query parameters. All query parameters are mandatory and must
/// be sent in alphabetical order.
///
/// Sample query string:
/// end-date=2020-03-02T00:00:00-05:00&start-date=2020-03-01T00:00:00-05:00
///
/// Returns a `Vec<Therm>` in a response body.
fn past(req: Request<Body>) -> Response<Body> {
    let query: Option<PastInput> = query_parameters(&req);
    match query {
        None => bad_request(),
        Some(input) => {
            let connection = crate::establish_connection();
            let result = Thermostat::query_dates(&connection, &input.start_date, &input.end_date);
            drop(connection);
            match result {
                Err(_) => internal_server_error(),
                Ok(result) => match serde_json::to_string(&result) {
                    Err(_) => internal_server_error(),
                    Ok(body) => Response::new(Body::from(body)),
                },
            }
        }
    }
}

/// # Time
/// Returns the API system time for setting the time on devices that do
/// not have an RTC. It's intended use is for the user to compare the
/// device's time to the server's time, and store an offset value that is
/// factored into all time calculations from that point forward.
fn time() -> Response<Body> {
    Response::builder()
        .header("Content-Type", "text/plain")
        .body(Body::from(Utc::now().timestamp().to_string()))
        .unwrap()
}

/// # Version
/// Returns the current version of the ThermHub software.
///
/// Returns a `VersionWrapper` in the response body.
fn version() -> Response<Body> {
    Response::builder()
        .header("Content-Type", "text/plain")
        .body(Body::from(format!("{}", crate::VERSION)))
        .unwrap()
}

fn release_notes() -> Response<Body> {
    Response::builder()
        .header("Content-Type", "text/markdown")
        .body(Body::from(fs::read_to_string("release-notes.md").unwrap()))
        .unwrap()
}

async fn install_1(req: Request<Body>) -> Response<Body> {
    match *req.method() {
        Method::GET => match install().await {
            Ok(install_response) => match serde_json::to_string(&install_response) {
                Ok(data) => Response::new(Body::from(data)),
                Err(_) => internal_server_error(),
            },
            Err(_) => internal_server_error(),
        },
        _ => method_not_allowed(),
    }
}

async fn install_2(req: Request<Body>) -> Response<Body> {
    if !Method::GET.eq(req.method()) {
        return method_not_allowed();
    }
    let code: Option<InstallTwoInput> = query_parameters(&req);
    match code {
        None => bad_request(),
        Some(code) => {
            let response = get_token(&code.code, GRANT_PIN).await;
            println!("response: {:?}", response);
            match response {
                Err(_) => internal_server_error(),
                Ok(token_response) => {
                    let token = token_response.to_token();
                    let db = crate::establish_connection();
                    let token = save_token(&token, &db);
                    drop(db);
                    match token {
                        None => Response::new(Body::from("false")),
                        Some(_) => Response::new(Body::from("true")),
                    }
                }
            }
        }
    }
}

/// # Query Parameters
/// Turns an HTTP request into a struct containing the query parameters
fn query_parameters<'de, T, V>(req: &'de Request<V>) -> Option<T>
where
    T: Deserialize<'de>,
{
    let query: &str = match req.uri().query() {
        Some(query) => query,
        None => "",
    };
    let parsed: Result<T, _> = serde_urlencoded::from_str(&query);
    match parsed {
        Ok(parsed) => Some(parsed),
        Err(err) => {
            eprintln!("[ hyper] could not decode query params {:?}", err);
            None
        }
    }
}

/// # Method not allowed
/// Returns a response payload that indicates 405 method not allowed.
fn method_not_allowed() -> Response<Body> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(Body::from("405 Method Not Allowed"))
        .unwrap()
}

/// # Bad Request
/// Returns a response payload that indicates a 400 bad request.
fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from("400 Bad Request"))
        .unwrap()
}

/// # Not Found
/// Returns a response payload that indicates a 404 not found.
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("404 Not found"))
        .unwrap()
}

/// # Internal Server Serror
/// Returns a response payload that indicates a 500 internal server error.
fn internal_server_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("500 Internal Server Error"))
        .unwrap()
}
