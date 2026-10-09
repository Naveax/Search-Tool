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

### Native Shell icons and keyboard focus (following iteration)

The optional Windows 11 flyout now uses Windows Shell file-type and folder
icons rather than hard-coded geometric glyphs. `SHGetFileInfoW` is called with
`SHGFI_USEFILEATTRIBUTES | SHGFI_ICON | SHGFI_SMALLICON`, passing only a
bounded extension/type key, **never the actual indexed file path**. This
also works with the isolated synthetic index, whose paths do not exist.
Icons are cached per type (maximum 96 keys) and owned `HICON` resources are
released via `DestroyIcon` at window destruction. A real local Windows Shell
unit test resolves synthetic file/folder icon handles and frees both.

The results ListBox now participates in Tab navigation and paints its own
focus rectangle for keyboard navigation. Enter opens search results only
when the focused control is the search Edit or results ListBox; buttons for
appearance, category filters and the detail Open action receive their native
keyboard events. Unit tests cover non-result focus and icon type keys.

NAVEAX local Windows 11 results: 41/41 GUI Rust tests PASS, GUI Clippy
`-D warnings` PASS, release build PASS, `--smoke` PASS. The full Windows 11
visual review, actual IME/key focus tree, and live query clear/repopulate
regression have **not** been accepted: Nexowire's private-desktop access was
forbidden and no new screenshot is claimed. This iteration does not replace
Windows-owned SearchHost or change the installed production release.

### Automated hidden Win32 regression (following iteration)

The developer-only `--ui-selftest` mode requires an explicitly provided
isolated index directory. It always creates a hidden resident Native GUI,
never foregrounds an existing instance, and exits after checking real EDIT,
LISTBOX and detail HWND state. A stable six-record synthetic fixture lives
at `tests/fixtures/gui-synthetic-index`; three entries match `SearchTool`.
The fixture does not contain real user files, and no file is opened. The
index reader may create a transient ignored `C.stidx.publish.lock` file in
the test fixture; it does not modify the installed production index.

The test covers initial three-result search, detail selection updates,
empty query, whitespace query, no-match query, cleared hidden details and
restored results. It explicitly sends Win32 `WM_COMMAND`/`EN_CHANGE` after
same-process `SetWindowTextW`, and `LBN_SELCHANGE` after `LB_SETCURSEL`,
because programmatic hidden controls do not consistently generate these
notifications. This exercises the real application message handlers but
is **not** evidence of physical typing, IME or screen-reader parity.

`pwsh -NoProfile -File .github/scripts/windows-gui-hidden-regression.ps1`
repeats this regression using the built Release executable and a bounded
20-second timeout. Its exact spawned process is cleaned on timeout/failure;
the self-test never shows a modal error dialog. Windows GitHub Actions runs
the test after the workspace Release build. Initial NAVEAX local run passed
with process exit code 0 and `PASS` report; CI acceptance for this commit
must be checked separately. The installed Search Tool production release
remains unchanged and the source release-state is still `INVALIDATED`.

### Native Tab / Shift+Tab message translation (following iteration)

The previous custom Win32 popup had `WS_TABSTOP` styles on its Edit,
ListBox and owner-drawn buttons, but its raw `GetMessageW` loop only called
`TranslateMessage` / `DispatchMessageW`. Those styles alone do not
translate Tab keys into focus traversal. The loop now routes **only**
`WM_KEYDOWN / VK_TAB` through `IsDialogMessageW`, and skips normal
message dispatch if Win32 handled it. Return/Enter, Escape, Up/Down,
system keys and ordinary Unicode text/IME dispatch retain their existing
paths, preventing the dialog manager from rewriting Search Tool's search
semantics.

The Windows hidden GUI regression now calls real
`GetNextDlgTabItem` on the created controls. It checks both directions
(Edit <-> ListBox), ListBox -> first category, last category -> appearance,
and appearance -> Open, with a selected result. When a query is empty,
whitespace or has no matches, the hidden results ListBox is correctly
skipped; it rejoins Tab order on repopulation. Native 42/42 Rust GUI tests,
Clippy and the hidden regression passed locally for this iteration.

