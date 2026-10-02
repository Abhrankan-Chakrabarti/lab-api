use super::{
    audit::{AuditEvent, SchoolAuditLog},
    db::{DbError, SchoolDb},
};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

/// Shared application state for the School API.
#[derive(Clone)]
pub struct SchoolState {
    pub db: Arc<SchoolDb>,
    pub admin_users: Arc<HashSet<String>>,
    pub audit_log: Option<Arc<SchoolAuditLog>>,
}

impl SchoolState {
    pub fn new(db: SchoolDb, audit_log: Option<SchoolAuditLog>) -> Self {
        Self {
            db: Arc::new(db),
            admin_users: Arc::new(admin_users_from_env()),
            audit_log: audit_log.map(Arc::new),
        }
    }
}

fn admin_users_from_env() -> HashSet<String> {
    std::env::var("LAB_API_ADMIN_USERS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|user| !user.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn is_admin(user: &str, admin_users: &HashSet<String>) -> bool {
    !user.is_empty() && admin_users.contains(user)
}

fn authenticated_admin(headers: &HeaderMap, state: &SchoolState) -> Result<String, Response> {
    let user = headers
        .get("X-Authenticated-User")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if is_admin(user, &state.admin_users) {
        Ok(user.to_owned())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrBody {
                error: "admin access required".to_owned(),
            }),
        )
            .into_response())
    }
}

fn audit_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrBody {
            error: "audit logging is unavailable".to_owned(),
        }),
    )
        .into_response()
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

#[derive(Debug, Deserialize)]
struct AuditQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Serialize)]
struct AuditResponse {
    events: Vec<AuditEvent>,
    limit: usize,
    offset: usize,
}

const DEFAULT_AUDIT_PAGE_SIZE: usize = 50;
const MAX_AUDIT_PAGE_SIZE: usize = 100;

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

/// Return one student with every schema column.
///
/// GET /school/api/admin/tables/:table/students/:student_code
///
/// This endpoint requires an authenticated Nginx username in
/// `X-Authenticated-User` and that username must be listed in
/// `LAB_API_ADMIN_USERS`.
async fn student_detail_full(
    headers: HeaderMap,
    State(state): State<SchoolState>,
    Path((table, student_code)): Path<(String, i64)>,
) -> impl IntoResponse {
    let user = match authenticated_admin(&headers, &state) {
        Ok(user) => user,
        Err(response) => return response,
    };

    let Some(audit_log) = state.audit_log.as_deref() else {
        return audit_unavailable();
    };

    let student = match state.db.student_detail_full(&table, student_code) {
        Ok(result) => result,
        Err(error) => return map_err(error).into_response(),
    };

    if audit_log
        .record_admin_student_view(&user, &table, student_code)
        .is_err()
    {
        eprintln!("failed to write School admin audit event");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrBody {
                error: "audit logging failed; student details were not returned".to_owned(),
            }),
        )
            .into_response();
    }

    Json(student).into_response()
}

