$exclusive = $false
$targetDir = ""
$waitMinutes = 0
$cargoArgs = @()
$slots = 2
$jobs = 8
$low = $false
$cfg = Join-Path $env:LOCALAPPDATA "iw4l-cargo-slot.txt"
if (Test-Path $cfg) {
    foreach ($line in Get-Content $cfg) {
        if ($line -match '^\s*slots\s*=\s*([12])\s*$') { $slots = [int]$Matches[1] }
        if ($line -match '^\s*jobs\s*=\s*([1-8])\s*$') { $jobs = [int]$Matches[1] }
        if ($line -match '^\s*priority\s*=\s*low\s*$') { $low = $true }
    }
}
if ($slots -eq 1) { $exclusive = $true }
foreach ($a in $args) {
    $s = [string]$a
    if ($cargoArgs.Count -eq 0 -and $s -eq "+exclusive") { $exclusive = $true; continue }
    if ($cargoArgs.Count -eq 0 -and $s -like "+target=*") { $targetDir = $s.Substring(8); continue }
    if ($cargoArgs.Count -eq 0 -and $s -like "+wait=*") { $waitMinutes = [int]$s.Substring(6); continue }
    $cargoArgs += $s
}

if ($cargoArgs.Count -eq 0) {
    Write-Host "usage: scripts\cargo-slot.ps1 [+exclusive] [+target=D:\dir] [+wait=minutes] <cargo subcommand> <args>"
    exit 2
}

$sub = $cargoArgs[0]
$rest = @()
if ($cargoArgs.Count -gt 1) { $rest = $cargoArgs[1..($cargoArgs.Count - 1)] }

if ($sub -eq "fmt") {
    Write-Host "cargo-slot: cargo fmt is not allowed in this fork"
    exit 2
}

$jobsSubs = @("build", "b", "check", "c", "test", "t", "clippy", "run", "r", "doc", "bench", "rustc")
$cmdArgs = @($sub)
if ($jobsSubs -contains $sub) {
    $hasJobs = $false
    $userArgs = @()
    $passThrough = @()
    $afterDashes = $false
    foreach ($a in $rest) {
        if ($afterDashes) { $passThrough += $a; continue }
        if ($a -eq "--") { $afterDashes = $true; continue }
        if ($a -eq "-j" -or $a -like "-j*" -or $a -like "--jobs*") { $hasJobs = $true }
        $userArgs += $a
    }
    if ($hasJobs) {
        $joined = " " + ($userArgs -join " ") + " "
        if ($joined -notmatch "(\s-j\s*8\s)|(\s--jobs[ =]8\s)") {
            Write-Host "cargo-slot: only -j 8 is allowed"
            exit 2
        }
        $kept = @()
        $skipNext = $false
        foreach ($a in $userArgs) {
            if ($skipNext) { $skipNext = $false; continue }
            if ($a -eq "-j" -or $a -eq "--jobs") { $skipNext = $true; continue }
            if ($a -like "-j*" -or $a -like "--jobs=*") { continue }
            $kept += $a
        }
        $userArgs = $kept
    }
    $cmdArgs += @("-j", [string]$jobs)
    $cmdArgs += $userArgs
    if ($afterDashes) { $cmdArgs += "--"; $cmdArgs += $passThrough }
    foreach ($a in $userArgs) {
        if ($a -eq "--release" -or $a -eq "--workspace" -or $a -eq "--all" -or $a -like "--profile*") { $exclusive = $true }
    }
} else {
    $cmdArgs += $rest
}

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$noSlotSubs = @("version", "--version", "-V", "metadata", "tree", "pkgid", "locate-project", "help", "--help", "-h")
if ($noSlotSubs -contains $sub) {
    & cargo @cmdArgs
    exit $LASTEXITCODE
}
if ($targetDir -ne "") {
    $env:CARGO_TARGET_DIR = $targetDir
} elseif (-not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = "D:\iw4l-target"
}
if (-not $env:MINECRAFTOSS_ROOT) {
    $env:MINECRAFTOSS_ROOT = "C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3"
}

if (-not ('CargoSlotJob' -as [type])) {
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class CargoSlotJob {
    [StructLayout(LayoutKind.Sequential)]
    struct BasicLimits {
        public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit; public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize; public uint ActiveProcessLimit;
        public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct IoCounters { public ulong A, B, C, D, E, F; }
    [StructLayout(LayoutKind.Sequential)]
    struct ExtendedLimits {
        public BasicLimits Basic; public IoCounters Io;
        public UIntPtr ProcessMemoryLimit; public UIntPtr JobMemoryLimit; public UIntPtr PeakProcessMemoryUsed; public UIntPtr PeakJobMemoryUsed;
    }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr job, int infoClass, ref ExtendedLimits info, uint length);
    [DllImport("kernel32.dll")] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    public static IntPtr Job = IntPtr.Zero;
    public static bool KillTreeOnExit() {
        if (Job != IntPtr.Zero) return true;
        Job = CreateJobObject(IntPtr.Zero, null);
        if (Job == IntPtr.Zero) return false;
        ExtendedLimits info = new ExtendedLimits();
        info.Basic.LimitFlags = 0x2000;
        if (!SetInformationJobObject(Job, 9, ref info, (uint)Marshal.SizeOf(typeof(ExtendedLimits)))) return false;
        return AssignProcessToJobObject(Job, GetCurrentProcess());
    }
}
"@
}
if (-not [CargoSlotJob]::KillTreeOnExit()) { Write-Host "cargo-slot: warning: cargo will outlive this shell if it is killed" }

