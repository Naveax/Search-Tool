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
