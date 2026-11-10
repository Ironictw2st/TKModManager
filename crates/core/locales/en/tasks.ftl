# Launch panel, background tasks, and the Conflicts and Logs tabs.

## Conflicts tab

tabs-conflicts-filter = Filter paths (e.g. db/land_units)
tabs-conflicts-rescan = Rescan
tabs-conflicts-col-file = File
tabs-conflicts-col-packs = Provided by, in load order (last one wins)
tabs-conflicts-scanning = Scanning…
# First entry of the pack list.
tabs-conflicts-all = All ({ $count } shared { $count ->
        [one] file
       *[other] files
    })
tabs-conflicts-unreadable = ⚠ { $pack }: unreadable
tabs-conflicts-winner = Winner: { $pack }
tabs-conflicts-shown = { $count } shared { $count ->
        [one] file
       *[other] files
    }
tabs-conflicts-shown-pair = { $count } { $count ->
        [one] file
       *[other] files
    } { $winner } takes from { $loser }
tabs-conflicts-explain = When two mods have the same file, the game uses the copy from the mod lower in your load order. Most overlaps are on purpose (fixes and compatibility patches); the ones marked "check" are worth a look.
tabs-conflicts-view-pairs = By mod
tabs-conflicts-view-files = All files
tabs-conflicts-pairs-count = { $count } { $count ->
        [one] overlap
       *[other] overlaps
    } between mods
tabs-conflicts-none = No two enabled mods share a file.
# One card per pair of mods. $winner loads later, so the game uses its copies.
tabs-conflicts-pair-card = { $winner } replaces { $count } { $count ->
        [one] file
       *[other] files
    } from { $loser }
# Second line of a card: the main kind of file, and how much it matters.
tabs-conflicts-pair-card-sub = Mostly { $kind } · { $level }
tabs-conflicts-level-check = check
tabs-conflicts-level-fine = usually fine
tabs-conflicts-pair-title = { $winner } replaces { $count } { $count ->
        [one] file
       *[other] files
    } from { $loser }
tabs-conflicts-kind-count = { $kind }: { $count }
tabs-conflicts-pair-check = Some of these are game data or scripts. The game uses { $winner }'s version, so the matching changes from { $loser } do not apply. That is fine when { $winner } is a patch made for { $loser }; otherwise part of { $loser } may not work.
tabs-conflicts-pair-fine = These are models, art or text. The game shows { $winner }'s version; nothing breaks.
tabs-conflicts-pair-order = To use { $loser }'s version instead, move it below { $winner } in the mod list.
tabs-conflicts-pair-files = Show files
tabs-conflicts-pair-show = Show in mod list
tabs-conflicts-pair-show-tip = Selects both mods in the mod list

## Troubleshooting tab: the last game, in words

tabs-trouble-last-game = Last game
tabs-trouble-never = You have not started the game from the manager yet.
tabs-trouble-running = The game is running.
# $date = when it started, $profile = the profile it used
tabs-trouble-closed = { $date } ({ $profile }): the game closed normally.
tabs-trouble-crashed = { $date } ({ $profile }): the game crashed.
tabs-trouble-unknown = { $date } ({ $profile }): the manager did not see how the game ended.
tabs-trouble-se-on = Script extender v{ $version } was on.
tabs-trouble-se-missing = The script extender was on, but no version for your game is installed.
tabs-trouble-se-off = The script extender was off.
tabs-trouble-nothing-changed = Your mods are the same as the last time the game closed normally.
tabs-trouble-changed = { $count } { $count ->
        [one] mod changed
       *[other] mods changed
    } since the last time the game closed normally ("What changed" lists them).
tabs-trouble-what-to-do = Press Send report: it posts everything the developers need to the 190 Expanded Discord. You do not need to find any files.
tabs-trouble-changes = What changed
tabs-trouble-changes-tip = Mods added, updated or removed since the last time the game closed normally
tabs-trouble-show-logs = Log files (for modders)
tabs-trouble-show-logs-tip = The raw log files the game and the script extender write

