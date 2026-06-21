# pgprofile — PRD

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

## Non-goals (v1)

- Not a daemon or continuous monitor.
- No live session view or query killing (pg_activity covers that).
- No percentiles or stddev (pgss cannot support them windowed).
- No real bind values, no within-session timeline, no cross-instance rollup.
- No log parsing.

## Functional requirements

- Connect via a single connection string.
- Snapshot `pg_stat_statements` on an interval with `showtext=false`.
- Accumulate per-tick deltas keyed by `(userid, dbid, queryid, toplevel)` into an in-memory session profile (`acc`), robust to entry eviction via a per-key regression guard.
- Capture each query's normalized text once, on first sighting; cache by queryid.
- Bound the session by `--time <dur>` or Ctrl-C; both take a final snapshot and render.
- Live-print the current top-N each tick as a preview of the final report (to stderr).
- On exit, render a top-N table ranked by total exec time, with a per-query text legend; output as pretty table (TTY) or TSV/JSON (piped).

## Per-query columns

Total exec time and % share (headline), calls, mean (exact = Δexec/Δcalls), rows, cache-hit ratio, temp spill, WAL bytes.

## Core data model

- **`acc`** — the session profile. A map keyed by `(userid, dbid, queryid, toplevel)`; each value is the running sum of that query's per-tick counter deltas. This is what gets ranked and rendered. Size bounded by distinct queries seen, not by tick count.
- **`prev`** — the immediately previous snapshot; scratch, used once per tick to compute the next delta, then replaced.
- Each tick: `delta = cur - prev` per key, **with a regression guard** — if `cur[key] < prev[key]` (eviction-and-reinsert, or reset), use `cur[key]` as the delta instead of subtracting. Add deltas into `acc`.
- `acc` is never reset; identifiers (`queryid`, `dbid`, `userid`) survive resets, so a query reappearing post-reset lands back in its own `acc` entry. "Bank and continue."
- Session total = sum of observed per-tick deltas (equals `latest − baseline` only in the eviction-free, reset-free case; correct in all others).

## Technical approach

- **Connect-time column introspection** resolves PG15–18 schema differences — including the PG17 `blk_read_time` → `shared_blk_read_time`/`local_blk_read_time` rename — and collects newer columns when present (NULL otherwise). Introspect the actual view columns rather than inferring from server version, because the extension version can lag the server version.
- `pg_read_all_stats` / `pg_monitor` enables full query text; degrade gracefully (counters still work; text shows "not permitted" vs "evicted" distinctly).
- Query text harvested once per queryid the tick it first appears, while pgss still holds it; persisted into the session, independent of pgss eviction.
- Ephemeral in-memory store built on a **serializable session struct**, so `--output` / `--report` persistence and richer output surfaces bolt on without touching the core.
- **Capability ladder** detected at connect: `pg_stat_statements` present is the floor (absent → tool cannot run, says so); `pg_read_all_stats` unlocks full text; PG16+ unlocks clean per-entry reset detection via `stats_since`; PG18 adds newer columns and tighter queryid grouping.

## v0 implementation stack (Rust)

- **clap** (derive) — argument parsing.
- **postgres** (sync rust-postgres) — single-connection driver. Chosen over `sqlx` because the SELECT is built dynamically from runtime column introspection, which defeats sqlx's compile-time-checked query macros. Sync chosen over `tokio-postgres` because the tool is one connection doing a sequential snapshot/sleep loop — no concurrency to exploit.
- **humantime** — parse `--time` values (`5m`, `2h`) via a clap value parser.
- **ctrlc** — SIGINT handler; flips a flag, the tick loop breaks to the render path (final snapshot first, not an immediate abort).
- **comfy-table** — final table rendering.
- **crossterm** — live preview repaint (cursor control), on stderr so piped stdout stays a clean artifact. Deliberately *not* ratatui — no TUI.
- **serde / serde_json** — `--format json` output; also makes the session struct serializable for later `--output`.

Persistence, when added, is **rusqlite**; the serializable session struct keeps it driver-agnostic.

## Deferred (TODO, non-blocking)

- **Reset/failover detection + graceful handling.** v0 floor is the per-key regression guard, which prevents corruption (cost: a small, unflagged undercount at a reset, plus the lost pre-reset seam). Deferred work: explicit detection via `pg_stat_statements_info.stats_reset` advancing; on a *detected global reset*, re-anchor all of `prev` (`prev = cur`) and skip banking that one interval; scope re-anchor to global resets only (per-key eviction guards just that key); mark reset timestamp in metadata and surface in the report; compute rates against summed-observed-elapsed rather than wall-clock. Behavior already decided: bank `acc` and continue.
- **"Have I seen enough" convergence signals** — designed against real captured sessions: calls-as-sample-size, reconstructed within-session stddev → mean ± CI, rank-stability (Kendall tau over top-N), exec-time-share stability, new-queryids-per-tick decay, captured-span + representativeness caveat.
- **On-disk persistence** (`--output` / `--report`), **HTML report**, **`--order-by`** flags, per-tick rate columns.

## Success criteria

An operator points the tool at a CNPG or plain Postgres cluster with zero config change and, after a bounded capture, gets an accurate ranked profile of where query time went — something that otherwise requires pgbadger plus log configuration they may not be able to enable.
