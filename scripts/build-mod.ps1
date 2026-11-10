<#
.SYNOPSIS
    Builds one mod in release mode and copies what a release ships to builds/<crate>/.

    The DLL is always shipped. What goes next to it is `files` under [package.metadata.archive]
    in the mod's Cargo.toml, which every mod must declare: paths relative to the mod folder,
    wildcards allowed, a folder meaning everything below it. Each file keeps its relative path in
    the output and the zip. `files = []` ships the DLL alone.

.PARAMETER Mod
    PascalCase mod name, the same as its [lib] name in Cargo.toml (for example AutoPickup).
    The crate and folder name is this value in lowercase.

.PARAMETER Zip
    Also create builds/<crate>/<Mod>-<version>.zip with the shipped files.

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
Add-Type -AssemblyName System.IO.Compression.FileSystem

$root = Split-Path -Parent $PSScriptRoot
$crate = $Mod.ToLower()
$crateDir = Join-Path $root "mods/$crate"
$config = "$Mod.toml"

if (-not (Test-Path $crateDir)) { throw "No crate at '$crateDir'." }
if (-not (Test-Path (Join-Path $crateDir $config))) { throw "Missing '$config' in '$crateDir'." }

# Cargo parses the manifest, so `files` can be written any way TOML allows.
$package = (cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root "Cargo.toml") |
    ConvertFrom-Json).packages | Where-Object name -eq $crate
if (-not $package) { throw "Cargo does not know a package named '$crate'." }
$archive = $package.metadata.archive
if (-not ($archive -and $archive.PSObject.Properties.Name -contains "files")) {
    throw "'$crate' has no 'files' list under [package.metadata.archive] in its Cargo.toml."
}

# Paths of the files to ship next to the DLL, relative to the mod folder.
function Resolve-Shipped {
    $found = @(foreach ($pattern in @($archive.files)) {
        if ([IO.Path]::IsPathRooted($pattern) -or $pattern -match '(^|[\\/])\.\.([\\/]|$)') {
            throw "files entry '$pattern' must be inside the mod folder."
        }
        $items = @(Get-ChildItem -Path (Join-Path $crateDir $pattern) -ErrorAction SilentlyContinue)
        if (-not $items) { throw "files entry '$pattern' matches nothing in '$crateDir'." }
        foreach ($item in $items) {
            if ($item.PSIsContainer) { Get-ChildItem $item.FullName -Recurse -File } else { $item }
        }
    })
    $found | ForEach-Object { [IO.Path]::GetRelativePath($crateDir, $_.FullName).Replace('\', '/') } | Sort-Object -Unique
}
$shipped = @(Resolve-Shipped)

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
Get-ChildItem $out -Exclude *.zip | Remove-Item -Recurse -Force
Copy-Item $dll -Destination $out
foreach ($file in $shipped) {
    $target = Join-Path $out $file
    New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null
    Copy-Item (Join-Path $crateDir $file) -Destination $target
}
Write-Host "Built $out/$Mod.dll (+ $($shipped.Count) file(s))"

if ($Zip) {
    if (-not $Version) {
        $Version = $package.version
    }
    $zipPath = Join-Path $out "$Mod-$Version.zip"
    Remove-Item $zipPath -ErrorAction SilentlyContinue
    $zipFile = [IO.Compression.ZipFile]::Open($zipPath, "Create")
    try {
        foreach ($name in @("$Mod.dll") + $shipped) {
            $null = [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zipFile, (Join-Path $out $name), $name)
        }
    } finally {
        $zipFile.Dispose()
    }
    Write-Host "Created $zipPath"
}
