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


## Supported provider feasibility: source research and non-invasive probe

Source review: 2026-10-15.

Microsoft's [Windows Search extensibility overview](https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr)
documents a Shell namespace extension + protocol handler for indexing
items in **custom data stores**. Its [protocol handler registration
reference](https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration)
requires `ISearchProtocol`, `IUrlAccessor` and `IFilter` with a
crawl scope and registered handler DLL. These are not equivalent to
substituting a third-party filesystem-ranker behind Windows' `file:`
protocol or `SearchHost`, and cannot by themselves guarantee that
an existing Start search view changes.

The documentation describing **Start menu Files group** inclusion via
`PHSearchConnectors` is expressly about **Windows 7**:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-search-connector.
Do not infer that the exact same UI behavior is available in current
Windows 10 or Windows 11 builds. The connector settings are not
installed or modified by this project.

A supported **custom, nonduplicate** data source is the only candidate
for an isolated protocol-handler proof. Re-indexing existing `file:`
items under private `searchtool:` URLs risks duplicate or broken
open-result actions and does not meet the desired experience. A proof
must verify, in *both Windows 10 and 11*, that an isolated synthetic
custom item is visible using the **native Windows-key, Start and Win+S
flow**, without any extra visible application, and that the original
app/settings/file results are unaffected. Explorer and SystemIndex
results alone are necessary but insufficient evidence.

The new `scripts/windows-search-provider-feasibility.ps1` is a
strictly **read-only** probe with no COM registration, registry writes,
user input or SearchHost interaction. It records only the Windows
version family, `WSearch` state and the proof checklist. Even with
`WSearch=Running`, it returns `NOT_ACCEPTED` because there is no
validated native Start result fixture; it is intentionally safe on
interactive user PCs. Windows PowerShell 5.1 and PowerShell 7
`-SelfTest` cover four distinct states: absent Start proof,
stopped Search index, non-target Windows Server and a candidate that
has all isolated fixture evidence. The latter still cannot authorize
a release. CI tests fail-closed, not falsely PASS as product acceptance.

**2026-10-15 read-only output:** Windows 11 Pro build 22631,
`WSearch=Running`, provider candidate `ISearchProtocol +
IUrlAccessor + IFilter + ShellNamespace`, native result participation
`BLOCKED` pending fixtures. Production Windows Search untouched.

### Next engineering steps (not acceptance-complete)

1. Build an out-of-process, low-privilege, synthetic Shell namespace
   and protocol-handler fixture for a **new** isolated data source,
   without opening/SearchHost UI injection and without mapping any
   existing private user files into the provider.
2. Register it **only** in disposable Windows 10 and Windows 11 VMs
   with snapshot rollback, narrow crawl scope and private scheme.
   Measure its presence in SystemIndex, Explorer and *real* Start/Win+S
   separately; verify files/apps/settings and IME behave normally.
3. Unregister/restore exact shell state, verify no orphan indexing
   scope/service or duplicate result. If native Start does not display
   it, reject the route and stop rather than shipping a second popup.
4. Only after the isolated native proof may we discuss production
   provider registration or full user acceptance. Installation and
   package sealing remain gated; PR #81 remains DRAFT and release
   INVALIDATED.

## Registered Windows IFilter extension candidate, compile-only P0 fixture

Development proof (2026-10-15): `experiments/native-ifilter-fixture/`
contains a functional C++ Windows SDK `IFilter` + `IPersistStream`
COM DLL for a new synthetic `STFIXTURE-TEXT-1` format. MSVC built the
DLL and a separate direct-`DllGetClassObject` test executable without
registration, and the executable returned:

`NATIVE_IFILTER_COM_FACTORY_STREAM_UTF8_REGISTRATION_DENIED_PASS`

Positive/negative tests cover COM reference ownership, no aggregation,
UTF-8/Turkish/Greek content extraction via real chunked `GetText`,
malformed format, invalid UTF-8, overlong stream rejection and
`DllRegisterServer`/`DllUnregisterServer` refusing OS registration.
A new Windows CI step runs the compile/direct-COM test without touching
SearchHost, shell associations, the Windows Search index or the registry.
The fixture and build/test conditions are documented in its local README.

**This is an implementation, but NOT native Search integration.**
The test DLL is not registered anywhere, does not expose a protocol
handler/Shell namespace data source, does not index user files and
does not show any result in Start/Win+S. It is a narrow example of
Microsoft's *new-format text extraction* extension point, not a
backend swap for the user's entire Rust file-search index. Any VM
installation, proprietary data-source crawling and Start visibility
remain separate P0 physical acceptance work. Release stays INVALIDATED.

## Unregistered ISearchProtocol + IUrlAccessor COM pipeline proof

Added a second standalone, registration-disabled C++ Windows SDK DLL
(`experiments/native-ifilter-fixture/protocol_fixture.cpp`) implementing
`ISearchProtocol` and `IUrlAccessor` for **one synthetic URL only**:
`stfixture-proto-test://root/document`. The custom protocol accessor
reports synthetic MIME/name/size/time and supplies the UTF-8 fixture
document using a fresh in-memory COM `IStream` on each call. Unknown
paths, user file URLs, calls before Init and after ShutDown are rejected.

The new test `protocol_test.cpp` directly loads both native DLLs by
file path, validates COM lifetimes/security refusals, then binds the
protocol accessor to an `IStream`, loads that stream into the
independent custom `IFilter`, enumerates its text chunk and verifies
the full expected Turkish/German/Greek UTF-16 content. It emits
`NATIVE_PROTOCOL_TO_IFILTER_END_TO_END_NO_REGISTRATION_PASS`.
MSVC `/W4 /WX` compile and local direct-COM test PASS on Windows.
Both DLLs refuse `DllRegisterServer` and `DllUnregisterServer`
with `E_ACCESSDENIED`; no OS integration is installed.

This advances the **test-only** protocol-to-filter plumbing but is not
a fully indexable or secure Windows Search extension:
`IUrlAccessor::BindToFilter` and per-user security descriptors are
unimplemented; there is no Shell namespace registration, container
child enumeration, crawl scope, real SearchProtocolHost invocation, or
visible Start/Win+S result. The handler cannot be installed on the
interactive user PC and must not be released. Full native Windows 10/11
acceptance remains **BLOCKED** pending authorized disposable VM proof.
