use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

use pgprofile::acc::{rows_to_map, Acc};
use pgprofile::live;
use pgprofile::pgss;
use pgprofile::report::{render, Format};

#[derive(Parser)]
#[command(name = "pgprofile", version, about = "On-demand Postgres query profiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

const CONNSTR_HELP: &str = "Postgres connection string

Examples:
  postgres://user:password@host:5432/dbname";

#[derive(Subcommand)]
enum Command {
    /// Connect, introspect pg_stat_statements columns, display raw data
    Query {
        #[arg(help = "Postgres connection string", long_help = CONNSTR_HELP)]
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
        #[arg(help = "Postgres connection string", long_help = CONNSTR_HELP)]
        connstr: String,
        /// Capture duration (e.g. 5m, 30s); Ctrl-C stops capture early
        #[arg(long, value_parser = parse_duration)]
        duration: Option<Duration>,
        /// Snapshot interval
        #[arg(long, default_value = "2s", value_parser = parse_duration)]
        interval: Duration,
        /// Show top-N queries in live preview and final report
        #[arg(long, default_value_t = 20)]
        top: usize,
        /// Output format
        #[arg(long, default_value = "table", value_parser = ["table", "tsv", "json"])]
        format: String,
    },
    /// Quick cluster info: server version, pg_stat_statements, capability ladder
    Info {
        #[arg(help = "Postgres connection string", long_help = CONNSTR_HELP)]
        connstr: String,
    },
}

fn parse_duration(s: &str) -> std::result::Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

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

    let snapshot = pgss::snapshot(&mut client, &caps)?;
    let display: Vec<_> = snapshot.into_iter().take(rows).collect();

    println!("{} rows (showing up to {rows}):", display.len());
    for row in &display {
        println!(
            "  queryid={} calls={} total_exec={:.2}ms rows={}",
            row.queryid, row.calls, row.total_exec_time, row.rows
        );
    }
    Ok(())
}

fn run_profile(
    connstr: &str,
    duration: Option<Duration>,
    interval: Duration,
    top: usize,
    format: &str,
) -> pgprofile::Result<()> {
    let fmt: Format = format.parse()
        .map_err(pgprofile::PgprofileError::InvalidDuration)?;

    let mut client = postgres::Client::connect(connstr, postgres::NoTls)
        .map_err(pgprofile::PgprofileError::Connection)?;
    let caps = pgprofile::capability::detect(&mut client)?;

    if !caps.pgss_present {
        return Err(pgprofile::PgprofileError::PgssNotInstalled);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || { r.store(false, Ordering::SeqCst); })
        .expect("Error setting Ctrl-C handler");

    let mut acc = Acc::new();
    let mut text_cache: HashMap<i64, String> = HashMap::new();
    let start = Instant::now();

    let initial_rows = pgss::snapshot(&mut client, &caps)?;
    let mut prev = rows_to_map(initial_rows);
    let mut tick: usize = 0;

    eprintln!("Capturing... press Ctrl-C to stop{}",
        duration.map(|d| format!(" (or wait {:?})", d)).unwrap_or_default());

    loop {
        std::thread::sleep(interval);

        let cur_rows = pgss::snapshot(&mut client, &caps)?;

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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Query { connstr, columns, rows } => {
            if let Err(e) = run_query(&connstr, columns, rows) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        Command::Profile { connstr, duration, interval, top, format } => {
            if let Err(e) = run_profile(&connstr, duration, interval, top, &format) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
        Command::Info { connstr } => {
            if let Err(e) = run_info(&connstr) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
    }
}
