"""Batch exact tutorial exports and offline Chromium captures.

    python3 -I -B make_figures.py --list figures.json --out figures [--vendor DIR] [--check] [--prune]

Headless research pictures only; no native-rendering or performance claim.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
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
PARAMETERS = ('view', 'proj', 'emph', 'ghost', 'cap', 'w', 'h', 'cs', 'ss', 'fit')
REQUIRED = {'id', 'record', 'step', 'when', 'view', 'proj', 'emph'}
OPTIONAL = {'cap', 'ghost', 'w', 'h', 'cs', 'ss', 'fit'}


def recipe_exporter():
    # Load once, only when a recipe is present. No runner or search is executed.
    if '_recipe_exporter' not in globals():
        spec = importlib.util.spec_from_file_location('tutorial_recipe_exporter', HERE / 'export_tutorial.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        globals()['_recipe_exporter'] = module
    return globals()['_recipe_exporter']


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
    def render(value, depth=0):
        if isinstance(value, dict) and value:
            items = [f'{json.dumps(key)}: {render(item, depth + 1)}' for key, item in value.items()]
            opening, closing = '{', '}'
        elif isinstance(value, list) and any(isinstance(item, (dict, list)) for item in value):
            items = [render(item, depth + 1) for item in value]
            opening, closing = '[', ']'
        else:
            return json.dumps(value, allow_nan=False)
        return (opening + '\n' + ',\n'.join(' ' * (depth + 1) + item for item in items)
                + '\n' + ' ' * depth + closing)

    # Preserve the standard encoder's key/scalar semantics before formatting.
    document = json.loads(json.dumps(document, allow_nan=False))
    Path(path).write_text(render(document) + '\n', encoding='utf-8')


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
    if (not isinstance(document, dict) or not {'format', 'figures'} <= set(document)
            or not set(document) <= {'format', 'figures', 'sheets'}
            or document['format'] != LIST_FORMAT or not isinstance(document['figures'], list)
            or not document['figures']):
        raise ValueError(f'expected {LIST_FORMAT} with a non-empty figures list')
    figures, names, records = document['figures'], set(), set()
    for entry in figures:
        required = (REQUIRED - {'record', 'step', 'when'}) | {'state'} if isinstance(entry, dict) and 'state' in entry else REQUIRED
        if (not isinstance(entry, dict) or not required <= set(entry)
                or not set(entry) <= required | OPTIONAL):
            raise ValueError('invalid figure fields')
        name = entry['id']
        if not isinstance(name, str) or not ID.fullmatch(name) or name in names:
            raise ValueError(f'invalid or duplicate figure id: {name}')
        names.add(name)
        if 'state' in entry:
            recipe_exporter().prepare_recipe(entry['state'], root=ROOT)
        else:
            if type(entry['step']) is not int or entry['step'] < 0 or entry['when'] not in ('before', 'after'):
                raise ValueError(f'{name}: invalid step or when')
            records.add(record_directory(entry['record']))
        for key, choices in (('view', ('local', 'global')), ('proj', ('persp', 'stereo')),
                             ('emph', ('moved', 'off', 'none')), ('ghost', ('near', 'all', 'none')),
                             ('fit', ('all', 'cap', 'core'))):
            if key in entry and entry[key] not in choices:
                raise ValueError(f'{name}: invalid {key}')
        fit = entry.get('fit', 'all')
        ghost = entry.get('ghost', 'near' if entry['view'] == 'local' else 'all')
        if fit == 'cap' and entry['view'] != 'local':
            raise ValueError(f'{name}: fit=cap requires view=local')
        if fit == 'core' and (ghost != 'none' or entry['emph'] == 'none'):
            raise ValueError(f'{name}: fit=core requires ghost=none and emph other than none')
        if 'cap' in entry and (type(entry['cap']) is not int or not 0 <= entry['cap'] < 600):
            raise ValueError(f'{name}: invalid cap (0..599)')
        for key in ('w', 'h'):
            if key in entry and (type(entry[key]) is not int or not 1 <= entry[key] <= 8192):
                raise ValueError(f'{name}: invalid {key} (1..8192)')
        for key in ('cs', 'ss'):
            # Facet (cell) and sticker shrink, 0.40 to 1.00 in steps of 0.01 as the page's sliders.
            if key in entry and (type(entry[key]) not in (int, float) or not 0.4 <= entry[key] <= 1
                                 or abs(entry[key] * 100 - round(entry[key] * 100)) > 1e-9):
                raise ValueError(f'{name}: invalid {key} (0.40..1.00)')
    sheets_list(document.get('sheets', []), figures)
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


def sheets_list(sheets, figures):
    if not isinstance(sheets, list):
        raise ValueError('invalid sheets list')
    panels = {entry['id'] for entry in figures}
    names = set(panels)
    for sheet in sheets:
        if not isinstance(sheet, dict) or set(sheet) != {'id', 'panels', 'labels', 'columns'}:
            raise ValueError('invalid sheet fields')
        name = sheet['id']
        if not isinstance(name, str) or not ID.fullmatch(name) or name in names:
            raise ValueError(f'invalid or duplicate sheet id: {name}')
        names.add(name)
        if (not isinstance(sheet['panels'], list) or not sheet['panels']
                or any(not isinstance(p, str) or p not in panels for p in sheet['panels'])):
            raise ValueError(f'{name}: invalid sheet panels (must name figures)')
        if (not isinstance(sheet['labels'], list) or len(sheet['labels']) != len(sheet['panels'])
                or any(not isinstance(label, str) for label in sheet['labels'])):
            raise ValueError(f'{name}: invalid sheet labels')
        if type(sheet['columns']) is not int or not 1 <= sheet['columns'] <= len(sheet['panels']):
            raise ValueError(f'{name}: invalid sheet columns')
    return sheets


def export_state(record, step, when, out):
    subprocess.run([sys.executable, '-I', '-B', str(HERE / 'export_tutorial.py'), str(record),
                    '--step', str(step), '--when', when, '--out', str(out)], check=True)


def capture(data, shots, out, vendor):
    command = ['node', str(HERE / 'capture_tutorial.mjs'), '--data', str(data),
               '--list', str(shots), '--out', str(out)]
    if vendor is not None:
        command.extend(['--vendor', str(vendor)])
    subprocess.run(command, check=True)


def export_key(entry):
    if 'state' in entry:
        return json.dumps(entry['state'], sort_keys=True, separators=(',', ':'))
    return record_directory(entry['record']), entry['step'], entry['when']


def compose_sheet(sheet, pictures):
    """Scalable Pillow default font, no PNG metadata, white RGB pixels, fixed layout."""
    import PIL
    from PIL import Image, ImageDraw, ImageFont

    panels = []
    for name in sheet['panels']:
        with Image.open(pictures / f'{name}.png') as source:
            panel = Image.new('RGB', source.size, 'white')
            rgba = source.convert('RGBA')
            panel.paste(rgba, (0, 0), rgba)
            panels.append(panel)
    width, height = max(p.width for p in panels), max(p.height for p in panels)
    margin, gap = 12, 12
    columns, rows = sheet['columns'], (len(panels) + sheet['columns'] - 1) // sheet['columns']
    canvas_width = margin * 2 + columns * width + (columns - 1) * gap
    size = max(24, round(canvas_width / 60))
    font = ImageFont.load_default(size=size)
    measure = ImageDraw.Draw(Image.new('RGB', (1, 1)))
    line_height = measure.textbbox((0, 0), 'Ag', font=font)[3] + 6
    labels = []
    for label in sheet['labels']:
        lines = []
        for paragraph in label.split('\n'):
            line = ''
            for char in paragraph:
                if line and measure.textlength(line + char, font=font) > width:
                    lines.append(line)
                    line = ''
                line += char
            lines.append(line)
        labels.append(lines)
    label_height = max(map(len, labels)) * line_height + gap
    canvas = Image.new('RGB', (canvas_width,
                             margin * 2 + rows * (height + label_height) + (rows - 1) * gap), 'white')
    draw = ImageDraw.Draw(canvas)
    for i, (panel, lines) in enumerate(zip(panels, labels)):
        x = margin + i % columns * (width + gap)
        y = margin + i // columns * (height + label_height + gap)
        canvas.paste(panel, (x + (width - panel.width) // 2, y + (height - panel.height) // 2))
        for j, line in enumerate(lines):
            offset = int((width - draw.textlength(line, font=font)) // 2)
            draw.text((x + offset, y + height + gap + j * line_height), line, font=font, fill='black')
    output = pictures / f'{sheet["id"]}.png'
    canvas.save(output, format='PNG', optimize=False, compress_level=9)
    return {'id': sheet['id'], 'entry': sheet,
            'panels': [{'id': name, 'png_sha256': sha((pictures / f'{name}.png').read_bytes())}
                       for name in sheet['panels']],
            'composition': {'pillow': PIL.__version__, 'font': 'Pillow default (scalable)', 'size': size,
                            'margin': margin, 'gap': gap},
            'png': f'png/{sheet["id"]}.png', 'png_sha256': sha(output.read_bytes())}


def build(figures, temporary, vendor, sheets=()):
    data, pictures = temporary / 'data', temporary / 'capture'
    states, shots = {}, []
    session = recipe_exporter().ReplaySession() if any('state' in entry for entry in figures) else None
    for entry in figures:
        key = export_key(entry)
        if key not in states:
            state = f'state-{len(states)}'
            directory = data / state
            if 'state' in entry:
                recipe_exporter().export_recipe(entry['state'], directory, session)
            else:
                export_state(*key, directory)
            raw = (directory / 'tutorial.json').read_bytes()
            tutorial = json.loads(raw, object_pairs_hook=no_duplicates)
            states[key] = (state, tutorial, sha(raw))
        state, _, _ = states[key]
        shots.append({'name': entry['id'], 'state': state,
                      'cs': 0.76, 'ss': 0.82, **{key: entry[key] for key in PARAMETERS if key in entry}})
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
        state, tutorial, tutorial_sha = states[export_key(entry)]
        result = results[name]
        digest = sha((pictures / f'{name}.png').read_bytes())
        if (result['state'] != state or result['header_digest'] != tutorial['digest']
                or result['tutorial_sha256'] != tutorial_sha or result['png_sha256'] != digest):
            raise ValueError(f'{name}: capture provenance or PNG hash mismatch')
        rows.append({'id': name, 'entry': entry, 'tutorial': tutorial,
                     'capture': {key: result[key] for key in (*PARAMETERS, 'framing', 'header_digest', 'tutorial_sha256')},
                     'png': f'png/{name}.png', 'png_sha256': digest})
    manifest = {'format': MANIFEST_FORMAT, 'versions': captured['versions'],
                'sources': captured['sources'], 'figures': rows}
    if sheets:
        manifest['sheets'] = [compose_sheet(sheet, pictures) for sheet in sheets]
    # No temporary directories, vendor paths or output paths enter the artifact.
    return manifest, pictures


def differences(manifest, baseline, out):
    if baseline.get('format') != MANIFEST_FORMAT or not isinstance(baseline.get('figures'), list):
        raise ValueError('invalid figures manifest')
    previous = {row['id']: row for kind in ('figures', 'sheets') for row in baseline.get(kind, [])}
    current = {row['id']: row for kind in ('figures', 'sheets') for row in manifest.get(kind, [])}
    different = set(previous) ^ set(current)
    for name in previous.keys() & current.keys():
        old, new = previous[name], current[name]
        if (old['png_sha256'] != new['png_sha256'] or old.get('entry') != new.get('entry')
                or old.get('capture') != new.get('capture')
                or old.get('tutorial') != new.get('tutorial') or old.get('panels') != new.get('panels')
                or old.get('composition') != new.get('composition')):
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
    sheets = sheets_list(read_json(list_path).get('sheets', []), figures)
    out = Path(out)
    baseline = read_json(out / 'manifest.json') if check else None
    expected = {f'{entry["id"]}.png' for entry in [*figures, *sheets]}
    png = out / 'png'
    extras = sorted((path for path in png.iterdir() if path.name not in expected and path.is_file()),
                    key=lambda path: path.name) if png.exists() else []
    if extras:
        print('Unlisted files in png/: ' + ', '.join(path.name for path in extras), file=sys.stderr)
    with tempfile.TemporaryDirectory(prefix='magic600-figures-') as directory:
        manifest, pictures = build(figures, Path(directory), vendor, sheets)
        if check:
            changed = differences(manifest, baseline, out)
            if changed:
                print('Figures differ: ' + ', '.join(changed), file=sys.stderr)
                return 1
            print(f'check: {len(figures)} figures and {len(sheets)} sheets match the manifest')
            return 0
        png.mkdir(parents=True, exist_ok=True)
        for entry in [*figures, *sheets]:
            name = f'{entry["id"]}.png'
            shutil.copyfile(pictures / name, png / name)
        if prune:
            for path in extras:
                path.unlink()
        write_json(out / 'manifest.json', manifest)
    print(f'wrote {len(figures)} figures, {len(sheets)} sheets and manifest.json')
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
    except (OSError, ValueError, KeyError, TypeError, AssertionError, EOFError, subprocess.CalledProcessError) as exc:
        print(f'tutorial figures: FAIL ({exc})', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
