use rusqlite::{params, Connection};
use serde::Serialize;
use std::{path::PathBuf, sync::Arc, time::Duration};

const MAX_AUDIT_PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct SchoolAuditLog {
    path: Arc<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub user: String,
    pub table: String,
    pub student_code: String,
    pub action: String,
}

impl SchoolAuditLog {
    pub fn open(path: impl Into<PathBuf>) -> rusqlite::Result<Self> {
        let audit_log = Self {
            path: Arc::new(path.into()),
        };

        let connection = audit_log.connect()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS school_admin_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                username TEXT NOT NULL,
                table_name TEXT NOT NULL,
                student_code TEXT NOT NULL,
                action TEXT NOT NULL CHECK (action = 'admin_student_view')
            );",
        )?;

        Ok(audit_log)
    }

    fn connect(&self) -> rusqlite::Result<Connection> {
        let connection = Connection::open(&*self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        Ok(connection)
    }

    pub fn record_admin_student_view(
        &self,
        username: &str,
        table: &str,
        student_code: i64,
    ) -> rusqlite::Result<()> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO school_admin_audit
                (timestamp, username, table_name, student_code, action)
             VALUES (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?1, ?2, ?3, 'admin_student_view')",
            params![username, table, student_code.to_string()],
        )?;

        Ok(())
    }

    pub fn recent_events(&self, limit: usize, offset: usize) -> rusqlite::Result<Vec<AuditEvent>> {
        let connection = self.connect()?;
        let limit = limit.clamp(1, MAX_AUDIT_PAGE_SIZE) as i64;
        let offset = offset.min(i64::MAX as usize) as i64;
        let mut statement = connection.prepare(
            "SELECT timestamp, username, table_name, student_code, action
             FROM school_admin_audit
             ORDER BY id DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = statement.query_map(params![limit, offset], |row| {
            Ok(AuditEvent {
                timestamp: row.get(0)?,
                user: row.get(1)?,
                table: row.get(2)?,
                student_code: row.get(3)?,
                action: row.get(4)?,
            })
        })?;

        rows.collect()
    }
}
