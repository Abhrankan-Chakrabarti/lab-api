use rusqlite::{
    types::{ToSql, ValueRef},
    Connection, OpenFlags, Row,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

const MAX_PAGE_SIZE: usize = 100;
const MAX_SEARCH_LENGTH: usize = 100;
const MAX_OFFSET: usize = i64::MAX as usize;
const MAX_TABLE_NAME_LENGTH: usize = 128;

/// Columns that are safe to expose through the normal student listing API.
///
/// Everything else remains inside School.db and is never returned by
/// `SchoolDb::page()`.
///
/// These names intentionally match the CSV/database headers exactly.
const SAFE_COLUMNS: &[&str] = &[
    "Student Code",
    "Roll No",
    "Student Name",
    "Student DOB",
    "Academic Year",
];

#[derive(Clone)]
pub struct SchoolDb {
    path: Arc<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct TableList {
    pub tables: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ColumnMeta {
    pub name: String,
    pub type_name: String,
    pub notnull: bool,
    pub pk: bool,
}

#[derive(Debug, Serialize)]
pub struct TableSchema {
    pub table: String,
    pub columns: Vec<ColumnMeta>,
}

#[derive(Debug, Serialize)]
pub struct TablePage {
    pub table: String,
    pub columns: Vec<String>,
    pub rows: Vec<Map<String, Value>>,
    pub limit: usize,
    pub offset: usize,
    pub returned: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct StudentDetail {
    pub table: String,
    pub student: Map<String, Value>,
}

#[derive(Debug)]
pub enum DbError {
    InvalidTable,
    TableNotFound,
    StudentNotFound,
    Database(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTable => write!(f, "invalid table name"),
            Self::TableNotFound => write!(f, "table not found"),
            Self::StudentNotFound => write!(f, "student not found"),
            Self::Database(message) => write!(f, "database error: {message}"),
        }
    }
}

impl std::error::Error for DbError {}

impl SchoolDb {
    /// Open the database in read-only mode.
    ///
    /// The database file must already exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DbError> {
        let path = path.into();

        if !path.is_file() {
            return Err(DbError::Database(format!(
                "database file does not exist: {}",
                path.display()
            )));
        }

        // Verify that the database is actually readable.
        let _ = Self::connect(&path)?;

        Ok(Self {
            path: Arc::new(path),
        })
    }

    /// Create a new read-only SQLite connection.
    fn connect(path: &Path) -> Result<Connection, DbError> {
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| DbError::Database(error.to_string()))
    }

    fn conn(&self) -> Result<Connection, DbError> {
        Self::connect(&self.path)
    }

    /// List only real application tables.
    ///
    /// SQLite's internal sqlite_* tables are deliberately excluded.
    pub fn list_tables(&self) -> Result<TableList, DbError> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT name
                FROM sqlite_master
                WHERE type = 'table'
                  AND name NOT LIKE 'sqlite_%'
                ORDER BY name COLLATE NOCASE, name
                "#,
            )
            .map_err(|error| DbError::Database(error.to_string()))?;

        let tables = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| DbError::Database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DbError::Database(error.to_string()))?;

        Ok(TableList { tables })
    }

    /// Validate a user-supplied table identifier.
    ///
    /// Table names are still checked against sqlite_master. This validation
    /// exists in addition to identifier quoting, rather than relying on
    /// quoting alone.
    fn validate_table_name(table: &str) -> Result<(), DbError> {
        if table.is_empty() || table.len() > MAX_TABLE_NAME_LENGTH || table.as_bytes().contains(&0)
        {
            return Err(DbError::InvalidTable);
        }

        Ok(())
    }

    /// Verify that a table exists in the database.
    fn assert_table(&self, table: &str) -> Result<(), DbError> {
        Self::validate_table_name(table)?;

        let tables = self.list_tables()?;

        if !tables.tables.iter().any(|name| name == table) {
            return Err(DbError::TableNotFound);
        }

        Ok(())
    }

    /// Return the complete schema of a table dynamically using PRAGMA table_info.
    ///
    /// This is metadata only. No row values are returned here.
    pub fn schema(&self, table: &str) -> Result<TableSchema, DbError> {
        self.assert_table(table)?;

        let conn = self.conn()?;

        /*
         * PRAGMA table_info() does not support normal SQLite parameters
         * for the table identifier, so quote it safely.
         */
        let pragma_table = quote_ident(table);
        let sql = format!("PRAGMA table_info({pragma_table})");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| DbError::Database(error.to_string()))?;

        let columns = stmt
            .query_map([], |row| {
                Ok(ColumnMeta {
                    name: row.get::<_, String>(1)?,
                    type_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    notnull: row.get::<_, i64>(3)? != 0,
                    pk: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|error| DbError::Database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DbError::Database(error.to_string()))?;

        if columns.is_empty() {
            return Err(DbError::TableNotFound);
        }

        Ok(TableSchema {
            table: table.to_owned(),
            columns,
        })
    }

    /// Return only the safe columns that actually exist in the table.
    ///
    /// This allows the database schema to remain dynamic while preventing
    /// sensitive columns from being exposed by the normal listing API.
    fn safe_columns(&self, table: &str) -> Result<Vec<String>, DbError> {
        let schema = self.schema(table)?;

        let available: HashSet<&str> = schema
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();

        Ok(SAFE_COLUMNS
            .iter()
            .filter(|column| available.contains(**column))
            .map(|column| (*column).to_owned())
            .collect())
    }

    /// Fetch one paginated table page.
    ///
    /// Only SAFE_COLUMNS are returned.
    ///
    /// `limit` is capped at MAX_PAGE_SIZE.
    ///
    /// `search`, when supplied, performs a case-insensitive literal substring
    /// search across the safe columns only.
    pub fn page(
        &self,
        table: &str,
        limit: usize,
        offset: usize,
        search: Option<&str>,
    ) -> Result<TablePage, DbError> {
        self.assert_table(table)?;

        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        let offset = offset.min(MAX_OFFSET);

        let columns = self.safe_columns(table)?;

        if columns.is_empty() {
            return Ok(TablePage {
                table: table.to_owned(),
                columns,
                rows: Vec::new(),
                limit,
                offset,
                returned: 0,
                total: 0,
            });
        }

        let conn = self.conn()?;

        let quoted_columns: Vec<String> =
            columns.iter().map(|column| quote_ident(column)).collect();

        let select_list = quoted_columns.join(", ");
        let quoted_table = quote_ident(table);

        let trimmed_search = search
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_utf8(value, MAX_SEARCH_LENGTH));

        let (where_clause, search_pattern) = if let Some(query) = trimmed_search {
            let escaped_query = escape_like_pattern(query);
            let pattern = format!("%{escaped_query}%");

            let likes: Vec<String> = quoted_columns
                .iter()
                .map(|column| format!("CAST({column} AS TEXT) LIKE ?1 ESCAPE '\\'"))
                .collect();

            (Some(likes.join(" OR ")), Some(pattern))
        } else {
            (None, None)
        };

        // Count all rows matching the current search, independently of
        // LIMIT/OFFSET.
        let count_sql = match &where_clause {
            Some(where_clause) => {
                format!("SELECT COUNT(*) FROM {quoted_table} WHERE {where_clause}")
            }
            None => format!("SELECT COUNT(*) FROM {quoted_table}"),
        };

        let total_i64: i64 = match search_pattern.as_deref() {
            Some(pattern) => conn
                .query_row(&count_sql, [pattern], |row| row.get(0))
                .map_err(|err| DbError::Database(err.to_string()))?,
            None => conn
                .query_row(&count_sql, [], |row| row.get(0))
                .map_err(|err| DbError::Database(err.to_string()))?,
        };

        let total = usize::try_from(total_i64)
            .map_err(|_| DbError::Database("row count exceeds usize range".to_owned()))?;

        let order_column = if columns.iter().any(|column| column == "Student Code") {
            quote_ident("Student Code")
        } else {
            quoted_columns[0].clone()
        };

        let (sql, params): (String, Vec<Box<dyn ToSql>>) = if let Some(pattern) = search_pattern {
            let where_clause = where_clause
                .as_ref()
                .expect("search pattern must have a WHERE clause");

            let sql = format!(
                r#"
                    SELECT {select_list}
                    FROM {quoted_table}
                    WHERE {where_clause}
                    ORDER BY {order_column}
                    LIMIT ?2
                    OFFSET ?3
                "#
            );

            (
                sql,
                vec![
                    Box::new(pattern),
                    Box::new(limit as i64),
                    Box::new(offset as i64),
                ],
            )
        } else {
            let sql = format!(
                r#"
                    SELECT {select_list}
                    FROM {quoted_table}
                    ORDER BY {order_column}
                    LIMIT ?1
                    OFFSET ?2
                "#
            );

            (sql, vec![Box::new(limit as i64), Box::new(offset as i64)])
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|err| DbError::Database(err.to_string()))?;

        let param_refs: Vec<&dyn ToSql> = params.iter().map(|param| param.as_ref()).collect();

        let rows_iter = stmt
            .query_map(param_refs.as_slice(), |row| row_to_json(row, &columns))
            .map_err(|err| DbError::Database(err.to_string()))?;

        let mut rows = Vec::with_capacity(limit);

        for result in rows_iter {
            rows.push(result.map_err(|err| DbError::Database(err.to_string()))?);
        }

        let returned = rows.len();

        Ok(TablePage {
            table: table.to_owned(),
            columns,
            rows,
            limit,
            offset,
            returned,
            total,
        })
    }

    /// Fetch one student using the safe Student Code identifier.
    ///
    /// Only SAFE_COLUMNS are returned. No sensitive database columns can
    /// be exposed through this lookup.
    pub fn student_detail(&self, table: &str, student_code: i64) -> Result<StudentDetail, DbError> {
        self.assert_table(table)?;

        let columns = self.safe_columns(table)?;

        if columns.is_empty() {
            return Err(DbError::Database(
                "table has no safe student columns".to_owned(),
            ));
        }

        let student_code_column = "Student Code";

        if !columns.iter().any(|column| column == student_code_column) {
            return Err(DbError::Database(
                "table has no Student Code column".to_owned(),
            ));
        }

        let conn = self.conn()?;

        let quoted_columns: Vec<String> =
            columns.iter().map(|column| quote_ident(column)).collect();

        let select_list = quoted_columns.join(", ");
        let quoted_table = quote_ident(table);
        let quoted_student_code = quote_ident(student_code_column);

        let sql = format!(
            r#"
                SELECT {select_list}
                FROM {quoted_table}
                WHERE {quoted_student_code} = ?1
                LIMIT 1
            "#
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|error| DbError::Database(error.to_string()))?;

        let mut rows = stmt
            .query([student_code])
            .map_err(|error| DbError::Database(error.to_string()))?;

        match rows
            .next()
            .map_err(|error| DbError::Database(error.to_string()))?
        {
            Some(row) => {
                let student = row_to_json(row, &columns)
                    .map_err(|error| DbError::Database(error.to_string()))?;

                Ok(StudentDetail {
                    table: table.to_owned(),
                    student,
                })
            }
            None => Err(DbError::StudentNotFound),
        }
    }
}

