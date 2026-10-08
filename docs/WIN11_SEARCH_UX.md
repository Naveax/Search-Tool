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
changes the Windows index, registers handlers, or alters SearchHost. It also
checks for the Windows taskbar's locale-independent UI Automation
`SearchButton` and records only its numeric bounds if visible. This does
not toggle or click the button; unsupported/hidden buttons are reported
as `found=false` rather than treated as a failed SystemIndex probe.

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

## Real Windows 11 visual reference (2026-10-08)

The actual taskbar Search button was opened through its native UI Automation
TogglePattern for a short, bounded inspection, then returned to its original
closed state. The private desktop screenshots were **not committed** because
they include other user desktop windows. Only sanitized, approximate layout
metrics are in `docs/evidence/windows11-search-reference-20261008.json`.

At 3440x1440 / 96 DPI on Windows 11 build 22631, the native flyout occupied
approximately 776x725 physical pixels, centered horizontally just above the
48px bottom taskbar. The native taskbar SearchButton bounds (UIA exact) were
x=1457, y=1392, 44x48. Its top search field starts roughly 32px below the
flyout border, navigation pills about 80px down, followed by best-match
results and a second details/actions column.

The pending **resident-only** Native preset uses 780x720 logical dimensions
for default-size Windows 11 sessions and moves the search field/category row
toward the measured native offsets. Explicit non-default user window
dimensions, standalone Search Tool windows and Windows 10 layouts are
preserved. This remains an independent Search Tool interface. The two-column
Windows Search application/details experience, native ranking, SearchHost
rendering and native Search-button ownership are **not implemented** and
must not be marked as passing.

### Keyboard-first behavior (pending GUI iteration)

When focus remains in the query field, **Enter now opens the first result** if
results exist and no other row is selected. If a result is selected, Enter
retains the selected row instead of silently replacing it with the first.
Down from the query field moves to the first result; Up from that first row
returns to the query field. Empty-result Enter has no side effect. Deterministic
unit tests cover the first-result fallback and no-result/selected-row guards.
This aligns the optional popup with common search keyboard habits; it does
**not** establish native SearchHost application/settings category parity.

### Resident launch bug fixed

In the earlier implementation, WM_CREATE consumed `State.initial_request`
via `.take()` before startup decided whether to show the resident window.
Therefore a real `--resident --query ...` launch was mistakenly hidden, even
though the user explicitly requested Search Tool. Startup visibility now
captures the request flag **before** CreateWindowExW and activates the
input box for explicit requests. Background-only resident startup still
begins hidden. A deterministic unit test covers all launch modes.

A short physical Windows 11 UI acceptance probe at the exact development
commit `db0515f27ee9c14106f271c34efd32e279d30142` verified
`--resident --ui-preview --query searchtest` was visible in 9/9 polls
over 855ms, while idle `--resident --ui-preview` was hidden in 5/5
polls. The popup **was not the foreground window** in that active user
session, so foreground and complete visual parity remain unverified.
Both test processes were terminated. Sanitized measurements live at
`docs/evidence/windows11-resident-launch-20261008.json`.
These are transient developer preview acceptance checks, not
SearchHost/taskbar takeover or production deployment.



### Fluent-style native resident preview iteration

A physical Windows 11 PrintWindow capture of the independent 780x720
resident GUI exposed dated Win32 visuals: hard rectangular search borders,
square accent-filled category buttons and a blank slab when no results
were available. These were **observed**, not inferred from unit tests.

The next optional preview iteration uses GDI rounded search-field
surfaces and rounded category chips with subdued native selection and
a small accent underline. In native Windows 11 mode only, the result
list uses the flyout background and is hidden when there are no results;
a legible in-place empty-state explanation replaces the unused slab.
Other themes and Windows 10 standalone behavior remain unchanged.

This still does **not** implement the Windows-owned taskbar Search panel,
the two-column app/details layout, native category semantics or native
result ranking. It needs real Windows 11 preview screenshot comparison
and keyboard/accessibility checks before being marked visually accepted.

**Next integration decision:** if Search Tool-specific non-filesystem content
must appear in Windows Search/Explorer, prototype a minimal signed protocol
handler with Shell namespace support in an isolated VM first. For ordinary
existing filesystem files, do not install duplicate protocol handlers or
rewrite Windows Search configuration automatically. No COM extension
registration is included in this iteration.
### Two-column Best Match detail preview (2026-10-08)

The optional Windows 11 resident Native GUI now splits its 780x720 flyout
into left result rows and a right details surface. Real Win32 STATIC controls
show name/type/path, and a keyboard-focusable Open button uses the
existing selected-result open operation. Selection changes refresh the detail.
A compact window retains the original full-width single list.

At 96 DPI: list=(24,158)..(396,696), detail=(412,158)..(756,696).
This is an optional Search Tool popup, not Windows SearchHost.

A physical Windows 11 PrintWindow capture of six synthetic index records
showed three result rows and the right pane without disturbing the active
desktop. All three synthetic row changes updated detail name/type/path and
kept the Open button visible. Details are hidden at the start of each
query refresh to avoid stale content during empty-query early returns.

External SetWindowText did not change the classic Edit control's internal
buffer used by the app; UIA ValuePattern was unavailable. Therefore
live typing/IME/empty-query behavior is NOT physically accepted based on
those inconclusive external tests. The synthetic paths do not exist,
so the Open action was not clicked.

Sanitized evidence: docs/evidence/windows11-search-two-column-20261008.json.
Real screenshots remain private in the isolated review folder.

### Detail cleanup and long-path legibility (subsequent iteration)

The optional two-column flyout now clears the underlying Win32 STATIC texts
for the selected result name, kind and full path whenever the selected detail
becomes unavailable. Hiding a STATIC alone leaves its previous text readable
through automation/Win32 inspection, so explicit clearing prevents stale
information from surviving an empty or no-result query. Native result title
and path fields use Win32 end/path ellipsis rather than overflowing the card;
full text remains stored for accessibility clients when a result is selected.
A deterministic test checks the selected-file -> empty -> selected-folder ->
empty detail-content transitions.

At this iteration 38/38 GUI Rust unit tests, Clippy (`-D warnings`), Release
build and `--smoke` passed on NAVEAX Windows 11. The original physical
PrintWindow image predates these changes; a fresh UI screenshot and actual
keyboard/IME query-clear/repopulate regression remain **NOT VERIFIED**.
The isolated desktop API declined access, so no physical UI PASS is asserted.
None of this alters Windows SearchHost ownership or the native Search entry
point, and `RELEASE_STATE` must remain INVALIDATED until full acceptance.
