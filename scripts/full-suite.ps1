$repo = Split-Path -Parent $PSScriptRoot
$slot = Join-Path $PSScriptRoot "cargo-slot.ps1"
$extra = @($args)
Push-Location $repo
$sw = [System.Diagnostics.Stopwatch]::StartNew()

& $slot +exclusive @extra test --workspace --locked --tests --no-fail-fast
$ws = $LASTEXITCODE

$scenarios = Join-Path $repo "third_party\scenarios"
$made = $false
if (-not (Test-Path $scenarios)) {
    New-Item -ItemType Directory -Force (Join-Path $scenarios "survival") | Out-Null
    New-Item -ItemType File (Join-Path $scenarios "survival\difficulty-reference.csv") | Out-Null
    New-Item -ItemType File (Join-Path $scenarios "survival\difficulty-damage-reference.csv") | Out-Null
    $made = $true
}
try {
    & $slot +exclusive @extra test -p minecraftoss-core -p minecraftoss-world -p minecraftoss-generator -p minecraftoss-player -p minecraftoss-entities --locked --tests --no-fail-fast
    $oss = $LASTEXITCODE
} finally {
    if ($made) { Remove-Item -Recurse -Force $scenarios }
}

Pop-Location
$sw.Stop()
Write-Host ("full-suite: workspace exit {0}, minecraftoss exit {1}, {2:N1} min" -f $ws, $oss, $sw.Elapsed.TotalMinutes)
if ($ws -ne 0) { exit $ws }
exit $oss
