# Native Windows Search IFilter, unregistered test fixture

**Status:** Working C++ COM fixture, **not an installed Windows Search
provider**, not a replacement for Win/Start/Win+S results, and not a release
artifact. All test outputs remain under ignored `out/`.

## Purpose

This is the smallest functional example of a documented, supported extension
point that can allow Windows Search to parse a **new file format** without
changing the user's ordinary Windows search entrypoints:

- `fixture_filter.cpp` exports an `IFilter` and `IPersistStream` COM
  object, with class factory, reference accounting and a fixed private CLSID.
- The entire test format is **synthetic**: bytes start with
  `STFIXTURE-TEXT-1\n`, followed by UTF-8 text, at most 8192 bytes total.
  The input is provided by `IStream`, not opened from user paths.
- `fixture_test.cpp` loads the DLL **directly by path** and exercises its
  COM factory, text chunk streaming in small buffers, UTF-8 to UTF-16
  conversion including Turkish and Greek text, invalid magic, malformed UTF-8,
  oversized content, object lifetime and aggregation refusal.
- `DllRegisterServer` and `DllUnregisterServer` are hard-coded to return
  `E_ACCESSDENIED`, including in the test harness, to prevent accidental
  registration through `regsvr32`.
- Build + test: `pwsh -NoProfile -File experiments/native-ifilter-fixture/build_fixture.ps1`.
  The PowerShell wrapper discovers the actual installed compiler via
  Microsoft's `vswhere.exe`, including supported non-2022 Visual Studio
  install paths, and fails if no 64-bit MSVC environment is available.
  The lower-level `build_fixture.cmd` requires the wrapper's
  `SEARCHTOOL_VCVARS_PATH` environment variable and deliberately refuses
  silent fallback to an unrelated compiler. This avoids assuming a
  hard-coded runner image layout. Requires the Windows SDK and MSVC;
  runs only the private DLL loaded into the test process, and does **not**
  modify the registry, Windows Search, Start, taskbar or Explorer.
- CI compiles/tests on `windows-latest` without registration.

## Scope of what this proves

A binary implementing `IFilter` and `IPersistStream` compiles against
the Windows SDK and can extract genuine text content from a custom
in-memory format through its COM interface.

**It does not establish:**

1. Registration, SearchProtocolHost loading or content indexing through the
   operating system. There is no `.stfixture` file association or COM
   registration in this PR.
2. SystemIndex, File Explorer or native Start/Win+S result visibility.
3. Integration of the separate Rust Search Tool index with the Windows
   file ranking/search engine. A filter for a proprietary format is only
   an extension for **new content types**, not an API for re-ranking
   Microsoft's own indexed files.
4. Safe OS installation, uninstallation or physical keyboard/IME/Narrator
   interaction.

