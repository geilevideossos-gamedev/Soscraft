$repo = Split-Path -Parent $PSScriptRoot
$slot = Join-Path $PSScriptRoot "cargo-slot.ps1"
$all = @("core", "world", "generator", "player", "entities")
$usage = "usage: scripts\clippy-vendored.ps1 [+exclusive] [+target=D:\dir] [+wait=minutes] [core|world|generator|player|entities ...]"
$pass = @()
$targetDir = ""
$crates = @()
foreach ($a in $args) {
    $s = [string]$a
    if ($s -eq "+exclusive" -or $s -like "+wait=*") { $pass += $s; continue }
    if ($s -like "+target=*") { $targetDir = $s.Substring(8); continue }
    $name = $s.ToLowerInvariant()
    if ($name -like "minecraftoss-*") { $name = $name.Substring(13) }
    if ($all -notcontains $name) {
        Write-Host $usage
        exit 2
    }
    if ($crates -notcontains $name) { $crates += $name }
}
if ($crates.Count -eq 0) { $crates = $all }
if ($targetDir -eq "") { $targetDir = $env:CARGO_TARGET_DIR }
if (-not $targetDir) { $targetDir = "D:\iw4l-target" }
$lintDir = Join-Path $targetDir "clippy-vendored"

$cargoArgs = @("check")
foreach ($c in $crates) { $cargoArgs += @("-p", "minecraftoss-$c") }
$cargoArgs += @("--locked", "--all-targets")

$scenarios = Join-Path $repo "third_party\scenarios"
$made = $false
$oldWrapper = $env:RUSTC_WRAPPER
$oldTarget = $env:CARGO_TARGET_DIR
Push-Location $repo
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$code = 1
try {
    if ($crates -contains "player" -and -not (Test-Path $scenarios)) {
        New-Item -ItemType Directory -Force (Join-Path $scenarios "survival") | Out-Null
        foreach ($f in @("difficulty-reference.csv", "difficulty-damage-reference.csv")) {
            $file = New-Item -ItemType File (Join-Path $scenarios "survival\$f")
            $file.LastWriteTime = [datetime]"2000-01-01"
        }
        $made = $true
    }
    $env:RUSTC_WRAPPER = "clippy-driver"
    & $slot "+target=$lintDir" @pass @cargoArgs
    $code = $LASTEXITCODE
} finally {
    if ($made) { Remove-Item -Recurse -Force $scenarios }
    $env:RUSTC_WRAPPER = $oldWrapper
    $env:CARGO_TARGET_DIR = $oldTarget
    Pop-Location
}
$sw.Stop()
Write-Host ("clippy-vendored: exit {0}, {1}, lint dir {2}, {3:N1} min" -f $code, ($crates -join " "), $lintDir, $sw.Elapsed.TotalMinutes)
exit $code
