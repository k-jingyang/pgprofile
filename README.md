# pgprofile

An on-demand PostgreSQL query profiler built on `pg_stat_statements`.

pgprofile runs for a fixed capture window you control: start it, run your
workload, stop it, and it reconstructs pgbadger-style "where did query time go"
reports without log parsing, config changes, or superuser access. It works
the same way on CloudNativePG (CNPG) and traditional installs.

## Why

pgbadger-style profiling depends on log parsing via `log_min_duration_statement`.
CNPG complicates this since logs are usually piped to a central log store. On traditional installations,
it means config changes, file copying, and operational overhead. pgprofile instead
samples `pg_stat_statements` on an interval and accumulates deltas, with no
preload beyond the extension and no superuser.

## Install

Download a prebuilt binary from the [releases page](https://github.com/k-jingyang/pgprofile/releases),
or install via the shell installer:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/k-jingyang/pgprofile/releases/latest/download/pgprofile-installer.sh | sh
```

Or build from source:

```sh
cargo install --path .
```

## Requirements

- The target database has the `pg_stat_statements` extension installed.
- A connection string pgprofile can use for a single connection (`postgresql://user:pass@host:port/dbname`).
- `pg_read_all_stats` (or `pg_monitor`) membership unlocks full query text for
  queries run by other roles; without it, pgprofile still profiles counters
  for every query, just with limited text visibility into others' queries.

## Usage

### `info`: check what's available on a cluster

```sh
pgprofile info "postgresql://user:pass@host:5432/dbname"
```

Reports server version, whether `pg_stat_statements` is installed, whether
the connecting role can see all sessions' stats, and which columns are
available (this varies across PG15–18).

### `query`: one-shot snapshot

```sh
pgprofile query "postgresql://user:pass@host:5432/dbname" --rows 20
```

Dumps a single snapshot of `pg_stat_statements`, useful for a quick look or to
confirm connectivity/columns before a full profiling run. Pass `--columns` to
just list the columns pgprofile detected.

### `profile`: capture and report

```sh
pgprofile profile "postgresql://user:pass@host:5432/dbname" \
  --duration 5m \
  --interval 2s \
  --top 20 \
  --format table
```

Samples `pg_stat_statements` on `--interval` until `--duration` elapses or
Ctrl-C is pressed, printing a live top-N preview to stderr as it goes. On
exit it takes a final snapshot, banks it, and renders a ranked report to
stdout: `table` for a TTY-friendly view, or `tsv`/`json` for piping into
other tools.

| Flag | Default | Description |
|---|---|---|
| `--duration` | none (Ctrl-C to stop) | Capture length, e.g. `5m`, `30s` |
| `--interval` | `2s` | Snapshot interval |
| `--top` | `20` | Number of queries shown in the preview and final report |
| `--format` | `table` | `table`, `tsv`, or `json` |

Report columns: total exec time and % share, calls, mean exec time, rows,
cache-hit ratio, temp spill, WAL bytes.

## Non-goals

pgprofile is not a daemon or continuous monitor, doesn't show live sessions or
kill queries (use `pg_activity` for that), doesn't compute percentiles or
stddev (`pg_stat_statements` can't support them windowed), and doesn't do log
parsing.

See [`docs/pgprofile-PRD.md`](docs/pgprofile-PRD.md) for the full design
rationale.

## License

[PostgreSQL License](LICENSE).