These checks establish deterministic Win32 control ordering and the
message-loop translation path, **not** physical Tab/Shift+Tab input,
screen-reader support, IME correctness or pixel-perfect Windows Search
parity. The Nexowire private-desktop API again returned `FORBIDDEN`.
Source: Microsoft Learn `IsDialogMessageW` and `GetNextDlgTabItem`
documentation. PR #81 stays DRAFT and the release remains INVALIDATED.

### System High Contrast color safety (subsequent iteration)

The optional Search Tool Win32 popup now reads Windows' High Contrast state
with `SystemParametersInfoW(SPI_GETHIGHCONTRAST)`, without changing that
system setting. When it is enabled, the popup uses the user-selected Win32
`GetSysColor` colors (`COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_HIGHLIGHT`,
`COLOR_HIGHLIGHTTEXT`) for its surfaces, text, selected results and actions.
It disables translucent/backdrop effects and background-image painting;
colorful Shell result icons are not painted and row text uses the reclaimed
space. Normal user-selected themes remain unchanged when High Contrast is off.

`WM_SYSCOLORCHANGE` refreshes the effective palette; `WM_SETTINGCHANGE`
refreshes it when the High Contrast flag changes. Brush replacement first
allocates the new brushes and updates the Win32 class background brush before
releasing old handles, avoiding a dangling class HBRUSH on runtime updates.
The regular theme control still uses its existing runtime theming path.

A pure unit test checks an artificial High Contrast palette and COLORREF
channel order, and the hidden GUI regression sends a synthetic
`WM_SYSCOLORCHANGE` and checks that the computed palette and all three
synthetic search results survive. No Windows user accessibility setting is
modified by tests. This is deterministic code-path validation, **not** a
physical High Contrast screenshot, full screen-reader verification or a
claim of exact Windows SearchHost parity. Microsoft's documented guidance:
https://learn.microsoft.com/en-us/windows/win32/winauto/high-contrast-parameter.
Release remains INVALIDATED and the production build is unchanged.

### Native MSAA accessibility labels and result descriptions

The optional Windows Search Tool popup now creates non-focusable hidden STATIC
labels immediately before the native EDIT and owner-drawn LISTBOX controls.
Their accessible names are "Arama sorgusu" and "Arama sonuçları". As the
LISTBOX already uses LBS_HASSTRINGS, its LB_ADDSTRING accessibility text now
includes title, file/folder type and full path; the owner-drawn visual layout
is unchanged. Clearing results clears these accessible strings too.

The developer-only --ui-selftest-inspect-ms option keeps the hidden synthetic
GUI responsive for a bounded 0..30000ms with PeekMessageW and DispatchMessageW,
allowing external MSAA clients to query the controls. Normal CI self-testing
has zero extra delay. The PowerShell 5.1 script
.github/scripts/windows-gui-msaa-regression.ps1 waits for test HWND creation,
checks both control names and the first synthetic result through
AccessibleObjectFromWindow / IAccessible::get_accName, and always terminates
only its own hidden test process. A separate Windows GitHub Actions gate
executes this test after Release compilation. No desktop input, global
accessibility setting changes, installed index, or production GUI is involved.

Real Windows 11 NAVEAX MSAA checks passed for EDIT and LISTBOX names and one
synthetic item. A distinct UI Automation tree probe of the hidden parent
exposed limited/generic data; UI Automation parity, real screen-reader
speech, focus behavior with a visible GUI, IME, and SearchHost integration
remain NOT VERIFIED. PR #81 stays DRAFT, Issue #82 stays open and release
state stays INVALIDATED.

References:
- https://learn.microsoft.com/en-us/accessibility-tools-docs/items/win32/edit_name
- https://learn.microsoft.com/en-us/windows/win32/winauto/exposing-owner-drawn-list-box-items

### Owner-drawn category/button MSAA states (following iteration)

