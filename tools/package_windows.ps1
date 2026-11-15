# Build the Section 11 standalone distribution for Windows.
#
# Produces dist\HITMAN Coral Palm Resort\ containing:
#   HITMAN Coral Palm Resort.exe   the game
#   assets\                        every texture, shader and data file
#   README.txt                     controls, requirements, save locations
#
# The folder is fully portable: it can be moved anywhere, needs no installer,
# no admin rights, no Rust toolchain and no internet connection. Assets resolve
# relative to the executable (see src/main.rs), so a double-click always works.
#
# Usage:  pwsh -File tools/package_windows.ps1 [-SkipBuild]

param(
    [switch]$SkipBuild,
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$name = "HITMAN Coral Palm Resort"
$dist = Join-Path $root "dist"
$app  = Join-Path $dist $name

if (-not $SkipBuild) {
    Write-Host "Building ($Profile)..."
    cargo build --profile $Profile --bin hitman_lowpoly
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$exe = Join-Path $root "target\$Profile\hitman_lowpoly.exe"
if (-not (Test-Path $exe)) { throw "missing $exe" }

if (Test-Path $app) { Remove-Item $app -Recurse -Force }
New-Item -ItemType Directory -Path $app -Force | Out-Null

Copy-Item $exe (Join-Path $app "HITMAN Coral Palm Resort.exe")
Copy-Item (Join-Path $root "assets") (Join-Path $app "assets") -Recurse
Copy-Item (Join-Path $PSScriptRoot "DISTRIBUTION_README.txt") (Join-Path $app "README.txt")

# Strip anything the game never reads at runtime so the download stays small.
Get-ChildItem (Join-Path $app "assets") -Recurse -Include *.meta,*.txt.bak |
    Remove-Item -Force -ErrorAction SilentlyContinue

$size = [math]::Round(((Get-ChildItem $app -Recurse | Measure-Object Length -Sum).Sum / 1MB), 1)
Write-Host "Packaged $name ($size MB):"
Get-ChildItem $app | Format-Table Name, Length

Compress-Archive -Path (Join-Path $app "*") -DestinationPath (Join-Path $dist "$name-windows-x86_64.zip") -Force
Write-Host "Wrote dist\$name-windows-x86_64.zip"
