use super::db::{DbError, SchoolDb};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared application state for the School API.
#[derive(Clone)]
pub struct SchoolState {
    pub db: Arc<SchoolDb>,
}

/// Query parameters for paginated table requests.
///
/// Example:
///
///     /school/api/tables/II_A?limit=25&offset=0
///
///     /school/api/tables/II_A?search=Abhrankan
#[derive(Debug, Deserialize)]
pub struct PageQuery {
    /// Number of records to return.
    ///
    /// Defaults to 25.
    /// The database layer caps this at 100.
    #[serde(default = "default_limit")]
    limit: usize,

    /// Number of records to skip.
    ///
    /// Defaults to 0.
    #[serde(default)]
    offset: usize,

    /// Optional search string.
    ///
    /// The database layer searches across the safe/display columns.
    search: Option<String>,
}

fn default_limit() -> usize {
    25
}

/// Standard API error response.
#[derive(Debug, Serialize)]
struct ErrBody {
    error: String,
}

/// Convert internal database errors into safe HTTP responses.
///
/// Database implementation details are deliberately not exposed to
/// the client. This is especially important because the School
/// database contains sensitive student information.
fn map_err(error: DbError) -> impl IntoResponse {
    let (status, message) = match error {
        DbError::InvalidTable => (StatusCode::BAD_REQUEST, "invalid table name".to_owned()),
        DbError::TableNotFound => (StatusCode::NOT_FOUND, "table not found".to_owned()),
        DbError::StudentNotFound => (StatusCode::NOT_FOUND, "student not found".to_owned()),
        DbError::Database(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    };

    (status, Json(ErrBody { error: message }))
}

/// School API health endpoint.
///
/// GET /school/api/health
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "scope": "school"
    }))
}

/// Return all available School database tables.
///
/// GET /school/api/tables
///
/// Example response:
///
/// {
///     "tables": [
///         "II_A",
///         "II_B",
///         "III_A"
///     ]
/// }
async fn tables(State(state): State<SchoolState>) -> impl IntoResponse {
    match state.db.list_tables() {
        Ok(result) => Json(result).into_response(),

        Err(error) => map_err(error).into_response(),
    }
}

/// Return the dynamically discovered schema of a table.
///
/// GET /school/api/tables/:table/schema
///
/// Example:
///
/// GET /school/api/tables/II_A/schema
async fn schema(State(state): State<SchoolState>, Path(table): Path<String>) -> impl IntoResponse {
    match state.db.schema(&table) {
        Ok(result) => Json(result).into_response(),

        Err(error) => map_err(error).into_response(),
    }
}

/// Return a paginated table.
///
/// GET /school/api/tables/:table
///
/// Supported query parameters:
///
///     limit
///     offset
///     search
///
/// The response includes `total`, which is the number of rows
/// matching the search before LIMIT/OFFSET pagination is applied.
///
/// Examples:
///
///     /school/api/tables/II_A
///     /school/api/tables/II_A?limit=25
///     /school/api/tables/II_A?limit=25&offset=25
///     /school/api/tables/II_A?search=Abhrankan
async fn table_page(
    State(state): State<SchoolState>,
    Path(table): Path<String>,
    Query(query): Query<PageQuery>,
) -> impl IntoResponse {
    match state
        .db
        .page(&table, query.limit, query.offset, query.search.as_deref())
    {
        Ok(result) => Json(result).into_response(),

        Err(error) => map_err(error).into_response(),
    }
}

/// Return one student using Student Code.
///
/// GET /school/api/tables/:table/students/:student_code
///
/// Only the safe/display columns returned by the database layer
/// are exposed.
async fn student_detail(
    State(state): State<SchoolState>,
    Path((table, student_code)): Path<(String, i64)>,
) -> impl IntoResponse {
    match state.db.student_detail(&table, student_code) {
        Ok(result) => Json(result).into_response(),
        Err(error) => map_err(error).into_response(),
    }
}

