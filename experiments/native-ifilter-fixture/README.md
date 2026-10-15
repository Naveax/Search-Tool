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
- Build + test: `cmd /c experiments\native-ifilter-fixture\build_fixture.cmd`.
  Requires Windows SDK and Visual Studio 2022 MSVC; runs only the private
  DLL loaded into the dedicated test process, does **not** modify the
  registry, the Windows Search service, Start, taskbar or Explorer.
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
