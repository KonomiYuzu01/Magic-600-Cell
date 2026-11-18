"""Runbook for the C9 owner-machine batch (LP3, owner 16 and 18 November 2026). Standard library only.

    python -B tools/perf/c9/runbook.py --list                       the steps with commands and prerequisites
    python -B tools/perf/c9/runbook.py --dry-run [--build DIR] [--check-quiet]

The batch runs on one day on the owner's machine (owner, 16 November 2026), attended steps first: route B's input
check B1 and charter E2 need the owner and the foreground. The unattended steps follow, each started only when the
quiet-machine gate passes: W-J item 5 and W3 on the product build (ADR-R01.3, route B), RPI.79's gate run, three
cold E3 runs, LX-2, LX-4 (4 h time box, owner 18 November 2026), and route B's three cold probe runs (B3) and its
UI/3D skew measurement (B2). "Unattended" means the owner need not watch; the W-J, W3 and probe windows still need an
unlocked desktop and the foreground.

The runbook runs nothing. `--dry-run` checks, without a GPU, window or capture, every prerequisite it can (files and
tools present, the PresentMon version, the build identity readable, scripts that compile or import, fixtures present)
and prints one line per step. It exits 1 when a prerequisite of a step that is built is missing; a step whose
component is not built yet prints its missing prerequisites as a warning. Time estimates are DRAFTS from the runners'
own timings (run_scene.ps1: 20 s pause, 4 s preroll, 192 s trace per cold run; the probe: about 31 s per harness).

The quiet-machine gate is the probe's (`work/experiments/c9-present-probe/run_probe.py --check-only`: owner idle at
least 300 s, no build, capture, Godot or busy Codex process, CPU at most 15 %, mains power, an unlocked desktop; exit
0 quiet, 5 not quiet). `quiet_machine()` runs it; `--check-quiet` adds its answer to a dry run as information. The
tests never run it.
"""
from __future__ import annotations

import argparse
import importlib
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))
import presentmon_capture  # noqa: E402

PROBE = 'work/experiments/c9-present-probe/run_probe.py'
BUILDS = 'work/build/c9-acceptance'
IDENTITY = 'c9-build-identity.json'
IDENTITY_FORMAT = 'magic600-c9-acceptance-build-v1'
# The probe's own Godot location (run_probe.py, locate_godot), under %LOCALAPPDATA%/Microsoft/WinGet/Packages.
GODOT_GLOB = ('GodotEngine.GodotEngine.Mono_*/Godot_v4.7.2-stable_mono_win64/'
              'Godot_v4.7.2-stable_mono_win64_console.exe')
RUNNER = 'renderer/acceptance/run_scene.ps1'
DECLARE = "-Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended"


@dataclass(frozen=True)
class Step:
    id: str
    title: str
    attended: bool
    minutes: int
    missing: str | None          # the component that is not built yet, or None when the step can run
    commands: tuple = ()
    prerequisites: tuple = ()    # (kind, argument) pairs checked by evaluate()
    run_time: tuple = ()         # conditions only the run itself can check
    sources: tuple = field(default=())

    @property
    def built(self):
        return self.missing is None


def wj_w3_commands():
    runs = [f"& {RUNNER} -Scene {scene} -Build <product build> {DECLARE}   # three times"
            for scene in ('S4', 'I_a', 'I_b', 'w3')]
    gate = ('python -B tools/perf/renderer_gate.py <run 1> <run 2> <run 3> '
            '--out work/loop-memory/c9-acceptance/<scene>-summary.json   # per scene')
    return (*runs, gate)


