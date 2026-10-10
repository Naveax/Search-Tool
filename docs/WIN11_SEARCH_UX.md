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

### External MSAA focusability and native keyboard traversal

The isolated offscreen Search Tool now undergoes a cross-process
focusability and traversal audit without injecting keys or changing
foreground focus. Using `IAccessible` from independent Windows PowerShell,
the native EDIT must expose role `ROLE_SYSTEM_TEXT` (42) and the LISTBOX
`ROLE_SYSTEM_LIST` (33); both must set
`STATE_SYSTEM_FOCUSABLE` (0x100000). The native dialog manager is read
through `GetNextDlgTabItem`, verifying both forward and reverse
Tab candidates across EDIT, Tümü, Dosyalar, Klasörler, İçerik,
LISTBOX, Görünüm and Aç. All child HWND identities are checked.

Three consecutive NAVEAX external regressions passed. In the offscreen
session, UI Automation instead reports `ControlType.Pane` and
`IsKeyboardFocusable=False` for Edit and ListBox, consistent with
the independent native Win32 UIA provider baseline's limited role
mapping. This observation does not override correct native MSAA
focusable flags; it also does **not** establish true keyboard-focus
events, real Tab keystroke behavior, Narrator speech or full UIA
acceptance. No focus was stolen from the desktop, no real file was
opened and production indexes remain unchanged. Release status
remains INVALIDATED.

### Native category name-change WinEvents

The owner-drawn native Search category buttons already encode which
category is selected in their MSAA-visible Windows titles (for example,
`Tümü (seçili)`). Previously `update_tab_labels` changed these button
titles without explicitly notifying external accessibility clients.
It now compares each actual button title against the new label and
calls `NotifyWinEvent(EVENT_OBJECT_NAMECHANGE, button HWND,
OBJID_CLIENT, CHILDID_SELF)` only when the name changes **and** the
parent flyout is visible. Repeated selections and hidden synthetic
windows generate no duplicate or offscreen desktop notifications.

The independent PowerShell/Win32 regression subscribes to name-change
events for the native All and Files button HWNDs via `SetWinEventHook`,
drives synthetic category WM_COMMAND messages without input injection,
and requires the exact `all,files,all,files` event order on Files
selection and All restoration. Selecting the already-active All mode
must not emit a fifth event. Existing MSAA button names and UIA
name reads are also verified. Three consecutive isolated NAVEAX
regressions passed.

This verifies delivery to a separate WinEvent listener, not actual
Narrator speech, UIA Toggle/SelectionItem patterns, physical keyboard
navigation, Windows SearchHost integration, or Windows Search visual
acceptance. PR #81 remains DRAFT/OPEN and release state INVALIDATED.

### Re-selecting the active category is a true no-op

A native category chip previously executed `refresh_results` even if
the requested category was already the active mode. For queries that
require expensive content scanning, that could repeat work, change
the elapsed-time/status text and unnecessarily rebuild result rows.
The `WM_COMMAND` handler now returns early for unchanged modes.
Actual category changes and EDIT query changes still refresh results.

The hidden Win32 GUI regression switches to Files, writes a synthetic
status marker, selects Files again using the real `WM_COMMAND` path
and requires the marker, result count, selected row, details path and
accessible selected-category caption to remain unchanged. Switching
back to All continues to refresh normally. The independent offscreen
MSAA WinEvent regression also confirms a repeated selected category
emits no duplicate name-change event.

The change is limited to the draft native Search GUI. No service or
index modifications, desktop keyboard input or physical release
acceptance were performed. Release gate stays INVALIDATED.

### Fail-closed native result row-to-file mapping

The native owner-drawn LISTBOX has `LBS_HASSTRINGS` but **not**
`LBS_SORT`, so each displayed row is inserted in the same order as
its associated `State.results` entry. The former detail and opening
paths fell back to the visual selection index if `LB_GETITEMDATA`
failed. Worse, a wrong-but-in-range item-data index could display and
open a **different result** from the selected row.

Both the detail card and the Open action now share one verified
selection mapping: the selected row must be within range, its
`LB_GETITEMDATA` lookup must succeed, and the mapped index must
match that unsorted visual row. On any discrepancy, the detail
card clears, the Open button is hidden, and `ShellExecuteW` is not
called. Valid mappings continue to work normally. This is an
additional guard on top of filesystem existence/type checks.

The hidden same-process Win32 regression corrupts the synthetic
first result's mapping with a negative value, an out-of-range value
and a valid-but-wrong second-result index. It asserts no stale path
is exposed and opening returns false, then restores the valid mapping
and checks that correct details return. A pure Rust unit test covers
valid and invalid mapping boundaries. No real indexed file or
installed Search Tool application is touched; release status remains
INVALIDATED.

### Verify result row mapping at native ListBox insertion

The preceding fail-closed Open guard now has a matching validation at
the point where result rows are first created. `LB_ADDSTRING` must
insert at the expected unsorted row index; `LB_SETITEMDATA` must
succeed; and reading `LB_GETITEMDATA` must return that exact index.
Only then is the row appended to `State.results` and its icon cached.

If any native insertion/mapping step fails, the application removes
the attempted row, resets the entire ListBox and clears its result
cache, details and actionable controls, rather than exposing an
inconsistent partial list. The status explicitly reports that the
result list could not be constructed safely, distinguishing this from
a valid query with zero matches.

The offscreen Win32 regression intentionally requests a mismatched
expected row index in an isolated empty ListBox, asserts zero rows
remain, then verifies that a correctly mapped row inserts and reads
back successfully. The fixture resets before normal synthetic
searches. This does not induce filesystem changes, physical focus
or changes to installed production indexes. The release acceptance
gate remains INVALIDATED.

