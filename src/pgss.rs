use crate::capability::Capabilities;
use crate::error::{PgprofileError, Result};

#[derive(Debug, Clone)]
pub struct Row {
    pub userid: i64,
    pub dbid: i64,
    pub queryid: i64,
    pub toplevel: bool,
    pub total_exec_time: f64,
    pub calls: i64,
    pub rows: i64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
    pub temp_blks_read: i64,
    pub temp_blks_written: i64,
    pub wal_bytes: Option<i64>,
}

pub fn snapshot(client: &mut postgres::Client, caps: &Capabilities) -> Result<Vec<Row>> {
    let sql = build_query(caps);
    let rows = client
        .query(&sql, &[])
        .map_err(PgprofileError::Query)?;

    Ok(rows
        .iter()
        .map(|r| {
            let wal_bytes: Option<i64> = if caps.columns.contains(&"wal_bytes".to_string()) {
                r.get("wal_bytes")
            } else {
                None
            };
            Row {
                userid: r.get::<_, i64>("userid"),
                dbid: r.get::<_, i64>("dbid"),
                queryid: r.get::<_, i64>("queryid"),
                toplevel: r.get::<_, bool>("toplevel"),
                total_exec_time: r.get::<_, f64>("total_exec_time"),
                calls: r.get::<_, i64>("calls"),
                rows: r.get::<_, i64>("rows"),
                shared_blks_hit: r.get::<_, i64>("shared_blks_hit"),
                shared_blks_read: r.get::<_, i64>("shared_blks_read"),
                temp_blks_read: r.get::<_, i64>("temp_blks_read"),
                temp_blks_written: r.get::<_, i64>("temp_blks_written"),
                wal_bytes,
            }
        })
        .collect())
}

pub fn fetch_query_text(client: &mut postgres::Client, queryid: i64) -> Result<Option<String>> {
    let rows = client
        .query(
            "SELECT query FROM pg_stat_statements(showtext := true) WHERE queryid = $1 LIMIT 1",
            &[&queryid],
        )
        .map_err(PgprofileError::Query)?;
    Ok(rows.into_iter().next().map(|r| r.get::<_, String>(0)))
}

fn build_query(caps: &Capabilities) -> String {
    // Use PG17+ renamed columns (shared_blk_read_time, etc.) if present; fall back
    let exec_time_col = if caps.columns.contains(&"total_exec_time".to_string()) {
        "total_exec_time"
    } else {
        "total_time AS total_exec_time"
    };

    let wal_col = if caps.columns.contains(&"wal_bytes".to_string()) {
        ", wal_bytes"
    } else {
        ""
    };

    format!(
        "SELECT userid::bigint, dbid::bigint, queryid::bigint, toplevel, \
         {exec_time_col}, calls::bigint, rows::bigint, \
         shared_blks_hit::bigint, shared_blks_read::bigint, \
         temp_blks_read::bigint, temp_blks_written::bigint{wal_col} \
         FROM pg_stat_statements(showtext := false)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_with_pgss() {
        let caps = Capabilities {
            pgss_present: true,
            can_read_all_stats: false,
            has_stats_since: false,
            columns: vec![
                "userid".to_string(), "dbid".to_string(), "queryid".to_string(),
                "toplevel".to_string(), "total_exec_time".to_string(),
                "calls".to_string(), "rows".to_string(),
                "shared_blks_hit".to_string(), "shared_blks_read".to_string(),
                "temp_blks_read".to_string(), "temp_blks_written".to_string(),
                "wal_bytes".to_string(),
            ],
        };
        let q = build_query(&caps);
        assert!(q.contains("total_exec_time"));
        assert!(q.contains("wal_bytes"));
        assert!(q.contains("pg_stat_statements(showtext := false)"));
    }

    #[test]
    fn test_build_query_without_wal_bytes() {
        let caps = Capabilities {
            pgss_present: true,
            can_read_all_stats: false,
            has_stats_since: false,
            columns: vec![
                "userid".to_string(), "dbid".to_string(), "queryid".to_string(),
                "total_exec_time".to_string(), "calls".to_string(), "rows".to_string(),
                "shared_blks_hit".to_string(), "shared_blks_read".to_string(),
                "temp_blks_read".to_string(), "temp_blks_written".to_string(),
            ],
        };
        let q = build_query(&caps);
        assert!(!q.contains("wal_bytes"));
    }
}
