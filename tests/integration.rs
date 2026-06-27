//! Integration tests — skipped when DATABASE_URL is not set or pgss is absent.

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

#[test]
fn test_detect_capabilities_requires_pgss() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    // If pgss is absent, skip (not fail) — some test clusters don't have it.
    if !caps.pgss_present {
        eprintln!("WARN: pg_stat_statements not found — skipping pgss-dependent tests");
        return;
    }
    assert!(!caps.columns.is_empty(), "should detect at least some columns");
    assert!(caps.columns.contains(&"queryid".to_string()));
    assert!(caps.columns.contains(&"calls".to_string()));
    assert!(caps.columns.contains(&"total_exec_time".to_string()));
}

#[test]
fn test_detected_columns_nonempty() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    if !caps.pgss_present { return; }
    assert!(!caps.columns.is_empty(), "should detect at least some columns");
}

#[test]
fn test_server_version_nonempty() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let v = pgprofile::capability::server_version(&mut client).expect("version");
    assert!(v.starts_with("PostgreSQL"), "version string: {v}");
}

#[test]
fn test_snapshot_returns_rows() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    if !caps.pgss_present { return; }
    let rows = pgprofile::pgss::snapshot(&mut client, &caps).expect("snapshot");
    // May be empty on idle cluster, but should not error
    let _ = rows;
}
