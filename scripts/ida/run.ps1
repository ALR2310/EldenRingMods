<#
.SYNOPSIS
    Runs one IDAPython script from scripts/ida/ on an existing eldenring.exe.i64 database.

.DESCRIPTION
    Uses idat (headless): idat -A -S"<script> <args>" -L<log> <db>.i64.
    Databases live in dumps/eldenring/<version>/eldenring.exe.i64 (version = the exe's
    FileVersion). The script's output goes to dumps/eldenring/<version>/out/ (environment
    variable ER_OUT). Do not use a database that the IDA GUI has open; close the GUI first.

    idat is taken from the ER_IDAT environment variable, otherwise from PATH.

.PARAMETER Script
    File name in scripts/ida/ (for example decompile.py), with or without the .py extension.

.PARAMETER Version
    Version folder in dumps/eldenring/ (for example 2.7.1.0), or `all` to run every version
    that has an .i64 in turn. Default: the latest version.

.PARAMETER Arguments
    Arguments passed to the script (RVAs, AOB patterns, --x flags...). Quote those that
    contain spaces; with aob.py each pattern is its own double-quoted string.

.EXAMPLE
    pwsh scripts/ida/run.ps1 decompile 1e92100 --callees
    pwsh scripts/ida/run.ps1 aob -Version all "48 8B 05 ?? ?? ?? ?? 48 85 C0"
    pwsh scripts/ida/run.ps1 -Script xrefs.py -Version 2.7.1.0 25e100 --depth=2
#>
param(
    [Parameter(Mandatory, Position = 0)][string]$Script,
    [string]$Version = "",
    [Parameter(ValueFromRemainingArguments)][string[]]$Arguments = @()
)

$ErrorActionPreference = "Stop"
$Here = $PSScriptRoot
$RepoRoot = Split-Path -Parent (Split-Path -Parent $Here)
$IdaRoot = Join-Path $RepoRoot "dumps\eldenring"

$Idat = if ($env:ER_IDAT) { $env:ER_IDAT } else { (Get-Command idat -ErrorAction SilentlyContinue).Source }
if (-not $Idat -or -not (Test-Path $Idat)) { throw "idat.exe not found. Set the ER_IDAT environment variable to its full path." }

if (-not $Script.EndsWith(".py")) { $Script += ".py" }
$ScriptPath = Join-Path $Here $Script
if (-not (Test-Path $ScriptPath)) { throw "Script '$Script' not found in $Here" }

$All = Get-ChildItem $IdaRoot -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "eldenring.exe.i64") } |
    Sort-Object { try { [version]$_.Name } catch { [version]"0.0" } }
if (-not $All) { throw "No database found in $IdaRoot" }

$Targets = if ($Version -eq "all") { $All }
elseif ($Version) { $All | Where-Object Name -eq $Version }
else { $All | Select-Object -Last 1 }
if (-not $Targets) { throw "Version '$Version' not found. Available: $($All.Name -join ', ')" }

# Join the arguments into one string for -S; arguments with spaces get double quotes.
$Joined = ($Arguments | ForEach-Object { if ($_ -match '\s') { '\"' + $_ + '\"' } else { $_ } }) -join " "

foreach ($T in $Targets) {
    $Db = Join-Path $T.FullName "eldenring.exe.i64"
    $Out = Join-Path $T.FullName "out"
    New-Item -ItemType Directory -Force $Out | Out-Null
    $Log = Join-Path $Out ("{0}.log" -f [IO.Path]::GetFileNameWithoutExtension($Script))
    $env:ER_OUT = $Out
    $cmdline = '-A -S"{0} {1}" -L"{2}" "{3}"' -f $ScriptPath, $Joined, $Log, $Db
    Write-Host "==> [$($T.Name)] $Script $Joined" -ForegroundColor Cyan
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $Idat -ArgumentList $cmdline -Wait -PassThru -NoNewWindow
    Write-Host ("    finished in {0:N0}s (exit {1}), output: {2}" -f $sw.Elapsed.TotalSeconds, $p.ExitCode, $Out)
    # Echo the RESULT lines (aob.py, ...) so versions can be compared at a glance.
    Get-ChildItem $Out -Filter "*.txt" |
        Where-Object { $_.LastWriteTime -gt (Get-Date).AddSeconds(-($sw.Elapsed.TotalSeconds + 5)) } |
        ForEach-Object { Select-String -Path $_.FullName -Pattern '^RESULT ' | ForEach-Object { "    [$($T.Name)] $($_.Line)" } }
}
