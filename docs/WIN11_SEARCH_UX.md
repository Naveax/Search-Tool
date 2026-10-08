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

## Windows Search catalog read-only probe

The new `scripts/windows-native-search-audit.ps1` uses the officially documented read-only
`Search.CollatorDSO.1` OLE DB provider with one fixed `SELECT TOP 1` query against
`SYSTEMINDEX`. It reports only service status, whether an indexed row exists,
and query elapsed time. It never outputs the indexed file name, content or paths,
changes the Windows index, registers handlers, or alters SearchHost.

- Run the self-test with Windows PowerShell 5.1: `powershell.exe -NoProfile -File scripts/windows-native-search-audit.ps1 -SelfTest`.
- Run the actual local inspection: `powershell.exe -NoProfile -File scripts/windows-native-search-audit.ps1`.
- Live **single-query** read-only checks on October 8, 2026: Windows 11 NAVEAX
  `WSearch=Running`, SQL `PASS`, one row; Windows 10 work-pc
  `WSearch=Running`, SQL `PASS`, one row. These are connectivity checks,
  **not** evidence that Search Tool accelerates the native Search flyout.
- Source: [Microsoft: using SQL and AQS to query the index](https://learn.microsoft.com/en-us/windows/win32/search/using-sql-and-aqs-to-query-the-index).
  Source: [Microsoft: Windows Search development platform](https://learn.microsoft.com/en-us/windows/win32/search/-search-3x-wds-development-ovr).

The opt-in Search Tool flyout now derives its anchoring edge from the actual
monitor/work-area margins (bottom/top/left/right) and clamps on compact screens.
This does not confer taskbar Search-button integration or native SearchHost
rendering parity.

**Next integration decision:** if Search Tool-specific non-filesystem content
must appear in Windows Search/Explorer, prototype a minimal signed protocol
handler with Shell namespace support in an isolated VM first. For ordinary
existing filesystem files, do not install duplicate protocol handlers or
rewrite Windows Search configuration automatically. No COM extension
registration is included in this iteration.