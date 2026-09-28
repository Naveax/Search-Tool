use search_core::{BuildOptions, IndexBuilder, InputRecord, SearchStore};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next();
    if mode.as_deref() == Some("--measure") {
        let path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index path"))?;
        return measure_only(&path);
    }
    if mode.as_deref() == Some("--query") {
        let path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing index path"))?;
        let query = args
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing query"))?;
        let rounds = args.next().and_then(|v| v.parse().ok()).unwrap_or(100);
        let limit = args.next().and_then(|v| v.parse().ok()).unwrap_or(64);
        return measure_query(&path, &query, rounds, limit);
    }

    let records = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100_000)
        .clamp(10_000, 10_000_000);
    let path = temp_index(records);
    let rss_before = rss_kib();
    let started = Instant::now();
    let mut builder = IndexBuilder::create(
        &path,
        BuildOptions {
            sort_chunk_entries: 131_072,
        },
    )?;
    builder.push(InputRecord {
        file_id: 1,
        parent_id: 1,
        size_bytes: 0,
        flags: search_core::FLAG_DIRECTORY,
        name: "root",
    })?;
    for id in 2..=records {
        let name = synthetic_name(id);
        builder.push(InputRecord {
            file_id: id,
            parent_id: 1,
            size_bytes: (id % 8_388_608) + 32,
            flags: 0,
            name: &name,
        })?;
    }
    let stats = builder.finish()?;
    let build_ms = started.elapsed().as_millis();
    let rss_after_build = rss_kib();

    println!("records={records}");
    println!("build_ms={build_ms}");
    println!("index_bytes={}", stats.index_bytes);
    println!("name_index_bytes={}", stats.name_index_bytes);
    println!("id_index_bytes={}", stats.id_index_bytes);
    println!(
        "total_index_bytes={}",
        stats.index_bytes + stats.name_index_bytes + stats.id_index_bytes
    );
    println!("string_pool_bytes={}", stats.string_pool_bytes);
    println!("rss_before_build_kib={}", display_opt(rss_before));
    println!("rss_after_build_kib={}", display_opt(rss_after_build));

    // Measure opening/searching in a fresh process. This avoids contaminating the
    // idle/read-side RSS with the builder's allocator arenas and dirty-page state.
    let exe = std::env::current_exe()?;
    let output = Command::new(exe).arg("--measure").arg(&path).output()?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        cleanup_family(&path);
        return Err(io::Error::other(format!(
            "measure subprocess failed: {message}"
        )));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));

    cleanup_family(&path);
    Ok(())
}

fn measure_only(path: &Path) -> io::Result<()> {
    let rss_before_open = rss_kib();
    let open_started = Instant::now();
    let mut store = SearchStore::open(path)?;
    let open_us = open_started.elapsed().as_micros();
    let rss_after_open = rss_kib();

    let rounds = 250_u64;
    let query_started = Instant::now();
    let mut total_hits = 0usize;
    for round in 0..rounds {
        let query = match round % 4 {
            0 => "node",
            1 => "package",
            2 => "report_",
            _ => "project_",
        };
        total_hits += store.search_prefix(query, 64)?.len();
    }
    let avg_query_us = query_started.elapsed().as_micros() as f64 / rounds as f64;
    let rss_after_queries = rss_kib();

    println!("open_us={open_us}");
    println!("memory_hint_bytes={}", store.memory_hint_bytes());
    println!("rss_before_open_kib={}", display_opt(rss_before_open));
    println!("rss_after_open_kib={}", display_opt(rss_after_open));
    println!("rss_after_queries_kib={}", display_opt(rss_after_queries));
    println!("query_rounds={rounds}");
    println!("avg_prefix_query_us={avg_query_us:.2}");
    println!("total_hits={total_hits}");
    Ok(())
}

fn measure_query(path: &Path, query: &str, rounds: u64, limit: usize) -> io::Result<()> {
    let mut store = SearchStore::open(path)?;
    let started = Instant::now();
    let mut hits = 0usize;
    for _ in 0..rounds.max(1) {
        hits += store.search_prefix(query, limit.max(1))?.len();
    }
    let elapsed = started.elapsed();
    println!("query={query}");
    println!("rounds={}", rounds.max(1));
    println!("limit={}", limit.max(1));
    println!("total_hits={hits}");
    println!(
        "avg_query_us={:.2}",
        elapsed.as_micros() as f64 / rounds.max(1) as f64
    );
    Ok(())
}

fn synthetic_name(id: u64) -> String {
    match id % 10_000 {
        0 => "node.exe".to_string(),
        1 => "package.json".to_string(),
        2 => "package-lock.json".to_string(),
        3 => "npm-cache.log".to_string(),
        4 => format!("project_{id:08}.rs"),
        5 => format!("report_{id:08}.txt"),
        _ => format!("file_{id:08}.dat"),
    }
}

fn temp_index(records: u64) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("search-tool-bench-{records}-{n}.stidx"))
}

fn cleanup_family(path: &Path) {
    for suffix in ["", ".names", ".ids", ".ncp", ".icp"] {
        let mut value = path.as_os_str().to_os_string();
        value.push(suffix);
        let _ = fs::remove_file(PathBuf::from(value));
    }
}

fn rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(value) = line.strip_prefix("VmRSS:") {
                return value
                    .split_whitespace()
                    .next()
                    .and_then(|value| value.parse().ok());
            }
        }
        None
    }
    #[cfg(target_os = "windows")]
    {
        windows_working_set_kib()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "windows")]
fn windows_working_set_kib() -> Option<u64> {
    use std::ffi::c_void;
    use std::mem::size_of;

    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "GetCurrentProcess"]
        fn get_current_process() -> *mut c_void;
        #[link_name = "K32GetProcessMemoryInfo"]
        fn get_process_memory_info(
            process: *mut c_void,
            counters: *mut ProcessMemoryCountersEx,
            cb: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCountersEx {
        cb: size_of::<ProcessMemoryCountersEx>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
        private_usage: 0,
    };
    let ok = unsafe {
        get_process_memory_info(
            get_current_process(),
            &mut counters,
            size_of::<ProcessMemoryCountersEx>() as u32,
        )
    };
    (ok != 0).then_some((counters.working_set_size as u64) / 1024)
}

fn display_opt(value: Option<u64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |value| value.to_string())
}
