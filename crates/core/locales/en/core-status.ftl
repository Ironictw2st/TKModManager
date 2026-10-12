# Mod status (the coloured dot in the mod list) and script-extender requirements.

## Status names (column tooltips and section titles)

status-pending = Update pending
status-old = Older than game patch
status-ok = Up to date
status-unknown = Unknown
status-local = Local file

## Status details. Dates look like "Sep 18, 2026".

status-not-installed = Not installed
status-local-data = Local pack in data/: no Workshop version to compare
status-local-folder = Pack from an extra folder: no Workshop version to compare
status-pending-text = Update pending: Steam has the { $latest } version, { $installed } is installed. Use "Force update from Steam" to download it now.
status-no-info = No update information yet (Steam has not reported this item)
status-old-text = Last updated { $updated }, before the current game build ({ $cutoff }). It may still work fine.
status-ok-text = Up to date (updated { $updated })

## Mods installed from Nexus Mods

status-nexus-version = version { $version }
status-nexus-version-unknown = unknown version
# $newer is "version 1.2" or the file name; $installed is "version 1.1" or "unknown version".
status-nexus-pending = Update on Nexus Mods: { $newer } ({ $date }) is available, { $installed } is installed. Download it with "Mod Manager Download" on the mod's Files tab.
status-nexus-archive = Installed from an archive: no Nexus page to compare
status-nexus-ok = From Nexus Mods, { $installed }

## Script-extender versions a mod supports

se-range-between = { $min } – { $max }
se-range-min = { $min } or newer
se-range-max = up to { $max }
se-source-none = Does not use the script extender
# Joined with " · ": "Declared in SE/script_extender.json · by Ironic · version 0.28"
se-source-declared = Declared in SE/script_extender.json
se-source-author = by { $author }
se-source-version = version { $range }

## Why a launch would not meet a mod's script-extender need

se-unmet-off = The script extender is off for this profile.
se-unmet-no-dll = No script extender DLL matches this game build.
se-unmet-too-old = Needs script extender { $range }; v{ $installed } is installed.
se-unmet-too-new = Supports script extender { $range } only; v{ $installed } is installed.
se-unmet-some-too-old = Some mods need a newer script extender than v{ $installed }.
se-unmet-some-too-new = Some mods do not support script extender v{ $installed }.
