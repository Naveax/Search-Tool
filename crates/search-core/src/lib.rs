#![forbid(unsafe_code)]

pub mod ai;
pub mod attributes;
pub mod benchmark;
pub mod cleanup;
pub mod config;
pub mod content;
pub mod delta;
pub mod duplicate;
pub mod filters;
pub mod freshness;
pub mod governor;
pub mod index;
pub mod index_lock;
pub mod knowledge;
pub mod live;
pub mod low_end;
pub mod maintenance;
pub mod metadata;
pub mod multi;
pub mod natural;
pub mod quarantine;
pub mod query;
pub mod relationship;
pub mod service_state;
pub mod store;
pub mod web_resolver;

pub use config::{AppConfig, GovernorConfig};
pub use filters::{
    matches_filters, parse_date, parse_search_query, parse_size, ItemTypeFilter, ParsedSearchQuery,
    SearchFilters,
};
pub use freshness::{
    capture_index_generation, pending_delta, read_sidecar_generation, sidecar_freshness_path,
    sidecar_is_fresh, write_sidecar_generation, IndexGeneration,
};
pub use governor::{Decision, ResourceGovernor, ResourceSample, WorkClass};
pub use index_lock::{mutation_lock_path, IndexMutationGuard};
pub use low_end::{IoClass, LowEndPolicy, QueryCost};
pub use store::{
    repair_sidecars, BuildOptions, IndexBuilder, InputRecord, SearchHit, SearchStore, StoreRecord,
    StoreStats, FLAG_DIRECTORY, FLAG_HIDDEN, FLAG_REPARSE_POINT, FLAG_SYSTEM,
};

pub use delta::{
    checkpoint_path, delta_path, delta_record_count, load_latest_delta, read_checkpoint,
    write_checkpoint, DeltaOp, DeltaRecord, DeltaWriter, SyncCheckpoint,
};
pub use live::{LiveSearchHit, LiveSearchStore, DEFAULT_MAX_DELTA_ENTRIES};

pub use query::{bounded_levenshtein, fuzzy_distance, fuzzy_seed, relevance_score};
pub use relationship::{relation_for_query, RelationQuery};
pub use service_state::{
    read_service_state, service_state_path, unix_millis, write_service_state, MaintenanceKind,
    ServiceVolumeState, SyncState,
};

pub use cleanup::{cleanup_decision, is_protected_path, CleanupAction, CleanupDecision};
pub use knowledge::{classify_path, Classification, FileClass, RiskLevel};

pub use content::{
    content_checkpoints_path, content_path, is_text_candidate, ContentIndex, ContentIndexBuilder,
};

pub use ai::{IntentPrediction, QueryIntent, TinyIntentModel};

pub use natural::{content_terms, query_subject, rule_intent};

pub use web_resolver::{
    parse_google_custom_search_json, web_cache_path, WebCache, WebLookupQuery, WebResult,
    DEFAULT_WEB_CACHE_LIMIT_BYTES,
};

pub use maintenance::{
    compact_index, publish_staged_index, recover_compaction, remove_index_family,
    repair_index_sidecars, verify_index, verify_index_deep, DeepVerifyReport, VerifyReport,
};

pub use duplicate::{exact_files_equal, full_fingerprint, sample_fingerprint};
pub use metadata::{
    for_each_merged_candidate_group, size_index_path, GroupScanStats, SizeEntry, SizeIndex,
    SizeIndexBuilder,
};
pub use multi::{
    discover_volume_indexes, volume_from_index_path, MultiLiveSearchStore, VolumeSearchHit,
};

pub use attributes::{
    attribute_checkpoints_path, attribute_index_path, AttributeEntry, AttributeIndex,
    AttributeIndexBuilder,
};
pub use benchmark::{run_synthetic_benchmark, SyntheticBenchReport};

pub use quarantine::{
    list_quarantine, purge_quarantine, quarantine_path, restore_quarantine, QuarantineEntry,
};