/// Safely quote a SQLite identifier.
///
/// SQLite identifiers are escaped by doubling embedded double quotes:
///
///     abc"def -> "abc""def"
///
/// This is intentionally separate from SQL value parameters because
/// SQLite parameters cannot be used for table/column identifiers.
fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Safely truncate a UTF-8 string without splitting a code point.
///
/// The previous implementation used `&value[..100]`, which can panic when
/// byte 100 falls in the middle of a multi-byte UTF-8 character.
fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }

    let mut end = max_bytes;

    while !value.is_char_boundary(end) {
        end -= 1;
    }

    &value[..end]
}

/// Escape a value intended for a SQLite LIKE expression.
///
/// `%` and `_` are normally wildcards. Backslash is used as the explicit
/// escape character in the generated LIKE expression.
fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Convert one SQLite row into a JSON object using the dynamically
/// selected safe column names.
fn row_to_json(row: &Row<'_>, columns: &[String]) -> rusqlite::Result<Map<String, Value>> {
    let mut object = Map::with_capacity(columns.len());

    for (index, column) in columns.iter().enumerate() {
        let value = value_from_row(row, index)?;
        object.insert(column.clone(), value);
    }

    Ok(object)
}

/// Convert SQLite values into serde_json values.
fn value_from_row(row: &Row<'_>, index: usize) -> rusqlite::Result<Value> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(Value::Null),

        ValueRef::Integer(value) => Ok(Value::from(value)),

        ValueRef::Real(value) => {
            // JSON cannot represent NaN or Infinity.
            if value.is_finite() {
                Ok(Value::from(value))
            } else {
                Ok(Value::Null)
            }
        }

        ValueRef::Text(value) => Ok(Value::String(String::from_utf8_lossy(value).into_owned())),

        // Never expose arbitrary binary data through this API.
        ValueRef::Blob(_) => Ok(Value::String("[blob]".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rusqlite::Connection;
    use serde_json::Value;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_db_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "lab-api-school-db-test-{}-{}.db",
            std::process::id(),
            id
        ))
    }

    fn create_test_db(path: &Path) -> SchoolDb {
        let conn = Connection::open(path).expect("failed to create test database");

        conn.execute_batch(
            r#"
            CREATE TABLE II_A (
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

            INSERT INTO II_A (
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
            VALUES
                (1001, 1, 'Alice', '2018-01-15', '2026-27',
                 'Alice Father', 'Alice Mother', '9999999999',
                 9000000001, 9000000002, 'TEST000001',
                 '123456789012', 'Y'),

                (1002, 2, 'Bob', '2018-02-20', '2026-27',
                 'Bob Father', 'Bob Mother', '8888888888',
                 9000000003, 9000000004, 'TEST000002',
                 '987654321098', 'N'),

                (1003, 3, 'Élodie', '2018-03-25', '2026-27',
                 'Élodie Father', 'Élodie Mother', '7777777777',
                 9000000005, 9000000006, 'TEST000003',
                 '111122223333', 'Y'),

                (1004, 4, '100%_Student', '2018-04-30', '2026-27',
                 'Percent Father', 'Percent Mother', '6666666666',
                 9000000007, 9000000008, 'TEST000004',
                 '444455556666', 'N');
            "#,
        )
        .expect("failed to populate test database");

        drop(conn);

        SchoolDb::open(path).expect("failed to open SchoolDb")
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
    }

    fn row_values(page: &TablePage, column: &str) -> Vec<String> {
        page.rows
            .iter()
            .map(|row| match row.get(column) {
                Some(Value::String(value)) => value.clone(),
                Some(value) => value.to_string(),
                None => panic!("missing column {column:?}"),
            })
            .collect()
    }

    #[test]
    fn sensitive_columns_are_not_returned() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db.page("II_A", 100, 0, None).expect("page query failed");

        let sensitive = [
            "Father Name",
            "Mother Name",
            "Guardian Number",
            "Student Contact Number",
            "Guardian Contact Number",
            "Bank IFS Code",
            "Bank A/C number",
            "Aadhaar Y/N",
        ];

        for row in &page.rows {
            for column in sensitive {
                assert!(
                    !row.contains_key(column),
                    "sensitive column {column:?} leaked into API response"
                );
            }
        }

        assert!(page.columns.contains(&"Student Code".to_owned()));
        assert!(page.columns.contains(&"Roll No".to_owned()));
        assert!(page.columns.contains(&"Student Name".to_owned()));
        assert!(page.columns.contains(&"Student DOB".to_owned()));
        assert!(page.columns.contains(&"Academic Year".to_owned()));

        cleanup(&path);
    }

    #[test]
    fn unicode_search_does_not_panic_or_truncate_invalid_utf8() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db
            .page("II_A", 25, 0, Some("Élodie"))
            .expect("unicode search failed");

        assert_eq!(page.returned, 1);
        assert_eq!(row_values(&page, "Student Name"), vec!["Élodie".to_owned()]);

        cleanup(&path);
    }

    #[test]
    fn long_unicode_search_is_safely_truncated() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let search = format!("{}É", "x".repeat(MAX_SEARCH_LENGTH));

        let result = db.page("II_A", 25, 0, Some(&search));

        assert!(result.is_ok(), "long UTF-8 search should not panic or fail");

        cleanup(&path);
    }

    #[test]
    fn like_wildcards_are_treated_as_literal_text() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db
            .page("II_A", 25, 0, Some("100%_Student"))
            .expect("literal wildcard search failed");

        assert_eq!(page.returned, 1);
        assert_eq!(
            row_values(&page, "Student Name"),
            vec!["100%_Student".to_owned()]
        );

        cleanup(&path);
    }

    #[test]
    fn wildcard_search_does_not_match_every_row() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db
            .page("II_A", 25, 0, Some("%"))
            .expect("wildcard search failed");

        assert_eq!(
            page.returned, 1,
            "a literal '%' search should only match rows containing '%'"
        );

        assert_eq!(
            row_values(&page, "Student Name"),
            vec!["100%_Student".to_owned()]
        );

        cleanup(&path);
    }

    #[test]
    fn limit_is_capped_at_maximum_page_size() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db
            .page("II_A", usize::MAX, 0, None)
            .expect("bounded page query failed");

        assert!(
            page.returned <= MAX_PAGE_SIZE,
            "returned {} rows, exceeding MAX_PAGE_SIZE {}",
            page.returned,
            MAX_PAGE_SIZE
        );

        cleanup(&path);
    }

    #[test]
    fn huge_offset_is_bounded_safely() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let page = db
            .page("II_A", 25, usize::MAX, None)
            .expect("huge offset should be handled safely");

        assert_eq!(page.returned, 0);

        cleanup(&path);
    }

    #[test]
    fn pagination_is_deterministically_ordered() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let first = db
            .page("II_A", 2, 0, None)
            .expect("first page query failed");

        let second = db
            .page("II_A", 2, 0, None)
            .expect("second page query failed");

        assert_eq!(
            row_values(&first, "Student Code"),
            row_values(&second, "Student Code")
        );

        assert_eq!(
            row_values(&first, "Student Code"),
            vec!["1001".to_owned(), "1002".to_owned()]
        );

        cleanup(&path);
    }

    #[test]
    fn pages_do_not_overlap() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let first = db
            .page("II_A", 2, 0, None)
            .expect("first page query failed");

        let second = db
            .page("II_A", 2, 2, None)
            .expect("second page query failed");

        let first_codes = row_values(&first, "Student Code");
        let second_codes = row_values(&second, "Student Code");

        assert_eq!(first_codes, vec!["1001".to_owned(), "1002".to_owned()]);
        assert_eq!(second_codes, vec!["1003".to_owned(), "1004".to_owned()]);

        assert!(
            first_codes.iter().all(|code| !second_codes.contains(code)),
            "pagination pages must not overlap"
        );

        cleanup(&path);
    }

    #[test]
    fn invalid_table_names_are_rejected() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let empty = db.page("", 25, 0, None);
        assert!(matches!(empty, Err(DbError::InvalidTable)));

        let nul = db.page("bad\0table", 25, 0, None);
        assert!(matches!(nul, Err(DbError::InvalidTable)));

        let too_long = "x".repeat(MAX_TABLE_NAME_LENGTH + 1);
        let long_name = db.page(&too_long, 25, 0, None);
        assert!(matches!(long_name, Err(DbError::InvalidTable)));

        cleanup(&path);
    }

    #[test]
    fn sql_injection_style_table_name_is_not_accepted_as_a_real_table() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let malicious = r#"II_A"; DROP TABLE II_A; --"#;

        let result = db.page(malicious, 25, 0, None);

        assert!(matches!(
            result,
            Err(DbError::TableNotFound | DbError::InvalidTable)
        ));

        // Verify the real table still exists after the attempted attack.
        let page = db
            .page("II_A", 1, 0, None)
            .expect("real table should remain accessible");

        assert_eq!(page.returned, 1);

        cleanup(&path);
    }

    #[test]
    fn missing_table_returns_table_not_found() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let result = db.page("DOES_NOT_EXIST", 25, 0, None);

        assert!(matches!(result, Err(DbError::TableNotFound)));

        cleanup(&path);
    }

    #[test]
    fn schema_is_discovered_dynamically() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let schema = db.schema("II_A").expect("schema query failed");

        let names: Vec<_> = schema
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();

        assert!(names.contains(&"Student Code"));
        assert!(names.contains(&"Roll No"));
        assert!(names.contains(&"Student Name"));
        assert!(names.contains(&"Guardian Number"));
        assert!(names.contains(&"Bank A/C number"));
        assert!(names.contains(&"Aadhaar Y/N"));

        cleanup(&path);
    }

    #[test]
    fn tables_are_discovered_without_hardcoded_class_names() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let tables = db.list_tables().expect("table discovery failed");

        assert!(tables.tables.contains(&"II_A".to_owned()));
        assert!(
            !tables
                .tables
                .iter()
                .any(|table| table.starts_with("sqlite_")),
            "internal SQLite tables must not be exposed"
        );

        cleanup(&path);
    }

    #[test]
    fn total_reports_all_rows_independently_of_pagination() {
        let path = std::env::temp_dir().join("lab-api-test-total-pagination.db");
        let db = create_test_db(&path);

        let first_page = db.page("II_A", 2, 0, None).expect("first page should load");

        assert_eq!(first_page.returned, 2);
        assert_eq!(first_page.total, 4);

        let second_page = db
            .page("II_A", 2, 2, None)
            .expect("second page should load");

        assert_eq!(second_page.returned, 2);
        assert_eq!(second_page.total, 4);

        let empty_page = db
            .page("II_A", 2, 100, None)
            .expect("out-of-range page should load");

        assert_eq!(empty_page.returned, 0);
        assert_eq!(empty_page.total, 4);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn total_reports_matching_rows_for_search() {
        let path = std::env::temp_dir().join("lab-api-test-total-search.db");
        let db = create_test_db(&path);

        let page = db
            .page("II_A", 1, 0, Some("Alice"))
            .expect("search page should load");

        assert_eq!(page.returned, 1);
        assert_eq!(page.total, 1);

        let page = db
            .page("II_A", 1, 0, Some("2018"))
            .expect("date search page should load");

        assert_eq!(page.returned, 1);
        assert_eq!(page.total, 4);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn student_detail_returns_only_safe_columns() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let detail = db
            .student_detail("II_A", 1001)
            .expect("student detail query failed");

        assert_eq!(detail.table, "II_A");

        assert_eq!(detail.student.get("Student Code"), Some(&Value::from(1001)));
        assert_eq!(detail.student.get("Roll No"), Some(&Value::from(1)));
        assert_eq!(
            detail.student.get("Student Name"),
            Some(&Value::from("Alice"))
        );
        assert_eq!(
            detail.student.get("Student DOB"),
            Some(&Value::from("2018-01-15"))
        );
        assert_eq!(
            detail.student.get("Academic Year"),
            Some(&Value::from("2026-27"))
        );

        let sensitive = [
            "Father Name",
            "Mother Name",
            "Guardian Number",
            "Student Contact Number",
            "Guardian Contact Number",
            "Bank IFS Code",
            "Bank A/C number",
            "Aadhaar Y/N",
        ];

        for column in sensitive {
            assert!(
                !detail.student.contains_key(column),
                "sensitive column {column:?} leaked into student detail"
            );
        }

        cleanup(&path);
    }

    #[test]
    fn missing_student_returns_student_not_found() {
        let path = test_db_path();
        let db = create_test_db(&path);

        let result = db.student_detail("II_A", 9999);

        assert!(matches!(result, Err(DbError::StudentNotFound)));

        cleanup(&path);
    }
}
