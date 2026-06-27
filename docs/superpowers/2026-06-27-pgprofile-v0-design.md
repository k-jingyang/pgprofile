# pgprofile v0 — Design Spec

> Working name; swap as desired. A bounded, on-demand PostgreSQL query profiler built on `pg_stat_statements`.

## One-liner

A bounded, on-demand Postgres query profiler that reconstructs pgbadger-style "where did query time go" reports from `pg_stat_statements` — with no log parsing, no config changes, and no superuser — working identically on CNPG and traditional installs.

## Problem

pgbadger-style query profiling depends on log parsing via `log_min_duration_statement`. That is impractical on CloudNativePG (log streams trapped in pods), and everywhere it means config changes, overhead, and often a restart. Operators need to profile query performance on a *running* cluster they may not control, without touching server configuration.

## Users

DBAs and platform/application engineers diagnosing query performance during an incident, load test, or capacity review — especially on CNPG.

## Goals

- `perf record` / `perf report` shape: start, capture over a workload, stop, get a ranked report.
- Zero-install: works against any cluster where stock `pg_stat_statements` is present — no preload beyond that, no restart, no superuser.
- One connection string, one instance; identical behavior on CNPG and plain Postgres.
- A live preview during capture that doubles as a "is it working / roughly what's hot" view.

## Non-goals (v0)

- Not a daemon or continuous monitor.
- No live session view or query killing (pg_activity covers that).
- No percentiles or stddev (pgss cannot support them windowed).
- No real bind values, no within-session timeline, no cross-instance rollup.
- No log parsing.

## Deferred (v0 excluded)

- Reset/failover detection + graceful handling (deferred in PRD).
- "Have I seen enough" convergence signals (deferred in PRD).
- On-disk persistence (`--output` / `--report`), HTML report, `--order-by` flags, per-tick rate columns.

## CLI

### Subcommands

| Subcommand | Purpose |
|------------|---------|
| `query` | Dev tool: connect, introspect pg_stat_statements columns, display raw data |
| `profile` | The profiler: start capture, sample pg_stat_statements, render report |
| `info` | Quick cluster info: PG version, pg_stat_statements enabled, capability ladder |

### `profile` flags

| Flag | Type | Default | Purpose |
|------|------|---------|---------|
| `--duration <dur>` | `humantime` | — | Capture duration, then render (required or Ctrl-C) |
| `--interval <dur>` | `humantime` | `2s` | Snapshot interval |
| `--top <N>` | `usize` | 20 | Show top-N queries |
| `--format <fmt>` | `table \| tsv \| json` | `table` | Output format |
| `--output <file>` | `PathBuf` | — | Write report to file (reserve; deferred) |

### `query` flags

| Flag | Type | Default | Purpose |
|------|------|---------|---------|
| `--columns` | flag | — | Show available columns only |
| `--rows <N>` | `usize` | 50 | Show up to N rows |

### `info`

No flags. Prints: server version, pg_stat_statements enabled, capability ladder (column support per PG version, `pg_read_all_stats`, `stats_since`).

### Connection

First positional argument: connection string. Falls back to standard `PG*` env vars (postgres-rust library behavior).

## Functional requirements

- Connect via a single connection string.
- Snapshot `pg_stat_statements` on an interval with `showtext=false`.
- Accumulate per-tick deltas keyed by `(userid, dbid, queryid, toplevel)` into an in-memory session profile (`acc`), robust to entry eviction via a per-key regression guard.
- Capture each query's normalized text once, on first sighting; cache by queryid.
- Bound the session by `--duration <dur>` or Ctrl-C; both take a final snapshot and render.
- Live-print the current top-N each tick as a preview of the final report (to stderr).
- On exit, render a top-N table ranked by total exec time, with a per-query text legend; output as pretty table (TTY) or TSV/JSON (piped).

## Per-query columns

Total exec time and % share (headline), calls, mean (exact = Δexec/Δcalls), rows, cache-hit ratio, temp spill, WAL bytes.

## Core data model

