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
`SearchButton` and records only its numeric bounds if visible. The
`AutomationId` string alone is insufficient proof: the probe now examines
all matching UIA peers, requires a real `ControlType.Button` owned by
`explorer.exe`, and verifies a `TaskbarFrame` / `TaskbarFrameRepeater`
ancestor in the UIA hierarchy. An unrelated application can expose the
same AutomationId and must not be mistaken for the native taskbar entry
point. These provenance guards were reproduced as RED selftests before
being corrected; a real read-only Windows 11 run confirmed a trusted
Explorer-owned button at `(697,1032)` with `44x48` bounds on a
1920x1080 desktop; a subsequent independent read-only sample reported
`(719,1032)` with the same button size. The position is live UI
metadata, not a fixed Search Tool layout constant. This does not toggle or click the button;
unsupported/hidden/unverified peers are reported as `found=false` rather
than treated as a failed SystemIndex probe. Windows 11 builds may use
different taskbar automation hierarchies, so lack of this identification
is diagnostic only, not proof that Windows Search is absent.

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

### Restore classic-theme results after clipping and theme changes

The native flyout correctly hid its results ListBox when too little
vertical space remained for one complete row. Switching from that
clipped Native layout to a classic theme (such as Graphite), then
expanding the window, did not necessarily show the cached result list:
classic `resize_controls` restored geometry but never the ListBox's
previously cleared WS_VISIBLE state. A new hidden Win32 test reproduced
this without saving theme settings or changing foreground focus and
failed before the correction (`switching from clipped native to classic
theme lost cached result list`).

The classic query refresh path also left an empty ListBox visible and
keyboard reachable after clearing an otherwise valid search. A separate
hidden Win32 check reproduced this (`empty classic search exposed a
focusable empty results list`). Both failures are now resolved using
theme-specific geometry with a common `result_list_has_room` dispatch.
The native layout retains its prior 1/2/4-row navigation calculation;
classic layout independently requires space for a complete result row
below its own title, subtitle, query, tabs and status. Query refresh
updates ListBox visibility in **all** themes, including empty and
error paths, and theme changes/resizes also recalculate visibility.
Native automatic Best Match selection remains deliberately distinct
from classic search's unselected-on-requery behavior.

The hidden fixture now checks clipped Native -> classic transition,
classic requery while too short, restoration when expanded, empty-query
Tab exclusion, results returning after new input and safe Down selection
when returning to Native. A 96/120/144-DPI unit test checks classic
minimum row space. Before the changes the regression was RED; it is
GREEN after the fix. Local workspace tests (87 core, 10 Windows
platform, **60 GUI**), Clippy, release build, offscreen Win32, external
MSAA/WinEvent and release-state checks PASS. CI #365 for `de56eec`
subsequently finished SUCCESS on Windows and Ubuntu. Production
app/index/service, user input, deployment and release remain untouched.
Physical Win11 visual, IME, Narrator/full UIA and final package
acceptance remain pending; `package_status=INVALIDATED` remains in force.

### Restore query focus before hiding a focused results list

The result ListBox could be hidden while holding keyboard focus when a
visible native Search flyout was resized below its complete-row limit.
Query refresh previously returned focus to EDIT only for an empty result
set; a nonempty but clipped list and the native WM_SIZE path lacked an
explicit focus restoration step. This risked leaving focus on a control
that had become inaccessible. The classic resize path had a separate
focus guard, making behavior inconsistent across themes.

All ListBox visibility changes now use `set_result_list_visible`,
called by native resizing, classic resizing, and query refresh. Before
SW_HIDE, it restores focus to the query EDIT **only** when the parent
window is visible and the ListBox holds thread keyboard focus. Hidden
or offscreen synthetic windows never take physical desktop focus;
cached results return when a full row fits again. Existing four-case
`should_restore_query_focus` unit tests and the hidden Win32 fixture
cover those policies and all resize/requery/theme transitions. Local
workspace (87 core, 10 platform, 60 GUI), Clippy, release build,
external MSAA/WinEvent and release-state checks PASS. This is a code
safeguard, **not** physical keyboard/IME/Narrator/UIA acceptance.
Production files and service remain unchanged, and
`package_status=INVALIDATED` remains in force.

### Responsive 4/2/1 category navigation in classic themes

Graphite and the other classic Search Tool themes previously placed
four 94-logical-pixel category buttons in a single row, even with only
360 logical pixels of client width. The latter buttons could extend
past the client edge while retaining their Win32 visible and keyboard
Tab-target states. Unlike the native layout, the classic result-list
height calculation did not account for wrapping category rows.

A hidden Win32 regression first reproduced the bug on a synthetic
Graphite flyout: at 360px client width, four filters failed to wrap to
two rows. It then passed after adding `classic_category_columns` and
`classic_category_rows_height`, shared by the classic control placement
and `classic_result_list_has_room` geometry. Four filters now occupy 4
columns when at least 94 logical pixels per chip fit, 2 columns on
moderately narrow windows, or 1 column on extra-narrow windows. Status
and ListBox coordinates follow the actual navigation row count.

The expanded offscreen fixture verifies 360px two-row navigation, a
360x190 client that hides the lower two filter buttons and ListBox
from native Tab traversal, a requery while clipped that does not
re-expose those controls, and 210px single-column category stacking.
Restoring the window returns the previous category/Tab layout and the
cached synthetic results. A 96/120/144-DPI pure unit regression checks
the three column thresholds and minimum row space for 360px and 210px
clients. Local tests (87 core, 10 Windows platform, 60 GUI), Clippy,
release build, hidden Win32, external MSAA/WinEvent and release-state
checks PASS; CI #367 for `3687187` subsequently completed SUCCESS on
Windows and Ubuntu. Physical Windows 11 visual parity, keyboard/IME,
Narrator/full UIA and package acceptance are still pending. Production
application/index/service, foreground desktop and installed release
remain unchanged; `package_status=INVALIDATED` remains in force.

### Stack classic heading and appearance action on very narrow screens

At a 210-logical-pixel client width, the classic flyout's theme button
retained its 108-pixel width beside the heading, leaving only about 50
pixels for the title and subtitle. The last responsive navigation fix
had wrapped the filter buttons but not this header. A real hidden Win32
fixture now requires a readable >=130-logical-pixel title/subtitle and
the appearance button below them, with the query control placed below
the appearance button. This regression first failed with
`extra-narrow classic flyout clipped heading beside appearance button`.

Classic layout now uses `classic_stacked_header_offset` at constrained
widths. The title and subtitle use the full content width, the
appearance button takes its own full-width row below them, and the
query, filters, status and results list are shifted downward by the
corresponding 8px gap plus 34px button height. The same offset feeds
`classic_result_list_has_room`, so a requery cannot re-show a clipped
ListBox that no longer has a complete row of space. Wider screens
retain the previous side-by-side title/appearance layout. Unit checks
at 96, 120 and 144 DPI verify the breakpoint and updated minimum-row
height. The previously RED hidden Win32 regression is now GREEN.
Local workspace (87 core, 10 Windows platform, 60 GUI), Clippy,
release build, offscreen Win32, external MSAA and release-state checks
PASS. Physical Win11 visual/IME/Narrator/full UIA and package
acceptance are still pending, and `package_status=INVALIDATED` remains
in force. Production binaries, indexes and services were not changed.

### Hide fully clipped query and appearance actions in tiny flyouts

At extremely small classic window heights, the earlier responsive
header still left keyboard Tab targets for controls entirely or partly
outside the client rectangle. For a synthetic 210x90 logical-pixel
classic client, the full-width Appearance action (ending at 100px)
and the EDIT search field (ending around 164px) retained `WS_VISIBLE`
even though neither was fully usable. A new hidden Win32 regression
first failed with `very-short classic window retained clipped
Appearance or query Tab targets`.

The classic layout now measures the complete native child bounds before
showing each control. At 210x90 both actions are hidden; at 210x120 the
Appearance action fits but the query EDIT remains hidden. In a real
visible window, if the current focused child becomes clipped, focus is
moved to the remaining usable action, or the parent window when both
are inaccessible. The hidden synthetic fixture never changes the
physical desktop focus. The native layout now also re-evaluates the
EDIT visibility, so a later classic-to-Native theme switch at 120px
restores the higher native search box correctly. All cached query text
and results remain unchanged.

Extended the hidden HWND regression to verify 90px/120px classic
transitions, classic -> Native -> classic roundtrip, EDIT/Appearance
visibility and query text preservation, and full restoration at the
original window dimensions. The regression is GREEN after the change.
Local workspace (87 core, 10 Windows platform, 60 GUI), Clippy,
release build, hidden Win32, external MSAA/WinEvent, and release-state
checks PASS. Previous commit `0b26b1f` CI #368 completed SUCCESS on
Windows and Ubuntu. Physical Windows 11 keyboard/IME, Narrator/full
UIA, visual parity and package acceptance are still pending. The
installed app, user index and service remain unchanged;
`package_status=INVALIDATED` stays in force.

### Never transfer native navigation focus to a clipped query field

The native category and Appearance resize branches formerly used
`SetFocus(EDIT)` unconditionally when their own HWND fell below the
client edge. At a client height shorter than the native search field's
68-logical-pixel bottom boundary, the EDIT is also hidden; therefore
an actual focused category or Appearance button clipped by a workspace
resize could transfer focus to another inaccessible child. This is a
code-level focus hazard, not something the hidden fixture can validate
using physical keyboard focus.

A shared `native_clipped_focus_target` now chooses the query EDIT only
when the complete native input control fits the client; otherwise it
chooses the parent flyout. The category and Appearance focus-handoff
paths use the same rule. A new unit test verifies both possible focus
targets, and the hidden Win32 regression exercises native client
heights of 60 logical pixels (EDIT/category/Appearance/result list all
hidden) and 72 logical pixels (EDIT restored, navigation still hidden),
then returns to the normal-size flyout, preserving its search text and
results. The offscreen HWND never becomes the foreground window or
takes real keyboard focus.

Local verification PASS: workspace Rust suites (87 core, 10 Windows
platform, **61 GUI** and other suites), Clippy, release build,
synthetic Win32 layout/Tab regression, external MSAA/WinEvent,
release-state check/selftest and `git diff --check`. The previous
commit `7df8a7a` CI #369 completed SUCCESS on Windows and Ubuntu.
Physical Win11 visual parity, real keyboard/IME, Narrator/full UIA and
final package acceptance remain pending. The installed package, user
index and production service were not modified, and
`package_status=INVALIDATED` remains in force.

### Use a visible focus fallback for every clipped search control

The previous native focus safety change covered category/Appearance
controls, but two other focus-handoff paths still tried `SetFocus(EDIT)`
unconditionally: a clipped category in the classic theme and a focused
results ListBox hidden by `WM_SIZE` or an empty/short query. If the
query EDIT had already been hidden because the client was too short,
those paths could transfer focus to an inaccessible child. This is a
code-level focus issue, not physical focus validation.

`native_clipped_focus_target` has been generalized to
`clipped_focus_target`, returning the search EDIT only when it is
visible and the parent HWND otherwise. Native navigation, classic
navigation and result-list hiding now share the fallback rule. The
results-list handler reads the EDIT's actual `WS_VISIBLE` style before
moving focus; the other branches use their current calculated
minimum complete-control-height condition. All focus changes remain
conditional on the real popup being visible and the hidden synthetic
Win32 fixture never activates foreground input.

The renamed unit regression verifies both possible focus targets;
existing hidden real-Win32 resize/requery/theme tests remain GREEN.
Local `cargo fmt`, workspace tests (87 core, 10 Windows platform, 61
GUI), Clippy, release build, hidden Win32, cross-process MSAA/WinEvent
and release-state tests PASS. CI #370 for `ce6d489` completed SUCCESS
on Windows and Ubuntu. Production binaries, service, user indexes,
physical keyboard and installed package are unchanged. Physical Win11
visual parity, IME/keyboard, Narrator/full UIA, and final package
acceptance are still outstanding. `package_status=INVALIDATED` remains
in force.

### Return focus safely when the selected detail Open action disappears

The native two-column results view has a keyboard-focusable `Aç` (Open)
button in its selected-result detail card. Resizing to a one-column
flyout, refreshing results so the selected row disappears, or changing
away from the native theme hides that detail action via
`update_detail_controls`. Previously it issued `ShowWindow(SW_HIDE)`
without checking whether the hidden button currently held keyboard
focus; the code could leave focus tied to an inaccessible action.

The guarded focus handoff now runs before hiding the detail controls.
Only when the flyout parent is actually visible and the Open button
has thread keyboard focus does it transfer focus to another reachable
control: the results ListBox when visible, the query EDIT when the
ListBox is hidden but the query remains visible, and the parent HWND
when both children are hidden. An added pure unit regression verifies
all four visibility combinations. The existing real hidden-Win32
fixture checks detail-card visibility, accessible text clearing and
native Tab traversal on 470px compact and 170px tiny flyout layouts.
The synthetic fixture never takes physical desktop focus, so this is
a tested code-level safeguard rather than physical keyboard or UIA
focus acceptance.

Local `cargo fmt`, workspace tests (87 search-core, 10 Windows
platform, **62 GUI** and the remaining suites), Clippy, release build,
hidden Win32, external MSAA/WinEvent and release-state checks PASS.
CI #371 from prior commit `b5f7e10` requires final Windows job status
verification. Production binaries, index, service, installed package
and desktop/IME remain untouched. Physical Windows 11 visual parity,
keyboard/IME, Narrator/full UI Automation and final package acceptance
remain pending; `package_status=INVALIDATED` stays in force.

### Reject mismatched owner-drawn result-row identities

The Windows owner-drawn ListBox `WM_DRAWITEM` notification supplies
both a physical `item_id` and a separate `item_data` field. The previous
`draw_result_row` renderer selected a cached search-result row by
`item_data` alone without checking that it matched the actual ListBox
slot being drawn. A stale or corrupted mapping could therefore paint
another file's name and path in the wrong row, even though the detail
card and ShellExecute path already independently reject bad mappings.

Added `verified_draw_result_index` requiring the Win32 ListBox `item_id`
to equal `item_data` and both to reference an existing cached result.
The renderer now clears the drawing rectangle **before** rejecting an
invalid row so outdated file pixels are not retained. No row is drawn
for negative/sentinel IDs, mismatched but in-range mappings, out of
range indexes or an empty results cache.

