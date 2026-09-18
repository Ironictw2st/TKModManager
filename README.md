# TK Mod Manager

A mod manager and launcher for **Total War: THREE KINGDOMS** that replaces the CA launcher:
profiles, real load-order control, movie packs, Workshop metadata, conflict detection, and
one-click injection of the [script extender](https://github.com/Ironictw2st/TK-ScriptExtender)
DLL. Portable single exe, self-updating.

## How it launches the game

The manager writes `tkmm_mods.txt` into the game folder and starts `Three_Kingdoms.exe`
directly with that file (Steam must be running). Workshop packs load from where Steam put them
via `add_working_directory`; nothing is copied into `data/`. Movie-type packs in `data/` that
you turn off are excluded with `exclude_pack_file`; files are never renamed or moved.

Script extender: enable it in the launch bar. After the game reaches the main menu the DLL is
injected once; the DLL verifies the game build itself and the launch bar shows the result.
A DLL built for another game build is never injected.

## Development

```
npm install
npm run tauri dev          # app with hot reload
npm test                   # frontend unit tests (vitest)
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri build --no-bundle   # portable exe -> src-tauri/target/release/tk-mod-manager.exe
```

Data lives in `%APPDATA%\TKModManager` (settings, profiles, tags/notes, Workshop cache,
DLL versions, generated options pack). See `RELEASING.md` for the update channels.
