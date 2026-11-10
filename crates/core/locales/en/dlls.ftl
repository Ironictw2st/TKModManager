# The DLLs tab: the DLLs the manager injects into the game, and the script extender's features.

## Cards (left column)

dlls-se-name = Script extender
# Second line of the script extender's card. $version = the version that would be injected.
dlls-card-on = On · v{ $version }
dlls-card-off = Off for this profile · v{ $version }
dlls-card-not-installed = Not installed

## Script extender details

dlls-se-about = The all-in-one script extender for Three Kingdoms: engine crash fixes, AI improvements and tools that mods build on. Switch any of its features off below.
dlls-se-status-on = v{ $version } will be injected when this profile starts the game.
dlls-se-status-off = Off for this profile (v{ $version } is installed).
dlls-se-status-no-match = No installed version was made for your game build. Install the latest below.
dlls-se-status-none = Not installed yet. Install it below.
dlls-se-use = Use the script extender with this profile
dlls-se-versions = Versions
dlls-se-features = Features

## Features

dlls-se-target = Settings for
# $profile = name of the active profile
dlls-se-target-profile = This profile ({ $profile })
dlls-se-target-default = Every profile without its own settings
dlls-se-target-tip = A profile can have its own settings or follow the default ones.
dlls-se-use-default-tip = Untick to give this profile its own settings, starting from the default ones.
dlls-se-summary-uses-default = This profile follows the default settings. Untick "Use the default settings" to change them for this profile only.
dlls-se-summary-all-default = Everything is at the script extender's defaults. Changes are saved at once and used at the next launch.
# $keys = the names of the changed settings
dlls-se-summary-changed = Changed from the defaults: { $keys }. Changes are saved at once and used at the next launch.
# Added after the help text of settings that change the campaign itself.
dlls-se-must-match = In co-op both players need the same setting.
dlls-se-reset-question = Set every feature back to the script extender's defaults?