A new eight-case GUI unit regression initially failed RED for item 0
mapped to item-data 1 (`Some(1)` instead of `None`), then passed GREEN
after the identity validation. The existing hidden real-Win32 result
insertion/selection/path and native/classic resize fixture remained
GREEN. `cargo fmt`, workspace suites (87 core, 10 Windows platform,
**63 GUI**), Clippy, release build, offscreen Win32, external
MSAA/WinEvent and release-state check/selftest all PASS. The previous
`465a066` CI #372 finished SUCCESS for Windows and Ubuntu. This patch
still requires its own CI. Production app/index/service, foreground
keyboard/IME, installed package and release were untouched. Physical
Windows 11 visual comparison, keyboard/IME, Narrator/full UI Automation
and final package acceptance remain pending; `package_status=INVALIDATED`
remains in force.

### Require native accessibility-label identity before painting search rows

The previous owner-draw safeguard checked that a Win32 ListBox slot's
`item_id` equals its stored `item_data`. It did not check the **actual
native ListBox text** against the cached file description. A same-count
label substitution at a correctly numbered slot therefore still
painted the cached filename/path for a native row whose accessibility
label described a different result. Opening a file was already safely
blocked by `selected_detail_row`, but paint and accessibility could
disagree about the same result.

`verified_draw_result_row` now requires a verified `item_id`/`item_data`
pair, the native ListBox row count to match the Rust result cache, and
the complete Win32 `LB_GETTEXT` accessibility name/type/path to match
the expected bounded UTF-16 label before any file row is painted. On
failure, `draw_result_row` clears the draw rectangle and paints no
cached name, path or icon. The checks use the existing fail-closed
helpers shared with keyboard selection and ShellExecute; normal valid
results are still rendered.

Extended the hidden real-Win32 fixture with a same-slot, same-count
native label substitution: initial RED failure
`substituted native row fixture did not invalidate visible rendering`,
then GREEN with the new text-identity check. It additionally asserts
valid first/second rows can be drawn after a refresh and a spurious
extra native ListBox row disables even otherwise valid row drawing.
Local `cargo fmt`, workspace tests (87 core, 10 Windows platform,
63 GUI), Clippy, release build, offscreen Win32, MSAA/WinEvent and
release-state check/selftest all PASS. Previous CI #373 for `f5b9a45`
completed SUCCESS on Windows and Ubuntu. Current commit's CI requires
separate confirmation. Production package, index, service and desktop
were not touched; physical Win11 visual parity, keyboard/IME,
Narrator/full UIA and final packaging acceptance remain pending.
`package_status=INVALIDATED` remains in force.

### Ignore stale clicks from controls hidden by responsive layout

Native and classic flyouts now hide clipped category chips, the result
list and selected-result Open action. Although hidden HWNDs are removed
from keyboard Tab traversal, the existing `WM_COMMAND` handler still
accepted a queued or synthetic `BN_CLICKED` from those HWNDs after the
resize. A hidden Folders chip could silently change the active search
category and requery cached results; a hidden `Aç` button could invoke
the Open handler, revoke a valid selection and change status despite
its action no longer being available to the user.

The `BN_CLICKED` dispatch now checks the actual source child HWND's
`WS_VISIBLE` style before processing category, detail Open and
Appearance-button actions. Hidden actions are ignored; visible category
buttons continue to update mode and results normally. This guard
applies equally to native and classic themes. It does not attempt to
validate the real foreground keyboard/IME and does not alter the
production UI.

Extended the hidden real-Win32 fixture to send stale `WM_COMMAND`
notifications to a clipped native Folders chip (210x170 client), a
clipped classic category chip (360x190), and the hidden detail Open
button in a 470px one-column native flyout. The Open fixture requires
a nonexistent synthetic result path *before* sending the message so
no file or app can be launched. Each check first reproduced a RED
failure (`hidden category chip accepted a stale BN_CLICKED command` or
`hidden Open action accepted a stale BN_CLICKED command`), then GREEN
after guarding the dispatcher. Search mode, cached result count,
selection and status remain unchanged in these cases.

Local workspace tests (87 core, 10 Windows platform, 63 GUI), Clippy,
release build, offscreen Win32, external MSAA/WinEvent, release-state
check/selftest and formatting PASS. The prior CI #374 for `c38092d`
completed SUCCESS on Windows and Ubuntu. This commit's CI must be
verified separately. Physical Windows 11 visual parity, actual
keyboard/IME, Narrator/full UIA and final release acceptance remain
pending; installed app, service and index were not modified, and
`package_status=INVALIDATED` is unchanged.

### Ignore stale ListBox notifications after responsive layout hides results

`WM_SIZE` hides the result ListBox when the flyout cannot fit one full
row. Previously, an already-queued `LBN_SELCHANGE` message from that
hidden HWND still reached `WM_COMMAND`. In native mode the handler
rejected the hidden ListBox and cleared its previously valid cached
Best match; a stale `LBN_DBLCLK` could likewise reach `open_selected`
and revoke the selection. This defeated the requirement that clipped
results preserve their selection for normal-size restoration.

Both notification branches now check the ListBox HWND's `WS_VISIBLE`
style first. Hidden list messages are ignored, while visible lists
retain existing integrity validation. A hidden real-Win32 regression
sends both notifications at native 170-logical-pixel client height
with three synthetic results, asserting selection, status and cache
stay unchanged. Before the fix it failed RED with `hidden ListBox
processed a stale selection or double-click notification`; after the
fix the regression is GREEN. The same behavior was tested in a clipped
classic Graphite layout, and normal-size restoration preserved its
cached results and selection. The selected fixture path is confirmed
absent on disk; no file is opened and no desktop input is affected.

Local `cargo fmt`, workspace tests (87 core, 10 Windows platform,
63 GUI and other suites), Clippy, release build, hidden Win32,
external MSAA/WinEvent, release-state checks/selftest and diff checks
PASS. Prior CI #375 for `cf7c394` finished SUCCESS on both Windows
and Ubuntu. The current commit requires separate CI verification.
Offscreen full UIA still reports Edit/List as Pane, so UIA acceptance,
physical Windows 11 visual comparison, keyboard/IME, Narrator and final
package acceptance remain pending. Production app/index/service,
installed package and user desktop remain untouched;
`package_status=INVALIDATED` remains in force.

### Preserve verified selected files through a temporarily hidden ListBox refresh

A second selection regression remained after stale hidden ListBox
notifications were blocked. `refresh_results` previously used
`selected_detail_row` to identify the file selected before requery.
That accessor deliberately rejects a hidden native ListBox to prevent
clipped results from becoming actionable. However, this visibility
requirement also discarded a correct row 1 selection on an identical
query when the flyout was too short to show the ListBox, resetting the
selection to Best match row 0.

Extracted `verified_cached_selected_row`: it validates the ListBox
row count, selected index/item-data equality, and full native
accessible name/type/path against the cached row without requiring
`WS_VISIBLE`. It is used by `refresh_results` to preserve valid
selection identity even while the list is hidden. The existing
`selected_detail_row` still checks `WS_VISIBLE` before returning the
verified row for the detail card or ShellExecute. Thus hidden results
never become actionable and their detail/path controls stay blank.

The hidden real-Win32 fixture now selects row 1 while the native
flyout has only 170 logical pixels of height, then refreshes the same
query. It initially failed RED with `clipped native query refresh
changed the previously selected file`; it is GREEN after the fix.
The fixture also deliberately corrupts hidden row 1's native item-data
mapping before a second refresh: the invalid selection is rejected,
the verified first Best match is chosen, and details stay hidden.

Local `cargo fmt`, workspace tests (87 core, 10 Windows platform,
63 GUI and remaining suites), Clippy, release build, hidden Win32,
external MSAA/WinEvent, release-state check/selftest and diff checks
PASS. Prior CI #376 for `9667077` finished SUCCESS on Windows and
Ubuntu. This commit requires independent CI. Physical Windows 11
visual comparison, actual keyboard/IME, Narrator/full UI Automation
and final package acceptance remain pending. UIA offscreen baseline
still reports Edit/List as Pane; `package_status=INVALIDATED` remains
in force. Production application, service, user index, installed
package and foreground desktop were not modified.

### Hide unreadably narrow results even with sufficient vertical room

ListBox visibility previously checked whether at least one full result
row fit vertically but not whether the list was wide enough to read a
file name. A tall flyout resized to 130 logical pixels leaves only 82
logical pixels of native result width (90 in classic) while previously
retaining a tiny, unusable ListBox in keyboard Tab traversal. A new
hidden real-Win32 regression first failed RED with
`very narrow native flyout retained an unreadable result Tab target`.

Both `native_result_list_has_room` and `classic_result_list_has_room`
now require at least **120 DPI-scaled logical pixels** of result width
in addition to their minimum complete-row height. Below that limit,
the ListBox is hidden and excluded from Tab, while cached query and
results remain intact. Restoring width re-shows the ListBox and
preserves the native selected result. Hidden Win32 fixture checks 130
vs 210 logical-pixel widths for native and classic layouts. A new GUI
unit test covers the precise readable-width threshold and one pixel
below it at 96/120/144 DPI. The previously RED regression is GREEN.

Local `cargo fmt`, full workspace suites (87 core, 10 Windows platform,
**64 GUI** and others), Clippy, release build, hidden Win32 regression,
external MSAA/WinEvent, release-state check/selftest and diff checks
PASS. Previous CI #377 for `0f5be9e` completed SUCCESS Windows/Ubuntu;
this commit requires its own CI verification. Full UIA still reports
offscreen Edit/List as Pane, so physical Win11 visual comparison,
actual keyboard/IME, Narrator/full UI Automation and final package
acceptance remain pending. No installed package, production
app/index/service or foreground desktop was changed;
`package_status=INVALIDATED` remains in force.

### Do not revoke a cached selection on Down or Enter while results are clipped

Native results are deliberately hidden when responsive width/height
cannot display a usable row. The previous keyboard helpers
`prepare_query_down_selection` and `prepare_search_enter_selection`
returned `reject_native_keyboard_selection` for a hidden ListBox,
which cleared the verified cached selected file even though it was
only temporarily not actionable. Likewise `open_selected` called the
same rejection path when Enter came from the query field or Shell
bridge and the ListBox was clipped. A user could therefore lose a
previously selected row without the results changing or becoming
corrupt simply by pressing Down/Enter with a short flyout.

These three paths now distinguish an invisible ListBox from an
*invalid visible* ListBox. If hidden, they return `false` without
changing its cached selection, status or detail controls and never
attempt ShellExecute or move focus to the clipped result. If visible,
the original count/item-data/accessibility-label, filesystem and scope
validations still reject corrupt or disappeared results, clearing an
invalid selection as before.

The existing real hidden Win32 regression now explicitly selects the
second cached synthetic result while the native flyout is clipped,
then calls the Down/Enter preparation and Open handlers. The fixture
was RED with `clipped native Down or Enter revoked a verified cached
selection`, then GREEN after the fix. It also sends a synthetic
`WM_SHELL_BRIDGE_KEY` Enter message through the real Shell bridge
handler and checks selection/status/detail text still match the
original. Synthetic paths are first verified nonexistent; the HWND
remains offscreen and never receives physical keyboard input.

Local `cargo fmt`, full workspace tests (87 core, 10 Windows platform,
64 GUI and other suites), Clippy, release build, hidden Win32
regression, external MSAA/WinEvent and release-state check/selftest
all PASS. The previous `e571e30` CI #378 completed SUCCESS for Windows
and Ubuntu. New commit CI requires its own confirmation. Full UIA
still reports Edit/List as Pane in offscreen tests; physical Win11
visual comparison, actual keyboard/IME, Narrator/full UIA and final
package acceptance remain pending. No production install/index/service,
user desktop, package or release was changed;
`package_status=INVALIDATED` remains in force.

### Preserve readable file names in compact result cards

The prior result ListBox visibility gate allowed at least 120 logical
pixels of content width. However, the existing owner-drawn card always
reserved 36px for an icon and 86px for a type badge, leaving only a
few pixels of title width in a 120px ListBox. This made its row visible
and keyboard-accessible but effectively unreadable.

Extracted `result_row_text_geometry` as the single source of layout
bounds used by the actual Win32 owner-draw renderer. Cards narrower
than 260 DPI-scaled logical pixels use a compact presentation: hide
the purely visual file icon/type badge and give the filename and path
the complete area between 12px insets. At or above 260px, the old
icon, type badge, selected background, focus rectangle and normal title
geometry remain. Native ListBox accessibility labels still include the
complete file name, type and full path, regardless of visual width.

Added a pure geometry regression for 96/120/144 DPI. Before the fix it
failed RED with `icons/badges consume compact filename space`; after
the fix it is GREEN and verifies at least 80 logical pixels of file
name width for a 112px compact card, plus preservation of the normal
320px card decorations. Existing real offscreen Win32 popup regression
and owner-drawn result identity checks remain GREEN.

Local checks PASS: `cargo fmt`, full workspace tests (87 core, 10
Windows platform, **65 GUI** and other suites), Clippy, release build,
hidden Win32, external MSAA/WinEvent, release-state check/selftest and
diff checks. The previous CI #379 for `e6bd8c8` completed SUCCESS on
Windows and Ubuntu. This change still requires its own CI acceptance.
Offscreen full UI Automation Edit/List roles remain Pane, so physical
Windows 11 visual review, keyboard/IME, Narrator/full UIA and package
acceptance remain pending. Production app/index/service, desktop and
installed package were not modified; `package_status=INVALIDATED`.

### Exact-head Windows 11 Search visual comparison gate (offline, read-only)

`scripts/windows11-search-parity-audit.ps1` now compares sanitized,
**approximate** native SearchHost geometry from
`docs/evidence/windows11-search-reference-20261008.json` with the
previous two-column Search Tool candidate evidence. It reports the
signed (candidate minus native) X/Y/width/height deltas in physical
pixels. The stored native bounds are *manually approximated* from a
real 3440x1440 screenshot at 96 DPI; these deltas are diagnostic only,
not pixel-perfect proof. The previous candidate record is from commit
`899658b4` and describes a synthetic fixture. Against the current
`12af423` HEAD its measured differences are x=-2, y=+5,
width=+4, height=-5 px, but the audit returns **BLOCKED** with
`acceptance=NOT_ACCEPTED` for stale/synthetic and non-equivalent
evidence. It does not open the Search UI or use desktop input.

