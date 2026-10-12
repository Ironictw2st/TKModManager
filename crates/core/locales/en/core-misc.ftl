# Messages from downloads, Steam, Nexus Mods, logs and shortcuts.

## File sizes. $size is already written with a "." decimal point, e.g. "1.5".

size-bytes = { $size } B
size-kb = { $size } KB
size-mb = { $size } MB
size-gb = { $size } GB
size-tb = { $size } TB

## Game stores. Steam, Epic and Game Pass are names and are not translated.

store-unknown = unknown store

## Desktop shortcut for a profile (its description in Windows)

shortcut-description = Play Three Kingdoms with the { $profile } profile
err-no-desktop = desktop folder not found
err-shortcut = cannot create the shortcut: { $error }

## Log sources (Logs tab)

logs-script-extender = Script extender v{ $version }
logs-script-log = Script log (newest)
logs-ironic-log = Ironic log (newest)
err-log-outside = that file is outside the game folder and the DLL folder

## Downloads and updates

err-network = network error: { $error }
# $status is an HTTP status such as "404 Not Found"
err-github-status = GitHub API returned { $status }
err-download = download failed: { $error }
err-download-status = download returned { $status }
err-replace-file = cannot replace { $path }: { $error }
err-update-no-exe = the update zip contains no executable
err-invalid-json = { $path }: invalid JSON: { $error }

## Archives (zip, 7z, rar)

err-not-archive = { $path } is not a zip, 7z or rar archive
err-bad-archive = damaged archive: { $error }
# $name is a path inside the archive that would land outside the target folder
err-unsafe-path = unsafe path in archive: { $name }

## Steam: "Force update" downloads (one message per Workshop item)

steam-not-running = Steam is not running (start Steam and try again)
steam-out-of-date = the Steam client is out of date
steam-init-failed = Steam API init failed ({ $code })
steam-no-ugc = the Steam client does not offer the Workshop interface this build needs
steam-download-refused = Steam refused the download (is the item still subscribed?)
steam-download-timeout = timed out; Steam may still finish it in the background
steam-download-updated = updated
steam-download-current = already up to date
steam-not-subscribed = you are not subscribed to this item (unsubscribed or deleted?)
# $state is a hexadecimal number
steam-download-unfinished = Steam did not finish the download (item state { $state })
err-steam-api = Steam API: { $error }

## Workshop collections

err-collection-link = that does not look like a Workshop collection link or id
err-collection-empty = the collection is empty, private, or not a collection

## Nexus Mods

err-nxm-not-link = not an nxm:// link
err-nxm-unrecognised = unrecognised nxm link: { $url }
# $variants is a comma-separated list of folder names in the archive
err-nexus-variants = the archive has several variants: { $variants }
err-nexus-no-packs = the archive contains no mod packs
err-nexus-no-packs-in = the archive has no packs in { $folder }
err-nexus-no-key = no Nexus API key: add yours in Settings > Nexus Mods
err-nexus-key-rejected = Nexus Mods rejected the API key (check it in Settings > Nexus Mods)
err-nexus-premium-only = Nexus Mods only gives direct downloads to Premium members. Use the "Mod Manager Download" button on the mod's Files tab instead.
err-nexus-not-found = not found on Nexus Mods ({ $path })
err-nexus-rate-limit = Nexus Mods rate limit reached; try again later
err-nexus-status = Nexus Mods returned { $code }: { $message }
err-nexus-no-link = Nexus Mods returned no download link

## Co-op sync check: state of each pack compared with the other player's export

sync-identical = identical
sync-not-installed = not installed
sync-disabled = installed, not enabled
sync-different = different file
sync-unverified = present (no hash)
