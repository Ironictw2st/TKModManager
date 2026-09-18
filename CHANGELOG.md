# Changelog

The section for a tagged version becomes that release's notes, shown in the app's update prompt.

## [Unreleased]

## [0.3.0] - 2026-09-18

### Added
- **Mod status at a glance**: a status dot next to every mod. Green = up to date, amber = last
  updated before the current game build, red = update pending (Steam has a newer version than the
  one installed, read from Steam's own Workshop manifest, so it works offline), grey = local file
  or no data. Filter and sort by status; the details pane explains each state.
- **Script extender requirement**: an "SE" chip on mods that need the script extender. Detected
  from a marker file (`script/tkmm/requires_script_extender`, optional `min_version=`) or from
  Lua that calls the `se.*` API; override per mod in the details pane. The chip turns red when
  this launch would not provide it.
- **Launch warnings**: the launch panel lists enabled mods whose script-extender need is unmet
  (turned off, no DLL for this game build, or DLL older than required) with a one-click fix.
- Setting for the "older than game patch" date (defaults to the game exe's build date).

### Changed
- New layout: tabs, profile and a Settings button above the mod list; mod details and the launch
  controls (Play, Script extender, Skip intro, Load save) share the right-hand column.
- Profile actions moved into a compact "..." menu; the filter toolbar wraps instead of scrolling.

## [0.2.0] - 2026-09-18

### Added
- **Extra mod folders**: register any folder (for example RPFM MyMods) in Settings; its packs
  are listed and loaded in place, nothing is copied into data/.
- **Groups**: separators in the load order head a group you can name, fold, enable or disable
  as a whole, and drag as a block. Sort A-Z and Enabled-to-top now work inside each group.
- **Logs tab**: live view of the script extender log, lua_mod_log.txt and the newest script /
  ironic logs, with a line filter.
- **Crash helper**: launches are recorded with their exit code; after an abnormal exit a
  Details view shows what changed since the last clean launch plus the last log lines.
- **Co-op sync check**: profile exports carry size and sha256 per pack; paste a partner's
  export to see which packs are missing, different, disabled or out of order.
- **Update tracking**: "updated" badges and filter for enabled mods changed since the profile
  was last played; a note on mods last updated before the current game build; one-click
  "Enable all" for required items.
- **Workshop collections**: import a collection link as a profile; items you lack are listed.
- **Script extender**: editor for script_extender.cfg (main-menu build text), optional
  auto-inject when the game was started outside the manager, a pre-release channel, and a
  guard that never injects into a game that already has the DLL loaded.
- **Shortcuts and tray**: `--profile "Name" --launch` / `--minimized`, a Shortcut button that
  puts a per-profile launcher on the desktop, single-instance handling, and an optional tray
  icon with Play and a profile menu.

### Fixed
- Confirmations did nothing (Sort A-Z, delete profile, remove DLL): the webview's built-in
  confirm/prompt are unreliable under Tauri, so the app now uses its own dialogs.
- Drag-and-drop helper nodes no longer sit inside the table element.

## [0.1.1] - 2026-09-18

### Fixed
- No more command prompt windows flashing on every action: the Steam registry lookup now runs
  hidden, and the detected game paths are cached instead of being re-detected per command
  (which also makes every action faster).

## [0.1.0] - 2026-09-17

### Added
- Mod list of every Workshop and `data/` pack for Total War: THREE KINGDOMS, including MOVIE
  packs, with search, filters, column sorting, tags, notes and hidden packs.
- Profiles: ordered enable lists with drag-and-drop / Alt+Up/Down reordering, multi-select,
  "Sort A→Z" and "Enabled to top"; duplicate, rename, delete; import the CA launcher's
  `used_mods.txt`; export / import a profile as text.
- Launch without the CA launcher: writes `tkmm_mods.txt` in the game folder and starts the exe
  directly (Workshop packs load in place; data/ movie packs are toggled with
  `exclude_pack_file`, nothing is renamed or moved).
- Script extender: keeps DLL versions under `%APPDATA%\TKModManager\dll`, matches them to the
  installed game build by PE fingerprint, downloads releases from GitHub (sha256 verified),
  injects after the main menu and reports the DLL's verification result.
- Workshop metadata (titles, dates, required items) from Steam, seeded from the CA launcher
  cache when offline; conflict view of files shared between enabled packs.
- Skip-intro option (generated options pack). Portable exe with in-app self-update.