- **`acc`** — the session profile. A map keyed by `(userid, dbid, queryid, toplevel)`; each value is the running sum of that query's per-tick counter deltas. Size bounded by distinct queries seen, not by tick count.
- **`prev`** — the immediately previous snapshot; scratch, used once per tick to compute the next delta, then replaced.
- Each tick: `delta = cur - prev` per key, **with a regression guard** — if `cur[key] < prev[key]` (eviction-and-reinsert, or reset), use `cur[key]` as the delta instead of subtracting. Add deltas into `acc`.
- `acc` is never reset; identifiers survive resets. "Bank and continue."
- Session total = sum of observed per-tick deltas (equals `latest − baseline` only in the eviction-free, reset-free case; correct in all others).

## Technical approach

- **Connect-time column introspection** resolves PG15–18 schema differences — including the PG17 `blk_read_time` → `shared_blk_read_time`/`local_blk_read_time` rename — and collects newer columns when present (NULL otherwise). Introspect the actual view columns rather than inferring from server version.
- `pg_read_all_stats` / `pg_monitor` enables full query text; degrade gracefully (counters still work; text shows "not permitted" vs "evicted" distinctly).
- Query text harvested once per queryid the tick it first appears, while pgss still holds it; persisted into the session, independent of pgss eviction.
- Ephemeral in-memory store built on a **serializable session struct**, so `--output` / `--report` persistence and richer output surfaces bolt on without touching the core.
- **Capability ladder** detected at connect: `pg_stat_statements` present is the floor (absent → tool cannot run); `pg_read_all_stats` unlocks full text; PG16+ unlocks clean per-entry reset detection via `stats_since`; PG18 adds newer columns and tighter queryid grouping.

## v0 implementation stack (Rust)

| Crate | Purpose |
|-------|---------|
| `clap` (derive) | Argument parsing |
| `postgres` (sync rust-postgres) | Single-connection driver. Chosen over `sqlx` (dynamic SELECT defeats compile-time macros) and `tokio-postgres` (one sequential loop, no concurrency) |
| `humantime` | Parse `--duration` / `--interval` values (`5m`, `2h`) |
| `ctrlc` | SIGINT handler; flips a flag, the tick loop breaks |
| `comfy-table` | Final table rendering |
| `crossterm` | Live preview repaint (cursor control), on stderr |
| `serde` / `serde_json` | `--format json` output; serializable session struct |

Deferred: `rusqlite` for persistence.

## Milestones

| #  | Name           | What works                        | Key crates              | Scope |
|----|----------------|-----------------------------------|-------------------------|-------|
| 1  | **Connect + Introspect** | `pgprofile query <connstr>` prints pg_stat_statements columns found + row count | `postgres`, `clap` | Connect, introspect columns, print count |
| 2  | **Single Snapshot** | Full `SELECT` from introspected columns, display table | `comfy-table` | Dynamic query builder, single-result table |
| 3  | **Delta Loop** | Interval snapshots → deltas → `acc` → regression guard | `humantime`, `ctrlc` | Snapshot loop, accumulation, Ctrl-C |
| 4  | **Reporting** | Live preview (crossterm on stderr) + final table + TSV/JSON + Ctrl-C / `--duration` | `crossterm`, `serde` | Terminal UI, all output formats |

## Success criteria

An operator points the tool at a CNPG or plain Postgres cluster with zero config change and, after a bounded capture, gets an accurate ranked profile of where query time went — something that otherwise requires pgbadger plus log configuration they may not be able to enable.

## Project structure (v0)

```
pgprofile/
  Cargo.toml          — deps: postgres, clap, comfy-table (later: humantime, ctrlc, crossterm, serde)
  src/
    main.rs           — CLI dispatch (query/profile/info)
    lib.rs            — Re-exports, session root
    error.rs          — Custom error types and conversions
    pgss.rs           — Connect, introspect columns, snapshot pg_stat_statements
    acc.rs            — Accumulation / delta computation / regression guard
    report.rs         — Table rendering (TTY/TSV/JSON)
    capability.rs     — Capability ladder detection
  tests/
    integration.rs    — Integration test skeleton (skip if no PG)
```
