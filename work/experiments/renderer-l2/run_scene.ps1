param(
    [Parameter(Mandatory=$true)][ValidateSet('sa2','sd')][string]$Candidate,
    [Parameter(Mandatory=$true)][string]$Build,
    [ValidateSet('w1','w2','w3','w4')][string]$Scene = 'w3',
    [ValidateRange(1,100)][int]$Runs = 3,
    [ValidateSet('corrupt-label','swap-same-colour','delay-adoption','stale-binding','module-late','module-transient','module-reload')][string]$Inject,
    [string[]]$Declare = @(),
    [string]$Overlays,
    [ValidateRange(1000,3600000)][int]$TraceMs = 192000,
    [ValidateRange(0,60000)][int]$PrerollMs = 4000,
    [double]$TurnMs = 190,
    [string]$Adapter,
    [switch]$Short,
    [switch]$Geometry,
    [switch]$Validation,
    [switch]$NoVram,
    [switch]$DebugHalfTarget
)
$ErrorActionPreference = 'Stop'
if ((@($Short, $Geometry, $Validation) | Where-Object { $_ }).Count -gt 1) { throw 'Choose at most one of -Short, -Geometry and -Validation.' }
if (($NoVram -or $DebugHalfTarget) -and -not $Short) { throw '-NoVram and -DebugHalfTarget require -Short.' }
if ([double]::IsNaN($TurnMs) -or [double]::IsInfinity($TurnMs) -or $TurnMs -le 0 -or $TurnMs -gt 10000) { throw '-TurnMs must be finite and in (0,10000].' }
if ($Inject -and ($Scene -notin @('w3','w4') -or $Geometry -or $Validation)) { throw 'Fault injection requires a W3 or W4 run or short capture.' }
if ($Inject -like 'module-*' -and $Candidate -ne 'sd') { throw 'The module-* injections exist only for the Qt app (sd).' }
if (@($Declare | Where-Object { $_ -like 'overlays=*' }).Count) { throw 'Give the overlay configuration with -Overlays, not -Declare.' }
if ($PSBoundParameters.ContainsKey('Adapter') -and [string]::IsNullOrWhiteSpace($Adapter)) { throw '-Adapter must be nonempty.' }
$captureMode = -not $Geometry -and -not $Validation
$mode = if ($Geometry) { 'geometry' } elseif ($Validation) { 'validation' } elseif ($Short) { 'short' } else { 'run' }
if ($mode -eq 'run' -and -not $Inject -and [string]::IsNullOrWhiteSpace($Overlays)) { throw "State the overlay configuration with -Overlays, for example -Overlays 'none running'." }
if ($mode -eq 'run' -and -not $Inject) {
    # The gate marks a run without these two declarations conditions-missing; the app turns true and false into booleans.
    foreach ($fact in 'frame_generation','upscaling') {
        if (@($Declare | Where-Object { $_ -cmatch "^$fact=(true|false)$" }).Count -ne 1) { throw "Declare $fact once as true or false, for example -Declare @('frame_generation=false','upscaling=false')." }
    }
}
if ($captureMode) {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'PresentMon requires administrator rights. Run this script in an administrator PowerShell.' }
}
$repository = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../..')).Path
$finalizer = Join-Path $PSScriptRoot 'finalize_run.py'
$guardScript = Join-Path $PSScriptRoot 'file_guard.py'
$python = (Get-Command python -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
$buildDirectory = (Resolve-Path -LiteralPath $Build).Path
$launch = Get-Content -LiteralPath (Join-Path $buildDirectory 'launch.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($launch.format -ne 'magic600-l2-launch-v1' -or $launch.candidate -ne $Candidate) { throw 'launch.json does not match this candidate and format.' }
foreach ($key in 'executable','dll','working_directory','arguments','run_arguments','validation_arguments','separator','environment','validation_environment') {
    if ($null -eq $launch.PSObject.Properties[$key]) { throw "launch.json lacks $key." }
}
foreach ($key in 'executable','dll','working_directory') {
    $value = $launch.$key
    if ($value -isnot [string] -or -not [IO.Path]::IsPathRooted($value) -or -not (Test-Path -LiteralPath $value)) { throw "launch.json has no usable absolute $key path." }
}
if (-not (Test-Path -LiteralPath $launch.executable -PathType Leaf) -or -not (Test-Path -LiteralPath $launch.dll -PathType Leaf) -or -not (Test-Path -LiteralPath $launch.working_directory -PathType Container)) { throw 'launch.json requires executable and DLL files and a working directory.' }
foreach ($key in 'arguments','run_arguments','validation_arguments','separator') {
    if ($launch.$key -isnot [Array] -or @($launch.$key | Where-Object { $_ -isnot [string] }).Count) { throw "launch.json $key must be an array of strings." }
}
foreach ($key in 'environment','validation_environment') {
    if ($launch.$key -isnot [PSCustomObject]) { throw "launch.json $key must be an object." }
    foreach ($entry in $launch.$key.PSObject.Properties) {
        if ($entry.Name -match '[=\x00]' -or ($null -ne $entry.Value -and $entry.Value -isnot [string])) { throw "launch.json $key contains an invalid environment entry." }
    }
}
if (($Candidate -eq 'sa2' -and (($launch.separator.Count -ne 1) -or $launch.separator[0] -ne '--')) -or ($Candidate -eq 'sd' -and $launch.separator.Count -ne 0)) { throw 'launch.json has the wrong framework argument separator.' }
$presentMon = Join-Path $env:ProgramFiles 'Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe'
if ($captureMode -and -not (Test-Path -LiteralPath $presentMon -PathType Leaf)) { throw 'PresentMon 2.6.0.0 was not found at the pinned location.' }
$logman = Join-Path $env:SystemRoot 'System32/logman.exe'
$session = "magic600-$Candidate-capture"

function Native-Arguments([string[]]$Values) {
    # CommandLineToArgvW quoting, including a trailing backslash before the quote.
    return (($Values | ForEach-Object { '"' + (($_ -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"' }) -join ' ')
}
function Number-Argument([double]$Value) { return $Value.ToString('R', [Globalization.CultureInfo]::InvariantCulture) }
function Run-Helper([string]$File, [string[]]$Values) {
    $helper = Start-Process -FilePath $File -ArgumentList (Native-Arguments $Values) -PassThru -WindowStyle Hidden
    $null = $helper.Handle
    if ($helper.WaitForExit(30000)) { return $helper.ExitCode }
    try { $helper.Kill() } catch { }
    $null = $helper.WaitForExit(5000)
    return -1
}
function Trace-Session([string]$Name) {
    $code = Run-Helper $logman @('query', $Name, '-ets')
    if ($code -eq 0) { return 'running' }
    if ($code -eq -2144337918) { return 'missing' }
    return "unknown (logman exit $code)"
}
function Stop-Capture([string]$Name) {
    return Run-Helper $presentMon @('--session_name', $Name, '--terminate_existing_session')
}
function Remove-Session([string]$Name) {
    foreach ($controller in 'PresentMon', 'logman') {
        try {
            if ((Trace-Session $Name) -eq 'missing') { break }
            if ($controller -eq 'PresentMon') { $null = Stop-Capture $Name } else { $null = Run-Helper $logman @('stop', $Name, '-ets') }
        } catch { Write-Warning "Stopping trace session $Name with $controller failed: $($_.Exception.Message)" }
    }
    try { $state = Trace-Session $Name } catch { $state = "unknown ($($_.Exception.Message))" }
    if ($state -ne 'missing') { Write-Warning "Trace session $Name is still $state. Stop it with: logman stop $Name -ets. The next run refuses to start while it runs." }
}
function Assert-NoSession {
    foreach ($name in 'PresentMon','magic600-sb-capture','magic600-sa2-capture','magic600-sd-capture') {
        $state = Trace-Session $name
        if ($state -eq 'running') { throw "A trace session named $name is running. Stop it with: logman stop $name -ets" }
        if ($state -ne 'missing') { throw "logman could not query the trace session ${name}: $state." }
    }
}
function Assert-Idle {
    foreach ($other in Get-CimInstance -ClassName Win32_Process) {
        if ($other.ProcessId -eq $PID) { continue }
        $name = [string]$other.Name
        $command = [string]$other.CommandLine
        $reason = $null
        if ($name -match '^(cl|link|ninja|cmake|msbuild|dotnet|VBCSCompiler)\.exe$') { $reason = 'build' }
        elseif ($name -match '^(Godot.*|blender|ffmpeg)\.exe$') { $reason = 'framework or media process' }
        # PresentMonService.exe, the always-on service of Intel's installer, is allowed; its overlay client is not.
        elseif ($name -match '^(PresentMon|PresentMonUI|PresentMon-.+)\.exe$') { $reason = 'PresentMon overlay or capture' }
        elseif ($command -match 'codex_review\.py\b' -and $command -match '--kind["'']?(?:=|\s+)["'']?implement\b') { $reason = 'Codex implement call' }
        elseif ($name -match '^(powershell|pwsh)\.exe$' -and $command -match 'run_scene\.ps1\b') { $reason = 'another runner' }
        if ($reason) { throw "Cannot start while $reason runs: $name (PID $($other.ProcessId))." }
    }
}
function Start-App([string[]]$Values, [bool]$WithValidation) {
    # Apply environment overrides only to this child, including null removals.
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $launch.executable
    $info.WorkingDirectory = $launch.working_directory
    $info.Arguments = Native-Arguments $Values
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $false
    $maps = @($launch.environment)
    if ($WithValidation) { $maps += $launch.validation_environment }
    foreach ($map in $maps) {
        foreach ($entry in $map.PSObject.Properties) {
            if ($null -eq $entry.Value) { $info.EnvironmentVariables.Remove($entry.Name) }
            else { $info.EnvironmentVariables[$entry.Name] = [string]$entry.Value }
        }
    }
    # The framework's real presenting process inherits this console. Never hide its window.
    $app = New-Object Diagnostics.Process
    $app.StartInfo = $info
    if (-not $app.Start()) { throw 'The framework process did not start.' }
    $null = $app.Handle
    return $app
}

function Start-Guard([string]$Directory) {
    # The guard holds every identity file and its folders from before the launch until the finalizer has finished (L2-V-002).
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $python
    $info.Arguments = Native-Arguments @('-B',$guardScript,'--launch',(Join-Path $buildDirectory 'launch.json'),'--candidate',$Candidate,'--out',$Directory)
    $info.UseShellExecute = $false
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $guard = New-Object Diagnostics.Process
    $guard.StartInfo = $info
    if (-not $guard.Start()) { throw 'The file guard did not start; no gate evidence.' }
    $null = $guard.Handle
    $line = $guard.StandardOutput.ReadLineAsync()
    if (-not $line.Wait(60000)) { throw 'The file guard was not ready within 60 s; no gate evidence.' }
    if ($line.Result -cne 'ready') { throw 'The file guard refused to protect the build; see its message above. No gate evidence.' }
    return $guard
}
function Stop-Guard($Guard) {
    # Raw ASCII bytes; the guard also accepts the byte-order mark that the writer may send first.
    $release = [Text.Encoding]::ASCII.GetBytes("release`n")
    $Guard.StandardInput.BaseStream.Write($release, 0, $release.Length)
    $Guard.StandardInput.BaseStream.Flush()
    $Guard.StandardInput.Close()
    if (-not $Guard.WaitForExit(30000)) { throw 'The file guard did not release within 30 s; the series is stopped.' }
    if ($Guard.ExitCode -ne 0) { throw "The file guard exited $($Guard.ExitCode) at release: a protected file changed identity. The series is stopped." }
}

$ownership = New-Object Threading.Mutex($false, "Global\$session")
try { $owned = $ownership.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned = $true }
if (-not $owned) { $ownership.Dispose(); throw 'Another run_scene.ps1 invocation is running; start this one after it has ended.' }
try {
    Assert-Idle
    if ($captureMode) { Assert-NoSession }
    $stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ')
    $base = Join-Path $repository "work/loop-memory/perf/renderer/$Candidate"
    $runDirectories = @()
    $scenes = if ($Validation) { @('w1','w2','w3','w4') } elseif ($Geometry) { @('w1') } else { @($Scene) }
    $count = if ($Geometry -or $Validation -or $Short) { 1 } else { $Runs }
    $traceLength = if ($Short) { 30000 } elseif ($Validation) { 20000 } else { $TraceMs }
    $total = $count * @($scenes).Count
    $completed = 0
    foreach ($currentScene in $scenes) {
        for ($i = 1; $i -le $count; $i++) {
            Assert-Idle
            if ($captureMode) { Assert-NoSession }
            $runId = "$stamp-$currentScene-$i"
            $directory = Join-Path $base $runId
            New-Item -ItemType Directory -Path $directory | Out-Null
            if (@(Get-ChildItem -LiteralPath $directory -Force).Count -ne 0) { throw 'The new run directory is not empty.' }
            $arguments = @($launch.arguments)
            foreach ($argument in $launch.run_arguments) { $arguments += $argument.Replace('{out}', $directory) }
            if ($Validation) { $arguments += @($launch.validation_arguments) }
            $arguments += @($launch.separator)
            $appMode = if ($Geometry) { 'geometry' } else { 'run' }
            $arguments += @('--l2-mode',$appMode,'--l2-out',$directory,'--l2-run-id',$runId,'--l2-dll',$launch.dll)
            if (-not $Geometry) { $arguments += @('--l2-scene',$currentScene,'--l2-trace-ms',"$traceLength",'--l2-preroll-ms',"$PrerollMs") }
            if (-not $Geometry -and $currentScene -eq 'w3') { $arguments += @('--l2-turn-ms',(Number-Argument $TurnMs)) }
            $arguments += @('--l2-gpu-validation',$(if ($Validation) { '1' } else { '0' }), '--l2-conditions',$(if ($Validation) { 'record' } else { 'enforce' }))
            if ($Inject) { $arguments += @('--l2-inject',$Inject) }
            foreach ($condition in $Declare) { $arguments += @('--l2-declare',$condition) }
            if ($NoVram) { $arguments += '--l2-no-vram' }
            if ($DebugHalfTarget) { $arguments += '--l2-debug-half-target' }
            $process = $null
            $capture = $null
            $guard = $null
            try {
                $guard = Start-Guard $directory
                # Process creation times may come from the coarse system clock; the pause keeps the app's creation time after the guard's precise ready time.
                Start-Sleep -Milliseconds 100
                $process = Start-App $arguments ([bool]$Validation)
                $launchedPid = $process.Id
                $launchedCreated = $process.StartTime.ToFileTimeUtc()
                $csv = Join-Path $directory 'presentmon.csv'
                if ($captureMode) {
                    $captureArguments = @('--v1_metrics','--qpc_time','--process_id',"$launchedPid",'--output_file',$csv,'--session_name',$session)
                    $capture = Start-Process -FilePath $presentMon -ArgumentList (Native-Arguments $captureArguments) -PassThru -WindowStyle Hidden
                    $null = $capture.Handle
                }
                if (-not $process.WaitForExit($PrerollMs + $traceLength + 300000)) { throw 'The framework did not exit within 300 s after its preroll and trace; no gate evidence.' }
                $appExit = $process.ExitCode
                if ($captureMode) {
                    Start-Sleep -Seconds 2
                    if ($capture.HasExited) { throw "PresentMon exited $($capture.ExitCode) before its session was stopped; no gate evidence." }
                    $stopCode = Stop-Capture $session
                    if ($stopCode -ne 0) { throw "Stopping the PresentMon session failed (exit $stopCode); no gate evidence." }
                    if (-not $capture.WaitForExit(60000)) { throw 'PresentMon did not exit within 60 s of its session stop; no gate evidence.' }
                    if ($capture.ExitCode -ne 0) { throw "PresentMon exited $($capture.ExitCode); no gate evidence." }
                    $left = Trace-Session $session
                    if ($left -ne 'missing') { throw "The PresentMon session was not removed ($left); no gate evidence." }
                    $stream = [IO.File]::OpenRead($csv)
                    try { $complete = $stream.Length -gt 0 -and $stream.Seek(-1, [IO.SeekOrigin]::End) -ge 0 -and $stream.ReadByte() -eq 10 } finally { $stream.Dispose() }
                    if (-not $complete) { throw 'The PresentMon CSV does not end with a complete row; no gate evidence.' }
                }
                if ($appExit -eq 3) { throw 'The app exited 3: the framework window left the foreground or a condition sample failed, or the DLL ended the process after an unconfirmed drain (then no harness.json exists). No gate evidence.' }
                $finalArguments = @('-B',$finalizer,$directory,'--candidate',$Candidate,'--mode',$mode,'--launched-pid',"$launchedPid",'--launched-created',"$launchedCreated",'--app-exit',"$appExit")
                if ($PSBoundParameters.ContainsKey('Adapter')) { $finalArguments += @('--adapter',$Adapter) }
                if ($mode -eq 'run' -and $appExit -in @(0,2)) {
                    if ($Inject) { $finalArguments += '--fault-injection' }
                    else {
                        $answer = Read-Host "$runId`: did you watch the whole run, with nothing covering any part of the framework window? Type yes to keep it"
                        if ("$answer".Trim() -ne 'yes') { throw "$runId was not confirmed by the operator; no gate evidence." }
                        $overlay = "$Overlays; operator-declared after the run: watched throughout, no visible obstruction; automatic checks sample visibility every 100 ms and do not prove continuous full-area visibility"
                        $finalArguments += @('--overlays',$overlay)
                    }
                }
                & python @finalArguments
                $finalExit = $LASTEXITCODE
                Stop-Guard $guard
                if ($finalExit -eq 5) {
                    $refusalPath = Join-Path $directory 'refusal.json'
                    if (Test-Path -LiteralPath $refusalPath -PathType Leaf) {
                        $refusal = Get-Content -LiteralPath $refusalPath -Raw -Encoding UTF8 | ConvertFrom-Json
                        Write-Host "$runId`: refused: $($refusal.reasons -join ', ')"
                    }
                    throw 'The finalizer refused this run; the series is stopped.'
                }
                if ($finalExit -eq 2 -and -not $Inject) { throw 'A label or geometry check failed; the series is stopped.' }
                if ($finalExit -ne 0 -and $finalExit -ne 2) { throw "The finalizer exited $finalExit." }
                if ($mode -eq 'run') { $runDirectories += $directory }
                Write-Host "$runId`: framework exit $appExit, finalizer exit $finalExit ($mode)."
            } finally {
                # Every cleanup step is guarded and bounded; only this invocation's capture is stopped.
                try { if ($process -and -not $process.HasExited) { $process.Kill(); $null = $process.WaitForExit(5000) } } catch { Write-Warning "Stopping the framework failed: $($_.Exception.Message)" }
                if ($capture) {
                    try {
                        if (-not $capture.HasExited) {
                            $null = Stop-Capture $session
                            if (-not $capture.WaitForExit(30000)) { $capture.Kill(); $null = $capture.WaitForExit(5000) }
                        }
                    } catch { Write-Warning "Stopping PresentMon failed: $($_.Exception.Message)" }
                    try { if (-not $capture.HasExited) { $capture.Kill(); $null = $capture.WaitForExit(5000) } } catch { Write-Warning "Killing PresentMon failed: $($_.Exception.Message)" }
                    Remove-Session $session
                }
                try { if ($guard -and -not $guard.HasExited) { $guard.Kill(); $null = $guard.WaitForExit(5000) } } catch { Write-Warning "Stopping the file guard failed: $($_.Exception.Message)" }
                if ($process) { $process.Dispose() }
                if ($capture) { $capture.Dispose() }
                if ($guard) { $guard.Dispose() }
            }
            $completed++
            if ($completed -lt $total) { Start-Sleep -Seconds 20 }
        }
    }
} finally {
    try { $ownership.ReleaseMutex() } finally { $ownership.Dispose() }
}
if ($mode -eq 'run') {
    $summaryDirectory = Join-Path $base "$stamp-$Scene-summary"
    New-Item -ItemType Directory -Path $summaryDirectory | Out-Null
    $summaryPath = Join-Path $summaryDirectory 'summary.json'
    & python -B (Join-Path $repository 'tools/perf/renderer_gate.py') @runDirectories --out $summaryPath
    if ($LASTEXITCODE -ne 0) { throw "renderer_gate.py exited $LASTEXITCODE" }
    $summary = Get-Content -LiteralPath $summaryPath -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($result in $summary.scenes) {
        Write-Host "$($result.scene): $($result.verdict), fps=$($result.pooled.fps), p99_ms=$($result.pooled.p99_ms), vram_peak_mb=$($result.vram_peak_mb)"
        foreach ($run in $result.runs) { Write-Host "  $($run.run_id): fps=$($run.fps), p99_ms=$($run.p99_ms), vram_peak_mb=$($run.vram_peak_mb)" }
    }
    foreach ($invalid in $summary.invalid_runs) { Write-Host "$($invalid.run_id): invalid: $($invalid.reasons -join ', ')" }
    foreach ($unreadable in $summary.unreadable) { Write-Host "unreadable run: $($unreadable.reason)" }
    Write-Host "Gate summary: $summaryPath"
}
