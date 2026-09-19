# Build (and optionally run) the Qt app inside the KDE Craft Qt 6 / MSVC environment.
# Usage: powershell -ExecutionPolicy Bypass -File scripts\build.ps1 [-Release] [-Run] [-Test]
# Same recipe as RPFM's local build (Z:\Claude\GitHUb\RPFM\target\rpfm_build.ps1).
param([switch]$Release, [switch]$Run, [switch]$Test)
$ErrorActionPreference = 'Stop'

# Craft refuses to initialise if a unix 'sh' is on PATH (Git/MSYS). Strip those entries.
$env:PATH = (($env:PATH -split ';') | Where-Object {
    $_ -and ($_ -notmatch '\\Git\\') -and ($_ -notmatch 'mingw') -and ($_ -notmatch '\\usr\\bin') -and ($_ -notmatch 'msys')
}) -join ';'
# vcvarsall calls vswhere by bare name; keep its stderr out of Craft's JSON env capture.
$env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\Installer;$env:PATH"
# craftenv tries to Remove-Item env vars that may not exist; those errors are harmless.
$ErrorActionPreference = 'Continue'
& "C:/CraftRoot/Craft/craftenv.ps1" *> $null
$ErrorActionPreference = 'Stop'
# Craft's vcvars capture omits the Windows SDK bin dir (rc.exe / mt.exe).
$env:PATH = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64;$env:PATH"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Set-Location (Join-Path $PSScriptRoot '..')
$profileArgs = @(); if ($Release) { $profileArgs = @('--release') }

if ($Test) { cargo test -p tkmm_core; if ($LASTEXITCODE) { exit $LASTEXITCODE } }
cargo build -p tkmm @profileArgs
if ($LASTEXITCODE) { exit $LASTEXITCODE }
if ($Run) {
    $exe = if ($Release) { 'target\release\tkmm.exe' } else { 'target\debug\tkmm.exe' }
    Start-Process -FilePath $exe
}
exit 0
