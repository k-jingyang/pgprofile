use std::fmt;

#[derive(Debug)]
pub enum PgprofileError {
    Connection(postgres::Error),
    Query(postgres::Error),
    PgssNotInstalled,
    InvalidDuration(String),
}

impl fmt::Display for PgprofileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PgprofileError::Connection(e) => write!(f, "connection failed: {e}"),
            PgprofileError::Query(e) => write!(f, "query failed: {e}"),
            PgprofileError::PgssNotInstalled => write!(
                f,
                "pg_stat_statements is not installed or not in search_path; \
                 ensure the extension is created: CREATE EXTENSION IF NOT EXISTS pg_stat_statements"
            ),
            PgprofileError::InvalidDuration(s) => write!(f, "invalid duration: {s}"),
        }
    }
}

impl From<postgres::Error> for PgprofileError {
    fn from(e: postgres::Error) -> Self {
        PgprofileError::Connection(e)
    }
}

pub type Result<T> = std::result::Result<T, PgprofileError>;
