param(
    [Parameter(Mandatory=$true)][ValidateSet('w1','w2','w3','w4')][string]$Scene,
    [ValidateRange(1,100)][int]$Runs = 3,
    [ValidateSet('corrupt-label','swap-same-colour','delay-adoption','stale-binding')][string]$Inject,
    [string[]]$Declare = @(),
    [string]$Overlays,
    [ValidateRange(1,86400)][double]$Duration = 192,
    [ValidateRange(0,60)][double]$Preroll = 4
)
$ErrorActionPreference = 'Stop'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'PresentMon requires administrator rights. Run this script in an administrator PowerShell.'
}
$repository = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../..')).Path
$probe = Join-Path $PSScriptRoot 'build/sb_probe.exe'
$presentMon = Join-Path $env:ProgramFiles 'Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe'
if (-not (Test-Path -LiteralPath $probe)) { throw 'Build the probe in probe/build with build.cmd first.' }
if (-not (Test-Path -LiteralPath $presentMon)) { throw 'PresentMon 2.6.0.0 was not found at the pinned location.' }
if ($Inject -and $Scene -notin @('w3','w4')) { throw 'Fault injection requires W3 or W4.' }
if (-not $Inject -and -not $Overlays) { throw "State the overlay configuration with -Overlays, for example -Overlays 'none running'." }
if (@($Declare | Where-Object { $_ -like 'overlays=*' }).Count) { throw 'Give the overlay configuration with -Overlays, not -Declare.' }
function Native-Arguments([string[]]$Values) {
    # CommandLineToArgvW quoting, including a trailing backslash before the quote.
    return (($Values | ForEach-Object { '"' + (($_ -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"' }) -join ' ')
}
function Number-Argument([double]$Value) { return $Value.ToString('R', [Globalization.CultureInfo]::InvariantCulture) }
$stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ')
$base = Join-Path $repository 'work/loop-memory/perf/renderer/sb'
$summaryDirectory = Join-Path $base "$stamp-$Scene-summary"
New-Item -ItemType Directory -Path $summaryDirectory | Out-Null
$runDirectories = @()
for ($i = 1; $i -le $Runs; $i++) {
    $runId = "$stamp-$Scene-$i"
    $directory = Join-Path $base $runId
    New-Item -ItemType Directory -Path $directory | Out-Null
    $arguments = @('--scene', $Scene, '--duration', (Number-Argument $Duration), '--preroll', (Number-Argument $Preroll), '--out', $directory, '--run-id', $runId)
    if ($Inject) { $arguments += @('--inject', $Inject) }
    foreach ($condition in $Declare) { $arguments += @('--declare', $condition) }
    # No -WindowStyle: Windows applies it to the probe's first window, and a hidden
    # window's presents are not representative. -NoNewWindow keeps the console here.
    $process = Start-Process -FilePath $probe -ArgumentList (Native-Arguments $arguments) -WorkingDirectory $repository -PassThru -NoNewWindow
    # Cache the handle now; otherwise Windows PowerShell can report ExitCode as empty after exit.
    $null = $process.Handle
    $csv = Join-Path $directory 'presentmon.csv'
    # Capture until the probe exits, so a slow startup cannot cut the end of the gate interval.
    $captureArguments = @('--v1_metrics','--qpc_time','--process_id',"$($process.Id)",'--output_file',$csv,'--terminate_on_proc_exit')
    $capture = $null
    try {
        $capture = Start-Process -FilePath $presentMon -ArgumentList (Native-Arguments $captureArguments) -PassThru -WindowStyle Hidden
        $null = $capture.Handle
        $process.WaitForExit()
        # Check the probe first: a probe that failed before PresentMon saw it never ends the
        # capture. Exit 2 is an intentional label-check failure in a negative test; exit 3
        # means the window was hidden, minimised, cloaked, covered or not in the foreground
        # in some sample of the trace.
        if ($process.ExitCode -eq 3) { throw 'The probe window was not visible in the foreground throughout the trace; no gate evidence.' }
        if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 2) { throw "Probe exited $($process.ExitCode)." }
        if (-not $capture.WaitForExit(60000)) { throw 'PresentMon did not stop within 60 s of the probe exit; no gate evidence.' }
        if ($capture.ExitCode -ne 0) { throw "PresentMon exited $($capture.ExitCode); no gate evidence." }
        $runPath = Join-Path $directory 'run.json'
        $chains = Import-Csv -LiteralPath $csv | Where-Object { $_.ProcessID -eq "$($process.Id)" -and $_.SwapChainAddress } | Group-Object SwapChainAddress | Sort-Object Count -Descending
        if (-not $chains) { throw 'PresentMon has no swap-chain rows for the probe PID.' }
        $rows = @(@($chains)[0].Group)
        if ($null -eq $rows[0].PSObject.Properties['Dropped'] -or $null -eq $rows[0].PSObject.Properties['QPCTime']) {
            throw 'The PresentMon CSV has no Dropped or QPCTime column; displayed presents cannot be checked.'
        }
        # Every second of the gate interval [T0+10 s, T0+190 s), as renderer_gate.py counts it,
        # needs a displayed present: displayed preroll or warm-up frames cannot stand in for it.
        $marks = [IO.File]::ReadAllText($runPath) | ConvertFrom-Json
        $frequency = [int64]$marks.qpc_frequency
        $begin = [int64]$marks.markers.trace_start_qpc + 10 * $frequency
        $end = [Math]::Min([int64]$marks.markers.trace_start_qpc + 190 * $frequency, [int64]$marks.markers.trace_stop_qpc)
        $inInterval = 0; $dropped = 0; $blind = 0; $seconds = 0
        if ($end -gt $begin) {
            $seconds = [int][Math]::Ceiling(($end - $begin) / [double]$frequency)
            $shown = New-Object bool[] $seconds
            foreach ($row in $rows) {
                $t = [int64]$row.QPCTime
                if ($t -lt $begin -or $t -ge $end) { continue }
                $inInterval++
                if ($row.Dropped -eq '0') { $shown[[int][Math]::Floor(($t - $begin) / [double]$frequency)] = $true } else { $dropped++ }
            }
            $blind = @($shown | Where-Object { -not $_ }).Count
        }
        $modes = ($rows | Group-Object PresentMode | Sort-Object Count -Descending | ForEach-Object { "$($_.Name) $($_.Count)" }) -join ', '
        Write-Host "$runId`: PresentMon gate interval $inInterval presents, $dropped not displayed, $blind of $seconds seconds without a displayed present; present modes: $modes"
        if ($blind -gt 0) { throw "PresentMon shows $blind second(s) of the gate interval without a displayed present; no gate evidence." }
        # The automatic visibility checks are samples, not proof of continuous full-area
        # visibility, so a formal run also needs the operator's confirmation, given after the
        # run, that it was watched throughout with nothing covering the probe (Astra ruling
        # 20261003T033021Z-274f20af). The summary publishes it as declared.overlays.
        if ($Inject) { $overlay = 'fault-injection run; not gate evidence' }
        else {
            $answer = Read-Host "$runId`: did you watch the whole run, with nothing covering any part of the probe window? Type yes to keep it"
            if ("$answer".Trim() -ne 'yes') { throw "$runId was not confirmed by the operator; no gate evidence." }
            $overlay = "$Overlays; operator-declared after the run: watched throughout, no visible obstruction; automatic checks sample visibility every 100 ms and do not prove continuous full-area visibility"
        }
        # Replace the placeholders in the text: a ConvertFrom/ConvertTo-Json round trip in
        # Windows PowerShell 5.1 can change numbers in run.json.
        $text = [IO.File]::ReadAllText($runPath)
        if (-not $text.Contains('"FILL-FROM-CSV"') -or -not $text.Contains('"OPERATOR-CONFIRMATION-PENDING"')) { throw 'run.json lacks the swap-chain or operator-confirmation placeholder.' }
        $text = $text.Replace('"FILL-FROM-CSV"', '"' + @($chains)[0].Name + '"').Replace('"OPERATOR-CONFIRMATION-PENDING"', (ConvertTo-Json -InputObject $overlay -Compress))
        [IO.File]::WriteAllText($runPath, $text, [Text.UTF8Encoding]::new($false))
        $run = $text | ConvertFrom-Json
        $runDirectories += $directory
        Write-Host "$runId`: probe exit $($process.ExitCode), label check $($run.label_check.status), vram_peak_mb $($run.vram_peak_mb)"
    } finally {
        if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        if ($capture -and -not $capture.HasExited) { $capture.Kill(); $capture.WaitForExit() }
    }
    if ($i -lt $Runs) { Start-Sleep -Seconds 20 }
}
$summaryPath = Join-Path $summaryDirectory 'summary.json'
& python (Join-Path $repository 'tools/perf/renderer_gate.py') @runDirectories --out $summaryPath
if ($LASTEXITCODE -ne 0) { throw "renderer_gate.py exited $LASTEXITCODE" }
$summary = Get-Content -LiteralPath $summaryPath -Raw | ConvertFrom-Json
foreach ($result in $summary.scenes) {
    Write-Host "$($result.scene): $($result.verdict), fps=$($result.pooled.fps), p99_ms=$($result.pooled.p99_ms), vram_peak_mb=$($result.vram_peak_mb)"
    foreach ($run in $result.runs) { Write-Host "  $($run.run_id): fps=$($run.fps), p99_ms=$($run.p99_ms), vram_peak_mb=$($run.vram_peak_mb)" }
}
foreach ($invalid in $summary.invalid_runs) { Write-Host "$($invalid.run_id): invalid: $($invalid.reasons -join ', ')" }
foreach ($unreadable in $summary.unreadable) { Write-Host "unreadable run: $($unreadable.reason)" }
Write-Host "Gate summary: $summaryPath"
