"""Batch exact tutorial exports and offline Chromium captures.

    python3 -I -B make_figures.py --list figures.json --out figures [--vendor DIR] [--check] [--prune]

Headless research pictures only; no native-rendering or performance claim.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LIST_FORMAT = 'magic600-tutorial-figures/1'
MANIFEST_FORMAT = 'magic600-tutorial-figures-manifest/1'
ID = re.compile(r'[a-z0-9][a-z0-9-]{0,63}')
PARAMETERS = ('view', 'proj', 'emph', 'ghost', 'cap', 'w', 'h')
REQUIRED = {'id', 'record', 'step', 'when', 'view', 'proj', 'emph'}
OPTIONAL = {'cap', 'ghost', 'w', 'h'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'), object_pairs_hook=no_duplicates)


def write_json(path, document):
    Path(path).write_text(json.dumps(document, indent=1, allow_nan=False) + '\n', encoding='utf-8')


def record_directory(name):
    if (not isinstance(name, str) or not name or '\\' in name or '\x00' in name
            or Path(name).is_absolute() or PureWindowsPath(name).drive or '..' in name.split('/')):
        raise ValueError('record must be a repository-relative run directory')
    path = (ROOT / name).resolve()
    if not path.is_relative_to(ROOT) or not (path / 'run.json').resolve().is_relative_to(ROOT):
        raise ValueError('record is outside the repository')
    return path


def load_list(path):
    document = read_json(path)
    if (not isinstance(document, dict) or set(document) != {'format', 'figures'}
            or document['format'] != LIST_FORMAT or not isinstance(document['figures'], list)
            or not document['figures']):
        raise ValueError(f'expected {LIST_FORMAT} with a non-empty figures list')
    figures, names, records = document['figures'], set(), set()
    for entry in figures:
        if (not isinstance(entry, dict) or not REQUIRED <= set(entry)
                or not set(entry) <= REQUIRED | OPTIONAL):
            raise ValueError('invalid figure fields')
        name = entry['id']
        if not isinstance(name, str) or not ID.fullmatch(name) or name in names:
            raise ValueError(f'invalid or duplicate figure id: {name}')
        names.add(name)
        if type(entry['step']) is not int or entry['step'] < 0 or entry['when'] not in ('before', 'after'):
            raise ValueError(f'{name}: invalid step or when')
        for key, choices in (('view', ('local', 'global')), ('proj', ('persp', 'stereo')),
                             ('emph', ('moved', 'off', 'none')), ('ghost', ('near', 'all', 'none'))):
            if key in entry and entry[key] not in choices:
                raise ValueError(f'{name}: invalid {key}')
        if 'cap' in entry and (type(entry['cap']) is not int or not 0 <= entry['cap'] < 600):
            raise ValueError(f'{name}: invalid cap (0..599)')
        for key in ('w', 'h'):
            if key in entry and (type(entry[key]) is not int or not 1 <= entry[key] <= 8192):
                raise ValueError(f'{name}: invalid {key} (1..8192)')
        records.add(record_directory(entry['record']))
    # Reject reserved seeds before any simulator replay or capture, including when
    # a later figure names the reserved record. The exporter checks them again.
    for record in sorted(records):
        run = read_json(record / 'run.json')
        if not isinstance(run, dict) or not isinstance(run.get('start'), dict):
            raise ValueError('record has no valid start')
        start = run['start']
        seed = start.get('seed')
        if type(seed) is int and 9011 <= seed <= 9073:
            raise ValueError('start seeds 9011..9073 are reserved and refused')
    return figures


def export_state(record, step, when, out):
    subprocess.run([sys.executable, '-I', '-B', str(HERE / 'export_tutorial.py'), str(record),
                    '--step', str(step), '--when', when, '--out', str(out)], check=True)


def capture(data, shots, out, vendor):
    command = ['node', str(HERE / 'capture_tutorial.mjs'), '--data', str(data),
               '--list', str(shots), '--out', str(out)]
    if vendor is not None:
        command.extend(['--vendor', str(vendor)])
    subprocess.run(command, check=True)


def build(figures, temporary, vendor):
    data, pictures = temporary / 'data', temporary / 'capture'
    states, shots = {}, []
    for entry in figures:
        key = (record_directory(entry['record']), entry['step'], entry['when'])
        if key not in states:
            state = f'state-{len(states)}'
            directory = data / state
            export_state(*key, directory)
            raw = (directory / 'tutorial.json').read_bytes()
            tutorial = json.loads(raw, object_pairs_hook=no_duplicates)
            states[key] = (state, tutorial, sha(raw))
        state, _, _ = states[key]
        shots.append({'name': entry['id'], 'state': state,
                      **{key: entry[key] for key in PARAMETERS if key in entry}})
    shot_list = temporary / 'shots.json'
    write_json(shot_list, shots)
    capture(data, shot_list, pictures, vendor)
    captured = read_json(pictures / 'manifest.json')
    if captured.get('format') != 'magic600-tutorial-capture/1':
        raise ValueError('invalid capture manifest format')
    results = {shot['name']: shot for shot in captured['shots']}
    if len(results) != len(figures) or set(results) != {entry['id'] for entry in figures}:
        raise ValueError('capture manifest figures do not match the list')
    rows = []
    for entry in figures:
        name = entry['id']
        state, tutorial, tutorial_sha = states[(record_directory(entry['record']), entry['step'], entry['when'])]
        result = results[name]
        digest = sha((pictures / f'{name}.png').read_bytes())
        if (result['state'] != state or result['header_digest'] != tutorial['digest']
                or result['tutorial_sha256'] != tutorial_sha or result['png_sha256'] != digest):
            raise ValueError(f'{name}: capture provenance or PNG hash mismatch')
        rows.append({'id': name, 'entry': entry, 'tutorial': tutorial,
                     'capture': {key: result[key] for key in (*PARAMETERS, 'header_digest', 'tutorial_sha256')},
                     'png': f'png/{name}.png', 'png_sha256': digest})
    manifest = {'format': MANIFEST_FORMAT, 'versions': captured['versions'],
                'sources': captured['sources'], 'figures': rows}
    # No temporary directories, vendor paths or output paths enter the artifact.
    return manifest, pictures


def differences(manifest, baseline, out):
    if baseline.get('format') != MANIFEST_FORMAT or not isinstance(baseline.get('figures'), list):
        raise ValueError('invalid figures manifest')
    previous = {row['id']: row for row in baseline['figures']}
    current = {row['id']: row for row in manifest['figures']}
    different = set(previous) ^ set(current)
    for name in previous.keys() & current.keys():
        old, new = previous[name], current[name]
        if old['png_sha256'] != new['png_sha256'] or old['tutorial']['digest'] != new['tutorial']['digest']:
            different.add(name)
        try:
            if sha((out / 'png' / f'{name}.png').read_bytes()) != old['png_sha256']:
                different.add(name)
        except FileNotFoundError:
            different.add(name)
    return sorted(different)


def make_figures(list_path, out, vendor=None, check=False, prune=False):
    if check and prune:
        raise ValueError('--check cannot be combined with --prune')
    figures = load_list(list_path)
    out = Path(out)
    baseline = read_json(out / 'manifest.json') if check else None
    expected = {f'{entry["id"]}.png' for entry in figures}
    png = out / 'png'
    extras = sorted((path for path in png.iterdir() if path.name not in expected and path.is_file()),
                    key=lambda path: path.name) if png.exists() else []
    if extras:
        print('Unlisted files in png/: ' + ', '.join(path.name for path in extras), file=sys.stderr)
    with tempfile.TemporaryDirectory(prefix='magic600-figures-') as directory:
        manifest, pictures = build(figures, Path(directory), vendor)
        if check:
            changed = differences(manifest, baseline, out)
            if changed:
                print('Figures differ: ' + ', '.join(changed), file=sys.stderr)
                return 1
            print(f'check: {len(figures)} PNG hashes and state digests match the manifest')
            return 0
        png.mkdir(parents=True, exist_ok=True)
        for entry in figures:
            name = f'{entry["id"]}.png'
            shutil.copyfile(pictures / name, png / name)
        if prune:
            for path in extras:
                path.unlink()
        write_json(out / 'manifest.json', manifest)
    print(f'wrote {len(figures)} figures and manifest.json')
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--list', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--vendor', type=Path)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--prune', action='store_true')
    args = parser.parse_args(argv)
    try:
        return make_figures(args.list, args.out, args.vendor, args.check, args.prune)
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        print(f'tutorial figures: FAIL ({exc})', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