Run `powershell.exe -NoProfile -File scripts/windows11-search-parity-audit.ps1`
for the current pair, or supply `-CandidatePath <JSON>` for a later,
manual physical capture. A new candidate JSON uses `source_sha` (exact
current 40-hex commit), `capture_kind=PHYSICAL_WINDOWS11_DESKTOP`,
`physical_dpi=96`, `native_search_untouched=true`, matching
`desktop_physical_pixels` width/height, and
`search_tool_flyout.rect` with integer x/y/width/height. These are
*claims to audit*, not self-authenticating acceptance evidence. Even
matching current physical measurements yield at most **PARTIAL**, since
the original SearchHost rectangle is approximate and keyboard/IME,
Narrator/UIA, ranking, entry-point ownership and visual screenshots
remain unaccepted. The script **never returns parity PASS**.

`-SelfTest` checks legacy evidence remains blocked, signed geometry
deltas, simulated current-head physical evidence remains PARTIAL,
stale-head rejection, DPI mismatch and native Search ownership
provenance. CI runs the self-test under Windows PowerShell 5.1 and
PowerShell 7 and checks stored evidence cannot promote itself to
acceptance. This is a fail-closed *evidence/CI gate*, not a completed
physical desktop comparison or permission to release.
`package_status=INVALIDATED` stays in force; installed production
binary, index, service and user desktop remain unchanged.

### Compare Search Tool's offscreen UIA roles with untouched Win32 controls

The cross-process MSAA/WinEvent GUI regression already launches a
separate untouched Win32 EDIT/LISTBOX baseline after its hooks finish.
`.github/scripts/windows-gui-uia-compare-native.ps1` reuses that same
baseline output and reports both native and Search Tool's UIA Edit/List
roles in a deterministic JSON diagnostic without launching a redundant
second control process. The existing external MSAA checks still have to
PASS. If standard native Win32 controls advertise proper Edit/List UIA
roles on a given host but Search Tool fails to do so, the relative role
regression FAILS CI. Matching a limited native test-session baseline is
classified `MATCHED_LIMITED_NATIVE_BASELINE`, **never UIA acceptance**.
Both native and Search Tool controls currently report `ControlType.Pane`
in the same offscreen Windows 11 build 22631 session; that shared
limitation does not demonstrate physical keyboard focus, Narrator
functionality or full UIA provider quality.

Nine synthetic selftest cases ensure limited baseline parity stays
`DIAGNOSTIC_ONLY/NOT_ACCEPTED`, proper standard roles still require
physical checks, and a regression against standard native controls is
flagged as `FAIL`. UIA role observations must be unique and limited to
Edit/Pane for the query control or List/Pane for results. Duplicate,
conflicting, missing or unexpected role evidence now fails closed instead
of silently using the first observation or diagnosing invalid controls
as equivalent providers. The collected native baseline summary must also
appear exactly once. The Windows CI accessibility step runs the selftest
and this real offscreen cross-process comparison, without any user
desktop input. Physical Windows 11 keyboard/IME, UIA, Narrator and
visible SearchHost visual parity remain outstanding.
`package_status=INVALIDATED` is unchanged.

### Exclude unreadably narrow query EDIT controls from keyboard traversal

Both responsive layouts already hid unreadably narrow results. However,
when a tall flyout shrank to 130 logical pixels wide, the native and
classic query `Edit` HWNDs stayed `WS_VISIBLE` solely because their
vertical position fitted, despite their input area being under 120
logical pixels. The hidden real-Win32 regression first failed RED on
`very narrow native flyout retained an unreadable query Tab target`.

The shared `query_edit_has_room` function now requires at least 120
DPI-scaled logical pixels of usable input **interior width**, excluding
the theme's search border, as well as the pre-existing height check.
Both Native and Graphite layouts hide a query field below this limit;
when the flyout is visible, they transfer focus to a reachable control
or the parent instead of leaving focus on the clipped EDIT. Expanding
the width restores the query HWND and its Tab stop; the text buffer is
never cleared by this visibility change. Hidden Win32 regression covers
the 130px clipped and restored native/classic layouts. A new GUI pure
unit test checks both border widths at 96, 120 and 144 DPI and the
exact one-pixel-below threshold. This is a keyboard *layout* safeguard,
not evidence that physical focus/IME or Narrator acceptance passed.

Production app/index/service and foreground desktop remain untouched.
`package_status=INVALIDATED` remains in force.

### Exclude unreadably narrow category chips from Tab traversal

A tall flyout narrowed to 130 logical pixels already hid its query
Edit and result ListBox, but category buttons were still shown whenever
their **vertical** bounds fit. On Native layout they could be only
~42 pixels wide; Graphite's single-column buttons could be ~90 pixels,
less than their existing readable label width. The buttons remained
Tab targets and could trigger invisible-result filter changes. The
hidden Win32 regression first failed RED with `very narrow native
flyout retained unreadable category Tab targets`.

The shared `category_chip_has_room` predicate now enforces a minimum
DPI-scaled 64-pixel width for compact Native pills and 94 pixels for
Graphite category buttons, in addition to their existing vertical
bounds. The WM_SIZE/DPI layout hides narrower HWNDs and moves focus
from a clipped chip to the visible Edit or parent, never a hidden
input. A normal-width restore makes all four categories visible again.
The real hidden Win32 fixture covers Native and Graphite width changes
and sends a stale synthetic `BN_CLICKED` to the hidden Folders/Content
chip, asserting it cannot change the active filter or cached results.
A pure test checks exact-width and one-pixel-below thresholds at 96,
120 and 144 DPI. RED became GREEN with the width guard.

Local `cargo fmt`, workspace tests (87 core, 10 Windows platform,
**67 GUI** and remaining suites), Clippy, release build, hidden Win32,
relative native UIA/MSAA/WinEvent regression, release-state check and
release-state selftest all PASS. The previous `723012f` CI #384
finished SUCCESS on Windows and Ubuntu; this patch requires its own
CI. UIA remains `DIAGNOSTIC_ONLY/NOT_ACCEPTED` (both untouched native
controls and Search Tool exposed `ControlType.Pane` in the offscreen
provider baseline). Physical Windows 11 visual, keyboard/IME,
Narrator/UIA and final packaging acceptance remain outstanding.
Production install, service/index, user desktop and SearchHost were not
modified; `package_status=INVALIDATED` stays in force.

### Keep Appearance actions out of Tab when their width collapses

The earlier responsive-layout fixes hid unreadably narrow query EDIT,
ListBox and category-chip controls but still exposed an undersized
Appearance button. A 130 logical-pixel tall classic window leaves only
90 logical pixels of usable header width for its **text-labelled**
Appearance action, yet the button retained its visible HWND and Tab
stop. In the extreme native case, the ellipsis button could shrink
below its 32px icon hit area while remaining keyboard reachable.

The `appearance_button_has_room` visibility gate now enforces at least
94 DPI-scaled logical pixels for the classic text-labelled control and
32 for the native compact icon. If clipped, WM_SIZE transfers focus to
a reachable Edit or the parent and hides the Appearance HWND; its
already-present `WM_COMMAND` visibility guard rejects stale clicks.
Restoring width restores the original Appearance action, without
changing theme preference or cached results.

Extended the real offscreen Win32 regression at the classic 130px
width and the native 60px width to verify the undersized Appearance
control is excluded from Tab and restored when the flyout grows. The
classic regression was RED before the fix:
`very narrow classic flyout retained an unreadable Appearance Tab target`,
then GREEN after applying the visibility rule. An added pure unit
regression checks the 32px and 94px exact thresholds and one pixel
below each threshold at 96/120/144 DPI.

Local `cargo fmt`, workspace tests (87 core, 10 Windows platform,
**68 GUI** plus remaining suites), Clippy, release build, hidden Win32,
external MSAA/WinEvent and release-state check/selftest all PASS.
Current commit requires independent CI. Full UIA still reports Edit and
ListBox as Pane in the offscreen baseline and physical Windows 11 visual,
keyboard/IME, Narrator and package acceptance remain pending. No
production application, index, service, desktop input or SearchHost was
changed; `package_status=INVALIDATED` remains in force.


### Read-only native SearchHost live geometry inventory

Added `scripts/windows11-search-live-window-audit.ps1` for the pending P0 physical Windows 11 measurements. The script only enumerates SearchHost/SearchApp-owned top-level HWND metadata from the current session and reads DWM extended-frame physical bounds, class name and DPI. It does **not** activate Search, manipulate focus, use keyboard/mouse input, capture screenshots, read window titles, or touch indexing and installed applications.

Hidden, DWM-cloaked, wrong-session, non-Search or implausible candidate windows are rejected. A `WS_VISIBLE` flag is **not** proof that a window actually overlaps a monitor: offscreen test HWNDs can have normal-sized DWM frames at (-30000,-30000). The inventory reads the virtual desktop bounding box with `GetSystemMetrics` and the actual individual monitor rectangles using read-only `EnumDisplayMonitors`. At least half the candidate frame area must overlap both the virtual desktop **and the union of real monitor rectangles**, while legitimate negative X positions are allowed. The monitor-union coverage calculation slices intersecting rectangles by X coordinate and merges overlapping Y intervals; this avoids counting mirrored displays twice and correctly includes valid windows spanning several adjacent physical monitors. An absent monitor inventory is fail-closed, so a window in the empty gap below a shorter monitor cannot become a valid Search flyout candidate. A stale HWND with zero `GetDpiForWindow` is also rejected. Mixed-DPI coordinate virtualization and unusual monitor configurations may still be classified BLOCKED pending physical inspection, not automatically accepted.

If no qualified candidate is found, the result is **BLOCKED / NOT_ACCEPTED**. Even with a qualified visible candidate, process ownership alone cannot confirm the native Search flyout, so the result is at most **PARTIAL / NOT_ACCEPTED**. Windows 10 builds are excluded. Run `powershell.exe -NoProfile -File scripts/windows11-search-live-window-audit.ps1` or use `-OutputPath <JSON>` for sanitized metadata. An **18-check** `-SelfTest` exercises fail-closed and geometry cases under Windows PowerShell 5.1 and PowerShell 7 in CI, including wholly offscreen, one-pixel overlap, an empty gap between actual monitor rectangles, missing monitor enumeration, legitimate negative multi-monitor coordinates, true multi-monitor spanning frames, mirrored-monitor overlap without double-counting, and unknown window DPI.

The read-only probe on development Windows 11 build 22631 found no visible candidate and correctly returned BLOCKED. Physical visual/IME/Narrator/UIA acceptance and SearchHost interaction parity remain pending. No production installation, user desktop, SearchHost ownership, index or service was changed; `package_status=INVALIDATED` remains in force.

### Match Windows 11 OS build when comparing native Search visual evidence

The offline `windows11-search-parity-audit.ps1` previously compared candidate
and reference resolution, DPI and source SHA without checking Windows OS
build. Different Windows 11 builds can have different SearchHost geometry
and should not be labeled comparable just because the monitor size matches.
The sanitized native reference is Windows 11 Pro **build 22631**.

The candidate JSON now additionally requires `windows_build=22631`, matching
the exact build parsed from the reference's `platform` field. Missing or
malformed candidate OS build, an unknown reference build, and mismatches
(e.g. candidate 26100 against native 22631) each produce a distinct
fail-closed blocker and `result=BLOCKED`, `acceptance=NOT_ACCEPTED`.
Diagnostic output includes `reference_windows_build` and
`candidate_windows_build`. Before the fix, the new regression was RED:
`different Windows 11 build cannot use earlier native geometry`.
With the check added, PowerShell 5.1/7 selftests are GREEN (nine assertions).
The old synthetic record still remains BLOCKED. Even same-build physical
measurement can produce at most PARTIAL until genuine keyboard/IME,
Narrator/UIA, SearchHost identity, screenshots and other release gates pass.
No installed production Search Tool, index, service, desktop, SearchHost
window, or package was changed; `package_status=INVALIDATED` persists.

### Reject offscreen candidate rectangles from physical visual parity evidence

The offline Search parity audit previously checked source SHA, screen
resolution, DPI, Windows build and candidate capture labels but did
**not** establish that the candidate Search Tool frame intersected
its claimed physical desktop. A future candidate with plausible
metadata and a DWM-like rectangle at `(-30000,-30000)` could therefore
be classified `PARTIAL` even though the window was entirely offscreen.
A local synthetic probe reproduced this incorrect result before the
change. It never launched a real GUI or captured desktop pixels.

`Test-SufficientDesktopOverlap` now compares the candidate frame to
the matching reference desktop rectangle using exact signed physical
coordinates and decimal-area arithmetic. It requires **at least 50%**
of the candidate window area to lie inside the declared desktop.
Reference and candidate must share the same virtual desktop origin;
missing origins both mean `(0,0)`, while explicit negative X/Y origins
are supported for legitimate multi-monitor arrangements. Absent,
invalid, different-origin or offscreen candidates fail closed with a
separate blocker. This is only a *bounding-box* diagnostic, not proof
that a frame intersects each actual monitor in a multi-monitor union.
The separate read-only live SearchHost auditor performs monitor-union
checks; physical screenshot comparison is still outstanding.

With this fix the `-SelfTest` suite grew from nine to 15 assertions,
covering fully offscreen frames, one-pixel overlap, the exact 50%
threshold, a valid negative-origin desktop, mismatched origins and
malformed origin metadata. Before the fix, a same-build/same-commit
candidate whose rectangle was wholly outside the desktop yielded
`PARTIAL`; after the fix it is `BLOCKED / NOT_ACCEPTED`. PowerShell
5.1 and 7 selftests PASS. No input can return an actual visual `PASS`:
legitimate-looking measurements remain at most `PARTIAL`, and
`package_status=INVALIDATED` remains in force. The installed application,
index, service, SearchHost and foreground desktop remain untouched.

### Require real monitor-union coverage in offline visual evidence

The offline Search visual parity auditor's virtual-desktop rectangle
can enclose areas where there is **no physical monitor**, such as the
space below a shorter left-hand screen. A synthetically supplied
same-build, same-commit candidate HWND at `x=-1400, y=1130,
600x250` within a `(-1920,0)..(3440,1440)` desktop was incorrectly
reported as `PARTIAL` even though it fell entirely in this gap. The
new regression reproduced that outcome as RED.

