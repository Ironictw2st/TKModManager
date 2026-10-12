# Script-extender settings (Settings > Script extender, and per profile). Each setting has a
# name, a "-help" text shown as its tooltip, and for lists one line per choice ("-0", "-1"...).
# Keep { "{game}" }, { "{version}" } and { "{sync}" } exactly as they are (curly braces are
# special in these files, so a literal {game} is written that way), and keep file names such as
# se_crash.txt: they are real names the script extender uses.

se-key-build-number = Build text
se-key-build-number-help = Main-menu build text. { "{game}" } = the game's own text, { "{version}" } = script extender version, { "{sync}" } = multiplayer settings hash. The [se version.sync] tag is always added.

se-key-build-number-short = Short build text
se-key-build-number-short-help = Short form of the build text, same placeholders.

se-key-build-modified = Show the game as modified
se-key-build-modified-help = The main menu marks the game as modified.

se-key-autoresolve-hooks = Auto-resolve hooks
se-key-autoresolve-hooks-help = Master switch for every auto-resolve hook (off also disables the duel hook).

se-key-duel-power-hook = Duel power hook
se-key-duel-power-hook-help = Per-character duel bonus and stat formula in auto-resolve (190 Expanded "Duel CEO"). Off = those script calls are refused.

se-key-prebattle-single-delegate = Multiplayer: one Delegate vote for all
se-key-prebattle-single-delegate-help = Before a battle, one human's Delegate vote counts for every human.

se-key-postbattle-single-continue = Multiplayer: one Continue for all
se-key-postbattle-single-continue-help = After a battle, one human's Continue clears the screen for every human (an unmade captive choice takes the default).

se-key-save-chunking = Save chunking
se-key-save-chunking-help = Saved script values above the engine's 64 KiB limit are split into chunks. Off = vanilla (a large store resets on load).

se-key-marriage-inlaws = Relatives by marriage may marry
se-key-marriage-inlaws-help = Off = the engine rule: one marriage between two houses blocks every later one.

se-key-marriage-blood-generations = Blood-relative marriage ban (generations)
se-key-marriage-blood-generations-help = With in-law marriage on: blood relatives sharing an ancestor within this many generations may not marry. 0 = only the engine's close-kin rule.

se-key-horde-income = Horde income
se-key-horde-income-help = Adds faction- and force-scoped gdp_abs effects to the treasury.

se-key-horde-income-category = Horde income line
se-key-horde-income-category-help = Which income line receives the horde income.
se-key-horde-income-category-0 = Taxes
se-key-horde-income-category-1 = Mining (recommended)
se-key-horde-income-category-2 = Trade
se-key-horde-income-category-3 = Military force

se-key-followup-hooks = Mediate-peace button repair
se-key-followup-hooks-help = Repairs the MEDIATE PEACE diplomacy button.

se-key-ai-recruit-hook = AI recruitment hook
se-key-ai-recruit-hook-help = AI recruitment planner hook (AI scope for the recruit caches).

se-key-recruit-perm-cache = Unit permission cache
se-key-recruit-perm-cache-help = Reuses the unit-permission table: a large end-of-turn time saving.
se-key-recruit-perm-cache-0 = Off
se-key-recruit-perm-cache-1 = On
se-key-recruit-perm-cache-2 = Verify only

se-key-ai-recruit-cache = AI recruit-list cache
se-key-ai-recruit-cache-help = Diagnostic cache for AI recruit lists. Measured as not worth it: leave off.
se-key-ai-recruit-cache-0 = Off (recommended)
se-key-ai-recruit-cache-1 = Verify only
se-key-ai-recruit-cache-2 = Serve

se-key-ui-recruit-cache-ms = Recruit-list UI cache (ms)
se-key-ui-recruit-cache-ms-help = How long the recruit-list UI cache lives. 0 = off.

se-key-file-probe-cache-ms = Missing-file cache (ms)
se-key-file-probe-cache-ms-help = How long missing loose-file folders are remembered. 0 = off.

se-key-diag-crash = Crash reporter
se-key-diag-crash-help = 1 writes se_crash.txt next to the DLL when the game faults; 2 also logs every hook and native call to se_activity.txt (large).
se-key-diag-crash-0 = Off
se-key-diag-crash-1 = On (se_crash.txt)
se-key-diag-crash-2 = On + activity log

se-key-diag-diplomacy = Diplomacy diagnostics
se-key-diag-diplomacy-help = Diplomacy counters and a trace in dip_trace.txt.
se-key-diag-diplomacy-0 = Off
se-key-diag-diplomacy-1 = Counters, trace on request
se-key-diag-diplomacy-2 = Trace from injection
