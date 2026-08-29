use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::fs;
use std::net::SocketAddr;
use std::process::Command;

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct Snapshot {
    hostname: String,
    uptime: String,
    loadavg: String,
    mem_available_kb: Option<u64>,
}

#[derive(Serialize)]
struct Catalan {
    n: u64,
    value: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// C(0) = 1
///
/// C(n) = C(n-1) * 2 * (2n - 1) / (n + 1)
///
/// The result is stored in u128, so this demo intentionally
/// limits n to 34.
fn catalan(n: u64) -> Option<String> {
    if n > 34 {
        return None;
    }

    let mut c: u128 = 1;

    for i in 1..=n {
        let i = i as u128;

        c *= 2 * (2 * i - 1);
        c /= i + 1;
    }

    Some(c.to_string())
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "lab-api",
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
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".into())
        .trim()
        .to_string();

    let loadavg = fs::read_to_string("/proc/loadavg")
        .unwrap_or_else(|_| "unknown".into())
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");

    let mem_available_kb = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with("MemAvailable:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse::<u64>().ok())
        });

    Json(Snapshot {
        hostname,
        uptime,
        loadavg,
        mem_available_kb,
    })
}

async fn catalan_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > 34 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "n must be <= 34 for this demo (u128 limit)".into(),
            }),
        )
            .into_response();
    }

    match catalan(n) {
        Some(value) => Json(Catalan { n, value }).into_response(),

        None => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid n".into(),
            }),
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/snapshot", get(snapshot))
        .route("/v1/catalan/:n", get(catalan_handler));

    // Bind to localhost only.
    //
    // Nginx terminates TLS and proxies public /api/ requests
    // to this local service.
    let addr = SocketAddr::from(([127, 0, 0, 1], 8088));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind lab-api");

    eprintln!("lab-api listening on {addr}");

    axum::serve(listener, app)
        .await
        .expect("lab-api server failed");
}
