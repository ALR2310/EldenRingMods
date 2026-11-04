<#
.SYNOPSIS
    Builds one mod in release mode and copies its DLL and config to builds/<crate>/. A mod whose
    Cargo.toml has `ship-config = false` under [package.metadata.release] ships the DLL alone: it
    creates its config on first run.

.PARAMETER Mod
    PascalCase mod name, the same as its [lib] name in Cargo.toml (for example AutoPickup).
    The crate and folder name is this value in lowercase.

.PARAMETER Zip
    Also create builds/<crate>/<Mod>-<version>.zip containing the DLL, config and any
    license/notice .txt files.

.PARAMETER Version
    Version used in the zip name. Defaults to the version in the mod's Cargo.toml.

.EXAMPLE
    pwsh scripts/build-mod.ps1 -Mod AutoPickup -Zip
#>
param(
    [Parameter(Mandatory)]
    [string]$Mod,
    [switch]$Zip,
    [string]$Version
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$crate = $Mod.ToLower()
$crateDir = Join-Path $root "mods/$crate"
$config = "$Mod.toml"

if (-not (Test-Path $crateDir)) { throw "No crate at '$crateDir'." }
if (-not (Test-Path (Join-Path $crateDir $config))) { throw "Missing '$config' in '$crateDir'." }

$manifest = Get-Content (Join-Path $crateDir "Cargo.toml") -Raw
$shipConfig = $manifest -notmatch '(?m)^ship-config\s*=\s*false'

# Panic locations of this repo and of the dependencies would otherwise embed the build
# machine's paths (including the user name) in the DLL.
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
$sysroot = (rustc --print sysroot).Trim()
$env:RUSTFLAGS = "--remap-path-prefix=$root=. --remap-path-prefix=$cargoHome=.cargo --remap-path-prefix=$sysroot=.rustup"
try {
    cargo build --release -p $crate --manifest-path (Join-Path $root "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed for '$crate'." }
} finally {
    Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
}

$dll = Join-Path $root "target/release/$Mod.dll"
if (-not (Test-Path $dll)) { throw "'$dll' was not produced; check [lib] name in Cargo.toml." }

$out = Join-Path $root "builds/$crate"
New-Item -ItemType Directory -Force $out | Out-Null
# License and notice files (e.g. license-notice.txt) in the crate folder or its assets/ folder
# ship next to the DLL.
$notices = @(foreach ($dir in $crateDir, (Join-Path $crateDir "assets")) {
    if (Test-Path $dir) { Get-ChildItem $dir -File | Where-Object Name -match '(license|notice).*\.txt$' }
})
$shipped = @($dll) + @(if ($shipConfig) { Join-Path $crateDir $config }) + @($notices | ForEach-Object FullName)
Copy-Item $shipped -Destination $out -Force
if (-not $shipConfig) { Remove-Item (Join-Path $out $config) -ErrorAction SilentlyContinue }
Write-Host "Built $out/$Mod.dll"

if ($Zip) {
    if (-not $Version) {
        if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw "No version in $crate's Cargo.toml." }
        $Version = $Matches[1]
    }
    $zipPath = Join-Path $out "$Mod-$Version.zip"
    $files = @("$Mod.dll") + @(if ($shipConfig) { $config }) + @($notices | ForEach-Object Name) | ForEach-Object { Join-Path $out $_ }
    Compress-Archive -Path $files -DestinationPath $zipPath -Force
    Write-Host "Created $zipPath"
}
