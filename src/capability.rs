use crate::error::{PgprofileError, Result};

pub struct Capabilities {
    pub pgss_present: bool,
    pub can_read_all_stats: bool,
    pub has_stats_since: bool,
    pub columns: Vec<String>,
}

pub fn detect(client: &mut postgres::Client) -> Result<Capabilities> {
    let pgss_present = check_pgss_present(client)?;
    if !pgss_present {
        return Ok(Capabilities {
            pgss_present: false,
            can_read_all_stats: false,
            has_stats_since: false,
            columns: vec![],
        });
    }

    let columns = introspect_columns(client)?;
    let can_read_all_stats = check_role(client, "pg_read_all_stats")?;
    let has_stats_since = columns.contains(&"stats_since".to_string());

    Ok(Capabilities {
        pgss_present: true,
        can_read_all_stats,
        has_stats_since,
        columns,
    })
}

pub fn check_pgss(client: &mut postgres::Client) -> Result<()> {
    if check_pgss_present(client)? {
        Ok(())
    } else {
        Err(PgprofileError::PgssNotInstalled)
    }
}

pub fn server_version(client: &mut postgres::Client) -> Result<String> {
    let row = client
        .query_one("SELECT version()", &[])
        .map_err(PgprofileError::Query)?;
    Ok(row.get::<_, String>(0))
}

fn check_pgss_present(client: &mut postgres::Client) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT count(*) FROM pg_extension WHERE extname = 'pg_stat_statements'",
            &[],
        )
        .map_err(PgprofileError::Query)?;
    let count: i64 = row.get(0);
    Ok(count > 0)
}

fn introspect_columns(client: &mut postgres::Client) -> Result<Vec<String>> {
    let rows = client
        .query(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_name = 'pg_stat_statements' \
             ORDER BY ordinal_position",
            &[],
        )
        .map_err(PgprofileError::Query)?;
    Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
}

fn check_role(client: &mut postgres::Client, role: &str) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT pg_has_role(current_user, $1, 'MEMBER')",
            &[&role],
        )
        .map_err(PgprofileError::Query)?;
    Ok(row.get::<_, bool>(0))
}
