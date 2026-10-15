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

`NATIVE_PROTOCOL_TO_IFILTER_END_TO_END_NO_REGISTRATION_PASS`

`build_fixture.ps1` now builds both independent native DLLs and test
executables using Windows SDK/MSVC and runs both tests in the same
developer-only process context. CI does exactly the same on Windows.
Nothing is loaded by Microsoft SearchProtocolHost, SearchHost,
Windows Explorer, an installed Shell extension, or the Windows
registry.

**Deliberately incomplete for Windows Search indexing:**
`IUrlAccessor::BindToFilter` returns `E_NOTIMPL`; user ACL/security
descriptor retrieval is not implemented, because claiming unrestricted
read access to actual user items would be unsafe. The fixture has no
folder-container child URL enumeration (`PKEY_Search_UrlToIndex`),
registered private scheme/crawl root, Shell namespace item open
semantics or isolated installer/rollback. An in-process manual
`IStream -> IFilter` connection **does not mean** the Windows
gatherer can instantiate the filter or that Start/Win+S renders the
result. Do not register the fixture on an interactive user's PC.

The current research stage is **unregistered COM pipeline proof**,
not acceptance of native Start results, arbitrary filesystem ranking,
or the user's requirement of invisible integration in normal Windows
10/11 behavior.
