use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::fs;
use std::net::SocketAddr;
use std::process::Command;
use tower_http::services::ServeDir;

mod factor;
mod prime;

use factor::{divisor_count, divisor_sum, factorise, mobius, totient, MAX_FACTOR_N};
use prime::{
    is_prime, next_prime, previous_prime, prime_gap, prime_pi, MAX_PRIME_PI_N, MAX_PRIME_SCAN_N,
};

mod school;

use school::api::{router as school_router, SchoolState};
use school::audit::SchoolAuditLog;
use school::db::SchoolDb;
use school::import::import_database;

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
struct PrimeResult {
    n: u64,
    prime: bool,
}

#[derive(Serialize)]
struct PrimeGapResult {
    n: u64,
    previous_prime: u64,
    next_prime: u64,
    gap: u64,
}

#[derive(Serialize)]
struct FactorResponse {
    n: u64,
    factors: Vec<FactorEntry>,
}

#[derive(Serialize)]
struct FactorEntry {
    prime: u64,
    power: u32,
}

#[derive(Serialize)]
struct MobiusResult {
    n: u64,
    value: i8,
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
            "GET /v1/math/is-prime/:n",
            "GET /v1/math/next-prime/:n",
            "GET /v1/math/previous-prime/:n",
            "GET /v1/math/prime-pi/:n",
            "GET /v1/math/pi/:n",
            "GET /v1/math/prime-gap/:n",
            "GET /v1/math/factor/:n",
            "GET /v1/math/totient/:n",
            "GET /v1/math/mobius/:n",
            "GET /v1/math/divisor-count/:n",
            "GET /v1/math/divisor-sum/:n",
            "GET /v1/catalan/:n",
            "GET /v1/snapshot",
            "GET /school/",
            "GET /school/api/health",
            "GET /school/api/tables",
            "GET /school/api/tables/:table",
            "GET /school/api/tables/:table/schema",
            "GET /school/api/tables/:table/students/:student_code",
            "GET /school/api/admin/tables/:table/students/:student_code",
            "GET /school/api/admin/audit",
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

/// C(0)=1; C(n)=C(n-1)*2*(2n-1)/(n+1).
/// u128 safe for n <= 34.
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

/// F(0)=0, F(1)=1.
/// u128 safe for n <= 186.
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

async fn is_prime_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_PRIME_SCAN_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_PRIME_SCAN_N} for this demo"),
        )
        .into_response();
    }

    Json(PrimeResult {
        n,
        prime: is_prime(n),
    })
    .into_response()
}

async fn next_prime_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_PRIME_SCAN_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_PRIME_SCAN_N} for this demo"),
        )
        .into_response();
    }

    match next_prime(n) {
        Some(p) => Json(MathU64 {
            n,
            value: p.to_string(),
        })
        .into_response(),
        None => err(
            StatusCode::BAD_REQUEST,
            "no larger prime exists in u64 range",
        )
        .into_response(),
    }
}

async fn previous_prime_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_PRIME_SCAN_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_PRIME_SCAN_N} for this demo"),
        )
        .into_response();
    }

    match previous_prime(n) {
        Some(p) => Json(MathU64 {
            n,
            value: p.to_string(),
        })
        .into_response(),
        None => err(StatusCode::BAD_REQUEST, "n must be greater than 2").into_response(),
    }
}

/// Prime-counting function π(n), not the constant π.
async fn prime_pi_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_PRIME_PI_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!(
                "n must be <= {MAX_PRIME_PI_N} for this demo (prime-counting π(n) sieve limit)"
            ),
        )
        .into_response();
    }

    Json(MathU64 {
        n,
        value: prime_pi(n).to_string(),
    })
    .into_response()
}

async fn prime_gap_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_PRIME_SCAN_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_PRIME_SCAN_N} for this demo"),
        )
        .into_response();
    }

    match prime_gap(n) {
        Some((previous, next, gap)) => Json(PrimeGapResult {
            n,
            previous_prime: previous,
            next_prime: next,
            gap,
        })
        .into_response(),
        None => err(StatusCode::BAD_REQUEST, "n must be greater than 2").into_response(),
    }
}