async fn admin_audit(
    headers: HeaderMap,
    State(state): State<SchoolState>,
    Query(query): Query<AuditQuery>,
) -> impl IntoResponse {
    if let Err(response) = authenticated_admin(&headers, &state) {
        return response;
    }

    let Some(audit_log) = state.audit_log.as_deref() else {
        return audit_unavailable();
    };

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AUDIT_PAGE_SIZE)
        .clamp(1, MAX_AUDIT_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0);

    match audit_log.recent_events(limit, offset) {
        Ok(events) => Json(AuditResponse {
            events,
            limit,
            offset,
        })
        .into_response(),
        Err(_) => {
            eprintln!("failed to read School admin audit log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrBody {
                    error: "audit log could not be read".to_owned(),
                }),
            )
                .into_response()
        }
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
        .route(
            "/school/api/admin/tables/:table/students/:student_code",
            get(student_detail_full),
        )
        .route("/school/api/admin/audit", get(admin_audit))
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

    fn test_audit_path(test_name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "lab-api-school-audit-test-{}-{}.db",
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

    fn test_router(db: SchoolDb, audit_path: &std::path::Path) -> Router {
        let audit_log = SchoolAuditLog::open(audit_path).expect("failed to open test audit log");
        router(SchoolState {
            db: Arc::new(db),
            admin_users: Arc::new(["abhrankan".to_owned()].into_iter().collect()),
            audit_log: Some(Arc::new(audit_log)),
        })
    }

    #[tokio::test]
    async fn student_detail_endpoint_returns_safe_student_data() {
        let path = test_db_path("safe-student");
        let audit_path = test_audit_path("safe-student");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

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

        assert!(body["student"].get("Father Name").is_some());
        assert!(body["student"].get("Mother Name").is_some());

        assert!(body["student"].get("Guardian Number").is_none());
        assert!(body["student"].get("Student Contact Number").is_none());
        assert!(body["student"].get("Guardian Contact Number").is_none());
        assert!(body["student"].get("Bank IFS Code").is_none());
        assert!(body["student"].get("Bank A/C number").is_none());
        assert!(body["student"].get("Aadhaar Y/N").is_none());

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn admin_student_detail_returns_sensitive_fields() {
        let path = test_db_path("admin-student");
        let audit_path = test_audit_path("admin-student");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/tables/II_A/students/1001")
                .header("X-Authenticated-User", "abhrankan")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["student"]["Bank A/C number"], "SENSITIVEACCOUNT");
        assert_eq!(body["student"]["Guardian Number"], "9999999999");
        assert_eq!(body["student"]["Aadhaar Y/N"], "Y");

        let audit_log = SchoolAuditLog::open(&audit_path).unwrap();
        let events = audit_log.recent_events(10, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].timestamp.ends_with('Z'));
        assert_eq!(events[0].user, "abhrankan");
        assert_eq!(events[0].table, "II_A");
        assert_eq!(events[0].student_code, "1001");
        assert_eq!(events[0].action, "admin_student_view");

        let audit_router = test_router(SchoolDb::open(&path).unwrap(), &audit_path);
        let (audit_status, audit_body) = json_response(
            audit_router,
            Request::get("/school/api/admin/audit")
                .header("X-Authenticated-User", "abhrankan")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(audit_status, StatusCode::OK);
        assert_eq!(audit_body["events"][0]["user"], "abhrankan");
        assert_eq!(audit_body["events"][0]["table"], "II_A");
        assert_eq!(audit_body["events"][0]["student_code"], "1001");
        assert_eq!(audit_body["events"][0]["action"], "admin_student_view");

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn non_admin_gets_403() {
        let path = test_db_path("non-admin");
        let audit_path = test_audit_path("non-admin");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/tables/II_A/students/1001")
                .header("X-Authenticated-User", "teacher1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "admin access required");
        assert!(SchoolAuditLog::open(&audit_path)
            .unwrap()
            .recent_events(10, 0)
            .unwrap()
            .is_empty());

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn missing_header_gets_403() {
        let path = test_db_path("missing-admin-header");
        let audit_path = test_audit_path("missing-admin-header");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/tables/II_A/students/1001")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "admin access required");

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn admin_student_detail_missing_student_returns_not_found() {
        let path = test_db_path("admin-missing-student");
        let audit_path = test_audit_path("admin-missing-student");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/tables/II_A/students/9999")
                .header("X-Authenticated-User", "abhrankan")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "student not found");
        assert!(SchoolAuditLog::open(&audit_path)
            .unwrap()
            .recent_events(10, 0)
            .unwrap()
            .is_empty());

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn student_detail_endpoint_returns_not_found_for_missing_student() {
        let path = test_db_path("missing-student");
        let audit_path = test_audit_path("missing-student");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

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
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn non_admin_cannot_read_audit_log() {
        let path = test_db_path("non-admin-audit-read");
        let audit_path = test_audit_path("non-admin-audit-read");
        let db = create_test_db(&path);
        let audit_log = SchoolAuditLog::open(&audit_path).unwrap();
        audit_log
            .record_admin_student_view("abhrankan", "II_A", 1001)
            .unwrap();
        let router = test_router(db, &audit_path);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/audit")
                .header("X-Authenticated-User", "teacher1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "admin access required");
        assert!(body.get("events").is_none());

        cleanup(&path);
        cleanup(&audit_path);
    }

    #[tokio::test]
    async fn audit_write_failure_does_not_return_student_record() {
        let path = test_db_path("audit-write-failure");
        let audit_path = test_audit_path("audit-write-failure");
        let db = create_test_db(&path);
        let router = test_router(db, &audit_path);

        let connection = rusqlite::Connection::open(&audit_path).unwrap();
        connection
            .execute_batch(
                "CREATE TRIGGER reject_school_audit
                 BEFORE INSERT ON school_admin_audit
                 BEGIN
                     SELECT RAISE(FAIL, 'injected audit failure');
                 END;",
            )
            .unwrap();
        drop(connection);

        let (status, body) = json_response(
            router,
            Request::get("/school/api/admin/tables/II_A/students/1001")
                .header("X-Authenticated-User", "abhrankan")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            body["error"],
            "audit logging failed; student details were not returned"
        );
        assert!(body.get("student").is_none());
        assert!(!body.to_string().contains("SENSITIVEACCOUNT"));
        assert!(SchoolAuditLog::open(&audit_path)
            .unwrap()
            .recent_events(10, 0)
            .unwrap()
            .is_empty());

        cleanup(&path);
        cleanup(&audit_path);
    }
}
