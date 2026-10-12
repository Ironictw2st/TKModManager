# Three Kingdoms Coop (the 3-4 player multiplayer mod). "Setup" is Coop's own installer.

## One-line state, shown in Settings and the mod details. $tag is the version, e.g.
## "0.9.0" or "0.9.0 (Debug build)"; $has_tag is "yes" when there is one, otherwise "no".

coop-debug-build = { $version } (Debug build)
coop-summary-installed = { $has_tag ->
    [yes] installed { $tag }
   *[no] installed
}
coop-summary-proxy-reset = { $has_tag ->
    [yes] installed { $tag }, but Steam put the original amd_ags_x64.dll back: run Coop's Setup again
   *[no] installed, but Steam put the original amd_ags_x64.dll back: run Coop's Setup again
}
coop-summary-runtime-missing = { $has_tag ->
    [yes] proxy present { $tag }, but tw3k_coop.dll is missing: run Coop's Setup again
   *[no] proxy present, but tw3k_coop.dll is missing: run Coop's Setup again
}

## Problems shown before a launch and in the launch panel

coop-problem-pack-without-runtime = The tw3k_coop pack is enabled, but Three Kingdoms Coop's DLL is not installed. Without it the extra lobby seats do not work.
coop-problem-proxy-reset = Steam put the original amd_ags_x64.dll back (game update or file verification), so Three Kingdoms Coop will not load. Run Coop's Setup again.
coop-problem-unsupported-build = Three Kingdoms Coop only works on the Steam version 1.7.2. It is installed here but will stay inactive.
coop-problem-debug-with-se = Three Kingdoms Coop's Debug build hooks the battle vote function the script extender also uses. Older script extenders refuse to start with it; newer ones only turn off "one Delegate vote counts for every human". Install Coop's normal (release) Setup to keep both.

## After Coop's Setup has run

coop-setup-finished = Setup finished.
coop-setup-cancelled = Setup was cancelled; nothing was changed.
coop-setup-stopped = Setup stopped before installing (see its message); nothing was changed.
coop-setup-restart = Setup finished, but Windows needs a restart to complete it.
coop-setup-failed = Setup did not finish. Its own log and message explain why.

## Errors

err-coop-no-sums = this Coop release has no SHA256SUMS.txt, so its Setup cannot be verified
# $file is the Setup file name
err-coop-sums-no-line = SHA256SUMS.txt has no line for { $file }
err-coop-sha256 = { $file } failed its SHA256 check (expected { $expected }, got { $actual })
err-coop-game-running = Close the game first: Coop's Setup cannot replace its files while it runs.
err-coop-setup-declined = Setup was not started (administrator permission was declined).
err-coop-setup-start = Could not start Setup: { $error }
