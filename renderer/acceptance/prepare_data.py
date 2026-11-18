"""Prepare W-J's data directory for the C9 acceptance runs, or check a prepared one.

Prepare runs W-J's own exporter, work/experiments/renderer-wj/prepare_wj.py, in place and unchanged
with the pinned engine Python and -B. It reads work/experiments and research/jumbling and writes only
the output directory, which must lie under work/build/ or work/loop-memory/. A record of the inputs
(c9-prepare.json) is written next to the exports, and the result is checked before it is accepted.

--check uses the standard library only. For each menu it compares every exported array with its
header and with the immutable fixture research/jumbling/fixtures/wj-<menu>.json, checks motion.json
against the fixture and moving.i32, and checks both lattice control directories against their headers.
The core repeats its own checks when it loads the data; this check lets a dry run refuse stale data.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import math
import os
import platform
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
WJ = ROOT / 'work/experiments/renderer-wj'
FIXTURES = ROOT / 'research/jumbling/fixtures'
ENGINE_PYTHON = 'tools/.venv/engine/Scripts/python.exe'
ALLOWED_ROOTS = ('work/build', 'work/loop-memory')
DEFAULT_OUT = 'work/loop-memory/c9-acceptance/wj-data'
MENUS = ('S4', 'I_a', 'I_b')
STAGES = ('start', 'mid', 'end', 'sweep-before', 'sweep-after')
LATTICE = ('lattice-start', 'lattice-retained')
RENDER_FORMAT = 'magic600-jumbling-render/1'
MOTION_FORMAT = 'magic600-wj-motion/1'
ARRAYS = ('pose_index.i32', 'poses.f32', 'pose_lattice.i32')
LATTICE_FILES = ('labels.u32', 'destination.u32')
SLOTS = 259800
ITEM_SIZE = {'<i4': 4, '<u4': 4, '<f4': 4}
INPUTS = ('work/experiments/renderer-wj/prepare_wj.py', 'work/experiments/renderer-wj/fixture_data.py',
          'work/experiments/renderer-wj/lattice_wj.py', 'work/experiments/renderer-sb/reference_geometry.py',
          'research/jumbling/fixtures/wj.py')
RECORD = 'c9-prepare.json'


class Refused(Exception):
    pass


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as stream:
        for block in iter(lambda: stream.read(1 << 20), b''):
            digest.update(block)
    return digest.hexdigest()


def read_json(path):
    try:
        with open(path, encoding='utf-8') as stream:
            return json.load(stream)
    except (OSError, ValueError) as error:
        raise Refused(f'{path}: unreadable JSON ({type(error).__name__})') from error


def output_root(text):
    """Resolve an output directory and refuse one outside work/build/ or work/loop-memory/."""
    path = Path(text)
    path = (path if path.is_absolute() else ROOT / path).resolve()
    for allowed in ALLOWED_ROOTS:
        base = (ROOT / allowed).resolve()
        if path != base and path.is_relative_to(base):
            return path
    raise Refused(f'{path}: data must lie under {" or ".join(ALLOWED_ROOTS)} of this checkout')


def fixture_path(menu):
    return FIXTURES / f'wj-{menu}.json'


def reference(fixture, stage):
    return fixture['stages'][stage] if stage in fixture['stages'] else fixture['sweep'][stage.split('-')[1]]


def check_files(directory, header, names):
    """Each named file exists, has the header's digest and the size its shape and type give."""
    files = header.get('files')
    if not isinstance(files, dict) or set(files) != set(names):
        raise Refused(f'{directory}: header files {sorted(files or ())} differ from {sorted(names)}')
    for name in names:
        entry = files[name]
        path = directory / name
        if not path.is_file():
            raise Refused(f'{path}: missing')
        if (not isinstance(entry, dict) or entry.get('dtype') not in ITEM_SIZE
                or not isinstance(entry.get('shape'), list)
                or not all(isinstance(n, int) and n >= 0 for n in entry['shape'])):
            raise Refused(f'{path}: header entry malformed')
        size = math.prod(entry['shape']) * ITEM_SIZE[entry['dtype']]
        if path.stat().st_size != size:
            raise Refused(f'{path}: {path.stat().st_size} bytes, header gives {size}')
        if sha256_file(path) != entry.get('sha256'):
            raise Refused(f'{path}: SHA-256 differs from its header')
    return files


def check_menu(data, menu):
    path = fixture_path(menu)
    fixture = read_json(path)
    if fixture.get('menu_name') != menu:
        raise Refused(f'{path}: menu_name is not {menu}')
    for stage in STAGES:
        directory = data / menu / stage
        header = read_json(directory / 'header.json')
        expected = reference(fixture, stage)
        fixture_block = header.get('fixture') or {}
        if (header.get('format') != RENDER_FORMAT or header.get('dimension') != 4
                or fixture_block.get('menu') != menu or fixture_block.get('stage') != stage
                or header.get('digest') != expected['digest']):
            raise Refused(f'{directory}: header format, menu, stage or J1 digest differs from the fixture')
        files = check_files(directory, header, ARRAYS)
        for name in ARRAYS:
            if files[name] != expected['arrays'][name]:
                raise Refused(f'{directory / name}: differs from the fixture reference')
    motion = read_json(data / menu / 'motion.json')
    moving = data / menu / 'moving.i32'
    if not moving.is_file():
        raise Refused(f'{moving}: missing')
    sweep = fixture['sweep']
    if (motion.get('format') != MOTION_FORMAT or motion.get('dimension') != 4
            or motion.get('fixture_sha256') != sha256_file(path)
            or motion.get('moving_sha256') != sha256_file(moving)
            or motion.get('moving_sha256') != sweep['moved_sha256']
            or moving.stat().st_size != 4 * sweep['moved']):
        raise Refused(f'{data / menu}: motion.json or moving.i32 differs from the fixture sweep')


