# The mod list (Mods tab) and the details pane on its right.

## Mod list: column headers

mods-col-mod = Mod
mods-col-notes = Notes
mods-col-source = Source
mods-col-type = Type
mods-col-size = Size
mods-col-updated = Updated

## Mod list: row size (toolbar and Settings)

mods-density-compact = Compact rows
mods-density-normal = Normal rows
mods-density-large = Large rows
mods-density-tip = Height of the rows in the mod list (also in Settings)

## Mod list: search and filters

mods-search-placeholder = Search title / file
mods-filter-all-sources = All sources
mods-filter-workshop = Workshop
mods-filter-folders = Folders
mods-filter-nexus = Nexus
mods-filter-all-types = Mods + movies
mods-filter-mod-packs = Mod packs
mods-filter-movie-packs = Movie packs
mods-filter-enabled-and-disabled = Enabled + disabled
mods-filter-enabled = Enabled
mods-filter-disabled = Disabled
mods-filter-updated = Updated since last played
mods-filter-any-status = Any status
mods-filter-pending = Update pending
mods-filter-old = Older than game patch
mods-filter-needs-se = Needs script extender
mods-filter-any-tag = Any tag
mods-show-hidden = Show hidden
# Check box; only visible when at least one profile entry's pack is missing.
mods-show-missing = Show missing ({ $count })
mods-show-missing-tip = Show entries whose pack is not installed any more (unsubscribed or deleted). They are kept in the profile so the load order comes back if you re-subscribe.

## Mod list: toolbar buttons

