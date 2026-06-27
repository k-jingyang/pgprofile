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
            temp_blks: e.temp_blks_read + e.temp_blks_written,
            wal_bytes: e.wal_bytes,
            query: texts.get(&key.2).cloned().unwrap_or_default(),
        }
    }).collect();
    writeln!(out, "{}", serde_json::to_string_pretty(&rows).unwrap_or_default()).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acc::AccEntry;

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