The four category chips were already standard Win32 owner-drawn BUTTON
controls, with their HWND captions populated by update_tab_labels. The
previous selected state prepended a decorative bullet, which did not explain
the meaning of the state to accessibility readers. Selected captions now use
explicit Turkish text such as "Tümü (seçili)" and "Dosyalar (seçili)";
unselected captions retain the plain labels. The visual chips continue to
render their existing fixed text/selected underline independently of the HWND
caption. The Appearance ("Görünüm") and Open ("Aç") button names are preserved.

The synthetic hidden Win32 regression now sends actual WM_COMMAND/BN_CLICKED
category notifications, checks that accessible selection captions change on
All -> Files -> All and that the expected search results return. The
Windows PowerShell 5.1 MSAA CI regression calls GetDlgItem on all six
owner-drawn buttons and verifies IAccessible::get_accName for each.
It then switches category through the hidden parent and verifies the
name change through actual MSAA, followed by restoration to All. Tests
do not click the physical desktop, open a result, or alter indexes.

Passing MSAA names do **not** establish semantic TogglePattern/SelectionItem
support in UIA, actual Narrator speech, visible keyboard interaction, or
Windows SearchHost parity. Those remain physical acceptance tasks.
PR #81 remains DRAFT, Issue #82 OPEN and release INVALIDATED.

### IME composition-aware keyboard routing (following iteration)

The optional Win32 Search Tool popup has custom WM_KEYDOWN handling for
Enter (open best result), Escape (close resident flyout), Up/Down navigation
and IsDialogMessageW Tab traversal. Those actions must not run while a user
is using an input method editor (IME) to compose or select text inside the
native EDIT control.

An EDIT-only SetWindowSubclass now observes WM_IME_STARTCOMPOSITION and
WM_IME_ENDCOMPOSITION; it always forwards the message to the native edit
procedure, leaving IME text and candidate rendering to Windows. An active
composition prevents the popup's global keyboard handling and defers
EN_CHANGE result refresh for partial/preedit text. On completion, the
subclass clears the guard and refreshes from the current native edit text.
WM_KILLFOCUS also clears the guard and refreshes so cancelled or interrupted
compositions cannot leave shortcuts suppressed. The normal Enter, Escape,
Up, Down, Tab and search paths remain unchanged outside composition.

A Rust unit test covers the focus/composition guard. The existing hidden
Win32 GUI regression now sends **synthetic** IME start/end and focus-loss
notifications to its isolated native EDIT, triggers native parent
WM_COMMAND/EN_CHANGE notifications and verifies no partial search, a
refresh on composition end or focus loss, recovery of normal search, and
a continuously hidden parent. No actual IME language settings, user input,
keyboard hooks, production index or SearchHost window are modified.

This establishes deterministic composition-boundary routing **only**.
Physical Japanese/Chinese/Korean IME typing, candidate selection,
committed-text timing, Escape/Enter interaction with real IME windows, and
visible focus transitions remain **NOT VERIFIED** and are still release
acceptance gates. API references:
- https://learn.microsoft.com/en-us/windows/win32/intl/wm-ime-startcomposition
- https://learn.microsoft.com/en-us/windows/win32/intl/wm-ime-endcomposition
- https://learn.microsoft.com/en-us/windows/win32/controls/en-change

PR #81 remains DRAFT, Issue #82 stays OPEN and release-state stays
INVALIDATED; no production deployment.

### Native result-list focus and visibility stability (subsequent iteration)

In the optional Windows 11 resident Native popup, every query previously
called ShowWindow(SW_HIDE) on the LISTBOX *before* computing results and then
called ShowWindow(SW_SHOW) again after matches arrived. Since search is
synchronous, these intermediate style changes were unnecessary. More
importantly, hiding a focused results LISTBOX risks leaving focus on a hidden
control, disrupting the expected keyboard query workflow.