### Native result list integrity during keyboard selection

The selected-result guard also checks that the native `LB_GETCOUNT`
exactly matches `State.results.len()`. Keyboard preparation for Down
from the search edit and Enter's Best match selection now rejects an
inconsistent native result count, and Down verifies that the selected
row's `LB_GETITEMDATA` still maps to the expected Rust result before
moving focus. Enter also verifies the selected row mapping before
reporting successful Best match preparation.

The hidden/offscreen Win32 fixture exercises a native extra row, a
missing row, and negative/out-of-range/wrong-but-valid item-data. It
checks that invalid rows cannot be opened, exposed through details, or
used as a keyboard navigation target, while recovered native rows
resume normal selection. These are synthetic safeguards, not physical
keyboard, IME, Narrator, or Windows 11 visual acceptance. The deployed
service/index is unchanged and `package_status=INVALIDATED` remains.

### Hidden native ListBox cannot expose actionable cached results

Even when `State.results` still contains previously verified entries,
`selected_detail_row` now rejects a ListBox whose own `WS_VISIBLE` style
is cleared. The same visibility check prevents Enter from preparing a
Best match while results are hidden. The Open path shares the guarded
selected-result lookup, so it cannot open or report a stale hidden row.

The hidden Win32 fixture first hides the real synthetic ListBox while
its cache still contains results, then verifies that Down and Enter
refuse the hidden list, that the detail path clears, and that Open
returns false without modifying a sentinel status. Restoring ListBox
visibility and its valid selection restores normal detail access. The
fixture does not use SendInput, physical desktop focus, or a production
index. This does not constitute real Windows 11 visual/keyboard or
Narrator acceptance; the release package stays INVALIDATED.

### Avoid duplicate programmatic query refresh

`set_query` previously called `SetWindowTextW` and then unconditionally
called `refresh_results`. Native EDIT controls can also synchronously
send `EN_CHANGE` for this text update, causing a redundant search before
the explicit refresh. `State.programmatic_edit_update` now suppresses
only `EN_CHANGE` during this programmatic text assignment. `set_query`
then performs exactly one explicit `refresh_results` call after the
updated scope and text are in place. Normal EDIT typing and committed
IME changes remain governed by the existing `EN_CHANGE` handler.

The synthetic hidden Win32 regression forces `EN_CHANGE` while this
flag is active and verifies neither status nor result count changes.
It then applies a programmatic query and verifies normal results are
restored. The fixture does not measure end-to-end latency, use a real
index, or establish physical keyboard/IME acceptance. Release status
remains `INVALIDATED`.

### Restore native result visibility before its detail card

After an empty or no-match query, the resident native ListBox is hidden.
When a later query returns results, `selected_detail_row` intentionally
refuses to provide a result while the ListBox still has `WS_VISIBLE`
cleared. The old refresh sequence updated detail controls *before*
showing the ListBox again, so displaying the selected path could depend
on incidental Win32 visibility notifications. `refresh_results` now
restores result-list visibility before updating the selected detail
controls. Empty lists still clear the card, and existing path-based
selection restoration continues to work.

The hidden synthetic Win32 regression explicitly checks that a no-match
to result-populated transition restores the first selected row, its
verified full detail path, and Open-button visibility. Rust workspace,
hidden Win32 and cross-process MSAA testing are used; this is not a
physical Windows 11/Narrator/UIA acceptance claim. The production index,
installed package and release gate are unchanged (`INVALIDATED`).

### Validate native result label alongside row item-data

The selected-result and Open guards already checked native `LB_GETCOUNT`
plus selected `LB_GETITEMDATA`. Those checks alone cannot detect a same-
count ListBox row replacement that retains the expected item-data but
shows another title/type/path. The native result accessible label is now
constructed by a single shared formatter, and production ListBox text
is read with bounded `LB_GETTEXTLEN`/`LB_GETTEXT` before a selected row is
accepted. Failed reads, an excessive label length, invalid UTF-16, or
text differing from the cached `ResultRow` all fail closed. Initial row
insertion also verifies the stored label, not only the row index and
data. This safeguards both the visible/accessible label and the Open
path, without reading any user files for the text comparison.

The hidden Win32 fixture deletes one synthetic native row, inserts a
substituted label into the same slot, restores the correct item-data,
and verifies that details, keyboard selection and Open reject the
mismatch. Refresh must restore a correctly labeled native result. The
missing-file Open preflight fixture separately synchronizes its
synthetic native label to exercise the later filesystem metadata check,
then restores original results before external cross-process MSAA and
WinEvent validation. These tests never invoke ShellExecute on a real
path, SendInput, or a deployed production index; release validation
remains `INVALIDATED` until physical Windows 11, Narrator/UIA and package
acceptance are completed.

### Isolate malformed indexed labels without dropping valid hits

A single indexed record with an embedded NUL or an oversized native
label used to fail ListBox text verification during insertion. The
failure path would clear the entire result list, including unrelated
valid hits. `verified_result_accessible_label` now rejects such entries
before native insertion, so `refresh_results` skips only the malformed
record. The same bound (65,536 UTF-16 code units) is used during native
`LB_GETTEXTLEN`/`LB_GETTEXT` comparison and when accepting cached rows.
A native insertion failure for a well-formed row still fails closed and
clears the partial list rather than exposing inconsistent mappings.

A Rust regression checks valid labels, embedded-NUL names and paths,
and an oversized label, plus recovery to a normal valid label. A hidden
Win32 regression verifies that an intentionally NUL-truncated native
label is rejected and rolled back without leaving an extra row. These
checks are synthetic and do not replace physical Windows 11, UIA,
Narrator, IME or package acceptance; release remains `INVALIDATED`.

