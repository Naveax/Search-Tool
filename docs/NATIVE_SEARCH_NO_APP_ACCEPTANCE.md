# P0 native Windows Search experience, no companion search application

Decision date: 2026-10-15. Status: **NOT ACCEPTED / BLOCKED**.
Workstream: draft PR #81, issue #82. No production installation or SearchHost modification.

## Non-negotiable user experience

The user must search exactly as an ordinary Windows 10/11 user does:

* Windows key and typing, taskbar Search, Win+S, Start search and File Explorer
  stay Microsoft's original entrypoints, keyboard/focus/dismissal semantics
  and visible UI. There is no Search Tool launcher, second search window,
  separate tab, per-query prompt, manual app launch or new learning step.
* Search Tool may run behind the scenes only if a supported Windows
  extension delivers **measurably improved results inside the original UI**
  without replacing/hijacking SearchHost, Start, Search, Explorer or
  supported shell associations. A hidden GUI process by itself does not
  improve the native Search experience and must not be started at login.
* Preserve installed Windows Search indexing/service and supported
  first-party file, app, settings, web, keyboard, IME, accessibility and
  high-contrast behavior. Never silently change the user's default
  associations or installed search locations.
* Installation/uninstallation must be reversible, with a demonstrable
  rollback of pre-existing user shortcuts and registry values.

## Actual supported Windows APIs, and their limits

Microsoft documents **IFilter/property handlers** for indexing the contents
of *new file types*, and **Shell data source + protocol handlers**
(`ISearchProtocol`, `IUrlAccessor` and content filters) for *new*
Windows Search data stores. These do **not** provide a generic public API
to redirect Windows 10/11 Start/Win+S file ranking to a separate Rust
index or replace Microsoft's SearchHost result hierarchy. In particular,
a `search-ms:` query launches an Explorer search, not inline Start UI.

* https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr
* https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-extidx-prot-implementing
* https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-about
* https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-registering-filters
* https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-qryidx-searchms

Microsoft's `com.microsoft.windows.websearchprovider` is an EEA-scoped
**web-results provider**, not a general disk-index backend interface:
https://learn.microsoft.com/en-us/windows/apps/develop/search/search-providers

Do not implement unsupported hooks or inject into SearchHost, Explorer,
StartMenuExperienceHost or SearchProtocolHost. Do not hijack `search:`,
`search-ms:`, `Win+S`, shell keyboard interception or global hotkeys.
Do not wrap another visible app in a Windows-looking rectangle and claim
the Windows-owned search experience has been replaced.

## Implementation status

The *next-version installer source* (not installed) now removes old
`Startup/Search Tool.lnk` rather than creating a hidden resident GUI
launcher. It continues to remove its old Start Menu GUI link and optional
Explorer context menu verbs. `searchtool:` remains a **private, explicitly
invoked** protocol and is not a replacement for native Search. The
existing background SearchToolIndexer service is a *different index*;
running it **does not make the native Search UI use its results**.

* `scripts/native-search-experience-contract.ps1 -SelfTest`: fail-closed
  negative/positive policy tests, Windows PowerShell 5.1 and PowerShell 7.
* `scripts/native-search-experience-contract.ps1`: default installer
  source inspection PASS but entire native result integration BLOCKED.
* `scripts/pristine-validation.ps1`: newly installed default path must
  **not** contain a GUI auto-start link; no separate Start Menu shortcut.
* `scripts/install-fault-matrix.ps1`: an isolated legacy startup shortcut
  is planted before upgrade, removed on a successful update stage, and
  must be restored byte-for-byte after each simulated install fault.
* New CI step validates all of the above fail-closed UX policy checks.

All of this is a **source/installer protection**, not native Search backend
integration or finished end-user performance acceptance.

## Read-only physical inventory, October 15, 2026

Windows 11 version: 10.0.22631 (23H2 family, according to probe).
The native WSearch service is Running, Windows SystemIndex OLE DB query
returned one row without exporting file contents, and a verified
Explorer-owned SearchButton existed on the taskbar. A **read-only**
visible SearchHost window probe returned **BLOCKED** because no opened,
visible, uncloaked SearchHost frame was available. No Search UI was
activated, clicked, keyboard-controlled, or photographed by these probes.

This does **not** establish native result/source integration, GUI
pixel parity or Narrator/IME acceptance.

## Ordered release acceptance requirements

1. **P0 supported-backend proof:** identify an actual Windows Search
   extensibility route that takes Search Tool's chosen improvements to
   Microsoft-owned Win+S/Start results. Demonstrate a one-file/new-format
   fixture in an isolated Windows 10 and Windows 11 test installation.
   Differentiate better indexing of proprietary formats from improving
   all existing filesystem ranking. If only unsupported injection could
   meet the goal, stop and report BLOCKED instead of shipping a fake
   substitute.
2. **P0 unchanged entrypoints:** compare genuine Windows search results,
   app results, settings, path/content queries and history before/after
   the supported provider on an isolated test machine; no separate
   Search Tool window or manual application launch.
3. **P0 physical input/accessibility:** keyboard navigation, Win/Win+S,
   Start, taskbar, Escape/dismissal, mouse, Turkish and other IMEs,
   Narrator/UIA, high contrast/light/dark, Windows 10 vs Windows 11,
   DPI scaling at 100%, 125% and mixed-monitor configurations.
   Capture real evidence without stealing focus from active users.
4. **P1 packaging:** test the new source on isolated Windows VMs,
   run full CI, install/upgrade/fault-injection/pristine/uninstall, check
   rollback and exact package hashes. Do not touch current production
   SearchHost or SearchIndexer for this development iteration.
5. **P1 release:** only after 1–4 have evidence can the release state
   be re-evaluated. PR #81 remains draft; `docs/RELEASE_STATE.json`
   stays INVALIDATED. Do not merge, install or seal now.

**Deferred, not completion blockers:** more synthetic Rust ranking
benchmarks, 20M+ synthetic scale expansion, cache micro-optimizations,
the separate Search Tool GUI visual theme and cold-cache profiling.
They cannot close the native UX requirement by themselves.
