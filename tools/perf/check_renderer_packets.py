"""Lint the stage 2.4 renderer packets and their shared gate numbers (headless)."""
from __future__ import annotations

import argparse
import importlib.util
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / 'docs/progress/1.0/packets/renderer'
# Loading the judge and wrapper is read only, including Python's import caches.
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import renderer_gate  # noqa: E402

spec = importlib.util.spec_from_file_location('renderer_packet_contract', ROOT / 'tools/agents/codex_review.py')
codex_review = importlib.util.module_from_spec(spec)
# The wrapper reads tempfile.gettempdir() at import, which probes candidate directories by creating
# files. A preset value is returned as is, so the checker stays read only; only parse_contract is used.
_tempdir, tempfile.tempdir = tempfile.tempdir, str(ROOT)
try:
    spec.loader.exec_module(codex_review)
finally:
    tempfile.tempdir = _tempdir

HEADINGS = ('## 1. Goal and acceptance', '## 2. Actual problem and reproduction',
            '## 3. Environment and versions', '## 4. Necessary source and evidence',
            '## 5. Attempts so far', '## 6. Constraints and owned files', '## 7. Required return format')
README_TEXT = ('PresentMon 2.6.0.0', '7 GB', '10 s', 'off by default')  # the gate numbers: GATE_REQUIRED
# The gate as stated in the README and the two ruling packets. Every fps, ms and cold-run count in
# these files must be the gate's own value, so a weakened copy fails instead of passing beside it.
GATE_FILES = ('README.md', 'E-2.4-04-', 'E-2.4-05-')
GATE_REQUIRED = (('an average of at least 30 fps', r'\ban average of at least 30 fps\b'),
                 ('at most 33.3 ms', r'\bat most 33\.3 ms\b'), ('three valid cold runs', r'\bthree valid cold runs\b'),
                 ('180 s', r'(?<![\d.])180 s\b'), ('259,800', r'(?<![\d,])259,800\b'))
GATE_ALLOWED = (('fps', r'([\d.]+) fps\b', {'30'}), ('ms', r'([\d.]+) ms\b', {'33.3'}),
                ('cold runs', r'\b(\w+) (?:valid )?cold runs?\b', {'three'}))
JUDGE_CONSTANTS = {'FPS_MIN': 30, 'P99_MAX_MS': 33.3, 'INTERVAL_S': 180, 'WARMUP_S': 10, 'RUNS_MIN': 3}


def read_text(path, problems):
    try:
        return path.read_text(encoding='utf-8')
    except (OSError, UnicodeError):
        problems.append(f'{path}: cannot read UTF-8 file')
        return None


def gate_problems(path, text):
    """The gate statement is present and no other fps, ms or cold-run count appears beside it."""
    problems = [f'{path}: missing gate text: {label}' for label, pattern in GATE_REQUIRED
                if not re.search(pattern, text)]
    for unit, pattern, allowed in GATE_ALLOWED:
        for value in sorted({m.group(1) for m in re.finditer(pattern, text)} - allowed):
            problems.append(f'{path}: gate value {value} {unit} differs from the gate')
    # The direction matters as much as the number: every fps value is a lower bound, every ms value an upper bound.
    for unit, bound in (('fps', 'at least'), ('ms', 'at most')):
        for m in re.finditer(r'(?<![\w.])[\d.]+ ' + unit + r'\b', text):
            before = ' '.join(re.findall(r'[A-Za-z]+', text[max(0, m.start() - 40):m.start()])[-2:]).lower()
            if before != bound:
                problems.append(f'{path}: "{m.group(0)}" must read "{bound} {m.group(0)}"')
    return problems


def check(directory=DIRECTORY):
    """Return one line per problem, always naming the affected file or directory."""
    directory = Path(directory)
    if not directory.is_dir():
        return [f'{directory}: missing directory']
    problems = []
    try:
        packets = sorted(directory.glob('E-2.4-*.md'))
    except OSError:
        return [f'{directory}: cannot list directory']
    for number in range(6):
        prefix = f'E-2.4-{number:02d}-'
        matching = [path for path in packets if path.name.startswith(prefix)]
        if len(matching) != 1:
            names = ', '.join(path.name for path in matching) or 'none'
            problems.append(f'{directory / (prefix + "*.md")}: expected exactly one packet (found {names})')
    for path in packets:
        text = read_text(path, problems)
        if text is None:
            continue
        lines = [line.rstrip() for line in text.splitlines()]
        if tuple(line for line in lines if line.startswith('## ')) != HEADINGS:
            problems.append(f'{path}: expected the seven section headings in order')
        if HEADINGS[-1] in lines:
            section7 = '\n'.join(lines[lines.index(HEADINGS[-1]) + 1:])
            if 'schemas/review-result.schema.json' not in section7 and not re.search(r'\bREADME\b', section7):
                problems.append(f'{path}: section 7 must name schemas/review-result.schema.json or README')
        if '--kind implement' in text:
            try:
                contract = codex_review.parse_contract(text)
            except codex_review.Refused as error:
                problems.append(f'{path}: {error}')
            else:
                if any(not glob.startswith('work/experiments/renderer-') for glob in contract['allowed_files']):
                    problems.append(f'{path}: allowed_files must stay under work/experiments/renderer-')
        elif codex_review.CONTRACT_RE.search(text):
            problems.append(f'{path}: a contract block is only allowed with --kind implement')
        if ('--gate' in text and ('--model gpt-6-astra' not in text or '--effort ultra' not in text
                                  or '--speed fast' in text)):
            problems.append(f'{path}: --gate requires --model gpt-6-astra --effort ultra and forbids --speed fast')
        if any(path.name.startswith(f'E-2.4-{number:02d}-') for number in range(1, 6)):
            if 'renderer_gate.py' not in text:
                problems.append(f'{path}: missing renderer_gate.py')
            if not any('NVIDIA' in line and re.search(r'\boff\b|Non-goals', line) for line in lines):
                problems.append(f'{path}: missing NVIDIA restriction on a line with off or Non-goals')
        if path.name.startswith(GATE_FILES[1:]):
            problems.extend(gate_problems(path, text))

    readme = directory / 'README.md'
    text = read_text(readme, problems)
    if text is not None:
        missing = [value for value in README_TEXT if value not in text]
        if missing:
            problems.append(f'{readme}: missing required text: {", ".join(missing)}')
        problems.extend(gate_problems(readme, text))
    for name, expected in JUDGE_CONSTANTS.items():
        if getattr(renderer_gate, name) != expected:
            problems.append(f'tools/perf/renderer_gate.py: {name} must be {expected}')
    return problems


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, nargs='?', default=DIRECTORY)
    args = parser.parse_args(argv)
    problems = check(args.directory)
    for problem in problems:
        print(problem)
    if not problems:
        print(f'renderer packets: ok ({len(list(args.directory.glob("E-2.4-*.md")))} packets)')
    return int(bool(problems))


if __name__ == '__main__':
    sys.exit(main())