STEPS = (
    Step('b1-input', 'Route B check B1: input through Godot\'s transparent window over the core\'s layer', True, 10,
         'the product shell (Godot transparent window over the core\'s DirectComposition layer) and its input check',
         run_time=('foreground', 'the owner at the machine'),
         sources=('owner decision 2026-11-18 route B, check 1',)),
    Step('e2-key-to-frame', 'Charter E2: key to the confirmed first correct frame, at least 100 presses', True, 20,
         'the product shell\'s E2 event log (magic600-c9-shell-events-v1) and sa2_presented with its display QPC',
         commands=('<product shell E2 run>, one run directory per run',
                   'python -B tools/perf/c9/key_to_frame.py <run dir> [<run dir> ...] --out <record>.json'),
         prerequisites=(('imports', 'key_to_frame'), ('presentmon', None)),
         run_time=('foreground', 'the owner at the machine', 'a fresh copy of the session fixture'),
         sources=('charter E2', 'RTE.43', 'V11.1', 'PHS.64')),
    Step('wj-w3', 'W-J item 5 (three cold runs each of S4, I_a, I_b) and W3 (three cold runs) on the product build',
         False, 50, 'the route-B product build (the W-J and W3 evidence must come from it, not the texture path)',
         commands=wj_w3_commands(),
         prerequisites=(('file', RUNNER), ('compiles', 'renderer/acceptance/finalize_run.py'),
                        ('compiles', 'renderer/acceptance/prepare_host.py'),
                        ('compiles', 'renderer/acceptance/file_guard.py'), ('imports', 'renderer_gate'),
                        ('command', 'powershell'), ('presentmon', None), ('build', None), ('godot-launch', None),
                        ('dir', 'work/loop-memory/c9-acceptance/wj-data')),
         run_time=('administrator PowerShell started on the desktop', 'foreground', 'mains power',
                   'native resolution, no frame generation or upscaling'),
         sources=('ADR-R01.3', 'owner decision 2026-11-18 route B')),
    Step('rpi79-gate', 'RPI.79: the renderer gate on the advertised combination with user stages, in one run', False,
         25, 'user shader stages compiled by DXC in the product build (RPI.80)',
         prerequisites=(('imports', 'renderer_gate'), ('presentmon', None)),
         run_time=('administrator PowerShell started on the desktop', 'foreground', 'mains power'),
         sources=('RPI.79', 'RPI.80', 'owner decision 2026-11-18 (DXC ships)')),
    Step('e3-cold-start', 'Charter E3: three cold runs, process start to the first correct frame, each at most 3 s',
         False, 10, 'the launcher and the product shell\'s startup log (magic600-c9-startup-v1)',
         commands=('<launcher> three times, 20 s apart, each on a fresh session copy',
                   'python -B tools/perf/c9/cold_start.py <run 1> <run 2> <run 3> --out <record>.json'),
         prerequisites=(('imports', 'cold_start'),),
         run_time=('no product process running', 'a fresh copy of the session fixture'),
         sources=('charter E3', 'owner decision 2026-11-18', 'RTE.42', 'CT-RTE-11')),
    Step('lx2', 'LX-2: fault placements, at least 100 commits under charter E2 during fault windows', False, 60,
         'the LX-2 fault harness on the product build',
         commands=('cargo run --locked --offline --release --manifest-path rust/Cargo.toml -p c600-lx2 -- '
                   '--runtime <product build c600-runtime> --sessions <new folder> --out <record>.json'
                   '   # metered and forced-kill half, alone and in a session (conditions 1 and 2)',),
         prerequisites=(('file', 'rust/c600-lx2/src/main.rs'),),
         sources=('RTE.42', 'CT-RTE-10', 'CT-RTE-11', 'CT-LG4-09')),
    Step('lx4', 'LX-4: 10,800 S/M/L samples in three series per puzzle, time box 4 hours', False, 240,
         'the LX-4 edit-script harness on the product build',
         commands=('cargo run --locked --offline --manifest-path rust/Cargo.toml -p c600-lx4 -- --out <new folder>'
                   '   # module set and edit script (DRAFT until RTE.51 step 1 freezes it)',),
         prerequisites=(('file', 'rust/c600-lx4/src/main.rs'),),
         run_time=('foreground',), sources=('RTE.52', 'CT-RTE-12', 'CT-EV1-06', 'owner decision 2026-11-18')),
    Step('b3-probe', 'Route B check B3: three cold runs of the present probe', False, 10, None,
         commands=(f'python -B {PROBE} --topmost --wait-quiet 1800   # three times',),
         prerequisites=(('compiles', PROBE), ('compiles', 'work/experiments/c9-present-probe/analyze.py'),
                        ('file', 'work/experiments/c9-present-probe/native/build.cmd'), ('vswhere', None),
                        ('command', 'dotnet'), ('godot', None)),
         run_time=('an unlocked desktop', 'mains power'),
         sources=('owner decision 2026-11-18 route B, check 3',)),
    Step('b2-skew', 'Route B check B2: skew between the UI frame and the 3D frame', False, 10,
         'a skew measurement in the product shell (UI frame times beside the core\'s confirmed presents)',
         run_time=('an unlocked desktop',), sources=('owner decision 2026-11-18 route B, check 2',)),
)


