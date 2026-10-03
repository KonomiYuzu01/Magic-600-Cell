param(
    [Parameter(Mandatory=$true)][ValidateSet('w1','w2','w3','w4')][string]$Scene,
    [ValidateRange(1,100)][int]$Runs = 3,
    [ValidateSet('corrupt-label','swap-same-colour','delay-adoption','stale-binding')][string]$Inject,
    [string[]]$Declare = @(),
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
        $capture.WaitForExit()
        if ($capture.ExitCode -ne 0) { throw "PresentMon exited $($capture.ExitCode); no gate evidence." }
        # Exit 2 is an intentional label-check failure in a negative test.
        if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 2) { throw "Probe exited $($process.ExitCode)." }
        $runPath = Join-Path $directory 'run.json'
        $chains = Import-Csv -LiteralPath $csv | Where-Object { $_.ProcessID -eq "$($process.Id)" -and $_.SwapChainAddress } | Group-Object SwapChainAddress | Sort-Object Count -Descending
        if (-not $chains) { throw 'PresentMon has no swap-chain rows for the probe PID.' }
        # Replace the placeholder in the text: a ConvertFrom/ConvertTo-Json round trip in
        # Windows PowerShell 5.1 can change numbers in run.json.
        $text = [IO.File]::ReadAllText($runPath)
        if (-not $text.Contains('"FILL-FROM-CSV"')) { throw 'run.json has no swap-chain placeholder.' }
        [IO.File]::WriteAllText($runPath, $text.Replace('"FILL-FROM-CSV"', '"' + @($chains)[0].Name + '"'), [Text.UTF8Encoding]::new($false))
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