def check_lattice(data):
    for name in LATTICE:
        directory = data / name
        header = read_json(directory / 'header.json')
        if header.get('format') != RENDER_FORMAT or (header.get('fixture') or {}).get('stage') != name:
            raise Refused(f'{directory}: header format or stage differs')
        files = check_files(directory, header, ARRAYS + LATTICE_FILES)
        for file in LATTICE_FILES:
            if files[file]['shape'] != [SLOTS] or files[file]['dtype'] != '<u4':
                raise Refused(f'{directory / file}: not {SLOTS} little-endian u32 values')


def check(data, menus):
    if not data.is_dir():
        raise Refused(f'{data}: not a directory')
    check_lattice(data)
    for menu in menus:
        check_menu(data, menu)


def experiments_listing():
    """Tracked changes and untracked or ignored files under work/experiments, to prove nothing is written there."""
    result = subprocess.run(['git', '-C', str(ROOT), 'status', '--porcelain', '--ignored', '--untracked-files=all',
                             '--', 'work/experiments', 'research/jumbling'],
                            capture_output=True, text=True, encoding='utf-8', check=False)
    if result.returncode:
        raise Refused('git status failed')
    return sorted(result.stdout.splitlines())


def engine_python():
    """The pinned engine Python of this checkout, or of the main checkout when this is a worktree."""
    result = subprocess.run(['git', '-C', str(ROOT), 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            capture_output=True, text=True, encoding='utf-8', check=False)
    roots = [ROOT] + ([Path(result.stdout.strip()).parent] if result.returncode == 0 and result.stdout.strip() else [])
    for root in roots:
        if (root / ENGINE_PYTHON).is_file():
            return root / ENGINE_PYTHON
    raise Refused(f'{ENGINE_PYTHON}: engine Python not found in this checkout or the main checkout')


def prepare(out, menus, python):
    python = python or engine_python()
    if not python.is_file():
        raise Refused(f'{python}: engine Python not found')
    exporter = WJ / 'prepare_wj.py'
    before = experiments_listing()
    inputs = {name: sha256_file(ROOT / name) for name in INPUTS}
    inputs.update({f'research/jumbling/fixtures/wj-{m}.json': sha256_file(fixture_path(m)) for m in menus})
    out.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
    command = [str(python), '-B', str(exporter), '--out', str(out), '--menu', *menus]
    print('running:', subprocess.list2cmdline(command), flush=True)
    result = subprocess.run(command, cwd=str(ROOT), env=environment, check=False)
    if result.returncode:
        raise Refused(f'prepare_wj.py exited {result.returncode}')
    if experiments_listing() != before:
        raise Refused('work/experiments or research/jumbling changed during the export')
    if any(sha256_file(ROOT / name) != digest for name, digest in inputs.items()):
        raise Refused('an exporter input changed during the export')
    check(out, menus)
    version = subprocess.run([str(python), '-B', '-c', 'import sys, numpy; print(sys.version.split()[0], numpy.__version__)'],
                             capture_output=True, text=True, check=False).stdout.split()
    record = {'format': 'magic600-c9-wj-data-v1',
              'created_utc': datetime.datetime.now(datetime.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
              'command': ['<engine python>', '-B', 'work/experiments/renderer-wj/prepare_wj.py', '--out', '<data>',
                          '--menu', *menus],
              'menus': list(menus), 'inputs': inputs,
              'python': version[0] if version else None, 'numpy': version[1] if len(version) > 1 else None,
              'platform': platform.platform(terse=True)}
    (out / RECORD).write_bytes((json.dumps(record, indent=2) + '\n').encode('utf-8'))
    print(f'W-J data: pass ({", ".join(menus)}) in {out}', flush=True)


def main(arguments=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--check', action='store_true', help='check prepared data; standard library only')
    parser.add_argument('--data', '--out', dest='data', default=DEFAULT_OUT,
                        help=f'data directory under work/build or work/loop-memory (default {DEFAULT_OUT})')
    parser.add_argument('--menu', choices=MENUS, nargs='+', default=list(MENUS))
    parser.add_argument('--python', type=Path, help='the pinned engine Python (default: found in the checkout)')
    options = parser.parse_args(arguments)
    menus = list(dict.fromkeys(options.menu))
    try:
        data = output_root(options.data)
        if options.check:
            check(data, menus)
            print(f'W-J data check: pass ({", ".join(menus)}) in {data}')
        else:
            prepare(data, menus, options.python)
    except Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
