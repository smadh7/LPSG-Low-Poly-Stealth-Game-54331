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

param(
    [switch]$SkipBuild,
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$name = "HITMAN Coral Palm Resort"
$dist = Join-Path $root "dist"
$app = Join-Path $dist $name

if (-not $SkipBuild) {
    Write-Host "Building ($Profile)..."
    cargo build --profile $Profile --bin hitman_lowpoly
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed with $LASTEXITCODE" }
}

$exe = Join-Path $root "target\$Profile\hitman_lowpoly.exe"
if (-not (Test-Path $exe)) { throw "missing $exe" }

if (Test-Path $app) { Remove-Item $app -Recurse -Force }
New-Item -ItemType Directory -Path $app -Force | Out-Null

Copy-Item -LiteralPath $exe -Destination (Join-Path $app "HITMAN Coral Palm Resort.exe")
Copy-Item -Path (Join-Path $root "assets") -Destination (Join-Path $app "assets") -Recurse
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "DISTRIBUTION_README.txt") `
          -Destination (Join-Path $app "README.txt")

# A shipped build has no .meta files; make sure none slipped in.
Get-ChildItem -Path (Join-Path $app "assets") -Recurse -File |
    Where-Object { $_.Extension -eq ".meta" } |
    Remove-Item -Force -ErrorAction SilentlyContinue

$files = Get-ChildItem -Path $app -Recurse -File
$mb = [math]::Round((($files | Measure-Object -Property Length -Sum).Sum / 1MB), 1)
Write-Host "Packaged '$name': $($files.Count) files, $mb MB"
Get-ChildItem -Path $app | Format-Table Name

$zip = Join-Path $dist "$name-windows-x86_64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $app "*") -DestinationPath $zip -Force
Write-Host "Wrote $zip"
