# Search Tool Roadmap

Last updated: 2026-09-28.

This is the ordered continuation backlog. Items marked blocker should be completed before calling the current source tree a final release candidate.

## P0 - Release blockers

1. **Recover the D: soak lab and rerun 60-minute soak**
   - Clear the unfinished workload/delta safely.
   - Run compact/maintain, metadata build, content build, verify-deep and doctor.
   - Investigate why the previous 60-minute run ended without its final JSON report.
   - Rerun with crash/restart enabled until a complete `result=PASS` report is written.

2. **Recover the real C: validation index**
   - Current index verifies but may contain pending delta and stale sidecars after interrupted content maintenance.
   - Recover/remove stale staging only through Search Tool recovery/maintenance semantics.
   - Finish metadata/content freshness and verify-deep.
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

5. **Compaction commit/swap fault injection**
   - Kill the compactor at the generation publish/swap boundary.
   - Restart and recover.
   - Require verify-deep PASS and no stale .tmp/.old/.new/delta-sort debris that can affect the next run.

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
