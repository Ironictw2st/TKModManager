# Main window, tray icon and shared dialogs.

## Shared dialogs

# Buttons of the "new version available" question.
dialog-install = Install
dialog-skip-version = Skip this version
dialog-not-now = Not now
dialog-copy = Copy to clipboard
dialog-ok = OK

## Tray icon menu

tray-show = Show TK Mod Manager
# $profile = name of the active profile
tray-play = Play ({ $profile })
tray-play-profile = Play profile
tray-quit = Quit

## Main window

app-tab-mods = Mods
app-tab-dlls = DLLs
app-tab-conflicts = Conflicts
app-tab-logs = Troubleshooting
app-profile = Profile
app-profile-actions = Actions
app-profile-actions-tip = Profile actions
app-settings = Settings
app-help-tip = Help: the tour and the first-time setup
app-help-tour = Take the tour
app-help-setup = Run the first-time setup again…
# Bar under the header for people who used the manager before the tour existed.
app-whats-new = New: a DLLs tab for the script extender and Coop, clearer Conflicts and Troubleshooting pages, and a one-minute tour.
app-whats-new-tour = Take the tour
app-whats-new-close = Not now
# Shown above the mod list when the game can't be found.
app-game-not-found = Three Kingdoms was not found in Steam, Epic or Game Pass. Set the game folder in Settings.

## App updates

app-update-title = Update TK Mod Manager
# $notes = the release notes
app-update-text =
    Download and install TK Mod Manager v{ $version }?

    The app will close and restart when it is done.

    { $notes }
app-update-install = Install and restart
app-update-available = TK Mod Manager v{ $version } is available.
app-update-downloading-version = Downloading v{ $version }…
app-update-downloading = Downloading update… { $percent }%
app-update-failed = Update failed
app-update-install-failed = The update could not be installed: { $error }
app-update-check-failed = Update check failed: { $error }
app-updates-title = Updates
app-up-to-date = TK Mod Manager is up to date.

## Crash of the manager itself (shown at the next start)

app-crashed-title = TK Mod Manager crashed last time
app-crashed-text = The manager closed because of an internal error. Details are in { $path } - please attach them to a bug report.

## Launching

app-no-such-profile = No profile named "{ $name }"
app-checking-workshop = Checking Workshop mods for updates…
app-starting = Starting…
app-quit-before-switch-copy = Quit the game before switching to another copy.
# Epic: CA's own launcher has mods ticked that load on top of ours.
app-ca-ticked-title = CA's launcher has mods ticked
app-ca-ticked-text =
    CA's launcher will also load { $count } { $count ->
        [one] mod
       *[other] mods
    } of its own:

    { $mods }

    They load on top of this profile and can upset the load order. Untick them in CA's launcher for a clean run.

    Start anyway?
app-coop-title = Three Kingdoms Coop
app-coop-start-anyway =
    { $problems }

    Start anyway?
app-shadowed-title = Older copy in the data folder
app-shadowed-text =
    The game loads packs by file name, and the data folder has an older copy of { $count ->
        [one] a pack
       *[other] packs
    } this profile enables from elsewhere:

    { $files }

    The game may load the old copy instead of the new one. Delete or move it out of the data folder to be sure the newest version loads.

    Start anyway?

## Profile "Actions" menu

app-menu-new-profile = New profile…
app-menu-duplicate = Duplicate…
app-menu-rename = Rename…
app-menu-delete = Delete
app-menu-install-archive = Install mod from archive…
app-menu-check-nexus = Check Nexus mods for updates
app-menu-force-update = Force update pending Workshop mods
app-menu-create-report = Create mod report
app-menu-send-report = Send report to Discord…
app-menu-shortcut = Create desktop shortcut
app-workshop-title = Workshop
app-no-pending-workshop = No Workshop mod has a pending update.
app-shortcut-created-title = Shortcut created
app-shortcut-created =
    Shortcut created:
    { $path }
app-new-profile-title = New profile
app-duplicate-profile-title = Duplicate profile
# Suggested name for a duplicated profile.
app-profile-copy-name = { $name } copy
app-profile-name = Profile name:
app-rename-profile-title = Rename profile
app-rename-profile-name = New name:
app-delete-profile-title = Delete profile
app-delete-profile-text = Delete profile "{ $name }"?

## Remove missing entries

app-purge-title = Remove missing entries
app-purge-no-workshop = The Workshop folder was not found, so every Workshop pack looks missing right now. Set it in Settings and rescan before removing these entries.
app-purge-text =
    Remove { $count } { $count ->
        [one] entry
       *[other] entries
    } for packs that are not installed, from "{ $profile }"?

    Their place in the load order goes with them: re-subscribing adds the mod back at the bottom, disabled.
app-purge-text-all =
    Remove { $count } { $count ->
        [one] entry
       *[other] entries
    } for packs that are not installed from "{ $profile }", and { $others } more from your other profiles?

    Their place in the load order goes with them: re-subscribing adds the mod back at the bottom, disabled.