Microsoft: [Developing Windows Search filter handlers](https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-conceptual),
[Implementing filter handlers](https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-constructing-filters),
and [Windows Search IFilter details](https://learn.microsoft.com/en-us/windows/win32/search/-search-ifilter-about).

## The actual P0 acceptance gate

On separate disposable Windows 10 and Windows 11 VMs with proven snapshot
rollback, a future isolated installer could register this test format
under a **private extension**, then establish the distinct evidence for
SystemIndex indexing, Explorer native search and Start/Win+S native result
visibility. The fixture must not register/repurpose existing `file:`,
`search:`, `search-ms:`, global hotkeys, SearchHost or Explorer shell
extensions. Do not promote or install this registration-disabled proof
directly on the user's host.

The main Search Tool product must never demand launching a separate app:
native Windows 10/11 search behavior is the absolute acceptance condition.
If the protocol handler and IFilter routes cannot produce a meaningful
improvement in Start/Win+S without a companion app, **do not ship this
experiment as a replacement**. See
`docs/NATIVE_SEARCH_NO_APP_ACCEPTANCE.md` and PR #81 / issue #82.

## Unregistered protocol-to-filter pipeline proof

New private DLL `protocol_fixture.cpp` implements the actual Windows
SDK `ISearchProtocol` and `IUrlAccessor` COM interfaces. Its
`ISearchProtocol::CreateAccessor` will only accept the **single fixed**
URL `stfixture-proto-test://root/document`; attempts to access a
`file:` URL, an unknown URL, or an uninitialized/shutdown protocol
are rejected. The accessor reports a synthetic MIME type/filename,
fixed modification timestamp and size, and supplies a fresh in-memory
`IStream` containing the original UTF-8 fixture document. Its
`DllRegisterServer` and `DllUnregisterServer` both return
`E_ACCESSDENIED`, and no CLSID or protocol name is registered.

The separate `protocol_test.cpp` loads this DLL directly, tests COM
factory and reference lifetimes and all allowed/disallowed URL paths,
then loads **the original private IFilter DLL** directly and feeds the
real `IUrlAccessor::BindToStream` bytes to its
`IPersistStream::Load` and `IFilter::GetChunk/GetText`. It checks
the final UTF-16 text includes correct Turkish, German and Greek
characters. The end-to-end harness prints:

`NATIVE_PROTOCOL_ROOT_CHILD_IFILTER_ACL_NO_REGISTRATION_PASS`

`build_fixture.ps1` now builds both independent native DLLs and test
executables using Windows SDK/MSVC and runs both tests in the same
developer-only process context. CI does exactly the same on Windows.
Nothing is loaded by Microsoft SearchProtocolHost, SearchHost,
Windows Explorer, an installed Shell extension, or the Windows
registry.

**Security and hosted filter binding (still test-only):**
For the single no-SID synthetic URL, `IUrlAccessor::GetSecurityDescriptor`
constructs a Windows **self-relative security descriptor** with one
explicit read ACE for the process token's user SID, not Everyone,
Authenticated Users or a NULL DACL. It rejects missing length pointers
and short output buffers without partial writes. The in-process
harness validates self-relative format, owner/ACE SID equivalence and
restriction to the current token. No SIDs are logged or persisted.

This is important: Microsoft's
[GetSecurityDescriptor documentation](https://learn.microsoft.com/en-us/windows/win32/api/searchapi/nf-searchapi-iurlaccessor-getsecuritydescriptor)
states that `E_NOTIMPL` for an URL **without a user SID** allows
the item to appear in all users' search results. The old unregistered
fixture returned `E_NOTIMPL`; this is now corrected before any
possible VM installation. A proper production provider needs security
impersonation, per-user ACL mapping and isolated threat review.

`IUrlAccessor::BindToFilter` now calls the **provided**
`IProtocolHandlerSite::GetFilter` for the private CLSID/MIME/extension
and loads the returned handler with the accessor's synthetic stream.
The test provides an in-process site double that chooses the separate
unregistered IFilter factory, checks the returned text chunks and
ensures COM lifetimes are balanced. With no site provided,
`BindToFilter` still fails closed with `E_NOTIMPL`. The protocol
handler never loads a filter DLL itself or calls a globally registered
COM class.

**Test-only root-to-document enumeration:** In addition to the one
synthetic document, `ISearchProtocol` now recognizes the synthetic
root `stfixture-proto-test://root` with or without its trailing slash
(the indexer may trim the final slash). `IsDirectory()` reports TRUE,
`BindToStream()` rejects root access, and the root MIME selects a
second COM class in the same *unregistered* fixture filter DLL.
That container `IFilter` produces exactly one `CHUNK_VALUE` with
`PKEY_Search_UrlToIndex` (Windows SDK GUID/propid), and its
`PROPVARIANT` holds the single exact synthetic document URL as
`VT_LPWSTR`. The caller owns its `CoTaskMem` allocations. The
in-process site-double test checks root metadata/ACL, short and
normalized URLs, child link value, end-of-chunks and repeated crawling;
all unknown nested URLs remain rejected. This matches the documented
child URL *data contract*, not a real Windows gatherer traversal.

**Deliberately incomplete for Windows Search indexing:** There is no
registered private scheme/crawl root, Shell namespace item open
semantics, account impersonation, security provider, isolated
installation/rollback or actual native Windows Search query evidence. A mocked
`IProtocolHandlerSite` is **not** Microsoft's SearchProtocolHost.
There is no native Explorer or Start/Win+S result; the user must not
see an additional application. Do not register on the interactive PC.

The current research stage is **unregistered COM pipeline proof**,
not acceptance of native Start results, arbitrary filesystem ranking,
or the user's requirement of invisible integration in normal Windows
10/11 behavior.

### Disposable Windows VM preflight (read-only, not registration permission)

Run `scripts/native-search-vm-preflight.ps1 -SelfTest` under Windows
PowerShell 5.1 or PowerShell 7, and then run it without parameters to
record the **local** build, guest signature and WSearch state. On a normal
developer workstation it must return `BLOCKED`, not an invented pass.
Its optional `-ManifestPath` accepts an **externally supplied** JSON
record with `schema: native-search-vm-evidence-v1`, `scope:
stfixture-proto-test`, matching `os_family` (Windows10/Windows11),
`operator_approved: true` and two distinct `snapshot_id` /
`rollback_rehearsal_id` receipts. A reported `PREFLIGHT_ONLY` still
does **not** validate the external receipts, authorize COM registration,
or establish a Windows Search result. A human must verify VM isolation,
snapshot restoration and guest permission before any separate test.
No production SearchHost/index/service state is modified by this script.

The evidence must then be recorded independently for actual WSearch
crawl, SystemIndex, Explorer, taskbar Search, Windows key/Start and Win+S,
original app/settings/files, ACL multiuser behavior, Shell open action
and exact uninstall/rollback in *both* Windows 10 and 11. If any native
surface cannot display the result, the full native UX stays BLOCKED.

### Partial COM stream input safety

`FixtureFilter::Load` now streams bounded chunks rather than assuming a
single `IStream::Read` fills the buffer. Tests exercise 1/2/3/7/255-byte
partial reads, UTF-8 split across boundaries, injected read errors,
zero-progress before EOF, exact 8192-byte acceptance, fragmented input
over the bound, failed-load invalidation and successful subsequent reload.
A legitimate COM memory stream's `S_OK` plus zero bytes is accepted only
when `Seek`/`Stat` prove the cursor equals the stream length; uncertain
zero-progress fails closed. These are private-COM tests, **not** evidence
of Microsoft SearchProtocolHost loading or native Win+S integration.
