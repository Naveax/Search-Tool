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

### Real Windows Search GetFilter selection rejected: HRESULT classification (2026-10-16)

The hosted Windows 11 ARM64 job
[#38955259780](https://github.com/Naveax/Search-Tool/actions/runs/38955259780)
confirmed our *corrected* extension-only
`IProtocolHandlerSite::GetFilter(nullptr, nullptr, L"stfixture", ...)`
selection still failed **three times**. ACL creation and all three
`BindToFilter` invocations succeeded, but no private filter factory,
IFilter Init/GetChunk, or SystemIndex item was observed. The
Windows 11 ARM guest completed SUCCESS and restored all private test
registrations and WSearch. Thus the next blocker is explicitly
**filter-host lookup/activation** rather than the earlier
service-token ACL failure.

The next test-only ETW markers classify the failing HRESULT without
recording any arbitrary process/user data: class-not-registered,
access-denied, invalid-argument, not-implemented, file-not-found,
not-found, generic failure, module-not-found, or other. EventWrite
still carries **zero payload descriptors**. A categorical HRESULT
alone does not authorize bypassing the isolated SearchFilterHost
or directly loading the filter DLL in the privileged protocol host.

## Synthetic Shell SFGAO_STREAM / read-only IStream contract (2026-10-17)

The registrationless, one-item `private_shell_namespace_test.cpp`
previously advertised `SFGAO_STREAM` through `GetAttributesOf`
while refusing `IShellFolder::BindToStorage(IID_IStream)` with
`E_NOTIMPL`. This was a real internal Shell capability mismatch:
the item claimed a stream yet could not supply that stream.

The in-process test-only Shell folder now exposes a bounded,
immutable **fixed synthetic memory stream**, from both
`BindToStorage(IID_IStream)` and
`BindToObject(IID_IStream)`, using independent COM objects.
It never reads disk, indexes user files, opens Explorer, starts a
second search panel, registers Shell COM objects, or installs anything.
Other item PIDLs, unsupported `IStorage`/folder interfaces,
and write/resize/commit operations fail closed; the fixture's
single-user/no-rename scope remains unchanged.

The native C++ selftest verifies the stream's exact fixed bytes,
`STATSTG` length and read-only mode, partial/full reads, EOF,
seek reset, independent `Clone`, fresh `BindToObject` content,
and denial of mutable storage. This is only a **Shell API contract**
test, **not a demonstrated Explorer window/namespace registration,
SearchHost results, native Windows 10/11 Start/Win+S ranking, or a
releasable product**.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/shell/sfgao
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellfolder-bindtostorage

## Shell URI negative control and Windows PowerShell 5.1 CI harness repair (2026-10-18)

The ARM64 hosted CI at 6d7a8da proved SystemIndex exact-URL/full-text
acceptance and reported RESOLVED_EXACT_PRIVATE_URI, but the test wrapper
read a blank Start-Process -PassThru ExitCode under Windows PowerShell 5.1.
It falsely marked the Shell URI probe as process-rejected after receiving
the positive category. Guest cleanup and WSearch restoration both passed.
This was a CI harness issue, not a SystemIndex failure.

The guest wrapper now uses an explicitly owned System.Diagnostics.Process
with typed exit code, bounded wait/output, private output/error readers
and guaranteed cleanup. Independent local Windows PowerShell 5.1 probes
confirmed exit codes 0 and 7 using the same .NET process API.

IMPORTANT: Windows Shell may resolve arbitrary URI strings even when they
are not indexed or exposed through an installed Shell namespace source.
The C++ guest probe now tests a second fixed, deliberately UNINDEXED URL
within the private scheme. If Shell accepts it, it reports
GENERIC_URI_PARSER_ACCEPTS_UNINDEXED_CONTROL; if rejected, it reports
PRIVATE_URI_RESOLVED_CONTROL_REJECTED. Both are diagnostics, NOT
Explorer query-result visibility or native Start/Win+S acceptance.
Neither category can authorize release or change the existing UI.

The physical developer machine only compiled the guest-specific binary,
ran native COM selftests and performed read-only diagnostics. No installed
Windows Search, user index, Shell registry, desktop input or user data
was changed. Exact-head hosted ARM64 results must be checked separately.
## Guest-only Windows Shell private URL resolution boundary (2026-10-18)

The existing hosted Windows 11 ARM64 test proved exactly one synthetic
private document in Windows Search SystemIndex, but it did not prove that
the Shell can materialize that indexed URL. This is a distinct acceptance
boundary required before any Explorer-visible integration may be claimed.

\`private_shell_resolution_guest.cpp\` adds a *read-only* fixed-URL
\`SHCreateItemFromParsingName\` / \`IShellItem\` probe for the sole
\`stfixture-proto-test://root/document\` fixture. It is compiled on developer
machines but is executed only by \`native-search-ephemeral-arm-com-smoke.ps1\`
after the **strict GitHub-hosted Windows 11 ARM64 admission gate** and
after the isolated SystemIndex fixture's positive acceptance. The
guest script launches the probe for at most 15 seconds, kills it on
timeout, validates only a fixed category and hexadecimal HRESULT, and
removes temporary stdout/stderr files in finally. A Shell item with
a nonmatching or unavailable parsing name is explicitly categorized as
NONMATCHING_PARSING_NAME, never accepted as native integration.
Its returned name is never printed. Nonzero process failures still fail CI
with only a bounded exit code/category emitted. No other files,
URLs, SIDs, windows or user documents are queried or logged.

\`UNRESOLVED\` means this particular Shell parser cannot materialize the
private indexed URL via the current protocol-only guest registration.
It does **not** negate the prior OLE DB SystemIndex proof. Conversely,
\`RESOLVED_EXACT_PRIVATE_URI\` establishes only programmatic Shell URL
resolution, *not* Explorer search-result visibility, native Start,
Win+S, keyboard/IME, visual parity, or a usable product. Neither result
changes the no-release decision.

Actual hosted ARM64 observation and complete CI are pending the new
exact-head run. Physical Windows Search and installed applications remain
unchanged. Source: Microsoft's Windows Search as a development platform
and Shell data source requirements:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-extidx-prot-implementing
## Windows Shell item factory interoperability (2026-10-17)

The one-item in-process namespace now verifies an additional **real OS
consumer API**, `SHCreateItemWithParent(nullptr, IShellFolder*, childPIDL,
IID_IShellItem, ...)`, on Windows MSVC x64. The Windows-supplied
`IShellItem` is successfully constructed from the private parent and
relative child PIDL. Its public `GetDisplayName(SIGDN_NORMALDISPLAY)`
returns the exact fixed synthetic display name. Its `GetAttributes`
returns `SFGAO_STREAM | SFGAO_READONLY`, and reports no filesystem or
folder attributes. All objects and allocated strings are released.
These checks prove only in-process Shell item interoperability with
a fixed private item, not shell namespace registration or Explorer,
Start/Win+S, SearchHost result display, performance or release fitness.

Official API contracts:
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-shcreateitemwithparent
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitem-getattributes
## Supported default Shell view factory, registrationless proof (2026-10-17)

On top of the pre-existing exact one-item `IShellFolder2` /
`IPersistFolder2` / `IEnumIDList` C++ fixture, the private
`IShellFolder::CreateViewObject` now implements Microsoft's
documented default DefView construction via
`SHCreateShellFolderView(SFV_CREATE, IShellView**)`.
It returns a fresh, independently owned `IShellView` object
on each requested view, rejects unsupported interface IDs,
and fails closed if the private folder is not initialized.
The registrationless MSVC x64 native test verifies two
distinct `IShellView` instances, COM release and negative
pre-initialization / unsupported-interface behavior. The
caller never invokes `IShellView::CreateViewWindow`.
**No Explorer process is started, no HWND is created by
the test, and no COM Shell namespace registration,
Windows Search service change or real file read occurs.**

Microsoft specifically lists `IShellFolder`,
`IShellFolder2`, `IPersistFolder` and `IPersistFolder2`
as the required data-source interfaces for this default
Shell view. This is **only an in-process factory contract**.
The private fixture still has no installed Shell data
source or real Explorer integration; `BindToStorage`,
leaf opening, full Explorer navigation, shell item verbs,
Start/Win+S results, keyboard/IME/accessibility testing
and two-OS interactive acceptance are unproven.
`CreateViewObject` success cannot make the separately
maintained Rust index appear in native Windows Start
search or change default file ranking. PR #81 must stay
DRAFT and the release remains INVALIDATED.

Official references:
https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shcreateshellfolderview
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellfolder-createviewobject

## In-memory Shell namespace property and root-PIDL contract (2026-10-17)

The prior exact branch checkpoint `6175e1d26f849890c3c82d1c36dc68950cf04eff`
completed all three CI jobs successfully
([Actions #39012072787](https://github.com/Naveax/Search-Tool/actions/runs/39012072787)).
That commit already proved basic `IShellFolder` enumeration, synthetic
PIDL validation and names **without registering a Shell extension**.

This incremental, registrationless C++ test extends the same
single-item synthetic object to the documented `IShellFolder2`
and `IPersistFolder2` contracts:

* `QueryInterface` for both extended interfaces.
* Two fixed, deterministic Shell columns:
  `PKEY_ItemNameDisplay` = `document.stfixture`;
  `PKEY_ParsingPath` = `stfixture-proto-test://root/document`.
  It implements the default column, column state, mapping,
  `GetDetailsOf` and `GetDetailsEx`; COM-owned BSTR/STRRET
  allocations are released by the test caller.
* Rejects foreign PIDLs, additional columns and unrelated
  Shell property keys. It deliberately supplies no filesystem
  identity or arbitrary user properties.
* `IPersistFolder2::GetCurFolder` has documented pre-init
  `S_FALSE` semantics, and returns an independent caller-owned
  copy of the original synthetic absolute root PIDL after init;
  mutation/release of one copy cannot affect subsequent copies.
* `GetDefaultSearchGUID` and `EnumSearches` remain
  `E_NOTIMPL`, deliberately not impersonating an actual Windows
  Search UI extension.

MSVC x64 and the existing registrationless native COM suite
pass locally. The same builder is already part of CI's Windows
x64 and Windows 11 ARM64 test jobs; new exact-head hosted CI
must still verify this extension.

**Limit of evidence:** This is **not** a COM-registered Explorer
namespace data source. `CreateViewObject`, `BindToStorage`,
Shell item opening, user interaction and live Explorer search
results remain unsupported. It does not rerank the user's
existing files in Windows 10/11 Start/Win+S/SearchHost. PR #81
remains DRAFT, release INVALIDATED; no user OS configuration or
actual Windows Search index was modified by this local test.

Official references:
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ishellfolder2
https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ipersistfolder2

## Passive native SearchHost UIA role/focus instrumentation (2026-10-17)

The latest exact-head [CI #38983663713](https://github.com/Naveax/Search-Tool/actions/runs/38983663713)
completed **all three jobs SUCCESS**, including Win11 ARM64
private exact-URL + content-token index acceptance and Windows x64
pristine-install/rollback tests. The Windows 11 physical read-only
probe still reports a running WSearch service, an Explorer-owned
taskbar `SearchButton`, and **no visible uncloaked native SearchHost
window at observation time**. That does *not* prove an interactive UI
failure, only absence of an observable open native Search panel.

New `scripts/windows11-search-native-uia-readonly.ps1` adds the next
non-invasive prerequisite to genuine UX acceptance. It first invokes
the existing strict live HWND/DWM geometry + physical monitor audit;
if there is no **single** credible visible Windows 11 SearchHost frame,
it returns `BLOCKED` without activating Search. Only if a visible
frame exists does it read UI Automation **ControlType**, is-offscreen,
keyboard-focusable and has-keyboard-focus flags within a bounded
same-session SearchHost/SearchApp UI subtree whose root rectangle
matches the independently audited native frame (24-pixel tolerance).
The traversal limits depth to five and visits at most 160 nodes.

The new script **never requests** a UIA `Name`, `Value`,
`TextPattern`, search query, filename, window title, user path,
screenshot or process command line; it neither launches SearchHost
nor clicks, sends keyboard input, changes focus or changes the index.
Its only output is fixed-schema aggregate role counts and a
`BLOCKED`/`PARTIAL_READ_ONLY_ROLES` classification. Even with
focus metadata present, native keyboard/IME/Narrator/visual behavior
and Search Tool result integration remain `NOT_ACCEPTED`.
The PowerShell 5.1 and PowerShell 7 selftests cover control roles,
offscreen/focus treatment, the untrusted-frame gate, absent controls,
and privacy fail-closed behavior; CI runs the selftests only, never
opens a desktop search surface.

Actual physical Naveax execution returned
`NO_UNIQUE_VERIFIED_NATIVE_SEARCH_FRAME`; no native input or
visual acceptance was claimed. This is a reusable passive capture
for a future **already opened** authentic Search panel, not a
substitute for user-supervised physical interaction tests.

The source-only AST installer guards and real native indexing
regression remain intact. PR #81 OPEN/DRAFT;
`docs/RELEASE_STATE.json.package.status` INVALIDATED.

## Read-only physical native UX baseline and stronger installer no-takeover guard (2026-10-17)

The latest exact-head CI
[#38982202468](https://github.com/Naveax/Search-Tool/actions/runs/38982202468)
completed **all three** hosted jobs successfully, including the strict
disposable Win11 ARM64 `SystemIndex`/exact synthetic URL/full-text
acceptance test. This does not mean physical Start/Win+S/Explorer UX
is complete.

Additional live checks were performed in **read-only mode** from the
existing Naveax development worktree. No SearchHost activation,
screenshot, mouse/keyboard input, desktop click, installation,
registry/service mutation or Windows indexing operation was performed:

* `windows-native-search-audit.ps1`: WSearch Running; standard
  `Search.CollatorDSO.1` could return one indexed result without
  revealing its name; an Explorer-owned SearchButton exists in the
  taskbar. This confirms that Windows' own Search entrypoint is
  present, **not** that Search Tool's ranking feeds it.
* `windows11-search-live-window-audit.ps1`: physical OS build
  `10.0.22631` (Windows 11), 3440×1440 monitored desktop.
  **BLOCKED:** there was no visible, uncloaked SearchHost/SearchApp
  candidate with a measurable actual frame during this passive
  observation. No native flyout capture or physical interaction
  acceptance was obtained.
* `native-search-experience-contract.ps1`: existing installer
  inspection PASS with no default companion GUI startup, while
  native result integration and physical interaction remain BLOCKED.
* `windows-search-provider-feasibility.ps1`: private synthetic
  protocol participation is proven, but a **Shell namespace +
  Explorer results + current Win10/11 native Start result proof**
  is still needed for an end-user native data-store extension.
  Microsoft Windows Search platform guidance explicitly requires a
  Shell data source to make custom store results available from
  Explorer. A Win7 search connector reference is **not** a Win11
  Start/SearchHost compatibility guarantee.

**New regression protection:** The installer policy now parses the
PowerShell syntax tree with the native PowerShell AST parser (without
executing the installer). It rejects direct mutations to
`HKLM/HKCU/HKCR ... Classes\search` and `search-ms`, deletion
of the Windows-owned search class, direct auto-start
`CurrentVersion\Run/RunOnce` writes, `schtasks.exe`,
`Register-ScheduledTask` and shortcut-creation member calls.
Exactly one deliberately scoped legacy cleanup of
`search\OpenWithProgids\SearchTool.Search` is allowed; the
private `searchtool:` registration remains allowed. Multiple
adversarial syntax/casing and positive cleanup controls PASS on both
Windows PowerShell 5.1 and PowerShell 7, with a real repository
source check. The AST guard covers identifiable direct commands,
**not** dynamic/evasive arbitrary code; future installer changes
still require manual security review and reversible isolated tests.

These additions **do not** demonstrate that a proprietary Rust index
can legally replace the native Windows 11 Start search ranking.
The physical interactive, visual, IME/focus, accessibility and
supported-native-result tests remain blocked. Keep PR #81 DRAFT
and `docs/RELEASE_STATE.json.package.status=INVALIDATED`.

Sources:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-extidx-prot-implementing

## First real private SystemIndex + content acceptance; now a strict CI regression gate (2026-10-17)

**Critical resolved blocker.** Genuine Windows 11 ARM64
[CI #38981622893](https://github.com/Naveax/Search-Tool/actions/runs/38981622893),
commit `a44702612d3a91a8bdd6149ac997014df441f360`,
ran exclusively in a disposable Microsoft-hosted VM and
**successfully indexed the exact private synthetic document**:

* `NATIVE_SEARCH_HOSTED_PRIVATE_ACL_OWNER_ACCESSCHECK_READ_GRANTED=1`:
  Windows itself granted the fixture owner `FILE_GENERIC_READ`
  against the returned self-relative security descriptor.
* A documented `ISearchNotifyInlineSite` callback for the
  indexer-assigned private DocID reported both QUERYABLE
  `S_OK` and PERSISTED `S_OK`.
* The catalog-wide item count increased from 1325 to 1326
  immediately after the private document notification.
* `NATIVE_SEARCH_HOSTED_PRIVATE_SCHEME_INDEX_OBSERVATION=PRIVATE_SCHEME_ROWS_FOUND`:
  1 private-scheme OLE DB row found.
* `NATIVE_SEARCH_HOSTED_PRIVATE_DIRECT_TEXT_DOCUMENT_INDEXED_EXACT`:
  the exact `stfixture-proto-test://root/document` URL
  was returned as its own `System.ItemUrl`.
* `NATIVE_SEARCH_HOSTED_PRIVATE_CONTENT_TOKEN_OBSERVATION=SYNTHETIC_MARKER_MATCH_VISIBLE`:
  the fixed synthetic stream token was searchable via a
  **rank-only** full-text query. No user URL, SID or indexed
  content was exported.
* The temporary COM/CSM registry entries were removed
  (PASS), and the Windows Search service was restored to
  Running (PASS).

**Root cause and fix:** The generated restrictive synthetic
DACL was valid enough for basic checks but its security
descriptor omitted Group SID; Microsoft `AccessCheck`
requires both Owner and Group SIDs. Restoring the Group SID
(using the same trusted fixture owner as metadata, not as an
additional ACE) changed the real guest's AccessCheck from
failure to success **without weakening the one-user DACL**.
Native x64 registrationless unit tests now assert both
SID fields and invoke real `AccessCheck` as well.

**Regression gate:** `native-search-ephemeral-arm-com-smoke.ps1`
now fails the disposable-guest CI unless the exact document
URL is found, precisely one private-scheme indexed document
row is observed, the query-user token matches the private
source owner, **and** the fixed synthetic content token is
visible. False-positive or missing-data cases are exercised
by Windows PowerShell 5.1 and PowerShell 7 self-tests.
Failure does not bypass the mandatory finally-block
private-registry/CSM cleanup. A successful regression gate
proves **only this synthetic private index fixture**, not
native end-user Search results or production release safety.

**Remaining:** Real Windows 10/11 Start, taskbar Search,
Win+S, SearchHost and Explorer integration/visual keyboard
focus accessibility acceptance remain unproven. The current
PR #81 is OPEN/DRAFT and `package.status` remains
`INVALIDATED`. Do not deploy or change production search
entrypoints based solely on this fixture success.

## P0 fix: missing Group SID in self-relative ACL (2026-10-17)

The first live Windows 11 ARM64 test of
`f919b84668e07b0d880697ba6a876a29d3a5536f`,
[CI #38981260948](https://github.com/Naveax/Search-Tool/actions/runs/38981260948),
exposed a real **security-descriptor validity problem**:
`NATIVE_SEARCH_HOSTED_PRIVATE_ACL_OWNER_ACCESSCHECK_READ_GRANTED=0`.
The guest fail-closed COM activation smoke aborted before the
crawl and safely removed all temporary registration; this run
cannot be cited for new query/content observations.

Microsoft's official `AccessCheck` contract says an access
check fails with `ERROR_INVALID_SECURITY_DESCR` when the
security descriptor lacks either an owner SID **or a group SID**.
Source review found our `build_private_test_descriptor` set
the owner and restrictive one-user DACL but never set
`SetSecurityDescriptorGroup`. Earlier checks verified owner,
ACE, SID equality and `IsValidSecurityDescriptor`, yet never
verified the group or performed a true Windows `AccessCheck`.
This explains why those tests missed the problem, although the
first live failure did not log a Win32 code to identify its
specific cause.

The private fixture now **sets the Group SID to the same
trusted synthetic source owner SID** before producing the
self-relative descriptor. This is solely a security-descriptor
metadata field; it does not add an ACE or broaden the DACL.
Production handlers must use authoritative data-store identity
metadata and must not copy this fixed private fixture policy.

The existing registrationless native x64 C++ test now asserts
that the returned security descriptor contains **both** an
owner and valid group matching the authorized source identity,
and performs a real Windows `AccessCheck` with an
impersonation copy of the owner token and
`FILE_GENERIC_READ`. Both the 1-user DACL restriction and
the OS access decision must pass. This updated native
test **PASSED locally**, along with existing COM, PowerShell
guest-eligibility and source safety tests. A new, independent
actual Win11 ARM64 guest run is still required to prove both
its `AccessCheck` outcome and the indexed URL's actual OLE DB
query visibility.

Release remains INVALIDATED, PR #81 DRAFT. Neither OS-index
ACLs, system-wide search roots, Shell entrypoints, production
Windows search settings, nor user files were modified on
Naveax/work-pc.

Official reference:
https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-accesscheck

## Windows AccessCheck against actual private item ACL and fixed-content query (2026-10-17)

The hosted [CI #38980016635](https://github.com/Naveax/Search-Tool/actions/runs/38980016635)
is now entirely green. In its disposable Windows 11 ARM64 guest,
`ISearchCatalogManager::NumberOfItems` increased from 1325 to
1326 after our exact synthetic document notification; the matched
DocID received QUERYABLE and PERSISTED callbacks with S_OK. The
same VM could return a numeric
`System.Search.Rank` row from `Search.CollatorDSO.1`, and the
query user's SID matched the synthetic owner SID. Nevertheless a
bounded query over **all** `stfixture-proto-test:` URL shapes
returned zero rows. The failed `COUNT(*)` query is not treated
as evidence that all provider queries fail, since Windows Search
uses a specialized SQL dialect.

The next **synthetic-only** CI guest diagnostic checks two
independent hypotheses without printing user information:

* The already-registered private COM activation smoke retrieves
  the actual self-relative `GetSecurityDescriptor` bytes in
  memory and calls Windows `AccessCheck` for
  `FILE_GENERIC_READ` using an impersonation copy of **the
  exact query/fixture owner process token**. Its only new output
  is `NATIVE_SEARCH_HOSTED_PRIVATE_ACL_OWNER_ACCESSCHECK_READ_GRANTED=0/1`.
  Neither ACL bytes, SID, privileges nor the user's token are
  sent to the build log. The COM smoke test refuses success
  if the restricted ACL does not grant owner-read. The smoke
  now correctly performs its own final `IUrlAccessor::Release`
  *after* `CloseAccessor`, matching host ownership.
* The guest-only OLE DB probe executes a fixed, synthetic-token
  full-text query
  `SELECT TOP 1 System.Search.Rank FROM SYSTEMINDEX WHERE CONTAINS('STFIXTURE')`.
  The known token is contained in the one hardcoded private
  in-memory IFilter payload. Only whether a numeric-result
  row matched, or an enumerated provider error, is returned.
  **No URL, path, filename, SID or indexed content is selected
  or printed**. A positive token match is still only
  corroboration, not exclusive proof of the source item's
  URL or authorizations.

Both experiments continue to run only in the already guarded
disposable GitHub-hosted Windows 11 ARM64 VM. They make no
production registry, Windows Search index, service, taskbar,
Start, Explorer or Win+S changes. The exact private-protocol
index SQL probe and cleanup remain independent release gates.
PR #81 DRAFT, release INVALIDATED.

## Catalog +1, owner match, but SQL COUNT unsupported: numeric visibility control (2026-10-17)

In [Windows 11 ARM64 CI #38979297058](https://github.com/Naveax/Search-Tool/actions/runs/38979297058),
the query-visible private-scheme count remained **0** and
the indexer-managed `QUERYABLE` and `PERSISTED`
callbacks for the synthetic DocID returned `S_OK`.
The documented catalog-wide `NumberOfItems` aggregate
was **1325** through the initial trials, increased to
**1326** immediately after the single synthetic item
notification, and remained 1326 through the private
document trial. This temporal correlation strengthens
the case for a new indexed item, but the aggregate
cannot prove which item changed.

The VM query user's SID was found to **match** the fixture
owner SID stored privately at registration. This verifies
identity equality only, not the effective DACL evaluated
by Search.CollatorDSO.1. The global OLE DB aggregate
`SELECT COUNT(*) FROM SYSTEMINDEX` returned
`SCALAR_COUNT_UNSUPPORTED_OR_FAILED`; Windows Search
SQL support for this aggregate remains unproven and
must not be treated as evidence of zero visible rows.

As a control, the disposable Windows 11 ARM64-only
provider probe now separately executes
`SELECT TOP 1 System.Search.Rank FROM SYSTEMINDEX`.
It only tests whether **a single non-identifying numeric
rank projection** can return any row to the query user's
token. It never selects, prints or stores paths,
URLs, file contents, names, item IDs, or SIDs.
A missing row, unsupported projection, or a provider
failure have distinct enumerated outcomes. On failure,
only a numeric HRESULT is exposed, not exception
messages that may contain sensitive URLs/queries.
This measurement remains separate from the fixed
private-scheme query, private owner identity comparison,
and catalog aggregate count. PR #81 remains DRAFT
and release state remains `INVALIDATED`.

## Query-token ACL owner and aggregate catalog visibility controls (2026-10-17)

Exact-head disposable Windows 11 ARM64
[CI #38977701322](https://github.com/Naveax/Search-Tool/actions/runs/38977701322)
returned the following **for the same indexer-assigned private
document DocID**: `ISearchNotifyInlineSite` QUERYABLE callback
`S_OK`, PERSISTED callback `S_OK`, with no Gatherer callback
observed. Nevertheless the Windows OLE DB
`Search.CollatorDSO.1` query found **zero** private-scheme
rows. The stage callback is evidence of indexer-reported phase
status, but not yet actual query visibility or Windows Start/
Explorer/Win+S integration.

The next strictly guest-only diagnostic separates three hypotheses
without exporting a user path, SID, file contents or catalog URLs:

* `ISearchCatalogManager::NumberOfItems` returns the COM
  indexer's **aggregate total item count** in the existing
  read-only `crawl_scope_readonly.exe`. It is inspected alongside
  the already documented aggregate queue/status properties;
  no `URLBeingIndexed` or real paths are retrieved.
* The disposable VM's OLE DB query provider executes
  **only** `SELECT COUNT(*) FROM SYSTEMINDEX` as an aggregate
  scalar health control, in addition to its hard-coded
  private-scheme URL query. The total number of visible rows
  is emitted without any individual URLs. If the provider
  rejects aggregate SQL, the diagnostic explicitly reports
  `SCALAR_COUNT_UNSUPPORTED_OR_FAILED`, not zero.
* The CI runner's current Windows token SID is compared in
  memory to **only** the protected `FixtureOwnerSid` value
  installed under the private synthetic COM CLSID on that same
  disposable VM. The output is a Boolean identity match and
  constrained source-status enumeration; neither SID is logged.
  This proves a token/owner identity match only, **not**
  the entire effective Windows Search access-control decision
  or the on-disk index ACL.

The global aggregate OLE DB query, registry read and all
identity comparisons are performed **after** the already
established GitHub-hosted Windows 11 ARM64 VM identity gate.
The physical developer machines can run registrationless
self-tests but cannot reach the live query code.
The original exact/private-scheme OLE DB result is reported
independently, all mandatory private-registry and crawl-scope
cleanup remains, PR #81 stays DRAFT and
`docs/RELEASE_STATE.json` remains `INVALIDATED`.

## Real per-DocID indexing phases via provider-managed callbacks (2026-10-17)

Existing Windows 11 ARM64
[CI #38975034381](https://github.com/Naveax/Search-Tool/actions/runs/38975034381)
proved three successful, **separate** queue-acknowledgement
HRESULTs (`GetPersistentItemsChangedSink`, `OnItemsChanged`,
per-URL result all `S_OK`), with **no persisted SystemIndex
item under any private URL shape** after the bounded observation.
Microsoft's indexer-managed notification API acknowledges queue
acceptance only; it does **not** report whether content was
gathered, queryable, or persisted.

As a deliberately **independent test** confined to the
disposable Microsoft-hosted Windows 11 ARM64 VM, the existing
privately gated `crawl_scope_guest_trial.exe` now supports
`monitor-document`. It implements `ISearchNotifyInlineSite`
and uses the documented provider-managed
`ISearchCatalogManager::GetItemsChangedSink` and
`ISearchItemsChangedSink::OnItemsChanged` interfaces for
**only** `stfixture-proto-test://root/document`, one ADD
notification with NORMAL priority.

The indexer-assigned DocID is never logged. Only callbacks
whose DocID matches the just-assigned synthetic item are
recorded; callback phases
`SEARCH_INDEXING_PHASE_GATHERER`,
`SEARCH_INDEXING_PHASE_QUERYABLE`, and
`SEARCH_INDEXING_PHASE_PERSISTED` have separate counters
and most recent HRESULTs. Other source URLs, document IDs,
catalog GUIDs, tokens or user information are neither
queried nor output. The temporary process waits at most 30
seconds for its *own* persisted callback, releases the
COM sink and observer, and exits; no persistent listener is
installed. A missing callback is explicitly an unobserved
phase, not a success or permanent rejection. Any absence or
error of the provider-managed sink is reported as its HRESULT,
not hidden by the prior persistent-sink success.

A registrationless MSVC C++ unit test verifies that a foreign
DocID cannot advance the synthetic item's phase counts and
that the synthetic Gatherer/Queryable/Persisted callbacks are
distinguished correctly. The CLI is proven to reject execution
on the physical developer machine (`0x80070005`), before
any COM Search manager access. The extra test does not change
standard `file:` rules, the system search UI, or source ACLs.
The prior exact and private-prefix SystemIndex queries still
independently gate real persistence and release remains
`INVALIDATED`.

Microsoft reference:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-notifyingofchanges
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-isearchcatalogmanager-getitemschangedsink

## Direct indexer queue acknowledgement for one synthetic item (2026-10-17)

After the previous disposable Windows 11 ARM64 run
[#38974164799](https://github.com/Naveax/Search-Tool/actions/runs/38974164799),
a bounded read-only private-scheme query found **zero**
`SystemIndex` rows for all canonical, root, SID-scoped and
unknown shapes. This was not an exact-URL alias issue.
Nevertheless, the real protocol still returned successfully
loaded in-memory IFilter text through Windows Search's
`SearchProtocolHost`.

Microsoft documents a narrower stage-specific mechanism:
`ISearchPersistentItemsChangedSink::OnItemsChanged` returns
both a **call HRESULT** and a **per-URL completion HRESULT**
for indexer queue acceptance. We have added a *single-item*
`notify-document` command to the already tightly guarded
`crawl_scope_guest_trial.exe`. Only the fixed
`stfixture-proto-test://root/document` URL is supplied with
`SEARCH_CHANGE_ADD` and normal priority. No user URL, wildcard,
file path or SID can be passed by the caller. The operation
requires the disposable GitHub-hosted Windows 11 ARM64 VM
guard exactly as the existing private CSM operations do.

The guest test invokes this command **after** confirming its
one nonhierarchical private document root and its scope rule,
before the bounded OLE DB queries. It prints only
`NATIVE_SEARCH_HOSTED_PRIVATE_NOTIFY_SINK_HRESULT`,
`...NOTIFY_CALL_HRESULT`, and
`...NOTIFY_URL_ACK_HRESULT`, formatted as hexadecimal
numbers. Missing/supported notification sink and negative
completion codes remain explicitly diagnostic; they do not
weaken mandatory root/rule rollback. Even if the indexer
accepts a notification, **the URL has not been proven persisted**
unless the separate exact and private-prefix SystemIndex
queries actually find a row. The release and original native
Windows entrypoint gates remain blocked.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-notifyingofchanges
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-isearchpersistentitemschangedsink-onitemschanged

## Disposable guest private-scheme SystemIndex URL-shape audit (2026-10-17)

Existing exact read-only SystemIndex checks cover the fixed
`stfixture-proto-test://root/document` and the
`stfixture-proto-test://root/document/` canonical alias.
This cannot distinguish **no indexed private items at all**
from **an indexed item under a third, unexpected private URL**
without further evidence. The actual Win11 ARM64 crawl has
confirmed two COM API fixes (correct host-owned `CloseAccessor`
and `GetCLSID(E_NOTIMPL)`) but **neither exact index URL was
observed** after an extended direct-document crawl.

To resolve this ambiguity, new
`scripts/native-search-private-index-shapes.ps1` runs
**only** on a strictly gated disposable, Microsoft-hosted
Win11 ARM64 VM with Windows PowerShell 5.1. It performs a
bounded, read-only OLE DB `TOP 20` query over an immutable
`System.ItemUrl LIKE 'stfixture-proto-test:%'` predicate.
All matching URL strings remain entirely inside VM memory;
the script verifies the private protocol prefix before use,
classifies them in five predefined groups, and emits **only
category counts and observation status**. The groups distinguish
exact document URL, trailing-slash URL, private root,
SID-scoped document, and other private URL shape. It will
**not** emit even a test runner SID or unknown raw URL.
If 20 rows are encountered, coverage is `INCOMPLETE`; an OLE
DB error yields `QUERY_FAILED_OR_RESULT_UNTRUSTED`, never a
false-negative claim. Normal Windows PCs are rejected before
database access. There are PowerShell 5.1 and 7 self-tests,
including rejection of an unrelated `file:` URL.

The private-scheme audit executes while the exact synthetic
direct-document root is still active, immediately after the
existing 23 bounded exact queries and **before** the mandatory
private CSM, COM, ETW and WSearch cleanup. This is not a
system-wide index search, not a file crawl, not a Shell default
change and not proof of Start/Explorer results. Release stays
`INVALIDATED`; PR #81 remains DRAFT.

## Don't advertise an unavailable document storage CLSID (2026-10-16)

After the corrected, host-owned `CloseAccessor` lifecycle,
actual Windows 11 ARM64 CI
[#38972254788](https://github.com/Naveax/Search-Tool/actions/runs/38972254788)
returned four real `CloseAccessor` events, two successful
`IPersistStream::Load` and `BindToFilter` completions,
with proper isolated-guest registry cleanup and WSearch
restoration. However neither exact synthetic document URL
appeared in SystemIndex during the bounded crawl. Thus the
object-lifetime fix is necessary COM correctness but **not**
proof of persistence.

A second API review discovered that `GetCLSID` claimed
`S_OK` with `CLSID_NULL` even though the fixture has no
valid storage-document CLSID. Microsoft explicitly permits
`E_NOTIMPL` or `E_FAIL` when such class information is
unavailable. The private accessor now reports `E_NOTIMPL`
after initializing the output to CLSID_NULL instead of
reporting an invalid successful class. Native in-process tests
assert this behavior and fixed zero-payload ETW event 163
reports whether the real Search host called `GetCLSID`.
The rest of the private IFilter fallback and production
security restrictions are unchanged.

This does not claim that `GetCLSID` was responsible for
the absent row. A second genuine Windows 11 ARM64 guest CI
is required to prove its effect, and original Start/Win+S
integration remains unverified.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iurlaccessor-getclsid

## Fix real COM accessor ownership contract (2026-10-16)

A source-level review against Microsoft
`ISearchProtocol::CloseAccessor` documentation revealed a genuine
COM lifetime violation: the prior test fixture called
`accessor->Release()` **inside** `CloseAccessor`. Microsoft
explicitly specifies that **the protocol host releases the
IUrlAccessor pointer after `CloseAccessor` returns**. Therefore
the previous implementation could double-release/free the
object when exercised by the real Windows Search host. This
risk also existed after otherwise successful `BindToFilter`.

The fixture now treats `CloseAccessor` as a completion
notification only; it validates non-null, emits one fixed,
zero-payload ETW event `CLOSE_ACCESSOR_HOST_OWNS_RELEASE`
(ID 162), and returns S_OK. The unregistered MSVC mock tests
were corrected to model the actual host: every mock callback
performs exactly one final `IUrlAccessor::Release()`
**after** `CloseAccessor`. One test also accesses
`IsDirectory()` between the close notification and the host
Release, detecting premature destruction. Native tests pass
without registering or changing the physical machine's
Windows Search installation.

This is a documented correctness and memory-safety fix,
**not yet** proof that the Windows 11 ARM64 test document will
persist in `SystemIndex`. The separate exact-row query and
original Win10/Win11 Start/Win+S/Explorer UX gates remain
unmet until an isolated live test proves them.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-isearchprotocol-closeaccessor

## Correct IUrlAccessor string-length contract and longer hosted indexing window (2026-10-16)

The real, hosted Windows 11 ARM64 run
[#38966252584](https://github.com/Naveax/Search-Tool/actions/runs/38966252584)
confirmed 3 `IPersistStream::Load` successes and 3 final
`BindToFilter` successes for the one synthetic text document,
but **no** row in either exact SystemIndex URL query. The new
privacy-minimal Gatherer Application event probe matched **zero**
events with this private scheme. This is *no matching event
observed*, not proof of indexing success or proof that there
were no other internal indexer failures.

A source/API review also found a concrete conformance bug:
`IUrlAccessor::GetFileName` and `GetDocFormat` reported
`pdwLength` including the terminating NUL. Microsoft specifies
the returned character count **excludes** NUL, and
`GetFileName.dwSize` is likewise stated without the terminator.
The private prototype now follows the corresponding boundary
semantics, without changing any real file handler. New native
C++ tests assert the returned count for both filename and
MIME format, test the exact `GetFileName` declared buffer
length, and reject a buffer shorter by one character.
Fixed no-payload ETW IDs 152-161 will show whether Windows Search
actually calls and successfully consumes `GetDocFormat`,
`GetFileName`, `GetLastModified`, `IsDirectory` and `GetSize`.
Neither the values nor user paths are printed.

Separately, only the **single disposable Win11 ARM64 guest**
direct-document root is observed for a longer but still bounded
period: 23 exact, read-only polls rather than 7, spaced 4
seconds apart. The private root/rule deletion and WSearch
restoration remain mandatory. This tests whether the absent
SystemIndex row was caused by deferred index commits rather than
an immediate crawl failure. No physical Windows PC is modified.

References:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iurlaccessor-getfilename
https://learn.microsoft.com/en-us/previous-versions/bb231402(v=vs.85)

## Read-only, private-scheme Windows Search Gatherer event analysis (2026-10-16)

Exact-head Windows 11 ARM64 run
[#38964193626](https://github.com/Naveax/Search-Tool/actions/runs/38964193626)
demonstrated real, hosted, `IPersistStream::Load` success
(**3**) and `IUrlAccessor::BindToFilter` final success
(**3**) on our one fixed synthetic text document. The private
container IFilter path still failed (**3**) and neither
`stfixture-proto-test://root/document` nor the canonical
trailing-slash variant was returned by the exact SystemIndex
queries during the bounded test window. All CI jobs passed;
native Windows Start/taskbar/Win+S/Explorer UX is still unproven.

Microsoft describes the Search **gatherer** as the component
that moves content from URL acquisition and filtering into the
actual index. A new, deliberately *read-only* Windows Search
event diagnostic therefore examines only the short **disposable
Win11 ARM64 guest** crawl interval. It reads Application events
from provider `Microsoft-Windows-Search`, selects only events
whose structured EventData `URL` field begins with
`stfixture-proto-test://`, and exports **only event ID,
sanitized hexadecimal HRESULT, and count**. Raw event XML,
message text, user URL, SID, process metadata, filenames and
other Windows Search content sources are not emitted, stored in
Git, or uploaded as CI artifacts. The provider and URL filter
are tested against a deliberately excluded synthetic user-file
example. The maximum bounded event window is 20 minutes;
1,200 events triggers `INCOMPLETE` rather than a false
negative. An absent matching event is reported as
`NO_MATCHING_PRIVATE_EVENTS_OBSERVED`, which is **not**
proof the gatherer successfully indexed the item.

The guest trial invokes the diagnostic after all three temporary
private crawl trials but **before mandatory cleanup**. Its
own trusted GitHub-hosted Windows 11 ARM64 identity gate
rejects use on Naveax/work-pc. This adds no registry changes,
search service controls or broad file/index querying beyond the
existing isolated synthetic trial. CI runs read-only parser
selftests on Windows PowerShell 5.1 and PS7.

Reference:
https://learn.microsoft.com/en-us/windows/win32/search/-search-indexing-process-overview

## Verified filter execution and exact SystemIndex URL variants (2026-10-16)

Windows 11 ARM64 job
[#38962650101](https://github.com/Naveax/Search-Tool/actions/runs/38962650101)
confirmed the fixed trailing-slash direct-root URL alias was actually
encountered **three** times by the real Windows Search protocol host.
ETW recorded 6 accepted private URLs, 6 `GetSecurityDescriptor`
successes, 6 `BindToFilter` calls, **3 successful GetFilter
resolutions**, 6 `BindToStream` calls, 9 filter factory calls,
6 IFilter Init calls and 12 IFilter GetChunk calls. This represents
genuine hosted COM/IFilter progress; it is not an in-process-only
mock. The host also emitted three `0x80004002` E_NOINTERFACE
diagnostic events whose exact origin is not yet isolated.

**Important read-only query gap:** the earlier SystemIndex audit
selected only the exact unsuffixed
`stfixture-proto-test://root/document` URL. Windows may preserve
the trailing slash when indexing a direct nonhierarchical search
root. Therefore absence of the unsuffixed row alone did **not**
exclude a row at the canonical test URL with slash. Additionally,
the prior `Measure-SyntheticRows` implementation incorrectly
compared a successful user-SID-scoped result against the fixed
SID-less URL. Both defects are now addressed:

- `native-search-systemindex-fixture-audit.ps1 -CanonicalDocumentRoot`
  checks **only** `stfixture-proto-test://root/document/` by
  exact read-only OLE DB query.
- The default mode continues to check **only** the unsuffixed URL.
- `-CurrentUserScoped` still derives the own SID internally and
  redacts it in its report, but now validates the returned row
  against the same dynamically constructed candidate.
- The two special modes cannot be combined. All three modes
  have Windows PowerShell 5.1 self-tests in CI.

The disposable Windows 11 ARM64 direct-root trial polls the two
fixed SID-less URL variants independently and reports which
**exact URL**, if either, was indexed. No wildcard enumeration,
real indexed path exposure, indexing-service mutation on physical
PCs, or native Start/Win+S acceptance claim is introduced.
Windows Search registration and WSearch restart continue to be
strictly limited to the throwaway Windows 11 ARM64 guest with
mandatory rollback. Release status remains INVALIDATED.

## Direct document root URL canonicalization, bounded evidence (2026-10-16)

Real Windows 11 ARM64 CI #38961786772 of
`3f0e430cba973518a5966d29ad538aadf2e9effe`
completed the new direct, nonhierarchical SID-less synthetic
document root trial and its strict cleanup, but found **no**
SystemIndex row. Compared with earlier jobs, the protocol logged
**two additional CreateAccessor invocations and two additional
rejected URLs**, while accepted URLs, security descriptors and
BindToFilter counts remained unchanged. Thus the direct-root
experiment has **not yet proved that Windows passed our exact
canonical document URL** to its accessor.

The parser now accepts precisely one additional alias,
`stfixture-proto-test://root/document/`, for the same immutable
synthetic **document**, never any subtree or real file. The
registrationless COM test confirms the trailing-slash alias is
non-directory and closes correctly, retaining the existing
negative descendant tests.

ETW fixed, zero-payload markers 141/142/143/144 classify,
respectively: trailing-slash document alias recognized, rejected
URL under the fixed synthetic root path, other private-scheme
URL shape, and URL outside the test scheme. No actual URL string,
SID, token, file path or arbitrary content is exported or
retained. The next isolated ARM64 job will show whether this
resolves the two new rejections or whether another opaque URL
shape needs investigation, before claiming real indexability.
The private CSM root, service and registry rollback remain
mandatory. Even a positive SystemIndex row cannot establish
Explorer, Start, Win+S or Windows `file:` ranking parity.

## Direct synthetic text-document indexability experiment (2026-10-16)

With Windows Search's real `IProtocolHandlerSite::GetFilter`, the
isolated Windows 11 ARM64 guest demonstrably resolves its built-in
`txt` IFilter three times while **both private custom-filter
selectors remain GTHR_E_FILTER_NOT_FOUND**. This proves that
SearchFilterHost can resolve a normal, already-installed filter.
It does **not** prove our own `.stfixture` association works.

The next experiment deliberately separates the **custom container
enumeration blocker** from **basic indexed plain-text document
feasibility**. For the one immutable synthetic *text document*
only, `BindToFilter` may use the Windows-host-selected built-in
`txt` filter if both private filters were not found. It still
obtains the filter via the isolated host API, loads only the
fixed in-memory UTF-8 fixture through IPersistStream, retains the
restricted source-owner ACL and does not modify Windows' `.txt`
associations. It **never** substitutes a text filter for the
custom container/child-URL filter. The new numeric ETW marker
140 records this exact fallback distinctly, and mock COM tests
verify that text chunks can be read and references released.

In the strictly gated disposable Windows 11 ARM64 guest, an
independent `add-document` / `remove-document` trial temporarily
registers `stfixture-proto-test://root/document` itself as a
**nonhierarchical root with an exact rule**. This avoids having
to enumerate a child from `stfixture-proto-test://root`. A
bounded read-only SystemIndex query checks only this fixed
private URL and reports `DIRECT_TEXT_DOCUMENT_INDEXED` or
`DIRECT_TEXT_DOCUMENT_NOT_PROVEN`. A `finally` rollback must
remove the exact root/rule and return private scope counters to
zero. There is no file: root modification, Windows Shell change,
production service restart or real user file in this experiment.

**Acceptance boundary:** a native row produced via built-in
text IFilter, if observed, demonstrates *basic native indexing*
of a synthetic text document, **not** correct custom IFilter
registration, real filesystem ranking, Explorer display or
Start/taskbar/Win+S parity. The customized container IFilter
and production identity mapping remain explicitly BLOCKED.

## Hosted FilterHost built-in text positive control (2026-10-16)

Exact-head CI `545ebf5117fe3b6184e802167cbf121cb9c003d4`
proved both private extension-based and private document-class
selectors returned `GTHR_E_FILTER_NOT_FOUND` on all three real
Windows 11 ARM64 `GetFilter` calls (six failures total). All
guest-only document-class registrations and cleanup succeeded, but
neither synthetic URL entered SystemIndex.

To distinguish a failure restricted to *new private registry
handlers* from a filter-host-wide inability to resolve *any*
IFilter, the isolated fixture now performs one **read-only built-in
text filter lookup** (`GetFilter(nullptr, nullptr, L"txt", ...)`)
only after both private selectors failed with the exact
`GTHR_E_FILTER_NOT_FOUND` HRESULT. A returned IFilter is
**immediately released** and never passed to the crawler or
used to index the synthetic document. Thus a successful positive
control is **NOT** custom IFilter success or native Windows UI
acceptance, and cannot mask the original private error.

Fixed no-payload ETW markers 137/138/139 distinguish text
control attempted/resolved/not-resolved; any returned HRESULT is
recorded numerically without URL, file contents, user SID, paths
or process arguments. The registrationless fake host asserts that
even if a test text filter can be resolved, the real private
`BindToFilter` returns the original missing-filter error and
leaks no filter object. This does not change built-in `.txt`
registration, any standard file content, service protections
or Shell configuration.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-implementations
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iprotocolhandlersite-getfilter

## Real GTHR_E_FILTER_NOT_FOUND (0x80040D16) and private document-class fallback (2026-10-16)

A genuine GitHub-hosted Windows 11 ARM64 ETW run
[#38958601451](https://github.com/Naveax/Search-Tool/actions/runs/38958601451)
proved that **all three** failing
`IProtocolHandlerSite::GetFilter` calls returned
`0x80040D16` exactly. This is the known gatherer HRESULT
`GTHR_E_FILTER_NOT_FOUND`: Windows Search could not locate or
create the document's filter. The ETW canary, WSearch restoration,
raw-ETL deletion and private-registry rollback markers all
passed. The Windows 11 ARM64 and Ubuntu jobs passed; the Windows
x64 job remained running at the time of this observation.

Microsoft documents two single-selector ways to resolve
`GetFilter`: file extension **without** its dot, or the
**document-type CLSID** (not the CLSID of the filter DLL itself).
After the exact private extension lookup returns only
`GTHR_E_FILTER_NOT_FOUND`, the isolated fixture now makes
one host-managed retry using the synthetic document-type CLSID.
This has no fallback for other errors. In the strictly gated
disposable guest, each of the two extra private document
CLSID registry keys has only a `PersistentHandler` value
pointing to the existing private persistent handler. No built-in
file type, production class, system-wide `file:` index,
Shell extension or Windows Search security boundary is changed.
The two document-class keys are rejected when preexisting,
then deleted and checked absent in the `finally` rollback.

The unregistered in-process COM mock is explicitly configured
to return `GTHR_E_FILTER_NOT_FOUND` on extension lookup and
then resolve the correct private filter by the document class.
Thus native MSVC tests cover the fallback without changing the
developer Windows installation. New numeric-only ETW markers
133/134/135 distinguish fallback attempted/failed/succeeded,
and 136 records the known missing-filter HRESULT. The exact
return code from any failed class-selector retry continues to
be reported via zero-payload ETW header fields.

**Not yet proven:** that this second documented selector works
in real SearchFilterHost, or that the fixture is indexed or
appears in Explorer, Start or Win+S. Do not merge/release from
mock tests alone.

Documentation:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iprotocolhandlersite-getfilter
https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-registering-filters
https://www.hresult.info/FACILITY_ITF/0x80040D16

## Exact private GetFilter HRESULT, without exporting any user data (2026-10-16)

The exact-head Windows 11 ARM64 CI of
`6d0e62ce0d3d1505207666aec71d34e5100f153f`
fully passed Windows 11 ARM64 / Windows x64 / Ubuntu jobs.
Its real, bounded ETW trace confirmed all three
`IProtocolHandlerSite::GetFilter` attempts returned a failing
HRESULT, but none belonged to our coarse buckets
(`REGDB_E_CLASSNOTREG`, `E_ACCESSDENIED`, `E_INVALIDARG`,
`E_NOTIMPL`, not-found, E_FAIL, module-not-found):
`NATIVE_SEARCH_HOSTED_PRIVATE_ETW_FILTER_OTHER_FAILURE_COUNT=3`.
The precise HRESULT is required before changing registry layout,
COM permissions or filter loading behavior.

The private fixture's guest-only trace now encodes that HRESULT
in **ETW standard event-header numeric fields only**: the upper
16 bits in `Task` and lower 16 bits in `Id`, while the provider
GUID and Keyword remain fixed. This has **zero event payload**;
no URL, file path, SID, token, process identifier or filter contents
are emitted by the DLL. The existing disposable-VM ETW reader
reconstructs the exact `0xXXXXXXXX` code and emits only its
occurrence count (never the raw trace or other events). A
PowerShell 5.1/7 self-test checks the decoder with synthetic
`0x8004170C` input. Event Task-as-error-half is an explicitly
non-production diagnostic convention, not general ETW modeling.

A successful Windows Search GetFilter resolution, an IFilter
instance and an indexed fixture row remain distinct acceptance
gates. This targeted numeric evidence is preferable to speculative
registry changes or disabling the SearchFilterHost isolation
boundary.

## Filter host selection aligned to Microsoft GetFilter contract (2026-10-16)

After the ephemeral guest owner's SID supplied an actual restricted
source ACL, the real Windows 11 ARM64 job #117154222084 achieved
**3 `IUrlAccessor::BindToFilter` calls**, each following a valid
restricted ACL returned by GetSecurityDescriptor. The first actual
filter call had previously been blocked by the service-account
ACL policy. No `IFilter` factory/Init/GetChunk events or exact
SystemIndex item appeared yet; the isolated Windows 11 ARM job
itself completed SUCCESS and cleanup restored WSearch.

Review of Microsoft's documented
`IProtocolHandlerSite::GetFilter` signature uncovered an additional
mismatch in the synthetic implementation: it previously passed
MIME, CLSID and file extension *simultaneously*, and supplied the
extension with a leading dot. Microsoft specifies that the
extension parameter must **omit** its leading dot, and the host
tries the MIME choice first when multiple selectors are provided.
For the two explicitly registered, private
`.stfixture`/`.stfixture-root` persistent handler chains, the
protocol now requests **only** the matching extension,
`stfixture` or `stfixture-root`, leaving CLSID and MIME null.
The registrationless mock host was updated to assert exactly this
contract. The real Windows Search host still owns filter selection
and isolated activation. A fixed ETW ID 120 means GetFilter
returned a failure and 121 means it succeeded; no HRESULT,
MIME string or user data is logged.

Source:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iprotocolhandlersite-getfilter

## Source-owned private test document ACL, never the service principal (2026-10-16)

Real Windows 11 ARM64 trace #38953621517 supplied the missing
security-boundary evidence. Its ETW canary was captured and actual
host callbacks included four protocol factories, four Init calls,
five CreateAccessor calls, three GetSecurityDescriptor calls,
**three failed restricted ACL builds**, two private URL rejections
and three accepted fixture URLs. **No** `BindToFilter` or IFilter
callback occurred and both synthetic URLs remained absent from
SystemIndex. The host is genuinely invoking the protocol but the
SID-less synthetic item's ACL cannot be derived from its
service-account token. The CI status was FAIL because a redundant
second `logman stop` left a nonzero native exit code, despite
successful fixture registry, CSM, ETL and WSearch cleanup. The trace
controller now stops its session exactly once.

The guest-only experiment can now define the owner of this one
**synthetic fixture** in the private test CLSID's
`HKLM\\SOFTWARE\\Classes\\CLSID\\{987AD765-D904-4F10-A650-915D8F4C0287}\\FixtureOwnerSid`
REG_SZ value. The strictly gated CI runner obtains its own test
account's SID and writes that value without ever printing the
SID. If the Windows Search process has a system/service identity,
`build_private_test_descriptor` continues to **refuse** that
principal as the item's ACL owner, but can use this separately
configured, valid, non-service **synthetic source owner** to build
its self-relative single-user read DACL. When the trusted temporary
owner is absent, malformed or not a real user, the service-account
accessor still fails closed. Existing local COM tests continue to
use the actual non-service test process owner when no CI registry
value is present. The ephemeral CLSID and its owner marker are
removed in the existing mandatory `finally` rollback.

**Critical production boundary:** A real user's file ACL must come
from authenticated/authoritative source metadata. This temporary
CI-only owner mapping is **not** a production identity authority,
not an invitation to treat system/LocalService as the user, and
not general permission to make indexed items public. Real
SearchProtocolHost filtering and native Explorer/Start/Win+S proof
remain independent, unproven acceptance requirements.

### First live invocation-level ETW result and refinement (2026-10-16)

The first ETW-enabled Windows 11 Enterprise ARM64 job
[#38952930648](https://github.com/Naveax/Search-Tool/actions/runs/38952930648)
produced **real Windows-owned protocol method evidence**. The
private GUID ETW canary was successfully captured; pre-canary host
events counted four protocol factories, four `ISearchProtocol::Init`
calls, four `CreateAccessor` calls, and three
`GetSecurityDescriptor` calls. No `BindToFilter`,
filter-factory, IFilter Init or GetChunk events were observed
during the bounded guest experiment; both synthetic SystemIndex
lookups were still NOT_FOUND. CSM rollback, raw ETL deletion,
private-registry cleanup and WSearch restoration markers were all
present. The overall ARM64 CI job **failed** at its post-trial
wrapper, so it must not be described as a passing exact-head CI.

This narrows the failure boundary to **URL acceptance / private
security descriptor / filter dispatch**, rather than a completely
uninvoked ISearchProtocol. The ETW continuation adds fixed IDs
107/108/109/110 for failed ACL build, successfully returned ACL,
the expected insufficient-buffer sizing probe, and the SID-scoped
E_NOTIMPL path. IDs 111/112 indicate rejected/accepted private
fixture URLs. These remain zero-payload numeric ETW events.

The canary now uses the presence of the deliberately generated
**factory ETW event** as its validity test, not a second requirement
that every assertion in the full in-process COM smoke executable
passes after WSearch crawls. A nonzero canary executable exit is
explicitly reported separately; it is still never counted as a
host callback. Missing canary ETW capture remains a fail-closed
trace error. The next CI run must verify these conditions and,
most importantly, reveal the actual ACL/URL return branch.
Do not weaken the fail-closed policy on service identities.

## Private invocation-level ETW tracing inside disposable Windows Search VM (2026-10-16)

The prior guest-only WSearch reload confirmed the real Windows-owned
`SearchProtocolHost` dynamically loaded `protocol_fixture.dll`,
but neither SID-less nor own-SID exact synthetic document appeared in
`SystemIndex`. A loaded DLL is not proof that Windows invokes its
COM factory/accessor/filter methods.

The **test-only** protocol and IFilter DLLs now emit *manifest-free
Event Tracing for Windows (ETW)* events with provider
`CBBAE291-F9D2-4E9D-A779-9DECE2129912`. Event descriptors
have a fixed ID, information level and keyword, **zero event data**.
Numbers 100/101/102/103/104/105 correspond to protocol
`DllGetClassObject`, `ISearchProtocol::Init`,
`CreateAccessor`, `IUrlAccessor::GetSecurityDescriptor`,
`BindToFilter` and `BindToStream`. IDs 200/201/202 are
private filter factory, IFilter Init and GetChunk.

Only the strictly gated disposable `windows-11-arm` VM runs
`-RunHostedGuest -ReloadGuestWSearch -TraceGuestCallbacks`.
It starts a GUID-restricted, named ETW session **after** the
ordinary in-process COM test and before the WSearch reload. The
session stays enabled across both private CSM crawl attempts. At
cleanup, a deliberately timed, in-process COM test emits a *canary*
after the actual crawl window, the ETW session stops and only
allowlisted **event-ID counts** from before the canary are logged.
The canary must contain the protocol-factory event; otherwise the
trial fails closed rather than claiming no Windows host callback.
The raw `.etl` is deleted and **never uploaded as a workflow
artifact**. ETW metadata may contain PIDs/timestamps locally within
the isolated runner, but none is printed or persisted in the repo.

No user SID, URL, file path, contents, token, Windows Event Log data,
shell action or third-party provider is recorded as a trace payload.
The provider's EventRegister/EventWrite calls are process-local
and do not require a provider manifest installed in the registry.
If the trace reports no host calls, that observation applies only to
the bounded test window; it does not prove that the host can never
call the protocol. In all cases, SystemIndex and native
Explorer/Start/Win+S acceptance are separate, unproven checks.

Microsoft documentation:
https://learn.microsoft.com/en-us/windows/win32/api/evntprov/nf-evntprov-eventregister
https://learn.microsoft.com/en-us/windows/win32/etw/writing-manifest-based-events
https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/logman

## Ephemeral Windows Search reload after private registration (2026-10-16)

CI #38948407084 confirmed the complete **private-only** dependent/
independent ProgID registration paths alone did **not** cause
`protocol_fixture.dll` to be loaded in a Windows Search protocol
host. The Windows 11 ARM64 guest was FULL_CRAWL or IDLE without a
pause reason, while the private module remained unobserved and
both SystemIndex fixture URLs remained NOT_FOUND. Cleanup PASS.

Microsoft states that registering a new filter handler without a
property handler can require **restarting Windows Search** to begin
automatic reindexing. The current CI experiment therefore adds an
**explicit** `-ReloadGuestWSearch` flag, executable only after
the existing `-RunHostedGuest` strict GitHub-hosted Windows 11
ARM64/repository/job/admin/clean-baseline gates pass. After the
private COM and filter registrations are installed, this ONE
throwaway guest restarts WSearch and verifies it returns to Running,
before invoking the existing private root/rule crawl trial. At
cleanup the guest stops WSearch to release its loaded DLLs, removes
only the private fixture registration/root/rule/files, and
restores the original Running service state in a finally block.
No physical host, ordinary `file:` root, Start, Explorer or Win+S
settings are touched.

This operation is intentionally **not** run as a CI step on
`windows-latest` Windows Server or on either connected development
PC. An accepted WSearch restart is not indexing acceptance, and a
successful CI job is not a replacement for independent snapshot
reversion evidence. The synthetic SystemIndex and private-module
results remain separate gates.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-implementations

## Isolated protocol class registration completeness test (2026-10-16)

The real Windows 11 ARM64 catalog-status probes observed
`CATALOG_STATUS=3` (`FULL_CRAWL`), paused reason `0`, and
aggregate incremental/notification/high-priority queues all at
zero at all four guest snapshots. The SearchProtocolHost process
was available for module enumeration, but the private protocol
DLL was **NOT_OBSERVED** (not an access-denied result) and both
exact synthetic SystemIndex lookups remained `NOT_FOUND`.
This makes a globally paused catalog an inadequate explanation,
and points attention at the handoff from Crawl Scope Manager to
registered protocol COM activation.

Microsoft's protocol-handler registration documentation includes
both **version-dependent** and **version-independent** ProgID
mappings and the corresponding CLSID reverse links. The earlier
minimal guest-only COM experiment provided only the version-dependent
`SearchTool.PrivateFixtureProtocol.1` ProgID. The next disposable
Windows 11 ARM64 run adds **only** private reverse
`CLSID\\ProgID` and `CLSID\\VersionIndependentProgID` links,
plus the private independent ProgID `SearchTool.PrivateFixtureProtocol`
with `CLSID` and `CurVer` entries. These additions cannot modify
existing Windows file types, third-party handlers, Start/Explorer,
`file:` roots or real user files; no ShellFolder or invalid TypeLib
registration is fabricated.

The private independent ProgID is checked absent before creation,
deleted in the same `finally` cleanup, and checked absent again
after cleanup. This is a *registration-resolution experiment*,
not Windows Search or native UX acceptance. The SystemIndex
and host-module observations remain separate.

Source:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration

## Real catalog status and aggregate queue counters (2026-10-16)

CI #38946953219 completed its Windows 11 ARM64 native guest job
successfully, but the read-only observer sampled
`NO_SEARCH_HOST_PROCESS` for both SearchProtocolHost and
SearchFilterHost immediately after private-root registration/reindex
and again after the bounded SystemIndex lookup, for **both** own-SID
and no-SID variants. The synthetic indexed item still was not found.
These are **four point-in-time observations**, not continuous proof
that SearchProtocolHost never executed.

The existing read-only
`experiments/native-ifilter-fixture/crawl_scope_readonly.cpp`
now uses the documented `ISearchCatalogManager::GetCatalogStatus`
and `NumberOfItemsToIndex` calls. It exports only enum numbers
and aggregate counts for pending incremental, notification and
high-priority items. It deliberately never calls
`URLBeingIndexed` or exposes other indexed paths.

The guarded, disposable Windows 11 ARM64 CI records these numbers at
both points when it also samples SearchProtocolHost module presence.
This distinguishes an actually paused or busy catalog with pending
queues from an active crawler that cannot load a handler. A passing
CI job means the read-only diagnostic executed safely, **not** that
a synthetic document was indexed.

The physical Win11 developer PC's read-only baseline observed
`CATALOG_STATUS=1` (paused),
`CATALOG_PAUSED_REASON=8` (user active), and aggregate pending
queues 0 incremental / 66 notifications / 3 high-priority at the
sampling instant. This is **not the hosted VM state**, and nothing
was changed on that physical machine.

Microsoft APIs:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-isearchcatalogmanager-getcatalogstatus
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-mngidx-catalog-manager

## Actual SearchProtocolHost private-DLL module evidence (2026-10-16)

After the Win11 ARM64 guest verified both own-SID and no-SID root
inclusion and `ReindexSearchRoot` request acceptance, both exact
SystemIndex fixture URL queries still returned **NOT_FOUND**. The
next diagnostic needs to distinguish *SearchProtocolHost actually
loading our test DLL* from merely accepting a CSM registration.

`scripts/native-search-host-module-probe.ps1` is strictly read-only.
It enumerates the Windows-owned `SearchProtocolHost` and
`SearchFilterHost` process module **basenames** and compares only
`protocol_fixture.dll` and `fixture_filter.dll`, respectively.
It exports no PIDs, owners, paths, unrelated DLLs, index rows,
event-message contents, file paths, or SIDs. It never attaches a
debugger, stops a service or changes a registry key.

Possible per-host states are `OBSERVED`, `NOT_OBSERVED`,
`NO_SEARCH_HOST_PROCESS`, and
`INCONCLUSIVE_ACCESS_DENIED`. A denied module enumeration **must
not** be classified as proof of DLL absence. PowerShell 5.1 and
PowerShell 7 fail-closed classifier self-tests are required.
The isolated ARM64 VM samples this read-only state immediately
after each private CSM root becomes active and again after its
bounded exact SystemIndex observation, before private registration
is rolled back. This test supplements, but does not itself prove,
indexing or Start/Explorer/Win+S native UI acceptance.

On the normal physical Win11 developer PC, the *read-only* module
observer saw one SearchProtocolHost process but module enumeration
was **INCONCLUSIVE_ACCESS_DENIED**. There was no SearchFilterHost
process at the moment of the sample. No installation or privileged
action was made to change that result.

Microsoft documents that protocol handler DLLs are dynamically
loaded by SearchProtocolHost on demand, and recommends specialized
debugging rather than directly attaching a debugger to active
protocol hosts:
https://learn.microsoft.com/en-us/windows/win32/search/-search-ws-protocolhandlertesting

**Architectural limitation:** Microsoft states that Windows Explorer
results for a *new data store* require a Shell data source/namespace
extension in addition to a protocol handler. A successful synthetic
SystemIndex lookup alone will not establish Explorer UX parity, let
alone replacement of original Start/Win+S results or file: ranking.
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr

## Targeted private-root indexing trigger (2026-10-15)

In real GitHub-hosted Windows 11 ARM64 tests, both the no-SID
container root and a per-user SID document root were accepted by
Windows Search's Crawl Scope Manager, but **neither resulted in an
observable SystemIndex row** during the bounded test interval.
The per-user route shows the missing SID was not the only blocker.
The root/rule registrations and private COM/IFilter markers were
successfully cleaned up.

The next narrowly scoped diagnosis calls
`ISearchCrawlScopeManager::IncludedInCrawlScope` for **only the
currently registered synthetic root**, then invokes
`ISearchCatalogManager::ReindexSearchRoot` for **that exact same
private root** in the disposable Windows 11 VM. Both results are
reported independently. The return from ReindexSearchRoot means only
that Windows accepted the incremental crawl *request*: it does not
establish that a protocol handler was loaded, a document was indexed
or Windows' normal Start/Win+S saw the result.

**Never** replace this call with catalog-wide `Reindex()` or
`Reset()`. Those methods affect unrelated user content. No
per-user SID is included in logs, no other crawl roots are changed,
and the existing temporary-root cleanup remains mandatory.

Reference:
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-isearchcatalogmanager-reindexsearchroot

## Disposable VM per-user SID document crawl experiment (2026-10-15)

Following the separate verified SID-matching COM implementation, the
ephemeral Windows 11 ARM64 CI job also exercises a **direct document
root** of the form
`stfixture-proto-test://{CURRENT-USER-SID}/root/document`.
The user SID is obtained from the CI runner's actual Windows token,
never supplied externally or printed. It adds a nonhierarchical
single-document CSM root and exact inclusion rule; polls Windows
SystemIndex only for that same current-user URL; then removes both
in `finally` and confirms the private root/rule counts returned to
zero. The existing no-SID directory enumeration experiment is still
run and rolled back first.

The new read-only SystemIndex audit option `-CurrentUserScoped`
constructs SQL only with that validated own SID and the one fixed
synthetic URL. JSON reports redact the SID even if the row exists.
This tests whether the documented per-user SearchProtocolHost identity
path can index a synthetic document without broadening ACLs or
modifying Windows' normal `file:` index rules. Actual service
invocation, indexing, Explorer, Start and Win+S visibility are still
separate evidence and cannot be inferred from a successful CSM
registration or a passing CI job.

## Per-user SID URL contract in synthetic protocol (2026-10-15)

Microsoft documents a critical distinction: a Search protocol URL with
`{S-1-...}` is dispatched in that user's security context, and
`IUrlAccessor::GetSecurityDescriptor` **must** return `E_NOTIMPL`.
For a URL without a SID, returning `E_NOTIMPL` would make the indexed
item readable by **all queries**, so that behavior must never be
silently applied to the system-scoped fixture.

The test-only `ISearchProtocol` implementation now additionally accepts
`stfixture-proto-test://{CURRENT-USER-SID}/root/document` and its
synthetic container only if the SID is both a valid non-service SID
and exactly the user of the current effective thread token (when
impersonating) or process token (when not impersonating). An unreadable
impersonation token fails closed. The SID is neither printed nor
persisted. Only this validated user-scoped accessor returns
`E_NOTIMPL`; the original SID-less fixture still constructs a
restricted self-relative ACL and rejects system/service identities.
The OS COM smoke test checks the user's SID-positive path, SYSTEM SID
negative path and security-descriptor return contract. This is still
not actual `SearchProtocolHost` user impersonation or SystemIndex
acceptance. No CSM roots, shell behavior, Windows service or registry
are changed by the protocol parser itself.

Sources:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration
and
https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iurlaccessor-getsecuritydescriptor.

## Temporary private IFilter registration diagnosis (2026-10-15)

**Post-CSM first result:** exact-head CI run
[#38747089177](https://github.com/Naveax/Search-Tool/actions/runs/38747089177),
the Windows 11 Enterprise ARM64 guest job passed with actual
`NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ADD_PASS`,
`NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ROOT_AND_RULE_OBSERVED_PASS`,
`NATIVE_SEARCH_HOSTED_PRIVATE_CSM_REMOVE_PASS`,
`NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ROLLBACK_PASS`, and
`NATIVE_SEARCH_EPHEMERAL_GUEST_PRIVATE_REGISTRY_CLEANUP_PASS`.
However the fixed synthetic URL remained
`NATIVE_SEARCH_HOSTED_PRIVATE_SYSTEMINDEX_ROW_NOT_PROVEN`
during the bounded observation. **No actual crawler/index/Start UX
acceptance can be claimed.**

Next supported dependency, per Microsoft's `Registering filter
handlers` guidance: the filter DLL also needs its **own**
class-registration / `PersistentHandler` mapping for the **private**
extension and a machine-readable installation path accessible to
`SearchFilterHost`. The isolated guest test now creates the
document and container `IFilter` COM CLSIDs,
`.stfixture`/`.stfixture-root` private extension association,
private per-extension persistent handlers, and copies only the
synthetic DLLs into a newly created dedicated folder under
`Program Files` in **that one ephemeral VM**. Its OS COM smoke
asserts `CoCreateInstance` for both private filter CLSIDs. All
temporary entries and test folder are removed in `finally`; the
physical hosts are untouched. This does not relax the strict
service-token SID guard: the private test document may still
be refused under a SearchProtocolHost system principal until
correct user impersonation is proven.

Microsoft:
https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-registering-filters

## Genuine Crawl Scope Manager experiment (2026-10-15)

After actual OS COM activation, the next *scoped* test queries the
real `ISearchManager → ISearchCatalogManager(SystemIndex) →
ISearchCrawlScopeManager` COM API. The separate
`crawl_scope_readonly.cpp` executable enumerates roots/rules but
**prints neither existing root paths nor other users' scope rules**;
only total and `stfixture-proto-test:` counts and exact root/item
inclusion state. On the untouched local Win11 build 22631 host, the
read-only API returned 7 roots and 182 rules, with **0 private roots,
0 private rules, root/item not included**.

In the existing strict `windows-11-arm` *disposable VM-only* COM
registration test, the new `crawl_scope_guest_trial.cpp` can add
only a private `stfixture-proto-test://root` root and a
`stfixture-proto-test://root/*` rule. The PowerShell orchestration
refuses any preexisting private root/rule, confirms the addition by
re-querying the Windows Search catalog, then makes a bounded exact
synthetic SystemIndex URL check. The registration, rule and root are
then removed, and both the 36-marker COM registry inventory and
the actual CSM inventory must be clean. A write invocation on the
physical Windows 11 host was tested and **rejected (E_ACCESSDENIED)**
before any CSM call. The guest run is for determining whether a real
Windows Search *crawler* can see the private COM protocol; it does
not prove it will. If no item appears, report `NOT_PROVEN` and
retain native acceptance BLOCKED.

References: Microsoft
`ISearchCrawlScopeManager::AddRoot`,
`AddUserScopeRule`, `RemoveRoot`, `RemoveScopeRule`, `SaveAll`.
This experiment does **not** register general `file:` handlers,
alter preexisting roots, modify the Windows shell or create a
companion application. The temporary GitHub CI VM must not be
confused with an independently restored Windows snapshot or the
real customer's Windows 10+11 environment.

## Native integration alternative: real GitHub-hosted Windows 11 desktop guest (2026-10-15)

**Live feasibility research changed the test plan.** The existing
`windows-latest` CI uses **Windows Server 2025**, not desktop Windows
11. GitHub publicly offers the separate `windows-11-arm` runner,
which is a **fresh Windows 11 Enterprise ARM64 virtual machine**.
The repository is public, and GitHub documents standard hosted
runners for public repositories as free; its Windows guest is
administrative and ephemeral. This avoids requesting elevated
Hyper-V rights on either user's physical Windows PC.

A new **independent CI job**, `windows11-desktop-native-feasibility`,
uses `runs-on: windows-11-arm`, and ONLY performs existing read-only
preflight, SystemIndex fixed synthetic URL query and private registry
marker checks. It records OS architecture/build, WSearch state, and
boolean presence of Explorer/StartMenuExperienceHost/SearchHost
processes; it never sends keyboard events, installs COM, changes
crawl scopes or makes a Start/Win+S acceptance claim. Its artifact is
`native-search-windows11-arm-readonly`. CI PASS for this job
means the *diagnostic ran safely*, not that Windows Search
integration worked; inspect its JSON for eligibility.

If the guest has a functional WSearch/SystemIndex, the next
engineering action is to compile the in-memory C++ COM fixture
natively for ARM64 in that disposable test OS; any subsequent *OS
registration* requires a separately reviewed, strict-root, reversible
run and host integrity evidence. The GitHub Actions VM is newly
provisioned but is not a documented independently restored snapshot.
It cannot by itself complete original Win10 x64 coverage, a full
interactive taskbar/Start/Win+S proof, or a production release.

**First actual GitHub Windows 11 guest result**: run
[#38738354020](https://github.com/Naveax/Search-Tool/actions/runs/38738354020),
job `windows11-desktop-native-feasibility` **PASS**.
Windows 11 Enterprise build **26200**, native **ARM64**,
`WSearch=Running`, `SystemIndex=NOT_FOUND` for the private
unregistered URL, private registry markers **CLEAN_BASELINE (36/36)**,
and Explorer, StartMenuExperienceHost, SearchHost processes **present**.
This proves a genuine supported Windows 11 desktop CI environment
with operational Windows Search and shell processes. No Search UI
content was queried or displayed. Receipt:
`docs/evidence/native-search-hosted-win11-arm-baseline-20261015.json`.

The fixture build harness now accepts `-Target arm64` in addition
to the existing validated x64 path, selecting the native VC ARM64
environment and rejecting an architecture mismatch. Windows 11 ARM
CI compiles/runs both **unregistered, in-process** COM DLL pipelines;
the test reports their real success/failure separately from the OS
integration. This makes productive use of the new guest while
continuing to prohibit live Windows Search registration on physical
PCs. An actual native Windows Search search-result visibility proof
is still needed before release.

**Next guest-only COM activation experiment:** A separate disposable
runner action now temporarily creates the private
`stfixture-proto-test` protocol registration, a single
`{987AD765-D904-4F10-A650-915D8F4C0287}` COM CLSID and private
ProgID **only on the GitHub-hosted Windows 11 ARM64 VM**. A native
`CoCreateInstance` C++ smoke binary checks that the OS COM runtime
really activates `ISearchProtocol` and creates the synthetic
root/document accessors. It does **not** call `DllRegisterServer`
(which continues to return E_ACCESSDENIED), does not register
`file:` or any common extension, does not add crawl scope and does
not touch the native shell. The unique registry entries are removed
in a `finally` block and the 36-marker read-only audit must show
`CLEAN_BASELINE` again. It is strictly guarded by GitHub-hosted
Windows 11 ARM64, administrative test identity, Windows Search
running, exact repository/job and a private fixture pre-baseline.
It cannot be invoked on either personal/work PC.

This tests OS COM activation/rollback, **not yet live
SearchProtocolHost crawling**, native Start UI, Win+S or Explorer.
A successful hosted test is not an independently restored snapshot
and does not change the remaining release BLOCKED status.

**Native API selection research:** Microsoft's Windows Search
`ISearchProtocol` + `IFilter` route is documented for indexing
*new private data stores*, not for replacing the general existing
`file:` ranking or Start/Win+S result engine. New Windows Search
web provider extensions are restricted to the **EEA**, use
**public HTTPS query and preview endpoints**, and are not a
private universal local-file scoring hook for non-EEA Windows 10/11.
Windows App SDK `AppContentIndexer` exposes app-managed content
identifiers queried by apps; its documentation does not establish
native Start re-ranking of existing files. These are **not accepted
substitutes** for the no-companion, no-user-behavior-change goal.

- GitHub desktop ARM runner and public repository costs:
  https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- Current runner images:
  https://github.com/actions/runner-images
- GitHub Windows Server hosted WSearch startup limitations:
  https://github.com/actions/runner-images/issues/10419
- Microsoft protocol handler:
  https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration
- EEA-only web search providers:
  https://learn.microsoft.com/en-us/windows/apps/develop/search/search-providers
- Windows App Content Search:
  https://learn.microsoft.com/en-us/windows/ai/apis/app-content-search

## Private registry-footprint baseline (2026-10-15)

The new `scripts/native-search-fixture-registry-audit.ps1` is
**read-only** and looks for exactly 36 fixture-specific registry key
or value-name markers per device. It inspects `HKLM` and the *current
account's* `HKCU`, each in 32-bit and 64-bit registry views. Targets
include private `stfixture-proto-test` Windows Search
`ProtocolHandlers` values/keys, protocol+filter COM CLSIDs,
`.stfixture` and `.stfixture-root` file-extension keys, and the
private Shell namespace/approval identifiers. It never retrieves
registry value data or probes arbitrary protocol names.

Both connected Windows 10 Pro build 19045 and Windows 11 Pro build
22631 physical computers returned **`CLEAN_BASELINE`**, 36
markers checked, zero present, zero read errors. PowerShell 5.1 and
PowerShell 7 self-tests cover checked targets and fail-closed
classification for present, unknown and inaccessible markers.
Evidence:
`docs/evidence/native-search-fixture-registry-dual-os-20261015.json`.
Windows CI now refuses a private fixture registration on the runner.

**Critical limit:** `CLEAN_BASELINE` does **not** inspect other user
hives, Windows Crawl Scope Manager roots/rules, Shell result actions
or the native Start UI. It is only a repeatable before/after *partial
rollback marker*, not evidence of complete rollback or a working
provider. A VM uninstall must also compare Crawl Scope Manager state,
SystemIndex rows, registered COM DLLs and native Windows entrypoints,
and an external verified snapshot restoration. Never install the DLLs
on either physical PC.

Microsoft documents protocol registration under Windows Search
`ProtocolHandlers`, COM CLSID/ProgID and separate Shell association
keys:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration

## Dual-machine Windows 10/11 read-only baseline and COM SID guard (2026-10-15)

Two **distinct** connected physical Windows PCs were probed without
registration, service control, shell input or user-content export:

| OS | Windows build | WSearch | Exact synthetic SystemIndex URL | VM access |
| --- | --- | --- | --- | --- |
| Windows 10 Pro | 19045 | Running | 0 rows (expected NOT_FOUND) | Hyper-V Get-VM denied |
| Windows 11 Pro | 22631 | Running | 0 rows (expected NOT_FOUND) | Hyper-V Get-VM denied |

The query is restricted to the exact fixed
`stfixture-proto-test://root/document` URL. This is useful as a
**before-install baseline** only; it does not validate a protocol-host
load or native Explorer/Start/Win+S results. Supporting anonymized
observations are in
`docs/evidence/native-search-readonly-dual-os-baseline-20261015.json`.

Microsoft's protocol handler documentation identifies no-SID custom
URLs as being invoked under the **system service context**; the
SearchProtocolHost may run in SYSTEM or an end-user context. The earlier
test-only DACL used `OpenProcessToken`, which is insufficient to
identify an actual end-user when hosted by a system service. The shared
`experiments/native-ifilter-fixture/fixture_sid_policy.h` now rejects
well-known SYSTEM, LocalService, NetworkService, service-specific
`S-1-5-80-...` identities, service/application-pool principals and
non-user group/public SIDs, failing with `E_ACCESSDENIED` rather than
emitting a service-account-owned searchable descriptor. C++ direct-COM
tests cover synthetic forbidden SIDs, malformed input and the normal
process user's positive control. Registration remains explicitly
disabled.

**Still not fixed:** multi-user ownership, service-host impersonation,
`IUrlAccessor2/3` user mapping, real Windows Search ACL filtering,
and authorized guest testing. This guard deliberately prevents
mistaking service identity for the user; it is not a production ACL
provider. Do not register the DLLs on either physical machine.

References:
https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-ph-install-registration
and
https://learn.microsoft.com/en-us/windows/win32/search/-search-indexing-process-overview.

## Exact synthetic SystemIndex audit (2026-10-15)

`scripts/native-search-systemindex-fixture-audit.ps1` performs a
**read-only, exact-URL** OLE DB query using the documented SystemIndex
SQL surface. It selects at most two `System.ItemUrl` values for the
**single fixed** `stfixture-proto-test://root/document` address,
never scans all users' results, exports names or file contents, writes
crawl scopes, loads a handler or opens the Search UI. Its query cannot
be supplied or modified by a CLI argument. A duplicate/foreign row,
query exception or missing URL produces an explicitly non-accepted
status. Windows PowerShell 5.1 runs the actual OLE DB probe; the
separate PowerShell 7 execution supports only mocked self-tests, and
live runs under PowerShell 7 fail closed rather than falling back to
shell commands.

**Observed on the untouched Windows 11 build 22631 developer PC:**
`WSearch=Running`, `query_rows=0`,
`systemindex_observation=NOT_FOUND`,
`reason=SYNTHETIC_FIXTURE_NOT_INDEXED`.
This is the **expected and safe** baseline because the private handler
was never registered. Both PowerShell 5.1 and 7 self-tests pass. Even
if a future authorized VM returns
`FOUND_SINGLE_FIXTURE_ROW`, that is proof of an index query only,
**not** Explorer, Start, taskbar Search, Win+S, user ACL mapping, result
open action, or rollback acceptance. The result always leaves
`native_start_accepted=false` and `native_win_s_accepted=false`.

Manual isolated-VM sequence once proper authorization exists:
preflight and externally verify disposable rollback snapshot; record
the exact URL baseline (normally NOT_FOUND); separately perform the
approved private provider registration/crawl within the VM; rerun the
exact URL query to prove SystemIndex observation; separately collect
real Explorer and native Start/taskbar/Win+S visibility evidence;
unregister and rollback VM; query again for absence, checking that
preexisting app/file/settings results and OS configuration are
unchanged. This script never performs these other actions.

Microsoft Search SQL references:
https://learn.microsoft.com/en-us/windows/win32/search/-search-sql-from
and
https://learn.microsoft.com/en-us/windows/win32/search/-search-sql-literalvaluecomparison.

## Disposable VM admission and evidence separation (2026-10-15)

The new `scripts/native-search-vm-preflight.ps1` is a **read-only**
admission probe with PowerShell 5.1/7 self-tests. It checks actual guest
manufacturer/model, Windows 10/11 build, WSearch service state and an
optional operator-provided evidence manifest for the private
`stfixture-proto-test` scheme. The manifest must declare an explicit
isolated-test approval, a snapshot receipt and a **distinct**
rollback-rehearsal receipt; missing, malformed or mismatched facts fail
closed. This is **not a permission grant**: guest detection and user
supplied receipts are advisory, cannot independently prove that a VM is
disposable, and must be verified externally by its administrator. The
script can emit at most `PREFLIGHT_ONLY`, never native acceptance,
registration permission or release eligibility.

On the current Windows 11 build 22631 developer PC the live result is
`BLOCKED`: `DISPOSABLE_VM_NOT_PROVEN` and
`VM_EVIDENCE_MANIFEST_MISSING`. `WSearch=Running` is only a baseline
observation. Existing Hyper-V `Get-VM` authorization was denied and no
other disposable test VM has been proven available. The read-only
preflight neither calls the actual indexer crawl nor changes system
services, registry, handlers, Start/Search/Explorer, shortcuts or user
files. Real native acceptance remains **BLOCKED**.

Required separate evidence on each *externally authorized* disposable
Windows 10 and 11 guest: snapshot/revert proof; baseline Windows
Search index and first-party app/settings/file results; private scheme
registration and actual SearchProtocolHost load; SystemIndex row;
Explorer native result; genuine Windows-key/Start, Win+S and taskbar
Search results; Shell open-result; user/SYSTEM token ACL and
multiuser access; IME/Narrator/focus/DPI; exact uninstall/revert.
No single SystemIndex success may be promoted to a Start/Win+S pass.

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
the full expected Turkish/German/Greek UTF-16 content. It initially emitted an unregistered stream/IFilter test result;
the hardened host-site + ACL test now emits
`NATIVE_PROTOCOL_ROOT_CHILD_IFILTER_ACL_NO_REGISTRATION_PASS`.
MSVC `/W4 /WX` compile and local direct-COM test PASS on Windows.
Both DLLs refuse `DllRegisterServer` and `DllUnregisterServer`
with `E_ACCESSDENIED`; no OS integration is installed.

This advances the **test-only** protocol-to-filter plumbing but is not
a fully indexable or secure Windows Search extension:
real user impersonation and full ACL ownership mapping are not
implemented; there is no Shell namespace registration, container
child enumeration, crawl scope, real SearchProtocolHost invocation, or
visible Start/Win+S result. The handler cannot be installed on the
interactive user PC and must not be released. Full native Windows 10/11
acceptance remains **BLOCKED** pending authorized disposable VM proof.

## Test-only host filter binding and restricted ACL hardening (2026-10-15)

Windows Search's
[IUrlAccessor::GetSecurityDescriptor contract](https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iurlaccessor-getsecuritydescriptor)
says a protocol item URL with **no embedded user SID** that returns
`E_NOTIMPL` can be queried by all users. Our fixed prototype URL has
no user SID. Its previous `E_NOTIMPL` behavior has been replaced **before
any OS integration** by a Windows self-relative descriptor with one
`FILE_GENERIC_READ` ACE for the test process token's user SID.
The implementation queries the current process token without exporting
identity data, emits an explicit restricted DACL (no Everyone/NULL DACL),
and refuses absent or undersized output buffers. The dedicated COM test
verifies owner SID, allowed ACE SID and actual process token SID match,
that `SE_SELF_RELATIVE` is set and the descriptor is valid, and that
short buffers remain unmodified.

Separately, `IUrlAccessor::BindToFilter` now uses the Windows SDK's
`IProtocolHandlerSite::GetFilter` interface rather than claiming a
global COM registration. Only an explicitly supplied site can resolve the
private synthetic `.stfixture` filter. The test provides a private
**in-process site double**, obtains an IFilter from the second DLL's
direct private class factory, and verifies the accessor passes a fresh
IStream into `IPersistStream::Load` so `GetChunk/GetText` returns
identical Turkish/German/Greek UTF-16 content. Without a site,
`BindToFilter` stays `E_NOTIMPL`; the test also checks it.
Both libraries still actively deny `DllRegisterServer` and
`DllUnregisterServer` and no Windows host was altered.

**NOT a production-ready security model:** in a real SearchProtocolHost
context the process token may be SYSTEM or a service account, not the
end-user who should see the item. Correct impersonation and
user-specific ACL mapping, real crawl-scope registration and
SearchProtocolHost traversal (the local `PKEY_Search_UrlToIndex`
container test does not prove those), real shell open behavior, registration/uninstallation, Microsoft Search host load,
and physical Win10/Win11 native Start/Win+S result visibility are
unproven. Keep this code unregistered, PR #81 DRAFT and release
INVALIDATED until isolated disposable VM acceptance.

## Isolated synthetic root enumeration proof (2026-10-15)

Added the **test-only container filter** in
`experiments/native-ifilter-fixture/fixture_filter.cpp`: one private
container COM CLSID, one `CHUNK_VALUE` with the exact documented
`PKEY_Search_UrlToIndex` GUID and property ID 2, and a caller-owned
`PROPVARIANT(VT_LPWSTR)` containing precisely the fixed synthetic
document URL. `GetValue` ends after one value and `GetChunk`
ends after one chunk. Each `IFilter::Init` resets enumeration for
deterministic repeated traversals.

The private protocol factory accepts the root with or without
a trailing slash, distinguishes it from the document using
`IUrlAccessor::IsDirectory`, rejects a document stream for the
root, and asks its **injected in-process test site** to bind
the container filter via `IProtocolHandlerSite::GetFilter`.
The synthetic document still uses the existing content filter.
Both root and child use the previous restricted, process-user
test ACL. Unknown nested URLs and real `file:` paths remain
rejected.

**Proof:** The private, unregistered COM test uses real SDK
`ISearchProtocol`, `IUrlAccessor` and `IFilter` interfaces.
It asserts the exact `PKEY_Search_UrlToIndex` identifier,
`CHUNK_VALUE`, the single child URL, end-of-enumeration,
repeat traversal, root metadata/security and byte-for-byte
Unicode document text. Output marker:
`NATIVE_PROTOCOL_ROOT_CHILD_IFILTER_ACL_NO_REGISTRATION_PASS`.
Tests run without opening Search, altering registry, installing
COM or registering any Windows Search crawl root.

**Still a hard P0 BLOCKER:** Microsoft documents that a real
hierarchical protocol handler needs a registered crawl root and
a container filter to enumerate URLs, but a locally mocked
`IProtocolHandlerSite` is **not** a Windows Search host crawl.
No Shell namespace open-result behavior, end-user token
impersonation, installed SearchProtocolHost execution,
SystemIndex result, Explorer result or native Start/Win+S result
has been demonstrated in disposable Windows 10 and 11 VMs.
In particular, this proprietary synthetic data-store approach
does not demonstrate modifying the existing Windows `file:`
provider's ranking. Preserve the normal Start/taskbar/Win+S
workflow without launching a companion app, PR DRAFT and
release INVALIDATED.

## Robust short-read IStream filter contract (2026-10-15)

The synthetic document filter previously assumed one `IStream::Read`
returns the complete payload. COM streams may legally return fewer bytes
than requested. It now reads multiple bounded chunks, counts the
cumulative 8192-byte maximum, preserves UTF-8 split across chunks and
rejects an error midway through an input without publishing incomplete
text. The test harness wraps `IStream` with an independent COM stream
implementation that returns 1, 2, 3, 7 or 255 bytes per call, and
injects an explicit read error or `S_OK` with zero bytes before EOF.

Observed Windows in-memory streams can also report `S_OK` plus zero
bytes at legitimate EOF. For that exact case, the filter checks that
`IStream::Seek(STREAM_SEEK_CUR)` equals the size returned from
`IStream::Stat`; without proof of genuine EOF it returns
`STG_E_READFAULT`, not a silently truncated index entry. Regression
checks include valid Turkish/German/Greek Unicode, the exact 8 KiB
maximum, an oversized fragmented document, a failed-load invalidation
and successful recovery with a subsequent new input. The native C++
build and complete unregistered protocol → document IFilter test PASS.

**Stop criterion:** Additional synthetic parser optimizations are
not a substitute for native Windows 10/11 user acceptance. The next P0
milestone must be an authorized disposable Windows 10/11 environment
with a rollback snapshot and *genuine* Start/Win+S Search-host evidence.
Until that exists, these DLLs stay unregistered, the system remains
untouched, PR #81 stays DRAFT and release remains INVALIDATED.
