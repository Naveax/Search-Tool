# Native Windows Search / Explorer integration decision

Last updated: 2026-10-05

## Decision

Search Tool keeps normal Windows Search, Start, and File Explorer surfaces owned and drawn by Windows.

The release path uses documented Windows Search and Shell extensibility only. Search Tool does not replace the system search experience by default.

The installed resident process therefore remains background-only by default:

```
--resident --no-shell-bridge
```

The legacy shell bridge remains opt-in only for explicit compatibility testing.

## Supported Microsoft integration models

### Windows Search protocol handlers

A protocol handler exposes a data store to the Windows Search indexer. Microsoft documents `ISearchProtocol` and `ISearchProtocol2` for custom stores.

The important architectural consequence is that Windows Search crawls and indexes that store into its own catalog. This does not make Search Tool's MFT/USN query engine the live backend of the existing Windows Search result list.

References:

- https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-extidx-prot-implementing
- https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr

### Shell data sources

Microsoft documents a Shell data source, historically called a Shell namespace extension, as the preferred way to expose custom data-store items through Windows Explorer and Shell functionality.

This can make custom-store items participate naturally in Explorer / Windows Search, but Windows Search still executes the catalog query.

References:

- https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr
- https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-overview

### The system search protocol

Windows allows another desktop search application to register as the handler for `search:`.

That routes the request to that application. It does not extend the existing Microsoft Search UI with a different local-file query backend.

Search Tool intentionally does not register `SearchTool.Search`, `search:` OpenWith, Capabilities, or RegisteredApplications ownership in the native-first configuration.

Reference:

- https://learn.microsoft.com/en-us/windows/win32/shell/search-protocol

### Windows Search Explorer / search-ms

The Windows search protocol documentation also describes `search-ms:` as a way to launch or query Windows Search Explorer.

It still uses the Windows Search catalog rather than Search Tool's own index.

Reference:

- https://learn.microsoft.com/en-us/windows/win32/shell/search-protocol

### Windows Search web providers

Microsoft documents a Windows Search web-provider app extension for web content in the EEA. The OS sends queries to an HTTPS endpoint and receives JSON suggestions plus optional HTML preview content.

That path is intended for web search content and is not an offline local-file backend integration for Search Tool.

Reference:

- https://learn.microsoft.com/windows/apps/develop/search/search-providers

## Current architecture

Normal user entry points remain Windows-owned:

- Win / Start typing;
- taskbar Search;
- File Explorer search box;
- Start / Explorer visual styling.

Search Tool can manage supported Windows Personalization/DWM state such as app/system light-dark mode, transparency, accent color, and supported accent surfaces. The controls themselves remain Windows controls.

Search Tool's own MFT/USN backend remains available through:

- the installed service/index;
- CLI;
- the private `searchtool:` protocol;
- explicit scoped helper entrypoints;
- the separate GUI only when deliberately invoked.

The default install does not expose the separate Search Tool search GUI as a Start Menu application.

## Future experiment boundary

A separate experimental branch may evaluate a native COM Shell data source plus Windows Search protocol handler for custom or non-file-system data stores.

Any such experiment must:

- use documented Shell / Windows Search interfaces;
- leave normal Windows Search ownership unchanged by default;
- uninstall cleanly without leaving Shell/indexer registration residue;
- be validated on supported Windows 10 and Windows 11 builds;
- state clearly that the Windows Search catalog executes the query.

Because Search Tool already indexes ordinary NTFS files directly, duplicating the same files into a second custom Windows Search data store is not a useful primary architecture unless it demonstrates a measurable user-facing benefit.

## Rejected release architecture

The release must not depend on undocumented modification of Windows SearchHost or Explorer internals, private UI implementation details, or a lookalike panel that hides the real Windows surface.

Those approaches are brittle and contradict the native-first requirement.

## Consequence

In the Microsoft documentation reviewed for this milestone, there is no supported public API that preserves the exact Microsoft SearchHost result UI while replacing its local-file query engine with Search Tool's MFT/USN backend.

The production design is therefore:

**native Windows UI + native Windows theme state + Search Tool background service/index + explicit Search Tool entrypoints when its custom backend is required.**
