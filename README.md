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

Script extender: enable it in the launch panel (right column). After the game reaches the main menu the DLL is
injected once; the DLL verifies the game build itself and the launch panel shows the result.
A DLL built for another game build is never injected.

## Mod status

Each mod has a status dot. Green means up to date. Amber means it was last updated before
the current game build, which is informational. Red means Steam has a newer version than the
installed one. Grey means a local pack or no data. An **SE** chip marks mods that need the
script extender, and it turns red when the current launch would not provide it.

### Declaring that your mod needs the script extender

Ship this file inside your pack:

```
SE/script_extender.json
```

Its contents look like this:

```json
{
    "author": "Ironic",
    "minimum_version": 0.28,
    "maximum_version": 0.28,
    "notes": ""
}
```

A pack needs the script extender exactly when it contains this file. Every field is optional,
and a trailing comma is tolerated. Versions may be numbers or strings, such as `"0.28.1"`. They
compare as dotted versions, so `0.30` is newer than `0.28`. A maximum of `0.28` allows every
0.28.x release. The manager warns before launch when the installed DLL falls outside the range.
You can still override any mod by hand in its details pane. The game never loads the `SE/`
folder, so the file is harmless.

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