Search rebuilding now leaves the visible LISTBOX shown while searching and
only changes its visibility after the actual result count is known. On
empty/whitespace queries, failed reads, search errors and genuine zero
matches, it hides the list. Immediately before hiding a focused ListBox,
it moves keyboard focus to the native EDIT, **but only when the parent popup
is actually visible**. This avoids stealing focus from the desktop when
the hidden deterministic self-test runs. Non-Native popup behavior and
normal result selection remain unchanged. A pure Rust unit test verifies
the visibility/focus guard, while the existing hidden Win32 regression still
checks 3-result -> empty/no-match -> 3-result transitions, result details,
and the ListBox's Tab eligibility.

This change has not been verified via physical typing, visible screenshot
or real Windows 11 Narrator/UIA focus tracking. It is not a claim of
pixel-perfect SearchHost entry-point parity. PR #81 remains DRAFT, Issue #82
remains OPEN, and source release-state remains INVALIDATED. The installed
production service and GUI are unchanged.

### Native accessible result-count names (subsequent iteration)

The standard owner-drawn LISTBOX already receives its MSAA name from the
preceding hidden non-focusable STATIC sibling. Its old name only said
"Arama sonuçları", which gave no result count to screen-reader users. The
name now includes the current number of matches, for example
"Arama sonuçları (0 sonuç)" or "Arama sonuçları (3 sonuç)". The same status
update path synchronizes the label with the actual results; the visual
flyout, result rows, selection handling and native SearchHost stay unchanged.

A pure Rust regression covers 0/1/3 label formatting. The deterministic
same-process hidden Win32 GUI regression checks initial 0, 3 hits, cleared
0 on empty/whitespace/no-match, and 3 again on repopulation. The separate
Windows PowerShell 5.1 MSAA regression verifies that
AccessibleObjectFromWindow/IAccessible::get_accName returns the 3-result
name from the real, isolated native LISTBOX control. An exploratory attempt
to mutate the EDIT from the **external** PowerShell test process did not
reliably deliver EN_CHANGE to the test popup and was discarded instead of
counting it as a successful 3->0->3 external MSAA test. Those transitions
are tested through the application's existing same-process Win32
notification harness. No real user data, Shell window or installed index is
accessed.

This is not proof of physical screen-reader announcements, live UIA events,
focus interaction or exact Windows SearchHost visual parity; these remain
open release acceptance gates. PR #81 stays DRAFT, Issue #82 stays OPEN and
package_status remains INVALIDATED. No production deployment occurred.

### Preserve the selected result across a synchronous search refresh

The optional native Search popup previously reset the owner-drawn LISTBOX
and unconditionally selected row 0 after each query or category refresh,
even if the user was navigating the second or third result and the same
file was still present. This could cause keyboard selection and the preview
detail card to jump unexpectedly.

Before rebuilding, the popup now saves the current selected result's
**full path**, not just its display name. After rebuilding it locates the
matching path in the new result set and restores the same row selection.
If the path was removed, it falls back to the first available result; if
there are no rows, it leaves the list empty and clears stale detail text.
Only the optional native Search popup has this restoration behavior.
No result data, file contents, native Windows Search entry point or
production index is modified.

A pure Rust test covers duplicate filenames with distinct full paths,
missing previous selection, default selection and empty results. The
hidden same-process Win32 regression selects the second synthetic hit via
LB_SETCURSEL + WM_COMMAND/LBN_SELCHANGE, reissues a real query update,
and verifies the second selected row and full path in the native detail
card persist. Existing tests still cover no-match clearing and
repopulation fallback. This does **not** constitute a physical Windows
11 Tab/arrow/focus or screen-reader acceptance test.

PR #81 remains DRAFT/OPEN, Issue #82 OPEN, and RELEASE_STATE INVALIDATED.
Installed production GUI, running service and live indexes are untouched.

### Consistent Down-arrow selection from the search query

The custom Win32 popup previously handled Down-arrow from the native EDIT by
unconditionally selecting row zero before moving keyboard focus into the
LISTBOX. After a user selected a later result and returned to the query via
Shift+Tab, the subsequent Down-arrow threw away that preserved selection,
contradicting the result-refresh selection behavior described above.

