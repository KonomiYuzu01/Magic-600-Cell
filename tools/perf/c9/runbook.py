"""Runbook for the C9 owner-machine batch (LP3, owner 16 and 18 November 2026). Standard library only.

    python -B tools/perf/c9/runbook.py --list                       the steps with commands and prerequisites
    python -B tools/perf/c9/runbook.py --dry-run [--build DIR] [--check-quiet]

The batch runs on one day on the owner's machine (owner, 16 November 2026), attended steps first: the shell's first
launch over a real core (ABI 5's GPU smoke, part 2), route B's input check B1 and charter E2 need the owner and the
foreground. The unattended steps follow, each started only when the quiet-machine gate passes: the core self-test's
WARP and hardware modes (`check_core.py --gpu`, ABI 5's GPU smoke, part 1), W-J item 5 and W3 on the product build
(ADR-R01.3, route B), RPI.79's gate run on a stage build (three cold W3 runs with the user stages), three
cold E3 runs, LX-2, LX-4 (4 h time box, owner 18 November 2026), and route B's three cold probe runs (B3) and its
UI/3D skew measurement (B2). "Unattended" means the owner need not watch; the W-J, W3 and probe windows still need an
unlocked desktop and the foreground.

The steps run in the owner runbook's order and carry its labels (A1 to A3 attended, U1 to U8 unattended).
`tools/perf/c9/unattended.ps1` runs U1 to U8 with one command from an administrator PowerShell on the desktop. In each
shell run (U3, U6, U7, U8) it starts PresentMon and the poster without a window and `launch.py` last, so the shell gets
the focus (C9, 20 Nov 2026: a capture started after the shell took it away). For U3, U7 and U8 `foreground.py` then
makes the shell topmost and asks once for the foreground (no foreground-lock workaround); a shell run in which the
owner's input arrives or the shell loses the foreground is stopped, set aside as `<name>.aborted-<n>` and run again,
at most twice.

The runbook runs nothing. `--dry-run` checks, without a GPU, window or capture, every prerequisite it can (files and
tools present, the PresentMon version, the build identity readable, scripts that compile or import, fixtures present,
and whether the product shell's sources write the log rows or handle the messages a harness needs: `mentions`, a text
search, DRAFT) and prints one line per step. It exits 1 when a prerequisite of a step that is built is missing; a step whose
component is not built yet prints its missing prerequisites as a warning. Time estimates are DRAFTS from the runners'
own timings (run_scene.ps1: 20 s pause, 4 s preroll, 192 s trace per cold run; the probe: about 31 s per harness).

One build identity (PHS.23, PHS.64): the sitting runs on the frozen candidate only, so every build a step uses is made
from the checkout's HEAD. `--dry-run` reads HEAD once (`git rev-parse HEAD`, its only process) and refuses a route-B
acceptance build, a stage build or a product build made from another commit, and a product build made from a changed
tree (C9, 21 November 2026).

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
PREPARE_HOST = 'renderer/acceptance/prepare_host.py'
PREPARE_DATA = 'renderer/acceptance/prepare_data.py'
WJ_DATA = 'work/loop-memory/c9-acceptance/wj-data'
# C9 U5c: the gate's user-stage pair; prepare_host.py --stages builds it into a route-B build's stages/ folder.
STAGES = 'renderer/acceptance/stages'
RENDER_BACKEND = 'rust/c600-render/src/backend.rs'
# The acceptance host's route (prepare_host.py --route): U4 needs route B's; a build without one is a texture build.
SURFACE = 'surface'
SHELL = 'renderer/shell'
LAUNCH = 'renderer/shell/launch.py'
BUILD_PRODUCT = 'renderer/shell/build_product.py'
# build_product.py's output: one folder per build, each with its record (head, dirty, identity, parts).
PRODUCT_BUILDS = 'work/build/c9-product'
PRODUCT_RECORD = 'product.json'
FINALIZE = 'renderer/acceptance/finalize_run.py'
# C9 owner item 9 (PR 385): on route B the core writes presented.jsonl, the traced frames its own frame statistics
# confirmed, and the finalizer's 'presented' check reads it; PresentMon becomes the cross-check.
ROUTE_B_DISPLAY = ('route B\'s display evidence is the core\'s own frame statistics (presented.jsonl, the finalizer\'s '
                   '"presented" check), cross-checked against the PresentMon capture (owner, 20 November 2026, item 9)')
# The owner runbook's labels (C9 draft, 18 November 2026): A attended, U unattended.
LABELS = {'shell-launch': 'A1', 'b1-input': 'A2', 'e2-key-to-frame': 'A3', 'core-gpu': 'U1', 'b3-probe': 'U2',
          'b2-skew': 'U3', 'wj-w3': 'U4', 'rpi79-gate': 'U5', 'e3-cold-start': 'U6', 'lx2': 'U7', 'lx4': 'U8'}
SOURCE_SUFFIXES = ('.cs', '.gd', '.rs', '.py', '.cpp', '.h')
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
    prepare = (f'python -B {PREPARE_HOST} --route surface   # route B\'s host into {BUILDS}/<build>, offline; '
               'no Godot window or GPU device')
    data = (f'python -B {PREPARE_DATA}   # W-J\'s exports for S4, I_a and I_b into {WJ_DATA}, checked; no GPU')
    check = (f'& {RUNNER} -Scene <scene> -Build <route-B build> {DECLARE} -DryRun   # per scene (S4, I_a, I_b, w3): '
             'verifies the build (route: surface) and the W-J data, starts nothing')
    runs = [f"& {RUNNER} -Scene {scene} -Build <route-B build> {DECLARE}   # three times"
            for scene in ('S4', 'I_a', 'I_b', 'w3')]
    gate = ('python -B tools/perf/renderer_gate.py <run 1> <run 2> <run 3> '
            '--out work/loop-memory/c9-acceptance/<scene>-summary.json   # per scene')
    return (prepare, data, check, *runs, gate)


def stage_gate_commands():
    # A stage build runs W3 only (run_scene.ps1 refuses the W-J scenes on it), so U5 has a build of its own and U4's
    # three built-in-chain W3 runs are its comparison (RTE.60).
    prepare = (f'python -B {PREPARE_HOST} --route surface --stages {STAGES}   # route B\'s host with the gate\'s '
               f'user-stage pair and DXC beside the core into {BUILDS}/<stage build>; offline, no GPU device')
    check = (f'& {RUNNER} -Scene w3 -Build <stage build> {DECLARE} -DryRun   # verifies the stage build and prints '
             'the stage plan; starts nothing')
    runs = f'& {RUNNER} -Scene w3 -Build <stage build> {DECLARE}   # three times, cold'
    gate = ('python -B tools/perf/renderer_gate.py <run 1> <run 2> <run 3> '
            '--out work/loop-memory/c9-acceptance/w3-stages-summary.json   # U4\'s W3 runs are the built-in '
            'comparison')
    return (prepare, check, runs, gate)


STEPS = (
    Step('shell-launch', 'ABI 5 GPU smoke, part 2: the product shell\'s first launch over a real core', True, 10, None,
         commands=('python -B renderer/core/check_core.py   # the core into work/build/, CPU self-test only',
                   f'python -B {BUILD_PRODUCT}   # the host and the shell into work/build/c9-product/<build>',
                   f'python -B {LAUNCH} --out <new folder> --dry-run   # verifies every build part, starts nothing; '
                   'exit 2: no build, or a part does not verify',
                   f'python -B {LAUNCH} --out <new folder>   # from a desktop PowerShell; resize the window twice, '
                   'then close it'),
         prerequisites=(('compiles', 'renderer/core/check_core.py'), ('compiles', LAUNCH),
                        ('compiles', BUILD_PRODUCT), ('product-build', None)),
         run_time=('foreground', 'the owner at the machine', 'native resolution, no frame generation or upscaling',
                   'pass: the shell exits 0; launch.json in the run folder; events.jsonl with frame and presented '
                   'rows (display_qpc); the 3D view under the transparent window; no error on resize or close'),
         sources=('ABI 5 (route B): sa2_create, surface frames, sa2_presented, sa2_resize',
                  'owner decision 2026-11-18 route B')),
    Step('b1-input', 'Route B check B1: input through Godot\'s transparent window over the core\'s layer', True, 10,
         None,
         commands=(f'python -B {LAUNCH} --out <new folder> --echo   # press keys and click, then close the window',),
         prerequisites=(('compiles', LAUNCH), ('mentions', (SHELL, 'input.jsonl')), ('product-build', None)),
         run_time=('foreground', 'the owner at the machine',
                   'pass: every key press and click shows on the echo panel and is in <out>/input.jsonl'),
         sources=('owner decision 2026-11-18 route B, check 1',)),
    Step('e2-key-to-frame', 'Charter E2: key to the confirmed first correct frame, at least 100 presses', True, 20,
         None,
         commands=(f'python -B {LAUNCH} --out <new folder>   # the session is always on (<out>/session, twist menu '
                   'wj-I_a); press about 120 twists (J, K, L; Shift for the inverse) at a steady pace, about one a '
                   'second, touching no other window, then close the window',
                   'python -B tools/perf/c9/key_to_frame.py <run dir> [<run dir> ...] --out <record>.json'),
         prerequisites=(('imports', 'key_to_frame'), ('presentmon', None), ('compiles', LAUNCH),
                        ('mentions', (SHELL, 'WriteString("t", "key")')),
                        ('mentions', (SHELL, 'journal-reopen-and-kernel-replay')), ('product-build', None)),
         run_time=('foreground', 'the owner at the machine', 'a fresh run folder (the session is new in it)',
                   'the run row\'s correctness comes from the session\'s control at close '
                   '(journal-reopen-and-kernel-replay)',
                   'a key whose frame the core does not confirm counts as one failed key above 100 ms, and its run '
                   'stays valid (owner, 20 November 2026, item 2); at least 100 confirmed keys are needed, hence '
                   'about 120 presses; a lost foreground makes the run invalid'),
         sources=('charter E2', 'RTE.43', 'V11.1', 'PHS.64')),
    Step('core-gpu', 'ABI 5 GPU smoke, part 1: the renderer core self-test\'s WARP and hardware modes', False, 15, None,
         commands=('python -B renderer/core/check_core.py --gpu   # pass: check_core: ok (CPU, WARP and hardware)',),
         prerequisites=(('compiles', 'renderer/core/check_core.py'), ('file', 'renderer/core/build.cmd')),
         run_time=('the D3D12 debug layer present', 'no debug-layer error or corruption'),
         sources=('renderer/core/README.md, GPU checks (owner runs)', 'owner decision 2026-11-18 route B')),
    Step('b3-probe', 'Route B check B3: three cold runs of the present probe', False, 10, None,
         commands=(f'python -B {PROBE} --topmost --wait-quiet 1800   # three times',),
         prerequisites=(('compiles', PROBE), ('compiles', 'work/experiments/c9-present-probe/analyze.py'),
                        ('file', 'work/experiments/c9-present-probe/native/build.cmd'), ('vswhere', None),
                        ('command', 'dotnet'), ('godot', None)),
         run_time=('an unlocked desktop', 'mains power'),
         sources=('owner decision 2026-11-18 route B, check 3',)),
    Step('b2-skew', 'Route B check B2: skew between the UI frame and the 3D frame', False, 10, None,
         commands=('first presentmon_capture.py --process-name <the build\'s Godot executable> --output <capture>.csv '
                   '--seconds 600 without a window, then, last, '
                   f'python -B {LAUNCH} --build <product build> --out <run dir>, so the shell gets the focus; touch '
                   'no other window, do not resize the shell (a second core chain makes the run unreadable), close '
                   'it after 540 s and copy <capture>.csv to <run dir>/presentmon.csv   # unattended.ps1 -Steps U3',
                   'python -B tools/perf/c9/ui_skew.py <run dir> --out <record>.json'
                   '   # no bar: the numbers go to the owner; check "UI present inside the draw"'),
         prerequisites=(('imports', 'ui_skew'), ('presentmon', None), ('compiles', LAUNCH),
                        ('mentions', (SHELL, '"ui_frame"')), ('mentions', (SHELL, '"ui_swap_chain"')),
                        ('mentions', (SHELL, '"drawn_qpc"')), ('product-build', None)),
         run_time=('an unlocked desktop', 'administrator PowerShell for PresentMon'),
         sources=('owner decision 2026-11-18 route B, check 2',)),
    Step('wj-w3', 'W-J item 5 (three cold runs each of S4, I_a, I_b) and W3 (three cold runs) on the product build',
         False, 50, None,
         commands=wj_w3_commands(),
         prerequisites=(('file', RUNNER), ('compiles', FINALIZE),
                        ('compiles', PREPARE_HOST), ('compiles', PREPARE_DATA),
                        ('compiles', 'renderer/acceptance/file_guard.py'), ('imports', 'renderer_gate'),
                        ('command', 'powershell'), ('presentmon', None), ('build', SURFACE), ('godot-launch', None),
                        ('dir', WJ_DATA), ('mentions', ('renderer/core/src', 'presented.jsonl')),
                        ('mentions', (FINALIZE, "'presented.jsonl'"))),
         run_time=('administrator PowerShell started on the desktop', 'foreground', 'mains power',
                   'native resolution, no frame generation or upscaling',
                   'the W-J and W3 evidence comes from the route-B build, not a texture build', ROUTE_B_DISPLAY),
         sources=('ADR-R01.3', 'owner decision 2026-11-18 route B')),
    Step('rpi79-gate', 'RPI.79: the renderer gate on the advertised combination with user stages, in one run', False,
         25, None,
         commands=stage_gate_commands(),
         # C9 U5c (PR 316): the harness compiles both stages with the core's DXC when W3 loads, and the finalizer
         # refuses a failed compile, a measured frame without a row and a stopped frame.
         prerequisites=(('file', RUNNER), ('compiles', FINALIZE),
                        ('compiles', PREPARE_HOST), ('dir', STAGES), ('imports', 'renderer_gate'),
                        ('command', 'powershell'), ('presentmon', None),
                        ('mentions', (FINALIZE, "'stage-stopped'")), ('mentions', (FINALIZE, "'presented.jsonl'")),
                        ('stage-build', SURFACE)),
         run_time=('administrator PowerShell started on the desktop', 'foreground', 'mains power',
                   'native resolution, no frame generation or upscaling', ROUTE_B_DISPLAY),
         sources=('RPI.79', 'RPI.80', 'RTE.60', 'owner decision 2026-11-18 (DXC ships)')),
    Step('e3-cold-start', 'Charter E3: three cold runs, process start to the first correct frame, each at most 3 s',
         False, 10, None,
         commands=(f'python -B {BUILD_PRODUCT}   # E3\'s build carries the session fixture and c600-runtime',
                   f'python -B {LAUNCH} --out <new folder> --cold-start --dry-run   # checks the build, starts '
                   'nothing',
                   f'python -B {LAUNCH} --out <warm-up folder> --cold-start   # once, untimed: the first launch of a '
                   'fresh build also pays Windows\' first-run checks of its new binaries (PR 387\'s local leads: '
                   '4.37 s against 3.1 s before it, 2.58 s against 1.9 s after); '
                   'the cold definition allows earlier launches of the same build (cold_start.py); keep the folder',
                   f'python -B {LAUNCH} --out <run n> --cold-start   # three times, each into a new folder; the '
                   'launcher waits until 20 s after the previous product exit and copies the fixture fresh; the '
                   'shell quits at the first correct frame and writes startup.jsonl',
                   'python -B tools/perf/c9/cold_start.py <run 1> <run 2> <run 3> --out <record>.json',
                   'python -B tools/perf/c9/cold_start.py <warm-up folder> --out <warm-up record>.json   # the '
                   'warm-up\'s own time, reported beside the record and never counted (one run: insufficient)'),
         prerequisites=(('imports', 'cold_start'), ('compiles', LAUNCH), ('compiles', BUILD_PRODUCT),
                        ('mentions', (SHELL, '"--cold-start"')),
                        # PR 387: the host's model loads start before the core's surface opens (host ABI 6).
                        ('mentions', (SHELL, '"m6h_preload"')), ('product-build', None)),
         run_time=('no product process running', 'a fresh copy of the session fixture',
                   'the three timed runs come after the warm-up launch (unattended.ps1 -Steps U6 runs it first)'),
         sources=('charter E3', 'owner decision 2026-11-18', 'RTE.42', 'CT-RTE-11')),
    Step('lx2', 'LX-2: fault placements, at least 100 commits under charter E2 during fault windows', False, 79, None,
         commands=('cargo build --locked --offline --release --manifest-path rust/Cargo.toml -p c600-lx2 '
                   '--features device --bin c600-lx2-child   # the injection device of option (a) on c600-runtime\'s '
                   'fault hook, never shipped',
                   'cargo run --locked --offline --release --manifest-path rust/Cargo.toml -p c600-lx2 '
                   '--features device --bin c600-lx2-faults -- '
                   '--runtime <product build c600-runtime> --injection <c600-lx2-child release build> '
                   '--sessions <new folder> --out <record>.json'
                   '   # metered, forced-kill, allocator-ceiling, native recursion and native loop trials, alone '
                   'and in a session (conditions 1 and 2)',
                   'cargo run --locked --offline --manifest-path rust/Cargo.toml -p c600-lx4 -- --out <fixtures>'
                   '   # LX-4\'s module set and edit script, once; U8 uses the same fixtures',
                   'python -B tools/perf/c9/lx2.py post --fixtures <fixtures> --dry-run   # the fault plan and its '
                   'time; posts nothing',
                   'first presentmon_capture.py --process-name <the build\'s Godot executable> --output '
                   '<capture>.csv --seconds 4800 and python -B tools/perf/c9/lx2.py post --fixtures <fixtures> --run '
                   '<run dir>, both without a window (the poster waits for the shell), then, last, '
                   f'python -B {LAUNCH} --build <product build> --out <run dir> --lx4 --lx4-fixtures <fixtures> '
                   '--lx2-injection <c600-lx2-child release build>, so the shell gets the focus; touch no other window '
                   '  # unattended.ps1 -Steps U7; each metered fault\'s limit windows (its job runs to the host\'s 60 s '
                   'safety stop, no commit), then every fault\'s turn windows, with echo samples, about 13 minutes (at '
                   'most about 70); the poster closes the shell after the last phase; copy <capture>.csv to '
                   '<run dir>/presentmon.csv',
                   'python -B tools/perf/c9/lx2.py analyze --fixtures <fixtures> <run dir> --faults <record>.json '
                   '--out <lx2 record>.json   # conditions 3 and 4 per fault, with 1 and 2 from the faults record'),
         prerequisites=(('file', 'rust/c600-lx2/src/main.rs'), ('file', 'rust/c600-lx2/src/bin/c600-lx2-child.rs'),
                        ('mentions', ('rust/c600-runtime/src', 'pub fn main_with')),
                        ('mentions', ('rust/c600-lx2/src', 'main_with(native::point)')),
                        ('file', 'rust/c600-lx4/src/main.rs'), ('imports', 'lx2'), ('presentmon', None),
                        ('compiles', LAUNCH), ('mentions', (LAUNCH, '"--lx2-injection"')),
                        ('mentions', (SHELL, 'Magic600.LX2.Phase')), ('mentions', (SHELL, 'WriteString("t", "lx2")')),
                        ('mentions', (SHELL, 'Magic600.LX4.Reset')), ('mentions', (SHELL, 'WriteString("t", "key")')),
                        ('mentions', (LAUNCH, 'M600_LX4_DLL')), ('mentions', (LAUNCH, 'M600_LX4_FIXTURES')),
                        ('product-build', None)),
         run_time=('foreground', 'the owner starts it from the desktop, which stays unlocked'),
         sources=('RTE.42', 'CT-RTE-10', 'CT-RTE-11', 'CT-LG4-09', 'charter E2', 'renderer gate')),
    Step('lx4', 'LX-4: 10,800 S/M/L samples in three series per puzzle, time box 4 hours', False, 240, None,
         commands=('cargo run --locked --offline --manifest-path rust/Cargo.toml -p c600-lx4 -- --out <fixtures>'
                   '   # module set and edit script (DRAFT until RTE.51 step 1 freezes it)',
                   'python -B tools/perf/c9/lx4.py schedule --fixtures <fixtures>   # 10,800 S/M/L samples',
                   'first presentmon_capture.py --process-name <the build\'s Godot executable> --output '
                   '<capture>.csv --seconds 10800 and python -B tools/perf/c9/lx4.py post --fixtures <fixtures> '
                   '--puzzle jumble-off --run <run dir> --turn-key KeyJ, both without a window (the poster waits for '
                   f'the shell), then, last, python -B {LAUNCH} --build <product build> --out <run dir> --lx4 '
                   '--lx4-fixtures <fixtures> --puzzle jumble-off, so the shell gets the focus; touch no other window; '
                   'copy <capture>.csv to <run dir>/presentmon.csv   # unattended.ps1 -Steps U8',
                   'first presentmon_capture.py --process-name <the build\'s Godot executable> --output '
                   '<second capture>.csv --seconds 10800 and python -B tools/perf/c9/lx4.py post --fixtures <fixtures> '
                   '--puzzle jumble-on --run <second run dir> --turn-key KeyJ, both without a window, then, last, '
                   f'python -B {LAUNCH} --build <product build> --out <second run dir> --lx4 --lx4-fixtures <fixtures> '
                   '--puzzle jumble-on, so the shell gets the focus; touch no other window; copy <second capture>.csv '
                   'to <second run dir>/presentmon.csv   # unattended.ps1 -Steps U8; the host reopens the build\'s '
                   'jumble-on session and the core draws its posed engine images',
                   'python -B tools/perf/c9/key_to_frame.py <both run dirs> --out <e2 record>.json   # E2 in the same '
                   'runs',
                   'python -B tools/perf/c9/lx4.py analyze --fixtures <fixtures> <both run dirs> --e2 <e2 record>.json '
                   '--gate-reference <U4 W3 gate summary>.json --out <record>.json   # renderer_gate.py measures '
                   'acceptance builds only, so the summary is recorded, and counted as the gate only where the owner '
                   'accepted it (lx4.py)'),
         prerequisites=(('file', 'rust/c600-lx4/src/main.rs'), ('imports', 'lx4'), ('presentmon', None),
                        ('compiles', LAUNCH), ('mentions', (SHELL, '"--lx4"')),
                        ('mentions', (SHELL, 'Magic600.LX4.Reset')), ('mentions', (SHELL, '"lx4"')),
                        ('mentions', (SHELL, 'WriteString("t", "key")')),
                        # The module refuses to start without its library and fixtures (C9 product's launcher and
                        # product build hand them over).
                        ('mentions', (LAUNCH, '"--lx4-fixtures"')), ('mentions', (LAUNCH, 'M600_LX4_DLL')),
                        ('mentions', (LAUNCH, 'M600_LX4_FIXTURES')),
                        ('mentions', (BUILD_PRODUCT, 'c600-shell-lx4')),
                        # Jumble on: the build's jumble-on session fixture, and c600-render adopting the jumbling
                        # engine bundle as the core's posed engine images (C9 product, 20 November 2026).
                        ('mentions', (BUILD_PRODUCT, '"session-fixture-jumble-on"')),
                        ('mentions', (RENDER_BACKEND, 'Puzzle::Jumbling =>')),
                        # The rerun endpoint (RTE.50, PR 386): the shell runs the edited buffer's reference view as
                        # the host's view job and reports its end to the library.
                        ('mentions', (SHELL, '"lx4_rerun_ended"')), ('mentions', (SHELL, '"m6h_run_view"')),
                        ('product-build', None)),
         run_time=('foreground', 'the owner starts it from the desktop',
                   'a rerun sample ends at the first confirmed frame that drew the re-run\'s reference publication; '
                   'the host does not draw the run\'s value yet (lx4.py LIMITATIONS)'),
         sources=('RTE.51', 'RTE.52', 'RTE.53', 'CT-RTE-12', 'CT-EV1-06', 'owner decision 2026-11-18')),
)


def build_route(directory):
    """The route in a build's identity file ('texture' when it names none), or None when it is unreadable."""
    try:
        identity = json.loads((Path(directory) / IDENTITY).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return None
    return identity.get('route', 'texture') if isinstance(identity, dict) else None


def build_stages(directory):
    """The user-stage record in a build's identity file (prepare_host.py --stages), or None."""
    try:
        identity = json.loads((Path(directory) / IDENTITY).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return None
    stages = identity.get('stages') if isinstance(identity, dict) else None
    return stages if isinstance(stages, dict) else None


def newest_build(root=ROOT, route=None, stages=None):
    """The newest prepared acceptance build directory that holds an identity file (of that route, when given; with
    or without user stages, when stages is True or False), or None."""
    candidates = sorted((path.parent for path in (root / BUILDS).glob(f'*/{IDENTITY}')), key=lambda path: path.name)
    if route is not None:
        candidates = [path for path in candidates if build_route(path) == route]
    if stages is not None:
        candidates = [path for path in candidates if (build_stages(path) is not None) == stages]
    return candidates[-1] if candidates else None


def read_identity(directory, route=None, stages=False, head=None, note=''):
    """(ok, detail) for a build directory's identity file; with a route, the build must be that route's, and it must
    hold user stages exactly when stages is True; with a head, it must be built from that commit. The note follows the
    build's description, before any command that would fix it."""
    try:
        identity = json.loads((Path(directory) / IDENTITY).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return False, f'{IDENTITY} unreadable'
    if not isinstance(identity, dict):
        return False, f'{IDENTITY} is not {IDENTITY_FORMAT}'
    source = identity.get('source')
    if identity.get('format') != IDENTITY_FORMAT or not isinstance(source, dict) or not source.get('head'):
        return False, f'{IDENTITY} is not {IDENTITY_FORMAT}'
    built = identity.get('route', 'texture')
    detail = (f"build {Path(directory).name}, source {str(source['head'])[:12]}, ABI {identity.get('abi_version')}, "
              f'route {built}{note}')
    if route is not None and built != route:
        return False, f'{detail}, not {route}: python -B {PREPARE_HOST} --route {route}'
    if route is not None and (identity.get('stages') is not None) != stages:
        if stages:
            return False, f'{detail}, no user stages: python -B {PREPARE_HOST} --route {route} --stages {STAGES}'
        return False, f'{detail}, with user stages (W3 only): python -B {PREPARE_HOST} --route {route}'
    if head is not None and source['head'] != head:
        again = f' --route {route}' + (f' --stages {STAGES}' if stages else '') if route is not None else ''
        return False, f'{detail}, not HEAD {head[:12]}: python -B {PREPARE_HOST}{again}'
    return True, detail


def newest_product(root=ROOT):
    """The newest product build that holds its record, or None."""
    found = sorted((path.parent for path in (root / PRODUCT_BUILDS).glob(f'*/{PRODUCT_RECORD}')),
                   key=lambda path: path.name)
    return found[-1] if found else None


def read_product(directory, head=None):
    """(ok, detail) for a product build's record: readable, made from a clean tree and, with a head, from that commit.
    The parts' digests are launch.py --dry-run's to recompute."""
    try:
        record = json.loads((Path(directory) / PRODUCT_RECORD).read_text(encoding='utf-8'))
    except (OSError, ValueError, UnicodeError):
        return False, f'{PRODUCT_RECORD} unreadable in {Path(directory).name}'
    if not isinstance(record, dict) or not isinstance(record.get('head'), str) or not record.get('identity'):
        return False, f'{PRODUCT_RECORD} in {Path(directory).name} names no head or identity'
    detail = f"product build {Path(directory).name}, {record['identity']}, source {record['head'][:12]}"
    if record.get('dirty') is not False:
        return False, f'{detail}, built from a changed tree: python -B {BUILD_PRODUCT}'
    if head is not None and record['head'] != head:
        return False, f'{detail}, not HEAD {head[:12]}: python -B {BUILD_PRODUCT}'
    return True, detail


def checkout_head(run=subprocess.run):
    """The checkout's HEAD commit, or None when git does not answer."""
    try:
        completed = run(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], capture_output=True, text=True, timeout=60,
                        check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    text = completed.stdout.strip()
    return text if completed.returncode == 0 and len(text) == 40 else None


def godot(environ=None):
    environ = os.environ if environ is None else environ
    local = environ.get('LOCALAPPDATA')
    found = sorted((Path(local) / 'Microsoft' / 'WinGet' / 'Packages').glob(GODOT_GLOB)) if local else []
    return bool(found), 'Godot 4.7.2 .NET console found' if found else 'Godot 4.7.2 .NET console missing'


def evaluate(kind, argument, build=None, environ=None, head=None):
    """(ok, detail) for one prerequisite; it reads files and metadata only and starts nothing. With a head, every build
    must be made from that commit."""
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
    if kind == 'mentions':
        folder, token = argument
        where = ROOT / folder
        for path in [where] if where.is_file() else sorted(where.rglob('*')):
            if path.suffix in SOURCE_SUFFIXES and path.is_file():
                try:
                    if token in path.read_text(encoding='utf-8', errors='replace'):
                        return True, f'{folder} writes or handles {token}'
                except OSError:
                    continue
        return False, f'{folder} does not write or handle {token} yet'
    if kind == 'presentmon':
        found = presentmon_capture.installed(environ)
        if found['path'] is None:
            return False, 'PresentMon missing'
        return found['version_ok'], f"PresentMon {found['version'] or 'version unreadable'}"
    if kind == 'build':
        if build is None:
            return False, f'no prepared build under {BUILDS}'
        return read_identity(build, argument, head=head)
    if kind == 'stage-build':
        # U5's own build, found apart from --build: the newest build of that route with user stages.
        found = newest_build(ROOT, argument, stages=True)
        if found is None:
            return False, (f'no {argument} build with user stages under {BUILDS}: '
                           f'python -B {PREPARE_HOST} --route {argument} --stages {STAGES}')
        record = build_stages(found) or {}
        dxc = record.get('dxc') if isinstance(record.get('dxc'), dict) else {}
        return read_identity(found, argument, stages=True, head=head,
                             note=f", user stages: overdraw {record.get('overdraw')}, DXC {dxc.get('version')}")
    if kind == 'product-build':
        # The build U3, U6, U7 and U8 launch (unattended.ps1 -ProductBuild) and A1 to A3 use: the newest one.
        found = newest_product(ROOT)
        if found is None:
            return False, f'no product build under {PRODUCT_BUILDS}: python -B {BUILD_PRODUCT}'
        return read_product(found, head)
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
    return f'{index}. {step.id} ({LABELS.get(step.id, "-")}) [{kind}, about {step.minutes} min] {state}'


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
        out(f"{index}. {step.id} ({LABELS.get(step.id, '-')}): {step.title}")
        out(f"   {'attended (foreground)' if step.attended else 'unattended; start only when the quiet gate passes'}; "
            f'about {step.minutes} min (draft)')
        if not step.built:
            out(f'   not built yet: {step.missing}')
        for command in step.commands:
            out(f"   {'planned: ' if not step.built else ''}{command}")
        for kind, argument in step.prerequisites:
            shown = ' '.join(argument) if isinstance(argument, tuple) else argument
            out(f"   needs: {kind}{'' if argument is None else ' ' + shown}")
        for condition in step.run_time:
            out(f'   at run time: {condition}')
        out(f"   sources: {', '.join(step.sources)}")


def main(argv=None):
    parser = argparse.ArgumentParser(description='C9 owner-machine runbook (runs nothing).')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--list', action='store_true', help='print every step with its commands and prerequisites')
    mode.add_argument('--dry-run', action='store_true', help='check the prerequisites, one line per step')
    parser.add_argument('--build', type=Path, help='prepared build directory (default: the newest route-B build '
                        f'without user stages under {BUILDS}, else the newest); U5 finds its stage build itself')
    parser.add_argument('--check-quiet', action='store_true', help='also run the quiet-machine gate once (info only)')
    args = parser.parse_args(argv)
    if args.list:
        listing()
        return 0
    build = args.build or newest_build(route=SURFACE, stages=False) or newest_build()
    head = checkout_head()
    print(f"checkout HEAD {head[:12] if head else 'unreadable (builds are not compared with it)'}")
    code = dry_run(check=lambda kind, argument: evaluate(kind, argument, build, head=head))
    if args.check_quiet:
        quiet, gate = quiet_machine()
        print(f"quiet-machine gate: {'quiet' if quiet else 'not quiet'}"
              f"{'' if gate is None else ' (' + ', '.join(gate.get('reasons', []) or ['no reasons']) + ')'}")
    return code


if __name__ == '__main__':
    sys.exit(main())
