use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::fs;
use std::net::SocketAddr;
use std::process::Command;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const API_VERSION: &str = "v1";

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct Info {
    service: &'static str,
    api_version: &'static str,
    app_version: &'static str,
    endpoints: Vec<&'static str>,
    build_profile: &'static str,
    environment: &'static str,
}

#[derive(Serialize)]
struct Snapshot {
    hostname: String,
    uptime: String,
    loadavg: String,
    mem_available_kb: Option<u64>,
}

#[derive(Serialize)]
struct MathU64 {
    n: u64,
    value: String,
}

#[derive(Serialize)]
struct GcdResult {
    a: u64,
    b: u64,
    gcd: u64,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn err(status: StatusCode, msg: impl Into<String>) -> impl IntoResponse {
    (status, Json(ErrorBody { error: msg.into() }))
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "lab-api",
    })
}

async fn info() -> Json<Info> {
    Json(Info {
        service: "lab-api",
        api_version: API_VERSION,
        app_version: APP_VERSION,
        endpoints: vec![
            "GET /health",
            "GET /v1/info",
            "GET /v1/math/catalan/:n",
            "GET /v1/math/fibonacci/:n",
            "GET /v1/math/gcd/:a/:b",
            "GET /v1/catalan/:n",
            "GET /v1/snapshot",
        ],
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        environment: option_env!("LAB_API_ENV").unwrap_or("unknown"),
    })
}

async fn snapshot() -> impl IntoResponse {
    let hostname = fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "unknown".into())
        .trim()
        .to_string();

    let uptime = Command::new("uptime")
        .arg("-p")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into())
        .trim()
        .to_string();

    let loadavg = fs::read_to_string("/proc/loadavg")
        .unwrap_or_else(|_| "unknown".into())
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");

    let mem_available_kb = fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("MemAvailable:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|n| n.parse().ok())
    });

    Json(Snapshot {
        hostname,
        uptime,
        loadavg,
        mem_available_kb,
    })
}

/// C(0)=1; C(n)=C(n-1)*2*(2n-1)/(n+1). u128 safe for n <= 34.
fn catalan(n: u64) -> Option<u128> {
    if n > 34 {
        return None;
    }
    let mut c: u128 = 1;
    for i in 1..=n {
        c = c * 2 * (2 * i as u128 - 1) / (i as u128 + 1);
    }
    Some(c)
}

async fn catalan_handler(Path(n): Path<u64>) -> impl IntoResponse {
    match catalan(n) {
        Some(value) => Json(MathU64 {
            n,
            value: value.to_string(),
        })
        .into_response(),
        None => err(
            StatusCode::BAD_REQUEST,
            "n must be <= 34 for this demo (u128 limit)",
        )
        .into_response(),
    }
}

/// F(0)=0, F(1)=1. u128 safe for n <= 186.
fn fibonacci(n: u64) -> Option<u128> {
    if n > 186 {
        return None;
    }
    if n == 0 {
        return Some(0);
    }
    let mut a: u128 = 0;
    let mut b: u128 = 1;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    Some(if n == 1 { 1 } else { b })
}

async fn fibonacci_handler(Path(n): Path<u64>) -> impl IntoResponse {
    match fibonacci(n) {
        Some(value) => Json(MathU64 {
            n,
            value: value.to_string(),
        })
        .into_response(),
        None => err(
            StatusCode::BAD_REQUEST,
            "n must be <= 186 for this demo (u128 limit)",
        )
        .into_response(),
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

async fn gcd_handler(Path((a, b)): Path<(u64, u64)>) -> Json<GcdResult> {
    Json(GcdResult {
        a,
        b,
        gcd: gcd(a, b),
    })
}

fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/info", get(info))
        .route("/v1/snapshot", get(snapshot))
        // Prefer nested math resources (REST-ish)
        .route("/v1/math/catalan/:n", get(catalan_handler))
        .route("/v1/math/fibonacci/:n", get(fibonacci_handler))
        .route("/v1/math/gcd/:a/:b", get(gcd_handler))
        // Backward-compatible alias
        .route("/v1/catalan/:n", get(catalan_handler))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8088));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind 127.0.0.1:8088");
    eprintln!("lab-api {APP_VERSION} listening on {addr}");
    axum::serve(listener, app()).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, body::Body, http::Request};
    use serde_json::Value;
    use tower::ServiceExt;

    async fn json_response(request: Request<Body>) -> (StatusCode, Value) {
        let response = app().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();

        (status, serde_json::from_slice(&body).unwrap())
    }

    #[test]
    fn catalan_respects_u128_boundary() {
        assert_eq!(catalan(0), Some(1));
        assert_eq!(catalan(34), Some(812944042149730764));
        assert_eq!(catalan(35), None);
    }

    #[test]
    fn fibonacci_respects_u128_boundary() {
        assert_eq!(fibonacci(0), Some(0));
        assert_eq!(fibonacci(1), Some(1));
        assert_eq!(
            fibonacci(186),
            Some(332825110087067562321196029789634457848)
        );
        assert_eq!(fibonacci(187), None);
    }

    #[test]
    fn gcd_handles_zero_and_common_factors() {
        assert_eq!(gcd(0, 0), 0);
        assert_eq!(gcd(0, 42), 42);
        assert_eq!(gcd(84, 30), 6);
    }

    #[tokio::test]
    async fn info_advertises_math_routes_and_alias() {
        let (status, body) =
            json_response(Request::get("/v1/info").body(Body::empty()).unwrap()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["api_version"], "v1");
        assert_eq!(body["app_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(body["build_profile"], "debug");
        assert_eq!(body["environment"], "unknown");
        let endpoints = body["endpoints"].as_array().unwrap();
        assert!(endpoints.iter().any(|route| route == "GET /v1/catalan/:n"));
        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/fibonacci/:n"));
        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/gcd/:a/:b"));
    }

    #[tokio::test]
    async fn math_routes_return_expected_values() {
        let (status, body) = json_response(
            Request::get("/v1/math/catalan/10")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "16796");

        let (status, body) =
            json_response(Request::get("/v1/catalan/10").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "16796");

        let (status, body) = json_response(
            Request::get("/v1/math/fibonacci/10")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "55");

        let (status, body) = json_response(
            Request::get("/v1/math/gcd/84/30")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["gcd"], 6);
    }

    #[tokio::test]
    async fn math_routes_reject_out_of_range_inputs() {
        let (status, body) = json_response(
            Request::get("/v1/math/catalan/35")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("34"));

        let (status, body) = json_response(
            Request::get("/v1/math/fibonacci/187")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("186"));
    }

    #[tokio::test]
    async fn unknown_route_returns_not_found() {
        let response = app()
            .oneshot(
                Request::get("/v1/math/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