The GetMessage keyboard path now uses `prepare_query_down_selection`:
an existing valid selection is retained, a list with hits but no selection
chooses its first result and synchronizes the detail card, and an empty list
does nothing. Moving actual keyboard focus into LISTBOX stays in the normal
UI message loop, not the selection helper. This split lets the hidden
synthetic Win32 regression exercise the **same selection code** without
changing or simulating focus on the user's physical desktop.

Rust unit tests cover empty results, missing selection, first/later valid
selections, and out-of-range fallback. The hidden Win32 regression checks
that a previously selected second result and its full path survive the
Down-arrow selection preparation, that a cleared selection chooses row zero
with matching details, and that a zero-match list remains unselectable.
Normal Windows ListBox arrow handling after focus transfer is unchanged.
The same guard now also declines Down-arrow selection when the LISTBOX
control itself has been hidden by layout, even if cached result rows exist;
the hidden Win32 self-test temporarily toggles only its own LISTBOX style to
verify that no hidden control is selected or focused.

This is source and hidden-Win32 validation, not physical keyboard, real
IME, UI Automation/Narrator, Windows SearchHost visual parity or native
taskbar Search entry-point acceptance. PR #81 remains DRAFT/OPEN;
Issue #82 remains OPEN; package_status is INVALIDATED. No production
installation or live indexes were touched.

### Reconstructed result paths must be verified before opening

The indexed search store returns records whose full path must be
reconstructed through a parent chain. The Search Tool GUI previously used a
fallback built from just the drive letter and result filename when
reconstruction failed (for example `C:\name.txt`). That guessed location is
not evidence of the item's actual location and could misdirect the Open
action to an unrelated root-level file.

The GUI now accepts only **successfully reconstructed absolute drive paths**
with an actual drive separator and without embedded NUL characters. An
orphaned or unreadable parent chain, a drive-relative path such as
C:relative.txt, a relative-only filename, or an embedded NUL is excluded
from the actionable result list. This follows a fail-closed rule: do not
pretend to have located or opened an index hit if its path cannot be trusted.
The search-core index format and the underlying search algorithm remain
unchanged.

The native right-hand detail card now also strictly follows actual
LISTBOX selection. If there are indexed results but no selected row, no
first-result preview or Open button is offered until a valid selection is
made. Existing Down-arrow selection restores the first result where needed.

Pure Rust tests cover valid rooted drive paths, reconstruction errors,
drive-relative / relative strings, empty paths and embedded NUL. The
existing hidden Win32 self-test deselects a result, checks that detail path
and Open button are cleared, then uses the same Down-arrow selection helper
to repopulate the correct detail. These tests never ShellExecute synthetic
paths or modify production index files.

This is source-level result safety with deterministic Win32 regression
coverage. It does not establish visual, physical keyboard, IME or
screen-reader acceptance or native Windows SearchHost parity. PR #81
remains DRAFT, Issue #82 OPEN, and the release state INVALIDATED; no
production deployment occurred.

### Keyboard focus order follows the visual search layout

The optional native popup previously created its native EDIT immediately
before its LISTBOX, then the four owner-drawn category buttons. As Win32's
dialog keyboard navigation respects child creation/Z order, pressing Tab
from the search query skipped the visually intervening category chips,
jumped into results, and visited the categories only afterward.

The native controls are now created in visual reading order:
**search EDIT -> Tümü -> Dosyalar -> Klasörler -> İçerik -> results
LISTBOX -> Görünüm -> Aç** (when those controls are visible). The
non-focusable hidden STATIC immediately before EDIT and immediately
before LISTBOX remains in place to retain the verified accessible
control names and the result-count label.

The isolated hidden native Win32 regression uses GetNextDlgTabItem
rather than synthetic physical key injection to verify forward/reverse
traversal across EDIT, the category buttons, LISTBOX, appearance and
Open. It checks that a zero-result LISTBOX is skipped without skipping
categories and is reinserted at its correct place after repopulation.
The existing Windows MSAA regression confirms that moving the
controls did not break names, selected-category state, or accessible
result text. No Shell injection, production service or live index is
involved.

