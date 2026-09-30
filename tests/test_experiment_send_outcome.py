"""Headless C# receipt regression and experiment Send source contract."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile
import time
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'work/experiments/magic600-04/native'


def check_source():
    source = (NATIVE / 'ExperimentShell.cs').read_text(encoding='utf-8-sig')
    send = source.split('Task<bool> SendRoute(', 1)[1].split('void NativeInput(', 1)[0]
    assert 'ExperimentSendOutcome.Worker' in send
    finalization = re.search(r'var\s+(\w+)\s*=\s*ExperimentSendOutcome\.Finalize\(', send)
    assert finalization, 'Send must use the shared finalization decision'
    name = finalization.group(1)
    compact = re.sub(r'\s+', '', send)
    assert 'completion.TrySetResult(' + name + '.Accepted)' in compact
    assert 'lastCommandResult=' + name + '.LastResult;' in compact
    assert 'if(' + name + '.LogRejection)NativeDiagnostics.Write("Experimentaloperationrejected",fault);' in compact
    assert 'if(' + name + '.Accepted&&requestedAction=="session-new")pendingDefaultViews=true;' in compact
    assert re.search(r'if\(' + name + r'\.Accepted&&[^\n]+RequestSessionCompletion\(\);', send)
    assert 'accepted=importReceipt!=null||' not in compact
    print('PASS: Send uses the shared worker, receipt and acceptance decision', flush=True)


def remove_build(directory):
    for attempt in range(5):
        try:
            shutil.rmtree(directory)
            return
        except FileNotFoundError:
            return
        except OSError:
            if attempt == 4:
                raise
            time.sleep(0.1 * (attempt + 1))


def main():
    check_source()
    compiler = Path(os.environ.get('WINDIR', r'C:\Windows')) / 'Microsoft.NET/Framework/v4.0.30319/csc.exe'
    if os.name != 'nt' or not compiler.is_file():
        print('NOT RUN: the C# harness requires Windows and the .NET Framework csc.exe compiler')
        return 0
    # Python's Windows mode 0o700 drops the inherited sandbox access entries.
    mkdir = os.mkdir
    with patch('os.mkdir', side_effect=lambda path, mode: mkdir(path)):
        try:
            directory = Path(tempfile.mkdtemp(prefix='experiment-send-outcome-')).resolve()
        except OSError:
            directory = Path(tempfile.mkdtemp(prefix='experiment-send-outcome-', dir=ROOT / 'work')).resolve()
    try:
        executable = directory / 'ExperimentSendOutcomeRegression.exe'
        command = [str(compiler), '/nologo', '/target:exe', '/platform:x86', '/langversion:5',
                   '/utf8output', '/codepage:65001', '/main:ExperimentSendOutcomeRegression',
                   '/out:' + str(executable)]
        command += ['/reference:' + name for name in ('System.dll', 'System.Core.dll', 'System.Web.Extensions.dll')]
        command += [str(NATIVE / 'ExperimentSendOutcome.cs'), str(ROOT / 'tests/native/ExperimentSendOutcomeRegression.cs')]
        environment = dict(os.environ, TEMP=str(directory), TMP=str(directory))
        subprocess.run(command, check=True, timeout=60, env=environment)
        subprocess.run([str(executable)], check=True, timeout=30)
    finally:
        remove_build(directory)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