### Do not restore selection to a changed file/directory kind

Result refresh previously remembered only the selected full path. If an
indexed object changed from a file to a directory, or vice versa, a later
query could preserve that old selection on the newly typed object at
the same path. Selection restoration now records the selected path and
its `is_directory` flag and restores it only when both match a result.
If the old object is no longer represented, the first available hit
becomes Best match. This remains distinct from the Open preflight,
which still verifies the current filesystem kind immediately before
ShellExecute.

The pure Rust selection regression retains full-path disambiguation of
same-name entries, checks that unchanged file and folder selections
survive, and verifies that a file-to-folder transition does not retain
the previous file selection. Full local Rust/Win32/MSAA testing is not
physical Windows 11 keyboard, visual or Narrator/UIA acceptance. The
production deployment and `package_status=INVALIDATED` are unchanged.

### Clear stale detail text after an invalid Open attempt

The native row mapping and accessible-label guards fail closed if an
existing ListBox row is silently replaced without a selection change.
However, the detail card may still contain the previously selected
path until Win32 emits another notification. When Open rejects an
invalid selection, it now calls `update_detail_controls` before
returning, removing any stale visible detail text and the Open action.
It never falls back to the unverified native row index or calls
ShellExecute for the invalid selection.

The synthetic hidden Win32 test first confirms the old detail text is
still present following an in-place native label replacement with no
selection notification. It then exercises the actual Open handler and
verifies rejection, cleared detail path, hidden Open button and normal
recovery after a fresh query. This remains an offscreen test without
real desktop focus, filesystem opening or physical accessibility
acceptance. The release state stays `INVALIDATED`.

### Roll back rejected keyboard result selections

A corrupt native ListBox item-data or label could cause Down/Enter
selection preparation to return false after `LB_SETCURSEL` had selected
an unusable row. That invalid selected index could remain after the
mapping recovered, so a later Enter from the query would no longer
choose Best match (`LB_GETCURSEL` was already nonnegative). Shared
`reject_native_keyboard_selection` now deselects the failed row and
clears its detail controls. Successful existing selections are left
untouched, and no physical keyboard focus is moved by this helper.

The synthetic hidden Win32 test now injects a broken Best-match
item-data mapping, confirms Enter fails with no selected row or stale
detail path, repairs the mapping, and confirms Enter succeeds on retry.
The Down regression additionally confirms a corrupt selected row is
deselected after rejection. These offscreen checks do not establish
physical Windows 11 keyboard/IME, Narrator or full UIA acceptance; the
production release remains `INVALIDATED`.

### Deselect invalid results on native row-count mismatch

Keyboard Down and Enter previously refused a native ListBox whose
`LB_GETCOUNT` disagreed with the Rust results cache, but the early
return left any previously selected row and its detail text intact.
Both paths now use the existing rejected-selection cleanup for hidden
lists, incorrect native row counts and failed selection writes. That
cleanup explicitly deselects the ListBox row and removes stale detail
text/Open controls before declining the keyboard action. It does not
move the user's desktop focus or attempt ShellExecute.

The offscreen Win32 test appends an unmatched native row without
emitting a selection-change notification, first verifies that the old
selection and detail path are still present, then invokes Down and
verifies the selection is gone, detail path blank and Open hidden.
Enter remains unavailable while the row count is invalid; deleting the
spurious row restores normal keyboard selection. Rust, hidden Win32,
and cross-process MSAA regressions remain synthetic. Physical Windows
11, Narrator/UIA, IME and package acceptance remain outstanding and
`package_status=INVALIDATED` is unchanged.

### Clean up stale preselected rows on Enter

The native Enter preparation path distinguished a new Best-match
selection from an already selected row. If an existing selected
ListBox row silently acquired invalid item-data or accessible text,
Enter returned false without rechecking that preselected row. The
stale selection and detail card could persist until the next
selection change, even though the later Open path rejected it.

Before declining a preselected Enter, the handler now verifies the
selected native row with the same row-count, item-data and label
checks used by Open. Invalid rows are deselected via the shared
keyboard-rejection helper, which also clears stale detail controls.
A legitimate previously selected row is preserved exactly as before.
The hidden Win32 fixture mutates a selected row's item-data without a
selection event, asserts that Enter removes that stale selection and
hides Open, then repairs the mapping and confirms Best-match Enter
can select the row again. These are offscreen checks only; physical
Windows 11 visual, keyboard/IME and Narrator/UIA acceptance are still
required before changing `package_status=INVALIDATED`.

### Reject unresolved dot segments in indexed paths and scopes

The index rebuilds paths from parent chains and the Search flyout
applies a lexical, directory-boundary scope check before showing a hit.
A malformed indexed path such as `C:\Projects\..\Secrets\private.txt`
starts with the Projects folder lexically, but Windows resolves the
`..` component outside the intended directory. `verified_result_path`
now rejects any path containing an entire `.` or `..` component (with
either Windows or mixed separators) before the ListBox and Open path
are populated. `path_is_within_scope` independently rejects dot
components in either the indexed path or the supplied scope.

Unit regressions cover escaped and mixed-separator paths, root-level
traversal, invalid scope segments, and unaffected valid names such as
`.git` and `release..txt`. This is a lexical traversal safeguard, not
a guarantee about Windows junction/reparse-point targets or a claim
of canonical filesystem containment; no actual files are opened by
the test. It does not replace physical Windows 11 acceptance, and
`package_status=INVALIDATED` remains in force.

### Canonical scope check immediately before opening results

