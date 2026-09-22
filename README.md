# TK Mod Manager

A mod manager and launcher for **Total War: THREE KINGDOMS** that replaces the CA launcher:
profiles, real load-order control, movie packs, Workshop metadata, conflict detection, and
one-click injection of the [script extender](https://github.com/Ironictw2st/TK-ScriptExtender)
DLL. A native Windows app (Rust + Qt Widgets), portable and self-updating.

## Install

Download `TKModManager-x64.zip` from the latest
[release](https://github.com/Ironictw2st/TKModManager/releases/latest) or from
[Nexus Mods](https://www.nexusmods.com/totalwarthreekingdoms/mods/249), unpack it into any
folder and start `TKModManager.exe`. Updates install in place from the same zip.

## How it launches the game

The manager writes `tkmm_mods.txt` into the game folder and starts `Three_Kingdoms.exe`
directly with that file (Steam must be running). Workshop packs load from where Steam put them
via `add_working_directory`; nothing is copied into `data/`. Movie-type packs in `data/` that
you turn off are excluded with `exclude_pack_file`; files are never renamed or moved.

Script extender: enable it in the launch panel (right column). After the game reaches the main menu the DLL is
injected once; the DLL verifies the game build itself and the launch panel shows the result.
A DLL built for another game build is never injected.

## Mods from Nexus Mods

Paste your personal API key in **Settings > Nexus Mods** (the "Get my key…" button opens the
page), then click **Handle Mod Manager Download links**. The **Mod Manager Download** button on
a mod's Files tab then installs it in one click. A .zip, .7z or .rar you downloaded yourself
installs with **Actions > Install mod from archive…**.

Nexus mods are unpacked into `%APPDATA%\TKModManager\nexus` and load from there, just like
Workshop packs. The game folder is never touched. Installing another file of the same mod keeps
both copies: right-click the mod and use **Version** to switch, and the load order stays the same.
The manager checks installed Nexus mods for newer files and marks them with the red dot.

## Forcing a Workshop update

Steam downloads Workshop updates whenever it gets round to it. Right-click a mod with a red dot
and pick **Force update from Steam**, or use **Actions > Force update pending Workshop mods**,
to download it now. This runs a short helper through Valve's `steam_api64.dll`, so for a few
seconds Steam shows you as playing Three Kingdoms.

## Mod status

Each mod has a status dot. Green means up to date. Amber means it was last updated before
the current game build, which is informational. Red means Steam (or Nexus Mods) has a newer
version than the installed one. Grey means a local pack or no data. An **SE** chip marks mods that need the
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
The game never loads the `SE/` folder, so the file is harmless.

## Development

The workspace has two crates:

- `crates/core` (`tkmm_core`): everything except the window: pack scanning, profiles, groups,
  launch and injection, Workshop, updates. No GUI and no Qt needed; CI runs its tests.
- `crates/app` (`tkmm`): the Qt Widgets UI. It builds against Qt 6 (MSVC) from
  [KDE Craft](https://community.kde.org/Craft) in `C:\CraftRoot` and the ritual-generated Qt
  bindings from an RPFM checkout (the same setup RPFM uses), so it builds on a prepared Windows PC
  only.

```
cargo test -p tkmm_core                                                        # core tests
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 [-Release] [-Run]   # the app
powershell -ExecutionPolicy Bypass -File scripts\release.ps1 [-Publish]        # release zip
```

To run a dev build, put `C:\CraftRoot\bin` on `PATH` and set
`QT_PLUGIN_PATH=C:\CraftRoot\plugins`.

Data lives in `%APPDATA%\TKModManager` (settings, profiles, tags/notes, Workshop cache,
DLL versions, generated options pack). See `RELEASING.md` for releases and the update channels.
