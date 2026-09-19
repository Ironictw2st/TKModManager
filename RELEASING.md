# Releasing & auto-update

TK Mod Manager ships as a **portable zip**, `TKModManager-x64.zip`: `TKModManager.exe`, the Qt 6
DLLs and plugins it needs, their Craft dependencies and the MSVC runtime. The running app checks
this repo's latest GitHub release, downloads the zip and installs it over its own folder, then
restarts. No installer, no signing key.

## Cutting a release

The Qt app builds on the maintainer's PC only (KDE Craft Qt in `C:\CraftRoot` plus the
ritual-generated bindings from the RPFM checkout), so releases are made locally, not by CI.

1. Bump `version` in the root `Cargo.toml` (`[workspace.package]`); `cargo check` updates
   `Cargo.lock`.
2. Move the `Unreleased` items in `CHANGELOG.md` into a `## [x.y.z] - <date>` section. That
   section becomes the release notes and the text of the in-app update prompt.
3. Commit and push to `master` (CI runs the `tkmm_core` tests).
4. `powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Publish`
   - runs the core tests and `cargo build --release -p tkmm` in the Craft environment;
   - stages `release\TKModManager\` with `windeployqt6`, prunes plugins the app doesn't use,
     copies the non-Qt DLLs Craft's Qt links against (walking `dumpbin /dependents`) and the
     MSVC runtime;
   - starts the staged exe with Craft removed from `PATH` as a smoke test;
   - writes `release\TKModManager-x64.zip` (files at the zip root) and runs
     `gh release create v<version>` with the changelog section as notes.

   Without `-Publish` it stops after building the zip.

## How the updater works

`crates/core/src/update.rs`:

- `check_update` reads `releases/latest` and compares its tag with `CARGO_PKG_VERSION`. Debug
  builds never update.
- `install_update` downloads the zip, extracts it to `update.staging\` next to the exe, then
  for every file renames the existing one to `*.old` (Windows allows renaming a running exe and
  loaded DLLs) and moves the new one in. The renamed files are listed in `update.old.txt`.
  The app then restarts.
- On the next start `cleanup_old_files` deletes the files listed in `update.old.txt` and any
  leftover `update.staging\`; other files in the folder are never touched.

Copies older than 0.4.0 look for a single `TKModManager-x64.exe` asset and report that there is
no update; they need the zip installed once by hand.

## The script-extender DLL channel

`crates/core/src/dll.rs` reads the releases of `Ironictw2st/TK-ScriptExtender`, which must
publish `script_extender.dll` and `manifest.json`
(`{ version, game_exe_version, exe_timestamp, exe_size_of_image, sha256 }`). The manifest's
fingerprint must equal the installed exe's PE `TimeDateStamp` / `SizeOfImage`, otherwise the
DLL is listed as "other build" and never injected.

## Notes

- Without an Authenticode certificate SmartScreen may warn the first time the exe runs.
- The GitHub API check is unauthenticated (60 requests/hour/IP).
