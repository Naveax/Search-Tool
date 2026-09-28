# Search Tool Architecture

## Hot path

`NTFS MFT → disk-first index → sparse checkpoints → exact/prefix/ranking`

Normal arama AI veya web gerektirmez.

## Incremental path

`USN Journal → append-only delta → live overlay → idle compaction`

Full rescan yalnız journal/index recovery gerektirdiğinde yapılır.

## Expensive work

Content parsing, duplicate full hashing, metadata enrichment, AI ve compaction Resource Governor / LowEndPolicy ile sınırlandırılır. HDD'de kullanıcı aktifken pahalı background işler durur.

## Process isolation

- Core/CLI: Rust
- GUI: native Win32
- Service: native Windows service
- Rich content parser: ayrı worker
- Tiny AI: on-demand INT8 model

## Safety

Cleanup motoru sınıflandırma + policy sonucunu uygular. Otomatik kalıcı delete yoktur; izin verilen öğeler quarantine'e alınır ve restore edilebilir.
