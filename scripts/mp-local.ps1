param(
    [string]$Exe = "",
    [string]$Map = "minecraft:overworld",
    [string]$Friend = "Friend",
    [string]$World = "mp-test",
    [string]$Resolution = "",
    [string]$GameDir = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup",
    [string]$Relay = "",
    [string]$PublicAddress = "",
    [int]$Port = 4433,
    [switch]$Fresh,
    [switch]$HostSound,
    [switch]$FriendOnly,
    [switch]$CertsOnly,
    [switch]$FriendKit,
    [switch]$Bats
)

function Say([string]$message) { Write-Host "mp-local: $message" }
function Fail([string]$message) { Write-Host "mp-local: $message"; exit 1 }

if (-not (Test-Path -LiteralPath $GameDir -PathType Container)) { Fail "no game folder at $GameDir (pass -GameDir)" }
$GameDir = (Resolve-Path -LiteralPath $GameDir).Path
$artifacts = Join-Path $GameDir "iw4l-artifacts"
$localDir = Join-Path $artifacts "mp-local"
$certDir = Join-Path $localDir "certs"
$hostDir = Join-Path $localDir "host"
$friendDir = Join-Path $localDir "friend"
$caKey = Join-Path $certDir "ca-key.pem"
$caCert = Join-Path $certDir "iw4l-ca.pem"
$serverKey = Join-Path $certDir "server-key.pem"
$serverCert = Join-Path $certDir "server-cert.pem"
$serverName = "iw4l-prod"
$utf8 = New-Object System.Text.UTF8Encoding $false

if ($Map -notmatch '^[A-Za-z0-9_:.-]+$') { Fail "-Map must be a map name such as minecraft:overworld or iw4:mp_rust" }
if ($World -notmatch '^[A-Za-z0-9_.-]+$') { Fail "-World must be a plain folder name" }
if ($Friend -notmatch '^[A-Za-z0-9_]{1,16}$') { Fail "-Friend must be 1-16 letters, digits or underscores" }
if ($Resolution -ne "" -and $Resolution -notmatch '^\d+x\d+$') { Fail "-Resolution must look like 1280x720" }

function Write-Lines([string]$path, [string[]]$lines) {
    [System.IO.File]::WriteAllLines($path, $lines, $utf8)
}

function Read-Lines([string]$path) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return @() }
    return [System.IO.File]::ReadAllLines($path)
}

function Find-OpenSsl {
    foreach ($candidate in @("C:\Program Files\Git\usr\bin\openssl.exe", "C:\Program Files\Git\mingw64\bin\openssl.exe")) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $found = Get-Command openssl -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    Fail "OpenSSL was not found; install Git for Windows"
}

function Invoke-OpenSsl([string[]]$arguments) {
    $output = & $script:openssl @arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $output | ForEach-Object { Write-Host "  $_" }
        Fail ("openssl {0} failed" -f $arguments[0])
    }
    return $output
}

function Ensure-Certs {
    $script:openssl = Find-OpenSsl
    New-Item -ItemType Directory -Force -Path $certDir | Out-Null
    $caParts = @(@($caKey, $caCert) | Where-Object { Test-Path -LiteralPath $_ })
    if ($caParts.Count -eq 1) { Fail "the CA in $certDir is incomplete; refusing to replace it" }
    if ($caParts.Count -eq 0) {
        Say "making the local CA in $certDir"
        Invoke-OpenSsl @("genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:3072", "-out", $caKey) | Out-Null
        Invoke-OpenSsl @("req", "-x509", "-new", "-sha256", "-days", "3650", "-key", $caKey, "-out", $caCert, "-subj", "/CN=IW4L Release CA") | Out-Null
    }
    $serverParts = @(@($serverKey, $serverCert) | Where-Object { Test-Path -LiteralPath $_ })
    if ($serverParts.Count -eq 1) { Fail "the server identity in $certDir is incomplete; refusing to replace it" }
    if ($serverParts.Count -eq 0) {
        Say "making the relay's certificate"
        $csr = Join-Path $certDir "server.csr"
        $ext = Join-Path $certDir "server.ext"
        Invoke-OpenSsl @("genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:3072", "-out", $serverKey) | Out-Null
        Invoke-OpenSsl @("req", "-new", "-key", $serverKey, "-out", $csr, "-subj", "/CN=$serverName") | Out-Null
        Write-Lines $ext @(
            "basicConstraints=critical,CA:FALSE",
            "keyUsage=critical,digitalSignature,keyEncipherment",
            "extendedKeyUsage=serverAuth",
            "subjectAltName=DNS:iw4l-prod,DNS:iw4l-dev"
        )
        Invoke-OpenSsl @("x509", "-req", "-sha256", "-days", "825", "-in", $csr, "-CA", $caCert, "-CAkey", $caKey, "-CAserial", (Join-Path $certDir "ca.srl"), "-CAcreateserial", "-extfile", $ext, "-out", $serverCert) | Out-Null
        Remove-Item -LiteralPath $csr, $ext -Force -ErrorAction SilentlyContinue
    }
    Invoke-OpenSsl @("verify", "-CAfile", $caCert, $serverCert) | Out-Null
    Say "certificates ready: $certDir (hand out iw4l-ca.pem only, never the keys)"
}

