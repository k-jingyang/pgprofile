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

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Query { .. } => eprintln!("query: not yet implemented"),
        Command::Profile { .. } => eprintln!("profile: not yet implemented"),
        Command::Info { connstr } => {
            if let Err(e) = run_info(&connstr) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
    }
}
