# Build the portable release zip and (optionally) publish it as a GitHub release.
# Usage: powershell -ExecutionPolicy Bypass -File scripts\release.ps1 [-Publish] [-PreRelease]
#
#   1. cargo build --release -p tkmm inside the KDE Craft Qt 6 / MSVC environment
#   2. release\TKModManager\ = TKModManager.exe + windeployqt output + the non-Qt DLLs Craft's Qt
#      links against (found by walking `dumpbin /dependents`) + the MSVC runtime
#   3. smoke test: the staged exe must start with a PATH that has no Craft or Qt in it
#   4. release\TKModManager-x64.zip (files at the zip root: the updater unpacks it over the
#      install folder)
#   5. -Publish: gh release create v<version> with the CHANGELOG section as notes
#      -PreRelease: publish it as a GitHub pre-release, which the app offers only to users who
#      picked "Include pre-releases" in Settings (everyone else stays on the last full release)
param([switch]$Publish, [switch]$PreRelease)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $root

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "TK Mod Manager $version"

# Changelog section for this version (also checks it exists before building anything).
$lines = Get-Content CHANGELOG.md
$start = ($lines | Select-String -Pattern "^## \[$([regex]::Escape($version))\]" | Select-Object -First 1).LineNumber
if (-not $start) { throw "CHANGELOG.md has no '## [$version]' section" }
$notes = New-Object System.Collections.Generic.List[string]
for ($i = $start; $i -lt $lines.Count -and $lines[$i] -notmatch '^## '; $i++) { $notes.Add($lines[$i]) }
$notesFile = Join-Path $env:TEMP "tkmm-notes-$version.md"
[System.IO.File]::WriteAllText($notesFile, (($notes -join "`n").Trim() + "`n"), (New-Object System.Text.UTF8Encoding $false))

# --- build (same environment as scripts\build.ps1) ---
$cleanPath = $env:PATH
$env:PATH = (($env:PATH -split ';') | Where-Object {
    $_ -and ($_ -notmatch '\\Git\\') -and ($_ -notmatch 'mingw') -and ($_ -notmatch '\\usr\\bin') -and ($_ -notmatch 'msys')
}) -join ';'
$env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\Installer;$env:PATH"
$ErrorActionPreference = 'Continue'
& "C:/CraftRoot/Craft/craftenv.ps1" *> $null
$ErrorActionPreference = 'Stop'
$env:PATH = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64;$env:PATH"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location $root

cargo test -p tkmm_core --quiet
if ($LASTEXITCODE) { throw "core tests failed" }
cargo build --release -p tkmm
if ($LASTEXITCODE) { throw "release build failed" }

# --- stage ---
$stage = Join-Path $root 'release\TKModManager'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory $stage | Out-Null
$exe = Join-Path $stage 'TKModManager.exe'
Copy-Item target\release\tkmm.exe $exe

& C:\CraftRoot\bin\windeployqt6.exe --release --no-translations --no-system-d3d-compiler --no-opengl-sw --no-quick-import --compiler-runtime --dir $stage $exe | Out-Null
if ($LASTEXITCODE) { throw "windeployqt failed" }
Get-ChildItem $stage -Filter 'vc_redist*.exe' | Remove-Item -Force
# Image formats: previews and icons are PNG (built in), JPEG, GIF, WebP and ICO. Craft's KDE
# plugins (kimg_*) would pull in ffmpeg, x265, JPEG XL and more for nothing.
Get-ChildItem (Join-Path $stage 'imageformats') -File | Where-Object { $_.BaseName -notin @('qjpeg', 'qgif', 'qwebp', 'qico') } | Remove-Item -Force
# Widgets draw with the raster engine and the app does its networking in Rust: no D3D shader
# compiler, Qt Network (nothing links it), TLS, network-information or touch plugins.
foreach ($x in 'dxcompiler.dll', 'dxil.dll', 'Qt6Network.dll', 'tls', 'networkinformation', 'generic') {
    $p = Join-Path $stage $x
    if (Test-Path $p) { Remove-Item $p -Recurse -Force }
}

# Non-Qt dependencies from Craft (ICU, zlib, pcre2, harfbuzz, freetype, png, ...), transitively.
$craftBin = 'C:\CraftRoot\bin'
$seen = @{}
$queue = New-Object System.Collections.Generic.Queue[string]
Get-ChildItem $stage -Recurse -Include *.exe, *.dll | ForEach-Object { $queue.Enqueue($_.FullName); $seen[$_.Name.ToLower()] = $true }
while ($queue.Count) {
    $f = $queue.Dequeue()
    $deps = & dumpbin /nologo /dependents $f | Where-Object { $_ -match '^\s+(\S+\.dll)\s*$' } | ForEach-Object { $Matches[1] }
    foreach ($d in $deps) {
        $k = $d.ToLower()
        if ($seen[$k]) { continue }
        $seen[$k] = $true
        $src = Join-Path $craftBin $d
        if (Test-Path $src) {
            Copy-Item $src $stage
            $queue.Enqueue((Join-Path $stage $d))
        }
    }
}
# MSVC runtime next to the exe, so no redistributable install is needed.
$crt = Get-ChildItem "$env:VCToolsRedistDir\x64\Microsoft.VC*.CRT" -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
if ($crt) { Copy-Item "$($crt.FullName)\*.dll" $stage -Force } else { Write-Warning "MSVC runtime folder not found (VCToolsRedistDir=$env:VCToolsRedistDir)" }

# --- smoke test with a clean PATH ---
$env:PATH = ($cleanPath -split ';' | Where-Object { $_ -and $_ -notmatch 'CraftRoot' -and $_ -notmatch '\\Qt\\' }) -join ';'
Remove-Item Env:QT_PLUGIN_PATH -ErrorAction SilentlyContinue
Remove-Item Env:QT_QPA_PLATFORM_PLUGIN_PATH -ErrorAction SilentlyContinue
$running = Get-Process TKModManager, tkmm -ErrorAction SilentlyContinue
if ($running) {
    Write-Warning "A copy of TK Mod Manager is running; skipping the start-up smoke test (it would hand over to that copy)."
} else {
    $p = Start-Process -FilePath $exe -ArgumentList '--minimized' -PassThru
    Start-Sleep -Seconds 6
    if ($p.HasExited) { throw "staged exe exited on start (code $($p.ExitCode)); a DLL or plugin is probably missing" }
    Stop-Process -Id $p.Id -Force
    Write-Host "smoke test ok"
}

# --- zip ---
$zip = Join-Path $root 'release\TKModManager-x64.zip'
if (Test-Path $zip) { Remove-Item $zip -Force }
# Not Compress-Archive: Windows PowerShell 5.1 writes '\' into entry names.
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$za = [System.IO.Compression.ZipFile]::Open($zip, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($f in Get-ChildItem $stage -Recurse -File) {
        $name = $f.FullName.Substring($stage.Length + 1).Replace('\', '/')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($za, $f.FullName, $name, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally { $za.Dispose() }
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "built $zip ($size MB, $((Get-ChildItem $stage -Recurse -File).Count) files)"

if ($Publish) {
    $env:PATH = $cleanPath
    $args = @("v$version", $zip, "--repo", "Ironictw2st/TKModManager", "--title", "v$version", "--notes-file", $notesFile)
    if ($PreRelease) { $args += "--prerelease" }
    gh release create @args
    if ($LASTEXITCODE) { throw "gh release create failed" }
    Write-Host "published v$version$(if ($PreRelease) { ' (pre-release: only offered to users who opted in)' })"
}