A future **non-legacy** candidate JSON must include a
`monitor_rectangles` array of measured physical rectangles, e.g.
`[{"x":-1920,"y":0,"width":1920,"height":1080},
{"x":0,"y":0,"width":3440,"height":1440}]`. The auditor checks
each monitor's geometry against the recorded virtual desktop, then
requires at least half the candidate frame area to overlap the
**union** of real monitors. Intersection X-slices and merged Y-ranges
avoid double-counting mirrored/overlapping monitors and allow valid
frames that span several side-by-side displays. Missing, malformed,
outside-desktop, or unsupported monitor evidence fails closed.

The extended PowerShell 5.1 / 7 `-SelfTest` suite now has **20
assertions**: it includes a real-monitor gap (RED then GREEN), missing
monitor inventory, mirrored displays without double-counting, valid
triple-monitor spanning frames and invalid monitors extending beyond
the declared desktop. The previously RED case now returns `BLOCKED`.
Even geometrically plausible and current-head evidence produces at
most `PARTIAL / NOT_ACCEPTED`: monitor rectangles are source-provided
claims, not independently verified screenshot or UIA acceptance.
Existing saved synthetic visual evidence remains `BLOCKED`, and the
installed app, indexes, SearchHost, service and desktop were not touched.
`package_status=INVALIDATED` remains unchanged.

### Validate live monitor inventory against the virtual desktop

The read-only `scripts/windows11-search-live-window-audit.ps1` formerly
accepted a monitor rectangle extending beyond the recorded virtual
desktop when a SearchHost window intersected both areas sufficiently.
A new synthetic test first reproduced the bug (RED:
`monitor inventory outside virtual desktop is inconsistent`). Every
monitor must now have positive dimensions and be entirely within the
recorded virtual desktop; absent, invalid or more than 16 monitors
fail closed. The test is GREEN, with 21 PowerShell 5.1/7 checks.

The live report also includes `virtual_desktop` with only sanitized
Win32 X/Y/width/height and monitor rectangles, without window titles,
indexed file paths, query text or screenshots. The metadata records
`GetSystemMetrics/EnumDisplayMonitors` as its source and carries
`dpi_awareness=NOT_INDEPENDENTLY_VERIFIED`; it is *not* proof of precise
physical visual parity. An inconsistent monitor inventory is marked
BLOCKED and is never exposed as verified display geometry.

A new read-only measurement on the developer Windows 11 build 22631
session returned one 1920x1080 monitor and `BLOCKED / NOT_ACCEPTED`:
no visible, uncloaked native SearchHost flyout was available. This is
not equivalent to the archived 3440x1440 reference. No SearchHost
activation, keyboard, desktop screenshot or installed Search Tool
service/index mutation occurred. Exact-head visual parity,
keyboard/IME, Narrator/full UIA and final packaging acceptance are
still outstanding; `package_status=INVALIDATED` remains in force.

### Reject incomplete display inventories and fractional physical pixel geometry

Both the offline visual parity checker and the live, read-only SearchHost
inventory previously allowed a **subset** of monitors to account for the
reported virtual desktop. When a candidate window intersected the listed
monitor, contradictory metadata could still be classified `PARTIAL` even
though the full monitor inventory did not reproduce the virtual desktop's
minimum and maximum X/Y extents. A new RED regression reproduced that
false qualification in both tools. In addition to checking that every
monitor fits inside the virtual desktop, the complete set must now have
bounding extents **exactly equal** to the recorded virtual desktop; gaps
between monitors remain legal and are still excluded from window overlap.

The offline parity checker also accepted a `1200.5`-pixel virtual desktop
width with a correspondingly rounded monitor width, permitting fabricated
fractional physical pixels to yield `PARTIAL`. A separate RED regression
confirmed it. Desktop origins and dimensions must now use integer physical
pixel coordinates, while missing optional origins still mean zero for
older evidence. A further RED regression showed that PowerShell's
`[int]` cast rounded a fractional `96.4` DPI measurement to the expected
96 DPI, incorrectly allowing `PARTIAL` visual provenance. The DPI gate
now requires the exact measured 96 value, without integer rounding.
The PowerShell 5.1 and 7 suites are GREEN with 23 offline parity
assertions and 23 live geometry checks. Archived legacy evidence
and the latest live read-only probe both remain `BLOCKED/NOT_ACCEPTED`.
These repairs strengthen evidence validation but do not establish that
the supplied measurements are authentic, or complete keyboard, IME,
Narrator, UIA, screenshots, SearchHost integration or release acceptance.
The installed production application, service, index, SearchHost and
foreground desktop remain unchanged; `package_status=INVALIDATED`.

### Release temporary topmost z-order when the resident flyout loses focus

The resident Search popup is raised using `HWND_TOPMOST` when an optional
bridge overlay opens. Its `WM_ACTIVATE/WA_INACTIVE` dismissal path formerly
called only `ShowWindow(SW_HIDE)`, leaving the hidden window's topmost
extended style set; other dismissal paths correctly removed topmost via
`hide_bridge_overlay`. A new real Win32 hidden-fixture regression applies
`HWND_TOPMOST` to the synthetic invisible window with `SWP_NOACTIVATE`,
delivers `WA_INACTIVE` directly, and checks both continued invisibility
and removal of the `WS_EX_TOPMOST` style. The initial RED result was
`resident popup dismissal retained topmost z-order`. Routing inactive
resident dismissal through the existing shared cleanup path yields GREEN.
No foreground changes, injected input, screenshots, installed program,
SearchHost or production index/service modifications were required.
This fixes a z-order state leak, not physical visual, IME or Narrator
acceptance; `package_status=INVALIDATED` stays unchanged.

### Ignore queued Open and Shell-bridge input after resident dismissal

A native child button or ListBox can retain the `WS_VISIBLE` style even
when the containing Search flyout has been hidden. The previous Open and
ListBox double-click `WM_COMMAND` paths checked only the child style,
allowing stale queued notifications to reach `open_selected` after popup
closure. A newly added hidden Win32 regression sent a real button-click
message to an invisible parent with a visible-style child and observed
RED: the otherwise valid synthetic selection was unexpectedly cleared.
Both actionable paths now require `IsWindowVisible(parent)` in addition
to their existing visible-child checks. The test covers both Open and
ListBox double-click; separate corruption tests continue to call the
shared `open_selected` preflight directly, without pretending that an
invisible control received a real user click.

The asynchronous Shell bridge had a related stale-input problem:
`WM_SHELL_BRIDGE_CHAR` could modify the hidden query after closing the
popup. A second RED Win32 test reproduced the mutation using a posted-style
synthetic character. The CHAR and KEY handlers now ignore commands while
the flyout is not actually visible; the distinct BEGIN handler still owns
activation. All cases are GREEN in the hidden synthetic Win32 test.
No keyboard events, input hooks, production search results, foreground
window, SearchHost, service or installed package were changed. Actual
on-screen focus, IME, Narrator/UIA and visual parity remain unverified;
`package_status=INVALIDATED` is preserved.

### Clear delayed Appearance hover after resident popup dismissal

The native Appearance button sends `WM_THEME_BUTTON_HOT` to the parent.
A delayed hover message could previously mark an invisible flyout hot,
and dismissal did not reset the cached highlight if `WM_MOUSELEAVE`
was missed. Both cases were exercised on the isolated synthetic Win32
window without moving the user's pointer: a hidden-parent hover update
first failed RED; the corrected handler ignores invisible hover, and
the shared resident dismissal clears existing hot state. Both hidden
window assertions are now GREEN. The Appearance menu command also
requires an actually visible parent, not merely the persistent
`WS_VISIBLE` child style; a queued click after dismissal must not show
an unwanted menu over the user's desktop. No physical menu is opened
in the negative test because that would interfere with the active user
session. Physical pointer and visual acceptance remain unverified;
`package_status=INVALIDATED` remains unchanged.

### Clean up the resident popup after a successful result launch

Unlike Escape and focus loss, the successful `ShellExecuteW` path used to
call only `ShowWindow(SW_HIDE)` and clear the intent. That left the
previously acquired `HWND_TOPMOST` state and optional Shell-bridge active
flag unchanged, even though the flyout was hidden and the result had
opened successfully. A hidden, isolated Win32 regression recreates a
temporarily topmost resident HWND and active bridge flag, then invokes
only the post-success cleanup (never a real file launch). It first
failed RED: `successful resident Open retained topmost or active
keyboard bridge`. After routing this action through the same
`hide_bridge_overlay` cleanup used by Escape/focus loss, the regression
is GREEN. Temporary topmost, pending bridge scope, active-keyboard
routing, invisible parent state and stale intent/hover state now follow
the same dismissal path. The offscreen test does not establish that a
physical ShellExecute launch, keyboard hook or focus change works on a
real desktop. No production files, user programs, Windows Search index,
SearchHost or installed service were touched, and the package remains
`INVALIDATED` until full physical acceptance.

### Reject cancelled asynchronous Shell bridge BEGIN requests

The optional keyboard bridge queues `WM_SHELL_BRIDGE_BEGIN` after marking
its request active. If interception was cancelled before the UI thread
consumed that queued message, the previous handler unconditionally
cleared the current query and invoked `show_bridge_overlay`, effectively
reopening Search after cancellation. A safe pure-state regression for
active versus cancelled requests first failed RED, without sending any
window-showing message into the user's desktop. The BEGIN handler now
checks the atomic active state before any GUI mutation, discards an old
pending scope if cancelled, and exits. A second regression delivers an
actual cancelled `WM_SHELL_BRIDGE_BEGIN` to the invisible synthetic
Win32 parent and verifies it remains invisible with identical query and
scope while the pending stale scope is consumed. Pure and hidden-native
tests are GREEN. This does not exercise a physical low-level keyboard
hook or establish full IME, Narrator or SearchHost integration parity.
Production UI, foreground desktop, service and index remain untouched;
`package_status=INVALIDATED` remains in force.

### Version asynchronous Shell bridge BEGIN messages by request generation

Checking only `SHELL_BRIDGE_ACTIVE` fixes a cancelled request until a newer
request becomes active. An older queued `BEGIN` can then be dispatched while
the newer request is active, be mistaken for the new BEGIN, consume its
pending Explorer scope and clear its query. Added a nonzero monotonic
request-generation token to each posted `BEGIN` and verified both active
state and matching latest generation at dispatch. The generation and
scope are published while holding the same scope mutex to prevent a
mixed-generation snapshot. Cancelled current requests can drop only their
own pending scope; stale older requests may not clear newer scopes.
A pure Rust RED test reproduced the old-generation/new-active collision
(`old BEGIN displaced a newer active Search request`). The fix passes
that test. An additional hidden Win32 message test leaves a synthetic
newer request active, delivers the stale older BEGIN and asserts that
window visibility, query, scope and pending newer request remain unchanged.
The real low-level keyboard hook, physical IME, on-screen visual parity and
Narrator/UIA acceptance remain unverified. No production app, index,
SearchHost, foreground input or service was changed; release status remains
`INVALIDATED`.

### Prevent delayed bridge typing and shortcuts crossing Search sessions

The generation guard originally protected only `WM_SHELL_BRIDGE_BEGIN`.
Its queued follow-on `CHAR` and `KEY` messages carried no request identity.
An old Escape/Enter/Ctrl shortcut or translated character could therefore
be dispatched while a new Search session was visible and modify or close
that newer session. A pure RED regression reproduced acceptance of a stale
generation, then passed GREEN after all three bridge message types were
bound to the same full-width nonzero generation and the UI dispatcher
rejected stale or untagged input.

For both `CHAR` and `KEY`, `wParam` now carries the full request generation.
The `CHAR` UTF-16 code unit is carried in `lParam`; the `KEY` payload packs
the scan code and extended/control bits with the virtual key (8-bit VK)
in `lParam`. This avoids truncating generation on 32-bit Windows. A new
pure round-trip regression verifies Enter, Escape, left arrow, Ctrl+V and
Delete payloads without sending physical keyboard input. Normal typing
requires an active matching bridge; a same-generation queued cancellation
Escape can still dismiss it even after the hook has cleared ACTIVE.
The existing invisible synthetic Win32 GUI regression remains GREEN. No
live keyboard hook, visual matching, IME, Narrator, SearchHost or desktop
interaction was performed; release status remains `INVALIDATED`.

### Protect manually reopened Search against cancelled Shell bridge Escape

A same-generation queued `WM_SHELL_BRIDGE_KEY` Escape is intentionally
accepted after `SHELL_BRIDGE_ACTIVE` becomes false so that cancelling a
visible bridge popup can close it. However, the earlier handler checked
only the latest bridge generation and window visibility. If the resident
popup was dismissed and manually reopened through its global hotkey (or
an incoming IPC search request) before the queued Escape was dispatched,
that old Escape could close the **new manual session**. Added a distinct
per-window `bridge_session_generation` identity. `WM_SHELL_BRIDGE_BEGIN`
sets it; shared popup dismissal resets it to zero; manual global-hotkey
opening and valid `WM_COPYDATA` requests explicitly revoke bridge
ownership. `CHAR` and `KEY` handlers now require both the current
message generation and ownership of the displayed bridge session.
Cancellation Escape remains accepted only for its own still-displayed
bridge popup. A pure RED test reproduced acceptance of the stale Escape
against a manually reopened session; GREEN verifies that a generation-8
Escape cannot act when displayed ownership is zero. The hidden Win32
fixture also verifies that shared dismissal clears ownership and active
hook state. No real user keyboard, mouse, SearchHost, foreground UI,
production service or index was accessed. Physical visual, IME and
Narrator/UIA acceptance is still missing, so release stays `INVALIDATED`.

### Isolate failed bridge BEGIN posting and pending-scope consumption

After request N published its pending Explorer folder scope and released the
scope mutex, `PostMessageW` might fail. The previous failure path cleared
the pending scope unconditionally. If request N+1 published successfully
before N completed its failure cleanup, N erased N+1's scope. A pure Rust
regression for generation-aware cleanup first failed RED (`failed old
PostMessage can erase newer Explorer scope`). Cleanup now reacquires the
same scope mutex and clears only when the failed generation is still the
latest. An invisible synthetic Win32 fixture verifies N's failure does not
clear N+1's pending scope while N+1's own failure does clear it.

