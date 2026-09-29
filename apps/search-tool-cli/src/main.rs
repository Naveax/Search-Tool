#[cfg(windows)]
use search_core::parse_google_custom_search_json;
use search_core::{
    attribute_index_path, content_path, content_terms, parse_search_query, query_subject,
    rule_intent, sidecar_is_fresh, web_cache_path, AppConfig, AttributeIndex, ContentIndex,
    Decision, LiveSearchStore, MultiLiveSearchStore, QueryIntent, ResourceGovernor,
    TinyIntentModel, WebCache, WebLookupQuery, WorkClass, DEFAULT_WEB_CACHE_LIMIT_BYTES,
};
#[cfg(windows)]
use search_core::{capture_index_generation, pending_delta, write_sidecar_generation};
#[cfg(windows)]
use search_core::{is_text_candidate, ContentIndexBuilder, SearchStore};
use search_platform_windows::WindowsResourceProbe;
use std::{env, path::PathBuf, process::ExitCode, thread, time::Duration};

fn main() -> ExitCode {
    #[cfg(windows)]
    if env::var_os("SEARCH_TOOL_BACKGROUND").is_some() {
        let _ = search_platform_windows::enter_process_background_mode();
    }
    match env::args().nth(1).as_deref() {
        Some("status") | None => status(),
        Some("watch") => watch(),
        Some("ntfs-status") => ntfs_status(env::args().nth(2).as_deref()),
        Some("mft-count") => mft_count(env::args().nth(2).as_deref()),
        Some("index") => build_index(env::args().nth(2).as_deref(), env::args().nth(3).as_deref()),
        Some("search") => {
            search_index(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("fuzzy") => fuzzy_search(env::args().nth(2).as_deref(), env::args().nth(3).as_deref()),
        Some("related") => {
            related_search(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("content-build") => {
            content_build(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("content-build-rich") => content_build_rich(
            env::args().nth(2).as_deref(),
            env::args().nth(3).as_deref(),
            env::args().nth(4).as_deref(),
        ),
        Some("content-search") => {
            content_search(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("metadata-build") => {
            metadata_build(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("duplicates") => duplicates_report(
            env::args().nth(2).as_deref(),
            env::args().nth(3).as_deref(),
            env::args().nth(4).as_deref(),
        ),
        Some("cleanup") => cleanup_analyze(env::args().nth(2).as_deref()),
        Some("quarantine") => {
            quarantine_command(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("quarantine-list") => quarantine_list_command(env::args().nth(2).as_deref()),
        Some("restore") => {
            restore_command(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("purge") => {
            purge_command(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("route") => route_query(env::args().nth(2).as_deref(), env::args().nth(3).as_deref()),
        Some("web-lookup") => {
            web_lookup(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("smart") => smart_search(
            env::args().nth(2).as_deref(),
            env::args().nth(3).as_deref(),
            env::args().nth(4).as_deref(),
        ),
        Some("sync") => sync_index(env::args().nth(2).as_deref(), env::args().nth(3).as_deref()),
        Some("compact") => compact_command(env::args().nth(2).as_deref()),
        Some("verify") => verify_command(env::args().nth(2).as_deref()),
        Some("verify-deep") => verify_deep_command(env::args().nth(2).as_deref()),
        Some("repair") => repair_command(env::args().nth(2).as_deref()),
        Some("maintain") => maintain_command(env::args().nth(2).as_deref()),
        Some("doctor") => doctor_command(env::args().nth(2).as_deref()),
        Some("bench") => {
            bench_command(env::args().nth(2).as_deref(), env::args().nth(3).as_deref())
        }
        Some("version") | Some("--version") | Some("-V") => {
            println!("search-tool 0.1.0");
            ExitCode::SUCCESS
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn status() -> ExitCode {
    let cfg = AppConfig::default();
    let governor = ResourceGovernor::new(cfg.governor);
    let mut probe = WindowsResourceProbe::new();

    let _ = probe.sample();
    thread::sleep(Duration::from_millis(120));
    let sample = probe.sample();

    println!("Search Tool");
    println!("cpu_busy_percent={:.1}", sample.cpu_busy_percent);
    println!("memory_load_percent={}", sample.memory_load_percent);
    println!("user_idle_ms={}", sample.user_idle_ms);
    println!("on_battery={}", sample.on_battery);
    println!(
        "background_index={:?}",
        governor.decide(WorkClass::BackgroundIndex, sample)
    );
    println!("idle_ram_soft_limit_mb={}", cfg.idle_ram_soft_limit_mb);
    println!("idle_ram_hard_limit_mb={}", cfg.idle_ram_hard_limit_mb);

    ExitCode::SUCCESS
}

fn watch() -> ExitCode {
    let cfg = AppConfig::default();
    let governor = ResourceGovernor::new(cfg.governor);
    let mut probe = WindowsResourceProbe::new();
    let _ = probe.sample();

    loop {
        thread::sleep(Duration::from_millis(governor.busy_sample_ms()));
        let sample = probe.sample();
        let decision = governor.decide(WorkClass::BackgroundIndex, sample);

        println!(
            "cpu={:5.1}% mem={:3}% idle={:6}ms battery={} background={:?}",
            sample.cpu_busy_percent,
            sample.memory_load_percent,
            sample.user_idle_ms.min(999_999),
            sample.on_battery,
            decision
        );

        if decision == Decision::Run {
            thread::sleep(Duration::from_millis(governor.idle_sample_ms()));
        }
    }
}

#[cfg(windows)]
fn ntfs_status(arg: Option<&str>) -> ExitCode {
    use search_platform_windows::NtfsVolume;

    let drive = match parse_drive_letter(arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let volume = match NtfsVolume::open_drive(drive) {
        Ok(volume) => volume,
        Err(error) => {
            eprintln!("failed to open {drive}: {error}");
            eprintln!("Run the terminal as Administrator and use an NTFS volume.");
            return ExitCode::FAILURE;
        }
    };

    match volume.query_journal() {
        Ok(journal) => {
            println!("drive={}:", volume.drive_letter());
            println!("journal_id={}", journal.journal_id);
            println!("first_usn={}", journal.first_usn);
            println!("next_usn={}", journal.next_usn);
            println!("lowest_valid_usn={}", journal.lowest_valid_usn);
            println!("maximum_size={}", journal.maximum_size);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("USN journal query failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn ntfs_status(_arg: Option<&str>) -> ExitCode {
    eprintln!("ntfs-status is only available on Windows");
    ExitCode::FAILURE
}

#[cfg(windows)]
fn mft_count(arg: Option<&str>) -> ExitCode {
    use search_platform_windows::NtfsVolume;
    use std::time::Instant;

    let drive = match parse_drive_letter(arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let volume = match NtfsVolume::open_drive(drive) {
        Ok(volume) => volume,
        Err(error) => {
            eprintln!("failed to open {drive}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let started = Instant::now();
    let result = volume.enumerate_mft(|_record| Ok(()));
    match result {
        Ok(stats) => {
            let elapsed = started.elapsed();
            println!("records={}", stats.records);
            println!("batches={}", stats.batches);
            println!("elapsed_ms={}", elapsed.as_millis());
            if elapsed.as_secs_f64() > 0.0 {
                println!(
                    "records_per_second={:.0}",
                    stats.records as f64 / elapsed.as_secs_f64()
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("MFT enumeration failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn mft_count(_arg: Option<&str>) -> ExitCode {
    eprintln!("mft-count is only available on Windows");
    ExitCode::FAILURE
}

#[cfg(windows)]
fn parse_drive_letter(value: Option<&str>) -> Result<char, &'static str> {
    let value = value
        .ok_or("missing drive, example: search-tool ntfs-status C:")?
        .trim();
    let bytes = value.as_bytes();
    if !(bytes.len() == 1 || (bytes.len() == 2 && bytes[1] == b':')) {
        return Err("drive must be a single letter, optionally followed by ':'");
    }
    let drive = value.chars().next().ok_or("empty drive")?;
    if !drive.is_ascii_alphabetic() {
        return Err("drive must start with A-Z");
    }
    Ok(drive.to_ascii_uppercase())
}
#[cfg(windows)]
fn ensure_source_volume_available(drive: char) -> Result<(), String> {
    let root = format!("{drive}:\\");
    match std::fs::metadata(&root) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(format!("source volume {drive}: is not a directory")),
        Err(error) => Err(format!("source volume {drive}: is unavailable: {error}")),
    }
}

#[cfg(windows)]
fn build_index(drive_arg: Option<&str>, path_arg: Option<&str>) -> ExitCode {
    use std::path::Path;
    use std::time::Instant;

    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(path_arg) = path_arg else {
        eprintln!("missing index path, example: search-tool index C: C:\\ProgramData\\SearchTool\\c.stidx");
        return ExitCode::from(2);
    };
    let index_path = Path::new(path_arg);
    let started = Instant::now();
    match search_platform_windows::rebuild_index(drive, index_path) {
        Ok(stats) => {
            println!("drive={drive}:");
            println!("records={}", stats.store.records);
            println!("mft_batches={}", stats.enumeration.batches);
            println!("index_bytes={}", stats.store.index_bytes);
            println!("name_index_bytes={}", stats.store.name_index_bytes);
            println!("id_index_bytes={}", stats.store.id_index_bytes);
            println!("catchup_records={}", stats.catchup.records);
            println!("catchup_batches={}", stats.catchup.batches);
            println!("caught_up={}", stats.catchup.caught_up);
            println!("elapsed_ms={}", started.elapsed().as_millis());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("index build failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn build_index(_drive_arg: Option<&str>, _path_arg: Option<&str>) -> ExitCode {
    eprintln!("index is only available on Windows/NTFS");
    ExitCode::FAILURE
}

fn search_index(path_arg: Option<&str>, query_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("missing index path");
        return ExitCode::from(2);
    };
    let Some(query) = query_arg else {
        eprintln!("missing search query");
        return ExitCode::from(2);
    };
    let parsed = parse_search_query(query);
    if !parsed.rejected_filters.is_empty() {
        eprintln!(
            "ignored_invalid_filters={}",
            parsed.rejected_filters.join(",")
        );
    }
    if parsed.text.is_empty() && parsed.filters.is_empty() {
        eprintln!("query has no searchable text or valid filters");
        return ExitCode::from(2);
    }

    let target = std::path::Path::new(path);
    if target.is_dir() {
        let mut store = match MultiLiveSearchStore::open_index_directory(target) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index directory: {error}");
                return ExitCode::FAILURE;
            }
        };
        if parsed.filters.needs_attributes() {
            let stale = store.stale_attribute_volumes();
            if !stale.is_empty() {
                eprintln!("attribute metadata is stale for volumes={}; rebuild metadata before using size/date filters", stale.iter().collect::<String>());
                return ExitCode::from(3);
            }
        }
        const FILTER_ONLY_SCAN_BUDGET: usize = 100_000;
        let scan_budget = if parsed.text.is_empty() {
            FILTER_ONLY_SCAN_BUDGET
        } else {
            8_192
        };
        let hits = match store.search_filtered(&parsed, 50, scan_budget) {
            Ok(hits) => hits,
            Err(error) => {
                eprintln!("search failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        print_multi_hits(&mut store, hits);
        return ExitCode::SUCCESS;
    }

    let mut store = match LiveSearchStore::open(path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut attributes = if parsed.filters.needs_attributes() {
        let sidecar = attribute_index_path(path);
        if !sidecar_is_fresh(path, &sidecar).unwrap_or(false) {
            eprintln!("attribute metadata is stale; run `search-tool compact {path}` then `search-tool metadata-build <DRIVE>: {path}`");
            return ExitCode::from(3);
        }
        match AttributeIndex::open(&sidecar) {
            Ok(index) => Some(index),
            Err(error) => {
                eprintln!(
                    "attribute filters require metadata sidecar (run `search-tool metadata-build C: {path}`): {error}"
                );
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };

    const FILTER_ONLY_SCAN_BUDGET: usize = 100_000;
    let scan_budget = if parsed.text.is_empty() {
        FILTER_ONLY_SCAN_BUDGET
    } else {
        8_192
    };
    if parsed.text.is_empty() && store.base_record_count() > scan_budget as u64 {
        eprintln!(
            "filter_only_scan_capped={} total_base_records={} hint=add_text_to_narrow",
            scan_budget,
            store.base_record_count()
        );
    }

    let hits = match store.search_filtered(&parsed, attributes.as_mut(), 50, scan_budget) {
        Ok(hits) => hits,
        Err(error) => {
            eprintln!("search failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    for hit in hits {
        match store.reconstruct_path(&hit, 256) {
            Ok(path) => println!("{path}"),
            Err(_) => println!("{}", hit.name),
        }
    }
    ExitCode::SUCCESS
}

fn route_query(model_arg: Option<&str>, query_arg: Option<&str>) -> ExitCode {
    let (Some(model_path), Some(query)) = (model_arg, query_arg) else {
        eprintln!("usage: search-tool route MODEL QUERY");
        return ExitCode::from(2);
    };
    let model = match TinyIntentModel::load(model_path) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("failed to load tiny model: {error}");
            return ExitCode::FAILURE;
        }
    };
    let prediction = model.classify(query);
    println!("intent={:?}", prediction.intent);
    println!("confidence={}", prediction.confidence);
    println!("margin={}", prediction.margin);
    println!("subject={}", query_subject(query));
    println!("parameters={}", model.parameters());
    println!("resident_bytes={}", model.resident_bytes());
    ExitCode::SUCCESS
}

fn default_model_path() -> PathBuf {
    if let Some(path) = env::var_os("SEARCH_TOOL_MODEL") {
        return PathBuf::from(path);
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            for ancestor in parent.ancestors().take(3) {
                let candidate = ancestor.join("models").join("tiny-intent-v1.stm");
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("models").join("tiny-intent-v1.stm")
}

fn smart_search(
    index_arg: Option<&str>,
    query_arg: Option<&str>,
    model_arg: Option<&str>,
) -> ExitCode {
    let (Some(index), Some(query)) = (index_arg, query_arg) else {
        eprintln!("usage: search-tool smart INDEX QUERY [MODEL]");
        return ExitCode::from(2);
    };
    let intent = if let Some(intent) = rule_intent(query) {
        intent
    } else {
        let model_path = model_arg
            .map(PathBuf::from)
            .unwrap_or_else(default_model_path);
        let model = match TinyIntentModel::load(&model_path) {
            Ok(model) => model,
            Err(error) => {
                eprintln!(
                    "failed to load tiny model {}: {error}",
                    model_path.display()
                );
                return ExitCode::FAILURE;
            }
        };
        let prediction = model.classify(query);
        if prediction.confidence >= 65 {
            prediction.intent
        } else {
            QueryIntent::ExactSearch
        }
    };
    let subject = query_subject(query);
    match intent {
        QueryIntent::ContentSearch => {
            let terms = content_terms(query);
            content_search(Some(index), Some(&terms))
        }
        QueryIntent::RelatedSearch => related_search(Some(index), Some(&subject)),
        QueryIntent::FuzzySearch => fuzzy_search(Some(index), Some(&subject)),
        QueryIntent::CleanupAnalysis => cleanup_analyze(Some(index)),
        QueryIntent::WebLookup => web_lookup(Some(index), Some(&subject)),
        QueryIntent::Help => {
            print_help();
            ExitCode::SUCCESS
        }
        QueryIntent::ExactSearch | QueryIntent::Unknown => {
            search_index(Some(index), Some(&subject))
        }
    }
}

fn print_web_results(results: &[search_core::WebResult]) {
    for result in results {
        println!("{}", result.title);
        println!("  {}", result.link);
        if !result.snippet.is_empty() {
            println!("  {}", result.snippet.replace(['\r', '\n'], " "));
        }
    }
}

fn web_lookup(index_arg: Option<&str>, target_arg: Option<&str>) -> ExitCode {
    let (Some(index), Some(target)) = (index_arg, target_arg) else {
        eprintln!("usage: search-tool web-lookup INDEX TARGET");
        return ExitCode::from(2);
    };
    let query = WebLookupQuery::sanitized(target);
    if query.filename.is_empty() {
        eprintln!("web lookup target is empty after privacy sanitization");
        return ExitCode::from(2);
    }
    let cache = WebCache::new(web_cache_path(index), DEFAULT_WEB_CACHE_LIMIT_BYTES);
    match cache.get(query.cache_key()) {
        Ok(Some(results)) => {
            println!("source=cache");
            print_web_results(&results);
            return ExitCode::SUCCESS;
        }
        Ok(None) => {}
        Err(error) => eprintln!("web cache read skipped: {error}"),
    }

    let api_key = match env::var("SEARCH_TOOL_GOOGLE_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            eprintln!("web lookup is disabled: SEARCH_TOOL_GOOGLE_KEY is not configured");
            return ExitCode::from(3);
        }
    };
    let engine_id = match env::var("SEARCH_TOOL_GOOGLE_CX") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            eprintln!("web lookup is disabled: SEARCH_TOOL_GOOGLE_CX is not configured");
            return ExitCode::from(3);
        }
    };

    #[cfg(not(windows))]
    {
        let _ = (api_key, engine_id);
        eprintln!("native web lookup provider is only enabled in the Windows build");
        ExitCode::FAILURE
    }

    #[cfg(windows)]
    {
        let json = match search_platform_windows::google_custom_search_json(
            &api_key,
            &engine_id,
            &query.search_text(),
            5,
        ) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("web lookup failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        let results = parse_google_custom_search_json(&json, 5);
        if results.is_empty() {
            eprintln!("web lookup returned no parseable results");
            return ExitCode::from(4);
        }
        if let Err(error) = cache.put(query.cache_key(), &results) {
            eprintln!("web cache write skipped: {error}");
        }
        println!("source=web");
        print_web_results(&results);
        ExitCode::SUCCESS
    }
}

fn cleanup_analyze(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool cleanup INDEX_OR_DIR");
        return ExitCode::from(2);
    };
    let patterns = [
        "temp",
        "cache",
        "npm-cache",
        "node_modules",
        "__pycache__",
        "target",
        "build",
        "dist",
        ".log",
    ];
    let target = std::path::Path::new(path);
    if target.is_dir() {
        let mut store = match MultiLiveSearchStore::open_index_directory(target) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index directory: {error}");
                return ExitCode::FAILURE;
            }
        };
        let mut seen = std::collections::HashSet::new();
        let mut emitted = 0usize;
        for pattern in patterns {
            let Ok(hits) = store.search_prefix(pattern, 128) else {
                continue;
            };
            for hit in hits {
                if !seen.insert((hit.volume, hit.hit.file_id)) {
                    continue;
                }
                let Ok(full) = store.reconstruct_path(&hit, 256) else {
                    continue;
                };
                let class = search_core::classify_path(&full, hit.hit.flags);
                let decision = search_core::cleanup_decision(&full, class);
                if matches!(decision.action, search_core::CleanupAction::Keep) {
                    continue;
                }
                println!(
                    "{:?}\t{:?}\t{}\t{}",
                    decision.action, class.class, decision.confidence, full
                );
                emitted += 1;
                if emitted >= 500 {
                    break;
                }
            }
            if emitted >= 500 {
                break;
            }
        }
        return ExitCode::SUCCESS;
    }

    let mut store = match LiveSearchStore::open(path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut seen = std::collections::HashSet::new();
    let mut emitted = 0usize;
    for pattern in patterns {
        let Ok(hits) = store.search_prefix(pattern, 128) else {
            continue;
        };
        for hit in hits {
            if !seen.insert(hit.file_id) {
                continue;
            }
            let Ok(full) = store.reconstruct_path(&hit, 256) else {
                continue;
            };
            let class = search_core::classify_path(&full, hit.flags);
            let decision = search_core::cleanup_decision(&full, class);
            if matches!(decision.action, search_core::CleanupAction::Keep) {
                continue;
            }
            println!(
                "{:?}\t{:?}\t{}\t{}",
                decision.action, class.class, decision.confidence, full
            );
            emitted += 1;
            if emitted >= 500 {
                break;
            }
        }
        if emitted >= 500 {
            break;
        }
    }
    ExitCode::SUCCESS
}

fn print_live_hits(store: &mut LiveSearchStore, hits: Vec<search_core::LiveSearchHit>) {
    for hit in hits {
        match store.reconstruct_path(&hit, 256) {
            Ok(path) => println!("{path}"),
            Err(_) => println!("{}", hit.name),
        }
    }
}

fn print_multi_hits(store: &mut MultiLiveSearchStore, hits: Vec<search_core::VolumeSearchHit>) {
    for hit in hits {
        match store.reconstruct_path(&hit, 256) {
            Ok(path) => println!("{path}"),
            Err(_) => println!("{}:\\{}", hit.volume, hit.hit.name),
        }
    }
}

fn fuzzy_search(path_arg: Option<&str>, query_arg: Option<&str>) -> ExitCode {
    let (Some(path), Some(query)) = (path_arg, query_arg) else {
        eprintln!("usage: search-tool fuzzy INDEX_OR_DIR QUERY");
        return ExitCode::from(2);
    };
    let target = std::path::Path::new(path);
    if target.is_dir() {
        let mut store = match MultiLiveSearchStore::open_index_directory(target) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index directory: {error}");
                return ExitCode::FAILURE;
            }
        };
        match store.search_fuzzy(query, 2, 50) {
            Ok(hits) => {
                print_multi_hits(&mut store, hits);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fuzzy search failed: {error}");
                ExitCode::FAILURE
            }
        }
    } else {
        let mut store = match LiveSearchStore::open(path) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index: {error}");
                return ExitCode::FAILURE;
            }
        };
        match store.search_fuzzy(query, 2, 50) {
            Ok(hits) => {
                print_live_hits(&mut store, hits);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("fuzzy search failed: {error}");
                ExitCode::FAILURE
            }
        }
    }
}

fn related_search(path_arg: Option<&str>, query_arg: Option<&str>) -> ExitCode {
    let (Some(path), Some(query)) = (path_arg, query_arg) else {
        eprintln!("usage: search-tool related INDEX_OR_DIR QUERY");
        return ExitCode::from(2);
    };
    let target = std::path::Path::new(path);
    if target.is_dir() {
        let mut store = match MultiLiveSearchStore::open_index_directory(target) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index directory: {error}");
                return ExitCode::FAILURE;
            }
        };
        match store.search_related(query, 100) {
            Ok(hits) => {
                print_multi_hits(&mut store, hits);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("related search failed: {error}");
                ExitCode::FAILURE
            }
        }
    } else {
        let mut store = match LiveSearchStore::open(path) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index: {error}");
                return ExitCode::FAILURE;
            }
        };
        match store.search_related(query, 100) {
            Ok(hits) => {
                print_live_hits(&mut store, hits);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("related search failed: {error}");
                ExitCode::FAILURE
            }
        }
    }
}

#[cfg(windows)]
fn prepare_sidecar_build(
    index_path: &std::path::Path,
) -> Result<search_core::IndexGeneration, ExitCode> {
    match pending_delta(index_path) {
        Ok(true) => {
            eprintln!("sidecar build refused: pending index delta exists; run `search-tool compact {}` first", index_path.display());
            Err(ExitCode::from(3))
        }
        Ok(false) => capture_index_generation(index_path).map_err(|error| {
            eprintln!("failed to capture index generation: {error}");
            ExitCode::FAILURE
        }),
        Err(error) => {
            eprintln!("failed to inspect pending delta: {error}");
            Err(ExitCode::FAILURE)
        }
    }
}

#[cfg(windows)]
fn content_build(drive_arg: Option<&str>, path_arg: Option<&str>) -> ExitCode {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    const MAX_FILE_BYTES: u64 = 1024 * 1024;
    const MAX_TOKENS_PER_FILE: usize = 4096;

    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(path_arg) = path_arg else {
        eprintln!("usage: search-tool content-build C: INDEX");
        return ExitCode::from(2);
    };
    let index_path = Path::new(path_arg);
    if let Err(message) = ensure_source_volume_available(drive) {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    let generation = match prepare_sidecar_build(index_path) {
        Ok(generation) => generation,
        Err(code) => return code,
    };
    let mut store = match SearchStore::open(index_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut builder = match ContentIndexBuilder::create(content_path(index_path)) {
        Ok(builder) => builder,
        Err(error) => {
            eprintln!("failed to create content index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let cfg = AppConfig::default();
    let governor = ResourceGovernor::new(cfg.governor);
    let mut probe = WindowsResourceProbe::new();
    let _ = probe.sample();
    let mut indexed_files = 0_u64;
    let mut skipped_files = 0_u64;
    let background = env::var_os("SEARCH_TOOL_BACKGROUND").is_some();

    for record_index in 0..store.record_count() {
        if background && record_index % 128 == 0 {
            loop {
                let sample = probe.sample();
                match governor.decide(WorkClass::BackgroundIndex, sample) {
                    Decision::Pause => thread::sleep(Duration::from_millis(750)),
                    Decision::Throttle => {
                        thread::sleep(Duration::from_millis(25));
                        break;
                    }
                    Decision::Run => break,
                }
            }
        }
        let record = match store.read_record(record_index) {
            Ok(r) => r,
            Err(_) => {
                skipped_files += 1;
                continue;
            }
        };
        if record.is_directory() {
            continue;
        }
        let name = match store.read_name(record) {
            Ok(n) => n,
            Err(_) => {
                skipped_files += 1;
                continue;
            }
        };
        if !is_text_candidate(&name) {
            continue;
        }
        let relative = match store.reconstruct_path(record_index, 256) {
            Ok(p) => p,
            Err(_) => {
                skipped_files += 1;
                continue;
            }
        };
        let full = if relative.len() >= 2 && relative.as_bytes()[1] == b':' {
            relative
        } else {
            format!("{drive}:\\{relative}")
        };
        let file = match File::open(&full) {
            Ok(f) => f,
            Err(_) => {
                skipped_files += 1;
                continue;
            }
        };
        let size = match file.metadata() {
            Ok(m) => m.len(),
            Err(_) => {
                skipped_files += 1;
                continue;
            }
        };
        if size > MAX_FILE_BYTES {
            skipped_files += 1;
            continue;
        }
        let mut bytes = Vec::with_capacity(size as usize);
        if file
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() as u64 > MAX_FILE_BYTES
        {
            skipped_files += 1;
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        if builder
            .add_text(record.file_id, &text, MAX_TOKENS_PER_FILE)
            .is_ok()
        {
            indexed_files += 1;
        } else {
            skipped_files += 1;
        }
    }
    match builder.finish() {
        Ok(postings) => {
            let sidecar = content_path(index_path);
            if let Err(error) = write_sidecar_generation(&sidecar, generation) {
                eprintln!("content freshness marker failed: {error}");
                return ExitCode::FAILURE;
            }
            println!("indexed_files={indexed_files}");
            println!("skipped_files={skipped_files}");
            println!("postings={postings}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("content index finalize failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn content_build(_drive_arg: Option<&str>, _path_arg: Option<&str>) -> ExitCode {
    eprintln!("content-build is only available on Windows in this build");
    ExitCode::FAILURE
}

#[cfg(windows)]
const MAX_WORKER_PATH_BYTES: usize = 32 * 1024;
#[cfg(windows)]
const MAX_WORKER_MEMORY_BYTES: usize = 256 * 1024 * 1024;

#[cfg(windows)]
fn worker_path_bytes(path: &str) -> std::io::Result<&[u8]> {
    let bytes = path.as_bytes();
    if bytes.len() > MAX_WORKER_PATH_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path exceeds worker protocol limit",
        ));
    }
    Ok(bytes)
}

#[cfg(windows)]
fn parser_worker_requires_restart(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::InvalidData
    )
}

#[cfg(windows)]
struct ParserWorkerClient {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: std::io::BufReader<std::process::ChildStdout>,
    watchdog_tx: std::sync::mpsc::Sender<WorkerWatchdogCommand>,
    watchdog_thread: Option<std::thread::JoinHandle<()>>,
    _job: WorkerJob,
}

#[cfg(windows)]
enum WorkerWatchdogCommand {
    Arm { pid: u32, timeout: Duration },
    Disarm,
    Shutdown,
}

#[cfg(windows)]
fn terminate_worker_process(pid: u32) {
    const PROCESS_TERMINATE: u32 = 0x0001;

    unsafe extern "system" {
        fn OpenProcess(
            desired_access: u32,
            inherit_handle: i32,
            process_id: u32,
        ) -> *mut std::ffi::c_void;
        fn TerminateProcess(process: *mut std::ffi::c_void, exit_code: u32) -> i32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }

    let process = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if process.is_null() {
        return;
    }
    unsafe {
        let _ = TerminateProcess(process, 0xE000_0001);
        let _ = CloseHandle(process);
    }
}

#[cfg(windows)]
struct WorkerJob {
    handle: *mut std::ffi::c_void,
}

#[cfg(windows)]
impl WorkerJob {
    fn assign(child: &std::process::Child, process_memory_limit: usize) -> std::io::Result<Self> {
        use std::os::windows::io::AsRawHandle;

        const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: u32 = 9;
        const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x0000_0100;
        const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;

        #[repr(C)]
        #[derive(Default)]
        struct JobObjectBasicLimitInformation {
            per_process_user_time_limit: i64,
            per_job_user_time_limit: i64,
            limit_flags: u32,
            minimum_working_set_size: usize,
            maximum_working_set_size: usize,
            active_process_limit: u32,
            affinity: usize,
            priority_class: u32,
            scheduling_class: u32,
        }

        #[repr(C)]
        #[derive(Default)]
        struct IoCounters {
            read_operation_count: u64,
            write_operation_count: u64,
            other_operation_count: u64,
            read_transfer_count: u64,
            write_transfer_count: u64,
            other_transfer_count: u64,
        }

        #[repr(C)]
        #[derive(Default)]
        struct JobObjectExtendedLimitInformation {
            basic_limit_information: JobObjectBasicLimitInformation,
            io_info: IoCounters,
            process_memory_limit: usize,
            job_memory_limit: usize,
            peak_process_memory_used: usize,
            peak_job_memory_used: usize,
        }

        unsafe extern "system" {
            fn CreateJobObjectW(
                job_attributes: *mut std::ffi::c_void,
                name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn SetInformationJobObject(
                job: *mut std::ffi::c_void,
                info_class: u32,
                info: *const std::ffi::c_void,
                info_len: u32,
            ) -> i32;
            fn AssignProcessToJobObject(
                job: *mut std::ffi::c_void,
                process: *mut std::ffi::c_void,
            ) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        let handle = unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        let mut limits = JobObjectExtendedLimitInformation::default();
        limits.basic_limit_information.limit_flags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        limits.process_memory_limit = process_memory_limit;

        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                (&limits as *const JobObjectExtendedLimitInformation).cast(),
                std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
            )
        };
        if configured == 0 {
            let error = std::io::Error::last_os_error();
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(error);
        }

        let assigned = unsafe { AssignProcessToJobObject(handle, child.as_raw_handle().cast()) };
        if assigned == 0 {
            let error = std::io::Error::last_os_error();
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(error);
        }

        Ok(Self { handle })
    }
}

#[cfg(windows)]
impl Drop for WorkerJob {
    fn drop(&mut self) {
        unsafe extern "system" {
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        if !self.handle.is_null() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(windows)]
fn spawn_worker_watchdog() -> (
    std::sync::mpsc::Sender<WorkerWatchdogCommand>,
    std::thread::JoinHandle<()>,
) {
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Instant;

    let (tx, rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let mut armed: Option<(u32, Instant)> = None;
        loop {
            let command = if let Some((pid, deadline)) = armed {
                let wait = deadline.saturating_duration_since(Instant::now());
                match rx.recv_timeout(wait) {
                    Ok(command) => Some(command),
                    Err(RecvTimeoutError::Timeout) => {
                        terminate_worker_process(pid);
                        armed = None;
                        None
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(command) => Some(command),
                    Err(_) => break,
                }
            };

            let Some(command) = command else {
                continue;
            };
            match command {
                WorkerWatchdogCommand::Arm { pid, timeout } => {
                    armed = Some((pid, Instant::now() + timeout));
                }
                WorkerWatchdogCommand::Disarm => armed = None,
                WorkerWatchdogCommand::Shutdown => break,
            }
        }
    });
    (tx, thread)
}

#[cfg(windows)]
impl ParserWorkerClient {
    fn spawn(executable: &str) -> std::io::Result<Self> {
        Self::spawn_with_args(executable, &["serve"])
    }

    fn spawn_with_args(executable: &str, args: &[&str]) -> std::io::Result<Self> {
        use std::process::{Command, Stdio};

        let mut child = Command::new(executable)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let job = match WorkerJob::assign(&child, MAX_WORKER_MEMORY_BYTES) {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::new(
                    error.kind(),
                    format!("failed to sandbox parser worker in Job Object: {error}"),
                ));
            }
        };
        let input = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("worker stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("worker stdout unavailable"))?;
        let (watchdog_tx, watchdog_thread) = spawn_worker_watchdog();
        Ok(Self {
            child,
            input,
            output: std::io::BufReader::new(stdout),
            watchdog_tx,
            watchdog_thread: Some(watchdog_thread),
            _job: job,
        })
    }

    fn extract(&mut self, path: &str, max_chars: u32) -> std::io::Result<String> {
        use std::io::{Read, Write};

        let bytes = worker_path_bytes(path)?;
        let response_timeout = Duration::from_secs(5);
        self.watchdog_tx
            .send(WorkerWatchdogCommand::Arm {
                pid: self.child.id(),
                timeout: response_timeout,
            })
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "worker watchdog unavailable",
                )
            })?;

        let result = (|| -> std::io::Result<String> {
            // Arm before writing the request. A hostile/stuck child that never
            // drains stdin must not be able to block write_all/flush forever.
            self.input.write_all(&(bytes.len() as u32).to_le_bytes())?;
            self.input.write_all(&max_chars.to_le_bytes())?;
            self.input.write_all(bytes)?;
            self.input.flush()?;

            let mut header = [0_u8; 8];
            self.output.read_exact(&mut header)?;
            let status = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
            let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
            if len > 8 * 1024 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "worker response too large",
                ));
            }
            let mut payload = vec![0_u8; len];
            self.output.read_exact(&mut payload)?;
            let text = String::from_utf8(payload).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "worker returned non-UTF8")
            })?;
            if status == 0 {
                Ok(text)
            } else {
                Err(std::io::Error::other(text))
            }
        })();
        let _ = self.watchdog_tx.send(WorkerWatchdogCommand::Disarm);
        result
    }
}

#[cfg(windows)]
impl Drop for ParserWorkerClient {
    fn drop(&mut self) {
        use std::io::Write;
        use std::time::Instant;

        let _ = self.input.write_all(&0_u32.to_le_bytes());
        let _ = self.input.flush();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
            }
        }
        let _ = self.watchdog_tx.send(WorkerWatchdogCommand::Shutdown);
        if let Some(watchdog) = self.watchdog_thread.take() {
            let _ = watchdog.join();
        }
    }
}

#[cfg(windows)]
fn rich_content_candidate(name: &str) -> bool {
    if is_text_candidate(name) {
        return true;
    }
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf" | "eml" | "mht" | "mhtml"
    )
}

#[cfg(windows)]
fn content_build_rich(
    drive_arg: Option<&str>,
    path_arg: Option<&str>,
    worker_arg: Option<&str>,
) -> ExitCode {
    use std::path::Path;
    const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_CHARS: u32 = 2 * 1024 * 1024;
    const MAX_TOKENS_PER_FILE: usize = 8192;

    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(index_arg) = path_arg else {
        eprintln!("usage: search-tool content-build-rich C: INDEX WORKER_EXE");
        return ExitCode::from(2);
    };
    let worker = worker_arg.unwrap_or("search-tool-worker.exe");
    let index_path = Path::new(index_arg);
    if let Err(message) = ensure_source_volume_available(drive) {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    let generation = match prepare_sidecar_build(index_path) {
        Ok(generation) => generation,
        Err(code) => return code,
    };
    let mut store = match SearchStore::open(index_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut parser = match ParserWorkerClient::spawn(worker) {
        Ok(worker) => worker,
        Err(error) => {
            eprintln!("failed to start parser worker: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut builder = match ContentIndexBuilder::create(content_path(index_path)) {
        Ok(builder) => builder,
        Err(error) => {
            eprintln!("failed to create content index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let governor = ResourceGovernor::new(AppConfig::default().governor);
    let mut probe = WindowsResourceProbe::new();
    let _ = probe.sample();
    let mut indexed = 0_u64;
    let mut skipped = 0_u64;
    let background = env::var_os("SEARCH_TOOL_BACKGROUND").is_some();

    for record_index in 0..store.record_count() {
        if background && record_index % 32 == 0 {
            loop {
                match governor.decide(WorkClass::ContentParse, probe.sample()) {
                    Decision::Pause => thread::sleep(Duration::from_millis(1000)),
                    Decision::Throttle => {
                        thread::sleep(Duration::from_millis(50));
                        break;
                    }
                    Decision::Run => break,
                }
            }
        }
        let record = match store.read_record(record_index) {
            Ok(record) if !record.is_directory() => record,
            Ok(_) => continue,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let name = match store.read_name(record) {
            Ok(name) if rich_content_candidate(&name) => name,
            Ok(_) => continue,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let _ = name;
        let relative = match store.reconstruct_path(record_index, 256) {
            Ok(path) => path,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let full = absolute_drive_path(drive, &relative);
        if std::fs::metadata(&full)
            .map(|m| m.len() > MAX_SOURCE_BYTES)
            .unwrap_or(true)
        {
            skipped += 1;
            continue;
        }
        let text = match parser.extract(&full, MAX_CHARS) {
            Ok(text) if !text.is_empty() => text,
            Ok(_) => {
                skipped += 1;
                continue;
            }
            Err(error) => {
                if parser_worker_requires_restart(error.kind()) {
                    let _ = parser.child.kill();
                    let _ = parser.child.wait();
                    parser = match ParserWorkerClient::spawn(worker) {
                        Ok(worker) => worker,
                        Err(restart_error) => {
                            eprintln!(
                                "parser worker restart failed after {error}: {restart_error}"
                            );
                            return ExitCode::FAILURE;
                        }
                    };
                }
                skipped += 1;
                continue;
            }
        };
        if builder
            .add_text(record.file_id, &text, MAX_TOKENS_PER_FILE)
            .is_ok()
        {
            indexed += 1;
        } else {
            skipped += 1;
        }
    }
    match builder.finish() {
        Ok(postings) => {
            let sidecar = content_path(index_path);
            if let Err(error) = write_sidecar_generation(&sidecar, generation) {
                eprintln!("content freshness marker failed: {error}");
                return ExitCode::FAILURE;
            }
            println!("indexed_files={indexed}");
            println!("skipped_files={skipped}");
            println!("postings={postings}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("rich content index finalize failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn content_build_rich(
    _drive_arg: Option<&str>,
    _path_arg: Option<&str>,
    _worker_arg: Option<&str>,
) -> ExitCode {
    eprintln!("content-build-rich is only available on Windows");
    ExitCode::FAILURE
}

#[cfg(windows)]
fn metadata_build(drive_arg: Option<&str>, path_arg: Option<&str>) -> ExitCode {
    use search_core::{
        attribute_index_path, size_index_path, AttributeEntry, AttributeIndexBuilder, IoClass,
        SizeEntry, SizeIndexBuilder,
    };
    use std::path::Path;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::sync_channel,
    };
    use std::time::UNIX_EPOCH;

    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(path_arg) = path_arg else {
        eprintln!("usage: search-tool metadata-build C: INDEX");
        return ExitCode::from(2);
    };
    let index_path = Path::new(path_arg);
    if let Err(message) = ensure_source_volume_available(drive) {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    let generation = match prepare_sidecar_build(index_path) {
        Ok(generation) => generation,
        Err(code) => return code,
    };
    let store = match SearchStore::open(index_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut builder = match SizeIndexBuilder::create(size_index_path(index_path), 131_072) {
        Ok(builder) => builder,
        Err(error) => {
            eprintln!("failed to create metadata sidecar: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut attributes =
        match AttributeIndexBuilder::create(attribute_index_path(index_path), 131_072) {
            Ok(builder) => builder,
            Err(error) => {
                eprintln!("failed to create attribute sidecar: {error}");
                return ExitCode::FAILURE;
            }
        };
    let io_class = search_platform_windows::NtfsVolume::open_drive(drive)
        .map(|volume| volume.storage_class())
        .unwrap_or(IoClass::Unknown);
    let worker_count = match io_class {
        IoClass::Hdd => 1_usize,
        IoClass::Ssd => 4_usize,
        IoClass::Unknown => 2_usize,
    };
    let record_count = store.record_count();
    drop(store);

    let background = env::var_os("SEARCH_TOOL_BACKGROUND").is_some();
    let next_record = AtomicU64::new(0);
    let skipped = AtomicU64::new(0);
    let mut indexed = 0_u64;
    let mut write_failed = false;

    std::thread::scope(|scope| {
        let (result_tx, result_rx) = sync_channel::<Result<(u64, u64, i64), String>>(8_192);

        for _ in 0..worker_count {
            let result_tx = result_tx.clone();
            let next_record = &next_record;
            let skipped = &skipped;
            scope.spawn(move || {
                let mut worker_store = match SearchStore::open(index_path) {
                    Ok(store) => store,
                    Err(error) => {
                        let _ = result_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                let governor = ResourceGovernor::new(AppConfig::default().governor);
                let mut probe = WindowsResourceProbe::new();
                let _ = probe.sample();

                loop {
                    let record_index = next_record.fetch_add(1, Ordering::Relaxed);
                    if record_index >= record_count {
                        break;
                    }
                    if background && record_index.is_multiple_of(256) {
                        loop {
                            match governor.decide(WorkClass::MetadataUpdate, probe.sample()) {
                                Decision::Pause => thread::sleep(Duration::from_millis(500)),
                                Decision::Throttle => {
                                    thread::sleep(Duration::from_millis(20));
                                    break;
                                }
                                Decision::Run => break,
                            }
                        }
                    }

                    let record = match worker_store.read_record(record_index) {
                        Ok(record) if !record.is_directory() => record,
                        Ok(_) => continue,
                        Err(_) => {
                            skipped.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };
                    let relative = match worker_store.reconstruct_path(record_index, 256) {
                        Ok(path) => path,
                        Err(_) => {
                            skipped.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };
                    let full = absolute_drive_path(drive, &relative);
                    let metadata = match std::fs::metadata(&full) {
                        Ok(metadata) if metadata.is_file() => metadata,
                        _ => {
                            skipped.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };
                    let size = metadata.len();
                    let modified_unix_secs = metadata
                        .modified()
                        .ok()
                        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                        .map(|value| value.as_secs().min(i64::MAX as u64) as i64)
                        .unwrap_or(0);
                    if result_tx
                        .send(Ok((record.file_id, size, modified_unix_secs)))
                        .is_err()
                    {
                        return;
                    }
                }
            });
        }
        drop(result_tx);

        for result in result_rx {
            let (file_id, size, modified_unix_secs) = match result {
                Ok(result) => result,
                Err(error) => {
                    eprintln!("metadata worker failed: {error}");
                    write_failed = true;
                    break;
                }
            };
            if builder
                .push(SizeEntry {
                    size_bytes: size,
                    file_id,
                })
                .is_err()
                || attributes
                    .push(AttributeEntry {
                        file_id,
                        size_bytes: size,
                        modified_unix_secs,
                        platform_attributes: 0,
                    })
                    .is_err()
            {
                eprintln!("metadata sidecar write failed");
                write_failed = true;
                break;
            }
            indexed += 1;
        }
    });

    let skipped = skipped.load(Ordering::Relaxed);
    if write_failed {
        return ExitCode::FAILURE;
    }

    let size_entries = match builder.finish() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("size sidecar finalize failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let attribute_entries = match attributes.finish() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("attribute sidecar finalize failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let size_sidecar = size_index_path(index_path);
    let attribute_sidecar = attribute_index_path(index_path);
    if let Err(error) = write_sidecar_generation(&size_sidecar, generation) {
        eprintln!("size metadata freshness marker failed: {error}");
        return ExitCode::FAILURE;
    }
    if let Err(error) = write_sidecar_generation(&attribute_sidecar, generation) {
        eprintln!("attribute metadata freshness marker failed: {error}");
        return ExitCode::FAILURE;
    }
    println!("indexed_files={indexed}");
    println!("skipped_files={skipped}");
    println!("size_entries={size_entries}");
    println!("attribute_entries={attribute_entries}");
    ExitCode::SUCCESS
}

#[cfg(not(windows))]
fn metadata_build(_drive_arg: Option<&str>, _path_arg: Option<&str>) -> ExitCode {
    eprintln!("metadata-build is only available on Windows");
    ExitCode::FAILURE
}

#[cfg(windows)]
fn duplicates_report(
    drive_arg: Option<&str>,
    path_arg: Option<&str>,
    min_bytes_arg: Option<&str>,
) -> ExitCode {
    use search_core::{
        exact_files_equal, full_fingerprint, is_protected_path, sample_fingerprint,
        size_index_path, IoClass, LowEndPolicy, SizeIndex,
    };
    use search_platform_windows::NtfsVolume;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    if let Some(first) = drive_arg {
        // A drive designator such as "D:" is also a valid Windows directory
        // path. Parse it as a drive before considering directory/multi-volume
        // mode, otherwise every mounted drive is misrouted.
        if parse_drive_letter(Some(first)).is_err() {
            let candidate = Path::new(first);
            if candidate.is_dir() {
                return duplicates_report_multi(candidate, path_arg);
            }
        }
    }

    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(index_path) = path_arg else {
        eprintln!(
            "usage: search-tool duplicates C: INDEX [MIN_BYTES] | duplicates INDEX_DIR [MIN_BYTES]"
        );
        return ExitCode::from(2);
    };
    let min_bytes = min_bytes_arg
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1024 * 1024);
    let size_sidecar = size_index_path(index_path);
    if !sidecar_is_fresh(index_path, &size_sidecar).unwrap_or(false) {
        eprintln!("size metadata is stale; run: search-tool compact {index_path} && search-tool metadata-build {drive}: {index_path}");
        return ExitCode::from(3);
    }
    let mut sizes = match SizeIndex::open(size_sidecar) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("size metadata unavailable: {error}");
            eprintln!("run: search-tool metadata-build {drive}: {index_path}");
            return ExitCode::FAILURE;
        }
    };
    let mut store = match LiveSearchStore::open(index_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut duplicate_sets = 0_u64;
    let mut reclaimable = 0_u64;
    let io_class = NtfsVolume::open_drive(drive)
        .map(|volume| volume.storage_class())
        .unwrap_or(IoClass::Unknown);
    let low_end = LowEndPolicy::for_storage(io_class);
    let governor = ResourceGovernor::new(AppConfig::default().governor);
    let mut probe = WindowsResourceProbe::new();
    let _ = probe.sample();

    let result = sizes.for_each_candidate_group(min_bytes, 4096, |size, ids| {
        wait_for_expensive_slot(&low_end, &governor, &mut probe);
        let mut by_sample: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for &file_id in ids {
            let Some(hit) = store.get_by_file_id(file_id)? else {
                continue;
            };
            let relative = match store.reconstruct_path(&hit, 256) {
                Ok(path) => path,
                Err(_) => continue,
            };
            let path = PathBuf::from(absolute_drive_path(drive, &relative));
            let fingerprint = match sample_fingerprint(&path, size) {
                Ok(value) => value,
                Err(_) => continue,
            };
            by_sample.entry(fingerprint).or_default().push(path);
        }

        for paths in by_sample.into_values().filter(|paths| paths.len() >= 2) {
            let mut by_full: HashMap<u64, Vec<PathBuf>> = HashMap::new();
            for path in paths {
                wait_for_expensive_slot(&low_end, &governor, &mut probe);
                let hash = match full_fingerprint(&path) {
                    Ok(hash) => hash,
                    Err(_) => continue,
                };
                by_full.entry(hash).or_default().push(path);
            }

            for paths in by_full.into_values().filter(|paths| paths.len() >= 2) {
                let mut exact_sets: Vec<Vec<PathBuf>> = Vec::new();
                'candidate: for path in paths {
                    for set in &mut exact_sets {
                        if exact_files_equal(&set[0], &path).unwrap_or(false) {
                            set.push(path);
                            continue 'candidate;
                        }
                    }
                    exact_sets.push(vec![path]);
                }
                for set in exact_sets.into_iter().filter(|set| set.len() >= 2) {
                    duplicate_sets = duplicate_sets.saturating_add(1);
                    reclaimable = reclaimable
                        .saturating_add(size.saturating_mul(set.len().saturating_sub(1) as u64));
                    println!("GROUP size={size} copies={}", set.len());
                    for path in set {
                        println!(
                            "  protected={} {}",
                            is_protected_path(&path.to_string_lossy()),
                            path.display()
                        );
                    }
                }
            }
        }
        Ok(())
    });

    match result {
        Ok(stats) => {
            println!("duplicate_sets={duplicate_sets}");
            println!("reclaimable_bytes={reclaimable}");
            println!("candidate_groups={}", stats.candidate_groups);
            println!(
                "oversized_groups_skipped={}",
                stats.oversized_groups_skipped
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("duplicate scan failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn duplicates_report_multi(index_dir: &std::path::Path, min_bytes_arg: Option<&str>) -> ExitCode {
    use search_core::{
        discover_volume_indexes, exact_files_equal, for_each_merged_candidate_group,
        full_fingerprint, is_protected_path, sample_fingerprint, sidecar_is_fresh, size_index_path,
        IoClass, LiveSearchStore, LowEndPolicy, SizeIndex,
    };
    use search_platform_windows::NtfsVolume;
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct Source {
        volume: char,
        store: LiveSearchStore,
    }

    let min_bytes = min_bytes_arg
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1024 * 1024);
    let indexes = match discover_volume_indexes(index_dir) {
        Ok(indexes) if !indexes.is_empty() => indexes,
        Ok(_) => {
            eprintln!("no volume indexes found in {}", index_dir.display());
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("failed to discover volume indexes: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut any_hdd = false;
    let mut sources = Vec::with_capacity(indexes.len());
    let mut size_indexes = Vec::with_capacity(indexes.len());
    for (volume, index_path) in indexes {
        let size_path = size_index_path(&index_path);
        if !sidecar_is_fresh(&index_path, &size_path).unwrap_or(false) {
            eprintln!("size metadata is stale for {volume}:; metadata rebuild required");
            return ExitCode::from(3);
        }
        let sizes = match SizeIndex::open(size_path) {
            Ok(index) => index,
            Err(error) => {
                eprintln!("failed to open size metadata for {volume}:: {error}");
                return ExitCode::FAILURE;
            }
        };
        let store = match LiveSearchStore::open(&index_path) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open {volume}: index: {error}");
                return ExitCode::FAILURE;
            }
        };
        any_hdd |= NtfsVolume::open_drive(volume)
            .map(|v| v.storage_class() == IoClass::Hdd)
            .unwrap_or(false);
        size_indexes.push(sizes);
        sources.push(Source { volume, store });
    }

    let policy = LowEndPolicy::for_storage(if any_hdd { IoClass::Hdd } else { IoClass::Ssd });
    let governor = ResourceGovernor::new(AppConfig::default().governor);
    let mut probe = WindowsResourceProbe::new();
    let _ = probe.sample();

    const MAX_GROUP_ENTRIES: usize = 4096;
    let mut duplicate_sets = 0_u64;
    let mut reclaimable = 0_u64;

    let scan = for_each_merged_candidate_group(
        &mut size_indexes,
        min_bytes,
        MAX_GROUP_ENTRIES,
        |size, group| {
            let mut paths = Vec::with_capacity(group.len());
            for &(source, file_id) in group {
                let item = &mut sources[source];
                let Ok(Some(hit)) = item.store.get_by_file_id(file_id) else {
                    continue;
                };
                let Ok(relative) = item.store.reconstruct_path(&hit, 256) else {
                    continue;
                };
                let full = if relative.len() >= 2 && relative.as_bytes()[1] == b':' {
                    PathBuf::from(relative)
                } else {
                    PathBuf::from(format!(
                        "{}:\\{}",
                        item.volume,
                        relative.trim_start_matches(['\\', '/'])
                    ))
                };
                paths.push(full);
            }
            if paths.len() < 2 {
                return Ok(());
            }

            let mut by_sample: HashMap<u64, Vec<PathBuf>> = HashMap::new();
            for path in paths {
                wait_for_expensive_slot(&policy, &governor, &mut probe);
                if let Ok(hash) = sample_fingerprint(&path, size) {
                    by_sample.entry(hash).or_default().push(path);
                }
            }
            for paths in by_sample.into_values().filter(|paths| paths.len() >= 2) {
                let mut by_full: HashMap<u64, Vec<PathBuf>> = HashMap::new();
                for path in paths {
                    wait_for_expensive_slot(&policy, &governor, &mut probe);
                    if let Ok(hash) = full_fingerprint(&path) {
                        by_full.entry(hash).or_default().push(path);
                    }
                }
                for paths in by_full.into_values().filter(|paths| paths.len() >= 2) {
                    let mut exact_sets: Vec<Vec<PathBuf>> = Vec::new();
                    'candidate: for path in paths {
                        for set in &mut exact_sets {
                            if exact_files_equal(&set[0], &path).unwrap_or(false) {
                                set.push(path);
                                continue 'candidate;
                            }
                        }
                        exact_sets.push(vec![path]);
                    }
                    for set in exact_sets.into_iter().filter(|set| set.len() >= 2) {
                        duplicate_sets = duplicate_sets.saturating_add(1);
                        reclaimable = reclaimable.saturating_add(
                            size.saturating_mul(set.len().saturating_sub(1) as u64),
                        );
                        println!("GROUP size={size} copies={}", set.len());
                        for path in set {
                            println!(
                                "  protected={} {}",
                                is_protected_path(&path.to_string_lossy()),
                                path.display()
                            );
                        }
                    }
                }
            }
            Ok(())
        },
    );

    match scan {
        Ok(stats) => {
            println!("duplicate_sets={duplicate_sets}");
            println!("reclaimable_bytes={reclaimable}");
            println!("candidate_groups={}", stats.candidate_groups);
            println!(
                "oversized_groups_skipped={}",
                stats.oversized_groups_skipped
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("duplicate scan failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn wait_for_expensive_slot(
    policy: &search_core::LowEndPolicy,
    governor: &ResourceGovernor,
    probe: &mut WindowsResourceProbe,
) {
    loop {
        match policy.permit(governor, search_core::QueryCost::Expensive, probe.sample()) {
            Decision::Run => return,
            Decision::Throttle => {
                thread::sleep(Duration::from_millis(25));
                return;
            }
            Decision::Pause => thread::sleep(Duration::from_millis(250)),
        }
    }
}

#[cfg(not(windows))]
fn duplicates_report(
    _drive_arg: Option<&str>,
    _path_arg: Option<&str>,
    _min_bytes_arg: Option<&str>,
) -> ExitCode {
    eprintln!("duplicates is only available on Windows");
    ExitCode::FAILURE
}

#[cfg(windows)]
fn absolute_drive_path(drive: char, relative: &str) -> String {
    if relative.len() >= 2 && relative.as_bytes()[1] == b':' {
        relative.to_string()
    } else {
        format!("{drive}:\\{}", relative.trim_start_matches(['\\', '/']))
    }
}

fn quarantine_root(override_arg: Option<&str>) -> std::path::PathBuf {
    if let Some(value) = override_arg {
        return std::path::PathBuf::from(value);
    }
    #[cfg(windows)]
    {
        std::path::PathBuf::from(r"C:\.SearchToolQuarantine")
    }
    #[cfg(not(windows))]
    {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".search-tool-quarantine")
    }
}

fn quarantine_root_for_source(source: &str, override_arg: Option<&str>) -> std::path::PathBuf {
    if override_arg.is_some() {
        return quarantine_root(override_arg);
    }
    let bytes = source.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_uppercase();
        return std::path::PathBuf::from(format!(r"{drive}:\.SearchToolQuarantine"));
    }
    quarantine_root(None)
}

fn quarantine_command(path_arg: Option<&str>, root_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool quarantine PATH [ROOT]");
        return ExitCode::from(2);
    };
    let root = quarantine_root_for_source(path, root_arg);
    match search_core::quarantine_path(path, &root) {
        Ok(entry) => {
            println!("id={}", entry.id);
            println!("original={}", entry.original_path.display());
            println!("stored={}", entry.stored_path.display());
            println!("root={}", root.display());
            println!("size_bytes={}", entry.size_bytes);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("quarantine refused/failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn quarantine_list_command(root_arg: Option<&str>) -> ExitCode {
    let root = quarantine_root(root_arg);
    match search_core::list_quarantine(&root, 1000) {
        Ok(entries) => {
            for entry in entries {
                println!(
                    "{}\t{}\t{}",
                    entry.id,
                    entry.size_bytes,
                    entry.original_path.display()
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("quarantine list failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn restore_command(id_arg: Option<&str>, root_arg: Option<&str>) -> ExitCode {
    let Some(id) = id_arg else {
        eprintln!("usage: search-tool restore ID [ROOT]");
        return ExitCode::from(2);
    };
    let root = quarantine_root(root_arg);
    match search_core::restore_quarantine(&root, id) {
        Ok(entry) => {
            println!("restored={}", entry.original_path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("restore failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn purge_command(id_arg: Option<&str>, root_arg: Option<&str>) -> ExitCode {
    let Some(id) = id_arg else {
        eprintln!("usage: search-tool purge ID [ROOT]");
        return ExitCode::from(2);
    };
    let root = quarantine_root(root_arg);
    match search_core::purge_quarantine(&root, id) {
        Ok(entry) => {
            println!("purged={}", entry.original_path.display());
            println!("reclaimed_bytes={}", entry.size_bytes);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("purge failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn content_search(path_arg: Option<&str>, query_arg: Option<&str>) -> ExitCode {
    let (Some(path), Some(query)) = (path_arg, query_arg) else {
        eprintln!("usage: search-tool content-search INDEX_OR_DIR QUERY");
        return ExitCode::from(2);
    };
    let target = std::path::Path::new(path);
    if target.is_dir() {
        let mut store = match MultiLiveSearchStore::open_index_directory(target) {
            Ok(store) => store,
            Err(error) => {
                eprintln!("failed to open index directory: {error}");
                return ExitCode::FAILURE;
            }
        };
        let stale = store.stale_content_volumes();
        if !stale.is_empty() {
            eprintln!(
                "content index is stale/missing for volumes={} (those volumes are skipped)",
                stale.iter().collect::<String>()
            );
        }
        let hits = match store.search_content(query, 100) {
            Ok(hits) => hits,
            Err(error) => {
                eprintln!("content search failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        print_multi_hits(&mut store, hits);
        return ExitCode::SUCCESS;
    }

    let sidecar = content_path(path);
    if !sidecar_is_fresh(path, &sidecar).unwrap_or(false) {
        eprintln!(
            "content index is stale; results may miss recent content changes. Run `search-tool compact {path}` then rebuild content index"
        );
    }
    let mut content = match ContentIndex::open(sidecar) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("failed to open content index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut store = match LiveSearchStore::open(path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("failed to open base index: {error}");
            return ExitCode::FAILURE;
        }
    };
    let ids = match content.search(query, 100) {
        Ok(ids) => ids,
        Err(error) => {
            eprintln!("content search failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    for id in ids {
        if let Ok(Some(hit)) = store.get_by_file_id(id) {
            match store.reconstruct_path(&hit, 256) {
                Ok(path) => println!("{path}"),
                Err(_) => println!("{}", hit.name),
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(windows)]
fn sync_index(drive_arg: Option<&str>, path_arg: Option<&str>) -> ExitCode {
    let drive = match parse_drive_letter(drive_arg) {
        Ok(drive) => drive,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let Some(path) = path_arg else {
        eprintln!("missing index path");
        return ExitCode::from(2);
    };
    match search_platform_windows::sync_index_default(drive, path) {
        Ok(stats) => {
            println!("records={}", stats.records);
            println!("batches={}", stats.batches);
            println!("next_usn={}", stats.next_usn);
            println!("caught_up={}", stats.caught_up);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("USN sync failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn sync_index(_drive_arg: Option<&str>, _path_arg: Option<&str>) -> ExitCode {
    eprintln!("sync is only available on Windows/NTFS");
    ExitCode::FAILURE
}

fn compact_command(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool compact INDEX");
        return ExitCode::from(2);
    };
    match search_core::compact_index(path) {
        Ok(stats) => {
            println!("records={}", stats.records);
            println!("index_bytes={}", stats.index_bytes);
            println!("name_index_bytes={}", stats.name_index_bytes);
            println!("id_index_bytes={}", stats.id_index_bytes);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("compaction failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn verify_command(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool verify INDEX");
        return ExitCode::from(2);
    };
    match search_core::verify_index(path) {
        Ok(report) => {
            println!("status=ok");
            println!("records={}", report.records);
            println!("delta_entries={}", report.delta_entries);
            println!("memory_hint_bytes={}", report.memory_hint_bytes);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("index verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn verify_deep_command(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool verify-deep INDEX");
        return ExitCode::from(2);
    };
    match search_core::verify_index_deep(path) {
        Ok(report) => {
            println!("status=ok");
            println!("records={}", report.records);
            println!("names_checked={}", report.names_checked);
            println!("ids_checked={}", report.ids_checked);
            println!("parent_links_missing={}", report.parent_links_missing);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("deep index verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn maintenance_targets(path_arg: &str) -> std::io::Result<Vec<(Option<char>, std::path::PathBuf)>> {
    use std::path::Path;

    let target = Path::new(path_arg);
    if target.is_dir() {
        let indexes = search_core::discover_volume_indexes(target)?;
        if indexes.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no .stidx volume indexes found in directory",
            ));
        }
        Ok(indexes
            .into_iter()
            .map(|(volume, path)| (Some(volume), path))
            .collect())
    } else {
        Ok(vec![(
            search_core::volume_from_index_path(target),
            target.to_path_buf(),
        )])
    }
}

fn maintenance_label(volume: Option<char>, path: &std::path::Path) -> String {
    volume
        .map(|value| format!("{value}:"))
        .unwrap_or_else(|| path.display().to_string())
}

fn repair_command(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool repair INDEX_OR_DIR");
        return ExitCode::from(2);
    };
    let targets = match maintenance_targets(path) {
        Ok(targets) => targets,
        Err(error) => {
            eprintln!("failed to resolve repair targets: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut failures = 0usize;
    for (volume, index) in targets {
        let label = maintenance_label(volume, &index);
        if let Some(age_ms) = service_appears_active(&index) {
            eprintln!(
                "[{label}] service_guard=BLOCK state_age_ms={age_ms}; stop Search Tool service before repair"
            );
            failures = failures.saturating_add(1);
            continue;
        }
        match search_core::repair_index_sidecars(&index) {
            Ok(()) => println!("[{label}] status=repaired"),
            Err(error) => {
                eprintln!("[{label}] sidecar repair failed: {error}");
                failures = failures.saturating_add(1);
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(not(windows))]
const SERVICE_MAINTENANCE_GUARD_MS: u64 = 11 * 60 * 1000;

fn service_appears_active(_index: &std::path::Path) -> Option<u64> {
    #[cfg(windows)]
    {
        let selected = std::env::var("SEARCH_TOOL_SERVICE_NAME")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "SearchToolIndexer".to_owned());
        if search_platform_windows::is_service_running(&selected).unwrap_or(false) {
            return Some(0);
        }
        None
    }

    #[cfg(not(windows))]
    {
        let state = search_core::read_service_state(_index).ok()?;
        if state.updated_unix_ms == 0 {
            return None;
        }
        let age = search_core::unix_millis().saturating_sub(state.updated_unix_ms);
        (age <= SERVICE_MAINTENANCE_GUARD_MS).then_some(age)
    }
}

fn maintain_command(path_arg: Option<&str>) -> ExitCode {
    let Some(path) = path_arg else {
        eprintln!("usage: search-tool maintain INDEX_OR_DIR");
        return ExitCode::from(2);
    };
    let targets = match maintenance_targets(path) {
        Ok(targets) => targets,
        Err(error) => {
            eprintln!("failed to resolve maintenance targets: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut failures = 0usize;
    for (volume, index) in targets {
        let label = maintenance_label(volume, &index);
        println!("[{label}]");

        if let Some(age_ms) = service_appears_active(&index) {
            eprintln!(
                "service_guard=BLOCK state_age_ms={age_ms}; stop Search Tool service before manual maintenance"
            );
            failures = failures.saturating_add(1);
            continue;
        }

        if let Err(error) = search_core::recover_compaction(&index) {
            eprintln!("recovery=FAIL {error}");
            failures = failures.saturating_add(1);
            continue;
        }
        println!("recovery=PASS");

        let mut verified = search_core::verify_index(&index);
        if verified.is_err() {
            match search_core::repair_index_sidecars(&index) {
                Ok(()) => {
                    println!("sidecar_repair=PASS");
                    verified = search_core::verify_index(&index);
                }
                Err(error) => {
                    eprintln!("sidecar_repair=FAIL {error}");
                }
            }
        }

        let report = match verified {
            Ok(report) => report,
            Err(error) => {
                eprintln!("verify=FAIL {error}");
                failures = failures.saturating_add(1);
                continue;
            }
        };
        println!(
            "verify=PASS records={} delta_entries={}",
            report.records, report.delta_entries
        );

        if report.delta_entries > 0 {
            match search_core::compact_index(&index) {
                Ok(stats) => println!("compact=PASS records={}", stats.records),
                Err(error) => {
                    eprintln!("compact=FAIL {error}");
                    failures = failures.saturating_add(1);
                    continue;
                }
            }
        } else {
            println!("compact=SKIP delta_entries=0");
        }

        match search_core::verify_index(&index) {
            Ok(final_report) => println!(
                "final_verify=PASS records={} delta_entries={}",
                final_report.records, final_report.delta_entries
            ),
            Err(error) => {
                eprintln!("final_verify=FAIL {error}");
                failures = failures.saturating_add(1);
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn doctor_command(path_arg: Option<&str>) -> ExitCode {
    use search_core::{
        discover_volume_indexes, size_index_path, verify_index, volume_from_index_path,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    let Some(value) = path_arg else {
        eprintln!("usage: search-tool doctor INDEX_OR_DIR");
        return ExitCode::from(2);
    };
    let target = Path::new(value);
    let indexes: Vec<(Option<char>, PathBuf)> = if target.is_dir() {
        match discover_volume_indexes(target) {
            Ok(paths) if !paths.is_empty() => paths
                .into_iter()
                .map(|(volume, path)| (Some(volume), path))
                .collect(),
            Ok(_) => {
                eprintln!("no .stidx volume indexes found in {}", target.display());
                return ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("failed to enumerate index directory: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        vec![(volume_from_index_path(target), target.to_path_buf())]
    };

    let mut failures = 0usize;
    for (volume, index) in indexes {
        let label = volume
            .map(|v| format!("{v}:"))
            .unwrap_or_else(|| index.display().to_string());
        println!("[{label}]");
        match LiveSearchStore::open(&index) {
            Ok(store) => {
                println!("open=PASS");
                println!("base_records={}", store.base_record_count());
                println!("memory_hint_bytes={}", store.memory_hint_bytes());
            }
            Err(error) => {
                println!("open=FAIL {error}");
                failures += 1;
                continue;
            }
        }

        match verify_index(&index) {
            Ok(report) => println!(
                "verify=PASS records={} delta_entries={} memory_hint_bytes={}",
                report.records, report.delta_entries, report.memory_hint_bytes
            ),
            Err(error) => {
                println!("verify=FAIL {error}");
                failures += 1;
            }
        }

        let delta = search_core::delta_path(&index);
        let delta_bytes = fs::metadata(&delta).map(|m| m.len()).unwrap_or(0);
        println!("delta_bytes={delta_bytes}");
        println!(
            "pending_delta={}",
            search_core::pending_delta(&index).unwrap_or(true)
        );

        let attributes = attribute_index_path(&index);
        println!(
            "metadata_fresh={}",
            sidecar_is_fresh(&index, &attributes).unwrap_or(false)
        );
        let sizes = size_index_path(&index);
        println!(
            "sizes_fresh={}",
            sidecar_is_fresh(&index, &sizes).unwrap_or(false)
        );
        let content = content_path(&index);
        println!(
            "content_fresh={}",
            sidecar_is_fresh(&index, &content).unwrap_or(false)
        );

        match search_core::read_service_state(&index) {
            Ok(state) => {
                let age_ms = search_core::unix_millis().saturating_sub(state.updated_unix_ms);
                println!("service_state=PASS");
                println!("service_state_age_ms={age_ms}");
                println!("service_sync={:?}", state.sync_state);
                println!("service_storage={:?}", state.storage_class);
                println!("service_delta_bytes={}", state.delta_bytes);
                println!("service_last_sync_unix_ms={}", state.last_sync_unix_ms);
                println!(
                    "service_last_compact_unix_ms={}",
                    state.last_compact_unix_ms
                );
                println!("service_maintenance={:?}", state.maintenance);
                println!(
                    "service_maintenance_started_unix_ms={}",
                    state.maintenance_started_unix_ms
                );
                println!("service_last_error_code={}", state.last_error_code);
                println!(
                    "service_last_maintenance_exit={}",
                    state.last_maintenance_exit
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                println!("service_state=MISSING");
            }
            Err(error) => {
                println!("service_state=FAIL {error}");
                failures += 1;
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn bench_command(records_arg: Option<&str>, queries_arg: Option<&str>) -> ExitCode {
    let records = records_arg
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(100_000);
    let queries = queries_arg
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(250);
    match search_core::run_synthetic_benchmark(records, queries) {
        Ok(report) => {
            println!("records={}", report.records);
            println!("build_ms={}", report.build_time.as_millis());
            println!("open_us={}", report.open_time.as_micros());
            println!("index_bytes={}", report.index_bytes);
            println!("sidecar_bytes={}", report.sidecar_bytes);
            println!("memory_hint_bytes={}", report.memory_hint_bytes);
            println!("exact_queries={}", report.exact_queries);
            println!("exact_avg_us={}", report.exact_avg_micros());
            println!("prefix_queries={}", report.prefix_queries);
            println!("prefix_avg_us={}", report.prefix_avg_micros());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("benchmark failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("Search Tool - ultra-light local search and maintenance engine");
    println!();
    println!("Usage:");
    println!("  search-tool status                     Resource state / governor decision");
    println!("  search-tool ntfs-status C:             Query NTFS USN Journal");
    println!("  search-tool index C: INDEX             Build initial persistent NTFS index");
    println!("  search-tool sync C: INDEX              Apply USN delta without full rescan");
    println!("  search-tool search INDEX QUERY         Ranked search + filters");
    println!("  search-tool fuzzy INDEX QUERY          Bounded typo-tolerant search");
    println!("  search-tool related INDEX QUERY        Related application/tool ecosystem");
    println!("  search-tool content-build C: INDEX     Build text/code content sidecar");
    println!("  search-tool content-build-rich C: INDEX [WORKER]  Rich document content");
    println!("  search-tool content-search INDEX Q     Search indexed file contents");
    println!("  search-tool metadata-build C: INDEX    Build size/date attribute sidecars");
    println!("  search-tool duplicates C: INDEX [MIN]  Byte-verified duplicate report");
    println!("  search-tool cleanup INDEX_OR_DIR        Analyze cleanup candidates safely");
    println!("  search-tool quarantine PATH [ROOT]     Move policy-approved item safely");
    println!("  search-tool quarantine-list [ROOT]     List quarantined items");
    println!("  search-tool restore ID [ROOT]          Restore quarantined item");
    println!("  search-tool purge ID [ROOT]            Permanently delete quarantine item");
    println!("  search-tool route MODEL QUERY          Test tiny-AI routing");
    println!("  search-tool smart INDEX QUERY [MODEL]  Natural-language routed search");
    println!("  search-tool web-lookup INDEX TARGET    Privacy-sanitized on-demand lookup");
    println!("  search-tool compact INDEX              Merge delta into base index");
    println!("  search-tool verify INDEX               Fast index-family verification");
    println!("  search-tool verify-deep INDEX          Full record/id/parent verification");
    println!("  search-tool repair INDEX_OR_DIR        Rebuild recoverable sidecars");
    println!("  search-tool maintain INDEX_OR_DIR      Recover, repair, compact, verify");
    println!("  search-tool bench [RECORDS] [QUERIES]  Synthetic disk-first benchmark");
    println!("  search-tool version                    Show version");
}

#[cfg(all(test, windows))]
mod cli_tests {
    use super::{
        parse_drive_letter, parser_worker_requires_restart, worker_path_bytes, ParserWorkerClient,
        MAX_WORKER_PATH_BYTES,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn drive_parser_accepts_only_drive_designators() {
        assert_eq!(parse_drive_letter(Some("c")).unwrap(), 'C');
        assert_eq!(parse_drive_letter(Some("d:")).unwrap(), 'D');
    }

    #[test]
    fn drive_parser_rejects_full_paths() {
        assert!(parse_drive_letter(Some(r"C:\temp\index")).is_err());
        assert!(parse_drive_letter(Some("C:/temp/index")).is_err());
        assert!(parse_drive_letter(Some("CC")).is_err());
    }

    #[test]
    fn parser_worker_path_limit_matches_protocol() {
        assert!(worker_path_bytes(&"a".repeat(MAX_WORKER_PATH_BYTES)).is_ok());
        let error = worker_path_bytes(&"a".repeat(MAX_WORKER_PATH_BYTES + 1)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parser_worker_restart_policy_covers_protocol_corruption() {
        for kind in [
            std::io::ErrorKind::TimedOut,
            std::io::ErrorKind::BrokenPipe,
            std::io::ErrorKind::UnexpectedEof,
            std::io::ErrorKind::ConnectionAborted,
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::InvalidData,
        ] {
            assert!(parser_worker_requires_restart(kind));
        }
        assert!(!parser_worker_requires_restart(
            std::io::ErrorKind::InvalidInput
        ));
        assert!(!parser_worker_requires_restart(
            std::io::ErrorKind::NotFound
        ));
    }

    fn hostile_powershell(script: &str) -> ParserWorkerClient {
        let root = std::env::var_os("SystemRoot").expect("SystemRoot must be set on Windows");
        let exe = std::path::PathBuf::from(root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        ParserWorkerClient::spawn_with_args(
            exe.to_str().expect("PowerShell path must be UTF-8"),
            &[
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ],
        )
        .unwrap()
    }

    #[test]
    fn parser_worker_watchdog_terminates_hung_worker() {
        let mut parser = hostile_powershell("Start-Sleep -Seconds 30");
        let started = Instant::now();
        let error = parser.extract(r"C:\nonexistent.txt", 128).unwrap_err();
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_secs(4));
        assert!(elapsed < Duration::from_secs(10));
        assert!(parser_worker_requires_restart(error.kind()));
    }

    #[test]
    fn parser_worker_abrupt_exit_is_restartable() {
        let mut parser = hostile_powershell("exit 23");
        let error = parser.extract(r"C:\nonexistent.txt", 128).unwrap_err();
        assert!(parser_worker_requires_restart(error.kind()));
    }

    #[test]
    fn parser_worker_partial_stdout_is_restartable() {
        let mut parser = hostile_powershell(
            "$o=[Console]::OpenStandardOutput();$b=[byte[]](0,0,0,0);$o.Write($b,0,$b.Length);$o.Flush()",
        );
        let error = parser.extract(r"C:\nonexistent.txt", 128).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
        assert!(parser_worker_requires_restart(error.kind()));
    }

    #[test]
    fn parser_worker_invalid_utf8_is_restartable() {
        let mut parser = hostile_powershell(
            "$o=[Console]::OpenStandardOutput();$b=[byte[]](0,0,0,0,2,0,0,0,255,254);$o.Write($b,0,$b.Length);$o.Flush()",
        );
        let error = parser.extract(r"C:\nonexistent.txt", 128).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof
            ),
            "unexpected hostile-worker error kind: {:?}",
            error.kind()
        );
        assert!(parser_worker_requires_restart(error.kind()));
    }

    #[test]
    fn parser_worker_oversized_response_is_restartable_without_drain() {
        let mut parser = hostile_powershell(
            "$o=[Console]::OpenStandardOutput();$b=[byte[]](0,0,0,0,1,0,128,0);$o.Write($b,0,$b.Length);$o.Flush();Start-Sleep -Seconds 30",
        );
        let started = Instant::now();
        let error = parser.extract(r"C:\nonexistent.txt", 128).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                std::io::ErrorKind::InvalidData | std::io::ErrorKind::UnexpectedEof
            ),
            "unexpected hostile-worker error kind: {:?}",
            error.kind()
        );
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(parser_worker_requires_restart(error.kind()));
    }
}