Rejecting indexed `.` and `..` segments is insufficient when a path
within an Explorer-scoped search contains a Windows junction or symbolic
link to an external directory. The existing lexical prefix can still
match that path even though the filesystem resolves it outside scope.
The Open preflight now uses `std::fs::canonicalize` for both the selected
path and its requested scope, then re-applies the component-boundary
scope comparison to the resolved targets. If either cannot be resolved,
or the actual target is outside the requested directory, Open fails
closed with a scoped error status. Unscoped search opening behavior is
unchanged. This runs on Open, not on every query hit, avoiding per-result
filesystem I/O during typing.

A Rust regression constructs isolated temporary Windows directories,
verifies valid contained files/folders, missing files, sibling paths and
unscoped paths, then creates a real `mklink /J` junction inside the
scope pointing to a sibling. The lexical path matches but resolved
containment is rejected. The junction and temporary files are removed
after the test. This reduces reparse-point scope escapes but cannot
eliminate a concurrent filesystem swap between verification and
ShellExecute (TOCTOU); it is not an authorization sandbox. Physical
Windows 11 visual, IME/keyboard, Narrator/UIA and package acceptance
remain outstanding, with `package_status=INVALIDATED` unchanged.

### Clear stale Open details on missing or out-of-scope files

Filesystem Open preflight previously refused a deleted or type-changed
indexed file and rejected a scope target resolving outside the requested
folder, but kept the corresponding native selection and Open detail
button. A failed click could therefore leave the same non-actionable
result highlighted and apparently ready to open. Both rejection paths
now retain their user-facing error status and call the shared native
selection cleanup: the selected index is cleared, old detail text is
blanked and the Open control is hidden. The search result list remains
available and a later query can repopulate and select valid results.

The hidden Win32 regression already had a synchronized native label
for an intentionally missing file. It now asserts that the actual Open
handler rejects the file, keeps its error status, deselects the row and
hides stale Open details. A second isolated fixture creates a real
temporary file outside a temporary requested scope, verifies the row
is otherwise valid/openable, invokes the actual Open handler and
checks identical clearing without ShellExecute. Temporary files are
removed and synthetic index state restored before external MSAA
verification. These are synthetic/offscreen safety regressions, not a
physical Windows 11/Narrator/IME acceptance pass. The release remains
`package_status=INVALIDATED`.

### Reject corrupt native mouse/keyboard selection notifications

The Win32 `LBN_SELCHANGE` handler previously updated the detail card
but left an invalid ListBox row selected when item-data or the native
accessible label had silently diverged from its cached Rust result.
The native Windows 11 resident branch now verifies nonnegative
selected rows before honoring the notification; mismatched selections
are deselected through the common cleanup, which also clears stale
details and hides Open. Existing valid selection and non-native theme
behavior are unchanged.

The hidden Win32 regression corrupts a selected row's item-data,
sends the real `WM_COMMAND` `LBN_SELCHANGE` notification, verifies that
the selected index and detail path are cleared and Open hidden, then
restores the mapping to verify selection/detail recovery. MSAA external
events remain PASS. The known UIA provider role mismatch (`Pane`
reported for Edit/ListBox) remains unresolved; these offscreen checks
are not physical Windows 11 keyboard/IME or Narrator/UIA acceptance.
`package_status=INVALIDATED` remains in force.

### Clear corrupt native selection after a rejected double-click

The Open fail-closed path correctly refused a selected row whose native
ListBox accessible label or item-data no longer matched its cached Rust
result. However, when the mismatch was discovered through the actual
`LBN_DBLCLK` handler rather than a keyboard-preparation or selection-
change notification, the detail card was cleared but the bad ListBox
selection remained. The shared native rejection helper is now used by
`open_selected` when its selected result cannot be verified, removing
both the invalid native selection and the old detail/Open contents.

The synthetic Win32 test substitutes an invalid native row label while
preserving the formerly visible detail and selected index, sends the
actual `WM_COMMAND/LBN_DBLCLK` notification, and verifies no selected
index, no detail path, hidden Open, no launch, and successful recovery
after query refresh. The standard UIA baseline **also** reports Pane
for the untouched Windows Edit/ListBox controls in the offscreen test
session, so this release does not claim a Search Tool-specific UIA fix.
Physical Windows 11/Narrator/IME acceptance and package validation
remain outstanding with `package_status=INVALIDATED`.

### Preserve narrower explicit path filters inside Explorer scope

An Explorer-scoped query can also contain a user-specified `path:` or
`in:` filter. Both constraints were checked after reconstructing result
paths, but `apply_scope_filter` always replaced the parsed path filter
with the broader scope directory needle before candidate retrieval.
For example, a query under `C:\Projects` with
`path:"C:\Projects\docs"` fetched broad Projects candidates first;
its bounded candidate list could exclude matching `docs` hits.

When the explicit normalized path begins with the scope's directory
boundary and names a descendant, the filtered index retrieval now
preserves that narrower path term. A generic or unrelated path term
still uses the scope needle for bounded candidate collection, and the
existing post-query validation always enforces both the actual scope
and the user's explicit filter before showing any hit. Tests cover
nested paths, sibling-prefix lookalikes, generic/unrelated filters,
inside/outside paths, and unscoped queries. This improves bounded
search recall but does not guarantee exhaustive hits beyond the
underlying ranked candidate scan and its budgets. It does not alter
native Search popup focus or installed production behavior;
`package_status=INVALIDATED` is unchanged.

### Fix drive-qualified path filtering across indexed volumes

