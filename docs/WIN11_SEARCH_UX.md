# Windows 11 Search continuity: native-like flyout iteration

The product goal is to improve search speed without making users learn a new interaction.

## Implemented in this iteration
- Resident Search Tool popup is anchored just above the taskbar using the selected monitor's work area; it is clamped for narrow screens and negative multi-monitor origins.
- The independent standalone window keeps its earlier placement for compatibility.
- Resident popup closes on loss of activation, as users expect from a search flyout.
- Signature/default-looking resident sessions resolve to Windows' system appearance; explicit alternative appearance presets remain available.
- No permanent vivid accent rail in Native result rows, muted Native selection background, compact ellipsis appearance control, simpler title and subtitle.
- Unit tests cover ordinary, ultra-wide, compact and negative-monitor work areas.
- Native Windows Search, Win+S, taskbar, Start, and File Explorer remain owned by Windows. The legacy keyboard bridge stays opt-in.

## Acceptance requirements not yet satisfied
- Record an actual *Windows 11* reference Search flyout's screen bounds, scaling, focus transitions, keyboard navigation and visual structure at relevant resolutions.
- Compare the independently launched GUI's position, size, hierarchy and focus behavior against those measurements, not an assumed pixel-for-pixel clone.
- Confirm opening Windows Search from its taskbar button and Win+S has unchanged native behavior. There is no supported API in this project for substituting a third-party Win32 surface *inside* the Windows SearchHost flyout. Exact same launch surface and zero behavioral difference must not be marked PASS without an independently verified, supportable integration path.
- Validate search semantics: app/settings/file matching, ranking, keyboard-first navigation, IME, pointer/touch, high contrast and screen-reader behavior.
- Do not modify the installed production ZIP/service/index until physical GUI review, sealed build, rollback and Windows CI gates are complete.

## Out-of-scope physical gate
The previous mixed-DPI hardware gate is no longer a required user deliverable: GitHub Issue #6 was closed as `not_planned`. Its historical `BLOCKED` release evidence remains in the audit record, not a reason to hold the already VALIDATED native-polish production release. Do not fabricate a passing hardware test.

## Supported Windows integration research

Microsoft's [Windows Search development platform](https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr) documents protocol and file handlers for adding indexed data and extending Explorer search, not arbitrary third-party visual replacement of `SearchHost.exe`. Microsoft also documents [third-party web search providers](https://learn.microsoft.com/en-us/windows/apps/develop/search/search-providers), but that is an EEA-specific MSIX feature for **web** results and does not establish a general-purpose local file Search panel replacement. [`search:` protocol registration](https://learn.microsoft.com/en-us/windows/win32/shell/search-protocol) can select an app for that URI but must not be represented as taking over the taskbar's Search button or Win+S.

**Architectural decision:** preserve Windows-owned Search UI and explore official Windows Search indexing/handler integration if same-entry-point search is the non-negotiable goal. The separate Search Tool popup remains optional until real SearchHost parity and supported integration are proven. Do not enable the low-level keyboard bridge by default or rewrite protected shell ownership.

## Development state

The new GUI source is a pending package: `docs/RELEASE_STATE.json` is deliberately **INVALIDATED** on the feature branch until new source is packaged, physically visually reviewed on Windows 11 and passes the release gate. The already installed prior production build and its evidence remain valid as historical deployed artifacts.