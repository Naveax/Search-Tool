# Windows 11 hidden Search Tool GUI fixture

This is a six-record **synthetic** read-only index created for the
independent Search Tool Win32 GUI test. Its C: volume and `C:\Users\Demo`
file paths are fabricated. Three names match `SearchTool`:

- SearchTool Notes.md
- SearchTool Design.png
- SearchTool Example.txt

Only the five required index files are committed; the original transient
`.publish.lock` is deliberately excluded. Tests must never try to open
these nonexistent files or treat this index as an installed service index.

Run on an appropriate Windows 11 system after a Release build:

```powershell
pwsh -NoProfile -File .github/scripts/windows-gui-hidden-regression.ps1
```

The opt-in `--ui-selftest` mode creates a **hidden** window, uses the actual
EDIT, LISTBOX and details HWNDs, and explicitly delivers `EN_CHANGE` and
`LBN_SELCHANGE` messages. It checks search -> selection -> clear -> no-match
-> repopulation, stale detail clearing, and that the parent never becomes
visible. Its timeout is bounded. It does **not** establish interactive typing,
IME, focus navigation, screen reader, SearchHost or taskbar Search parity.
