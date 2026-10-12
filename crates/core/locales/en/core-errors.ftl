# Error messages. Most are shown after "… failed:" or on their own in a message box.
# $error is the system's own error text (often in the Windows language already).
# $path is a file or folder path.

## Profiles

err-profile-name-empty = The profile name is empty
err-profile-exists = A profile named "{ $name }" already exists
err-profile-missing = No such profile
err-profile-last = Cannot delete the last profile

## Files and folders

err-create-folder = cannot create { $path }: { $error }
err-replace-folder = cannot replace { $path }: { $error }
err-copy-file = cannot copy { $path }: { $error }
err-write-file = cannot write { $path }: { $error }
err-not-a-file = { $path } is not a file
err-game-not-found = game not found

## Script-extender DLL downloads

err-no-release = no release published yet
# $file is a file name such as script_extender.dll
err-release-missing-asset = release has no { $file }
err-bad-manifest = bad manifest.json: { $error }
err-sha256-mismatch = sha256 mismatch: manifest { $expected } vs download { $actual }
err-dll-in-use = that DLL version is installed and the game is running; quit the game first
err-bad-version = bad version
err-dll-not-found-after-download = installed folder not found after download
err-version-not-semver = version must be semver (e.g. 0.15.0): { $error }
err-dll-import-failed = import failed
err-dll-remove-running = quit the game before removing a DLL version
err-dll-no-such-version = no such version
