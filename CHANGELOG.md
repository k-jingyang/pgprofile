# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-07-07

### Added
- `x86_64-unknown-linux-musl` release target, for statically-linked Linux binaries (e.g. Alpine).

## [0.1.1] - 2026-07-05

### Fixed
- `profile` report's `wal_bytes` was accumulating the raw cumulative counter from `pg_stat_statements` on every tick instead of the per-tick delta, inflating reported WAL bytes for any query captured across multiple ticks.

## [0.1.0] - 2026-07-05

### Added
- `info` subcommand reporting server version, `pg_stat_statements` presence, and a capability ladder (columns available, `pg_read_all_stats` membership).
- `query` subcommand for a one-shot dynamic snapshot of `pg_stat_statements`, with a guard against rows carrying a NULL `queryid`.
- `profile` subcommand: a capture loop with live terminal preview and a final report, driven by an accumulator that guards against regressions on eviction.
- Report renderer with table, TSV, and JSON output formats.