## Logs tab

tabs-logs-filter = Filter lines (e.g. error)
tabs-logs-follow = Follow
tabs-logs-show-file = Show file
tabs-logs-placeholder = Logs appear here once the game has run: the script extender log, lua_mod_log.txt, and the newest script / ironic logs.

## Abnormal exit dialog (the game crashed or was killed)

tabs-crash-title = Abnormal exit
tabs-crash-intro = An exit code other than 0 usually means a crash, but it is also what you get when the game is closed from Task Manager.
tabs-crash-no-record = No launch record found.
# $date = when that launch started, $profile = profile name
tabs-crash-changed-since = Changed since the last clean launch ({ $date }, { $profile })
tabs-crash-no-baseline = No earlier launch that exited cleanly is on record yet.
tabs-crash-nothing-changed = Nothing: same packs, same files, same order.
tabs-crash-added = added: { $pack }
tabs-crash-file-changed = file changed: { $pack }
tabs-crash-removed = removed: { $pack }
tabs-crash-reordered = order: the load order of the common packs changed

## Launch panel (right side of the Mods tab)

launch-panel-running = Running…
launch-panel-play = ▶  Play
# Shown while Steam is still updating Workshop mods before a launch.
launch-panel-play-now = ▶  Play without waiting
launch-panel-running-tip = The game is already running
launch-panel-play-now-tip = Steam is still updating Workshop mods; start now and some may load their old version
launch-panel-play-tip = Launch { $profile } with { $count } { $count ->
        [one] mod
       *[other] mods
    }
launch-panel-summary = { $profile } · { $count } { $count ->
        [one] mod
       *[other] mods
    } enabled
# Label of the combo box that picks which installed copy of the game to use (Steam, Epic...).
launch-panel-game-copy = Copy
launch-panel-se-needed = { $count ->
        [one] { $count } enabled mod needs the script extender.
       *[other] { $count } enabled mods need the script extender.
    }
# $names = the first three mod names, $count = how many more there are
launch-panel-names-more = { $names } +{ $count } more
launch-panel-se-turn-on = Turn on script extender
launch-panel-se-settings = Script extender settings
launch-panel-coop-settings = Open Coop in the DLLs tab
launch-panel-coop-about = About Coop
launch-panel-se = Script extender
launch-panel-se-tip = Inject the script extender DLL once the main menu is up
launch-panel-se-config = Settings…
launch-panel-se-config-tip = The script extender's features for this profile (DLLs tab)
launch-panel-no-dll = no DLL
launch-panel-dll-tip = DLL that matches this game build
launch-panel-no-dll-tip = No DLL matching this game build is installed
launch-panel-se-own-defaults = Own settings: script extender defaults
# $settings = list of changed settings
launch-panel-se-own = Own settings: { $settings }
launch-panel-se-cfg-override = { $file } overrides these settings; delete or rename it to use them.
launch-panel-skip-intro = Skip intro
launch-panel-skip-intro-tip = Skip the two startup videos (generated options pack)
launch-panel-load-save = Load save
launch-panel-no-saves = (no saves)
launch-panel-main-menu = (main menu)
launch-panel-load-save-tip = Load this campaign save straight away
launch-panel-crashed = Game ended abnormally (exit code { $code })
launch-panel-crash-details = Details
launch-panel-mod-report = Mod report
launch-panel-mod-report-tip = Write a report of every loaded mod, the game's crash files and the logs to the Desktop, to send to a mod author
launch-panel-send-report = Send report
launch-panel-send-report-tip = Post the mod report and the crash dump to the 190 Expanded Discord
launch-panel-dismiss = Dismiss
# Tooltip of the launch status line; $message = current launch status.
launch-panel-status-tip = { $message } (open the Logs tab)
launch-panel-status-tip-pid = { $message } · pid { $pid } (open the Logs tab)
# Footer: $game = launch-panel-game-version or launch-panel-game-not-found
launch-panel-footer = { $game } · TK Mod Manager v{ $version }
launch-panel-game-version = Three Kingdoms { $version }
launch-panel-game-not-found = game not found