def newest_build(root=ROOT):
    """The newest prepared acceptance build directory that holds an identity file, or None."""
    candidates = sorted((path.parent for path in (root / BUILDS).glob(f'*/{IDENTITY}')), key=lambda path: path.name)
    return candidates[-1] if candidates else None


def read_identity(directory):
    """(ok, detail) for a build directory's identity file."""
    try:
        identity = json.loads((Path(directory) / IDENTITY).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return False, f'{IDENTITY} unreadable'
    if not isinstance(identity, dict):
        return False, f'{IDENTITY} is not {IDENTITY_FORMAT}'
    source = identity.get('source')
    if identity.get('format') != IDENTITY_FORMAT or not isinstance(source, dict) or not source.get('head'):
        return False, f'{IDENTITY} is not {IDENTITY_FORMAT}'
    return True, f"build {Path(directory).name}, source {str(source['head'])[:12]}, ABI {identity.get('abi_version')}"


def godot(environ=None):
    environ = os.environ if environ is None else environ
    local = environ.get('LOCALAPPDATA')
    found = sorted((Path(local) / 'Microsoft' / 'WinGet' / 'Packages').glob(GODOT_GLOB)) if local else []
    return bool(found), 'Godot 4.7.2 .NET console found' if found else 'Godot 4.7.2 .NET console missing'


def evaluate(kind, argument, build=None, environ=None):
    """(ok, detail) for one prerequisite; it reads files and metadata only and starts nothing."""
    environ = os.environ if environ is None else environ
    if kind == 'file':
        return (ROOT / argument).is_file(), argument
    if kind == 'dir':
        return (ROOT / argument).is_dir(), argument
    if kind == 'compiles':
        try:
            compile((ROOT / argument).read_bytes(), argument, 'exec', dont_inherit=True)
            return True, f'{argument} compiles'
        except (OSError, SyntaxError, ValueError):
            return False, f'{argument} does not compile'
    if kind == 'imports':
        sys.dont_write_bytecode = True
        try:
            importlib.import_module(argument)
            return True, f'{argument} imports'
        except Exception:  # noqa: BLE001 - any import failure is a missing prerequisite
            return False, f'{argument} does not import'
    if kind == 'command':
        return shutil.which(argument) is not None, f'{argument} on PATH'
    if kind == 'presentmon':
        found = presentmon_capture.installed(environ)
        if found['path'] is None:
            return False, 'PresentMon missing'
        return found['version_ok'], f"PresentMon {found['version'] or 'version unreadable'}"
    if kind == 'build':
        if build is None:
            return False, f'no prepared build under {BUILDS}'
        return read_identity(build)
    if kind == 'godot-launch':
        if build is None:
            return False, 'no launch.json without a build'
        try:
            launch = json.loads((Path(build) / 'launch.json').read_text(encoding='utf-8'))
            return Path(launch['executable']).is_file(), 'Godot from launch.json'
        except (OSError, ValueError, UnicodeError, KeyError, TypeError):
            return False, 'launch.json unreadable'
    if kind == 'godot':
        return godot(environ)
    if kind == 'vswhere':
        base = environ.get('ProgramFiles(x86)')
        present = bool(base) and (Path(base) / 'Microsoft Visual Studio' / 'Installer' / 'vswhere.exe').is_file()
        return present, 'Visual Studio locator (vswhere)'
    raise ValueError(f'unknown prerequisite kind {kind!r}')


def quiet_machine(run=subprocess.run, timeout=120):
    """The probe's quiet-machine gate: (quiet, gate JSON or None). Starts a process; never called by the tests."""
    command = [sys.executable, '-B', str(ROOT / PROBE), '--check-only']
    try:
        completed = run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return False, None
    try:
        gate = json.loads(completed.stdout)
    except ValueError:
        gate = None
    return completed.returncode == 0, gate


def line(index, step, results):
    kind = 'attended' if step.attended else 'unattended, quiet gate'
    failed = [detail for ok, detail in results if not ok]
    if not step.built:
        state = f'NOT BUILT YET: {step.missing}'
        if failed:
            state += f' (also missing: {"; ".join(failed)})'
    elif failed:
        state = f'MISSING: {"; ".join(failed)}'
    else:
        state = f'ready ({len(results)} prerequisite(s) checked)'
    return f'{index}. {step.id} [{kind}, about {step.minutes} min] {state}'


def dry_run(steps=STEPS, check=evaluate, out=print):
    """One line per step; 1 when a built step misses a prerequisite, else 0."""
    code = 0
    for index, step in enumerate(steps, 1):
        results = [check(kind, argument) for kind, argument in step.prerequisites]
        out(line(index, step, results))
        if step.built and not all(ok for ok, _ in results):
            code = 1
    attended = sum(step.minutes for step in steps if step.attended)
    out(f'total: about {attended} min attended, then about {sum(step.minutes for step in steps) - attended} min '
        f'unattended; built: {sum(step.built for step in steps)} of {len(steps)} steps')
    return code


def listing(steps=STEPS, out=print):
    for index, step in enumerate(steps, 1):
        out(f"{index}. {step.id}: {step.title}")
        out(f"   {'attended (foreground)' if step.attended else 'unattended; start only when the quiet gate passes'}; "
            f'about {step.minutes} min (draft)')
        if not step.built:
            out(f'   not built yet: {step.missing}')
        for command in step.commands:
            out(f"   {'planned: ' if not step.built else ''}{command}")
        for kind, argument in step.prerequisites:
            out(f"   needs: {kind}{'' if argument is None else ' ' + argument}")
        for condition in step.run_time:
            out(f'   at run time: {condition}')
        out(f"   sources: {', '.join(step.sources)}")


def main(argv=None):
    parser = argparse.ArgumentParser(description='C9 owner-machine runbook (runs nothing).')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--list', action='store_true', help='print every step with its commands and prerequisites')
    mode.add_argument('--dry-run', action='store_true', help='check the prerequisites, one line per step')
    parser.add_argument('--build', type=Path, help='prepared build directory (default: the newest under '
                        f'{BUILDS})')
    parser.add_argument('--check-quiet', action='store_true', help='also run the quiet-machine gate once (info only)')
    args = parser.parse_args(argv)
    if args.list:
        listing()
        return 0
    build = args.build or newest_build()
    code = dry_run(check=lambda kind, argument: evaluate(kind, argument, build))
    if args.check_quiet:
        quiet, gate = quiet_machine()
        print(f"quiet-machine gate: {'quiet' if quiet else 'not quiet'}"
              f"{'' if gate is None else ' (' + ', '.join(gate.get('reasons', []) or ['no reasons']) + ')'}")
    return code


if __name__ == '__main__':
    sys.exit(main())
