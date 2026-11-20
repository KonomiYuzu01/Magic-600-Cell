<#
C9 owner batch, unattended part: U1 to U8 (runbook.py's labels) in order with one command.

Run it from an administrator PowerShell started on the desktop, in the repository, after A1 to A3:

    & tools\perf\c9\unattended.ps1 -SceneBuild <acceptance build> -StageBuild <stage build> -ProductBuild <product build>

-SceneBuild is U4's route-B acceptance build, -StageBuild U5's stage build (both under work\build\c9-acceptance), and
-ProductBuild the product build (work\build\c9-product\<stamp>) that U3, U6, U7 and U8 launch and U7's fault trials
run. A step whose build is not given is refused before anything starts.
-Steps U7,U8 runs only those steps (always in U1..U8 order), for example to redo a step that failed.
-Suffix r2 appends "-r2" to every run folder, capture, record and gate summary this script writes, so a rerun never
meets an earlier run's files.
-Retries (0 to 2, default 2) is how often an aborted shell run is run again.
-DryRun starts nothing: it checks the inputs, prints every command and runs each tool's own dry run.

Every step writes one line to work\c9-runs\unattended.log (UTC time, step, exit code, run folder, notes). A failing
step is recorded and the script goes on with the next one. Child output goes to work\c9-runs\unattended\<stamp>\.

Before each measured run the script waits up to 30 min for the quiet-machine gate (run_probe.py --check-only, as
runbook.py uses it; the owner idle for 300 s is part of it) and logs a wait that ran out. In a shell run (U3, U6, U7,
U8) foreground.py starts first and takes the session's last-input time, then PresentMon and the poster start in this
console without a window, and launch.py starts last, so the shell gets the focus. For U3, U7 and U8 foreground.py makes
the shell topmost and asks once for the foreground (SetForegroundWindow; no foreground-lock workaround). When the
owner's input arrives, or the shell loses the foreground, before the run's measurement is over, the run is aborted:
the poster is stopped, the shell closed, PresentMon stopped, the run logged as exit=aborted, its folder and capture
set aside as <name>.aborted-<n>, and after the gate it runs again, at most -Retries times. The script closes the
shells itself (CloseMainWindow, the same message as the close button). Leave the machine alone until it prints
"unattended: done".
#>
[CmdletBinding()]
param(
    [switch]$DryRun,
    [ValidateSet('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8')]
    [string[]]$Steps = @('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8'),
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_-]*$')]
    [string]$Suffix = '',
    [string]$SceneBuild = '',
    [string]$StageBuild = '',
    [string]$ProductBuild = '',
    [ValidateRange(0, 2)]
    [int]$Retries = 2
)

$ErrorActionPreference = 'Continue'
$Root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\..'))
$Tag = if ($Suffix) { "-$Suffix" } else { '' }
$Runs = 'work\c9-runs'
$Acceptance = 'work\loop-memory\c9-acceptance'
$Fixtures = "$Acceptance\lx4-fixtures"
$Child = 'rust\target\release\c600-lx2-child.exe'
$Watcher = 'tools/perf/c9/foreground.py'
$PresentMon = 'C:\Program Files\Intel\PresentMon\PresentMonConsoleApplication\PresentMon-2.6.0-x64.exe'
$GodotName = 'Godot_v4.7.2-stable_mono_win64.exe'
$Declare = @('frame_generation=false', 'upscaling=false')
# foreground.py's exit codes: owner input, foreground not taken or lost (both abort a measuring run), no window.
$WatchAbort = @{ 10 = 'owner-input'; 11 = 'foreground-lost' }
$Stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
if ($DryRun) {
    $Log = "$Runs\unattended-dry.log"
    $Out = "$Runs\unattended-dry\$Stamp"
} else {
    $Log = "$Runs\unattended.log"
    $Out = "$Runs\unattended\$Stamp"
}
$Utf8 = New-Object Text.UTF8Encoding($false)
$Python = $null
$Cargo = $null
$script:Sessions = @()