function Resolve-GameExe {
    if ($Exe -ne "") {
        $path = $Exe
        if (-not [System.IO.Path]::IsPathRooted($path)) { $path = Join-Path $GameDir $path }
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Fail "no game exe at $path" }
        return (Resolve-Path -LiteralPath $path).Path
    }
    $newest = Get-ChildItem -LiteralPath $GameDir -Filter "iw4l-mp-*.exe" -File | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $newest) { Fail "no iw4l-mp-*.exe in $GameDir; name one with -Exe, for example -Exe iw4l-survival.exe" }
    return $newest.FullName
}

function Join-Bat([string]$exeName) {
    $lines = @("@echo off", "setlocal", "cd /d `"%~dp0`"")
    if ($Map.StartsWith("minecraft:")) {
        $lines += @(
            "if not exist `"iw4l-artifacts\minecraft-26.3\.complete`" (",
            "  echo The Minecraft 26.3 files are not downloaded yet.",
            "  echo Start $exeName once to the main menu, wait until its log says `"Minecraft files ready`", quit, then run this file again.",
            "  pause",
            "  exit /b 1",
            ")"
        )
    }
    return $lines + @(
        "set `"IW4L_MASTER_HOST_NAME=`"",
        "set `"IW4L_MASTER_JOIN=`"",
        "set /p `"IW4L_MASTER_JOIN=Join code from the host's chat: `"",
        "if `"%IW4L_MASTER_JOIN%`"==`"`" exit /b 1",
        "start `"`" `"%~dp0$exeName`" map $Map"
    )
}

function Host-Bat([string]$exeName) {
    return @(
        "@echo off",
        "setlocal",
        "cd /d `"%~dp0`"",
        "set `"IW4L_MASTER_JOIN=`"",
        "set `"IW4L_MASTER_HOST_NAME=Minecraft world`"",
        "set `"IW4L_MASTER_MAX_PLAYERS=4`"",
        "set `"IW4L_MASTER_ADDR=127.0.0.1:$Port`"",
        "set `"IW4L_MASTER_SERVER_NAME=$serverName`"",
        "set `"IW4L_MASTER_CA_CERT=%~dp0iw4l-artifacts\mp-local\certs\iw4l-ca.pem`"",
        "set `"IW4L_MASTER_STATUS_FILE=%~dp0iw4l-artifacts\master.status`"",
        "if not exist `"iw4l-master.exe`" (",
        "  echo iw4l-master.exe is missing. Build it and copy it next to $exeName, see section 17.2 of docs\MULTIPLAYER.md in the repo.",
        "  pause",
        "  exit /b 1",
        ")",
        "if not exist `"iw4l-artifacts\mp-local\certs\server-key.pem`" (",
        "  echo The relay has no certificate yet. Run scripts\mp-local.ps1 -CertsOnly first.",
        "  pause",
        "  exit /b 1",
        ")",
        "tasklist /fi `"imagename eq iw4l-master.exe`" | %SystemRoot%\System32\find.exe /i `"iw4l-master.exe`" >nul || start `"iw4l-master`" /min `"%~dp0iw4l-master.exe`" serve --bind 0.0.0.0:$Port --cert `"%~dp0iw4l-artifacts\mp-local\certs\server-cert.pem`" --key `"%~dp0iw4l-artifacts\mp-local\certs\server-key.pem`"",
        "start `"`" `"%~dp0$exeName`" map $Map"
    )
}

function Write-Bats {
    $exePath = Resolve-GameExe
    if ((Split-Path -Parent $exePath) -ne $GameDir) { Fail "the .bat files start an exe in $GameDir; copy $exePath there first" }
    $exeName = Split-Path -Leaf $exePath
    $hostBat = Join-Path $GameDir "Host Minecraft World.bat"
    $joinBat = Join-Path $GameDir "Join Minecraft World.bat"
    Write-Lines $hostBat (Host-Bat $exeName)
    Write-Lines $joinBat (Join-Bat $exeName)
    Say "wrote $hostBat and $joinBat for $exeName"
}

function Env-Value([string[]]$lines, [string]$key) {
    $value = $null
    foreach ($line in $lines) {
        $parts = $line.Split("=", 2)
        if ($parts.Count -eq 2 -and $parts[0].Trim() -eq $key) { $value = $parts[1].Trim().Trim('"') }
    }
    return $value
}

function Write-FriendKit {
    $exePath = Resolve-GameExe
    Ensure-Certs
    $exeName = Split-Path -Leaf $exePath
    $stem = [System.IO.Path]::GetFileNameWithoutExtension($exeName)
    $address = $PublicAddress
    if ($address -eq "") { $address = "HOST_IP" }
    if ($address -notmatch ':\d+$') { $address = "${address}:$Port" }
    $password = Env-Value (Read-Lines (Join-Path $GameDir ".env")) "IW4L_MASTER_PASSWORD"
    if ($null -eq $password) { $password = "" }
    $envLines = @(
        "IW4L_GAMES=",
        "IW4L_SKATE=off",
        "IW4L_MASTER_ADDR=$address",
        "IW4L_MASTER_SERVER_NAME=$serverName",
        "IW4L_MASTER_CA_CERT=iw4l-ca.pem",
        "IW4L_MASTER_PASSWORD=$password"
    )
    $howTo = @(
        "Joining the host's Minecraft world",
        "",
        "1. Keep this folder anywhere. It needs your own copy of Call of Duty: Modern Warfare 2.",
        "2. Open .env in a text editor. IW4L_MASTER_ADDR must be the host's public IP address with :$Port and IW4L_MASTER_PASSWORD the room password the host gave you.",
        "3. Start $exeName once and stay in the main menu. The first start asks for your Modern Warfare 2 folder, converts its files and downloads Minecraft 26.3 from Mojang, which takes a few minutes. Quit when the log says Minecraft files ready.",
        "4. Set your player name in the game's settings. It decides your player in the host's world.",
        "5. Run Join Minecraft World.bat and type the join code from the host's chat.",
        "",
        "Or start the game normally and pick the host's room in the server browser."
    )
    New-Item -ItemType Directory -Force -Path $localDir | Out-Null
    $zipPath = Join-Path $localDir "friend-kit-$stem.zip"
    $staging = Join-Path $localDir "friend-kit-staging"
    if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Write-Lines (Join-Path $staging ".env") $envLines
    Write-Lines (Join-Path $staging "Join Minecraft World.bat") (Join-Bat $exeName)
    Write-Lines (Join-Path $staging "How to join.txt") $howTo
    $entries = New-Object System.Collections.Generic.List[object]
    $entries.Add(@($exePath, $exeName))
    foreach ($name in @("LICENSE", "NOTICE", "COPYING-FreeFont.txt", "OFL-Oxanium.txt", "README.txt")) {
        $path = Join-Path $GameDir $name
        if (Test-Path -LiteralPath $path -PathType Leaf) { $entries.Add(@($path, $name)) } else { Say "warning: $name is missing from $GameDir" }
    }
    $entries.Add(@($caCert, "iw4l-ca.pem"))
    foreach ($name in @(".env", "Join Minecraft World.bat", "How to join.txt")) { $entries.Add(@((Join-Path $staging $name), $name)) }
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
    $zip = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($entry in $entries) {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $entry[0], "$stem/" + $entry[1]) | Out-Null
        }
        $zip.CreateEntry("$stem/iw4l-artifacts/") | Out-Null
    } finally {
        $zip.Dispose()
    }
    Remove-Item -LiteralPath $staging -Recurse -Force
    Say "wrote ${zipPath}:"
    $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
    try { foreach ($entry in $zip.Entries) { Write-Host ("  {0}" -f $entry.FullName) } } finally { $zip.Dispose() }
    if ($PublicAddress -eq "") { Say "fill in the host's public IP in the kit's .env, or run again with -PublicAddress <ip>" }
}

