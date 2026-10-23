"""Write CATALOGUE.md: chapter and appendix figures, then the unembedded pictures of figures.json.

Usage: python3 -I -B research/jumbling/tutorial/figures/catalogue.py [--check]

Saved figures must match their manifest hashes and picture recipes. Other undrawn
figures need a reason in MISSING; --check also detects a stale catalogue.
"""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

F = Path(__file__).resolve().parent
T = F.parent
# Match embeds before validating their captions or destinations: a broken path
# must never make an image disappear from the catalogue's checks.
EMBED = re.compile(r'''!\[(.*?)\]\(((?:[^()"']|"[^"]*"|'[^']*'|\([^()]*\))*)\)''', re.S)
FIGURE_LABEL = re.compile(r'^(Figure (?:\d+(?:\.\d+)*|[A-Z](?:\.\d+)+)):')

# Reasons for figures that are not drawn yet; each entry is removed when its figure lands.
MISSING = {}


def embedded_entries(svg_names=None):
    """Validate every inline image; a redraw may supply its drawable SVG names."""
    rows, errors = [], []
    for chapter in [*sorted(T.glob('0*.md')), *sorted(T.glob('A-*.md'))]:
        text = chapter.read_text(encoding='utf-8')
        for match in EMBED.finditer(text):
            alt, destination = match.groups()
            # Markdown permits angle brackets and an optional title.
            link = re.fullmatch(r'\s*(?:<([^<>]*)>|(\S+?))(?:\s+(?:"[^"]*"|\x27[^\x27]*\x27|\([^()]*\)))?\s*', destination)
            target = (link[1] if link[1] is not None else link[2]) if link else destination.strip()
            label = FIGURE_LABEL.match(alt)
            location = f'{chapter.name}:{text.count(chr(10), 0, match.start()) + 1}: {target}'
            problems = []
            if not label:
                problems.append('alt text must start with Figure <number>:')
            path = Path(target)
            kind = path.parent.name
            if (not link or kind not in ('png', 'svg')
                    or path.parent.as_posix() != 'figures/' + kind
                    or path.suffix != '.' + kind
                    or target != path.as_posix()
                    or not (T / path).resolve().is_relative_to((F / kind).resolve())):
                problems.append('target must be inside figures/png/ or figures/svg/ with the matching extension')
            elif kind == 'svg' and svg_names is not None:
                if path.stem not in svg_names:
                    problems.append('target has no diagram renderer')
            elif not (T / path).is_file():
                problems.append('target does not exist')
            if problems:
                errors.append(location + ': ' + '; '.join(problems))
                continue
            number, name = label[1], path.stem
            rows.append((chapter.name, number, kind, name))
    if errors:
        raise ValueError('\n'.join(errors))
    return rows


def entries():
    listed = json.loads((F / 'figures.json').read_text())
    pictures = {e['id']: e for e in listed.get('figures', [])}
    pictures.update({s['id']: dict(s, sheet=True) for s in listed.get('sheets', [])})
    rows = embedded_entries()
    embedded = {row[3] for row in rows}
    for name in sorted(set(pictures) - embedded):
        rows.append(('', '', 'png', name))
    return rows, pictures


def source(kind, name, pictures):
    if kind == 'svg':
        return '`diagrams.py`'
    entry = pictures.get(name)
    if entry is None:
        return 'not in `figures.json`'
    if entry.get('sheet'):
        return 'sheet of ' + ', '.join(f'`{p}`' for p in entry['panels'])
    if 'state' in entry:
        return 'state recipe'
    record = entry['record'].split('/runs/')[-1].removesuffix('/run')
    return f'`{record}` step {entry["step"]} {entry["when"]}, {entry["view"]} {entry["proj"]}'


def manifest(path):
    return json.loads(path.read_text()) if path.is_file() else {}


def figure_status(kind, name, pictures, png_manifest, svg_manifest):
    path = F / kind / f'{name}.{kind}'
    if kind == 'png':
        entry = pictures.get(name)
        recorded = png_manifest.get(name)
        expected = {k: v for k, v in entry.items() if k != 'sheet'} if entry else None
        if (not path.is_file() or recorded is None or expected is None
                or recorded.get('entry') != expected
                or hashlib.sha256(path.read_bytes()).hexdigest() != recorded.get('png_sha256')):
            return 'stale'
        if entry.get('sheet'):
            # A sheet is drawn only when every panel is drawn and the sheet was composed from the panel bytes
            # that the manifest records now.
            composed = [(p.get('id'), p.get('png_sha256')) for p in recorded.get('panels', [])]
            current = [(p, png_manifest.get(p, {}).get('png_sha256')) for p in entry['panels']]
            if composed != current or any(figure_status('png', p, pictures, png_manifest, svg_manifest) != 'drawn'
                                          for p in entry['panels']):
                return 'stale'
        return 'drawn'
    # Old diagram manifests without output hashes retain their existence rule.
    if any('sha256' in row.get('output', {}) for row in svg_manifest.values()):
        recorded = svg_manifest.get(name, {}).get('output', {})
        if (not path.is_file()
                or hashlib.sha256(path.read_bytes()).hexdigest() != recorded.get('sha256')):
            return 'stale'
    elif not path.is_file():
        return 'missing: ' + MISSING.get(name, '(no reason recorded)')
    return 'drawn'


def render():
    rows, pictures = entries()
    saved = manifest(F / 'manifest.json')
    png_manifest = {row['id']: row for kind in ('figures', 'sheets') for row in saved.get(kind, [])}
    svg_manifest = manifest(F / 'svg/manifest.json').get('figures', {})
    lines = ['# Figure catalogue', '',
             'Generated by `catalogue.py`; do not edit by hand. Embedded figures come first, chapters then appendices, then',
             'the pictures of `figures.json` that no chapter or appendix embeds yet.',
             '', '| Chapter | Figure | File | Source | Status |', '|---|---|---|---|---|']
    drawn = missing = stale = 0
    invalid = []
    for chapter, number, kind, name in rows:
        status = figure_status(kind, name, pictures, png_manifest, svg_manifest)
        if status == 'drawn':
            drawn += 1
        elif status == 'stale':
            stale += 1
            if chapter:
                invalid.append(name)
        else:
            missing += 1
            if name not in MISSING:
                invalid.append(name)
        lines.append(f'| {chapter} | {number} | `{kind}/{name}.{kind}` | {source(kind, name, pictures)} | {status} |')
    lines += ['', f'{drawn} drawn, {stale} stale, {missing} missing.', '']
    return '\n'.join(lines), invalid


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args(argv)
    try:
        text, unexplained = render()
    except ValueError as error:
        print('Invalid image embed: ' + str(error))
        return 1
    target = F / 'CATALOGUE.md'
    if args.check:
        stale = not target.exists() or target.read_text() != text
        for name in unexplained:
            print('stale or missing figure:', name)
        if stale:
            print('CATALOGUE.md is out of date')
        return 1 if stale or unexplained else 0
    target.write_text(text)
    print(text.splitlines()[-1])
    return 0


if __name__ == '__main__':
    sys.exit(main())