mods-add-group = Add group
mods-add-group-tip = Add a separator; the mods below it form a group you can fold, toggle and drag
mods-sort = Sort A–Z
mods-sort-tip = Sort by pack file name (the CA launcher's default order), inside each group
mods-sort-confirm = Sort the load order alphabetically by pack file name (inside each group)?
mods-enabled-to-top = Enabled to top
mods-enabled-to-top-tip = Move enabled mods above disabled ones, inside each group
mods-rescan = Rescan
mods-rescan-tip = Look for added and removed packs now (also runs when the window regains focus)
mods-remove-missing = Remove missing ({ $count })…
# Button that filters the list to mods updated since the profile was last played.
mods-updated-since-played = { $count } updated since last played

## Mod list: counts under the list, joined with " · "

mods-count-enabled = { $enabled } enabled of { $total }
mods-count-shown = { $count } shown
mods-count-missing = { $count } missing
mods-count-groups-hidden = groups hidden while sorting or filtering

## Mod list: rows

# Group header row: the group's name, then how many of its mods are enabled.
mods-group-row = { $name }   ({ $enabled }/{ $count } enabled)
mods-movie-header = Movie packs: always load after every mod pack; the engine fixes their order
# Short mark in the status column for mods that need the script extender.
mods-se-mark = SE
# Tooltip of the status dot: the status name, then its explanation.
mods-status-tip = { $status }: { $text }
mods-flag-missing = missing
mods-flag-updated = updated
mods-flag-hidden = hidden
mods-type-movie = MOVIE
mods-type-mod = mod
# A Nexus file version: "1.2 · Main file · installed Sep 22, 2026".
mods-version-installed = installed { $date }

## Mod list: groups

mods-group-name = Group name:
mods-new-group = New group
mods-rename-group-title = Rename group

## Mod list: right-click menu

mods-menu-enable-group = Enable group
mods-menu-disable-group = Disable group
mods-menu-rename-group = Rename…
mods-menu-add-group-above = Add group above…
mods-menu-remove-group = Remove group (keeps its mods)
# $count is 0 when only one mod is selected, otherwise the number of selected mods.
mods-menu-enable =
    { $count ->
        [0] Enable
       *[other] Enable ({ $count })
    }
mods-menu-disable =
    { $count ->
        [0] Disable
       *[other] Disable ({ $count })
    }
mods-menu-open-workshop = Open in Workshop
mods-menu-force-update = Force update from Steam
mods-menu-open-nexus = Open on Nexus Mods
mods-menu-version = Version
mods-menu-delete-version = Delete this version…
mods-menu-show-file = Show file in Explorer
mods-menu-hide = Hide
mods-menu-unhide = Unhide
mods-menu-copy-key = Copy key
mods-menu-remove-missing = Remove missing entries ({ $count })…

## Where a pack comes from (Source column, details)

state-source-workshop = Workshop
state-source-extra-folder = Extra folder
state-source-nexus = Nexus
# The profiles file could not be read; $error says why.
state-profiles-unreadable = { $error }. Fix or move that file; nothing will be saved over it until then.

## Details pane

details-select-a-mod = Select a mod to see its details.
details-legend = The dot next to each mod shows its status: green up to date, amber last updated before the current game build, red update pending on Steam, grey local file. SE marks mods that need the script extender.
details-group-default-name = Group
details-group-summary =
    Group · { $count ->
        [one] { $count } mod
       *[other] { $count } mods
    }, { $enabled } enabled
details-group-help = Every mod below this group header, up to the next group, belongs to it. Tick the header to turn the whole group on or off, drag it to move all of its mods, and right-click it to rename or remove it (its mods stay).
details-missing-pack = Missing pack
details-missing-pack-help = This profile entry points at a pack that is no longer installed (unsubscribed or deleted). It is skipped at launch.
details-force-update = Force update from Steam
details-nexus-title = Nexus Mods
details-nexus-author = by { $author }
details-nexus-versions =
    { $count ->
        [one] { $count } version installed: right-click the mod > Version to switch.
       *[other] { $count } versions installed: right-click the mod > Version to switch.
    }
details-open-nexus = Open on Nexus Mods
details-se-needed = Needs the script extender
details-se-not-needed = No script extender needed
# $problem is a sentence about the script extender this mod needs.
details-se-unmet-when-enabled = { $problem } (applies once this mod is enabled)
details-coop-title = Three Kingdoms Coop
# $summary describes the installed Coop DLL, e.g. "v1.2 installed".
details-coop-dll = Coop DLL { $summary }.
details-coop-missing = Coop's DLL is not installed. This pack needs it for the extra lobby seats: install it from Settings > Three Kingdoms Coop.
details-coop-settings = Coop settings
details-movie-help = Movie packs load after every mod pack and cannot be ordered. Enabled = loaded; disabled = excluded through the mod list. Files are never renamed or moved.
# Another copy of the same pack file; $place is where it is (e.g. "Workshop 123456" or "data/").
details-copy-newer = { $place } (newer)
details-copy-older = { $place } (older or same)
details-same-file-elsewhere = Same file name also in: { $copies }. The game loads packs by name, so only one copy loads; enabling this one switches the others off. A copy in data/ can still win, so delete old ones from there.
details-fact-movie-pack = movie pack
details-fact-mod-pack = mod pack
details-fact-file-date = file { $date }
details-open-workshop = Open in Workshop
details-show-file = Show file
details-required-title = Required items
# State of a required Workshop item.
details-required-missing = missing
details-required-on = on
details-required-off = off
details-required-enable-all = Enable all required items
details-tags-title = Tags
details-tag-remove = Remove tag
details-tag-placeholder = Add tag…
details-tag-add = Add
details-hidden = Hidden (only shown with "Show hidden")
details-notes-title = Notes
details-notes-placeholder = Your notes for this mod
details-description-title = Description

## Right column on pages other than Mods

details-page-dlls = DLLs add things to the game that mods alone cannot, such as engine fixes and smarter AI. Choose which ones this profile uses and switch their features on or off.
details-page-dlls-tip = Changes are saved straight away and used the next time you press Play.
details-page-conflicts = Two mods sometimes change the same file. The game then uses the one lower in your load order and ignores the other's copy.
details-page-conflicts-tip = Pairs marked "check" touch game data or scripts, so part of the upper mod may not work. Art-only overlaps are usually fine.
details-page-logs = What happened the last time you played, and a button to send a report if the game crashed.
details-page-logs-tip = Send report posts everything the developers need to the 190 Expanded Discord. You do not need to find any files yourself.
details-page-settings = Folders, updates, Nexus Mods, playing together and more. Pick a section on the left.
details-page-settings-tip = Take the tour again any time with the ? button at the top.