function Pick-Resolution {
    if ($Resolution -ne "") { return $Resolution }
    try {
        Add-Type -AssemblyName System.Windows.Forms
        if ([System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width -ge 2560) { return "1280x720" }
    } catch { }
    return "960x540"
}

function Write-RoleSettings([string]$dir, [hashtable]$changes) {
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($line in (Read-Lines (Join-Path $artifacts "settings.cfg"))) { $lines.Add($line) }
    foreach ($key in $changes.Keys) {
        $found = $false
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i].Trim().StartsWith("$key=")) { $lines[$i] = "$key=$($changes[$key])"; $found = $true }
        }
        if (-not $found) { $lines.Add("$key=$($changes[$key])") }
    }
    Write-Lines (Join-Path $dir "settings.cfg") $lines.ToArray()
}

function Write-RoleEnv([string]$dir) {
    $source = Read-Lines (Join-Path $GameDir ".env")
    $skip = @("IW4L_MASTER_HOST_NAME", "IW4L_MASTER_JOIN", "IW4L_MINECRAFT_WORLD", "IW4L_MASTER_STATUS_FILE", "IW4L_SETTINGS_PATH", "IW4L_CONSOLE_STDIN", "IW4L_CMDS")
    $paths = @("IW4L_GAMES", "IW4L_SKATE_ASSETS", "IW4L_MASTER_CA_CERT")
    $lines = New-Object System.Collections.Generic.List[string]
    $hasGames = $false
    $hasSkate = $false
    foreach ($line in $source) {
        $parts = $line.Split("=", 2)
        $key = $parts[0].Trim()
        if ($parts.Count -eq 2 -and $skip -contains $key) { continue }
        if ($parts.Count -eq 2 -and $paths -contains $key) {
            $value = $parts[1].Trim().Trim('"')
            if ($value -ne "" -and -not [System.IO.Path]::IsPathRooted($value)) { $value = Join-Path $GameDir $value }
            $line = "$key=`"$value`""
        }
        if ($key -eq "IW4L_GAMES") { $hasGames = $true }
        if ($key -eq "IW4L_SKATE" -or $key -eq "IW4L_SKATE_ASSETS") { $hasSkate = $true }
        $lines.Add($line)
    }
    if (-not $hasGames) { Fail "$GameDir\.env has no IW4L_GAMES; start the game once from that folder first" }
    if (-not $hasSkate) { $lines.Add("IW4L_SKATE=off") }
    Write-Lines (Join-Path $dir ".env") $lines.ToArray()
}

function Prepare-Role([string]$dir, [string]$exePath, [hashtable]$settings) {
    $roleArtifacts = Join-Path $dir "iw4l-artifacts"
    New-Item -ItemType Directory -Force -Path $roleArtifacts | Out-Null
    foreach ($old in (Get-ChildItem -LiteralPath $dir -Filter "*.exe" -File)) {
        try { Remove-Item -LiteralPath $old.FullName -Force -ErrorAction Stop } catch { Fail "$($old.FullName) is still running; quit that window first" }
    }
    if ([System.IO.Path]::GetPathRoot($exePath) -ne [System.IO.Path]::GetPathRoot($dir)) { Fail "$exePath must be on the same drive as $GameDir for the hard link" }
    $link = Join-Path $dir (Split-Path -Leaf $exePath)
    New-Item -ItemType HardLink -Path $link -Target $exePath -ErrorAction Stop | Out-Null
    foreach ($name in @("cache", "minecraft-26.3")) {
        $target = Join-Path $artifacts $name
        $junction = Join-Path $roleArtifacts $name
        if ($name -eq "cache") { New-Item -ItemType Directory -Force -Path $target | Out-Null }
        if (-not (Test-Path -LiteralPath $target)) { continue }
        $existing = Get-Item -LiteralPath $junction -Force -ErrorAction SilentlyContinue
        if ($existing) {
            if (-not ($existing.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) { Say "warning: $junction is a real folder, not a link to $target" }
            continue
        }
        New-Item -ItemType Junction -Path $junction -Target $target -ErrorAction Stop | Out-Null
    }
    $profileDir = Join-Path $roleArtifacts "profile"
    $mainProfile = Join-Path $artifacts "profile"
    if (-not (Test-Path -LiteralPath $profileDir) -and (Test-Path -LiteralPath $mainProfile)) {
        Copy-Item -LiteralPath $mainProfile -Destination $profileDir -Recurse
    }
    Write-RoleEnv $dir
    Write-RoleSettings $dir $settings
    Remove-Item -LiteralPath (Join-Path $dir "master.status") -Force -ErrorAction SilentlyContinue
    return $link
}

function Start-Hidden([string]$exe, [string]$arguments, [string]$dir, [string]$log, [hashtable]$vars, [bool]$stdin) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $env:ComSpec
    $psi.Arguments = "/d /s /c `"`"$exe`" $arguments 1>`"$log.out.log`" 2>`"$log.err.log`"`""
    $psi.WorkingDirectory = $dir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $stdin
    $psi.CreateNoWindow = $true
    foreach ($name in @("IW4L_MASTER_HOST_NAME", "IW4L_MASTER_JOIN", "IW4L_CMDS", "IW4L_MINECRAFT_WORLD")) {
        if ($psi.EnvironmentVariables.ContainsKey($name)) { $psi.EnvironmentVariables.Remove($name) }
    }
    foreach ($key in $vars.Keys) { $psi.EnvironmentVariables[$key] = [string]$vars[$key] }
    $shell = [System.Diagnostics.Process]::Start($psi)
    $stem = [System.IO.Path]::GetFileNameWithoutExtension($exe)
    $process = $null
    for ($i = 0; $i -lt 100 -and $null -eq $process; $i++) {
        Start-Sleep -Milliseconds 100
        $process = Get-Process -Name $stem -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | Select-Object -First 1
        if ($null -eq $process -and $shell.HasExited) { break }
    }
    if ($null -eq $process) { Fail "$exe did not start; see $log.err.log" }
    return [pscustomobject]@{ Name = $stem; Shell = $shell; Process = $process; Peak = [long]0; Dir = $dir }
}

function Update-Peak($runner) {
    if ($null -eq $runner) { return }
    try {
        if (-not $runner.Process.HasExited) {
            $runner.Process.Refresh()
            if ($runner.Process.PeakWorkingSet64 -gt $runner.Peak) { $runner.Peak = $runner.Process.PeakWorkingSet64 }
        }
    } catch { }
}

function Send-Line($runner, [string]$line) {
    if ($null -eq $runner -or $runner.Process.HasExited) { Say "that window is closed"; return }
    try {
        $runner.Shell.StandardInput.WriteLine($line)
        $runner.Shell.StandardInput.Flush()
    } catch { Say "could not reach that window's console" }
}

function Read-Status([string]$dir) {
    $status = @{}
    try {
        foreach ($line in (Read-Lines (Join-Path $dir "master.status"))) {
            $parts = $line.Split("=", 2)
            if ($parts.Count -eq 2) { $status[$parts[0]] = $parts[1] }
        }
    } catch { }
    return $status
}

function Wait-Status($runner, [string]$what, [int]$seconds, [scriptblock]$done) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        Update-Peak $runner
        if ($runner.Process.HasExited) { Fail "the window in $($runner.Dir) closed while waiting for $what; see its iw4l-artifacts\logs" }
        $status = Read-Status $runner.Dir
        if ($status["state"] -eq "failed" -or $status["state"] -eq "closed") { Fail "the master connection ended while waiting for $what; see $($runner.Dir)\iw4l-artifacts\logs" }
        if (& $done $status) { return $status }
        Start-Sleep -Milliseconds 250
    }
    return $null
}

