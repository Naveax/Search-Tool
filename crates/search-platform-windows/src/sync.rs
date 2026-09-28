use crate::{EnumerationStats, JournalCheckpoint, NtfsVolume};
use search_core::{
    attribute_checkpoints_path, attribute_index_path, checkpoint_path, compact_index,
    content_checkpoints_path, content_path, delta_path, delta_record_count, publish_staged_index,
    read_checkpoint, remove_index_family, sidecar_freshness_path, size_index_path,
    write_checkpoint, BuildOptions, DeltaOp, DeltaRecord, DeltaWriter, IndexBuilder,
    IndexMutationGuard, InputRecord, StoreStats, SyncCheckpoint, DEFAULT_MAX_DELTA_ENTRIES,
    FLAG_DIRECTORY, FLAG_HIDDEN, FLAG_REPARSE_POINT, FLAG_SYSTEM,
};
use std::{fs, io, path::Path};

const DEFAULT_SYNC_BATCHES: usize = 16;
const INITIAL_CATCHUP_BATCHES: usize = 64;

const ERROR_JOURNAL_DELETE_IN_PROGRESS: i32 = 1178;
const ERROR_JOURNAL_NOT_ACTIVE: i32 = 1179;
const ERROR_JOURNAL_ENTRY_DELETED: i32 = 1181;

