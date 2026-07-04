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
                    wal_bytes: cur_row.wal_bytes.zip(prev_row.wal_bytes).map(|(c, p)| c - p),
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

    fn make_row_with_wal(queryid: i64, calls: i64, wal_bytes: i64) -> Row {
        Row { wal_bytes: Some(wal_bytes), ..make_row(queryid, calls, 0.0) }
    }

    #[test]
    fn test_accumulates_delta() {
        let mut acc = Acc::new();
        let prev = rows_to_map(vec![make_row(1, 10, 100.0)]);
        let cur = rows_to_map(vec![make_row(1, 15, 150.0)]);
        acc.tick(&prev, &cur);
        let key = (10, 20, 1, true);
        assert_eq!(acc.entries[&key].calls, 5);
        assert!((acc.entries[&key].total_exec_time - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_wal_bytes_accumulates_as_delta_not_raw_cumulative() {
        let mut acc = Acc::new();
        let key = (10, 20, 1, true);

        // Baseline: query already has 1000 cumulative wal_bytes before capture starts.
        let mut prev = rows_to_map(vec![make_row_with_wal(1, 1, 1000)]);

        // Tick 1: cumulative rises to 1500 — true new WAL this tick is 500.
        let cur1 = rows_to_map(vec![make_row_with_wal(1, 2, 1500)]);
        acc.tick(&prev, &cur1);
        prev = cur1;

        // Tick 2: no new calls, cumulative unchanged — true new WAL this tick is 0.
        let cur2 = rows_to_map(vec![make_row_with_wal(1, 2, 1500)]);
        acc.tick(&prev, &cur2);

        assert_eq!(acc.entries[&key].wal_bytes, Some(500));
    }

    #[test]
    fn test_regression_guard_on_eviction() {
        let mut acc = Acc::new();
        let prev1 = rows_to_map(vec![make_row(1, 10, 100.0)]);
        let cur1 = rows_to_map(vec![make_row(1, 15, 150.0)]);
        acc.tick(&prev1, &cur1);
        // cur < prev: eviction + reinsert — use cur as delta
        let prev2 = cur1;
        let cur2 = rows_to_map(vec![make_row(1, 3, 30.0)]);
        acc.tick(&prev2, &cur2);
        let key = (10, 20, 1, true);
        // 5 (first delta) + 3 (regression guard)
        assert_eq!(acc.entries[&key].calls, 8);
        assert!((acc.entries[&key].total_exec_time - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_new_entry_mid_session() {
        let mut acc = Acc::new();
        let prev = rows_to_map(vec![make_row(1, 10, 100.0)]);
        let cur = rows_to_map(vec![make_row(1, 12, 120.0), make_row(2, 5, 50.0)]);
        acc.tick(&prev, &cur);
        let key2 = (10, 20, 2, true);
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
        assert_eq!(top[0].0 .2, 1); // queryid=1 highest
        assert_eq!(top[1].0 .2, 3); // queryid=3 second
    }
}