function Get-Utc { [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ') }

function Write-Log([string]$Step, $Code, [string]$Run, [string]$Note) {
    if ($null -eq $Code -or "$Code" -eq '') { $Code = 'none' }
    if (-not $Run) { $Run = '-' }
    $line = '{0} {1} exit={2} run={3}' -f (Get-Utc), $Step, $Code, $Run
    if ($Note) { $line += " | $Note" }
    [IO.File]::AppendAllText((Join-Path $Root $Log), $line + "`n", $Utf8)
    Write-Host $line
}

function Get-Label([string]$Text) { ($Text -replace '[^A-Za-z0-9_-]+', '-').Trim('-').ToLowerInvariant() }

function Stop-Tree([int]$Id) { $null = & taskkill.exe /PID $Id /T /F 2>&1 }

function Start-Child([string]$File, [string[]]$Arguments, [string]$Label) {
    $base = Join-Path $Root "$Out\$Label"
    # Start-Process joins the arguments with spaces and does not quote them.
    $quoted = @($Arguments | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } })
    $process = Start-Process -FilePath $File -ArgumentList $quoted -WorkingDirectory $Root -NoNewWindow -PassThru `
        -RedirectStandardOutput "$base.out.txt" -RedirectStandardError "$base.err.txt"
    $null = $process.Handle  # keeps ExitCode readable after the exit
    $process
}

# A blocking command without a window; 'timeout' after $Cap seconds (the process tree is stopped).
function Invoke-Step([string]$Step, [string]$File, [string[]]$Arguments, [int]$Cap, [string]$Run) {
    $shown = "$(if ($File) { Split-Path -Leaf $File } else { '<missing>' }) $($Arguments -join ' ')"
    if ($DryRun) { Write-Host "  [$Step] $shown"; return 0 }
    Write-Host "$(Get-Utc) $Step`: $shown"
    $note = $null
    try {
        if (-not $File) { throw 'the program is not on PATH' }
        $process = Start-Child $File $Arguments (Get-Label $Step)
        if ($process.WaitForExit($Cap * 1000)) {
            $code = $process.ExitCode
        } else {
            Stop-Tree $process.Id
            $null = $process.WaitForExit(30000)
            $code = 'timeout'; $note = "stopped after $Cap s"
        }
    } catch {
        $code = 'error'; $note = $_.Exception.Message -replace '\s+', ' '
    }
    Write-Log $Step $code $Run $note
    $code
}

# Prints a tool's own dry run (DryRun only): exit code and last line; the full text goes to the dry folder.
function Invoke-Dry([string]$Label, [string]$File, [string[]]$Arguments) {
    $global:LASTEXITCODE = 0
    $text = (& $File @Arguments 2>&1 | ForEach-Object { "$_" }) -join "`n"
    $code = $LASTEXITCODE
    [IO.File]::WriteAllText((Join-Path $Root "$Out\$Label.txt"), $text + "`n", $Utf8)
    $last = $text -split "`n" | Where-Object { $_.Trim() } | Select-Object -Last 1
    Write-Host "      dry run $Label`: exit $code | $last"
}

function Get-Shell([datetime]$Since) {
    @(Get-Process -Name 'Godot*' -ErrorAction SilentlyContinue |
        Where-Object { try { $_.StartTime -ge $Since.AddSeconds(-5) } catch { $false } })
}

# CloseMainWindow posts WM_CLOSE to the shell's window, as its close button does; true once one was posted.
function Close-Shell([datetime]$Since) {
    $posted = $false
    foreach ($shell in Get-Shell $Since) {
        try {
            $shell.Refresh()
            if ($shell.MainWindowHandle -ne [IntPtr]::Zero -and $shell.CloseMainWindow()) { $posted = $true }
        } catch { }
    }
    $posted
}

function Stop-Session([string]$Session) {
    $null = & logman.exe query $Session -ets 2>&1
    if ($LASTEXITCODE -eq 0) { $null = & logman.exe stop $Session -ets 2>&1 }
}

function Stop-Capture($Capture, [string]$Session) {
    try {
        if ($Capture.WaitForExit(2000)) {
            $text = "PresentMon exited $($Capture.ExitCode) by itself"
        } else {
            $null = & $PresentMon --session_name $Session --terminate_existing_session 2>&1
            if ($Capture.WaitForExit(60000)) {
                $text = "PresentMon stopped, exit $($Capture.ExitCode)"
            } else {
                $null = & logman.exe stop $Session -ets 2>&1
                if ($Capture.WaitForExit(15000)) { $text = "PresentMon stopped by logman, exit $($Capture.ExitCode)" }
                else { Stop-Tree $Capture.Id; $text = 'PresentMon killed' }
            }
        }
    } catch {
        $text = 'PresentMon stop error: ' + ($_.Exception.Message -replace '\s+', ' ')
    }
    Stop-Session $Session
    $text
}

# The reason in foreground.py's last line ({"t": "end", "reason": ...}), else the meaning of its exit code.
function Get-WatchReason([string]$Label, [int]$Code) {
    $reason = $null
    try {
        $last = Get-Content -LiteralPath (Join-Path $Root "$Out\$Label.out.txt") -ErrorAction Stop |
            Where-Object { $_.Trim() } | Select-Object -Last 1
        $reason = ($last | ConvertFrom-Json).reason
    } catch { }
    if (-not $reason) { $reason = $WatchAbort[$Code] }
    if (-not $reason) { $reason = "watcher exit $Code" }
    $reason
}