async fn factor_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_FACTOR_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_FACTOR_N} for this demo"),
        )
        .into_response();
    }

    let factors = factorise(n)
        .into_iter()
        .map(|factor| FactorEntry {
            prime: factor.prime,
            power: factor.power,
        })
        .collect();

    Json(FactorResponse { n, factors }).into_response()
}

async fn totient_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_FACTOR_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_FACTOR_N} for this demo"),
        )
        .into_response();
    }

    Json(MathU64 {
        n,
        value: totient(n).to_string(),
    })
    .into_response()
}

async fn mobius_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_FACTOR_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_FACTOR_N} for this demo"),
        )
        .into_response();
    }

    Json(MobiusResult {
        n,
        value: mobius(n),
    })
    .into_response()
}

async fn divisor_count_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_FACTOR_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_FACTOR_N} for this demo"),
        )
        .into_response();
    }

    Json(MathU64 {
        n,
        value: divisor_count(n).to_string(),
    })
    .into_response()
}

async fn divisor_sum_handler(Path(n): Path<u64>) -> impl IntoResponse {
    if n > MAX_FACTOR_N {
        return err(
            StatusCode::BAD_REQUEST,
            format!("n must be <= {MAX_FACTOR_N} for this demo"),
        )
        .into_response();
    }

    Json(MathU64 {
        n,
        value: divisor_sum(n).to_string(),
    })
    .into_response()
}

/// Build the existing public API router.
fn api_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/info", get(info))
        .route("/v1/snapshot", get(snapshot))
        // REST-style math resources.
        .route("/v1/math/catalan/:n", get(catalan_handler))
        .route("/v1/math/fibonacci/:n", get(fibonacci_handler))
        .route("/v1/math/gcd/:a/:b", get(gcd_handler))
        .route("/v1/math/is-prime/:n", get(is_prime_handler))
        .route("/v1/math/next-prime/:n", get(next_prime_handler))
        .route("/v1/math/previous-prime/:n", get(previous_prime_handler))
        .route("/v1/math/prime-pi/:n", get(prime_pi_handler))
        .route("/v1/math/pi/:n", get(prime_pi_handler)) // alias; document as π(n)
        .route("/v1/math/prime-gap/:n", get(prime_gap_handler))
        .route("/v1/math/factor/:n", get(factor_handler))
        .route("/v1/math/totient/:n", get(totient_handler))
        .route("/v1/math/mobius/:n", get(mobius_handler))
        .route("/v1/math/divisor-count/:n", get(divisor_count_handler))
        .route("/v1/math/divisor-sum/:n", get(divisor_sum_handler))
        // Backward-compatible alias.
        .route("/v1/catalan/:n", get(catalan_handler))
}

/// Build the School database portal.
///
/// The database is opened read-only. If SCHOOL_DB_PATH is not configured
/// or the database cannot be opened, the School API is disabled while
/// the existing lab-api continues to operate normally.
fn school_routes() -> Router {
    let database_path = match std::env::var("SCHOOL_DB_PATH") {
        Ok(path) if !path.trim().is_empty() => path,

        Ok(_) => {
            eprintln!("SCHOOL_DB_PATH is empty; school routes disabled");
            return Router::new();
        }

        Err(_) => {
            eprintln!("SCHOOL_DB_PATH not set; school routes disabled");
            return Router::new();
        }
    };

    match SchoolDb::open(&database_path) {
        Ok(db) => {
            eprintln!("school database: {database_path}");

            school_router(SchoolState::new(db, school_audit_log()))
        }

        Err(error) => {
            eprintln!("school database disabled: {error}");
            Router::new()
        }
    }
}

