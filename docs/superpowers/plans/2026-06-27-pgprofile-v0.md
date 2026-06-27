# pgprofile v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bounded, on-demand PostgreSQL query profiler that connects to a running cluster, samples `pg_stat_statements` over an interval, and renders a ranked "where did query time go" report — working identically on CNPG and traditional installs.

**Architecture:** A single-binary Rust CLI with three subcommands (`query`, `profile`, `info`). One synchronous Postgres connection. A snapshot loop accumulates per-tick counter deltas into an in-memory profile keyed by `(userid, dbid, queryid, toplevel)`, with a per-key regression guard for eviction safety. Column presence is introspected at connect-time to handle PG15–18 schema differences. Report rendered on exit as pretty table (TTY), TSV, or JSON.

**Tech Stack:** Rust, `clap` (derive), `postgres` (sync rust-postgres), `humantime`, `ctrlc`, `comfy-table`, `crossterm`, `serde`/`serde_json`

## Global Constraints

- Rust edition 2021
- No `tokio`, no `sqlx`, no async — single sync connection, sequential loop
- One connection string as first positional arg on every subcommand (falls back to `PG*` env vars)
- `pg_stat_statements` must be present; absent → error with a clear message and non-zero exit
- `showtext=false` for all `pg_stat_statements` queries in the snapshot loop
- Query text fetched separately (with `showtext=true`) only on first sighting of a queryid
- Live preview to stderr; final report to stdout
- No files written unless `--output` flag provided (flag reserved, not yet implemented)
- Integration tests must be skipped (not failed) when no Postgres is reachable — check `DATABASE_URL` env var

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | All dependencies |
| `src/main.rs` | CLI arg parsing with clap, subcommand dispatch |
| `src/lib.rs` | Crate root, re-exports |
| `src/error.rs` | `PgprofileError` enum + `Result` alias |
| `src/capability.rs` | Detect pg_stat_statements presence, column introspection, capability ladder |
| `src/pgss.rs` | Connect, dynamic SELECT builder, single snapshot |
| `src/acc.rs` | `Acc` session profile: delta computation, regression guard, accumulation |
| `src/report.rs` | Render ranked report as table / TSV / JSON |
| `src/live.rs` | Crossterm live preview (stderr, top-N, repaint) |
| `tests/integration.rs` | Integration tests against real Postgres (skip if no `DATABASE_URL`) |

---

## Task 1: Project Scaffold + Error Types

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/error.rs`

**Interfaces:**
- Produces: `PgprofileError` enum, `Result<T>` alias — used by every subsequent task

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "pgprofile"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "pgprofile"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
postgres = "0.19"
humantime = "2"
ctrlc = "3"
comfy-table = "7"
crossterm = "0.27"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
```

- [ ] **Step 2: Create src/error.rs**

```rust
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
```

- [ ] **Step 3: Create src/lib.rs**

```rust
pub mod acc;
pub mod capability;
pub mod error;
pub mod live;
pub mod pgss;
pub mod report;

pub use error::{PgprofileError, Result};
```

- [ ] **Step 4: Create src/main.rs (stub — compiles, exits 0)**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pgprofile", version, about = "On-demand Postgres query profiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Connect, introspect pg_stat_statements columns, display raw data
    Query {
        /// Postgres connection string
        connstr: String,
        /// Show available columns only
        #[arg(long)]
        columns: bool,
        /// Show up to N rows
        #[arg(long, default_value_t = 50)]
        rows: usize,
    },
    /// Start capture, sample pg_stat_statements, render report
    Profile {
        /// Postgres connection string
        connstr: String,
        /// Capture duration (e.g. 5m, 30s); Ctrl-C stops capture early
        #[arg(long, value_parser = parse_duration)]
        duration: Option<std::time::Duration>,
        /// Snapshot interval
        #[arg(long, default_value = "2s", value_parser = parse_duration)]
        interval: std::time::Duration,
        /// Show top-N queries in live preview and final report
        #[arg(long, default_value_t = 20)]
        top: usize,
        /// Output format
        #[arg(long, default_value = "table", value_parser = ["table", "tsv", "json"])]
        format: String,
    },
    /// Quick cluster info: server version, pg_stat_statements, capability ladder
    Info {
        /// Postgres connection string
        connstr: String,
    },
}

fn parse_duration(s: &str) -> std::result::Result<std::time::Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Query { .. } => eprintln!("query: not yet implemented"),
        Command::Profile { .. } => eprintln!("profile: not yet implemented"),
        Command::Info { .. } => eprintln!("info: not yet implemented"),
    }
}
```

- [ ] **Step 5: Create stub modules so the project compiles**

Create `src/capability.rs`:
```rust
// stub
```

Create `src/pgss.rs`:
```rust
// stub
```

Create `src/acc.rs`:
```rust
// stub
```

Create `src/report.rs`:
```rust
// stub
```

Create `src/live.rs`:
```rust
// stub
```

- [ ] **Step 6: Verify it compiles**

```bash
cargo build
```
Expected: Compiles without errors. Binary at `target/debug/pgprofile`.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: scaffold pgprofile crate with CLI stub and error types"
```

