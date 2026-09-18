# Changelog

The section for a tagged version becomes that release's notes, shown in the app's update prompt.

## [Unreleased]

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
