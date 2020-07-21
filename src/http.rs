use dotenv::dotenv;
use hyper::{Body, Request, Response, StatusCode};
use hyper::header::HeaderValue;
use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

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
                "/past" => past(),
                "/v" | "/version" => version(),
                "/release-notes" => release_notes(),
                _ => not_found(),
            };
            response.headers_mut().insert("Access-Control-Allow-Origin", HeaderValue::from_str(&cors_host).unwrap());
            Ok::<_, Infallible>(response)
        }))
    }));

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
/// based on query parameters. All query parameters are mandatory.
/// 
/// Sample query string: 
/// startDate=2020-03-01T00:00:00-05:00&endDate=2020-03-02T00:00:00-05:00
/// 
/// Returns a `Vec<Therm>` in a response body.
fn past() -> Response<Body> {
    Response::new(Body::from("Not implemented"))
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
        .body(Body::from("Not implemented!"))
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