These are Win32 keyboard-order and MSAA tests. Actual physical
Tab/Shift+Tab, IME interaction, Narrator/UI Automation, pixel-level
Windows 11 Search visual parity and native SearchHost integration
remain NOT VERIFIED. PR #81 remains DRAFT, Issue #82 OPEN, release
status INVALIDATED; installed production GUI remains unchanged.

### Search Enter and Shell bridge Enter use one selection pathway

The normal Win32 EDIT VK_RETURN route selected Best match when no item
was selected, but the optional Shell bridge WM_SHELL_BRIDGE_KEY Enter
route called open_selected directly. When a query had results but the
LISTBOX had no selected row, Shell bridge Enter did nothing.

Both routes now use prepare_search_enter_selection before attempting
to open a result. It chooses row zero only when the event originates
from the search-query context, the LISTBOX has no selection, and
results exist. An explicit selection (including a later row) remains
untouched. Any new selection also synchronizes the native detail card.
Other buttons retain their usual Enter behavior.

The Shell bridge bypasses the normal GetMessage WM_KEYDOWN guard. Its
Enter handler now explicitly checks IME composition state. While the
native EDIT is composing text, the bridge forwards Enter to the EDIT
instead of invoking open_selected. Normal Search keyboard routing
already contains the matching IME guard.

The hidden Win32 regression exercises this shared selection helper
with a second chosen result, no selection, a non-query source and an
empty result list. It checks native LISTBOX selection and detail paths,
without calling ShellExecute, posting keyboard input to the desktop
or opening any synthetic indexed file. Rust guard tests cover the IME
routing predicate, while external MSAA tests still verify the native
control accessibility names. This is not a physical Shell bridge,
IME candidate-window or end-user Open acceptance test.

PR #81 remains DRAFT/OPEN, Issue #82 OPEN and release state INVALIDATED.
No production GUI, running service, user files or indexes are changed.

### Opening indexed results requires current file-system verification

A successfully reconstructed absolute path proves only where an indexed hit
*was*, not whether it still exists when the user activates Open. The native
Search popup now performs a final `std::fs::metadata` check immediately
before `ShellExecuteW`, rejecting missing/inaccessible paths and entries
whose current object type no longer matches the indexed file/folder kind.
The popup stays open and displays the status message "Seçili sonuç artık
mevcut değil veya türü değişti" instead of attempting to launch a stale
result. The check applies to Enter, double-click and the Open button
because each invokes the shared `open_selected` handler.

An isolated Rust test creates a small temporary folder and file, checks
correct and mismatched file/directory types, deletes the file and folder,
and verifies the paths are subsequently rejected. The hidden Win32 GUI
regression also substitutes a uniquely absent temporary path into a
synthetic result, calls the **real** `open_selected` handler, and asserts
it returns false and reports the reason without calling `ShellExecuteW`,
displaying a window or touching a production index. It restores the
synthetic result afterward.

This is a best-effort just-before-open validation, **not** an atomic guarantee
against another process changing the path between the metadata check and
`ShellExecuteW` (TOCTOU). It does not prove real user-facing Shell
integration or acceptance with physical keyboard, IME, Narrator, UI
Automation or native Windows SearchHost. PR #81 stays DRAFT/OPEN; Issue
#82 stays OPEN; release state stays INVALIDATED. Installed production
software is untouched.

### Native MSAA result-count name-change notifications

The optional native results LISTBOX exposes a Turkish count in its MSAA
accessible name (for example "Arama sonuçları (3 sonuç)"), sourced from the
preceding non-focusable STATIC control. Changing that static text alone
does not explicitly tell accessibility event listeners that the LISTBOX
name changed. The status-update path now compares the previous and next
accessible names, updates the backing STATIC **only on a real change**,
and invokes the documented Windows `NotifyWinEvent` API with
`EVENT_OBJECT_NAMECHANGE`, the native LISTBOX handle,
`OBJID_CLIENT`, and `CHILDID_SELF`.