/// Build the School API router.
///
/// All School API endpoints live below /school/api/.
pub fn router(state: SchoolState) -> Router {
    Router::new()
        .route("/school/api/health", get(health))
        .route("/school/api/tables", get(tables))
        // More specific routes first for clarity.
        .route("/school/api/tables/:table/schema", get(schema))
        .route(
            "/school/api/tables/:table/students/:student_code",
            get(student_detail),
        )
        .route("/school/api/tables/:table", get(table_page))
        .with_state(state)
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

    fn test_db_path(test_name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "lab-api-school-api-test-{}-{}.db",
            std::process::id(),
            test_name
        ))
    }

    fn create_test_db(path: &std::path::Path) -> SchoolDb {
        let conn = rusqlite::Connection::open(path).expect("failed to create test database");

        conn.execute_batch(
            r#"
            CREATE TABLE "II_A" (
                "Student Code" INTEGER PRIMARY KEY,
                "Roll No" INTEGER,
                "Student Name" TEXT,
                "Student DOB" DATE,
                "Academic Year" TEXT,
                "Father Name" TEXT,
                "Mother Name" TEXT,
                "Guardian Number" TEXT,
                "Student Contact Number" INTEGER,
                "Guardian Contact Number" INTEGER,
                "Bank IFS Code" TEXT,
                "Bank A/C number" TEXT,
                "Aadhaar Y/N" TEXT
            );

            INSERT INTO "II_A" (
                "Student Code",
                "Roll No",
                "Student Name",
                "Student DOB",
                "Academic Year",
                "Father Name",
                "Mother Name",
                "Guardian Number",
                "Student Contact Number",
                "Guardian Contact Number",
                "Bank IFS Code",
                "Bank A/C number",
                "Aadhaar Y/N"
            )
            VALUES (
                1001,
                1,
                'Alice',
                '2018-01-15',
                '2026-27',
                'Sensitive Father',
                'Sensitive Mother',
                '9999999999',
                8888888888,
                7777777777,
                'SENSITIVEIFS',
                'SENSITIVEACCOUNT',
                'Y'
            );
            "#,
        )
        .expect("failed to initialize test database");

        drop(conn);

        SchoolDb::open(path).expect("failed to open SchoolDb")
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
    }

    async fn json_response(router: Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = router.oneshot(request).await.unwrap();

        let status = response.status();

        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();

        (status, serde_json::from_slice(&body).unwrap())
    }

    fn test_router(db: SchoolDb) -> Router {
        router(SchoolState { db: Arc::new(db) })
    }

    #[tokio::test]
    async fn student_detail_endpoint_returns_safe_student_data() {
        let path = test_db_path("safe-student");
        let db = create_test_db(&path);
        let router = test_router(db);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/tables/II_A/students/1001")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["table"], "II_A");

        assert_eq!(body["student"]["Student Code"], 1001);
        assert_eq!(body["student"]["Roll No"], 1);
        assert_eq!(body["student"]["Student Name"], "Alice");
        assert_eq!(body["student"]["Student DOB"], "2018-01-15");
        assert_eq!(body["student"]["Academic Year"], "2026-27");

        assert!(body["student"].get("Father Name").is_none());
        assert!(body["student"].get("Mother Name").is_none());
        assert!(body["student"].get("Guardian Number").is_none());
        assert!(body["student"].get("Student Contact Number").is_none());
        assert!(body["student"].get("Guardian Contact Number").is_none());
        assert!(body["student"].get("Bank IFS Code").is_none());
        assert!(body["student"].get("Bank A/C number").is_none());
        assert!(body["student"].get("Aadhaar Y/N").is_none());

        cleanup(&path);
    }

    #[tokio::test]
    async fn student_detail_endpoint_returns_not_found_for_missing_student() {
        let path = test_db_path("missing-student");
        let db = create_test_db(&path);
        let router = test_router(db);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/tables/II_A/students/9999")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "student not found");

        cleanup(&path);
    }
}
