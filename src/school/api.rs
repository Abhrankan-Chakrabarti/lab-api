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
    /// The database layer searches across all columns.
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

        DbError::Database(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "school database error".to_owned(),
        ),
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

/// Build the School API router.
///
/// All School API endpoints live below /school/api/.
pub fn router(state: SchoolState) -> Router {
    Router::new()
        .route("/school/api/health", get(health))
        .route("/school/api/tables", get(tables))
        // More specific route first for clarity.
        .route("/school/api/tables/:table/schema", get(schema))
        .route("/school/api/tables/:table", get(table_page))
        .with_state(state)
}
