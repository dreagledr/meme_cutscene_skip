# build.ps1 - release build, UPX compression of the launcher, package to out/
#
# Only the launcher is packed: the DLL is the payload that the launcher embeds
# (include_bytes!) at compile time and extracts to
# %LOCALAPPDATA%\meme_cutscene_skip\ at runtime. Packing the DLL in target/
# before the exe is relinked would give double packing with no gain and an extra
# unpacker inside the game. UPX is always the last step: the next `cargo build`
# overwrites the exe uncompressed (cargo does not know about packing).
#
# Keep this file ASCII-only: Windows PowerShell 5.1 reads .ps1 as ANSI (cp1251)
# unless the file has a UTF-8 BOM, so non-ASCII text breaks the parser.
#
#   pwsh build.ps1
#   pwsh build.ps1 -NoUpx
param(
    [switch]$NoUpx
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

$exe = "target/i686-pc-windows-msvc/release/meme_cutscene_skip.exe"
$dll = "target/i686-pc-windows-msvc/release/meme_cutscene_skip_lib.dll"

# Remove the exe before building: cargo relinks it and embeds the CURRENT DLL,
# and UPX will not fail with "already packed" on a second script run.
if (Test-Path $exe) { Remove-Item $exe -Force }

Write-Host "==> Building meme_cutscene_skip (release)..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

if ($NoUpx) {
    Write-Host "==> UPX skipped (-NoUpx)" -ForegroundColor Yellow
} else {
    Write-Host "==> UPX compression (launcher only)..." -ForegroundColor Cyan
    upx --best --lzma $exe
    if ($LASTEXITCODE -ne 0) { throw "UPX failed" }
}

Write-Host "==> Packaging to out/ ..." -ForegroundColor Cyan
$outDir = Join-Path $projectRoot "out"
$null = New-Item -ItemType Directory -Force $outDir
Copy-Item $exe $outDir -Force

$zipPath = Join-Path $outDir "meme_cutscene_skip.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $outDir "meme_cutscene_skip.exe") -DestinationPath $zipPath

$exeSize = [math]::Round((Get-Item $exe).Length / 1KB, 0)
$dllSize = [math]::Round((Get-Item $dll).Length / 1KB, 0)
$zipSize = [math]::Round((Get-Item $zipPath).Length / 1KB, 0)
Write-Host "==> Done: $exe ($exeSize KB; embedded DLL payload $dllSize KB)" -ForegroundColor Green
Write-Host "==> Zip:  $zipPath ($zipSize KB)" -ForegroundColor Green