## Background tasks: Nexus Mods installs, Workshop updates, mod reports

jobs-nxm-bad-link = Cannot install from Nexus Mods: { $error }
jobs-nexus-no-key = Add your Nexus Mods API key first (Settings > Nexus Mods), then click "Mod Manager Download" again.
jobs-nexus-fetching = Nexus Mods: fetching mod { $mod_id } file { $file_id }…
jobs-nexus-downloading = Nexus Mods: downloading… { $percent }%
jobs-nexus-installing = Nexus Mods: installing { $name }…
# File dialog title, and the file type names in its filter list.
jobs-archive-title = Install a mod archive
jobs-archive-filter = Mod archives
jobs-all-files = All files
jobs-installing = Installing { $name }…
jobs-installed = Installed { $name } ({ $count } { $count ->
        [one] pack
       *[other] packs
    }).
jobs-installed-version = Installed { $name } { $version } ({ $count } { $count ->
        [one] pack
       *[other] packs
    }).
jobs-variant-title = Choose a variant
jobs-variant-text =
    This archive ships the same pack in several folders (alternative versions).
    Which one do you want?
jobs-install-failed = Install failed: { $error }
# $mods = list of "file (source)"
jobs-replaced-old = Switched off the old copy of { $mods }; the new install loads in its place.
jobs-quit-before-switch = Quit the game before switching versions.
jobs-quit-before-delete = Quit the game before deleting mod files.
jobs-delete-version-title = Delete installed version
jobs-delete-version-text = Delete { $name } from the manager's Nexus folder?
jobs-nexus-title = Nexus Mods
jobs-nexus-nothing-to-check = No Nexus mods to check (or no API key set).
jobs-nexus-all-current = Every installed Nexus mod is up to date.
jobs-nexus-newer = { $count ->
        [one] { $count } Nexus mod has a newer file (red dot in the list).
       *[other] { $count } Nexus mods have newer files (red dots in the list).
    }
jobs-nexus-check-failed = Nexus update check failed: { $error }
jobs-steam-quit-first = Quit the game first: Steam does not update Workshop items of a running game.
jobs-steam-updating = Steam: updating { $count } Workshop { $count ->
        [one] item
       *[other] items
    }…
jobs-steam-downloading = Steam: downloading { $title }… { $percent }%
# One line per Workshop item Steam could not update.
jobs-steam-item-failed = { $title }: { $message }
jobs-steam-update-title = Workshop update
jobs-steam-update-incomplete =
    Steam could not bring every Workshop mod up to date, so some may load an older version:

    { $items }

    Start anyway?
jobs-waiting-for-steam = Waiting for Steam…
jobs-force-update-failed = Force update failed: { $error }
jobs-steam-done = { $count ->
        [one] Steam: { $count } Workshop item is up to date.
       *[other] Steam: { $count } Workshop items are up to date.
    }
jobs-steam-some-failed =
    { $updated } updated, { $failed } failed:

    { $items }
jobs-report-creating = Creating mod report (asking Steam for the current versions)…
jobs-report-saved = Mod report saved: { $path }
jobs-report-failed = Cannot create the mod report: { $error }
jobs-send-report-title = Send report to Discord
jobs-send-report-text =
    This posts a new thread in the 190 Expanded Discord (a public forum) with:
    • the mod report: every loaded mod, its Workshop / Nexus version, file paths on this PC (they include your Windows user name), recent launches, the game's crash records and the ends of the game's logs
    • the newest game crash dump from the last 7 days, when it fits Discord's size limit

    What were you doing when it crashed? (optional; the first line becomes the thread title)
jobs-send-report-send = Send
jobs-report-sending = Sending report to Discord…
jobs-report-sent-title = Send report
# $result = what was sent ("Report sent." or similar)
jobs-report-sent =
    { $result }
    Thanks, the mod team can see it on Discord now.
jobs-report-send-failed =
    The report could not be sent: { $error }

    Use "Create mod report" and send the file by hand instead.