There was a related read-side race: a `WM_SHELL_BRIDGE_BEGIN` handler could
pass its initial generation test, then a new producer publish N+1 before
the handler called `take_bridge_scope()`, causing N to consume N+1's scope.
The consumer now reacquires the mutex, rechecks active state and generation
*while holding that mutex*, and consumes only its matching scope. The
hidden-window regression verifies stale N cannot take N+1's scope and that
N+1 can still consume its own scope. The full set of tests uses isolated
fixture state only, no foreground window, production Windows Search index,
user keyboard, or real Shell hook. Physical IME, Narrator and visual parity
remain unverified, and the release state remains `INVALIDATED`.

### Remove temporary bridge topmost when a manual Search request takes over

A `WM_COPYDATA` Search request can arrive while the optional keyboard-bridge
popup still has its temporary `HWND_TOPMOST` z-order. The earlier IPC
handler revoked bridge input ownership but called `SW_RESTORE` without
first releasing topmost, so manual Search could remain persistently above
unrelated applications. The hidden-window regression now simulates a
never-shown native HWND with `WS_EX_TOPMOST` and active bridge ownership,
then invokes just the shared manual-handoff helper. Initially RED:
`manual bridge handoff retained temporary topmost z-order`. The fix routes
manual hotkey opening and accepted IPC requests through a single handoff
that clears the bridge's active state and pending scope, revokes the
bridge session identity, releases temporary topmost using
`SWP_NOACTIVATE`, and resets cached Appearance-button hover. The same
hidden HWND fixture is GREEN: not topmost, no bridge ownership, no stale
hover, and still invisible. No real `WM_COPYDATA` desktop activation,
physical keypress, SearchHost modification or production index/service
write was performed. Manual/physical visual, IME and Narrator parity remain
unverified; release status stays `INVALIDATED`.

### Do not intercept untranslated or dead-key bridge input

The optional keyboard bridge uses `ToUnicodeEx` to convert a typing key
into UTF-16 units before it suppresses the native key. The old
`post_translated_bridge_chars` returned `true` when conversion yielded
an empty vector, because its loop completed without attempting to post
anything. This incorrectly swallowed a dead key or failed layout
translation (including a `ToUnicodeEx` no-output result) even though no
character reached Search Tool. The bridge would also attempt to open
Search from such an empty first keystroke.

Translation forwarding now uses a small testable `post_bridge_utf16_units`
helper. Empty output returns `false`, making the hook's existing fallback
clear ACTIVE and pass the key through to the original foreground app.
Nonempty output succeeds only if every UTF-16 unit was posted, stopping
at the first failure. The pure Rust regression first failed RED
(`dead-key or untranslated key would be silently swallowed`), then passed
GREEN with empty, complete and early-post-failure cases. The release
build and the existing invisible synthetic Win32 regression also pass.
The test deliberately does not install the keyboard hook or mutate a real
keyboard layout/IME session. Physical international dead-key and IME
acceptance remain unverified; `package_status=INVALIDATED` stays in force.

### Ignore stale category-chip clicks after resident popup dismissal

A native child `BUTTON` can retain its `WS_VISIBLE` style after its parent
Search flyout is hidden. The category `WM_COMMAND/BN_CLICKED` handler used
to check only that child style. A queued click from the previous session
could therefore change the cached search category and run another index
query while the popup was dismissed, affecting the next visible session.
A new never-shown synthetic Win32 test sends the actual `BN_CLICKED` for
Files to an invisible parent whose child still has `WS_VISIBLE`. On the
old handler it failed RED: `hidden parent accepted stale category click
and triggered requery`. The handler now additionally requires
`IsWindowVisible(parent)` before processing the category action, matching
the already-existing protections for Open, Appearance and double-clicks.

The selected-category update was factored into `select_search_category`.
The production `WM_COMMAND` path calls it only when the parent and chip
are visible; hidden fixture tests call the same transition directly to
check requery, repeated-selection no-op and category accessibility names
without pretending their invisible parent has been clicked. The hidden
native regression and the separate external MSAA/WinEvent offscreen
regression are GREEN. No actual window was shown or focused, no user
mouse/keyboard was intercepted and no real file was opened. Physical
Windows 11 visual, keyboard/IME, Narrator/full UIA and SearchHost entry
point acceptance remain pending; `package_status=INVALIDATED` remains.

### Reject delayed ListBox selection changes after popup dismissal

The native `LBN_SELCHANGE` handler previously guarded only the child
ListBox `WS_VISIBLE` style. That flag can remain set after the resident
Search parent has been dismissed. A queued selection notification could
then encounter a temporarily inconsistent native row-to-cache mapping,
call the invalid-selection recovery routine and erase a still-cached
selection and its accessible detail while no real selection interaction
had occurred. The next manual Search session would inherit the loss.

The hidden Win32 regression now creates a selected synthetic row inside a
never-shown native parent whose child retains `WS_VISIBLE`, changes only
the synthetic selected row's `LB_SETITEMDATA` mapping, delivers the actual
`WM_COMMAND/LBN_SELCHANGE` and verifies the selected index, details and
status remain unchanged. This failed RED before the fix with `hidden
parent accepted stale ListBox selection-change notification`.

The real ListBox notification handler now requires both an actually
visible parent and a child with `WS_VISIBLE`, matching the existing
category, Open and double-click guards. Row validation, corrupt-selection
rejection and accessible detail updates are in a shared
`process_list_selection_change` routine. The hidden synthetic tests call
this shared routine directly for deliberate valid/corrupt row checks,
without forging interactive clicks on an invisible window. The native
hidden regression and out-of-process MSAA/WinEvent regression pass GREEN.
No user input, foreground window, SearchHost, production index or service
was modified. Physical Windows 11 UIA/IME/Narrator acceptance is still
missing and `package_status=INVALIDATED` remains.

### Reject stale native EDIT changes after resident popup dismissal

A native EDIT may retain `WS_VISIBLE` in its own style after its resident
Search parent has been hidden. Previously the `WM_COMMAND/EN_CHANGE`
handler did not check parent visibility, so a delayed notification from
an earlier search could synchronously rerun the index and replace cached
search status/results even while the user was no longer in Search Tool.
The never-shown native Win32 fixture now sends the real `EN_CHANGE` to its
hidden parent while the child still has the visible style, and asserts
that the status marker and three cached search results are unchanged.
This failed RED before the fix: `dismissed Search parent accepted stale
EN_CHANGE and repeated the index query`.

For resident mode, the production handler now ignores `EN_CHANGE` when
the Search parent is hidden. Normal visible edits still share the same
IME-aware, programmatic-update-aware `refresh_from_committed_edit_change`
handler. Isolated hidden UI tests invoke that shared handler directly
for their deliberate synthetic query changes, rather than spoofing user
input on an invisible parent. An asynchronous Shell bridge BEGIN now
clears its previous query and results explicitly with `set_query("")`
before showing the new bridge popup; it no longer depends on a native
hidden-parent `EN_CHANGE` side effect to clear old results.

Hidden native Win32 and offscreen external MSAA regressions pass GREEN.
No production SearchHost/index/service, actual desktop focus, physical
keyboard, IME session or user foreground window was modified. Real
Windows 11 visual, IME and Narrator/UIA acceptance is pending, and
`package_status=INVALIDATED` remains enforced.

### Do not run a dismissed-popup query after IME completion or focus loss

The native EDIT subclass tracks `WM_IME_STARTCOMPOSITION` and clears its
preedit guard when `WM_IME_ENDCOMPOSITION` or `WM_KILLFOCUS` arrives. Its
former completion path called `refresh_results` unconditionally, even
when the resident Search flyout had already been dismissed. Such a late
IME notification could unexpectedly perform an index query and overwrite
cached status/results after the user switched to another window.

The synthetic never-shown native Win32 fixture now starts composition,
changes the test-only query while its IME guard is set, places a status
marker, then delivers `WM_IME_ENDCOMPOSITION` with a hidden parent. The
original behavior failed RED (`dismissed IME finalization started an
offscreen index query`). The subclass now always releases the IME guard
but refreshes a resident Search query only if its parent is visible.
The nonresident application's existing edit semantics are preserved.
Hidden-window tests separately exercise the same shared committed-query
refresh invoked after a visible composition finishes, and verify that
focus-loss releases the guard without a hidden search. The native hidden
Win32 and external offscreen MSAA/WinEvent tests pass GREEN. No physical
IME is installed or activated, and no real keyboard, SearchHost or
production index/service is accessed. Physical Windows 11 visual, IME,
Narrator and full UIA acceptance remain outstanding, keeping
`package_status=INVALIDATED`.

### Drop queued native keyboard input targeted at a dismissed resident popup

The normal GetMessage loop previously evaluated Search's global Return,
Escape, Down/Up and Tab navigation for every queued native key without
checking whether the resident parent was still visible. It could also
forward delayed WM_CHAR input to a child EDIT/ListBox whose parent was
already dismissed. Child controls may retain WS_VISIBLE style after the
parent hides; the existing hidden WM_COMMAND guards do not stop native
keyboard dispatch. A delayed Enter could preflight/open a stale cached
selection, or delayed character input could alter an invisible query.

The main message loop now drops native keyboard and character events
owned by the resident Search parent or its direct children while that
parent is invisible. Covered events are WM_KEYDOWN/UP, WM_SYSKEYDOWN/UP,
WM_CHAR/DEADCHAR/SYSCHAR/SYSDEADCHAR/UNICHAR and WM_IME_CHAR. It checks
the actual HWND parent relationship, so unrelated queued messages are
not consumed. WM_HOTKEY, WM_COPYDATA, IME lifecycle bookkeeping and
nonresident windows retain their original handling. Visible Search's
normal keyboard/IME routing is unchanged.

The Rust regression `dismissed_resident_keyboard_queue_never_routes_old_text_or_enter`
first failed RED on the old unconditional route, then passed GREEN for
hidden native key/text/key-up cases and accepted hotkey, IPC, visible,
nonresident and unowned controls. A never-shown synthetic Win32 fixture
also asserts its EDIT and ListBox belong to a hidden resident parent and
that delayed Enter and character events are rejected. The hidden-native
regression passed GREEN. No physical keyboard/mouse input, foreground
window, real SearchHost, production index/service, file open, or app
activation was performed. Physical Windows 11 keyboard/IME, visual and
Narrator/UIA acceptance remain outstanding. Release stays `INVALIDATED`.

### Keep a newer Shell BEGIN alive through stale dismissal notifications

Native `WM_ACTIVATE` (WA_INACTIVE) and `WM_CLOSE` may be delivered after a
resident Search parent was already hidden. Previously both unconditionally
called `hide_bridge_overlay`, which clears `SHELL_BRIDGE_ACTIVE` and the
global pending Explorer folder scope. If the optional Shell hook had
published generation N+1 after an earlier popup's dismissal but before
an old deactivation/CLOSE reached the UI thread, that stale notification
could wrongly cancel N+1 and erase its pending search scope.

The never-shown native Win32 regression now publishes a synthetic active
newer generation and a pending Explorer scope while the resident parent
is hidden, then delivers the actual `WM_ACTIVATE/WA_INACTIVE` and
`WM_CLOSE` messages. Before the fix it failed RED with `hidden old
dismissal cancelled a newer pending Shell bridge session`. Both handlers
now call the real bridge cancellation path only for an actually visible
resident popup; late events on an already-hidden parent instead perform
visual-only cleanup through `clear_hidden_resident_chrome`. This still
removes temporary HWND_TOPMOST and stale Appearance hover, preserving
prior native z-order/hover regression behavior, without changing global
bridge session ownership or clearing N+1's pending scope.

The updated synthetic hidden Win32 and external MSAA/WinEvent regression
pass GREEN. Normal visible-prompt dismissal is unchanged. No live
window was shown, no user input/focus, production Windows Search index,
SearchHost or service was accessed. Physical Windows 11 visual,
keyboard/IME, Narrator/UIA and official Search entrypoint acceptance
remain unverified, and release status remains `INVALIDATED`.

### Reject malformed WM_COPYDATA headers and bounds before decoding

The IPC Search handler previously dereferenced `lParam` as a
`COPYDATASTRUCT` before checking anything about the incoming message.
A malformed local sender could provide `lParam=0`, terminating the GUI
with a null-pointer access violation. The never-shown synthetic Win32
fixture now delivers an actual `WM_COPYDATA` with a null header and checks
that the resident Search parent stays hidden and its original synthetic
query/results remain unchanged. Before the fix the hidden GUI exited
`-1073741819` (Windows access violation) without writing its expected
test report: **RED**. The handler now rejects a missing header before
any dereference and the same native fixture passes **GREEN**.

The prior handler also constructed a slice from the sender-provided
`cbData` size before checking the type-specific maximum. The new
`valid_copydata_length` guard enforces nonzero, even UTF-16 byte counts,
a maximum of `(MAX_QUERY_U16+1)*2` for legacy query requests, and 131072
bytes for query-plus-scope requests, and rejects unknown message types,
all *before* forming a slice. A focused Rust regression tests both exact
bounds and short, odd, oversized and unsupported inputs. The existing
accepted Search IPC behavior remains unchanged. No actual external
sender, production Search index/service, foreground desktop, SearchHost
or visible user window was touched. Physical Windows 11 visual/IME,
Narrator/UIA and Search entrypoint acceptance remain outstanding;
release status remains `INVALIDATED`.

### Strict type-2 query and Explorer-scope IPC framing

The documented same-process sender for type-2 `WM_COPYDATA` sends exactly
two UTF-16 NUL-terminated fields (`query\0scope\0`). The old decoder
accepted a missing final scope terminator (even a one-word `[0]` request),
ignored arbitrary bytes after the second terminator and replaced malformed
UTF-16 with U+FFFD. Those malformed payloads could unexpectedly activate
a dismissed Search popup, change its query or reinterpret the Explorer
scope. The old decoder also allowed a type-2 query larger than the
`MAX_QUERY_U16` native search-text budget.

Type-2 decoding now requires both NUL delimiters, no trailing words after
the second, query length at most 1024 UTF-16 units and scope length at
most 32767 UTF-16 units. Both text segments are decoded with strict
`String::from_utf16`, so unpaired surrogates are rejected rather than
silently replaced. Valid empty-global-search packets, the ordinary
query-plus-scope round trip and non-BMP Unicode remain accepted.