function Take-Slot($mutex) {
    try { return $mutex.WaitOne() } catch [System.Threading.AbandonedMutexException] { return $true }
}

function Master-Vars([string]$dir) {
    return @{
        "IW4L_MASTER_ADDR" = "127.0.0.1:$Port"
        "IW4L_MASTER_SERVER_NAME" = $serverName
        "IW4L_MASTER_CA_CERT" = $caCert
        "IW4L_MASTER_STATUS_FILE" = (Join-Path $dir "master.status")
        "IW4L_SETTINGS_PATH" = (Join-Path $dir "settings.cfg")
        "IW4L_CONSOLE_STDIN" = "1"
    }
}

function Start-Friend([string]$exePath, [string]$room, [string]$resolution) {
    $link = Prepare-Role $friendDir $exePath @{ "fullscreen" = "false"; "resolution" = $resolution; "player_name" = $Friend }
    $vars = Master-Vars $friendDir
    $vars["IW4L_MASTER_JOIN"] = $room
    Say "starting the friend window ($Friend) for room $room"
    return Start-Hidden $link "map $Map" $friendDir (Join-Path $friendDir "game") $vars $true
}

if ($CertsOnly) { Ensure-Certs; exit 0 }
if ($FriendKit) { Write-FriendKit; exit 0 }
if ($Bats) { Write-Bats; exit 0 }

