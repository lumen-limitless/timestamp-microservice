use std::net::SocketAddr;

use axum::{extract::Path, response::IntoResponse, routing::get, Json, Router};
use chrono::DateTime;
use hyper::Method;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

#[derive(Deserialize, Serialize)]
struct Date {
    date: String,
}

#[derive(Deserialize, Serialize)]
struct Response {
    unix: i64,
    utc: String,
}

#[derive(Deserialize, Serialize)]
struct Error {
    error: String,
}

// handle api requests without a date string and return the current unix and utc date
async fn now_handler() -> impl IntoResponse {
    let date = chrono::Utc::now();
    let unix = date.timestamp_millis();
    let utc = date
        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();

    Json(Response { unix, utc })
}

fn parse_date_or_timestamp(date: String) -> anyhow::Result<Response> {
    match date.parse::<i64>() {
        Ok(secs) => {
            let date = DateTime::<chrono::Utc>::from_timestamp(secs / 1000, 0).unwrap();
            let utc = date
                .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string();
            Ok(Response { unix: secs, utc })
        }
        Err(_) => {
            let date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S%.f"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S%.fZ"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S%.f%:z"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%d %B %Y, %Z"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%a %b %d %Y %H:%M:%S GMT%z"))
                .or_else(|_| chrono::NaiveDate::parse_from_str(&date, "%a %b %d %Y %H:%M:%S %z"))
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(&date, "%a %b %d %Y %H:%M:%S GMT%:z (%Z)")
                })
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(&date, "%a %b %d %Y %H:%M:%S %:z (%Z)")
                });
            match date {
                Ok(date) => {
                    let date = date.and_hms_opt(0, 0, 0).unwrap();
                    let secs = date.and_utc().timestamp_millis();
                    let utc = date
                        .and_utc()
                        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                        .format("%a, %d %b %Y %H:%M:%S GMT")
                        .to_string();
                    Ok(Response { unix: secs, utc })
                }
                Err(_) => Err(anyhow::Error::msg("Invalid Date")),
            }
        }
    }
}

// handle api requests with a date string and return the unix and utc date
async fn date_handler(Path(date): Path<String>) -> impl IntoResponse {
    match parse_date_or_timestamp(date) {
        Ok(res) => Ok(Json(res)),
        Err(_) => Err(Json(Error {
            error: "Invalid Date".to_string(),
        })),
    }
}

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_methods(vec![Method::GET])
        .allow_origin(Any);

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/api", get(now_handler))
        .route("/api/:date", get(date_handler))
        .layer(cors);

    // get the port to listen on
    let port = std::env::var("PORT").unwrap_or("8200".to_string());

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse::<u16>().unwrap()));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}