Name-change events are emitted only when the containing popup window is
visible; hidden native regression windows do not emit them, and repeated
status updates with an unchanged count do not create redundant events.
A Rust unit test checks the changed/unchanged and visible/hidden gating.
The existing hidden Win32 regression verifies 0/3/0 count text updates,
and the independent native Windows MSAA test still reads the real three-hit
LISTBOX name. The tests **do not** capture cross-process WinEvents or prove
that Microsoft Narrator speaks the count. Physical Narrator/UI Automation
acceptance remains open; no native Windows SearchHost or Shell entry-point
control is changed.

PR #81 remains DRAFT/OPEN, Issue #82 remains OPEN, and release state
remains INVALIDATED. There is no production installation or index change.

### External cross-process MSAA WinEvent delivery regression (following iteration)

The previous section documents production `NotifyWinEvent` emission
but explicitly did not prove event delivery to an external listener.
That gap is now covered by the Windows regression script. The normal
`--ui-selftest` remains fully hidden and runs exclusively on the
checked-in synthetic index; the dedicated
`--ui-selftest-winevent-offscreen` flag is accepted **only** with
`--ui-selftest` and a bounded `--ui-selftest-inspect-ms` duration.
After its hidden Win32 regression completes, that opt-in test process
moves its popup to (-30000, -30000) and applies
`SW_SHOWNOACTIVATE`: the HWND has a visible style so WinEvent delivery
is enabled, but is well outside ordinary display coordinates without requesting activation.

The independent Windows PowerShell 5.1 process subscribes using
`SetWinEventHook` with `WINEVENT_OUTOFCONTEXT`, scoped to the
synthetic GUI PID and the standard `EVENT_OBJECT_NAMECHANGE` event.
It pumps its **own** Windows messages, and asserts that the *actual*
LISTBOX HWND reports `OBJID_CLIENT/CHILDID_SELF` name-change callbacks
when changing the selected category from All to Folders and back.
The same MSAA client independently reads the changed and restored
accessible names. Its final repeated All command asserts there is no
third duplicate name-change event. The test additionally verifies
the offscreen coordinates and records foreground state for diagnostics.
On an interactive desktop the window is requested without activation;
headless CI can mark its only GUI window as foreground regardless.
This is not taken as proof of physical keyboard focus or Narrator behavior.

This is a real **cross-process WinEvent delivery** test, not merely
a pure callback predicate or a same-process simulated notification.
It does **not** verify Narrator speech, live UI Automation announcements,
physical keyboard/IME operation, SearchHost/taskbar entry-point parity
or pixel-level Windows Search visual acceptance. No user data, shell
hooks, production service, live index or installed GUI is modified.
PR #81 remains DRAFT/OPEN; Issue #82 OPEN; release status INVALIDATED.

### External UI Automation tree audit and remaining role/name gap

The independent Windows PowerShell 5.1 regression now also loads the
.NET UIAutomationClient/Types assemblies and inspects the same isolated
offscreen popup **from another process**, in addition to MSAA and
cross-process WinEvent checks. It checks the root process, native HWND,
control class identity and minimum descendant coverage, then verifies
the six category/action button HWND names through UIA and their changing
selected-category names after WM_COMMAND.

In the NAVEAX test environment, these UIA button names are correct.
However, `AutomationElement.FromHandle` exposes native EDIT and
LISTBOX as generic `ControlType.Pane` objects, with EDIT's UIA name
equal to the synthetic query text (`SearchTool`) rather than
`Arama sorgusu`, and LISTBOX's UIA name empty rather than the
result-count label. MSAA separately reports the correct names and
list item content. The script records a `correct roles and names=False`
diagnostic for this outstanding discrepancy; passing the basic UIA
HWND/name checks must **not** be interpreted as full UIA support.

Before user-facing accessibility acceptance, investigate the actual
UI Automation providers/proxies and ensure EDIT/LISTBOX name, role,
state, selection and list items are exposed correctly on an interactive
Windows 11 desktop. Narrator speech is still untested. Do not merge or
promote this draft release based only on passing MSAA or partial UIA.
No physical input or installed production index is touched.