if (-not ('CargoSlotQos2' -as [type])) {
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;
public static class CargoSlotQos2 {
    [StructLayout(LayoutKind.Sequential)]
    struct PowerThrottling { public uint Version; public uint ControlMask; public uint StateMask; }
    [DllImport("kernel32.dll")] static extern bool SetProcessInformation(IntPtr process, int infoClass, ref PowerThrottling info, uint size);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    static readonly string[] Names = { "cargo", "rustc", "rust-lld", "lld-link", "link", "clippy-driver", "build-script-build" };
    static readonly HashSet<int> Done = new HashSet<int>();
    static Timer timer;
    public static bool Low = false;
    public static int Freed = 0;
    static bool Free(IntPtr handle) {
        PowerThrottling p = new PowerThrottling();
        p.Version = 1; p.ControlMask = 5; p.StateMask = 0;
        return SetProcessInformation(handle, 4, ref p, 12);
    }
    static void Sweep(object state) {
        foreach (string name in Names) {
            foreach (Process proc in Process.GetProcessesByName(name)) {
                try {
                    lock (Done) { if (Done.Contains(proc.Id)) continue; }
                    if (Low) { proc.PriorityClass = ProcessPriorityClass.BelowNormal; }
                    if (Free(proc.Handle)) { Freed++; lock (Done) { Done.Add(proc.Id); } }
                } catch { } finally { proc.Dispose(); }
            }
        }
    }
    public static bool Start() {
        bool self = Free(GetCurrentProcess());
        if (timer == null) { timer = new Timer(Sweep, null, 0, 1000); }
        return self;
    }
}
"@
}
[CargoSlotQos2]::Low = $low
if (-not [CargoSlotQos2]::Start()) { Write-Host "cargo-slot: warning: could not opt this shell out of Windows power throttling" }

$m0 = New-Object System.Threading.Mutex($false, "Local\iw4l-cargo-slot-0")
$m1 = New-Object System.Threading.Mutex($false, "Local\iw4l-cargo-slot-1")
$held = @()
$timeout = [System.Threading.Timeout]::Infinite
if ($waitMinutes -gt 0) { $timeout = $waitMinutes * 60 * 1000 }
$mode = "shared"
if ($exclusive) { $mode = "exclusive" }
Write-Host ("cargo-slot: waiting for a {0} slot ({1})" -f $mode, (Get-Date -Format "HH:mm:ss"))

function Take-One($mutex, $ms) {
    try {
        return $mutex.WaitOne($ms)
    } catch [System.Threading.AbandonedMutexException] {
        return $true
    }
}

$start = Get-Date
if ($exclusive) {
    if (-not (Take-One $m0 $timeout)) { Write-Host "cargo-slot: timed out waiting"; exit 75 }
    $held += $m0
    $left = $timeout
    if ($timeout -ne [System.Threading.Timeout]::Infinite) {
        $left = [Math]::Max(0, $timeout - [int]((Get-Date) - $start).TotalMilliseconds)
    }
    if (-not (Take-One $m1 $left)) { $m0.ReleaseMutex(); Write-Host "cargo-slot: timed out waiting"; exit 75 }
    $held += $m1
} else {
    $idx = -1
    try {
        $idx = [System.Threading.WaitHandle]::WaitAny(@($m0, $m1), $timeout)
    } catch [System.Threading.AbandonedMutexException] {
        $idx = $_.Exception.MutexIndex
    }
    if ($idx -eq [System.Threading.WaitHandle]::WaitTimeout) { Write-Host "cargo-slot: timed out waiting"; exit 75 }
    if ($idx -eq 0) { $held += $m0 } else { $held += $m1 }
}

$waited = [int]((Get-Date) - $start).TotalSeconds
Write-Host ("cargo-slot: got the slot after {0}s, target {1}: cargo {2}" -f $waited, $env:CARGO_TARGET_DIR, ($cmdArgs -join " "))
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$code = 1
try {
    & cargo @cmdArgs
    $code = $LASTEXITCODE
} finally {
    foreach ($h in $held) { $h.ReleaseMutex() }
}
$sw.Stop()
Write-Host ("cargo-slot: exit {0} after {1:N0}s ({2} processes kept off power throttling)" -f $code, $sw.Elapsed.TotalSeconds, [CargoSlotQos2]::Freed)
exit $code
