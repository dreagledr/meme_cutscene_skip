#!/usr/bin/env pwsh
# Local release build: the same two archives CI publishes on a `v*` tag
# (.github/workflows/build.yml).
#
#   out/meme_cutscene_skip.zip           readme.txt
#                                        plugins/meme_cutscene_skip_lib.asi
#   out/meme_cutscene_skip_injector.zip  meme_cutscene_skip.exe
#
# Run:  pwsh -File build.ps1

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Set-Location $PSScriptRoot

cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "cargo build --release failed with exit code $LASTEXITCODE"
}

$release = 'target/i686-pc-windows-msvc/release'

New-Item -ItemType Directory -Force out | Out-Null

# The mod DLL, renamed to .asi and nested under plugins/ for external ASI loaders
# (no launcher, no injection) - see README.
New-Item -ItemType Directory -Force out/asi/plugins | Out-Null
Copy-Item "$release/meme_cutscene_skip_lib.dll" out/asi/plugins/meme_cutscene_skip_lib.asi
Copy-Item asi-readme.txt out/asi/readme.txt
Compress-Archive -Path out/asi/plugins, out/asi/readme.txt -DestinationPath out/meme_cutscene_skip.zip -Force

Copy-Item "$release/meme_cutscene_skip.exe" out/
Compress-Archive -Path out/meme_cutscene_skip.exe -DestinationPath out/meme_cutscene_skip_injector.zip -Force

Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($zip in Get-ChildItem -Path out -Filter *.zip) {
    Write-Host ("{0} ({1:N0} bytes)" -f $zip.Name, $zip.Length)
    $archive = [System.IO.Compression.ZipFile]::OpenRead($zip.FullName)
    $archive.Entries | ForEach-Object { Write-Host ("  " + $_.FullName + " (" + $_.Length + " bytes)") }
    $archive.Dispose()
}
