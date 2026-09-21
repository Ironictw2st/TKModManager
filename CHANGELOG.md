# Changelog

The section for a tagged version becomes that release's notes, shown in the app's update prompt.

## [Unreleased]

### Changed
- **Unsubscribed mods disappear from the list.** A mod you unsubscribe from (or delete) no longer
  sits in the list as a struck-through "missing" row. Its place in the load order is still kept,
  so re-subscribing puts the mod back exactly where it was, still enabled - but it is out of the
  way until then. **Show missing** in the list toolbar brings the rows back when you want to see
  them, and **Remove missing** clears them out for good, from every profile.
- The list now **rescans when the window comes back to the front**, so subscribing or
  unsubscribing in Steam and switching back is enough; there is also a **Rescan** button in the
  list toolbar, next to Sort A-Z. Newly subscribed mods pick up their Workshop title straight
  away instead of showing the raw file name until the next restart.
- "N enabled", the number beside each profile and the counts in the list no longer include packs
  that are not installed. Those are skipped at launch, so the counts now say what will load.

### Fixed
- **Profiles could be lost.** Saving removed the old file before renaming the new one into place,
  which left a moment where a crash or power cut lost it entirely - and a profiles file that
  failed to load was then overwritten with an empty one. The save now replaces the file in a
  single step, and nothing is ever written over a file that could not be read.
- **The app could close itself while fetching Workshop details.** Reading a mod's "Required items"
  cut the page at a fixed length, which crashed on any page with an em dash, or a Chinese,
  Japanese or Cyrillic title near that point. Required items are also read more accurately now:
  links further down the page are no longer listed as requirements.
- A **movie pack from the Workshop loaded even when you turned it off**, if another pack from the
  same mod was on. Data-folder movie packs were already handled; Workshop ones now are too.
- The **script extender's log is read from the point of injection**, not from the top. A line from
  a previous session could report "Script extender active" when nothing had loaded, or keep
  reporting a build mismatch after the right version was installed.
- An **interrupted script-extender download** left a folder behind that was listed as an installed
  version and could be injected, even though the file was incomplete and never checked.
- Right-clicking a mod that is not installed no longer offers Enable/Disable, which used to mark
  an entry as on although it could never load.

## [0.5.0] - 2026-09-19

### Added
- **Epic and Game Pass copies.** The manager finds every copy of the game on the PC (Steam
  libraries, the Epic launcher's installs, `XboxGames`) and you pick which one to manage: a
  **Copy** box next to Play and the full list in Settings. Each copy keeps its **own profiles**,
  so Steam Workshop packs no longer show up as "missing" under Epic.
- Packs in the game's own `mods\` folder (where the Epic and Game Pass builds keep them) are
  listed and loaded, shown as `mods/`. The Workshop row is Steam-only and becomes "Mods folder"
  for the other stores.
- **Launching the Epic copy.** Epic will not let anything but its own launcher start the game, and
  CA's launcher passes its own mod list, so the profile is written to the game's
  `user.script.txt`, which the game reads on every start. The manager opens Epic, you press Play
  in CA's launcher, and it then injects as usual. The file is removed when the game exits, and if
  the app was closed mid-launch. If CA's launcher has mods of its own ticked, the manager says so
  before launching. The Epic build ignores a save passed on the command line, so "Load save" is
  refused there.
- **Mod list row size**: Compact, Normal or Large, in the list toolbar and in Settings. Normal is
  the new default and is taller than the old rows. Double-clicking a row enables or disables it.
- **Every published script-extender version** is listed by "All versions…", with its date and the
  game build it was made for. Installing an older one **pins** it, so it is injected instead of
  the newest match until you choose "Always use newest" (rollback). The installed list has the
  same buttons.
- Startup **update offers**: the app checks for app and script-extender updates as before, but now
  asks before installing anything, with **Install**, **Skip this version** and **Not now**.
  A skipped version is never offered again.
- **Pre-release channel for the app.** Pre-releases are only offered when Settings →
  Behaviour → "Include pre-releases" is picked; the stable channel stays on full releases.
- A DLL release can support several game builds through a `builds` list in its `manifest.json`
  (script extender 0.36.1 uses it for the identical Steam and Epic 1.7.2.0 builds).

### Fixed
- "Open folder" and "Show file in Explorer" opened Documents when the path contained a space.
- Nothing downloads or installs without asking first, app or DLL.

## [0.4.0] - 2026-09-18

### Changed
- **Native Qt app.** The window is now built with Qt Widgets from Rust instead of an embedded web
  page (HTML/CSS). Same layout: header over the mod list, details and launch controls on the
  right. The look follows the Windows light/dark setting, with the Windows 11 style where it is
  available and a neutral Fusion style on Windows 10.
- Groups are real tree rows: fold them with the arrow, tick the header to turn the whole group
  on or off, and drag a group to move all of its mods.
- The app now ships as a zip (`TKModManager-x64.zip`: the exe plus its Qt files) and updates in
  place from it.

### Removed
- The per-mod manual script-extender override. A mod needs the script extender only when its
  pack has `SE/script_extender.json`.

### Upgrading from 0.3.x
- 0.3.x looks for a single exe in the release and will say there is no update. Download
  `TKModManager-x64.zip` once, unpack it into a folder and start `TKModManager.exe`. Settings,
  profiles, notes and downloaded DLLs are kept (they live in `%APPDATA%\TKModManager`).

## [0.3.1] - 2026-09-18

### Changed
- Script-extender mods are now declared by one file in the pack, `SE/script_extender.json`
  (`author`, `minimum_version`, `maximum_version`, `notes`). The v0.3.0 marker file and the Lua
  scan are gone. The manual override in the details pane stays.
- The launch panel warns when the installed DLL is newer than a mod's `maximum_version`, as well
  as when it is older than `minimum_version`. The details pane shows the author and notes.

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