/// Returns true when incremental USN replay is no longer trustworthy and the
/// durable base index must be reconciled from the MFT. Keep the Windows raw
/// codes here so callers do not have to guess how std maps them to ErrorKind.
pub fn usn_reconciliation_required(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::InvalidData | io::ErrorKind::NotFound
    ) || matches!(
        error.raw_os_error(),
        Some(ERROR_JOURNAL_DELETE_IN_PROGRESS)
            | Some(ERROR_JOURNAL_NOT_ACTIVE)
            | Some(ERROR_JOURNAL_ENTRY_DELETED)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexSyncStats {
    pub records: u64,
    pub batches: u32,
    pub next_usn: i64,
    pub caught_up: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialIndexStats {
    pub store: StoreStats,
    pub enumeration: EnumerationStats,
    pub catchup: IndexSyncStats,
}

/// Rebuilds the durable base index from the NTFS MFT, then replays a bounded
/// amount of USN history that accumulated while the base index was being built.
/// The USN checkpoint is intentionally captured before enumeration so no
/// create/rename/delete event during the initial scan is silently lost.
pub fn rebuild_index(drive: char, index_path: impl AsRef<Path>) -> io::Result<InitialIndexStats> {
    let index_path = index_path.as_ref();
    let volume = NtfsVolume::open_drive(drive)?;
    let journal_start = volume.query_journal()?;

    // When replacing an existing generation, build an entirely separate index
    // family first. Publishing it uses the crash-recoverable family swap from
    // search-core, so old main/sidecars cannot be mixed with new ones.
    let replacing = index_path.exists();
    let staging = rebuild_staging_path(index_path);
    if replacing {
        remove_index_family(&staging);
    }
    let build_path = if replacing {
        staging.as_path()
    } else {
        index_path
    };

    let mut builder = IndexBuilder::create(build_path, BuildOptions::default())?;
    let enumeration = volume.enumerate_mft(|record| {
        let name = record.decode_name();
        builder.push(InputRecord {
            file_id: record.file_reference_number,
            parent_id: record.parent_file_reference_number,
            size_bytes: 0,
            flags: attributes_to_store_flags(record.file_attributes),
            name: &name,
        })
    })?;
    let mut store = builder.finish()?;

    // Derived metadata belongs to the old generation. Missing sidecars are safe
    // and rebuildable; stale sidecars paired with a new file-id set are not.
    remove_generation_sidecars(index_path);

    if replacing {
        if let Err(error) = publish_staged_index(index_path, &staging) {
            remove_index_family(&staging);
            return Err(error);
        }
    }

    // The overlay/checkpoint now belong to the newly published generation.
    // Dropping the old delta only after the base family is safely installed
    // avoids losing a valid old generation if MFT enumeration fails.
    let _ = fs::remove_file(delta_path(index_path));
    write_checkpoint(
        checkpoint_path(index_path),
        SyncCheckpoint {
            journal_id: journal_start.journal_id,
            next_usn: journal_start.next_usn,
        },
    )?;

    let catchup = sync_index_bounded(drive, index_path, INITIAL_CATCHUP_BATCHES)?;
    let delta_records = delta_record_count(delta_path(index_path))?;
    if delta_records >= (DEFAULT_MAX_DELTA_ENTRIES as u64 / 2) {
        store = compact_index(index_path)?;
    }
    Ok(InitialIndexStats {
        store,
        enumeration,
        catchup,
    })
}

fn rebuild_staging_path(index_path: &Path) -> std::path::PathBuf {
    let mut value = index_path.as_os_str().to_os_string();
    value.push(".rebuild.new");
    value.into()
}

pub fn sync_index_once(drive: char, index_path: impl AsRef<Path>) -> io::Result<IndexSyncStats> {
    sync_index_bounded(drive, index_path, 1)
}

/// Replays at most `max_batches` 256 KiB journal pages. The bound prevents an
/// old/busy machine from being monopolized by a huge backlog, while allowing a
/// normal manual sync to catch up far faster than one page per invocation.
pub fn sync_index_bounded(
    drive: char,
    index_path: impl AsRef<Path>,
    max_batches: usize,
) -> io::Result<IndexSyncStats> {
    let index_path = index_path.as_ref();
    let _guard = IndexMutationGuard::try_acquire(index_path)?;
    let saved = read_checkpoint(checkpoint_path(index_path))?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "missing USN checkpoint; rebuild the index first",
        )
    })?;
    let volume = NtfsVolume::open_drive(drive)?;
    let current = volume.query_journal()?;
    validate_checkpoint(
        saved,
        current.journal_id,
        current.lowest_valid_usn,
        current.next_usn,
    )?;

    let target_usn = current.next_usn;
    if saved.next_usn >= target_usn {
        return Ok(IndexSyncStats {
            records: 0,
            batches: 0,
            next_usn: saved.next_usn,
            caught_up: true,
        });
    }

    let delta_file = delta_path(index_path);
    let mut delta: Option<DeltaWriter> = None;
    let mut next_usn = saved.next_usn;
    let mut records = 0_u64;
    let mut batches = 0_u32;
    let max_batches = max_batches.max(1);

    while next_usn < target_usn && (batches as usize) < max_batches {
        let stats = volume.read_journal_once_validated(
            JournalCheckpoint {
                journal_id: saved.journal_id,
                next_usn,
            },
            |record| {
                if !record.is_index_relevant() || record.is_rename_old_event() {
                    return Ok(());
                }
                let op = if record.is_delete_event() {
                    DeltaOp::Delete
                } else {
                    DeltaOp::Upsert
                };
                let name = if op == DeltaOp::Delete {
                    String::new()
                } else {
                    record.decode_name()
                };
                if delta.is_none() {
                    delta = Some(DeltaWriter::open(&delta_file)?);
                }
                delta
                    .as_mut()
                    .expect("delta writer initialized")
                    .append(&DeltaRecord {
                        op,
                        file_id: record.file_reference_number,
                        parent_id: record.parent_file_reference_number,
                        size_bytes: 0,
                        flags: attributes_to_store_flags(record.file_attributes),
                        name,
                    })
            },
        )?;

        batches += 1;
        records = records.saturating_add(stats.records);
        if stats.next_usn < next_usn {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "USN reader moved backwards; rebuild required",
            ));
        }
        if stats.next_usn == next_usn {
            break;
        }
        next_usn = stats.next_usn;
    }

    // Crash ordering is deliberate: publish all delta bytes first, then move
    // the checkpoint. Replaying an old page after a crash is harmless because
    // the live overlay is last-record-wins; skipping a page would not be.
    if let Some(delta) = delta.as_mut() {
        delta.sync()?;
    }
    if next_usn != saved.next_usn {
        write_checkpoint(
            checkpoint_path(index_path),
            SyncCheckpoint {
                journal_id: saved.journal_id,
                next_usn,
            },
        )?;
    }

    Ok(IndexSyncStats {
        records,
        batches,
        next_usn,
        caught_up: next_usn >= target_usn,
    })
}

