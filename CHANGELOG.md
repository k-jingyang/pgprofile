# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-05

### Added
- `info` subcommand reporting server version, `pg_stat_statements` presence, and a capability ladder (columns available, `pg_read_all_stats` membership).
- `query` subcommand for a one-shot dynamic snapshot of `pg_stat_statements`, with a guard against rows carrying a NULL `queryid`.
- `profile` subcommand: a capture loop with live terminal preview and a final report, driven by an accumulator that guards against regressions on eviction.
- Report renderer with table, TSV, and JSON output formats.
