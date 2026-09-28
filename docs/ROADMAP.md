# Search Tool Roadmap

Last updated: 2026-09-28.

This is the ordered continuation backlog. Items marked blocker should be completed before calling the current source tree a final release candidate.

## P0 - Release blockers

1. **Complete the hardened D: 60-minute soak**
   - D: recovery is clean: delta=0, metadata/sizes/content fresh and verify-deep PASS.
   - Ownership failure fixed: conflicting validators/SCM takeover are rejected and PASS/FAIL JSON is guaranteed.
   - A later long attempt ran 865.48 s / 9,984 operations / 416 checks and exposed a periodic fast-`verify` collision with the service mutation lock.
   - Fast `verify` and `verify-deep` now share the same bounded retry for only the exact busy-lock condition; unrelated failures remain fail-fast.
   - High-load regression PASS: 65.84 s, 4,992 operations, 26 checks, crash/restart exercised.
   - Remaining: commit the current fixes, rerun with crash/restart until a complete 60-minute `result=PASS` report exists, then clean/verify D: again.

2. **Recover the real C: validation index**
   - Current index opens and verify-deep passes at 1,427,984 records with delta=0.
   - Metadata and size sidecars are fresh; content is stale after an interrupted build.
   - Leave stale content staging to the supported content builder, which takes the build lock before cleaning/rebuilding it.
   - Finish content freshness, doctor and verify-deep.
   - Reinstall/start SearchToolIndexer and confirm automatic USN sync.

3. **Sleep/resume validation**
   - Record marker, checkpoint hash and service state.
   - Suspend Windows, resume, then verify the old marker.
   - Create a new marker and confirm USN catch-up.
   - Require doctor + verify-deep PASS and no forced full rebuild unless journal generation genuinely changed.

4. **Reboot validation**
   - Record pre-reboot marker/checkpoint.
   - Reboot Windows.
   - Confirm SearchToolIndexer auto-start.
   - Verify old and new markers, USN catch-up, doctor and verify-deep.

5. **Compaction commit/swap fault injection — COMPLETE**
   - Added deterministic abrupt child-process termination at 11 publish boundaries from durable marker through final main-file rename.
   - Found a real mixed-generation bug: the old main file remained present during sidecar swaps, so existence-only recovery could misclassify a partial publish as committed.
   - Fixed the protocol by removing the old main immediately after the durable marker, publishing sidecars, then renaming the staged main last. Main-file presence is now the commit bit.
   - Recovery + retry requires verify-deep PASS, correct logical search results, consumed delta and no compact/delta-sort debris. Regression PASS.

## P1 - Hardening

6. Hostile parser-worker matrix: hang, crash, partial stdout, invalid UTF-8, oversized output, corrupt/encrypted PDF, corrupt OOXML and zip-bomb-like containers.
7. Upgrade rollback fault injection: interrupt upgrade and prove old binary/config/index remain usable.
8. Multi-monitor GUI validation including mixed DPI, primary-display switch and monitor removal recovery.
9. Valid Web Resolver success/cache path with real Google Custom Search credentials; keep optional and privacy-sanitized.

## P2 - Performance and release evidence

10. Real 1M+ C: latency table for exact, prefix, fuzzy, filtered, relationship and content search: p50/p95/p99.
11. Rerun foreground-impact after the final source freeze.
12. Run 6-hour soak; ideally also 24-hour soak for leak/delta-growth confidence.
13. Run Defender + SmartScreen on a clean Windows installation with Defender enabled.
14. Run pristine-machine install -> initial index -> search -> service -> GUI -> uninstall.
15. Run the reference physical target: Celeron-class CPU, 4 GB RAM, mechanical HDD.
16. Tune governor/batch/compaction/content settings only from reference-machine evidence.

## Release freeze checklist

When no source changes remain:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace --release`
- `scripts/windows-release-gate.ps1`
- package + SHA-256 manifest
- record final benchmark/soak reports

Do not commit `target/`, VHDs, live indexes, temp sidecars or machine-specific validation data.
