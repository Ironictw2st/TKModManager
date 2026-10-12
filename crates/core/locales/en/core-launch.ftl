# Launching the game and loading the script extender. The launch-msg-* lines appear in the
# launch bar while the game starts; $pid is the game's process id.

## Errors that stop a launch

err-game-running = Three_Kingdoms.exe is already running
err-game-folder-not-found = game folder not found; set it in Settings
err-epic-load-save = the Epic version cannot be told to load a save; start the campaign from the main menu
err-launch-no-dll = no script-extender DLL installed (Settings > Script extender)
err-launch-no-matching-dll = no installed DLL matches this game build; injection would be refused
err-epic-not-listed = Epic's launcher does not list this install; start the game from Epic once
err-open-epic = cannot open Epic's launcher: { $error }
err-no-user-folder = cannot find the game's user folder
err-user-script-aside = cannot move your user.script.txt aside: { $error }
# $file is a program file name
err-cannot-start = cannot start { $file }: { $error }

## Launch progress

launch-msg-writing = Writing mod list
launch-msg-opening-epic = Opening Epic; press Play in CA's launcher
launch-msg-started = Game started (pid { $pid })
launch-msg-started-by-ca = Game started from CA's launcher (pid { $pid })
launch-msg-relaunched-by-steam = Game relaunched by Steam (pid { $pid })
launch-msg-epic-no-game = No game started; Epic's launcher was not used or Play was never pressed
launch-msg-exited-immediately = The game exited immediately (is Steam running?)
launch-msg-waiting-menu = Waiting for the main menu…
launch-msg-window-timeout = Timed out waiting for the game window; DLL not injected
launch-msg-already-loaded = Script extender already loaded in this game; not injecting again
launch-msg-cannot-inspect = Could not inspect the game process; DLL not injected
launch-msg-injecting = Main menu up; injecting DLL
launch-msg-injected = DLL loaded; waiting for verification
launch-msg-injection-failed = Injection failed: { $error }
launch-msg-active = Script extender active
launch-msg-active-with-coop = Script extender active, alongside Three Kingdoms Coop
launch-msg-refused-coop-debug = Script extender refused to start: Three Kingdoms Coop's Debug build hooks a function it uses. Install Coop's normal Setup, or a newer script extender
launch-msg-refused = DLL refused this game build (see the Logs tab)
launch-msg-unverified = DLL loaded, no verification line in the log yet
# $code is a hexadecimal exit code such as 0xC0000005
launch-msg-crashed = Game ended abnormally (exit code { $code })
launch-msg-exited = Game exited
launch-msg-external = Game started outside the manager (pid { $pid })
launch-msg-cfg-failed = Could not write the script extender settings: { $error }
launch-msg-external-no-dll = Game detected, but no DLL matches this game build

## The injector program was blocked. $file is tkmm-inject.exe; $detail is one of launch-injector-*.

launch-msg-injector-blocked = Script extender not loaded: Windows Security (or another antivirus) blocked or removed { $file } ({ $detail }). It loads script_extender.dll into the game, which antivirus programs treat as suspicious. Restore it under Windows Security > Virus & threat protection > Protection history and allow it, or add the manager's folder as an exclusion. The game keeps running without the script extender.
launch-injector-missing = the file is missing
launch-injector-threat = flagged as a threat
launch-injector-access-denied = access denied
# $status is the program's exit status as Windows reports it
launch-injector-no-answer = it stopped without an answer, { $status }