### Native Win32 UIA provider baseline: environment versus app

The preceding UIA audit found a real **observation** (Edit and ListBox
appear as `ControlType.Pane` to this external client), but that alone
cannot prove Search Tool is the cause. To separate environment/proxy
behavior from application regression, the new
`.github/scripts/windows-gui-uia-native-baseline.ps1` launches an
**independent** PowerShell process with a synthetic offscreen WinForms
parent, then creates unmodified child windows using the original
`user32!CreateWindowExW` classes `EDIT` and `LISTBOX`.
A second PowerShell process inspects those native child HWNDs through
the same .NET UIAutomationClient, verifying process, class, HWND and
far-offscreen window geometry. Neither process touches Search Tool,
installs UIA providers, injects input or accesses live indexes.

On NAVEAX, even these unmodified Windows standard controls return
`ControlType.Pane` rather than Edit/List. The existing cross-process
Search Tool MSAA/UIA regression now runs that baseline as well and
**fails** if the standard EDIT or LISTBOX is exposed with its expected
native UIA role but Search Tool's corresponding control is not.
If the native Windows baseline is also limited, the script explicitly
reports the provider limitation rather than attributing it to the
application or silently marking complete UIA parity as passed.
Baseline fixture errors, identity failures and onscreen placement fail
the test rather than being ignored. The baseline launches a second GUI
process only **after** the existing Search Tool MSAA/WinEvent callback
and category-state checks have finished. The first combined GitHub CI
run failed the external WinEvent receipt check after invoking the
additional process early; its baseline itself returned valid results.
The isolated ordering keeps event delivery mandatory rather than
suppressing that failure, and was verified in repeated local runs.

Actual Narrator, full UIA patterns, Edit/ListBox names, selection,
accessible list items and physical keyboard/IME still need testing in
an interactive Windows 11 session with a working UIA proxy. This
comparison is a **diagnostic**, not a release acceptance waiver.
PR #81 remains DRAFT/OPEN, Issue #82 OPEN, release state INVALIDATED.

### External MSAA result selection and item-role regression

The Windows PowerShell MSAA test now queries the actual synthetic
LISTBOX's `IAccessible` object from an **independent process**. It
asserts three result children, the native list role (33), and each
list-item role (34), with a nonempty accessible name for each entry.
It also verifies `accSelection` and each child's
`STATE_SYSTEM_SELECTED` bit agree exactly.

The test sends `LB_SETCURSEL` directly to the isolated offscreen
LISTBOX to select its second result and then restores the first one.
After each transition, a fresh external `IAccessible` read asserts
that exactly the intended item is marked selected. No desktop input,
ShellExecute, user file, production index or installed GUI is involved.
This strengthens test evidence for the native MSAA selection path;
it does **not** establish complete UI Automation selection patterns
or actual Narrator speech. Physical accessibility acceptance remains
open and the release gate stays INVALIDATED.

### Cross-process MSAA selection events

The external PowerShell regression now also installs a separate
`SetWinEventHook` observer scoped to the isolated native GUI process,
its real LISTBOX HWND and `OBJID_CLIENT`. Selecting child two through
`LB_SETCURSEL` must deliver `EVENT_OBJECT_SELECTION` (`0x8006`)
with child ID `2`, then selecting child one must deliver the same
event with ID `1`. Both event deliveries are required, ordered, and
checked against the separately verified `IAccessible.accSelection`
and `STATE_SYSTEM_SELECTED` values. A bounded message pump observes
the events from a **different process**, without desktop keyboard or
pointer injection.

The native Windows LISTBOX emits these events itself, so this change
does **not** add duplicate application-level `NotifyWinEvent` calls.
Three consecutive NAVEAX offscreen regression runs passed with
`8006:2,8006:1`. The result is not a physical Narrator, UI Automation
selection-pattern, or live SearchHost accessibility acceptance test.
PR #81 remains DRAFT/OPEN and release state INVALIDATED.