fn school_audit_log() -> Option<SchoolAuditLog> {
    let audit_path = match std::env::var("SCHOOL_AUDIT_DB_PATH") {
        Ok(path) if !path.trim().is_empty() => path,
        Ok(_) => {
            eprintln!("SCHOOL_AUDIT_DB_PATH is empty; admin student detail is disabled");
            return None;
        }
        Err(_) => {
            eprintln!("SCHOOL_AUDIT_DB_PATH not set; admin student detail is disabled");
            return None;
        }
    };

    match SchoolAuditLog::open(&audit_path) {
        Ok(audit_log) => {
            eprintln!("school audit database: {audit_path}");
            Some(audit_log)
        }
        Err(error) => {
            eprintln!("school audit logging unavailable: {error}");
            None
        }
    }
}

/// Build the complete application.
///
/// Route precedence:
///
///     /school/api/*   -> School API
///     /school/*       -> School static files
///     /api/*          -> existing API routes
///
/// The School database itself is never served as a static file.
fn app() -> Router {
    let school_api = school_routes();

    let school_static = Router::new().nest_service(
        "/school",
        ServeDir::new("static/school").append_index_html_on_directories(true),
    );

    api_router().merge(school_api).merge(school_static)
}

/// Run the one-shot School database importer.
///
/// The destination comes from SCHOOL_DB_PATH, exactly like normal server
/// mode. The running HTTP API is not started in this mode.
fn run_import(candidate: &str) -> Result<(), String> {
    let destination =
        std::env::var("SCHOOL_DB_PATH").map_err(|_| "SCHOOL_DB_PATH is not set".to_owned())?;

    if destination.trim().is_empty() {
        return Err("SCHOOL_DB_PATH is empty".to_owned());
    }

    eprintln!("lab-api {APP_VERSION} database import");
    eprintln!("candidate: {candidate}");
    eprintln!("destination: {destination}");

    let result = import_database(candidate, &destination).map_err(|error| error.to_string())?;

    eprintln!("database import successful");
    eprintln!("activated: {}", result.destination.display());
    eprintln!("tables: {}", result.tables.join(", "));

    Ok(())
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args();
    let _program = args.next();

    match args.next().as_deref() {
        Some("import") => {
            let Some(candidate) = args.next() else {
                eprintln!("usage: lab-api import <candidate.db>");
                std::process::exit(2);
            };

            if args.next().is_some() {
                eprintln!("usage: lab-api import <candidate.db>");
                std::process::exit(2);
            }

            if let Err(error) = run_import(&candidate) {
                eprintln!("database import failed: {error}");
                std::process::exit(1);
            }

            return;
        }

        Some(command) => {
            eprintln!("unknown command: {command}");
            eprintln!("usage:");
            eprintln!("  lab-api");
            eprintln!("  lab-api import <candidate.db>");
            std::process::exit(2);
        }

        None => {}
    }

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
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
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

    #[test]
    fn prime_utilities_work() {
        assert!(is_prime(2));
        assert!(is_prime(97));

        assert!(!is_prime(1));
        assert!(!is_prime(100));

        assert_eq!(next_prime(100), Some(101));
        assert_eq!(prime_pi(100), 25);
    }

    #[test]
    fn test_is_prime() {
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(is_prime(97));

        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(!is_prime(100));
    }

    #[test]
    fn test_next_prime() {
        assert_eq!(next_prime(100), Some(101));
        assert_eq!(next_prime(0), Some(2));
    }

    #[test]
    fn test_next_prime_overflow() {
        assert_eq!(next_prime(u64::MAX), None);
    }

    #[test]
    fn test_prime_pi() {
        assert_eq!(prime_pi(10), 4);
        assert_eq!(prime_pi(100), 25);
        assert_eq!(prime_pi(1000), 168);
    }

    #[tokio::test]
    async fn info_advertises_math_routes_and_alias() {
        let (status, body) =
            json_response(Request::get("/v1/info").body(Body::empty()).unwrap()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["api_version"], "v1");
        assert_eq!(body["app_version"], env!("CARGO_PKG_VERSION"));

        assert_eq!(
            body["build_profile"],
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );

        assert_eq!(
            body["environment"],
            option_env!("LAB_API_ENV").unwrap_or("unknown")
        );

        let endpoints = body["endpoints"].as_array().unwrap();

        assert!(endpoints.iter().any(|route| route == "GET /v1/catalan/:n"));

        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/fibonacci/:n"));

        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/gcd/:a/:b"));

        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/factor/:n"));

        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/previous-prime/:n"));

        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/totient/:n"));
        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/mobius/:n"));
        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/divisor-count/:n"));
        assert!(endpoints
            .iter()
            .any(|route| route == "GET /v1/math/divisor-sum/:n"));

        assert!(endpoints.iter().any(|route| route == "GET /school/"));
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
    async fn prime_routes_return_expected_values() {
        let (status, body) = json_response(
            Request::get("/v1/math/is-prime/97")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["prime"], true);

        let (status, body) = json_response(
            Request::get("/v1/math/next-prime/100")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "101");

        let (status, body) = json_response(
            Request::get("/v1/math/previous-prime/100")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "97");

        let (status, body) =
            json_response(Request::get("/v1/math/pi/100").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "25");

        let (status, body) = json_response(
            Request::get("/v1/math/prime-gap/1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["previous_prime"], 997);
        assert_eq!(body["next_prime"], 1009);
        assert_eq!(body["gap"], 12);
    }

    #[tokio::test]
    async fn factor_route_returns_expected_values() {
        let (status, body) = json_response(
            Request::get("/v1/math/factor/360")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 360);
        assert_eq!(body["factors"][0]["prime"], 2);
        assert_eq!(body["factors"][0]["power"], 3);
        assert_eq!(body["factors"][1]["prime"], 3);
        assert_eq!(body["factors"][1]["power"], 2);
        assert_eq!(body["factors"][2]["prime"], 5);
        assert_eq!(body["factors"][2]["power"], 1);
    }

    #[tokio::test]
    async fn factor_route_rejects_over_limit() {
        let over = MAX_FACTOR_N + 1;
        let (status, body) = json_response(
            Request::get(format!("/v1/math/factor/{over}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("1000000"));
    }

    #[tokio::test]
    async fn multiplicative_routes_return_expected_values() {
        let (status, body) = json_response(
            Request::get("/v1/math/totient/36")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 36);
        assert_eq!(body["value"], "12");

        let (status, body) = json_response(
            Request::get("/v1/math/mobius/30")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 30);
        assert_eq!(body["value"], -1);

        let (status, body) = json_response(
            Request::get("/v1/math/mobius/36")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], 0);

        let (status, body) = json_response(
            Request::get("/v1/math/divisor-count/360")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 360);
        assert_eq!(body["value"], "24");

        let (status, body) = json_response(
            Request::get("/v1/math/divisor-sum/360")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["n"], 360);
        assert_eq!(body["value"], "1170");
    }

    #[tokio::test]
    async fn multiplicative_routes_reject_over_limit() {
        let over = MAX_FACTOR_N + 1;
        for route in ["totient", "mobius", "divisor-count", "divisor-sum"] {
            let (status, body) = json_response(
                Request::get(format!("/v1/math/{route}/{over}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(body["error"].as_str().unwrap().contains("1000000"));
        }
    }

    #[tokio::test]
    async fn prime_pi_rejects_over_limit() {
        let over = MAX_PRIME_PI_N + 1;
        let (status, body) = json_response(
            Request::get(format!("/v1/math/pi/{over}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("1000000"));
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

        let (status, body) = json_response(
            Request::get("/v1/math/previous-prime/2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("greater than 2"));

        let (status, body) = json_response(
            Request::get("/v1/math/previous-prime/1000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("1000000"));
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

    #[tokio::test]
    async fn school_api_is_disabled_without_database_configuration() {
        std::env::remove_var("SCHOOL_DB_PATH");

        let response = app()
            .oneshot(
                Request::get("/school/api/tables")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