The scoped and explicit `path:` search filters use Windows absolute
paths, such as `C:\Users\Demo`. Previously the multi-volume search
passed that drive-qualified text directly into each `LiveSearchStore`,
whose `reconstruct_path` returns volume-relative paths such as
`Users\Demo\SearchTool Notes.md`. The local filter therefore returned
zero matches even for valid scoped queries. An end-to-end hidden Win32
regression reproduced the failure with both backslash and forward-slash
path spellings before the core fix.

`MultiLiveSearchStore::search_filtered` now strips the drive prefix for
local filtering on the matching volume, skips other drives, and gives
the only eligible volume the full requested result limit. Generic
path substrings continue searching all volumes. At the Windows GUI
query boundary, `path:` and `in:` accept forward, backward and mixed
separators and normalize them to the index convention, without
modifying query free text. The GUI continues to independently enforce
both Explorer scope and the user's explicit path constraint on every
reconstructed result before exposing it to Open.

Regression coverage includes a synthetic C:/D: multi-volume index with
identical names, correct drive-only filtering, unknown-drive rejection
and generic cross-volume matches. Hidden Win32 EDIT/WM_COMMAND now
exercises forward-slash `path:` and `in:` searches for three synthetic
`C:\Users\Demo` results, excludes an unrelated folder and restores
unfiltered query behavior. This fixes a real filtered-search failure,
not exhaustive recall beyond ranked scan limits. Physical Windows 11
visual/keyboard/IME, Narrator/full UIA and package acceptance remain
outstanding; `package_status=INVALIDATED` remains unchanged.

### Normalize separator characters in relative cross-volume path filters

After fixing drive-qualified filters, generic `path:` and `in:`
substrings remained inconsistent: `docs/report.txt` produced no hits
when a volume's indexed reconstructed path was `docs\report.txt`.
Unlike the Windows GUI's explicit path parsing, the shared core
multi-volume API passed generic fragments directly into the underlying
relative-path matcher without normalizing slash separators.