# One shell run: foreground.py, PresentMon (optional), the poster (optional), launch.py last; the close and abort
# rules; then the poster's end, PresentMon's stop, and the capture's copy into the run folder, or on an abort both set
# aside as <name>.aborted-<n>. Returns 'done', 'aborted' or 'kept' (aborted, but the folder could not be set aside).
function Invoke-ShellRun {
    param([string]$Step, [string]$Name, [int]$Attempt, [string[]]$LaunchArgs = @(), [int]$CaptureSeconds = 0,
          [string[]]$Poster = @(), [int]$CloseAfter = 0, [int]$PosterGrace = 0, [int]$Cap = 600, [switch]$Hold)
    $run = "$Runs\$Name"
    $csv = "$Runs\$Name.presentmon.csv"
    $session = "magic600-c9-$Name"
    $label = if ($Attempt) { "$Name.retry$Attempt" } else { $Name }
    $captureArgs = @('--v1_metrics', '--qpc_time', '--process_name', $GodotName, '--output_file', $csv,
        '--session_name', $session, '--timed', "$CaptureSeconds", '--terminate_after_timed', '--no_console_stats')
    $launchCommand = @('-B', '-u', 'renderer/shell/launch.py', '--build', $ProductBuild, '--out', $run) + $LaunchArgs
    $watchArgs = @('-B', '-u', $Watcher, '--window-of', $GodotName)
    if ($Hold) { $watchArgs += '--hold' }
    $posterArgs = @($Poster | ForEach-Object { $_.Replace('{run}', $run) })
    if ($CloseAfter) { $rule = "CloseMainWindow at $CloseAfter s" }
    elseif ($Poster.Count -and $PosterGrace) { $rule = "the poster closes the shell; CloseMainWindow $PosterGrace s after the poster exits if it is still open" }
    elseif ($Poster.Count) { $rule = "CloseMainWindow as soon as the poster exits (after its 'samples posted' line)" }
    else { $rule = 'the shell closes itself' }
    $rule += "; CloseMainWindow at $Cap s at the latest, kill 120 s after a close that did not end it"
    $abortRule = 'abort on owner input' + $(if ($Hold) { ' or a lost foreground' } else { '' }) +
        $(if ($Poster.Count) { ' before the poster exits' } else { ' before the close' })

    if ($DryRun) {
        $dryFolder = "$Out\$Name"
        Wait-Gate $Step
        Write-Host "  [$Step] watcher first, no window: python $($watchArgs -join ' ')"
        if ($CaptureSeconds) { Write-Host "  [$Step] PresentMon, no window: `"$PresentMon`" $($captureArgs -join ' ')" }
        if ($Poster.Count) {
            Write-Host "  [$Step] poster, no window: python $((@('-B', '-u') + $posterArgs) -join ' ')"
            Invoke-Dry "$Name-poster" $Python (@('-B') + @($Poster | ForEach-Object { $_.Replace('{run}', $dryFolder) }) + @('--dry-run'))
        }
        Write-Host "  [$Step] launch, last: python $($launchCommand -join ' ')"
        Invoke-Dry "$Name-launch" $Python (@('-B', 'renderer/shell/launch.py', '--build', $ProductBuild, '--out', $dryFolder) + $LaunchArgs + @('--dry-run'))
        Write-Host "  [$Step] close: $rule"
        Write-Host "  [$Step] $abortRule; then set aside as $Name.aborted-<n> and run again, at most $Retries times"
        if ($CaptureSeconds) { Write-Host "  [$Step] then stop PresentMon (--terminate_existing_session) and copy $csv to $run\presentmon.csv" }
        return 'done'
    }

    Wait-Gate $Step
    Write-Host "$(Get-Utc) $Step`: $run, attempt $($Attempt + 1) ($rule; $abortRule)"
    $notes = New-Object Collections.Generic.List[string]
    $capture = $null; $posterProcess = $null; $launch = $null; $watch = $null; $code = 'error'; $since = Get-Date
    $abort = $null; $watchSeen = $false
    try {
        # The watcher's baseline is the last input before this run: any later input is the owner's.
        $watch = Start-Child $Python $watchArgs "$label-watch"
        if ($CaptureSeconds) {
            Stop-Session $session
            $capture = Start-Child $PresentMon $captureArgs "$label-presentmon"
            $script:Sessions += $session
            Start-Sleep -Seconds 3
            if ($capture.HasExited) { $notes.Add("PresentMon exited $($capture.ExitCode) at its start") }
        }
        if ($Poster.Count) { $posterProcess = Start-Child $Python (@('-B', '-u') + $posterArgs) "$label-poster" }
        $since = Get-Date
        $launch = Start-Child $Python $launchCommand "$label-launch"
        $closing = $null; $posted = $false; $posterSeen = $null
        while (-not $launch.WaitForExit(2000)) {
            $now = Get-Date
            $elapsed = ($now - $since).TotalSeconds
            if ($null -eq $closing -and -not $watchSeen -and $watch.HasExited) {
                $watchSeen = $true
                $watchCode = $watch.ExitCode
                $reason = Get-WatchReason "$label-watch" $watchCode
                $measuring = -not ($posterProcess -and $posterProcess.HasExited)
                if ($WatchAbort.ContainsKey($watchCode) -and $measuring) {
                    $abort = $reason
                    $closing = $now
                    $notes.Add("aborted at $([int]$elapsed) s: $reason")
                    if ($posterProcess -and -not $posterProcess.HasExited) { Stop-Tree $posterProcess.Id }
                } else {
                    $notes.Add("watcher: $reason")
                }
            }
            if ($null -eq $closing) {
                $why = $null
                if ($CloseAfter -and $elapsed -ge $CloseAfter) { $why = "closed at $([int]$elapsed) s" }
                elseif ($posterProcess -and $posterProcess.HasExited) {
                    if ($null -eq $posterSeen) { $posterSeen = $now }
                    if (($now - $posterSeen).TotalSeconds -ge $PosterGrace) { $why = "closed after the poster exited $($posterProcess.ExitCode)" }
                }
                if (-not $why -and $elapsed -ge $Cap) { $why = "closed at the $Cap s limit" }
                if ($why) {
                    $closing = $now; $notes.Add($why)
                    # The run's own close: a foreground change from here on is not an abort.
                    if (-not $watch.HasExited) { Stop-Tree $watch.Id }
                }
            }
            if ($null -ne $closing) {
                if (-not $posted) { $posted = Close-Shell $since }
                if (($now - $closing).TotalSeconds -ge 120) {
                    $notes.Add('shell killed: still running 120 s after the close')
                    foreach ($shell in Get-Shell $since) { Stop-Tree $shell.Id }
                    if (-not $launch.WaitForExit(30000)) { Stop-Tree $launch.Id; $null = $launch.WaitForExit(10000) }
                    break
                }
            }
        }
        $code = if ($launch.HasExited) { $launch.ExitCode } else { 'hung' }
        $notes.Add("shell time $([int]((Get-Date) - $since).TotalSeconds) s")
        if ($posterProcess) {
            if (-not $posterProcess.WaitForExit(120000)) {
                Stop-Tree $posterProcess.Id; $null = $posterProcess.WaitForExit(10000); $notes.Add('poster stopped 120 s after the shell')
            }
            $notes.Add("poster exit $($posterProcess.ExitCode)")
        }
    } catch {
        $notes.Add('error: ' + ($_.Exception.Message -replace '\s+', ' '))
        if ($launch -and -not $launch.HasExited) { foreach ($shell in Get-Shell $since) { Stop-Tree $shell.Id }; Stop-Tree $launch.Id }
        if ($posterProcess -and -not $posterProcess.HasExited) { Stop-Tree $posterProcess.Id }
    } finally {
        if ($watch -and -not $watch.HasExited) { Stop-Tree $watch.Id }
        if ($capture) { $notes.Add((Stop-Capture $capture $session)) }
    }
    if ($abort) {
        $n = 1
        while ((Test-Path -LiteralPath (Join-Path $Root "$run.aborted-$n")) -or
               (Test-Path -LiteralPath (Join-Path $Root "$Runs\$Name.aborted-$n.presentmon.csv"))) { $n++ }
        $result = 'aborted'
        try {
            if (Test-Path -LiteralPath (Join-Path $Root $run)) {
                Move-Item -LiteralPath (Join-Path $Root $run) -Destination (Join-Path $Root "$run.aborted-$n") -ErrorAction Stop
            }
            if (Test-Path -LiteralPath (Join-Path $Root $csv)) {
                Move-Item -LiteralPath (Join-Path $Root $csv) -Destination (Join-Path $Root "$Runs\$Name.aborted-$n.presentmon.csv") -ErrorAction Stop
            }
            $notes.Add("set aside as $Name.aborted-$n")
        } catch {
            $notes.Add('not set aside: ' + ($_.Exception.Message -replace '\s+', ' '))
            $result = 'kept'
        }
        Write-Log $Step 'aborted' $run ($notes -join '; ')
        return $result
    }
    if ($capture) {
        $from = Join-Path $Root $csv
        $folder = Join-Path $Root $run
        if ((Test-Path -LiteralPath $from -PathType Leaf) -and (Test-Path -LiteralPath $folder -PathType Container)) {
            try { Copy-Item -LiteralPath $from -Destination (Join-Path $folder 'presentmon.csv'); $notes.Add('capture copied') }
            catch { $notes.Add('capture copy failed: ' + ($_.Exception.Message -replace '\s+', ' ')) }
        } else { $notes.Add('no capture copied') }
    }
    Write-Log $Step $code $run ($notes -join '; ')
    'done'
}

# A shell run with at most $Retries reruns after an abort.
function Invoke-ShellStep {
    param([string]$Step, [string]$Name, [string[]]$LaunchArgs = @(), [int]$CaptureSeconds = 0,
          [string[]]$Poster = @(), [int]$CloseAfter = 0, [int]$PosterGrace = 0, [int]$Cap = 600, [switch]$Hold)
    for ($attempt = 0; $attempt -le $Retries; $attempt++) {
        $result = Invoke-ShellRun -Step $Step -Name $Name -Attempt $attempt -LaunchArgs $LaunchArgs `
            -CaptureSeconds $CaptureSeconds -Poster $Poster -CloseAfter $CloseAfter -PosterGrace $PosterGrace `
            -Cap $Cap -Hold:$Hold
        if ($result -eq 'done') { return }
        if ($result -eq 'kept') { Write-Log $Step 'gave-up' "$Runs\$Name" 'the aborted run could not be set aside'; return }
        if ($DryRun) { return }
    }
    Write-Log $Step 'gave-up' "$Runs\$Name" "aborted $($Retries + 1) times"
}

function Get-Stamps {
    @(Get-ChildItem -LiteralPath (Join-Path $Root $Acceptance) -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^\d{8}T\d{9}Z$' } | ForEach-Object { $_.Name } | Sort-Object)
}

# run_scene.ps1's quiet gate refuses builds, Godot, PresentMon; wait up to $Limit s for them to end, then go on.
function Wait-Quiet([string]$Step, [int]$Limit = 900) {
    $deadline = (Get-Date).AddSeconds($Limit)
    while ($true) {
        $busy = @(Get-CimInstance -ClassName Win32_Process | Where-Object {
            $_.ProcessId -ne $PID -and ([string]$_.Name -match '^(cl|link|ninja|cmake|msbuild|dotnet|VBCSCompiler|Godot.*|blender|ffmpeg|PresentMon|PresentMonUI|PresentMon-.+)\.exe$') })
        if (-not $busy.Count) { return }
        if ((Get-Date) -ge $deadline) {
            Write-Log "$Step/quiet" 'busy' $null ("still running after $Limit s: " + (($busy | ForEach-Object { "$($_.Name) $($_.ProcessId)" }) -join ', '))
            return
        }
        Start-Sleep -Seconds 15
    }
}

# The quiet-machine gate runbook.py uses (run_probe.py --check-only: owner idle 300 s, no build, capture, Godot or busy
# Codex process, CPU at most 15 %, mains power, an unlocked desktop). Checked every 15 s for up to $Limit s before each
# measured run; if it is still not quiet then, the wait is logged and the run goes ahead.
function Wait-Gate([string]$Step, [int]$Limit = 1800) {
    $command = 'python -B work/experiments/c9-present-probe/run_probe.py --check-only'
    if ($DryRun) { Write-Host "  [$Step] first the quiet-machine gate: $command, every 15 s for up to $Limit s"; return }
    $deadline = (Get-Date).AddSeconds($Limit)
    $waited = $false
    while ($true) {
        $global:LASTEXITCODE = $null
        $text = (& $Python -B 'work/experiments/c9-present-probe/run_probe.py' --check-only 2>&1 | ForEach-Object { "$_" }) -join "`n"
        $code = $LASTEXITCODE
        if ($code -eq 0) {
            if ($waited) { Write-Host "$(Get-Utc) $Step/gate: quiet" }
            return
        }
        if ($code -ne 5) {
            Write-Log "$Step/gate" $code $null 'the gate check failed; the run goes ahead without it'
            return
        }
        $reasons = try { @(($text | ConvertFrom-Json).reasons) -join '; ' } catch { 'not quiet' }
        if (-not $waited) { Write-Host "$(Get-Utc) $Step/gate: waiting up to $Limit s: $reasons"; $waited = $true }
        if ((Get-Date) -ge $deadline) {
            Write-Log "$Step/gate" 'not-quiet' $null "still not quiet after $Limit s, the run goes ahead: $reasons"
            return
        }
        Start-Sleep -Seconds 15
    }
}

# One run_scene.ps1 run in this process; returns its new run folder (or nothing).
function Invoke-Scene([string]$Step, [string]$Scene, [string]$Build) {
    $sceneScript = Join-Path $Root 'renderer\acceptance\run_scene.ps1'
    $params = @{ Scene = $Scene; Build = $Build; Overlays = 'none running'; Declare = $Declare; Unattended = $true }
    $shown = "& renderer/acceptance/run_scene.ps1 -Scene $Scene -Build $Build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended"
    if ($DryRun) {
        Wait-Gate $Step
        Write-Host "  [$Step] $shown"
        $params.DryRun = $true
        $global:LASTEXITCODE = 0
        try { $text = (& $sceneScript @params *>&1 | ForEach-Object { "$_" }) -join "`n"; $code = $LASTEXITCODE }
        catch { $text = 'error: ' + $_.Exception.Message; $code = 'error' }
        [IO.File]::WriteAllText((Join-Path $Root "$Out\$(Get-Label $Step).txt"), $text + "`n", $Utf8)
        $idle = $text -split "`n" | Where-Object { $_ -match 'idle|administrator|build verified' } | Select-Object -First 2
        Write-Host "      dry run: exit $code | $($idle -join ' | ')"
        return
    }
    Wait-Gate $Step
    Wait-Quiet $Step
    Write-Host "$(Get-Utc) $Step`: $shown"
    $before = Get-Stamps
    $global:LASTEXITCODE = $null
    $note = $null
    try { & $sceneScript @params | Out-Host; $code = $LASTEXITCODE }
    catch { $code = 'error'; $note = $_.Exception.Message -replace '\s+', ' ' }
    $new = @(Get-Stamps | Where-Object { $before -notcontains $_ })
    $run = if ($new.Count) { "$Acceptance\$($new[-1])" } else { $null }
    Write-Log $Step $code $run $note
    if ($run) { $run }
}

function Invoke-Gate([string]$Step, [string[]]$Made, [string]$Summary) {
    if ($DryRun) { Write-Host "  [$Step] python -B -u tools/perf/renderer_gate.py <run 1> <run 2> <run 3> --out $Summary"; return }
    if (-not $Made.Count) { Write-Log $Step 'skipped' $null 'no run folder'; return }
    $null = Invoke-Step $Step $Python (@('-B', '-u', 'tools/perf/renderer_gate.py') + $Made + @('--out', $Summary)) 600 $Summary
}

function Invoke-U1 {
    Wait-Gate 'U1'
    $null = Invoke-Step 'U1' $Python @('-B', '-u', 'renderer/core/check_core.py', '--gpu') 2700 $null
}

function Invoke-U2 {
    foreach ($i in 1..3) {
        Wait-Gate "U2.$i"
        $null = Invoke-Step "U2.$i" $Python @('-B', '-u', 'work/experiments/c9-present-probe/run_probe.py', '--topmost', '--wait-quiet', '1800') 3600 $null
    }
}

function Invoke-U3 {
    Invoke-ShellStep -Step 'U3' -Name "u3$Tag" -CaptureSeconds 600 -CloseAfter 540 -Cap 590 -Hold
    $null = Invoke-Step 'U3/ui_skew' $Python @('-B', '-u', 'tools/perf/c9/ui_skew.py', "$Runs\u3$Tag", '--out', "$Runs\u3$Tag-skew.json") 600 "$Runs\u3$Tag-skew.json"
}

function Invoke-U4 {
    $count = if ($DryRun) { 1 } else { 3 }
    foreach ($scene in 'S4', 'I_a', 'I_b', 'w3') {
        $made = @()
        foreach ($i in 1..$count) { $run = Invoke-Scene "U4/$scene.$i" $scene $SceneBuild; if ($run) { $made += $run } }
        Invoke-Gate "U4/$scene/gate" $made "$Acceptance\$scene-summary$Tag.json"
    }
}

function Invoke-U5 {
    $count = if ($DryRun) { 1 } else { 3 }
    $made = @()
    foreach ($i in 1..$count) { $run = Invoke-Scene "U5/w3-stages.$i" 'w3' $StageBuild; if ($run) { $made += $run } }
    Invoke-Gate 'U5/gate' $made "$Acceptance\w3-stages-summary$Tag.json"
}

function Invoke-U6 {
    $made = @()
    foreach ($i in 1..3) {
        Invoke-ShellStep -Step "U6.$i" -Name "e3$Tag-$i" -LaunchArgs @('--cold-start') -Cap 300
        $made += "$Runs\e3$Tag-$i"
    }
    $null = Invoke-Step 'U6/cold_start' $Python (@('-B', '-u', 'tools/perf/c9/cold_start.py') + $made + @('--out', "$Runs\e3$Tag.json")) 600 "$Runs\e3$Tag.json"
}

function Invoke-U7 {
    Wait-Gate 'U7/faults'
    $null = Invoke-Step 'U7/faults' $Cargo @('run', '--locked', '--offline', '--release', '--manifest-path', 'rust/Cargo.toml',
        '-p', 'c600-lx2', '--features', 'device', '--bin', 'c600-lx2-faults', '--', '--runtime', "$ProductBuild\runtime\c600-runtime.exe",
        '--injection', $Child, '--sessions', "$Runs\u7$Tag-sessions", '--out', "$Runs\u7$Tag-faults.json") 2700 "$Runs\u7$Tag-faults.json"
    Invoke-ShellStep -Step 'U7' -Name "u7$Tag" -LaunchArgs @('--lx4', '--lx4-fixtures', $Fixtures, '--lx2-injection', $Child) `
        -CaptureSeconds 4800 -Poster @('tools/perf/c9/lx2.py', 'post', '--fixtures', $Fixtures, '--run', '{run}') -PosterGrace 60 -Cap 4500 -Hold
    $null = Invoke-Step 'U7/analyze' $Python @('-B', '-u', 'tools/perf/c9/lx2.py', 'analyze', '--fixtures', $Fixtures, "$Runs\u7$Tag",
        '--faults', "$Runs\u7$Tag-faults.json", '--out', "$Runs\u7$Tag-lx2.json") 900 "$Runs\u7$Tag-lx2.json"
}

function Invoke-U8 {
    foreach ($puzzle in 'jumble-off', 'jumble-on') {
        $name = if ($puzzle -eq 'jumble-off') { "u8-off$Tag" } else { "u8-on$Tag" }
        Invoke-ShellStep -Step "U8/$puzzle" -Name $name -LaunchArgs @('--lx4', '--lx4-fixtures', $Fixtures, '--puzzle', $puzzle) `
            -CaptureSeconds 10800 -Poster @('tools/perf/c9/lx4.py', 'post', '--fixtures', $Fixtures, '--puzzle', $puzzle, '--run', '{run}', '--turn-key', 'KeyJ') `
            -PosterGrace 0 -Cap 10500 -Hold
    }
    $null = Invoke-Step 'U8/key_to_frame' $Python @('-B', '-u', 'tools/perf/c9/key_to_frame.py', "$Runs\u8-off$Tag", "$Runs\u8-on$Tag", '--out', "$Runs\u8$Tag-e2.json") 900 "$Runs\u8$Tag-e2.json"
    # renderer_gate.py measures acceptance builds only, so U4's W3 summary is recorded as a reference, never as the gate
    # (lx4.py --gate-reference; the verdict stays insufficient until the owner rules on it).
    $analyze = @('-B', '-u', 'tools/perf/c9/lx4.py', 'analyze', '--fixtures', $Fixtures, "$Runs\u8-off$Tag", "$Runs\u8-on$Tag",
        '--e2', "$Runs\u8$Tag-e2.json", '--out', "$Runs\u8$Tag-lx4.json")
    # This run's U4 summary, else the one without a suffix (a rerun of U8 alone).
    $reference = @("$Acceptance\w3-summary$Tag.json", "$Acceptance\w3-summary.json") |
        Where-Object { Test-Path -LiteralPath (Join-Path $Root $_) -PathType Leaf } | Select-Object -First 1
    if ($DryRun -and -not $reference) { $reference = "$Acceptance\w3-summary$Tag.json" }
    if ($reference) { $analyze += @('--gate-reference', $reference) }
    else { Write-Log 'U8/analyze' 'note' $null "no $Acceptance\w3-summary$Tag.json or w3-summary.json; analyzed without a gate reference" }
    $null = Invoke-Step 'U8/analyze' $Python $analyze 900 "$Runs\u8$Tag-lx4.json"
}

function Test-Inputs {
    $problems = @()
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { $problems += 'this PowerShell is not an administrator one (PresentMon and run_scene.ps1 need it)' }
    if (-not $Python) { $problems += 'python is not on PATH' }
    if (-not $Cargo -and $Steps -contains 'U7') { $problems += 'cargo is not on PATH (U7 fault trials)' }
    if (-not (Test-Path -LiteralPath $PresentMon -PathType Leaf)) { $problems += 'PresentMon 2.6.0 is not at its pinned path' }
    if (-not (Test-Path -LiteralPath (Join-Path $Root $Watcher) -PathType Leaf)) { $problems += "missing $Watcher" }
    $builds = @{ SceneBuild = @('U4'); StageBuild = @('U5'); ProductBuild = @('U3', 'U6', 'U7', 'U8') }
    $given = @{ SceneBuild = $SceneBuild; StageBuild = $StageBuild; ProductBuild = $ProductBuild }
    foreach ($key in $builds.Keys) {
        $needs = @($builds[$key] | Where-Object { $Steps -contains $_ })
        if (-not $needs.Count) { continue }
        if (-not $given[$key]) { $problems += "-$key is needed for $($needs -join ', ')" }
        elseif (-not (Test-Path -LiteralPath (Join-Path $Root $given[$key]) -PathType Container)) { $problems += "missing folder $($given[$key]) (-$key)" }
    }
    if ($Steps | Where-Object { $_ -in 'U7', 'U8' }) {
        if (-not (Test-Path -LiteralPath (Join-Path $Root $Fixtures) -PathType Container)) { $problems += "missing folder $Fixtures" }
    }
    if ($Steps -contains 'U4' -and -not (Test-Path -LiteralPath (Join-Path $Root "$Acceptance\wj-data") -PathType Container)) { $problems += "missing folder $Acceptance\wj-data" }
    if ($Steps -contains 'U7' -and -not (Test-Path -LiteralPath (Join-Path $Root $Child) -PathType Leaf)) { $problems += "missing $Child" }
    $outputs = @{
        U3 = @("$Runs\u3$Tag", "$Runs\u3$Tag.presentmon.csv", "$Runs\u3$Tag-skew.json")
        U4 = @("$Acceptance\S4-summary$Tag.json", "$Acceptance\I_a-summary$Tag.json", "$Acceptance\I_b-summary$Tag.json", "$Acceptance\w3-summary$Tag.json")
        U5 = @("$Acceptance\w3-stages-summary$Tag.json")
        U6 = @("$Runs\e3$Tag-1", "$Runs\e3$Tag-2", "$Runs\e3$Tag-3", "$Runs\e3$Tag.json")
        U7 = @("$Runs\u7$Tag", "$Runs\u7$Tag-sessions", "$Runs\u7$Tag-faults.json", "$Runs\u7$Tag.presentmon.csv", "$Runs\u7$Tag-lx2.json")
        U8 = @("$Runs\u8-off$Tag", "$Runs\u8-on$Tag", "$Runs\u8-off$Tag.presentmon.csv", "$Runs\u8-on$Tag.presentmon.csv", "$Runs\u8$Tag-e2.json", "$Runs\u8$Tag-lx4.json")
    }
    foreach ($step in $Steps) {
        if (-not $outputs.ContainsKey($step)) { continue }
        foreach ($path in $outputs[$step]) {
            if (Test-Path -LiteralPath (Join-Path $Root $path)) { $problems += "$path exists already ($step writes it; choose another -Suffix)" }
        }
    }
    $problems
}

Push-Location -LiteralPath $Root
$awake = $false
$transcript = $false
try {
    $Python = (Get-Command python -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    $Cargo = (Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source
    $null = New-Item -ItemType Directory -Force -Path (Join-Path $Root $Out)
    $head = (& git -C $Root rev-parse --short=12 HEAD 2>$null)
    $order = @('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'U7', 'U8') | Where-Object { $Steps -contains $_ }
    Write-Host "unattended: $(if ($DryRun) { 'DRY RUN, nothing starts' } else { 'leave the machine alone until it prints unattended: done' })"
    Write-Host "  folder $Root, head $head, steps $($order -join ' '), suffix '$Suffix', retries $Retries, log $Log, child output $Out"
    $problems = @(Test-Inputs)
    foreach ($problem in $problems) { Write-Host "  check: $problem" }
    if ($problems.Count -and -not $DryRun) {
        Write-Log 'start' 'refused' $null ($problems -join '; ')
        Write-Host 'unattended: nothing was started.'
        return
    }
    Write-Log 'start' 0 $null "head $head; steps $($order -join ','); suffix '$Suffix'; builds scene '$SceneBuild' stage '$StageBuild' product '$ProductBuild'$(if ($DryRun) { '; dry run' })"
    if (-not $DryRun) {
        if (-not ('C9Unattended.Power' -as [type])) {
            Add-Type -Namespace C9Unattended -Name Power -MemberDefinition '[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);'
        }
        # ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED for this script's run: no sleep, display stays on.
        $null = [C9Unattended.Power]::SetThreadExecutionState([uint32]2147483651)
        $awake = $true
        try { $null = Start-Transcript -LiteralPath (Join-Path $Root "$Out\transcript.txt"); $transcript = $true } catch { }
    }
    foreach ($step in $order) {
        Write-Host "--- $step $(Get-Utc)"
        try { & "Invoke-$step" }
        catch { Write-Log $step 'error' $null ('error: ' + ($_.Exception.Message -replace '\s+', ' ')) }
    }
    Write-Log 'end' 0 $null "steps $($order -join ',')$(if ($DryRun) { '; dry run' })"
    Write-Host 'unattended: done'
} finally {
    foreach ($session in $script:Sessions) { Stop-Session $session }
    if ($awake) { $null = [C9Unattended.Power]::SetThreadExecutionState([uint32]2147483648) }
    if ($transcript) { try { $null = Stop-Transcript } catch { } }
    Pop-Location
}
