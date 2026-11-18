"""Evidence records and shared log reading for the C9 measurement tools (PHS.65, PHS.66). Standard library only.

DRAFT (the integrator's choice under PHS.65; the owner may change it). Every measurement tool in this folder writes
one JSON record, format `magic600-c9-measure-record-v1`, with:

- `measurement`: what was measured, and `evidence_kind`: "performance" or "windows-directx", never "source";
- `synthetic`: true when any input is a synthetic fixture (its header says so, the tool got --synthetic, or the input
  lies under tests/). A synthetic record has the verdict "synthetic" and `passes_bar` false, whatever its numbers;
- `build_identity`: the one identity the inputs name, or null (then the record never passes);
- `inputs` (and `outputs`): role, file name and SHA-256 of each file; names only, never paths;
- `tools`: the Python version and the SHA-256 of the tool and of each helper module it loads;
- `command`: the tool's arguments, with paths made repository-relative or reduced to their name;
- `bar` (the bar, its rule and its source), `result`, `verdict`, `reasons`, `passes_bar` and `drafts`.

A record states what its input logs show. It does not by itself establish the machine, the conditions or the build
that produced them (PHS.64): those come from the run and its own records. A record is never a claim that a phase
passed (PHS.66). Verdicts: met, not-met, insufficient (too few observations or runs), invalid (no valid run, or
mixed builds), synthetic, no-bar. An unreadable run is kept as an invalid run with its reason code. Exit codes of
the tools: 0 met or no-bar, 1 the record could not be written, 2 usage, 3 not-met or insufficient, 4 invalid,
5 synthetic.
"""
from __future__ import annotations

import hashlib
import json
import platform
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE.parent))
from b412_summary import RunError, integer, require, sanitize  # noqa: E402

FORMAT = 'magic600-c9-measure-record-v1'
KINDS = ('performance', 'windows-directx')
VERDICTS = ('met', 'not-met', 'insufficient', 'invalid', 'synthetic', 'no-bar')
EXIT_CODES = {'met': 0, 'no-bar': 0, 'not-met': 3, 'insufficient': 3, 'invalid': 4, 'synthetic': 5}


def sha256(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def file_entry(role, path):
    path = Path(path)
    return {'role': role, 'name': path.name, 'bytes': path.stat().st_size, 'sha256': sha256(path)}


def tool_versions(tool, helpers=(), extra=None):
    """The Python version and the digests of the tool and of the helpers it loads (paths relative to tools/perf)."""
    perf = HERE.parent
    versions = {'python': platform.python_version(), 'implementation': platform.python_implementation(),
                'tool': {'name': Path(tool).name, 'sha256': sha256(tool)},
                'helpers': {name: sha256(perf / name) for name in ('c9/evidence.py', 'b412_summary.py', *helpers)}}
    versions.update(extra or {})
    return versions


def portable_argument(value):
    """A command argument without private paths: repository-relative inside the checkout, else the bare name."""
    text = str(value)
    if '/' not in text and '\\' not in text:
        return text
    try:
        resolved = Path(text).resolve()
    except (OSError, ValueError):
        return '<path>'
    try:
        return resolved.relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return 'outside-repository:' + resolved.name


def portable_command(tool, argv):
    return ['python', '-B', portable_argument(tool), *map(portable_argument, argv)]


def is_synthetic(paths, declared, flag=False):
    """True when the caller says so, any input declares itself synthetic, or any input lies under tests/."""
    tests = (ROOT / 'tests').resolve()
    under_tests = any(tests == Path(path).resolve() or tests in Path(path).resolve().parents for path in paths)
    return bool(flag) or any(value is True for value in declared) or under_tests


def make_record(measurement, kind, *, synthetic, build_identity, inputs, tools, command, bar, result, verdict,
                reasons=(), drafts=(), outputs=None):
    """The record, sanitized. A synthetic record never passes; neither does one without a build identity."""
    if kind not in KINDS:
        raise ValueError(f'evidence kind {kind!r} is not one of {KINDS}; source evidence is never recorded here')
    if verdict not in VERDICTS:
        raise ValueError(f'unknown verdict {verdict!r}')
    reasons = sorted(set(reasons))
    if synthetic:
        verdict = 'synthetic'
        reasons = sorted(set(reasons) | {'synthetic-input'})
    elif verdict == 'met' and not (isinstance(build_identity, str) and build_identity.strip()):
        verdict = 'invalid'
        reasons = sorted(set(reasons) | {'missing-build-identity'})
    record = {'format': FORMAT, 'measurement': measurement, 'evidence_kind': kind, 'synthetic': bool(synthetic),
              'created_utc': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
              'build_identity': build_identity, 'inputs': list(inputs), 'tools': tools, 'command': list(command),
              'bar': bar, 'result': result, 'verdict': verdict, 'reasons': reasons,
              'passes_bar': verdict == 'met', 'drafts': list(drafts)}
    if outputs is not None:
        record['outputs'] = list(outputs)
    return sanitize(record)


def write_record(path, record):
    """Write a new record; an existing file is never replaced."""
    with Path(path).open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(record, indent=2, ensure_ascii=False, allow_nan=False) + '\n')


def read_jsonl(path):
    """The rows of a JSONL log: one object per nonblank line, or RunError."""
    rows = []
    try:
        with Path(path).open(encoding='utf-8', newline='') as stream:
            for line in stream:
                if line.strip():
                    rows.append(json.loads(line))
    except (ValueError, UnicodeError):
        raise RunError('bad-json') from None
    except OSError:
        raise RunError('missing-file') from None
    require(bool(rows) and all(isinstance(row, dict) for row in rows), 'bad-row')
    return rows


def read_header(rows, expected_format):
    """The log's one `run` row, which must come first: format, run_id, build_identity, qpc_frequency, synthetic."""
    head = rows[0]
    require(head.get('t') == 'run' and sum(row.get('t') == 'run' for row in rows) == 1, 'bad-header')
    require(head.get('format') == expected_format, 'bad-format')
    require(isinstance(head.get('run_id'), str) and bool(head['run_id'].strip()), 'bad-header')
    require(isinstance(head.get('build_identity'), str) and bool(head['build_identity'].strip()),
            'missing-build-identity')
    require(integer(head.get('qpc_frequency')) and head['qpc_frequency'] > 0, 'bad-header')
    require(type(head.get('synthetic')) is bool, 'bad-header')
    return head


def report(record, stream=None):
    """The verdict in one line, for the console."""
    stream = stream or sys.stdout
    reasons = f" ({', '.join(record['reasons'])})" if record['reasons'] else ''
    print(f"{record['measurement']}: verdict {record['verdict']}{reasons}; passes bar: {record['passes_bar']}",
          file=stream)


def exit_code(record):
    return EXIT_CODES[record['verdict']]


def under(path, root=ROOT):
    """True when path lies inside root."""
    try:
        Path(path).resolve().relative_to(Path(root).resolve())
        return True
    except ValueError:
        return False
