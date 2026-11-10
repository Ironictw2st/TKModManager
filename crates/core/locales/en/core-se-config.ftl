# Script-extender features (the DLLs tab: per profile, or the default). Each setting has a
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

se-key-naval-to-land = Battles on water fought on land
se-key-naval-to-land-help = A battle between armies on a river or the sea, which the game cannot fight, is fought on the nearest land instead.

se-key-followup-respond-hook = Ultimatum answer repair
se-key-followup-respond-hook-help = Repairs ACCEPT and REJECT on follow-up offers made to you, such as the emperor's "Issue Ultimatum". Needs the mediate-peace repair on.

se-key-engine-crash-guards = Engine crash guards
se-key-engine-crash-guards-help = Guards against engine crashes seen in crash reports.

se-key-cai-null-guard = AI end-turn crash guard
se-key-cai-null-guard-help = Stops an AI end-turn crash on a missing object.

se-key-siege-force-assault = AI assaults sieges it can win
se-key-siege-force-assault-help = AI armies sitting outside a settlement assault it when the odds are good. Single player only.

se-key-siege-odds-gate = Odds-based AI siege waiting (experimental)
se-key-siege-odds-gate-help = Replaces the AI's flat 50% chance to keep waiting with one based on the odds. Known to crash: leave it off.

se-key-siege-odds-bands = Siege odds bands
se-key-siege-odds-bands-help = Used by the odds-based siege waiting: odds:chance pairs, separated by commas.

se-key-ai-skills = AI skill choices
se-key-ai-skills-help = Lets mods steer which skills AI characters pick. Does nothing until a mod sets it up.

se-key-ai-ceos = AI equipment choices
se-key-ai-ceos-help = Lets mods steer which items AI characters equip. Does nothing until a mod sets it up.

se-key-ai-court = AI court appointments
se-key-ai-court-help = Lets mods steer who the AI appoints to court posts. Does nothing until a mod sets it up.

se-key-ai-build = AI building choices
se-key-ai-build-help = Lets mods steer what the AI builds. Does nothing until a mod sets it up.

se-key-battle-xform-fast = Faster battle loading
se-key-battle-xform-fast-help = A faster version of a battle setup routine, with exactly the same results.

se-key-ceo-ui-refresh = Character panel crash guard
se-key-ceo-ui-refresh-help = Closes a character panel whose items changed (the character joined or left a faction), so clicking an old item card cannot crash.

se-key-audio = Mod music and sounds
se-key-audio-help = Lets mods play their own music and sounds.

se-key-ui-dev-loader = UI dev loader (for modders)
se-key-ui-dev-loader-help = Loads UI layouts and scripts from disk and rebuilds them on save. For mod authors; refused in multiplayer.

## Feature areas (DLLs tab)
se-area-battle = Battles
se-area-campaign-ai = Campaign AI
se-area-characters = Characters and diplomacy
se-area-economy = Economy
se-area-multiplayer = Multiplayer
se-area-stability = Stability and saves
se-area-performance = Performance
se-area-interface = Interface and sound
se-area-diagnostics = Diagnostics
se-area-menu-text = Main-menu text
