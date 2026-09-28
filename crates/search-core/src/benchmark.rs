use crate::store::{
    id_checkpoints_path, ids_path, name_checkpoints_path, names_path, BuildOptions, IndexBuilder,
    InputRecord, SearchStore,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticBenchReport {
    pub records: u64,
    pub build_time: Duration,
    pub open_time: Duration,
    pub exact_queries: u64,
    pub exact_total: Duration,
    pub prefix_queries: u64,
    pub prefix_total: Duration,
    pub index_bytes: u64,
    pub sidecar_bytes: u64,
    pub memory_hint_bytes: usize,
}

impl SyntheticBenchReport {
    pub fn exact_avg_micros(self) -> u128 {
        if self.exact_queries == 0 {
            0
        } else {
            self.exact_total.as_micros() / self.exact_queries as u128
        }
    }

    pub fn prefix_avg_micros(self) -> u128 {
        if self.prefix_queries == 0 {
            0
        } else {
            self.prefix_total.as_micros() / self.prefix_queries as u128
        }
    }
}

pub fn run_synthetic_benchmark(records: u64, queries: u64) -> io::Result<SyntheticBenchReport> {
    let records = records.clamp(1, 10_000_000);
    let queries = queries.clamp(1, 100_000);
    let path = temporary_index_path();

    let result = (|| -> io::Result<SyntheticBenchReport> {
        let build_started = Instant::now();
        let mut builder = IndexBuilder::create(
            &path,
            BuildOptions {
                // Keep the benchmark representative of the low-RAM production path.
                sort_chunk_entries: 32_768,
            },
        )?;
        for i in 0..records {
            let name = synthetic_name(i);
            let parent_id = if i < 256 { 0 } else { (i % 255) + 1 };
            builder.push(InputRecord {
                file_id: i + 1,
                parent_id,
                size_bytes: (i.wrapping_mul(4099)) & ((1_u64 << 34) - 1),
                flags: if i % 97 == 0 {
                    crate::store::FLAG_DIRECTORY
                } else {
                    0
                },
                name: &name,
            })?;
        }
        let stats = builder.finish()?;
        let build_time = build_started.elapsed();

        let open_started = Instant::now();
        let mut store = SearchStore::open(&path)?;
        let open_time = open_started.elapsed();
        let memory_hint_bytes = store.memory_hint_bytes();

        let exact_started = Instant::now();
        for i in 0..queries {
            let target = (i.wrapping_mul(104_729)) % records;
            let name = synthetic_name(target);
            let _ = store.search_exact(&name, 8)?;
        }
        let exact_total = exact_started.elapsed();

        let prefix_started = Instant::now();
        for i in 0..queries {
            let shard = (i.wrapping_mul(31)) % 4096;
            let query = format!("file-{shard:04x}-");
            let _ = store.search_prefix(&query, 32)?;
        }
        let prefix_total = prefix_started.elapsed();

        Ok(SyntheticBenchReport {
            records,
            build_time,
            open_time,
            exact_queries: queries,
            exact_total,
            prefix_queries: queries,
            prefix_total,
            index_bytes: stats.index_bytes,
            sidecar_bytes: stats.name_index_bytes.saturating_add(stats.id_index_bytes),
            memory_hint_bytes,
        })
    })();

    cleanup_index_family(&path);
    result
}

fn synthetic_name(i: u64) -> String {
    // Shared prefixes stress the prefix index more than fully random filenames would.
    let shard = i % 4096;
    format!("file-{shard:04x}-{i:010}.dat")
}

fn temporary_index_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("search-tool-bench-{stamp}.stidx"))
}

fn cleanup_index_family(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(names_path(path));
    let _ = fs::remove_file(ids_path(path));
    let _ = fs::remove_file(name_checkpoints_path(path));
    let _ = fs::remove_file(id_checkpoints_path(path));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_benchmark_exercises_disk_first_store() {
        let report = run_synthetic_benchmark(2_000, 8).unwrap();
        assert_eq!(report.records, 2_000);
        assert_eq!(report.exact_queries, 8);
        assert!(report.index_bytes > 0);
        assert!(report.sidecar_bytes > 0);
        assert!(report.memory_hint_bytes < 1024 * 1024);
    }
}
