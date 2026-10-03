# Readiness capture (packet E-2.4-00, acceptance item 3). Run from an ADMINISTRATOR PowerShell:
#   powershell -ExecutionPolicy Bypass -File work\experiments\renderer-readiness\capture.ps1
# It starts the minimal D3D12 program, captures 20 s with PresentMon 2.6.0.0 on that process only,
# fills the swap chain into run.json from the capture, and runs tools/perf/renderer_gate.py.
# The capture stays private under work/loop-memory/perf/renderer/readiness/.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$exe = Join-Path $PSScriptRoot 'build\minimal_d3d12.exe'
$presentmon = "$env:ProgramFiles\Intel\PresentMon\PresentMonConsoleApplication\PresentMon-2.6.0-x64.exe"
$run = Join-Path $root ("work\loop-memory\perf\renderer\readiness\" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
foreach ($path in $exe, $presentmon) { if (-not (Test-Path $path)) { throw "missing: $path" } }
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw 'PresentMon needs an administrator terminal (without it, it exits with code 6 and writes no CSV).' }
New-Item -ItemType Directory -Force -Path $run | Out-Null

Write-Host "Starting the readiness program for 26 s (Esc closes it early) ..."
$program = Start-Process -FilePath $exe -ArgumentList '26', "`"$run`"" -PassThru
Start-Sleep -Seconds 2
& $presentmon --v1_metrics --qpc_time --process_id $program.Id --output_file (Join-Path $run 'presentmon.csv') --timed 20 --terminate_after_timed | Out-Null
Write-Host "PresentMon exit code: $LASTEXITCODE"
$program.WaitForExit()

$csv = Import-Csv (Join-Path $run 'presentmon.csv') | Where-Object { [int]$_.ProcessID -eq $program.Id }
$chain = ($csv | Group-Object SwapChainAddress | Sort-Object Count -Descending | Select-Object -First 1).Name
if (-not $chain) { throw 'no PresentMon rows for the program' }
$runJson = Join-Path $run 'run.json'
[IO.File]::WriteAllText($runJson, [IO.File]::ReadAllText($runJson).Replace('FILL-FROM-CSV', $chain))
Write-Host "rows for the program: $($csv.Count); swap chain $chain"

$python = Join-Path $root 'tools\.venv\engine\Scripts\python.exe'
if (-not (Test-Path $python)) { $python = 'python' }
& $python (Join-Path $root 'tools\perf\renderer_gate.py') $run --out (Join-Path $run 'summary.json')
Write-Host "renderer_gate.py exit code: $LASTEXITCODE"
Get-Content (Join-Path $run 'summary.json')