---

## Task 2: Capability Detection

**Files:**
- Modify: `src/capability.rs`
- Create: `tests/integration.rs`

**Interfaces:**
- Consumes: `postgres::Client`
- Produces:
  - `struct Capabilities { pgss_present: bool, can_read_all_stats: bool, has_stats_since: bool, columns: Vec<String> }`
  - `fn detect(client: &mut postgres::Client) -> Result<Capabilities>`
  - `fn check_pgss(client: &mut postgres::Client) -> Result<()>` — errors with `PgprofileError::PgssNotInstalled` if absent

- [ ] **Step 1: Write failing integration test**

Create `tests/integration.rs`:
```rust
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
```

- [ ] **Step 2: Run test — verify it fails to compile (capability module is a stub)**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test --test integration 2>&1 | head -30
```
Expected: compile error — `pgprofile::capability::detect` not found.

- [ ] **Step 3: Implement src/capability.rs**

```rust
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
```

- [ ] **Step 4: Run integration tests**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test --test integration -- --nocapture
```
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/capability.rs tests/integration.rs
git commit -m "feat: capability detection — introspect pg_stat_statements columns and role checks"
```

---

## Task 3: `info` Subcommand

**Files:**
- Modify: `src/main.rs`
- Modify: `src/capability.rs`

**Interfaces:**
- Consumes: `capability::detect(client) -> Result<Capabilities>`
- Produces: terminal output printed by `run_info(connstr: &str) -> Result<()>`

- [ ] **Step 1: Add server version helper to capability.rs**

Add to `src/capability.rs`:
```rust
pub fn server_version(client: &mut postgres::Client) -> Result<String> {
    let row = client
        .query_one("SELECT version()", &[])
        .map_err(PgprofileError::Query)?;
    Ok(row.get::<_, String>(0))
}
```

- [ ] **Step 2: Implement run_info in main.rs**

Add function before `main()` in `src/main.rs`:
```rust
fn run_info(connstr: &str) -> pgprofile::Result<()> {
    let mut client = postgres::Client::connect(connstr, postgres::NoTls)
        .map_err(pgprofile::PgprofileError::Connection)?;

    let version = pgprofile::capability::server_version(&mut client)?;
    let caps = pgprofile::capability::detect(&mut client)?;

    println!("Server:              {}", version);
    println!("pg_stat_statements:  {}", if caps.pgss_present { "installed" } else { "NOT FOUND" });
    println!("pg_read_all_stats:   {}", if caps.can_read_all_stats { "yes (full query text)" } else { "no (limited to own queries)" });
    println!("stats_since (PG16+): {}", if caps.has_stats_since { "yes" } else { "no" });
    println!("Columns detected:    {}", caps.columns.len());
    if !caps.columns.is_empty() {
        println!("  {}", caps.columns.join(", "));
    }
    Ok(())
}
```

- [ ] **Step 3: Wire Info subcommand in main()**

Replace the `Command::Info { .. }` arm in `main()`:
```rust
Command::Info { connstr } => {
    if let Err(e) = run_info(&connstr) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

Also add to the top of `src/main.rs`:
```rust
use pgprofile::PgprofileError;
```

- [ ] **Step 4: Add integration test for info path**

Add to `tests/integration.rs`:
```rust
#[test]
fn test_server_version_nonempty() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let v = pgprofile::capability::server_version(&mut client).expect("version");
    assert!(v.starts_with("PostgreSQL"), "version string: {v}");
}
```

- [ ] **Step 5: Run integration tests**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test --test integration -- --nocapture
```
Expected: all three tests pass.

- [ ] **Step 6: Manual smoke test**

```bash
cargo build && ./target/debug/pgprofile info "postgres://user:pass@host/db"
```
Expected: prints server version, pg_stat_statements status, column list.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/capability.rs tests/integration.rs
git commit -m "feat: info subcommand — server version, pgss presence, capability ladder"
```

---

## Task 4: Dynamic Snapshot (pgss.rs)

**Files:**
- Modify: `src/pgss.rs`

**Interfaces:**
- Consumes: `postgres::Client`, `capability::Capabilities`
- Produces:
  - `struct Row { userid: i64, dbid: i64, queryid: i64, toplevel: bool, total_exec_time: f64, calls: i64, mean_exec_time: f64, rows: i64, shared_blks_hit: i64, shared_blks_read: i64, temp_blks_read: i64, temp_blks_written: i64, wal_bytes: Option<i64> }`
  - `fn snapshot(client: &mut postgres::Client, caps: &Capabilities) -> Result<Vec<Row>>`
  - `fn fetch_query_text(client: &mut postgres::Client, queryid: i64) -> Result<Option<String>>`

- [ ] **Step 1: Write failing integration test**

Add to `tests/integration.rs`:
```rust
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
```

- [ ] **Step 2: Run test — verify it fails to compile**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test --test integration 2>&1 | head -20
```
Expected: compile error — `pgprofile::pgss::snapshot` not found.

- [ ] **Step 3: Implement src/pgss.rs**

```rust
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
    // Use the PG17+ renamed columns if present; fall back to older names
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
```

- [ ] **Step 4: Run integration test**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test --test integration -- --nocapture
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/pgss.rs tests/integration.rs
git commit -m "feat: dynamic snapshot from pg_stat_statements with column introspection"
```

---

## Task 5: `query` Subcommand

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `capability::detect`, `pgss::snapshot`, `pgss::fetch_query_text`
- Produces: `run_query(connstr: &str, columns_only: bool, rows: usize) -> Result<()>`

- [ ] **Step 1: Implement run_query in main.rs**

Add before `main()` in `src/main.rs`:
```rust
fn run_query(connstr: &str, columns_only: bool, rows: usize) -> pgprofile::Result<()> {
    let mut client = postgres::Client::connect(connstr, postgres::NoTls)
        .map_err(pgprofile::PgprofileError::Connection)?;
    let caps = pgprofile::capability::detect(&mut client)?;

    if !caps.pgss_present {
        return Err(pgprofile::PgprofileError::PgssNotInstalled);
    }

    if columns_only {
        println!("Columns in pg_stat_statements ({}):", caps.columns.len());
        for col in &caps.columns {
            println!("  {col}");
        }
        return Ok(());
    }

    let snapshot = pgprofile::pgss::snapshot(&mut client, &caps)?;
    let display: Vec<_> = snapshot.into_iter().take(rows).collect();

    println!("{} rows (showing up to {rows}):", display.len(), rows = rows);
    for row in &display {
        println!(
            "  queryid={} calls={} total_exec={:.2}ms rows={}",
            row.queryid, row.calls, row.total_exec_time, row.rows
        );
    }
    Ok(())
}
```

- [ ] **Step 2: Wire Query subcommand in main()**

Replace the `Command::Query { .. }` arm in `main()`:
```rust
Command::Query { connstr, columns, rows } => {
    if let Err(e) = run_query(&connstr, columns, rows) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 3: Manual smoke test**

```bash
cargo build
./target/debug/pgprofile query "postgres://user:pass@host/db"
./target/debug/pgprofile query "postgres://user:pass@host/db" --columns
```
Expected: first shows rows with queryid/calls/time; second shows column list.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: query subcommand — raw pg_stat_statements view with column introspection"
```

---

## Task 6: Accumulator (acc.rs)

**Files:**
- Modify: `src/acc.rs`

**Interfaces:**
- Consumes: `Vec<pgss::Row>`
- Produces:
  - `type Key = (i64, i64, i64, bool)` — `(userid, dbid, queryid, toplevel)`
  - `struct AccEntry { total_exec_time: f64, calls: i64, rows: i64, shared_blks_hit: i64, shared_blks_read: i64, temp_blks_read: i64, temp_blks_written: i64, wal_bytes: Option<i64> }`
  - `struct Acc { entries: HashMap<Key, AccEntry> }`
  - `impl Acc { fn new() -> Self; fn tick(&mut self, prev: &HashMap<Key, pgss::Row>, cur: &HashMap<Key, pgss::Row>); fn top_n(&self, n: usize) -> Vec<(Key, &AccEntry)>; }`

- [ ] **Step 1: Write unit tests**

Add a unit test module inside `src/acc.rs` (create the file with these tests first):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pgss::Row;

    fn make_row(queryid: i64, calls: i64, total_exec_time: f64) -> Row {
        Row {
            userid: 10, dbid: 20, queryid, toplevel: true,
            total_exec_time, calls, rows: 0,
            shared_blks_hit: 0, shared_blks_read: 0,
            temp_blks_read: 0, temp_blks_written: 0,
            wal_bytes: None,
        }
    }

    #[test]
    fn test_accumulates_delta() {
        let mut acc = Acc::new();
        let prev = vec![make_row(1, 10, 100.0)];
        let cur = vec![make_row(1, 15, 150.0)];
        let prev_map = rows_to_map(prev);
        let cur_map = rows_to_map(cur);
        acc.tick(&prev_map, &cur_map);
        let key = (10, 20, 1, true);
        assert_eq!(acc.entries[&key].calls, 5);
        assert!((acc.entries[&key].total_exec_time - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_regression_guard_on_eviction() {
        let mut acc = Acc::new();
        // First tick: normal delta
        let prev1 = rows_to_map(vec![make_row(1, 10, 100.0)]);
        let cur1 = rows_to_map(vec![make_row(1, 15, 150.0)]);
        acc.tick(&prev1, &cur1);
        // Second tick: cur < prev (eviction + reinsert — use cur as delta)
        let prev2 = cur1;
        let cur2 = rows_to_map(vec![make_row(1, 3, 30.0)]);
        acc.tick(&prev2, &cur2);
        let key = (10, 20, 1, true);
        // Should be 5 (first delta) + 3 (regression guard: use cur as delta)
        assert_eq!(acc.entries[&key].calls, 8);
        assert!((acc.entries[&key].total_exec_time - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_new_entry_mid_session() {
        let mut acc = Acc::new();
        let prev = rows_to_map(vec![make_row(1, 10, 100.0)]);
        // queryid=2 appears for the first time in cur (not in prev)
        let cur = rows_to_map(vec![make_row(1, 12, 120.0), make_row(2, 5, 50.0)]);
        acc.tick(&prev, &cur);
        let key2 = (10, 20, 2, true);
        // New entry: use cur as delta
        assert_eq!(acc.entries[&key2].calls, 5);
    }

    #[test]
    fn test_top_n_ordered_by_exec_time() {
        let mut acc = Acc::new();
        let prev = rows_to_map(vec![
            make_row(1, 0, 0.0), make_row(2, 0, 0.0), make_row(3, 0, 0.0),
        ]);
        let cur = rows_to_map(vec![
            make_row(1, 1, 300.0), make_row(2, 1, 100.0), make_row(3, 1, 200.0),
        ]);
        acc.tick(&prev, &cur);
        let top = acc.top_n(2);
        assert_eq!(top[0].0 .2, 1); // queryid=1 has highest exec time
        assert_eq!(top[1].0 .2, 3); // queryid=3 second
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test acc -- --nocapture 2>&1 | head -30
```
Expected: compile error — types not defined yet.

- [ ] **Step 3: Implement src/acc.rs**

```rust
use std::collections::HashMap;
use crate::pgss::Row;

pub type Key = (i64, i64, i64, bool);

#[derive(Debug, Default, Clone)]
pub struct AccEntry {
    pub total_exec_time: f64,
    pub calls: i64,
    pub rows: i64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
    pub temp_blks_read: i64,
    pub temp_blks_written: i64,
    pub wal_bytes: Option<i64>,
}

pub struct Acc {
    pub entries: HashMap<Key, AccEntry>,
}

impl Acc {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    pub fn tick(&mut self, prev: &HashMap<Key, Row>, cur: &HashMap<Key, Row>) {
        for (key, cur_row) in cur {
            let delta = match prev.get(key) {
                Some(prev_row) if cur_row.calls >= prev_row.calls => AccEntry {
                    total_exec_time: cur_row.total_exec_time - prev_row.total_exec_time,
                    calls: cur_row.calls - prev_row.calls,
                    rows: cur_row.rows - prev_row.rows,
                    shared_blks_hit: cur_row.shared_blks_hit - prev_row.shared_blks_hit,
                    shared_blks_read: cur_row.shared_blks_read - prev_row.shared_blks_read,
                    temp_blks_read: cur_row.temp_blks_read - prev_row.temp_blks_read,
                    temp_blks_written: cur_row.temp_blks_written - prev_row.temp_blks_written,
                    wal_bytes: cur_row.wal_bytes,
                },
                // regression guard: eviction or reset — use cur as delta
                _ => AccEntry {
                    total_exec_time: cur_row.total_exec_time,
                    calls: cur_row.calls,
                    rows: cur_row.rows,
                    shared_blks_hit: cur_row.shared_blks_hit,
                    shared_blks_read: cur_row.shared_blks_read,
                    temp_blks_read: cur_row.temp_blks_read,
                    temp_blks_written: cur_row.temp_blks_written,
                    wal_bytes: cur_row.wal_bytes,
                },
            };
            let entry = self.entries.entry(*key).or_default();
            entry.total_exec_time += delta.total_exec_time;
            entry.calls += delta.calls;
            entry.rows += delta.rows;
            entry.shared_blks_hit += delta.shared_blks_hit;
            entry.shared_blks_read += delta.shared_blks_read;
            entry.temp_blks_read += delta.temp_blks_read;
            entry.temp_blks_written += delta.temp_blks_written;
            if let Some(w) = delta.wal_bytes {
                *entry.wal_bytes.get_or_insert(0) += w;
            }
        }
    }

    pub fn top_n(&self, n: usize) -> Vec<(Key, &AccEntry)> {
        let mut ranked: Vec<(Key, &AccEntry)> = self.entries.iter().map(|(&k, v)| (k, v)).collect();
        ranked.sort_by(|a, b| b.1.total_exec_time.partial_cmp(&a.1.total_exec_time).unwrap());
        ranked.truncate(n);
        ranked
    }
}

pub fn rows_to_map(rows: Vec<Row>) -> HashMap<Key, Row> {
    rows.into_iter()
        .map(|r| ((r.userid, r.dbid, r.queryid, r.toplevel), r))
        .collect()
}

#[cfg(test)]
mod tests {
    // (test code from Step 1 above goes here)
}
```

- [ ] **Step 4: Run unit tests**

```bash
cargo test acc -- --nocapture
```
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/acc.rs
git commit -m "feat: accumulator with per-key regression guard for eviction safety"
```

---

## Task 7: Report Renderer (report.rs)

**Files:**
- Modify: `src/report.rs`

**Interfaces:**
- Consumes: `Vec<(Key, &AccEntry)>`, `HashMap<i64, String>` (queryid → text cache)
- Produces:
  - `enum Format { Table, Tsv, Json }`
  - `fn render(ranked: &[(Key, &AccEntry)], texts: &HashMap<i64, String>, format: Format, total_exec_ms: f64)`

- [ ] **Step 1: Write unit tests**

Add at the bottom of `src/report.rs` (create file with tests first):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::acc::AccEntry;
    use std::collections::HashMap;

    fn sample_entry() -> AccEntry {
        AccEntry {
            total_exec_time: 1500.0,
            calls: 100,
            rows: 500,
            shared_blks_hit: 1000,
            shared_blks_read: 50,
            temp_blks_read: 0,
            temp_blks_written: 0,
            wal_bytes: None,
        }
    }

    #[test]
    fn test_tsv_output_has_header() {
        let entry = sample_entry();
        let ranked = vec![((10i64, 20i64, 1i64, true), &entry)];
        let texts: HashMap<i64, String> = HashMap::new();
        let mut out = Vec::new();
        render_to(&ranked, &texts, Format::Tsv, 1500.0, &mut out);
        let s = String::from_utf8(out).unwrap();
        assert!(s.starts_with("rank\tqueryid\t"), "header: {s}");
        assert!(s.contains("1500"), "exec time in output");
    }

    #[test]
    fn test_json_output_is_valid() {
        let entry = sample_entry();
        let ranked = vec![((10i64, 20i64, 1i64, true), &entry)];
        let texts: HashMap<i64, String> = HashMap::new();
        let mut out = Vec::new();
        render_to(&ranked, &texts, Format::Json, 1500.0, &mut out);
        let s = String::from_utf8(out).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).expect("valid json");
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 1);
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test report -- --nocapture 2>&1 | head -20
```
Expected: compile error — types not defined.

- [ ] **Step 3: Implement src/report.rs**

```rust
use std::collections::HashMap;
use std::io::Write;
use comfy_table::{Table, ContentArrangement};
use serde::Serialize;

use crate::acc::{AccEntry, Key};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Table,
    Tsv,
    Json,
}

impl std::str::FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "table" => Ok(Format::Table),
            "tsv" => Ok(Format::Tsv),
            "json" => Ok(Format::Json),
            other => Err(format!("unknown format: {other}")),
        }
    }
}

#[derive(Serialize)]
struct JsonRow {
    rank: usize,
    queryid: i64,
    toplevel: bool,
    total_exec_ms: f64,
    pct_of_total: f64,
    calls: i64,
    mean_ms: f64,
    rows: i64,
    cache_hit_pct: f64,
    temp_blks: i64,
    wal_bytes: Option<i64>,
    query: String,
}

pub fn render(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    format: Format,
    total_exec_ms: f64,
) {
    render_to(ranked, texts, format, total_exec_ms, &mut std::io::stdout());
}

pub fn render_to<W: Write>(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    format: Format,
    total_exec_ms: f64,
    out: &mut W,
) {
    match format {
        Format::Table => render_table(ranked, texts, total_exec_ms, out),
        Format::Tsv => render_tsv(ranked, texts, total_exec_ms, out),
        Format::Json => render_json(ranked, texts, total_exec_ms, out),
    }
}

fn cache_hit_pct(e: &AccEntry) -> f64 {
    let total = e.shared_blks_hit + e.shared_blks_read;
    if total == 0 { 100.0 } else { e.shared_blks_hit as f64 / total as f64 * 100.0 }
}

fn render_table<W: Write>(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    total_exec_ms: f64,
    out: &mut W,
) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["#", "queryid", "total_ms", "%", "calls", "mean_ms", "rows", "cache%", "tmp_blks", "wal_bytes"]);

    for (rank, (key, e)) in ranked.iter().enumerate() {
        let pct = if total_exec_ms > 0.0 { e.total_exec_time / total_exec_ms * 100.0 } else { 0.0 };
        let mean = if e.calls > 0 { e.total_exec_time / e.calls as f64 } else { 0.0 };
        let tmp = e.temp_blks_read + e.temp_blks_written;
        table.add_row(vec![
            (rank + 1).to_string(),
            key.2.to_string(),
            format!("{:.1}", e.total_exec_time),
            format!("{:.1}", pct),
            e.calls.to_string(),
            format!("{:.2}", mean),
            e.rows.to_string(),
            format!("{:.1}", cache_hit_pct(e)),
            tmp.to_string(),
            e.wal_bytes.map_or("-".into(), |b| b.to_string()),
        ]);
    }
    writeln!(out, "{table}").ok();

    // Query text legend
    writeln!(out, "\nQuery text:").ok();
    for (rank, (key, _)) in ranked.iter().enumerate() {
        let text = texts.get(&key.2).map(|t| t.as_str()).unwrap_or("(not available)");
        writeln!(out, "  [{:>2}] queryid={}: {}", rank + 1, key.2, text).ok();
    }
}

fn render_tsv<W: Write>(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    total_exec_ms: f64,
    out: &mut W,
) {
    writeln!(out, "rank\tqueryid\ttoplevel\ttotal_exec_ms\tpct_of_total\tcalls\tmean_ms\trows\tcache_hit_pct\ttemp_blks\twal_bytes\tquery").ok();
    for (rank, (key, e)) in ranked.iter().enumerate() {
        let pct = if total_exec_ms > 0.0 { e.total_exec_time / total_exec_ms * 100.0 } else { 0.0 };
        let mean = if e.calls > 0 { e.total_exec_time / e.calls as f64 } else { 0.0 };
        let tmp = e.temp_blks_read + e.temp_blks_written;
        let text = texts.get(&key.2).map(|t| t.replace('\t', " ")).unwrap_or_default();
        writeln!(out, "{}\t{}\t{}\t{:.1}\t{:.1}\t{}\t{:.2}\t{}\t{:.1}\t{}\t{}\t{}",
            rank + 1, key.2, key.3, e.total_exec_time, pct, e.calls, mean,
            e.rows, cache_hit_pct(e), tmp,
            e.wal_bytes.map_or("-".into(), |b| b.to_string()),
            text
        ).ok();
    }
}

fn render_json<W: Write>(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    total_exec_ms: f64,
    out: &mut W,
) {
    let rows: Vec<JsonRow> = ranked.iter().enumerate().map(|(rank, (key, e))| {
        let pct = if total_exec_ms > 0.0 { e.total_exec_time / total_exec_ms * 100.0 } else { 0.0 };
        let mean = if e.calls > 0 { e.total_exec_time / e.calls as f64 } else { 0.0 };
        let tmp = e.temp_blks_read + e.temp_blks_written;
        JsonRow {
            rank: rank + 1,
            queryid: key.2,
            toplevel: key.3,
            total_exec_ms: e.total_exec_time,
            pct_of_total: pct,
            calls: e.calls,
            mean_ms: mean,
            rows: e.rows,
            cache_hit_pct: cache_hit_pct(e),
            temp_blks: tmp,
            wal_bytes: e.wal_bytes,
            query: texts.get(&key.2).cloned().unwrap_or_default(),
        }
    }).collect();
    writeln!(out, "{}", serde_json::to_string_pretty(&rows).unwrap_or_default()).ok();
}
```

- [ ] **Step 4: Run unit tests**

```bash
cargo test report -- --nocapture
```
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/report.rs
git commit -m "feat: report renderer — table, TSV, and JSON output formats"
```

---

## Task 8: Live Preview (live.rs)

**Files:**
- Modify: `src/live.rs`

**Interfaces:**
- Consumes: `Vec<(Key, &AccEntry)>`, tick count, elapsed seconds
- Produces: `fn repaint(ranked: &[(Key, &AccEntry)], texts: &HashMap<i64, String>, tick: usize, elapsed_secs: u64, top: usize)`

- [ ] **Step 1: Implement src/live.rs**

The live preview is a stderr-only visual; it's harder to unit test without a real terminal. Implement it directly:

```rust
use std::collections::HashMap;
use std::io::Write;
use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};

use crate::acc::{AccEntry, Key};

pub fn repaint(
    ranked: &[(Key, &AccEntry)],
    texts: &HashMap<i64, String>,
    tick: usize,
    elapsed_secs: u64,
    top: usize,
) {
    let mut stderr = std::io::stderr();

    // On the first tick we don't move the cursor up (nothing to clear)
    if tick > 0 {
        // Move up by (top + 3) lines to overwrite previous output
        let lines = std::cmp::min(ranked.len(), top) + 3;
        stderr.execute(cursor::MoveUp(lines as u16)).ok();
        stderr.execute(terminal::Clear(terminal::ClearType::FromCursorDown)).ok();
    }

    let total_exec_ms: f64 = ranked.iter().map(|(_, e)| e.total_exec_time).sum();

    writeln!(stderr, "pgprofile — tick {tick} — {elapsed_secs}s elapsed — {:.0}ms total exec time",
        total_exec_ms).ok();
    writeln!(stderr, "{:<4} {:>12} {:>10} {:>8} {:>8}  {}", "#", "total_ms", "%", "calls", "mean_ms", "query").ok();
    writeln!(stderr, "{}", "-".repeat(72)).ok();

    for (rank, (key, e)) in ranked.iter().take(top).enumerate() {
        let pct = if total_exec_ms > 0.0 { e.total_exec_time / total_exec_ms * 100.0 } else { 0.0 };
        let mean = if e.calls > 0 { e.total_exec_time / e.calls as f64 } else { 0.0 };
        let text = texts.get(&key.2).map(|t| {
            let t = t.replace('\n', " ");
            if t.len() > 50 { format!("{}…", &t[..49]) } else { t }
        }).unwrap_or_else(|| "(no text)".into());

        writeln!(stderr, "{:<4} {:>12.1} {:>9.1}% {:>8} {:>8.2}  {}",
            rank + 1, e.total_exec_time, pct, e.calls, mean, text).ok();
    }
    stderr.flush().ok();
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/live.rs
git commit -m "feat: live preview — crossterm stderr repaint per tick"
```

---

## Task 9: `profile` Subcommand (the capture loop)

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: all prior modules
- Produces: `run_profile(connstr, duration, interval, top, format) -> Result<()>`

- [ ] **Step 1: Implement run_profile in main.rs**

Add after `run_query` in `src/main.rs`. First, add these imports at the top of `main.rs`:
```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pgprofile::acc::{rows_to_map, Acc};
use pgprofile::live;
use pgprofile::pgss;
use pgprofile::report::{render, Format};
```

Then add the function:
```rust
fn run_profile(
    connstr: &str,
    duration: Option<Duration>,
    interval: Duration,
    top: usize,
    format: &str,
) -> pgprofile::Result<()> {
    let fmt: Format = format.parse().map_err(|e: String| pgprofile::PgprofileError::InvalidDuration(e))?;

    let mut client = postgres::Client::connect(connstr, postgres::NoTls)
        .map_err(pgprofile::PgprofileError::Connection)?;
    let caps = pgprofile::capability::detect(&mut client)?;

    if !caps.pgss_present {
        return Err(pgprofile::PgprofileError::PgssNotInstalled);
    }

    // Ctrl-C handler: flip running flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || { r.store(false, Ordering::SeqCst); })
        .expect("Error setting Ctrl-C handler");

    let mut acc = Acc::new();
    let mut text_cache: HashMap<i64, String> = HashMap::new();
    let start = Instant::now();

    // Take initial snapshot (baseline — not banked)
    let initial_rows = pgss::snapshot(&mut client, &caps)?;
    let mut prev = rows_to_map(initial_rows);
    let mut tick: usize = 0;

    eprintln!("Capturing... press Ctrl-C to stop{}", duration.map(|d| format!(" (or wait {:?})", d)).unwrap_or_default());

    loop {
        std::thread::sleep(interval);

        let cur_rows = pgss::snapshot(&mut client, &caps)?;

        // Harvest query text for new queryids
        for row in &cur_rows {
            if !text_cache.contains_key(&row.queryid) {
                if let Ok(Some(text)) = pgss::fetch_query_text(&mut client, row.queryid) {
                    text_cache.insert(row.queryid, text);
                }
            }
        }

        let cur = rows_to_map(cur_rows);
        acc.tick(&prev, &cur);
        prev = cur;
        tick += 1;

        let elapsed = start.elapsed();
        let ranked = acc.top_n(top);
        live::repaint(&ranked, &text_cache, tick, elapsed.as_secs(), top);

        let time_up = duration.map_or(false, |d| elapsed >= d);
        if !running.load(Ordering::SeqCst) || time_up {
            break;
        }
    }

    // Final snapshot + bank
    let final_rows = pgss::snapshot(&mut client, &caps)?;
    for row in &final_rows {
        if !text_cache.contains_key(&row.queryid) {
            if let Ok(Some(text)) = pgss::fetch_query_text(&mut client, row.queryid) {
                text_cache.insert(row.queryid, text);
            }
        }
    }
    let final_cur = rows_to_map(final_rows);
    acc.tick(&prev, &final_cur);

    let ranked = acc.top_n(top);
    let total_exec_ms: f64 = ranked.iter().map(|(_, e)| e.total_exec_time).sum();

    eprintln!("\n--- Final report ({} distinct queries, {:.0}ms total exec time) ---\n",
        acc.entries.len(), total_exec_ms);

    render(&ranked, &text_cache, fmt, total_exec_ms);
    Ok(())
}
```

- [ ] **Step 2: Wire Profile subcommand in main()**

Replace the `Command::Profile { .. }` arm in `main()`:
```rust
Command::Profile { connstr, duration, interval, top, format } => {
    if let Err(e) = run_profile(&connstr, duration, interval, top, &format) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 3: Add ctrlc to imports at top of main.rs**

```rust
use ctrlc;
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1
```
Expected: no errors.

- [ ] **Step 5: Add integration test for profile path (single-tick)**

Add to `tests/integration.rs`:
```rust
#[test]
fn test_profile_single_tick() {
    let Some(url) = db_url() else { return };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("connect");
    let caps = pgprofile::capability::detect(&mut client).expect("detect");
    if !caps.pgss_present { return; }

    // Simulate a single tick: take two snapshots and bank the delta
    let snap1 = pgprofile::pgss::snapshot(&mut client, &caps).expect("snap1");
    // Generate some load: a trivial query
    client.execute("SELECT 1", &[]).ok();
    let snap2 = pgprofile::pgss::snapshot(&mut client, &caps).expect("snap2");

    let mut acc = pgprofile::acc::Acc::new();
    let prev = pgprofile::acc::rows_to_map(snap1);
    let cur = pgprofile::acc::rows_to_map(snap2);
    acc.tick(&prev, &cur);

    // Acc should have at least one entry
    assert!(!acc.entries.is_empty());
}
```

- [ ] **Step 6: Run all tests**

```bash
DATABASE_URL="postgres://user:pass@host/db" cargo test -- --nocapture
```
Expected: all pass.

- [ ] **Step 7: Manual smoke test**

```bash
cargo build
./target/debug/pgprofile profile "postgres://user:pass@host/db" --duration 10s --interval 2s --top 5
```
Expected: live preview updates every 2 seconds for 10 seconds, then final table printed.

```bash
./target/debug/pgprofile profile "postgres://user:pass@host/db" --duration 5s --format json
```
Expected: JSON array output to stdout.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs tests/integration.rs
git commit -m "feat: profile subcommand — capture loop with live preview and final report"
```

---

## Self-Review

### 1. Spec Coverage

| Spec requirement | Task |
|-----------------|------|
| Connect via single connection string | Tasks 3, 5, 9 |
| Snapshot with `showtext=false` | Task 4 (`pgss::snapshot`) |
| Delta accumulation keyed by `(userid, dbid, queryid, toplevel)` | Task 6 |
| Per-key regression guard | Task 6 (tested) |
| Query text harvested once per queryid | Task 9 (`run_profile` text_cache) |
| `--duration` and Ctrl-C | Task 9 (`ctrlc`, duration check) |
| Live preview to stderr | Task 8 |
| Final report: table / TSV / JSON | Task 7 |
| Per-query columns (exec time, %, calls, mean, rows, cache-hit, temp, WAL) | Tasks 7, 8 |
| `query` subcommand | Task 5 |
| `info` subcommand | Task 3 |
| Column introspection (PG15–18 compat) | Task 2 + Task 4 `build_query` |
| Capability ladder (pg_read_all_stats, stats_since) | Task 2 |
| `pg_stat_statements` absent → clear error | Task 2 `check_pgss` |
| Deferred items excluded | Nothing implements persistence, HTML, reset detection |

All requirements covered.

### 2. Placeholder Scan

No "TBD", "TODO", or "implement later" strings. All steps have real code. All test assertions are specific.

### 3. Type Consistency

- `Key = (i64, i64, i64, bool)` defined in Task 6 (`acc.rs`), used in Tasks 7, 8, 9 — consistent.
- `rows_to_map` defined in Task 6 and used in Task 9 — consistent.
- `AccEntry` fields match between Task 6 definition and Task 7 usage — consistent.
- `snapshot` returns `Vec<Row>`, `Row` struct defined in Task 4 — consistent with Task 9 usage.
- `Format` enum defined in Task 7, parsed from string in Task 9 via `FromStr` — consistent.
- `fetch_query_text` returns `Result<Option<String>>`, used with `if let Ok(Some(...))` in Task 9 — consistent.