The pure Rust regression first failed RED when `[0]` decoded as a valid
empty search request, then confirmed separate RED behavior for an unpaired
UTF-16 surrogate, and now passes GREEN for all invalid framing and
boundary cases. The hidden native Win32 regression also delivers actual
malformed type-2 `WM_COPYDATA` messages to the never-shown test window,
verifying they return 0 without opening, focusing or altering it. No
invalid packet was delivered to the user's live Search installation.
The physical Windows 11 visual, IME, Narrator/UIA and official Search
entrypoint acceptance gates are still pending; release remains `INVALIDATED`.

### Strict legacy single-query WM_COPYDATA decoding

The older type-1 IPC path used `String::from_utf16_lossy` and ignored
anything following the first UTF-16 NUL. A malformed sender could change
the effective query by inserting an unpaired surrogate (silently replaced
with U+FFFD) or a hidden suffix. A separate test-first regression first
failed **RED** for an unpaired surrogate decoded into a real Search query.

The shared `decode_legacy_ipc_query` now accepts both original raw UTF-16
packets and packets with one optional *final* NUL for compatibility.
Any NUL followed by another UTF-16 unit is rejected, as are malformed
surrogates and queries exceeding the 1024-unit native search limit.
Valid empty requests, ordinary Unicode text and non-BMP symbols are
preserved. The focused Rust test passes **GREEN**. Never-shown native
Win32 tests also dispatch actual malformed type-1 `WM_COPYDATA` to the
isolated test window and confirm no activation, query/result change or
focus takeover; hidden GUI and offscreen MSAA tests pass.

No real SearchHost instance, foreground window, production index/service,
user keyboard/mouse or deployed application was changed. Physical visual,
IME, Narrator/UIA and Search entrypoint acceptance remain outstanding;
release status stays `INVALIDATED`.

### Preserve non-BMP Unicode as one Shell bridge character

The optional Shell typeahead keyboard hook previously posted **one
`WM_SHELL_BRIDGE_CHAR` for each UTF-16 code unit** returned by
`ToUnicodeEx`. A valid non-BMP character such as a folder emoji is a
surrogate pair, so that design queued two independent messages and
applied two separate `EM_REPLACESEL` edits/search refreshes. The first
half was not a valid Unicode character, and partial posting failures
could leave an orphan half in the Search query.

The new posting path validates the *entire* translated UTF-16 sequence
with `char::decode_utf16` before any posts. Empty, unpaired-surrogate and
NUL translations are not captured. Each complete Unicode scalar is
posted as a single 32-bit codepoint in `lParam`; the UI thread validates
it with `char::from_u32`, encodes it back to one or two UTF-16 units,
and performs a single native `EM_REPLACESEL`. The search text-length
limit accounts for the scalar's actual UTF-16 width. Backspace remains
U+0008 and uses the existing surrogate-aware previous-boundary logic.
Multiple *distinct* scalars are still separately queued, so there is
no claim of atomic delivery across a multi-character translation.

The focused `shell_bridge_unicode_scalar_posts_surrogate_pair_atomically`
regression failed **RED** when one emoji produced two posts and passes
**GREEN** after the change. It also verifies that a malformed UTF-16
translation cannot partially post an earlier valid prefix. The hidden
native Win32 fixture uses the same EDIT replacement routine to append
one complete emoji, reject an isolated surrogate, and delete the whole
pair with Backspace while maintaining cached results and never showing
or focusing the test window. Hidden Win32 and external MSAA/WinEvent
regressions pass. No real keyboard hook was installed by this test and
physical Windows 11 IME/keyboard/Narrator/full UIA acceptance remains
unverified. Release status remains `INVALIDATED`.

### Clear stale IME preedit state when dismissing resident Search

The resident flyout previously cleared its bridge generation and active
keyboard interception during `hide_bridge_overlay`, but did not clear
`state.ime_composing`. Some IMEs terminate through focus changes and may
not deliver a final `WM_IME_ENDCOMPOSITION`. In that case a hidden popup
could retain `ime_composing=true` across dismissal and a later manual or
Shell bridge opening, suppressing normal committed search refresh and
popup shortcuts even though no active IME remained.

The never-shown native Win32 fixture now sends a synthetic
`WM_IME_STARTCOMPOSITION` to its test EDIT, confirms the composing guard,
and calls the real resident dismissal routine. Before the fix it failed
**RED**: `resident dismissal retained stale IME composition across
sessions`. `hide_bridge_overlay` now clears `ime_composing` together with
`bridge_session_generation`, before clearing global bridge ownership.
The same fixture passes **GREEN**; native hidden GUI and external
MSAA/WinEvent diagnostics also pass. This is only an internal lifecycle
regression: no real IME, user keyboard/focus, visible desktop window,
production SearchHost or index/service was manipulated. Physical Windows
11 IME/Narrator/full UIA and visual acceptance remain pending; release
status stays `INVALIDATED`.

### Reset IME preedit state again when a new Search session takes ownership

Clearing IME state on resident dismissal does not fully handle a queued
old `WM_IME_STARTCOMPOSITION`: after the parent hides and clears the
state, a delayed START can still arrive at its native child EDIT,
marking `ime_composing=true` until a later END/KILLFOCUS. If an explicit
hotkey/IPC or a new Shell keyboard-bridge BEGIN then reopens Search,
it could inherit this old preedit guard, deferring EN_CHANGE and normal
Search keyboard shortcuts in a session with no active composition.

The never-shown native Win32 regression now injects an old
`WM_IME_STARTCOMPOSITION` after dismissal, verifies the stale guard,
then calls the same `handoff_bridge_to_manual_session` path used by
manual Search and valid IPC requests. It first failed **RED**:
`manual Search takeover inherited a delayed hidden IME composition`.
The handoff now clears `ime_composing` alongside bridge session
ownership. The accepted `WM_SHELL_BRIDGE_BEGIN` handler also explicitly
clears the old IME guard before resetting query/results and showing
its new session. Hidden Win32 and external MSAA/WinEvent regressions
pass **GREEN** without opening any physical window or interacting with
real IME, production SearchHost, desktop input or Windows Search index.
Physical Windows 11 IME/keyboard, Narrator/full UIA and visual parity
remain unverified. Release remains `INVALIDATED`.

### Ignore stale native EDIT focus notifications on an invisible flyout

A native EDIT child retains the `WS_VISIBLE` style after its resident
Search parent is dismissed. A delayed `WM_COMMAND/EN_SETFOCUS` could
still set `state.edit_focused=true` and request a repaint of the Search
border while its parent was invisible. If no matching `EN_KILLFOCUS`
arrived, the next session could inherit a misleading focus highlight.

The never-shown native Win32 fixture now sets up a hidden resident parent
with a visible-style EDIT child, delivers the actual synthetic parent
`WM_COMMAND/EN_SETFOCUS` notification and asserts the cached focus
state stays false. Before the fix, it failed **RED** with `dismissed
Search parent accepted delayed EDIT focus notification`. The handler
now accepts `EN_SETFOCUS` only when the resident parent is actually
visible, while nonresident windows continue to handle focus normally.
Resident dismissal also resets cached `edit_focused` without depending
on a later child `EN_KILLFOCUS`. The hidden native Win32 regression and
external MSAA/WinEvent diagnostic both pass **GREEN**.

No real Windows foreground/focus, keyboard/mouse, production SearchHost,
Windows Search index or service was modified. Physical keyboard/IME,
Narrator/full UIA, visible Windows 11 Search parity and native entrypoint
acceptance remain pending; release stays `INVALIDATED`.

### Fail closed on an unusable explicitly requested Explorer scope

