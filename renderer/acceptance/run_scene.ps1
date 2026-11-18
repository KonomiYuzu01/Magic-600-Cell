# C9 acceptance copy of work/experiments/renderer-l2/run_scene.ps1 (see PROVENANCE.md). One invocation makes one
# cold run, or one untimed geometry or lattice check, of W3 or one W-J fixture (S4, I_a, I_b) on the S-A2 host
# (Godot 4.7.2 .NET, net8.0) with the product core renderer/core (ABI 3). -DryRun validates every input and prints
# the planned commands; it starts no Godot, PresentMon, logman, file guard or D3D12 device and creates nothing.
param(
    [Parameter(Mandatory=$true)][ValidateSet('w3','S4','I_a','I_b')][string]$Scene,
    [Parameter(Mandatory=$true)][string]$Build,
    [ValidateSet('run','geometry','lattice')][string]$Mode = 'run',
    [string]$Data,
    [ValidateSet('corrupt-label','corrupt-index','swap-same-colour','stale-pose','delay-adoption','stale-binding')][string]$Inject,
    [string[]]$Declare = @(),
    [string]$Overlays,
    [ValidateRange(1000,3600000)][int]$TraceMs = 192000,
    [ValidateRange(0,60000)][int]$PrerollMs = 4000,
    [double]$TurnMs = 190,
    [string]$Adapter,
    [switch]$Unattended,
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
$wj = $Scene -ne 'w3'
$appScene = if ($wj) { 'wj' } else { 'w3' }
if ([double]::IsNaN($TurnMs) -or [double]::IsInfinity($TurnMs) -or $TurnMs -le 0 -or $TurnMs -gt 10000) { throw '-TurnMs must be finite and in (0,10000].' }
if ($Mode -eq 'lattice' -and -not $wj) { throw 'The lattice check needs a W-J fixture: -Scene S4, I_a or I_b.' }
if ($Inject) {
    if ($Mode -ne 'run') { throw 'Fault injection needs -Mode run.' }
    $faults = if ($wj) { @('corrupt-index','swap-same-colour','stale-pose','delay-adoption','stale-binding') } else { @('corrupt-label','swap-same-colour','delay-adoption','stale-binding') }
    if ($faults -cnotcontains $Inject) { throw "The injection $Inject does not apply to $Scene." }
}
if (-not $wj -and $PSBoundParameters.ContainsKey('Data')) { throw '-Data is for the W-J fixtures only.' }
if (@($Declare | Where-Object { $_ -like 'overlays=*' }).Count) { throw 'Give the overlay configuration with -Overlays, not -Declare.' }
if ($PSBoundParameters.ContainsKey('Adapter') -and [string]::IsNullOrWhiteSpace($Adapter)) { throw '-Adapter must be nonempty.' }
$gateRun = $Mode -eq 'run' -and -not $Inject
if ($gateRun) {
    if ([string]::IsNullOrWhiteSpace($Overlays)) { throw "State the overlay configuration with -Overlays, for example -Overlays 'none running'." }
    # The gate marks a run without these two declarations conditions-missing; the app turns true and false into booleans.
    foreach ($fact in 'frame_generation','upscaling') {
        if (@($Declare | Where-Object { $_ -cmatch "^$fact=(true|false)$" }).Count -ne 1) { throw "Declare $fact once as true or false, for example -Declare @('frame_generation=false','upscaling=false')." }
    }
}
$captureMode = $Mode -eq 'run'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
$administrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($captureMode -and -not $administrator -and -not $DryRun) { throw 'PresentMon requires administrator rights. Run this script in an administrator PowerShell.' }
$repository = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$finalizer = Join-Path $PSScriptRoot 'finalize_run.py'
$guardScript = Join-Path $PSScriptRoot 'file_guard.py'
$hostScript = Join-Path $PSScriptRoot 'prepare_host.py'
$dataScript = Join-Path $PSScriptRoot 'prepare_data.py'
$gateScript = Join-Path $repository 'tools/perf/renderer_gate.py'
$python = (Get-Command python -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
$buildDirectory = (Resolve-Path -LiteralPath $Build).Path
$launchPath = Join-Path $buildDirectory 'launch.json'
$launch = Get-Content -LiteralPath $launchPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($launch.format -ne 'magic600-l2-launch-v1' -or $launch.candidate -ne 'sa2') { throw 'launch.json is not an S-A2 level 2 launch file.' }
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
if ($launch.separator.Count -ne 1 -or $launch.separator[0] -ne '--') { throw 'launch.json has the wrong framework argument separator.' }
# The ABI 4 core reads these eight shaders from the DLL's folder (renderer/core/check_core.py SHADERS).
$nativeDirectory = Split-Path -Parent $launch.dll
foreach ($shader in 'count_vs.dxil','draw_ps.dxil','draw_vs.dxil','geometry_cs.dxil','id_ps.dxil','lattice_cs.dxil','product_ps.dxil','product_vs.dxil') {
    if (-not (Test-Path -LiteralPath (Join-Path $nativeDirectory $shader) -PathType Leaf)) { throw "The build lacks $shader next to sa2_interop.dll." }
}
$dataDirectory = $null
if ($wj) {
    $dataText = if ($Data) { $Data } else { Join-Path $repository 'work/loop-memory/c9-acceptance/wj-data' }
    if (-not (Test-Path -LiteralPath $dataText -PathType Container)) { throw "The W-J data directory $dataText does not exist; prepare it with renderer/acceptance/prepare_data.py." }
    $dataDirectory = (Resolve-Path -LiteralPath $dataText).Path
}
$presentMon = Join-Path $env:ProgramFiles 'Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe'
$presentMonFound = Test-Path -LiteralPath $presentMon -PathType Leaf
if ($captureMode -and -not $presentMonFound -and -not $DryRun) { throw 'PresentMon 2.6.0.0 was not found at the pinned location.' }
$logman = Join-Path $env:SystemRoot 'System32/logman.exe'
$session = 'magic600-sa2-capture'

function Native-Arguments([string[]]$Values) {
    # CommandLineToArgvW quoting, including a trailing backslash before the quote.
    return (($Values | ForEach-Object { '"' + (($_ -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"' }) -join ' ')
}
function Number-Argument([double]$Value) { return $Value.ToString('R', [Globalization.CultureInfo]::InvariantCulture) }
function Invoke-Check([string]$Name, [string[]]$Values) {
    # A bounded-output Python check that starts no device; its lines are shown indented. Native stderr must not stop
    # the script before the exit code is read, so errors are collected as text.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& $python @Values 2>&1 | ForEach-Object { "$_" })
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    foreach ($line in $output) { Write-Host "  $line" }
    if ($code -ne 0) { throw "$Name failed (exit $code); nothing was started." }
    return $output
}
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
        elseif ($name -match '^(powershell|pwsh)\.exe$' -and $command -match 'run_scene\.ps1\b' -and $command -notmatch '-DryRun\b') { $reason = 'another runner' }
        if ($reason) { throw "Cannot start while $reason runs: $name (PID $($other.ProcessId))." }
    }
}
function Start-App([string[]]$Values) {
    # Apply environment overrides only to this child, including null removals.
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $launch.executable
    $info.WorkingDirectory = $launch.working_directory
    $info.Arguments = Native-Arguments $Values
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $false
    foreach ($entry in $launch.environment.PSObject.Properties) {
        if ($null -eq $entry.Value) { $info.EnvironmentVariables.Remove($entry.Name) }
        else { $info.EnvironmentVariables[$entry.Name] = [string]$entry.Value }
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
    $info.Arguments = Native-Arguments @('-B',$guardScript,'--launch',$launchPath,'--candidate','sa2','--out',$Directory)
    $info.UseShellExecute = $false
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $guard = New-Object Diagnostics.Process
    $guard.StartInfo = $info
    if (-not $guard.Start()) { $guard.Dispose(); throw 'The file guard did not start; no gate evidence.' }
    try {
        $null = $guard.Handle
        $line = $guard.StandardOutput.ReadLineAsync()
        if (-not $line.Wait(60000)) { throw 'The file guard was not ready within 60 s; no gate evidence.' }
        if ($line.Result -cne 'ready') { throw 'The file guard refused to protect the build; see its message above. No gate evidence.' }
    } catch {
        $failure = $_
        # The caller never receives a guard that did not become ready, so stop it here: it may already hold files.
        try { if (-not $guard.HasExited) { $guard.Kill(); $null = $guard.WaitForExit(5000) } } catch { Write-Warning "Stopping the file guard failed: $($_.Exception.Message)" }
        $guard.Dispose()
        throw $failure
    }
    return $guard
}
function Stop-Guard($Guard) {
    # Raw ASCII bytes; the guard also accepts the byte-order mark that the writer may send first.
    $release = [Text.Encoding]::ASCII.GetBytes("release`n")
    $Guard.StandardInput.BaseStream.Write($release, 0, $release.Length)
    $Guard.StandardInput.BaseStream.Flush()
    $Guard.StandardInput.Close()
    if (-not $Guard.WaitForExit(30000)) { throw 'The file guard did not release within 30 s; the run is stopped.' }
    if ($Guard.ExitCode -ne 0) { throw "The file guard exited $($Guard.ExitCode) at release: a protected file changed identity. The run is stopped." }
}

# Checks that start no device and write nothing, before every run and in the dry mode.
Write-Host 'build check:'
$null = Invoke-Check 'The build check' @('-B',$hostScript,'--verify',$buildDirectory,'--godot',$launch.executable)
if ($wj) {
    Write-Host 'W-J data check:'
    $null = Invoke-Check 'The W-J data check' @('-B',$dataScript,'--check','--data',$dataDirectory,'--menu',$Scene)
}

$stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ')
$label = if ($wj) { "wj-$Scene" } else { 'w3' }
$runId = if ($Mode -eq 'run') { "$stamp-$label" } else { "$stamp-$label-$Mode" }
$directory = Join-Path $repository "work/loop-memory/c9-acceptance/$stamp"
$arguments = @($launch.arguments)
foreach ($argument in $launch.run_arguments) { $arguments += $argument.Replace('{out}', $directory) }
$arguments += @($launch.separator)
$arguments += @('--l2-mode',$Mode,'--l2-out',$directory,'--l2-run-id',$runId,'--l2-dll',$launch.dll,'--l2-scene',$appScene)
if ($wj) { $arguments += @('--l2-menu',$Scene,'--l2-data',$dataDirectory) }
# The untimed checks run outside any trace; a 1 s trace keeps the W-J load's preserved readback small.
if ($Mode -eq 'run') { $arguments += @('--l2-trace-ms',"$TraceMs",'--l2-preroll-ms',"$PrerollMs") } else { $arguments += @('--l2-trace-ms','1000') }
$arguments += @('--l2-turn-ms',(Number-Argument $TurnMs),'--l2-gpu-validation','0','--l2-conditions','enforce')
if ($Inject) { $arguments += @('--l2-inject',$Inject) }
foreach ($condition in $Declare) { $arguments += @('--l2-declare',$condition) }
$csv = Join-Path $directory 'presentmon.csv'
$summaryPath = Join-Path $directory 'summary.json'
$operatorText = if ($Unattended) {
    "$Overlays; unattended: the owner was away and no operator watched the run; automatic checks sample visibility every 100 ms and do not prove continuous full-area visibility"
} else {
    "$Overlays; operator-declared after the run: watched throughout, no visible obstruction; automatic checks sample visibility every 100 ms and do not prove continuous full-area visibility"
}
$operatorArguments = @(if ($gateRun) { '--overlays', $operatorText; if ($Unattended) { '--unattended' } } elseif ($Mode -eq 'run') { '--fault-injection' })

if ($DryRun) {
    Write-Host 'file guard plan:'
    $planned = Invoke-Check 'The file guard plan' @('-B',$guardScript,'--plan','--launch',$launchPath,'--candidate','sa2')
    try { Assert-Idle; $idle = 'yes' } catch { $idle = "no: $($_.Exception.Message)" }
    $sceneText = if ($wj) { "$Scene (app scene wj, menu $Scene)" } else { 'w3' }
    Write-Host 'dry run: no process was started and no file or directory was created'
    Write-Host "  scene: $sceneText; mode: $Mode; run ID: $runId"
    Write-Host "  build: $buildDirectory"
    if ($wj) { Write-Host "  W-J data: $dataDirectory" }
    Write-Host "  run directory (created at the run): $directory"
    Write-Host "  administrator: $(if ($administrator) { 'yes' } else { 'no' })$(if ($captureMode) { ' (a run needs yes)' } else { ' (not needed)' })"
    Write-Host "  PresentMon 2.6.0.0: $(if ($presentMonFound) { 'found' } else { 'missing' })$(if ($captureMode) { '' } else { ' (not used)' })"
    Write-Host "  machine idle now: $idle"
    Write-Host "  protected files: $($planned.Count)"
    Write-Host "  launch environment overrides: $((@($launch.environment.PSObject.Properties) | ForEach-Object { $_.Name }) -join ', ')"
    Write-Host 'planned commands:'
    $step = 1
    if ($captureMode) { Write-Host "  $step. pause 20 s, then check that no build, framework, PresentMon or runner process and no trace session runs"; $step++ }
    Write-Host "  $step. guard: $python $(Native-Arguments @('-B',$guardScript,'--launch',$launchPath,'--candidate','sa2','--out',$directory))"; $step++
    Write-Host "  $step. app: $(Native-Arguments @($launch.executable)) $(Native-Arguments $arguments)"; $step++
    if ($captureMode) { Write-Host "  $step. PresentMon: $(Native-Arguments @($presentMon)) $(Native-Arguments @('--v1_metrics','--qpc_time','--process_id','<app PID>','--output_file',$csv,'--session_name',$session))"; $step++ }
    Write-Host "  $step. finalizer: $python $(Native-Arguments (@('-B',$finalizer,$directory,'--candidate','sa2','--mode',$Mode,'--launched-pid','<app PID>','--launched-created','<app start FILETIME>','--app-exit','<app exit>') + $operatorArguments))"; $step++
    if ($captureMode) { Write-Host "  $step. gate: $python $(Native-Arguments @('-B',$gateScript,$directory,'--out',$summaryPath))" }
    exit 0
}

$ownership = New-Object Threading.Mutex($false, "Global\$session")
try { $owned = $ownership.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned = $true }
if (-not $owned) { $ownership.Dispose(); throw 'Another run_scene.ps1 invocation is running; start this one after it has ended.' }
$finalExit = $null
try {
    # One run per invocation: the 20 s pause keeps level 2's gap between runs when invocations follow each other.
    if ($captureMode) { Start-Sleep -Seconds 20 }
    Assert-Idle
    if ($captureMode) { Assert-NoSession }
    New-Item -ItemType Directory -Path $directory | Out-Null
    if (@(Get-ChildItem -LiteralPath $directory -Force).Count -ne 0) { throw 'The new run directory is not empty.' }
    $process = $null
    $capture = $null
    $guard = $null
    try {
        $guard = Start-Guard $directory
        # Process creation times may come from the coarse system clock; the pause keeps the app's creation time after the guard's precise ready time.
        Start-Sleep -Milliseconds 100
        $process = Start-App $arguments
        $launchedPid = $process.Id
        $launchedCreated = $process.StartTime.ToFileTimeUtc()
        if ($captureMode) {
            $captureArguments = @('--v1_metrics','--qpc_time','--process_id',"$launchedPid",'--output_file',$csv,'--session_name',$session)
            $capture = Start-Process -FilePath $presentMon -ArgumentList (Native-Arguments $captureArguments) -PassThru -WindowStyle Hidden
            $null = $capture.Handle
        }
        $limit = if ($captureMode) { $PrerollMs + $TraceMs + 300000 } else { 300000 }
        if (-not $process.WaitForExit($limit)) { throw 'The framework did not exit within 300 s after its preroll and trace; no gate evidence.' }
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
        $finalArguments = @('-B',$finalizer,$directory,'--candidate','sa2','--mode',$Mode,'--launched-pid',"$launchedPid",'--launched-created',"$launchedCreated",'--app-exit',"$appExit")
        if ($PSBoundParameters.ContainsKey('Adapter')) { $finalArguments += @('--adapter',$Adapter) }
        if ($Mode -eq 'run' -and $appExit -in @(0,2)) {
            if ($gateRun -and -not $Unattended) {
                # Discard keys typed or pasted while the run was on screen (a right-click pastes), so only an answer
                # typed at this prompt counts. A host without a console input buffer keeps the old behaviour.
                try { $Host.UI.RawUI.FlushInputBuffer() } catch { }
                $answer = Read-Host "$runId`: did you watch the whole run, with nothing covering any part of the framework window? Type yes to keep it"
                if ("$answer".Trim() -ne 'yes') { throw "$runId was not confirmed by the operator; no gate evidence." }
            }
            $finalArguments += $operatorArguments
        }
        & $python @finalArguments
        $finalExit = $LASTEXITCODE
        try { Stop-Guard $guard } catch {
            # The guard saw a protected file change after the finalizer wrote its record: keep nothing that reads as accepted.
            foreach ($name in @('run.json','fault-run.json',"$Mode-record.json")) {
                $written = Join-Path $directory $name
                if (Test-Path -LiteralPath $written -PathType Leaf) { Move-Item -LiteralPath $written -Destination "$written.rejected" }
            }
            throw
        }
        if ($finalExit -eq 5) {
            $refusalPath = Join-Path $directory 'refusal.json'
            if (Test-Path -LiteralPath $refusalPath -PathType Leaf) {
                $refusal = Get-Content -LiteralPath $refusalPath -Raw -Encoding UTF8 | ConvertFrom-Json
                Write-Host "$runId`: refused: $($refusal.reasons -join ', ')"
            }
            throw 'The finalizer refused this run.'
        }
        if ($finalExit -ne 0 -and $finalExit -ne 2) { throw "The finalizer exited $finalExit." }
        Write-Host "$runId`: framework exit $appExit, finalizer exit $finalExit ($Mode)."
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
} finally {
    try { $ownership.ReleaseMutex() } finally { $ownership.Dispose() }
}
if ($Mode -ne 'run') {
    $record = Get-Content -LiteralPath (Join-Path $directory "$Mode-record.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    Write-Host "$runId`: $Mode check $($record.status). Record: $(Join-Path $directory "$Mode-record.json")"
    if ($finalExit -eq 2) { exit 2 }
    exit 0
}
if (-not $gateRun) {
    # A fault-injection run: its record is fault-run.json, which the gate never reads; no gate summary is made.
    $fault = Get-Content -LiteralPath (Join-Path $directory 'fault-run.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $checkText = "label check $($fault.label_check.status)"
    if ($wj) { $checkText += ", pose check $($fault.pose_check.status)" }
    Write-Host "$runId`: fault-injection run ($Inject), never gate evidence: $checkText"
    if ($finalExit -eq 2) { exit 2 }
    exit 0
}
$run = Get-Content -LiteralPath (Join-Path $directory 'run.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$checkText = "label check $($run.label_check.status)"
if ($wj) { $checkText += ", pose check $($run.pose_check.status)" }
$checkText += "; watched: $(if ($Unattended) { 'no (unattended)' } else { 'yes (operator-declared)' })"
Write-Host "$runId`: $checkText"
# One run's gate summary (verdict insufficient-runs by design); README.md gives the pooled three-run command.
& $python -B $gateScript $directory --out $summaryPath
if ($LASTEXITCODE -ne 0) { throw "renderer_gate.py exited $LASTEXITCODE" }
$summary = Get-Content -LiteralPath $summaryPath -Raw -Encoding UTF8 | ConvertFrom-Json
foreach ($result in $summary.scenes) {
    foreach ($one in $result.runs) {
        $vramBar = if ($null -eq $one.vram_peak_mb) { 'unknown' } elseif ([double]$one.vram_peak_mb -le 7168) { 'within 7168 MB' } else { 'above 7168 MB' }
        $barText = "fps $($one.fps) (bar >= 30), p99 $($one.p99_ms) ms (bar <= 33.3), peak VRAM $($one.vram_peak_mb) MB ($vramBar)"
        Write-Host "  $($one.run_id): $barText; run meets both frame bars: $($one.met)"
    }
    Write-Host "  $($result.scene): verdict for this run alone: $($result.verdict)"
}
foreach ($invalid in $summary.invalid_runs) { Write-Host "$($invalid.run_id): invalid: $($invalid.reasons -join ', ')" }
foreach ($unreadable in $summary.unreadable) { Write-Host "unreadable run: $($unreadable.reason)" }
Write-Host "Run summary: $summaryPath"
if ($finalExit -eq 2) { exit 2 }