pub fn sync_index_default(drive: char, index_path: impl AsRef<Path>) -> io::Result<IndexSyncStats> {
    sync_index_bounded(drive, index_path, DEFAULT_SYNC_BATCHES)
}

fn remove_generation_sidecars(index_path: &Path) {
    let attributes = attribute_index_path(index_path);
    let content = content_path(index_path);
    let sizes = size_index_path(index_path);
    for path in [
        attribute_checkpoints_path(&attributes),
        sidecar_freshness_path(&attributes),
        attributes,
        content_checkpoints_path(&content),
        sidecar_freshness_path(&content),
        content,
        sidecar_freshness_path(&sizes),
        sizes,
    ] {
        let _ = fs::remove_file(path);
    }
}

fn validate_checkpoint(
    saved: SyncCheckpoint,
    journal_id: u64,
    lowest_valid_usn: i64,
    current_next_usn: i64,
) -> io::Result<()> {
    if journal_id != saved.journal_id
        || saved.next_usn < lowest_valid_usn
        || saved.next_usn > current_next_usn
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "USN journal reset/truncation detected; rebuild the index",
        ));
    }
    Ok(())
}

fn attributes_to_store_flags(attributes: u32) -> u16 {
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    let mut flags = 0_u16;
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        flags |= FLAG_DIRECTORY;
    }
    if attributes & FILE_ATTRIBUTE_HIDDEN != 0 {
        flags |= FLAG_HIDDEN;
    }
    if attributes & FILE_ATTRIBUTE_SYSTEM != 0 {
        flags |= FLAG_SYSTEM;
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        flags |= FLAG_REPARSE_POINT;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_validation_accepts_current_range() {
        let saved = SyncCheckpoint {
            journal_id: 7,
            next_usn: 500,
        };
        assert!(validate_checkpoint(saved, 7, 100, 900).is_ok());
    }

    #[test]
    fn checkpoint_validation_rejects_reset_or_truncation() {
        let saved = SyncCheckpoint {
            journal_id: 7,
            next_usn: 500,
        };
        assert!(validate_checkpoint(saved, 8, 100, 900).is_err());
        assert!(validate_checkpoint(saved, 7, 600, 900).is_err());
        assert!(validate_checkpoint(saved, 7, 100, 400).is_err());
    }

    #[test]
    fn reconciliation_recognizes_windows_journal_entry_deleted() {
        let error = io::Error::from_raw_os_error(ERROR_JOURNAL_ENTRY_DELETED);
        assert!(usn_reconciliation_required(&error));
        assert!(!usn_reconciliation_required(&io::Error::from_raw_os_error(
            5
        )));
    }

    #[test]
    fn generation_sidecar_cleanup_removes_data_and_freshness() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("search-tool-generation-cleanup-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let index = root.join("D.stidx");
        let attributes = attribute_index_path(&index);
        let content = content_path(&index);
        let sizes = size_index_path(&index);
        let paths = [
            attribute_checkpoints_path(&attributes),
            sidecar_freshness_path(&attributes),
            attributes,
            content_checkpoints_path(&content),
            sidecar_freshness_path(&content),
            content,
            sidecar_freshness_path(&sizes),
            sizes,
        ];
        for path in &paths {
            fs::write(path, b"x").unwrap();
        }

        remove_generation_sidecars(&index);
        assert!(paths.iter().all(|path| !path.exists()));
        let _ = fs::remove_dir_all(root);
    }
}