An accepted type-2 `WM_COPYDATA` request encodes `query\0scope\0`.
The decoder previously passed even a nonempty but unusable `scope` to
`normalize_scope` and treated a `None` result as permission to search
globally. For example, a scope consisting only of `\` or `\\` was
normalized away, and `secret\0\\0` became an unrestricted search for
`secret`. This silently widened the requested search scope.

The new `validated_explicit_scope` helper distinguishes an **actually
empty** scope field (intentional global search) from a **nonempty** field
that normalization eliminates (invalid, reject entire request). It is
shared by type-2 IPC and the `search:`/`searchtool:` URI parser's
`scope`, `location`, and `crumb=location:` forms. No untrusted invalid
explicit scope can silently be converted into a global request. Valid
absolute paths and the explicitly empty global-search form are retained.

Two Rust regressions first failed **RED**: the IPC test reproduced the
invalid-root-to-global fallback and the URI test reproduced it for
`search:q=secret&scope=%5C`. Both now pass **GREEN**, including quoted,
whitespace-only and separator-only invalid scopes. The never-shown
native Win32 fixture also delivers actual type-2 `WM_COPYDATA` requests
with invalid scopes, checks return 0 and confirms the synthetic Search
popup stays hidden with its prior query and cached results unchanged.
There was no interaction with real Shell Search, desktop focus, production
index/services, real file launching or installed Search Tool. Physical
Windows 11 UI/keyboard/IME, Narrator/full UIA and SearchHost entrypoint
acceptance are still pending; release status stays `INVALIDATED`.

### Reject conflicting repeated URI location parameters

A `search:` or `searchtool:` URI can contain several separately named
scope fields (`scope`, `location`, `crumb=location:`). Previously each
field independently replaced `SearchRequest.scope`, with the last one
winning. In particular, `search:q=secret&scope=C%3A%5CPrivate&location=`
would silently discard the original `C:\Private` restriction and expand
search to all indexed files, despite both field values individually
passing the earlier explicit-scope validation. A later different path
could likewise redirect search away from the initial location.

The URI decoder now records whether an explicit scope field has already
been seen and requires every subsequent explicit scope to match it.
Windows case-insensitive path-equivalent repeats are accepted, as are
repeated intentionally empty global fields. Switching between global and
restricted scopes, or providing two distinct restricted folders, rejects
the entire URI. This applies consistently across `scope`, `location` and
Explorer `crumb=location:` parameters, independent of parameter order.

A new focused Rust regression first failed **RED** on the
`scope=C:\Private` then `location=` case. It now passes **GREEN**,
including alternate path conflicts and legitimate repeated locations.
These are pure URI parser tests; they do not open visible GUI windows,
install a keyboard hook, access SearchHost or the production index, or
replace the still-required physical UIA/IME/visual acceptance.
Release remains `INVALIDATED`.

### Refuse drive-relative Windows scope paths before broad indexed search

On Windows `C:` and `C:Private` are **drive-relative** path expressions,
not absolute `C:\` or `C:\Private` locations. The earlier
`normalize_scope` accepted `C:` unchanged; the lexical indexed-path
scope filter appended a backslash to build `c:\`, unintentionally
matching every absolute path on drive C. This could broaden an explicit
scoped Search request to the entire volume.

`normalize_scope` now rejects a drive-letter-colon prefix unless it is
followed by an actual root separator, after converting optional `/` to
Windows `\`. The rule applies consistently to command-line `--scope`,
type-2 `WM_COPYDATA`, URI `scope`/`location`/Explorer location crumbs,
and other normalized Explorer scope inputs. `C:\` and `C:/Private`
remain valid absolute forms; `C:`, `c:Private` and
`D:docs\reports` fail closed rather than expanding or reinterpreting
scope.

The new `drive_relative_scope_must_not_expand_to_volume_root` Rust
regression failed **RED** when the old code accepted `C:`. It now
passes **GREEN**, checking normalization, explicit URI/IPC validation
and valid absolute roots. The never-shown native Win32 fixture also
sends actual type-2 `WM_COPYDATA` messages with `C:` and `C:Private`
and confirms no activation or query/result mutation. Physical Windows
11 visual, keyboard/IME, Narrator/full UIA and SearchHost entrypoint
acceptance are still outstanding; release remains `INVALIDATED`.

### Reject invalid explicit CLI scope and Search URI before opening the GUI

The native command-line entrypoint has two more ways to silently broaden
search despite the parser-level scope fixes. Previously `--query secret
--scope C:` first stored the query, then assigned the now-invalid
`normalize_scope("C:") == None`. Launch proceeded as an unrestricted
Search query instead of reporting invalid input. Similarly, the
`--search-uri` flag and directly supplied `search:`/`searchtool:`
arguments ignored parse failures. An earlier command-line `--query`
therefore remained active, potentially as a global query, even when the
caller explicitly supplied an unusable URI. Missing `--scope` and
`--search-uri` values were also ignored.

The CLI now calls testable `apply_cli_scope` and `apply_cli_search_uri`
helpers. Invalid nonempty scopes, rejected or empty Search URIs, and
missing flag arguments return `io::ErrorKind::InvalidInput` **before
opening the Search GUI**. The prior SearchRequest remains unchanged on
failure. Truly empty explicit scope and valid scoped or global URIs still
work, including documented unkeyed `search:just%20text` input.

The focused `cli_explicit_invalid_scope_or_uri_cannot_turn_restricted_search_global`
regression failed **RED** when old `--scope=C:` behavior returned `Ok`
and deleted `C:\Private` restriction. It now passes **GREEN** for
invalid drive-relative, separator-only and whitespace-only CLI scopes,
invalid/rejected URIs, and valid positive controls. No user-visible
Search window, physical keyboard/IME, production index, SearchHost or
service was touched. Physical visual/UIA/Narrator/IME acceptance is
still unverified and release status remains `INVALIDATED`.

### Strict decoded URI bytes: no invalid UTF-8 or embedded Win32 NUL

The `search:` and `searchtool:` URI parser previously called
`String::from_utf8_lossy` after percent decoding. Invalid UTF-8
sequences such as `%FF` were silently replaced by U+FFFD rather than
rejected. Percent-encoded `%00` produced an embedded NUL that could
truncate native EDIT/query text while preserving a different logical
`SearchRequest` value. Both cases could silently reinterpret an
explicitly requested query or Explorer scope.

`percent_decode` now returns `Option<String>` using strict
`String::from_utf8` and rejects any decoded NUL. The URI parser
propagates failure for named and unkeyed `q`/`query` text, `scope` and
`location`, and Explorer `crumb` values. Well-formed UTF-8, `+`
space decoding and non-BMP emoji in queries remain supported.

The regression `search_uri_rejects_invalid_utf8_and_embedded_nul_bytes`
failed **RED** for `search:q=%FF`, accepted as query `�` by the old
parser. It passes **GREEN** after the strict decoder, including malformed
multibyte UTF-8, embedded NUL in query/scope/crumb, and positive
multilingual UTF-8 + emoji preservation. Existing private-protocol,
Explorer scope-crumb and plus-space tests also pass.

No foreground GUI, physical keyboard or IME, production SearchHost,
index/service or real file launch was exercised by these pure parser
tests. Physical Windows 11 visual, IME, Narrator and full UIA acceptance
remains pending; release state stays `INVALIDATED`.

### Decode and validate URI parameter names before scope decisions

URI query strings can percent-encode parameter **names**, not only their
values. Previously the Search Tool parser compared the raw names against
`scope`, `location` and `crumb`; an encoded name such as `sc%6fpe` was
ignored as unknown. `search:q=secret&sc%6fpe=C%3A%5CPrivate` therefore
became a **global** search for `secret` instead of respecting the explicit
`C:\Private` restriction. This was independent of the earlier strict
UTF-8 and repeated-scope checks on decoded parameter values.

The URI parser now passes each parameter name through the existing
strict `percent_decode` before interpreting it, with invalid UTF-8 or
embedded NUL rejected. Encoded names for `scope`, `location` and
`crumb=location:` are recognized and flow through the same scope
validation/contradiction checks as their unescaped forms.

`uri_percent_encoded_scope_keys_must_not_be_silently_ignored` failed
**RED** with `sc%6fpe` ignored, returning `scope=None`. It now passes
**GREEN** for multiple correctly encoded key variants and malformed,
unsafe and contradictory forms. The prior URI decoding regression
remains GREEN. This is parser-only validation on the isolated developer
worktree; physical GUI, keyboard/IME, Narrator/full UIA and native
SearchHost integration acceptance remain outstanding. Release remains
`INVALIDATED`.

### Require a fully qualified Windows path for explicit Search scopes

Drive-relative syntax (`C:` / `C:folder`) was rejected in the previous
revision, but bare relative locations such as `Private`, `.\Private`,
root-relative `\Private`, and incomplete UNC locations such as
`\\server` were still accepted by `normalize_scope`. Scoped indexed
search operates on *absolute indexed paths*, so an ambiguous relative
request could not reliably identify its intended directory; using a
substring-style filter without a fully qualified root risks broader
results than the requested folder.

`normalize_scope` now requires either a rooted drive path (`C:\` or
`C:\Private`) or a complete UNC server/share pair (`\\server\share`
and descendants). Single leading separators, bare relative paths,
incomplete server names, duplicate/missing UNC share separators and
Windows device namespace prefixes are rejected. Slash normalization
and trailing separator handling remain compatible with legitimate
absolute Windows and UNC paths. The same normalization applies to
Explorer scope extraction, explicit `--scope`, `search:` URI scope,
and scoped type-2 `WM_COPYDATA` requests.

The focused
`relative_or_incomplete_unc_scope_must_not_match_unrelated_locations`
Rust regression failed **RED** when the previous implementation
accepted the bare `Private` name. It passes **GREEN** after enforcing
rooted paths; valid drive and UNC shares remain accepted. The native
never-shown Win32 fixture also checks rejection of incomplete UNC and
relative scoped IPC requests without opening the parent or changing
cached query/results. Physical GUI, IME, Narrator/full UIA and supported
Search entrypoint acceptance remain outstanding; release status stays
`INVALIDATED`.

### Reject unresolved dot segments in explicit Search scopes at entry

Windows resolves `.` and `..` path components before opening a folder.
Although the result-level `path_is_within_scope` filter already refuses
unresolved dot segments, `normalize_scope` previously **accepted**
`C:\Private\..\Public` as an explicit search location. Consequently
CLI, IPC or URI input would appear valid and start a search, only to
silently discard candidates later rather than report the unsafe or
ambiguous location at the entrypoint. A folder scope must not be treated
as a harmless lexical prefix when it can resolve into another directory.

`normalize_scope` now reuses the existing `has_dot_path_segment` guard
after slash normalization. Whole-component `.` or `..` in drive or
UNC directory paths is rejected before the request is accepted. Valid
names such as `.config` and `release..txt` continue to work, as do
ordinary absolute drive and UNC paths. This guard applies consistently
to CLI `--scope`, `search:` and `searchtool:` URIs, Explorer location
extraction, and scoped type-2 `WM_COPYDATA` IPC.

The test `explicit_scopes_with_dot_segments_must_fail_before_search`
failed **RED** because the previous normalizer returned the unresolved
`C:\Private\..\Public` path. It now passes **GREEN**, including
URI/IPC variants and legitimate dotted folder names. The hidden native
Win32 fixture additionally sends actual invalid scoped IPC messages
and confirms its never-shown parent and cached query/results remain
unchanged. This remains developer-lab evidence only: physical Windows
11 visuals, keyboard/IME, Narrator/full UIA and supported native
SearchHost entrypoint integration are unverified, and release remains
`INVALIDATED`.

### Reject missing `=` on explicit URI parameters instead of broadening Search

The URI parser previously skipped nonempty query-string components lacking
`=` even when processing a keyed URI. As a result,
`search:q=secret&scope` became an unscoped/global `secret` Search rather
than rejecting the missing value for the explicitly requested scope.
The same ambiguity affected `location`, Explorer `crumb`, percent-encoded
parameter names (`sc%6fpe`) and any bare parameter following keyed input.

Within a keyed URI (containing `=`), all nonempty components must now be
`key=value`; bare tokens reject the complete URI before any Search
session begins. An empty trailing `&`, explicitly empty `scope=` and
legacy unkeyed `search:meeting%20notes` remain compatible.

The `uri_missing_equals_must_not_silently_drop_explicit_scope`
Rust regression was first **RED** on `search:q=secret&scope`, yielding a
`SearchRequest { scope: None }`. It is **GREEN** after the fail-closed
parser change, including malformed `location`, `crumb`, encoded keys and
positive compatibility cases. The previous encoded-name and documented
Search URI tests also pass. This is isolated parser and offscreen GUI
laboratory work, not physical Windows 11 visual, IME, Narrator or full
UIA acceptance. The package remains `INVALIDATED`.

### Enforce a consistent UTF-16 query limit on CLI and Search URI input

Native Search Tool's `refresh_results` reads at most `MAX_QUERY_U16`
(1,024 UTF-16 units) from its EDIT. The already strict type-2 Win32
`WM_COPYDATA` query decoder enforces this maximum, but explicit
`--query` and the named/unkeyed `search:` or `searchtool:` URI parsers
previously accepted longer strings. A caller could therefore see one
full text value in native EDIT while indexing only its 1,024-unit
prefix, causing search semantics to differ from the requested query.
`--query` also accepted a literal embedded NUL, which could truncate
native Win32 text while leaving a different high-level string.

A shared `validated_external_query` now checks the trimmed text has no
NUL and fits the exact UTF-16 query budget. The CLI's `--query` parser
uses a testable `apply_cli_query` that rejects invalid input without
mutating an earlier `SearchRequest`; a missing `--query` value now
returns InvalidInput. Named `q`/`query` and unkeyed URI text use the
same validator after strict percent decoding, rejecting excess length
rather than silently truncating. Valid 1,024-unit text including 512
Unicode emoji scalars remains supported.

The new `external_queries_must_not_silently_truncate_or_embed_nul`
regression first failed **RED** when CLI accepted a 1,025-unit query.
It passes **GREEN** for overlong BMP/non-BMP and embedded-NUL negative
cases and the exact 1,024-unit positive control, while previous URI
and UTF-8 regressions remain GREEN. No desktop foreground, physical
IME/keyboard, production SearchHost, index, service or file launch was
used. Physical Windows 11 visual/UIA/Narrator/IME acceptance remains
outstanding; release stays `INVALIDATED`.

### Reject incomplete and non-hexadecimal URI percent escapes

The strict UTF-8 Search URI decoder previously still preserved a bare
`%` for an incomplete or non-hexadecimal escape rather than rejecting
it. In particular `search:q=secret&sc%ope=C%3A%5CPrivate` silently
ignored the malformed `sc%ope` parameter name as unknown and launched
a global Search for `secret`, even though the caller supplied what was
intended as a scoped Explorer location. Similar ambiguity affected
malformed escapes in `q`, `scope`, `location`, and `crumb` values.

`percent_decode` now rejects every `%` that is not followed by exactly
two hexadecimal digits, regardless of whether the escape occurs in a
parameter name or value. This combines with the existing strict UTF-8,
embedded-NUL and explicit-scope validations. Properly escaped `%25`
still becomes a literal percent sign inside the query.

The focused `search_uri_rejects_malformed_percent_escapes_without_global_fallback`
regression first failed **RED** when `sc%ope` became an ignored key and
an unrestricted SearchRequest. It passes **GREEN** for truncated and
non-hexadecimal encodings, plus the well-formed `%25` positive control.
The earlier encoded-key and invalid-UTF-8 regressions remain GREEN.
No production index, live SearchHost, desktop focus or visible GUI was
exercised. Physical Windows 11 parity, real keyboard/IME, Narrator/full
UIA and supported entrypoint acceptance are still unverified; release
remains `INVALIDATED`.

### Preserve query identity across repeated URI aliases and malformed location crumbs

In `search:` and `searchtool:` URIs the `q` and `query` parameters are
aliases for the same search text. Previously the parser unconditionally
accepted the last nonblank query parameter, so `search:q=secret&query=public`
executed `public`, while `search:q=secret&q=` silently preserved `secret`.
This makes a request ambiguous and can search for text different from
its first explicit query. `accept_uri_query` now tracks the first explicit
query (including an intentionally empty value) and only permits repeated
aliases if their validated, trimmed query text is identical. Conflicting
values in either order reject the complete request.

An Explorer `crumb=location` value without a separating colon was also
silently ignored. In `search:q=secret&crumb=location`, that could become
a global Search request although a location crumb was explicitly named.
The decoder now rejects exactly `crumb=location` (case-insensitive)
without `:`, while preserving other native Explorer crumbs such as
`crumb=kind:document` and valid `crumb=location:C%3A%5CPrivate`.

Both focused GUI Rust regressions first failed **RED** on their original
parser behavior and passed **GREEN** after the fixes. Positive controls
confirm repeated identical query aliases and other valid Explorer crumbs
remain supported. The existing explicit-scope conflict and URI parser
tests also pass. These are isolated parsing tests, not physical keyboard,
IME, UIA/Narrator, SearchHost entrypoint, or Windows 11 visual parity
acceptance. Release status remains `INVALIDATED`.

### Native Win32 EDIT enforces the same 1,024-unit query budget

External CLI, URI, and scoped WM_COPYDATA query ingress rejects text over
1,024 UTF-16 units. The user-facing native Win32 EDIT control, however,
was still created without `EM_LIMITTEXT`, leaving the larger default
Win32 input limit active. `refresh_results` retrieves no more than
`MAX_QUERY_U16` units. Thus a user could type or paste more text than
Search actually executes, with a misleading longer visible query.

After creation and validation of the native EDIT HWND, the UI now sends
`EM_LIMITTEXT` with the shared `MAX_QUERY_U16` value. A never-shown
synthetic Windows 11 GUI regression queries the actual control using
`EM_GETLIMITTEXT`; this test first **FAILED RED** on the prior release
build with `native EDIT text limit differs from indexed-query UTF-16
budget`. It then **PASSED GREEN** after setting the limit. An additional
runtime test sends 1,025 UTF-16 `x` characters through native
`EM_REPLACESEL` (the same native limit path as paste), asserts the text
length stops at 1,024, and restores the original empty synthetic EDIT.
It neither sends desktop input nor changes focus or a visible window.

These tests address native keyboard/paste bounds but do not constitute
physical keyboard or IME acceptance: actual IME composition, Narrator,
full UI Automation, visual parity and supported taskbar SearchHost
entrypoint remain outstanding. Release remains `INVALIDATED`.

### Shell bridge accepts the final valid UTF-16 query unit

After limiting the native EDIT control to 1,024 UTF-16 units, the
optional legacy Shell bridge still calculated its append boundary as
`MAX_QUERY_U16 - 1`. As a result, a 1,023-unit query could not receive
its final valid one-unit character even though the native EDIT allowed
1,024. This affected the bridge's synthesized Unicode input but not
the external CLI/URI/IPC parsers or normal Win32 EDIT input.

`append_bridge_unicode_scalar` now checks the exact shared
`MAX_QUERY_U16` budget. The never-shown native Win32 regression creates
a 1,023-character query, sets the synthetic EDIT caret to its end, and
exercises the real Shell bridge scalar append path. The original code
**FAILED RED** with `Shell bridge rejected the final valid UTF-16 query
unit`; it **PASSED GREEN** after the boundary correction. The fixture
also confirms a 1,025th character is rejected, a complete non-BMP
Unicode surrogate pair fits when two units remain, and the bridge does
not append extra text beyond that full boundary. No desktop keyboard
hook, SendInput, foreground/focus or real user index was involved.

This is offscreen bridge/native-control regression evidence, not
physical Windows11 keyboard/IME, Narrator/full UIA, live SearchHost
entrypoint or visual acceptance. Release remains `INVALIDATED`.

### Push Explorer scopes down before global semantic result limits

In All mode, a valid Explorer `scope` was handled *after* detecting
related and natural-language queries. Those branches retrieved the
highest-ranked candidates across the whole index, capped their number,
and only then applied the `path_is_within_scope` guard. This prevented
showing out-of-scope results, but it could hide relevant files from the
selected folder if global top-N candidates were all outside the folder.

The All-mode dispatch is now centralized in `choose_all_query_route`.
When a folder scope exists, it selects `search_filtered` with the
scoped indexed-path predicate before its candidate cap. For an unscoped
request, the prior explicit-filter, related, natural-language, and
ranked routes remain in their original order. As a consequence,
scoped semantic requests currently use the scoped lexical retrieval
route rather than an unrestricted natural/related global shortlist;
relevance parity for these requests still requires further
fixture and real-world evaluation. Their final per-result containment
checks remain unchanged.

The focused `scoped_all_queries_must_push_scope_before_global_candidate_limit`
regression **FAILED RED** for a scoped related query routed to global
`Related`, and **PASSED GREEN** for related, natural, ordinary and
explicit-filter combinations after the correction. Existing nested
`path:` filter tests remain GREEN. This does not establish live Search
ranking quality or Windows 11 physical acceptance; release state stays
`INVALIDATED`.

### Preserve scoped natural-language subjects and ecosystem aliases

The scoped All-mode routing correction also exposed a relevance regression:
`search_filtered` was given the entire instruction sentence, such as
`find the file report.txt`, rather than the extracted filename. For
recognized related ecosystems (`Rust`, `Node.js`, etc.), exact lexical
matching of the ecosystem name additionally dropped related filenames
such as `cargo.toml` that contain no literal `rust` token.

For scoped All-mode requests without a separately parsed explicit filter,
`prepare_scoped_all_query` now preserves the query's semantic meaning
before the Explorer path constraint is added. Recognized ecosystem
requests become a `SearchFilters.application` predicate, using the
existing related-application alias matching for indexed file names and
paths; their full sentence is not treated as a literal file name. Other
natural-language requests reuse `query_subject` to choose the filename
or meaningful keyword. Ordinary scoped names, explicit `ext:`/`path:`
filters and unscoped dispatch stay unchanged. The final reconstructed
path containment check remains mandatory.

The `scoped_semantic_queries_preserve_intended_subject_and_related_aliases`
regression failed **RED** when the full related sentence survived as
literal search text. It passed **GREEN** after the fix, checking that a
scoped `cargo.toml` alias is allowed under `C:\Private` but not under
`C:\Public`, and verifying filename, ordinary-query, explicit-filter
and unscoped compatibility controls. The preceding scoped-routing test
remains GREEN.

**Remaining retrieval limitation:** `LiveSearchStore::search_filtered_with_root_prefix`
may still cap or sample a global candidate set before its path filter
is applied. The routing and semantic tests alone do **not** establish
exhaustive discovery of in-folder matches in large indexes; this needs
separate bounded candidate retrieval and data-level evaluation. Physical
Windows 11 visuals, keyboard/IME, Narrator/full UIA and live native
SearchHost integration are also unverified. Release stays `INVALIDATED`.

### Bounded candidate recovery for scoped name searches in the live index

The underlying `LiveSearchStore::search_filtered_with_root_prefix` first
retrieved a *globally* ranked set of at most 128–2,048 name candidates,
then applied the scoped path and attribute predicates. Even correctly
routed GUI requests therefore lost scoped hits located behind many
out-of-scope names. This is a result-recall bug, not evidence of a scope
escape: existing per-hit checks prevented out-of-scope files from being
shown.

Scoped, nonempty-text filtered searches now retain the fast ranked
candidate path. Only if it fails to supply the requested number of
in-scope results, a bounded recovery pass considers indexed records up
to the caller-provided `scan_budget` and applies the same verified
parent-chain, path-component and attribute constraints *before*
accepting additional results. The pass matches the same normalized
filename prefix as indexed search, deduplicates by file ID and stops
once the result cap is reached. Unscoped search behavior and existing
fast-path caps remain unchanged. This is bounded recovery, not an
unbounded or fully exhaustive scan; files beyond the budget may still
be missed, and frequent broad scoped queries can incur more work.

A real temporary on-disk synthetic index with 350 earlier global
`report-...` matches in `C:\Outside` and one later
`C:\Private\report-z-private.txt` reproduced the problem: the original
code returned zero hits (**RED**). After the recovery change the same
index returns the in-scope file (**GREEN**), while a second assertion
confirms a deliberately insufficient 128-record scan budget cannot
silently reach past that budget. Existing attribute-sidecar tests remain
GREEN. No user documents, live Windows index, SearchHost services or
production index files were read or altered. Physical Windows 11 GUI,
keyboard/IME, full UIA/Narrator and native Search entrypoint acceptance
remain unverified; release remains `INVALIDATED`.

### Prioritize fresh delta records in bounded live candidate scans

The bounded scoped recovery described above used `scan_live_candidates`,
which read up to `scan_budget` base index records **before** adding any
live delta upserts. In a large index, base records occupied every slot,
so recent file moves and creations from the delta could be missing from
scoped results even when the folder matched exactly. This did not cause
out-of-scope results to be displayed but could hide current files.

The bounded live candidate scanner now visits delta `Upsert` entries
first, excludes deleted/tombstoned entries, then fills the remaining
slots with base records whose file IDs have no delta override. It stops
at the requested candidate budget and retains the existing
verified-parent-chain and path containment filters. The change also
applies when the scanner is used for empty-text filtered searches,
making freshly indexed changes visible even when the base index is
larger than the candidate budget.

A new synthetic on-disk regression constructs a drive with 350 older
out-of-scope `report-a...` base records, an inserted `report-z-new.txt`
and an existing out-of-scope record moved to `C:\Private` as
`report-z-moved.txt` through delta Upserts. With a 128-record candidate
budget, the old implementation found only the new item and missed the
moved one (**RED**). The new implementation finds both (**GREEN**),
verifying their reconstructed paths are under `C:\Private`. Previous
scoped ranked-starvation and delta rename/delete tests remain GREEN.

This is bounded candidate selection, **not** full recall for delta sets
larger than the budget. The original hash-map iteration order and
unrelated-delta candidate starvation are addressed below, but
real-world latency and very-large-index recall still need evaluation;
physical Windows 11 UI/IME, Narrator/full UIA and supported taskbar
Search entrypoint acceptance remain unverified. Release stays
`INVALIDATED`.

### Stable bounded delta ordering and named-search candidate admission

The next regression revealed that `scan_live_candidates` enumerated its
live `HashMap` delta in randomized order and then stopped at the
candidate budget. Two identical scoped searches could therefore choose
different upserts when a delta held more files than the budget. The
new `bounded_delta_candidates_have_stable_file_id_order` test first
**FAILED RED** on returned IDs `[57, 64, 13, 44, 26]` rather than
`[7, 12, 13, 18, 26]`. Live upserts are now sorted by file ID before
filling bounded candidate slots; the test is **GREEN**. Sorting borrows
upsert references rather than duplicating their filenames in memory.

A separate synthetic on-disk fixture created 350 globally earlier
`report-a...` base names, 300 nonmatching delta Upserts and one newer
in-scope `C:\Private\report-z-private.txt`. Before correction, the
128-slot bounded recovery considered unrelated delta files first and
reported zero matches (**RED**) even though the desired file existed.
For scoped filename recovery only, `scan_live_candidates` now accepts
an optional normalized name prefix and filters delta Upserts *before*
allocating candidate slots. The base scan applies the same prefix check
but still examines no more than the caller's base-record budget. The
fixture now locates the required in-scope file (**GREEN**). Unscoped
and empty-text candidate scans still use the unrestricted path, and
previous move/delete and scoped-recall regressions remain GREEN.

**Boundaries:** The total number of emitted candidates is still capped.
The delta prefix pass can examine all in-memory Upserts (bounded by
`max_delta_entries`) and sorting costs CPU proportional to delta size;
performance must be benchmarked before release. When more than the
candidate limit *matching* upserts exist, later file IDs may still be
missed. No actual desktop, live user index or production SearchHost was
used. Physical Windows 11 GUI/IME, Narrator/full UIA, native taskbar
integration and release acceptance remain outstanding; package state
stays `INVALIDATED`.

### Count scoped delta candidates only after verifying their real folder

Even after normalized-name filtering, a folder-scoped query could still
lose the only in-folder match whenever more than 128 filename-matching
**out-of-scope** delta Upserts had lower file IDs. Previously, the
candidate cap was applied to those global name matches before the
verified directory scope was checked. In a real synthetic index with
250 `report-a...` Upserts in `C:\Outside` and one
`C:\Private\report-z-private.txt`, scoped query `report` returned zero
results at candidate budget 128 (**RED**).

`scan_live_candidates` now accepts an optional *already normalized*
root prefix along with the normalized filename prefix. Before either
delta or base items consume a result-candidate slot, it reconstructs
their verified parent-chain path and uses the same component-boundary
containment predicate as final result authorization. The root is
normalized once per query, and the final scope/attribute filter still
runs. The focused test passes **GREEN** with the private file present;
it additionally confirms a similarly named sibling directory
`C:\Private-old` and a broken indexed parent chain cannot enter the
scoped candidate list. Prior delta ordering, in-scope move/creation,
unrelated-delta and budget regressions remain GREEN.

**Limits:** This restores candidate quality within the bounded base
scan and in-memory delta traversal. The base scan still considers at
most `scan_budget` raw records; no exhaustive indexed subtree scan is
claimed. The delta scope filter may need parent-path reconstruction for
many matching live Upserts, adding CPU cost. High-delta-cardinality
latency and recall should be measured on isolated synthetic indexes
before physical UI acceptance. Release remains `INVALIDATED`; no live
SearchHost, production index or foreground desktop was touched.

### Synthetic large-delta latency benchmark and verified-parent reuse

An explicitly ignored, opt-in Rust test `bench_scoped_large_delta` builds
synthetic on-disk indexes with 1,024, 8,192 and 16,384 matching
out-of-folder delta Upserts, plus one target `report-z-private.txt` in
`C:\Private`. For each size, it executes nine optimized-build scoped
`report` searches, discards the first warm-up and reports median and
largest of eight wall-clock microsecond observations. Each search
asserts the correct private hit. Run with:

`cargo test -p search-core --release bench_scoped_large_delta -- --ignored --nocapture`

On the same isolated development PC, the measured **baseline** medians
were 2,363 / 11,073 / 24,217 microseconds for the three sizes; largest
observations were 3,541 / 14,232 / 26,327 microseconds. An analysis of
the hot loop showed that most matching delta files shared a parent,
whose entire verified ancestry was unnecessarily reconstructed anew for
every file before excluding it from the requested scope.

`candidate_inside_root` now memoizes a parent's *fully verified* path
for the duration of a single bounded candidate scan. Each candidate's
name is appended to the verified parent path and checked with the same
component-boundary scope predicate; the normal final reconstruction
and filter still runs before any hit is returned. Self-parent roots
and parentless records use the canonical path check. A missing,
non-directory, cyclical or over-depth parent fails closed, and the
local memo is capped at 256 parent entries to avoid unbounded memory.
Refreshing the index cannot preserve a stale per-scan memo.

The same opt-in benchmark after this change gave median 2,018 / 7,812 /
14,843 microseconds; largest observations 2,170 / 12,216 / 15,451
microseconds, respectively, with all targets found. This is a measured
single-machine synthetic improvement, approximately 15%, 29% and 39%
in medians, **not** a universal speed or p95 guarantee. The new regular
`cached_parent_scope_matches_canonical_full_path_checks` regression
compares the cached verdict against canonical `reconstruct_path`
across valid siblings, nested paths, sibling prefixes, missing and
non-directory parents, cyclic chains and self-parent roots. The existing
scope/delta regressions continue to pass. Real large-index performance,
physical Windows 11 UI/IME, Narrator/full UIA and supported native
Search entrypoint acceptance remain unverified. Release stays
`INVALIDATED`.

### Preserve fresh delta files in ordinary (unscoped) top-K search

The ordinary `LiveSearchStore::search_prefix` implementation filled its
result limit with old base-index names **before** even examining recent
delta Upserts. When the base had at least `limit` matches, newly indexed
files were invisible even if their names sorted earlier than all old
matches. This affected normal global GUI Search, not only scoped paths.
For example, with eight `report-aNN` names already in the base, a newly
created `report-000-new.txt` was omitted by `search_prefix("report", 3)`.

The new `prefix_search_should_include_recent_delta_entries` synthetic
on-disk regression first **FAILED RED** with only old base names. The
search now merges the global base candidate page and all matching live
delta Upserts **before** applying the `limit` cap. An ordered top-K map
keeps only at most `limit` hits in memory, keyed by normalized filename
and file ID for deterministic ordering. Delta tombstones and renamed
base files remain excluded; nonmatching delta names do not consume
candidate slots. The regression is **GREEN**, including an earlier base
file renamed outside the query; the existing delta rename/delete test
also remains GREEN.

This change guarantees that a lexically earlier matching live delta hit
is considered for the ordinary bounded prefix result set, but does not
establish exhaustive base-index recall beyond its existing global
candidate page. Each ordinary query may inspect up to the in-memory
delta count, bounded by configured delta retention. Real GUI input,
Windows 11 visual parity, physical IME, Narrator/full UIA and native
SearchHost integration remain unverified. Release stays `INVALIDATED`.

### Scoped delta path pruning and 64K synthetic scale check (2026-10-14)

The scoped search hot path previously reconstructed and lowercased every
child's full path even when a fully verified parent directory was an
unrelated sibling of the requested search root. For example, children
under `C:\Outside` cannot appear below `C:\Private`. The one-scan,
256-entry parent memo now caches a scope relation: already inside,
possibly on the root's ancestor chain (check the child), or definitely
outside/invalid. Only the ancestor case constructs and checks a child's
full path. The existing canonical final-result path reconstruction and
filter authorization remain unchanged. Invalid, cyclic and non-directory
parent chains remain fail-closed. The synthetic parent-cache equivalence
test now also checks nested and exact-file-like root paths.

The live candidate prefix filter now uses allocation-free ASCII-only
case-insensitive comparison while retaining `normalize_name` for all
non-ASCII names, including expanding Unicode lowercase sequences. An
additional regression compares this optimization with the canonical
normalizer across ASCII and Unicode names and query prefixes.

The opt-in `bench_scoped_large_delta` matrix was extended to 32,768 and
65,536 upserts. The 64K synthetic benchmark explicitly raises only its
isolated `open_with_limit` cap, since the normal production delta limit
rejects that many records; the production limit was **not changed**.
Eight timed runs follow a warm-up, reporting median and maximum
observations, not an estimate of population p95. On the same Windows
development machine and synthetic data, measurements were:

| Delta upserts | Baseline median (us) | Optimized median (us) | Change |
| ---: | ---: | ---: | ---: |
| 1,024 | 1,440 | 776 | -46.1% |
| 8,192 | 10,447 | 3,594 | -65.6% |
| 16,384 | 19,459 | 7,801 | -59.9% |
| 32,768 | 44,045 | 15,924 | -63.8% |
| 65,536 | 82,853 | 36,230 | -56.3% |

All benchmark invocations returned the expected in-scope target. These
figures describe a **combined** change to prefix matching and verified
parent scope pruning; they do not isolate the contribution of each
suboptimization. They are not production latency guarantees, and the
benchmark's in-memory working set and eight-sample maximum do not
establish representative real-world CPU, memory or p95 performance.
The search candidate-budget and nonexhaustive retrieval caveats remain.
Physical Windows 11 UI, IME, Narrator, multi-monitor/DPI and supported
native Search entry-point acceptance are not inferred from these tests.
`docs/RELEASE_STATE.json` remains `INVALIDATED`.