$exePath = Resolve-GameExe
if ($Map.StartsWith("minecraft:") -and -not (Test-Path -LiteralPath (Join-Path $artifacts "minecraft-26.3\.complete"))) {
    Fail "the Minecraft 26.3 files are not in $artifacts yet; start the game once to the menu first"
}
$resolution = Pick-Resolution
$hostRunner = $null
$friendRunner = $null
$relayRunner = $null
$slots = @()

try {
    if ($FriendOnly) {
        if (-not (Test-Path -LiteralPath $caCert)) { Fail "no certificates in $certDir; run the script without -FriendOnly first" }
        $status = Read-Status $hostDir
        if ($status["state"] -ne "hosting" -or -not $status["room"]) { Fail "the host window is not hosting; run the script without -FriendOnly first" }
        if (-not (Get-Process -Name "iw4l-master" -ErrorAction SilentlyContinue)) { Say "warning: no iw4l-master is running" }
        $friendRunner = Start-Friend $exePath $status["room"] $resolution
    } else {
        Ensure-Certs
        $relayExe = $Relay
        if ($relayExe -eq "") { $relayExe = Join-Path $GameDir "iw4l-master.exe" }
        if (-not (Test-Path -LiteralPath $relayExe -PathType Leaf)) {
            Fail "no relay at $relayExe; build it with '& scripts\cargo-slot.ps1 build --release --locked -p iw4l-master' and copy D:\iw4l-target\release\iw4l-master.exe there"
        }
        $relayExe = (Resolve-Path -LiteralPath $relayExe).Path
        $m0 = New-Object System.Threading.Mutex($false, "Local\iw4l-cargo-slot-0")
        $m1 = New-Object System.Threading.Mutex($false, "Local\iw4l-cargo-slot-1")
        Say "waiting for both cargo slots, so no build runs beside two games and a relay"
        if (Take-Slot $m0) { $slots += $m0 }
        if (Take-Slot $m1) { $slots += $m1 }
        Say "holding both cargo slots until the windows close"
        if (Get-Process -Name "iw4l-master" -ErrorAction SilentlyContinue) {
            Say "an iw4l-master is already running; using it"
        } else {
            New-Item -ItemType Directory -Force -Path $localDir | Out-Null
            $relayRunner = Start-Hidden $relayExe "serve --bind 127.0.0.1:$Port --cert `"$serverCert`" --key `"$serverKey`"" $localDir (Join-Path $localDir "master") @{} $false
            Start-Sleep -Seconds 1
            if ($relayRunner.Process.HasExited) { Fail "the relay stopped at once; see $localDir\master.err.log" }
            Say "relay listening on 127.0.0.1:$Port (log $localDir\master.err.log)"
        }
        if ($Fresh) {
            $save = Join-Path $hostDir "iw4l-artifacts\saves\$World"
            if (Test-Path -LiteralPath $save) { Remove-Item -LiteralPath $save -Recurse -Force; Say "deleted the world $World" }
        }
        $hostSettings = @{ "fullscreen" = "false"; "resolution" = $resolution }
        if (-not $HostSound) { $hostSettings["master_volume"] = "0.000" }
        $link = Prepare-Role $hostDir $exePath $hostSettings
        $vars = Master-Vars $hostDir
        $vars["IW4L_MASTER_HOST_NAME"] = "Local world"
        $vars["IW4L_MASTER_MAX_PLAYERS"] = "4"
        $vars["IW4L_MINECRAFT_WORLD"] = $World
        Say "starting the host window: $(Split-Path -Leaf $exePath) map $Map, world $World"
        $hostRunner = Start-Hidden $link "map $Map" $hostDir (Join-Path $hostDir "game") $vars $true
        $status = Wait-Status $hostRunner "the room" 90 { param($s) $s["state"] -eq "hosting" -and $s["room"] -and $s["room"] -notmatch '^0+$' }
        if ($null -eq $status) { Fail "the host found no room within 90 s; see $hostDir\iw4l-artifacts\logs" }
        Say "room $($status["room"])"
        $friendRunner = Start-Friend $exePath $status["room"] $resolution
        $members = Wait-Status $hostRunner "the friend" 120 { param($s) [int]("0" + $s["members"]) -ge 2 }
        if ($null -eq $members) { Say "warning: the friend has not joined the room after 120 s" } else { Say "both players are in the room" }
    }

    Say "console: 'h <command>' goes to the host window, 'f <command>' to the friend window, 'quit' closes both"
    while ($true) {
        foreach ($runner in @($hostRunner, $friendRunner, $relayRunner)) { Update-Peak $runner }
        $open = @(@($hostRunner, $friendRunner) | Where-Object { $null -ne $_ -and -not $_.Process.HasExited })
        if ($open.Count -eq 0) { break }
        $line = $null
        try { if ([Console]::KeyAvailable) { $line = [Console]::ReadLine() } } catch { }
        if ($null -eq $line) { Start-Sleep -Milliseconds 200; continue }
        $line = $line.Trim()
        if ($line -eq "") { continue }
        if ($line -eq "quit") {
            Send-Line $hostRunner "quit !"
            Send-Line $friendRunner "quit !"
            continue
        }
        $parts = $line.Split(" ", 2)
        if ($parts.Count -lt 2) { Say "use h <command>, f <command> or quit"; continue }
        switch ($parts[0]) {
            "h" { Send-Line $hostRunner $parts[1] }
            "f" { Send-Line $friendRunner $parts[1] }
            default { Say "use h <command>, f <command> or quit" }
        }
    }
} finally {
    foreach ($runner in @($hostRunner, $friendRunner, $relayRunner)) { Update-Peak $runner }
    if ($null -ne $relayRunner -and -not $relayRunner.Process.HasExited) {
        Stop-Process -Id $relayRunner.Process.Id -Force -ErrorAction SilentlyContinue
        Say "relay stopped"
    }
    foreach ($slot in $slots) { try { $slot.ReleaseMutex() } catch { } }
    $mib = { param($r) if ($null -eq $r) { "-" } else { "{0:N0} MiB" -f ($r.Peak / 1MB) } }
    $line = "{0} exe={1} map={2} host={3} friend={4} relay={5}" -f (Get-Date -Format "yyyy-MM-dd HH:mm"), (Split-Path -Leaf $exePath), $Map, (& $mib $hostRunner), (& $mib $friendRunner), (& $mib $relayRunner)
    if ($null -ne $hostRunner -or $null -ne $friendRunner) {
        try { Add-Content -LiteralPath (Join-Path $localDir "memory.log") -Value $line -Encoding ascii } catch { }
        Say "peak working sets: $line"
    }
}
