//! Integration tests — skipped when DATABASE_URL is not set.

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

#[test]
fn test_detect_capabilities_requires_pgss() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    // pg_stat_statements must be enabled on the test cluster
    assert!(caps.pgss_present, "pg_stat_statements must be installed");
}

#[test]
fn test_detected_columns_nonempty() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    assert!(!caps.columns.is_empty(), "should detect at least some columns");
    // core columns present in all PG versions
    assert!(caps.columns.contains(&"queryid".to_string()));
    assert!(caps.columns.contains(&"calls".to_string()));
    assert!(caps.columns.contains(&"total_exec_time".to_string()));
}
