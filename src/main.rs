use axum::{extract::Path, response::IntoResponse, routing::get, Json, Router};
use chrono::DateTime;
use hyper::Method;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST]);

    let router = Router::new()
        .route("/", get(root))
        .route("/api", get(now_handler))
        .route("/api/:date", get(date_handler))
        .layer(cors);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    // let addr = SocketAddr::from(([127, 0, 0, 1], 8200));
    // tracing::debug!("listening on {}", addr);
    // axum::Server::bind(&addr)
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();

    Ok(router.into())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

//handler that receive a date and return a json with the unix and utc date
#[derive(Deserialize, Serialize)]
struct Date {
    date: String,
}

#[derive(Deserialize, Serialize)]
struct Response {
    unix: i64,
    utc: String,
}

// handle empty api requests and return the current date
async fn now_handler() -> impl IntoResponse {
    let date = chrono::Utc::now();
    let unix = date.timestamp_millis();
    let utc = date
        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();

    Json(Response { unix, utc })
}

// handle api requests with a date string and return the unix and utc date
async fn date_handler(Path(date): Path<String>) -> impl IntoResponse {
    match parse_date_or_timestamp(date) {
        Ok(res) => Ok(Json(res)),
        Err(_) => Err(r#"{error : "Invalid Date"}"#),
    }
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
