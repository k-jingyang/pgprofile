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

    if tick > 0 {
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
