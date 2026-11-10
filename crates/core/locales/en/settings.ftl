# Settings tab, and the script extender features in the DLLs tab (se-ui-*).

## Shared

settings-change = Change…
settings-auto-detect = Auto-detect
settings-open-folder = Open folder
settings-remove = Remove
settings-check-updates = Check for updates
settings-install = Install
settings-not-found = not found
settings-checking = Checking…
settings-checking-github = Checking GitHub…
settings-install-failed = Install failed
settings-close-game-first = Close the game first
# Shown after "Latest version": $version is like "v0.28.0" or "v1.2 (2026-09-30)".
settings-latest = Latest: { $version }
settings-latest-installed = Latest: { $version } (installed)
# Release channel choices (app and script extender).
settings-channel-stable = Stable releases
settings-channel-prerelease = Include pre-releases
# Table column headers.
settings-col-version = Version
settings-col-game-build = Game build
settings-col-status = Status
settings-col-released = Released
settings-col-made-for = Made for game build
settings-col-load-order = Load order
settings-col-pack = Pack

## Game and mod folders

settings-group-folders = Game and mod folders
settings-game-copy = Game copy
settings-game-copy-tip = Every copy of the game found on this PC; pick the one to manage and launch
settings-no-copy-found = no copy found — use Change…
settings-game-folder = Game folder
# How the game folder was set: chosen by hand or found automatically.
settings-game-root-manual = { $path }  (manual)
settings-game-root-auto = { $path }  (auto)
settings-workshop-folder = Workshop folder
# Shown instead of "Workshop folder" for Epic and Game Pass copies.
settings-mods-folder = Mods folder
# $store: Steam, Epic or Game Pass.
settings-mods-folder-own = { $path }  (the game's own mods folder; { $store } has no Workshop)
settings-mods-folder-none = none — { $store } keeps mods in <game folder>\mods, which does not exist yet
settings-game-version = Game version
settings-game-version-store = { $version } ({ $store })
settings-refresh-mods = Refresh mods
settings-extra-folders = Extra mod folders
settings-add-folder = Add folder…
settings-extra-folders-hint = Packs in extra folders (for example your RPFM MyMods folder) load straight from there; nothing is copied into data/. The mod list is written to tkmm_mods.txt in the game folder; CA's used_mods.txt is only read.
settings-older-than-patch = Older than patch
settings-cutoff-tip = Workshop mods last updated before this date get the amber dot
settings-cutoff-reset = Use game build date
settings-pick-game-folder = Select the Total War THREE KINGDOMS folder
settings-pick-workshop-folder = Select the Workshop content folder (779340)
settings-pick-pack-folder = Select a folder that holds .pack files

## Behaviour

settings-group-behaviour = Behaviour
settings-tray = Keep running in the tray when the window is closed (tray menu: Play, profiles, Quit)
settings-app-updates = Check for app updates on startup
settings-app-channel-tip = Pre-release builds are only offered when you pick them here
settings-dll-updates = Check for script-extender updates on startup
settings-ws-updates = Update enabled Workshop mods through Steam before each launch
settings-ws-updates-tip = Steam and CA's launcher do this before starting the game; without it an update Steam has not downloaded yet leaves the old version in place
# Followed by a number box with the suffix below ("24 hours").
settings-cache-hours = Re-fetch Workshop titles and dates after
# Shown after the number; {" "} keeps the leading space.
settings-cache-hours-suffix = {" "}hours
settings-row-size = Mod list row size
settings-language = Language
settings-language-tip = Translations live in TRANSLATING.md on GitHub; missing text shows in English
settings-theme-note = The look follows the Windows light / dark setting.

## Nexus Mods

settings-group-nexus = Nexus Mods
settings-nexus-api-key = API key
settings-nexus-key-placeholder = paste your personal API key
settings-nexus-save = Save and check
settings-nexus-get-key = Get my key…
settings-nexus-get-key-tip = Opens nexusmods.com > Site preferences > API keys; copy the Personal API Key at the bottom
settings-nexus-no-key = No key: Nexus installs are off.
settings-nexus-key-saved = Key saved.
settings-nexus-checking = Checking the key…
settings-nexus-premium = Signed in as { $name } (Premium: installs from any Files tab button).
settings-nexus-signed-in = Signed in as { $name }. Use the "Mod Manager Download" buttons on nexusmods.com to install.
settings-nexus-links = Download links
settings-nexus-register = Handle Mod Manager Download links
settings-nexus-register-tip = Make the "Mod Manager Download" button on nexusmods.com open this app (per user, no admin rights)
settings-nexus-handled-ours = Handled by TK Mod Manager.
settings-nexus-handled-other = Handled by another program: { $command }
settings-nexus-handled-none = Not handled by any program.
settings-nexus-links-title = Nexus download links
# Followed by the command line of the program that has the links now, then settings-nexus-links-switch.
settings-nexus-links-current = Nexus download links currently open:
settings-nexus-links-switch = Switch them to TK Mod Manager? The other manager can take them back from its own settings.
settings-nexus-links-take = Make the "Mod Manager Download" button on nexusmods.com open TK Mod Manager?
settings-nexus-register-failed = Could not register the link handler: { $error }
settings-nexus-archive = Install from archive…
settings-nexus-folder = Open Nexus folder
settings-nexus-hint = Mods from Nexus are unpacked into the manager's own folder and load from there, like extra folders; the game folder is never touched. Installing another file of the same mod keeps both: switch with right-click > Version.

## Script extender (DLL)

settings-group-dll = Script extender (DLL)
# $timestamp and $size identify the game exe (hex numbers).
settings-dll-game-build = Game build { $timestamp } / { $size }. A DLL is only injected when its manifest matches these values, and never into a game that already has it loaded.
settings-no-manifest = no manifest
# Status column of the DLL lists; several are joined with " · ".
settings-status-matches = matches game
settings-status-other-build = other game build
settings-status-build-unknown = game build unknown
settings-status-installed = installed
settings-status-pinned = pinned
settings-status-pinned-blocked = pinned, but can't be injected
settings-status-will-inject = will inject
settings-dll-pin = Use this version
settings-dll-pin-tip = Pin the selected version: every launch injects it, even when a newer one is installed (rollback)
settings-dll-unpin = Always use newest
settings-dll-unpin-tip = Unpin: launches inject the newest installed version that matches the game
settings-dll-remove = Remove version
settings-dll-remove-title = Remove DLL
settings-dll-remove-question = Remove script extender { $version }?
settings-dll-log = View log
settings-dll-import = Import local DLL…
settings-dll-pick-file = Select script_extender.dll
# File type filter of the file dialog: keep "(*.dll)" as it is.
settings-dll-filter = DLL (*.dll)
settings-dll-import-title = Import DLL
settings-dll-import-version = Version of this DLL (semver, e.g. 0.28.0):
settings-dll-imported-title = Imported
settings-dll-imported = Imported { $version }; its manifest is pinned to the current game build.
settings-dll-check = Check for DLL updates
settings-dll-catalog = All versions…
settings-dll-catalog-tip = Every published script-extender release: install an older one to roll back
settings-dll-loading-versions = Loading all versions…
settings-dll-versions-failed = Could not load versions: { $error }
settings-dll-versions-count =
    { $count ->
        [one] { $count } version published
       *[other] { $count } versions published
    }
settings-dll-versions-title = Script extender versions
settings-dll-versions-intro = Every published script-extender release, newest first. Pick one and choose "Use this version" to roll back: it is downloaded if needed (you'll be asked first) and used for every launch until you choose "Always use newest". Only versions made for your game build can be injected.
# $version is like "v0.28.0".
settings-version-prerelease = { $version } (pre-release)
settings-dll-wrong-build = v{ $version } was built for a different game build, so it would never be injected. Pick a version marked "matches game".
settings-dll-wrong-build-short = v{ $version } was built for a different game build, so it would never be injected.
settings-dll-using = Using v{ $version } for every launch
settings-dll-auto = Also inject when the game was started outside the manager (while this app is running)
settings-dll-moved = The script extender's versions and features are in the DLLs tab.
settings-dll-open-tab = Open the DLLs tab
settings-dll-update-title = Script extender update
settings-dll-install-title = Install script extender
settings-dll-install-question = Download and install script extender v{ $version }?
settings-dll-will-pin = It will be used for every launch from now on, until you choose "Always use newest".
settings-dll-other-pinned = You have v{ $pinned } pinned, so v{ $pinned } stays in use until you pin this one or choose "Always use newest".
settings-dll-next-launch = If it matches this game build, it will be the version injected at the next launch.
settings-dll-downloading = Downloading v{ $version }…
settings-dll-installed-match = Installed v{ $version } (matches this game build)
settings-dll-installed-other = Installed v{ $version } (built for a different game build)
settings-dll-installed-match-pinned = Installed v{ $version } (matches this game build), used for every launch
settings-dll-installed-other-pinned = Installed v{ $version } (built for a different game build), used for every launch

## Three Kingdoms Coop

settings-group-coop = Three Kingdoms Coop (multiplayer with up to 4 players)
settings-coop-title = Three Kingdoms Coop
settings-coop-install = Install…
settings-coop-update = Update…
settings-coop-repair = Repair…
settings-coop-install-tip = Download Coop's official Setup from GitHub (you are asked first) and run it
settings-coop-steam-only = Coop only works on the Steam version 1.7.2
settings-coop-workshop = Workshop pack
settings-coop-workshop-tip = Open the tw3k_coop pack in Steam to subscribe
settings-coop-project = Project page
settings-coop-uninstall = Uninstall…
settings-coop-uninstall-tip = Opens Windows Settings > Apps; remove "Three Kingdoms Coop" there. Its uninstaller puts Steam's amd_ags_x64.dll back.
settings-coop-updates = Check for Coop updates on startup (only when Coop is installed)
settings-coop-about = Coop is a separate mod by Thom-Ernst. Its DLL loads by itself whenever the game starts, from this manager too, and runs alongside the script extender. Steam version 1.7.2 only. The manager never changes the game folder itself: installing runs Coop's own Setup.
# Status line. $summary describes the installed Coop ("v1.0 installed", ...); $pack is one of the settings-coop-pack-* lines.
settings-coop-pack-enabled = the tw3k_coop pack is enabled in this profile
settings-coop-pack-disabled = the tw3k_coop pack is installed but not enabled in this profile
settings-coop-pack-missing = the tw3k_coop pack is not installed (subscribe on the Workshop)
settings-coop-wrong-store = Not available on the { $store } copy: Coop only works on the Steam version 1.7.2.
settings-coop-inactive = Coop { $summary }, but it stays inactive: this game build is not the Steam 1.7.2 build it supports.
settings-coop-unsupported = This game build is not the Steam 1.7.2 build Coop supports.
settings-coop-active = Coop { $summary }; { $pack }.
settings-coop-broken = Coop { $summary }.
settings-coop-not-installed = Not installed; { $pack }.
settings-coop-latest-installed = Latest: { $version }, installed
settings-coop-none-found = No Coop release with a Setup was found on GitHub.
settings-coop-check-failed = Coop update check failed: { $error }
settings-coop-update-title = Three Kingdoms Coop update
settings-coop-installed-version = the installed version
# $from is a version like "v1.0", or settings-coop-installed-version.
settings-coop-update-question = Update Three Kingdoms Coop from { $from } to v{ $version }?
settings-coop-repair-question = Repair Three Kingdoms Coop with Setup v{ $version }?
settings-coop-install-question = Install Three Kingdoms Coop v{ $version }?
# $setup is the Setup file name, $repo the GitHub project (owner/name).
settings-coop-install-details = This downloads { $setup } from github.com/{ $repo } (checked against the release's SHA256SUMS.txt) and starts it. Setup asks for administrator rights, replaces amd_ags_x64.dll in your game folder (keeping Steam's original as amd_ags_x64_orig.dll) and adds tw3k_coop.dll. Remove it later from Windows Settings > Apps.
settings-coop-install-separate = Coop is a separate mod by Thom-Ernst. Every player needs the same version, plus the tw3k_coop pack from the Workshop.
settings-coop-no-game-folder = The game folder was not found, so Coop's Setup has nowhere to install.
settings-coop-close-game = Close the game first: Coop's Setup cannot replace its files while it runs.
settings-coop-downloading = Downloading { $file }…
settings-coop-setup-task = Three Kingdoms Coop Setup is running; finish it to continue…
settings-coop-setup-running = Setup is running. Follow its window (it may ask for administrator rights).
settings-coop-needs-pack = Coop also needs its pack: subscribe to tw3k_coop on the Workshop ("Workshop pack" button), then enable it in the profile you play multiplayer with.
settings-coop-install-failed = Coop install failed

## Generated mod list

settings-group-modlist = Generated mod list
settings-modlist-hint = Exactly what is written to tkmm_mods.txt at launch for the active profile (before the optional skip-intro pack).

## Profiles: import, export, co-op sync check

settings-group-profiles = Profiles: import, export and co-op sync check
settings-import-ca = Import CA launcher list…
settings-import-ca-title = Import CA launcher list
settings-import-ca-default-name = CA launcher
settings-pick-used-mods = Select used_mods.txt
# File type filter of the file dialog: keep "(*.txt)" as it is.
settings-modlist-filter = Mod list (*.txt)
settings-new-profile-name = Name for the new profile:
settings-imported =
    Imported { $count ->
        [one] { $count } mod
       *[other] { $count } mods
    } into "{ $name }"
settings-imported-not-installed =
    Imported { $count ->
        [one] { $count } mod
       *[other] { $count } mods
    } into "{ $name }"; not installed: { $missing }
settings-imported-unknown =
    Imported { $count ->
        [one] { $count } mod
       *[other] { $count } mods
    } into "{ $name }"; { $unknown } not installed (kept as missing)
settings-import-collection = Import Workshop collection…
settings-collection-title = Import Workshop collection
settings-collection-input = Collection link or id:
settings-collection-fetching = Fetching the collection from Steam…
settings-collection-default-name = Collection { $id }
settings-collection-result = "{ $name }": { $installed } of { $total } collection items are installed and enabled.
settings-not-installed-title = Not installed
settings-not-installed-intro = Subscribe to these on the Workshop, then use Refresh mods:
settings-import-text = Import from text…
settings-import-text-title = Import from text
settings-import-text-intro = Paste a profile export (one pack per line).
settings-import-text-default-name = Imported
settings-next = Next
settings-nothing-to-import = Nothing to import.
settings-se-import-title = Script extender settings
settings-se-import-differs =
    The export also carries script extender settings, and { $count ->
        [one] 1 of them differs
       *[other] { $count } of them differ
    } from your defaults:
settings-se-diff-line = { $key }: you { $mine }, them { $theirs }
settings-se-import-use = Use your partner's settings for "{ $name }"? (Needed to join each other's multiplayer lobby.)
settings-export = Export active profile (with hashes)
settings-export-title = Export profile
settings-export-intro = Send this to your co-op partner. They use "Verify against an export" to compare.
settings-hashing = Hashing the enabled packs…
# $done and $total are sizes like "1.2 GB".
settings-hashing-file = Hashing { $file } — { $done } of { $total }
settings-verify = Verify against an export…
settings-verify-title = Verify against an export
settings-verify-intro = Paste your partner's export. The enabled packs of the active profile are compared with it (file size and sha256).
settings-verify-compare = Compare

## Co-op sync check result

settings-sync-title = Co-op sync check
settings-sync-ok = In sync: same game build, packs, files, order and script extender settings.
settings-sync-not-ok = Not in sync:
settings-sync-game-differs = Game build differs
settings-sync-game-detail = Total War: THREE KINGDOMS: you { $mine }, them { $theirs }
# Load order column: positions in your list and your partner's.
settings-sync-order-both = you { $ours } / them { $theirs }
settings-sync-order-theirs = them { $theirs }
settings-sync-order-yours = you { $pos }
settings-sync-order-moved = { $order }  moved
settings-sync-only-yours = only on your side
settings-sync-se-differs = SE setting differs
settings-sync-se-detail = { $name } ({ $key }): you { $mine }, them { $theirs }
settings-sync-coop-differs = Coop differs
settings-sync-coop-detail = Three Kingdoms Coop: you { $mine }, them { $theirs }
settings-sync-game-note = Every player needs the same game build. Update the game in Steam (or Epic) on the older side, and check that nobody is on a Steam beta branch.
settings-sync-game-unknown = Your partner's export does not record the game build (made with an older TK Mod Manager), so it was not compared.
settings-sync-coop-note = Every player needs the same Three Kingdoms Coop build (Settings > Three Kingdoms Coop).
settings-sync-moved-note = The packs marked "moved" load in a different place on your side. Every player in a multiplayer campaign should load the packs in the same order.
settings-sync-adopt = Use their load order
settings-sync-adopt-tip = Reorder the active profile to match your partner's export. Separators, disabled packs and packs only you have stay where they are.
settings-sync-adopted-title = Load order
settings-sync-adopted = The active profile now loads the shared packs in your partner's order. Run "Verify against an export" again to check.
settings-sync-se-note = Script extender settings that change the campaign must be identical, or you cannot see each other's lobby. Change them in the DLLs tab, or import your partner's export.
settings-sync-se-unknown = Your partner's export has no script extender settings (made with an older TK Mod Manager), so those were not compared.

## About

settings-group-about = About
# Command-line options (--profile, --launch, --minimized) stay as they are.
settings-about = TK Mod Manager v{ $version }. Data in { $path }. Shortcuts: --profile "Name" --launch starts a profile directly; --minimized starts in the tray.
settings-open-data = Open data folder
settings-check-app = Check for app updates

## Script extender features (DLLs tab)

se-ui-use-default = Use the default settings
# $game, $version and $sync show the placeholders as typed in the file ({game} ...); "[se version.sync]" stays as it is.
se-ui-group-menu-note = Placeholders: { $game } = the game's own text, { $version } = script extender version, { $sync } = multiplayer settings hash. The [se version.sync] tag is always added.
# Tooltip line under each setting's help: the key name in script_extender.cfg and its default value.
se-ui-key-tip = { $key }  (default { $default })
se-ui-other-keys = Other keys, kept as they are (edit them with "Edit as text…"): { $keys }
se-ui-reset = Reset to defaults
se-ui-edit-text = Edit as text…
se-ui-edit-intro = One key=value per line; # starts a comment. Keys the manager does not know are kept.
