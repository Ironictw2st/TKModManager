# Releasing & auto-update

TK Mod Manager ships as a **portable Windows exe** and self-updates from this repo's GitHub
Releases: the running app checks the latest release, downloads `TKModManager-x64.exe`, swaps
itself in place and relaunches. No installer, no signing key.

## Cutting a release

1. Bump the version (same semver) in `package.json`, `src-tauri/tauri.conf.json`,
   `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` (the `tk-mod-manager` entry; `cargo check`
   updates it).
2. Move the `Unreleased` items in `CHANGELOG.md` into a `## [x.y.z] - <date>` section (the
   release workflow uses it as the release notes).
3. Commit and push to `master`. The **Release** workflow creates the `v<version>` tag, builds
   with `tauri build --no-bundle` and publishes the exe. (Pushing a `v*` tag by hand also works.)
4. Older copies show the update banner on start (or Settings → About → Check for app updates).

## How the updater works

- `src-tauri/src/update.rs`: `check_update` queries `releases/latest`, compares the tag to
  `CARGO_PKG_VERSION`; `install_update` streams the exe (emitting `update-progress`), replaces
  the running binary with the `self-replace` crate and restarts. Debug builds never update.
- `src/updater.ts` wraps the commands; `src/panels/UpdateBanner.tsx` is the startup prompt and
  Settings → About has the manual check.

## The script-extender DLL channel

`src-tauri/src/dll.rs` reads the latest release of `Ironictw2st/TK-ScriptExtender`, which must
publish `script_extender.dll` and `manifest.json`
(`{ version, game_exe_version, exe_timestamp, exe_size_of_image, sha256 }`). The manifest's
fingerprint must equal the installed exe's PE `TimeDateStamp` / `SizeOfImage`, otherwise the
DLL is listed as "other build" and never injected.

## Notes

- WebView2 Runtime must be present (it is on current Windows 10/11).
- Without an Authenticode certificate SmartScreen may warn the first time the exe runs.
- The GitHub API check is unauthenticated (60 requests/hour/IP).