A new core regression builds isolated C: and D: index fixtures with
`docs\report.txt` on both drives, then queries the multi-volume store
using forward-slash `path:` and `in:` fragments, the equivalent
backslash form, and a missing file. The forward-slash case failed
with zero instead of two results before the fix. The shared
`local_path_filter` now normalizes `/` to `\` for generic relative
fragments as well as its existing drive-qualified branch. The same
regression passes after this change, both volumes remain eligible,
and a nonexistent relative path still has zero matches.

This patch leaves search-result opening and all production indexes
untouched. Synthetic offscreen Win32/MSAA checks still pass; physical
Windows 11, IME, Narrator/full UIA and package acceptance remain
outstanding, with `package_status=INVALIDATED` unchanged.

### Anchor drive-qualified filter matches at the volume root

After the previous drive-prefix conversion, an absolute filter like
`path:C:\report.txt` was translated to the relative substring
`report.txt` for the C: index. This admitted the unrelated indexed path
`Other\report.txt`, because the underlying `LiveSearchStore` used a
substring search rather than checking the start of the volume-relative
path. It also risked filling bounded candidate results with wrong-folder
hits. An isolated synthetic regression confirmed the defect: the
absolute root-file query returned two matches instead of one.

The multi-volume dispatcher now passes the relative root prefix only
for drive-qualified filters. The per-volume live candidate filter
checks this prefix on the reconstructed path *before* applying the
result limit, so nested same-name files cannot crowd out the requested
root path. Generic `path:` fragments still use substring matching,
and existing non-filtered search behavior is unchanged. The test
covers a root file and a same-name file in `Other`, independently
selects each qualified path, checks a one-result limit, and confirms
a generic filename filter still returns both. No production index or
installed service is modified. The usual bounded candidate scan may
still limit exhaustive recall. Physical Windows 11 visual, keyboard/
IME, Narrator/full UIA and packaging acceptance remain outstanding;
`package_status=INVALIDATED` is preserved.

### Preserve component boundaries for absolute drive-qualified filters

The previous root-prefix check accepted arbitrary suffixes: an absolute
`path:C:\report.txt` also matched `C:\report.txt-old`, and
`path:C:\Other` could match `C:\Other-old\report.txt`. These are not
the same file or folder, despite sharing their leading characters.
The per-volume rooted matcher now requires the path to equal the
requested relative target, continue with a directory separator,
or follow an explicitly trailing separator in the filter. An empty
relative prefix from `path:C:\` intentionally matches that drive's
whole index. This boundary check takes place inside candidate filtering
before applying the result limit; generic drive-free `path:` fragments
retain the original substring semantics.

The existing synthetic multi-volume root-path regression was extended
with similarly named sibling folders and files. It first failed,
returning two results for the one-file absolute query. After the fix,
root filename, nested file, folder-only path, trailing slash, lowercase
mixed slash form, root-of-drive and a one-result cap are checked. The
unqualified filename query continues to match all four synthetic files.
Local Rust, hidden Win32, external MSAA/WinEvent and release-state
regressions pass. This does not replace the pending physical Windows 11
visual/keyboard/IME, full Narrator/UIA or package acceptance, and
`package_status=INVALIDATED` remains in force.

### Fail closed on indexed paths with missing parent records

Live index path reconstruction previously stopped silently when an
indexed child referred to a parent file ID absent from both the base
index and delta. A record for a nested `report.txt` with a missing
parent could consequently be reconstructed as `report.txt`; the
multi-volume layer would then qualify it as `C:\report.txt`, falsely
making it look like a root-level file. The filtered search path also
substituted the hit name if reconstruction returned any error.

`LiveSearchStore::reconstruct_path` now returns `NotFound` on a
missing parent record instead of inventing a truncated root path.
When a path-based filter is active, `search_filtered` omits hits whose
paths cannot be reconstructed instead of falling back to filename-only
matching. Unfiltered ranked name search itself is unchanged, but the
GUI already requires verified reconstruction before presenting an
Open-ready row.

A regression builds an isolated C: index containing a root record
and an otherwise matching `report.txt` whose parent ID does not exist.
It demonstrates that the name remains indexed but that the fabricated
root path is rejected and `path:C:\report.txt` returns no hit. The test
first failed with the previous permissive reconstruction, then passed
after the change. This specifically closes the missing-parent case.
No production index is read or rewritten. Release status stays
`package_status=INVALIDATED` pending physical Windows 11 visual/IME,
Narrator/full UIA and packaging acceptance.

### Reject cyclic and depth-truncated indexed parent chains

An indexed file can still refer to a cyclic parent chain, such as
`report.txt -> loop-a -> loop-b -> loop-a`. Previously reconstruction
continued until the caller's depth budget was exhausted and returned
an apparently complete, deeply repeated path. Even a valid file under
`docs` could be returned as a misleading partial path when a caller
allowed too few ancestors to reach the index root.

Path reconstruction now tracks visited file IDs and rejects a repeated
ancestor with `InvalidData`. After consuming the requested depth, it
requires a verified terminal parent (`0` or a self-parent root record)
rather than accepting an unfinished chain. Existing missing-parent
`NotFound` checks remain. Path-based filtered searches already omit
failed reconstructions, and the GUI's Open-ready rows still require a
verified reconstructed path.

A synthetic C: fixture combines a cycle of two directory IDs plus a
child `report.txt` and a separate healthy `docs\\good.txt` under a
self-parent root record. The regression was RED before the fix because
cycle reconstruction returned a string. After the fix, the cyclic hit
and its absolute path-filtered search are rejected, a healthy path
needs enough depth to reach the root, and its normal path-filtered
query still succeeds. Offscreen Win32/MSAA and Rust suite PASS do not
establish physical Windows 11/Narrator/IME or release acceptance;
`package_status=INVALIDATED` remains unchanged.

### Reject self-parent files and non-directory ancestors

The prior cycle guard accepted a self-parent terminal record as a root
without checking the record's type. An ordinary indexed file with its
own file ID as parent could therefore look like a legitimate volume
root, and a different file could incorrectly be treated as a parent
directory when its index record was a regular file. An isolated C:
fixture reproduced both malformed cases: `self-file.txt` and an
invalid `ordinary-file\\nested.txt` chain were accepted by path
reconstruction before the correction.

`PathNode` now retains the directory flag from either the base index
or live delta. Only a directory can terminate a chain using its own
file ID as parent. All ancestors traversed in reconstruction must be
directories, and malformed paths return `InvalidData`; path-filtered
searches drop such candidates. Valid files under a real directory
root still reconstruct and filter normally. The regression was RED
before each guard and GREEN afterward.

The committed offscreen Win32 fixture originally labeled its synthetic
`Users` parent record as a regular file even though it held the Demo
directory. After adding the strict ancestor check the hidden GUI test
correctly failed; that test-only index record now has its directory
flag set. All five synthetic fixture files remain isolated; real user
indexes and installed Search Tool binaries were not changed. Hidden
Win32, external MSAA, Rust and release-state tests subsequently pass.
Full physical Windows 11/IME, Narrator/UIA and package acceptance remain
pending with `package_status=INVALIDATED`.

### Apply the same fail-closed path checks to direct index readers

`SearchStore::reconstruct_path`, also used by the CLI's direct-index
readers, previously stopped silently when an indexed parent ID was
missing, when a loop appeared, or when the requested depth ended. It
also allowed a regular file to be an ancestor or a self-parent root,
returning plausible but incomplete relative paths instead of errors.
The live multi-volume reader had already gained stronger checks, so
direct index consumers could disagree with the native GUI.

Direct reconstruction now tracks visited file IDs, requires every
ancestor to be a directory, permits a self-parent terminal only for a
directory, rejects missing parents with `NotFound`, and rejects cyclic
or depth-truncated chains with `InvalidData`. Correctly terminated
relative paths continue to reconstruct unchanged. A new synthetic
base-index test covers the orphan, cycle, regular-file ancestor,
self-parent file, insufficient depth, and healthy
`C:\docs\good.txt` cases. The test initially failed for `orphan.txt`
before the fix, then passed after the change. Workspace unit tests,
Clippy, release build, hidden Win32, external MSAA and release state
checks remain passing. This is code-only work against synthetic
fixtures, not a validation of the installed application or physical
Windows 11/IME/Narrator. `package_status=INVALIDATED` remains in force.

### Reconcile the omitted NTFS root directory during initial indexing

After direct-index reconstruction was made fail-closed, Windows CI #354
failed its isolated VHD integration at metadata collection: the same
18 enumerated MFT entries that previously yielded five indexed files
now produced zero. A diagnostic-only CI run enumerated the disposable
volume's exact IDs and directory flags; it confirmed that normal
children referenced the NTFS root directory's full file reference
number (low 48-bit MFT record number 5), but that the root itself was
absent from `FSCTL_ENUM_USN_DATA` output. The index therefore contained
valid child directories without the parent record needed for verified
path reconstruction. Other missing system parents remained unrelated.

On initial MFT indexing, the Windows platform layer now records only
observed root-directory parent references and root IDs, without
allocating a HashSet of every volume record. If exactly one full root
FRN is referenced, and the enumeration did not include the root, it
adds a self-parent directory anchor with that **exact** reference ID
and an empty name before sealing the index. This restores valid
volume-relative paths. Conflicting root sequences fail closed rather
than inventing an identity. If the root is already in the enumeration,
nothing is synthesized. Ordinary orphan and invalid-file-parent
records remain rejected by SearchStore and LiveSearchStore.

A Windows-platform synthetic regression covers the missing root, a
root already present, no root references, conflicting generations, and
successful direct reconstruction of `projects\\node.exe` through the
new anchor. Local Rust, hidden Win32, MSAA and release-state tests pass;
CI #356 subsequently completed SUCCESS on both Windows and Ubuntu,
including the isolated Windows NTFS/USN integration. Metadata indexing
recovered to five indexed files. Installed production indexes and
services were not touched;
`package_status=INVALIDATED` remains in force pending physical UX,
Narrator/IME and package acceptance.

### Prevent detail-card overlap on short native Search flyouts

The resident/native Windows 11-style two-column layout previously
remained enabled with only 190 DPI-scaled pixels of detail height.
However, the path label ends 242 logical pixels below its column top,
while the Open button occupies the bottom 60 pixels. On a short
flyout this produced overlapping action and path controls.

`native_result_columns` now requires enough vertical room for the path,
a 16 logical-pixel gap, the Open button and its 20-pixel bottom inset
(318 logical pixels at 96 DPI). Below that threshold it chooses the
single-column results list instead of rendering overlapping details.
An isolated 96/120-DPI regression first reproduced the old two-column
layout incorrectly remaining active at short heights, and then passed
with the adjusted threshold. The hidden Win32 fixture also resizes an
invisible window across the threshold: the result list stays accessible,
the Open button and cached detail path disappear on collapse, and the
selected details return upon restoration without rerunning the query.
It never moves physical keyboard focus or displays the test window.

Local workspace tests, release build, hidden Win32, external MSAA and
release-state checks pass. Real Windows 11 visual comparison, physical
keyboard/IME, Narrator/full UIA and packaging acceptance are still
outstanding. `package_status=INVALIDATED` is unchanged.

### Protect keyboard traversal and details during compact/DPI transitions

Following the detail-card overlap correction, the offscreen Win32
regression also checks the actual Windows dialog Tab chain at compact
height: ListBox -> appearance -> query, skipping the hidden Open button.
Returning to full height must reinsert the visible Open action after
appearance without dropping the selected result or accessible path.
This uses GetNextDlgTabItem, not physical keyboard input or activation.

The same invisible fixture now dispatches WM_DPICHANGED with suggested
bounds for a 125% scaling transition and restores the original DPI.
At both sizes the native two-column geometry, native ListBox contents,
selected path, search text, visible detail action and Tab order must
stay consistent. The hidden HWND is never made foreground. Existing
code already handled this test without further production logic changes;
this addition specifically guards future layout/DPI regressions, not
a claim of physical monitor/IME/keyboard acceptance. CI #357 for the
prior overlap fix finished SUCCESS on Windows and Ubuntu. All local
workspace, release, hidden GUI, MSAA and release-state checks pass;
`package_status=INVALIDATED` remains pending physical acceptance.

### Rescale detail-card fonts and EDIT padding on WM_DPICHANGED

A further offscreen Windows GUI audit found that `apply_dpi` created
fresh DPI-scaled fonts for the query, results list and navigation
buttons but failed to reassign them to five detail-card controls
(header, name, type, path and Open). The query EDIT's initial 14 logical
pixel left/right margin was also left at its old physical width when
the monitor scaling changed. A hidden Win32 regression read each native
control's `WM_GETFONT` and the actual `EM_GETMARGINS` after a synthetic
125% DPI transition, and first failed against the old implementation.

`apply_dpi` now sends WM_SETFONT to all five detail controls together
with the other UI controls *before* retiring the previous DPI font, and
updates the EDIT's left and right margins based on the new scale. The
hidden regression passes after this fix, including font and margin
verification when returning to the original DPI. Selected results,
Tab traversal, and accessible detail text remain unchanged. Unit,
release, hidden Win32, external MSAA and release-state checks PASS.
The preceding `0328a90` CI #358 finished SUCCESS on Windows and Ubuntu;
CI #359 for `af29330` subsequently finished SUCCESS on Windows and
Ubuntu. Production installed files and user indexes remain untouched. Physical
Windows 11 visual, keyboard/IME, Narrator/full UIA and package review
are still pending; `package_status=INVALIDATED` remains unchanged.

### Remove offscreen ListBox from Tab traversal at extreme small heights

When the resident native flyout was constrained to a height unable to
fit a single results row below its search/category/status controls,
`resize_controls` still gave the ListBox a 1-pixel rectangle below the
client edge and retained its WS_VISIBLE style. Keyboard Tab traversal
and assistive tools could consequently expose an unusable offscreen
result target. An isolated hidden HWND regression at 170 logical pixels
confirmed this, failing before the code change with three results still
in the ListBox but a visible-style, keyboard-reachable row container.

The native layout now requires enough client height for one complete
DPI-scaled result row before showing the ListBox. It preserves cached
result contents and selection when clipping the list, hides the native
result control from keyboard traversal while it cannot fit, and shows
it again on expanding. Detail state is refreshed **after** visibility
transitions, so the previous selected file path and Open action return
without requery. The regression verifies results retained during the
small-height transition, the hidden ListBox and skipped Tab target,
then restores the full window and verifies the same selection and path.
This touches only the isolated Search Tool native flyout, never Windows
SearchHost or installed production indexes. Local Rust, hidden Win32,
external MSAA and release-state regressions pass. Full physical visual,
keyboard/IME, Narrator/UIA and package acceptance are outstanding;
`package_status=INVALIDATED` is preserved.

### Reflow native category chips into two rows on narrow work areas

The resident Windows 11-style flyout formerly forced four category
chips into one row even when a 360 logical-pixel client could give each
chip only about 62 pixels. Labels were cramped, and status/results
still assumed one navigation row.

A hidden Win32 regression resizes the synthetic flyout to 360 logical
pixels, checks native category HWND coordinates for two nonoverlapping
rows and at least 80 logical pixels of width, and requires status to
follow the second row. All three indexed results and native Tab order
must survive. At the original width, the original one-row navigation,
selected result and full detail path must return without requery.
This regression was RED before the change and GREEN afterward.

The native layout now switches to two rows below the per-chip 80-pixel
space threshold; wider windows retain their existing single-row
layout. CI #360 for `fcf76b7` finished SUCCESS on Windows and Ubuntu.
Local Rust, release, hidden Win32, external MSAA and release-state
checks pass. Production application, service and user index are not
touched. Physical visual parity, keyboard/IME, Narrator/full UIA and
package acceptance remain pending; `package_status=INVALIDATED` stays.

### Preserve clipped results-list visibility across query refresh

The extreme short-height `WM_SIZE` guard hid a result ListBox when even
one row could not fit. However, a subsequent typed query could enter
`refresh_results -> update_native_result_visibility`, where the
`has_results` branch unconditionally showed that ListBox again. The
result could remain outside the window while reappearing in Tab and
accessibility traversal. A hidden Win32 fixture reproduces this by
shrinking the synthetic flyout to 170 logical pixels, checking the list
is hidden, then delivering the real EDIT/EN_CHANGE query notification
without changing its size. The test first failed with `query refresh
reopened offscreen ListBox in too-short flyout` and then passed after
the fix, retaining three cached results and a hidden, unselectable path
card until the window is enlarged.

A single pure `native_result_list_has_room` geometry calculation now
drives both resize and query visibility updates. It accounts for
96/120-DPI row height, compact two-row category navigation, status
height and margins. A unit test checks short versus usable heights at
normal, narrow and 125%-scaled flyout sizes. Query refresh reuses the
same geometry instead of blindly showing any nonempty ListBox; resize
restores the list and selection when space returns. Local workspace
(87 core, 10 Windows platform, 59 GUI), Clippy, release build, hidden
Win32, external MSAA and release-state checks PASS. Previous commit
`2a3f0ab` CI #361 completed SUCCESS on Windows and Ubuntu. Production
services and user indexes remain unchanged, with physical Win11
visual/IME/Narrator/UIA and package acceptance still pending.
`package_status=INVALIDATED` remains unchanged.

### Prevent category labels collapsing in extremely narrow work areas

The previous two-row native Search category layout ensured useful chip
widths around 360 logical pixels, but at 210 pixels it still divided
available category space between two chips per row, reducing each to
roughly 57 pixels. A synthetic hidden Win32 flyout regression resized
to 210 pixels and required four individually stacked category buttons
with at least 80 logical pixels per chip, distinct nonoverlapping
rows, status text below the last category, preserved list contents,
and a clean return to the wide single-row layout. Before the fix the
regression failed (`extra-narrow flyout did not stack readable category
buttons`); after the layout change it passed.

The new shared `native_category_columns` threshold chooses 4, 2 or 1
category columns based on minimum 80-DPI-scaled-pixel chip widths and
8-pixel gaps. Both control placement and the shared
`native_result_list_has_room` geometry use this decision, keeping the
status/ListBox placement consistent even when navigation takes four
rows. A unit regression verifies column breakpoints and minimum
usable result-row heights at 96 and 120 DPI. Workspace tests (87 core,
10 platform, 59 GUI), Clippy, release build, hidden Win32, external
MSAA and release-state checks PASS. The prior `0a7d508` CI #362
completed SUCCESS on Windows and Ubuntu. Production deployment and
foreground desktop remain untouched, while physical Windows 11
visual/IME/Narrator/full UIA and packaging acceptance are pending;
`package_status=INVALIDATED` remains unchanged.

### Keep clipped category actions out of keyboard traversal

After the 4/2/1-column layout change, a 210 logical-pixel-wide native
flyout could put its last two category buttons and status field below
the client edge when simultaneously reduced to 170 logical pixels in
height. Despite clipping, the child HWNDs retained `WS_VISIBLE`, so
native Tab traversal could still target inaccessible categories. A
hidden synthetic Win32 regression first reproduced this condition,
expecting only the two fully visible category controls and the
Appearance button in the Tab chain; the test was RED before the fix.

`resize_controls` now checks the full button rectangle against the
native client height, hides category and Appearance buttons when their
entire click area cannot fit, and hides the status text when clipped.
When a currently focused category becomes clipped on an actually
visible application window, focus returns to the query field; hidden
test windows never move physical keyboard focus. The existing ListBox
minimum-row safeguard remains intact. As the window grows, native
category, appearance and status controls are restored automatically;
the non-native layout branch also re-shows controls previously hidden
by the native layout. The hidden regression verifies the narrow/short
case, Tab order, a requery while clipped, and restoration of category
visibility, status, selected search result and accessible detail path.
All those checks are GREEN following the change.

The previous commit `055b033` completed CI #363 SUCCESS on Windows and
Ubuntu. Local workspace tests (87 core, 10 Windows platform, 59 GUI),
Clippy, release build, offscreen Win32, external MSAA/WinEvent and
release-state checks PASS. No installed Search Tool package, user index,
service, production shell or physical desktop was touched. Physical
Windows 11 visual parity, IME/keyboard, Narrator/full UIA and final
package acceptance remain outstanding, so `package_status=INVALIDATED`
is unchanged.
