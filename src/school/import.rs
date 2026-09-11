use super::db::{DbError, SchoolDb};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum ImportError {
    Io(String),
    Database(String),
}

impl From<io::Error> for ImportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<DbError> for ImportError {
    fn from(error: DbError) -> Self {
        let message = match error {
            DbError::InvalidTable => "invalid table name".to_owned(),
            DbError::TableNotFound => "table not found".to_owned(),
            DbError::StudentNotFound => "student not found".to_owned(),
            DbError::Database(message) => message,
        };

        Self::Database(message)
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "I/O error: {message}"),
            Self::Database(message) => write!(formatter, "database error: {message}"),
        }
    }
}

impl std::error::Error for ImportError {}

#[derive(Debug)]
pub struct ImportResult {
    pub destination: PathBuf,
    pub tables: Vec<String>,
}

/// Validate and safely activate a candidate School database.
///
/// The candidate is copied into the destination directory first, validated,
/// and only then renamed over the active database.
///
/// The existing database is never removed before the candidate has passed
/// validation.
pub fn import_database(
    candidate: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<ImportResult, ImportError> {
    let candidate = candidate.as_ref();
    let destination = destination.as_ref();

    if !candidate.is_file() {
        return Err(ImportError::Io(format!(
            "candidate database does not exist: {}",
            candidate.display()
        )));
    }

    let parent = destination.parent().ok_or_else(|| {
        ImportError::Io(format!(
            "destination has no parent directory: {}",
            destination.display()
        ))
    })?;

    fs::create_dir_all(parent)?;

    let staging = staging_path(destination);

    if staging.exists() {
        fs::remove_file(&staging)?;
    }

    // Copy first. The active database is untouched.
    fs::copy(candidate, &staging)?;

    let validation = match SchoolDb::validate(&staging) {
        Ok(validation) => validation,
        Err(error) => {
            let _ = fs::remove_file(&staging);
            return Err(ImportError::from(error));
        }
    };

    activate_staging(&staging, destination)?;

    Ok(ImportResult {
        destination: destination.to_path_buf(),
        tables: validation.tables,
    })
}

fn staging_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("School.db");

    let process_id = std::process::id();

    destination.with_file_name(format!(".{file_name}.{process_id}.importing"))
}

fn activate_staging(staging: &Path, destination: &Path) -> Result<(), ImportError> {
    #[cfg(unix)]
    {
        fs::rename(staging, destination)?;
        Ok(())
    }

    #[cfg(windows)]
    {
        let backup = destination.with_file_name(format!(
            ".{}.backup",
            destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("School.db")
        ));

        if backup.exists() {
            fs::remove_file(&backup)?;
        }

        if destination.exists() {
            fs::rename(destination, &backup)?;
        }

        match fs::rename(staging, destination) {
            Ok(()) => {
                let _ = fs::remove_file(&backup);
                Ok(())
            }
            Err(error) => {
                if backup.exists() && !destination.exists() {
                    let _ = fs::rename(&backup, destination);
                }

                Err(ImportError::Io(error.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rusqlite::Connection;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);

        std::env::temp_dir().join(format!(
            "lab-api-school-import-test-{}-{}",
            std::process::id(),
            id
        ))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn create_valid_database(path: &Path) {
        let conn = Connection::open(path).expect("failed to create database");

        conn.execute_batch(
            r#"
            CREATE TABLE II_A (
                "Student Code" INTEGER PRIMARY KEY,
                "Student Name" TEXT,
                "Father Name" TEXT
            );

            INSERT INTO II_A ("Student Code", "Student Name", "Father Name")
            VALUES (1001, 'Alice', 'Alice Father');
            "#,
        )
        .expect("failed to populate database");

        drop(conn);
    }

    #[test]
    fn valid_candidate_is_activated() {
        let directory = test_dir();
        fs::create_dir_all(&directory).expect("failed to create test directory");

        let candidate = directory.join("candidate.db");
        let destination = directory.join("School.db");

        create_valid_database(&candidate);

        let result = import_database(&candidate, &destination).expect("import should succeed");

        assert_eq!(result.destination, destination);
        assert_eq!(result.tables, vec!["II_A"]);
        assert!(destination.is_file());
        assert!(!staging_path(&destination).exists());

        let validation =
            SchoolDb::validate(&destination).expect("activated database should be valid");

        assert_eq!(validation.tables, vec!["II_A"]);

        cleanup(&directory);
    }

    #[test]
    fn missing_candidate_is_rejected() {
        let directory = test_dir();
        fs::create_dir_all(&directory).expect("failed to create test directory");

        let candidate = directory.join("missing.db");
        let destination = directory.join("School.db");

        let result = import_database(&candidate, &destination);

        assert!(matches!(result, Err(ImportError::Io(message))
            if message.contains("candidate database does not exist")));

        assert!(!destination.exists());

        cleanup(&directory);
    }

    #[test]
    fn malformed_candidate_is_rejected() {
        let directory = test_dir();
        fs::create_dir_all(&directory).expect("failed to create test directory");

        let candidate = directory.join("candidate.db");
        let destination = directory.join("School.db");

        fs::write(&candidate, b"not a sqlite database")
            .expect("failed to create malformed candidate");

        let result = import_database(&candidate, &destination);

        assert!(result.is_err());
        assert!(!destination.exists());
        assert!(!staging_path(&destination).exists());

        cleanup(&directory);
    }

    #[test]
    fn invalid_candidate_does_not_replace_existing_database() {
        let directory = test_dir();
        fs::create_dir_all(&directory).expect("failed to create test directory");

        let candidate = directory.join("candidate.db");
        let destination = directory.join("School.db");

        create_valid_database(&destination);

        fs::write(&candidate, b"not a sqlite database")
            .expect("failed to create invalid candidate");

        let result = import_database(&candidate, &destination);

        assert!(result.is_err());

        let validation =
            SchoolDb::validate(&destination).expect("existing database must remain valid");

        assert_eq!(validation.tables, vec!["II_A"]);
        assert!(!staging_path(&destination).exists());

        cleanup(&directory);
    }

    #[test]
    fn candidate_without_student_identity_is_rejected() {
        let directory = test_dir();
        fs::create_dir_all(&directory).expect("failed to create test directory");

        let candidate = directory.join("candidate.db");
        let destination = directory.join("School.db");

        let conn = Connection::open(&candidate).expect("failed to create database");

        conn.execute_batch(
            r#"
            CREATE TABLE II_A (
                "Roll No" INTEGER,
                "Student Name" TEXT
            );
            "#,
        )
        .expect("failed to create invalid schema");

        drop(conn);

        let result = import_database(&candidate, &destination);

        assert!(result.is_err());
        assert!(!destination.exists());
        assert!(!staging_path(&destination).exists());

        cleanup(&directory);
    }
}
