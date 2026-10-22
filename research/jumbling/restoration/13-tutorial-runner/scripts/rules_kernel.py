"""Propose one literal R22D-25 step with D19, without changing the runner's state.

The kernel runs in a child with max_steps=1. Its clock, exhaustion and integrity
failures remain distinct; only the runner replays a proposed word. Headless
research, catalogue r22d25-cat/1, with no coverage claim.
"""
import hashlib
import json
import math
from pathlib import Path
import re
import sys
import time

import runner_api as api

ROOT = Path(__file__).resolve().parents[5]
KERNEL = Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d25_search.py')
D19 = KERNEL.with_name('r22d19_rule.py')
SCRIPTS = [Path(__file__).resolve().relative_to(ROOT), KERNEL,
           KERNEL.with_name('r22d25_state.py'), D19,
           KERNEL.parent.parent / 'eb6.py', KERNEL.parent.parent / 'screen.py',
           *[Path('research/jumbling/theory') / name
             for name in ('blocks.py', 'classes.py', 'e3_class.py', 'rim.py')]]
CATALOGUE = 'r22d25-cat/1'
LIMITS = {'direct': 244, 'S': 244, 'T': 720, 'ST': 964}
FAMILIES = ('F1', 'F2', 'F3', 'd19', 'S', 'ST', 'T1')
STEP_FIELDS = ('family', 'kind', 'target', 'params', 'high_exact',
               'off_before', 'off_after', 'heights_after')
PATH_KEYS = frozenset({'unit', 'file', 'word', 'table', 'library', 'out'})


def _number(value):
    return (type(value) is int and value >= 0) or (type(value) is float and math.isfinite(value) and value >= 0)


def _token(value):
    """Only fixed words and exception type names may enter public diagnostics."""
    return value if isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9_]{1,64}', value) else None


def _safe(value, path_keys=False):
    """Validate copied metadata, leaving the kernel's parameters verbatim."""
    if isinstance(value, dict):
        return all(isinstance(k, str) and _safe(k) and k not in api.FORBIDDEN_KEYS
                   and (not path_keys or k not in PATH_KEYS) and _safe(v, path_keys)
                   for k, v in value.items())
    if isinstance(value, list):
        return all(_safe(v, path_keys) for v in value)
    if isinstance(value, str):
        return not (value.startswith(('/', '\\')) or re.match(r'^[A-Za-z]:', value))
    return value is None or type(value) in (bool, int) or (type(value) is float and math.isfinite(value))


def _ledger_status(doc):
    rows = doc.get('ledger')
    if not isinstance(rows, dict) or any(not isinstance(row, dict) or not isinstance(row.get('status'), str)
                                         for row in rows.values()):
        return None
    result = {name: row['status'] for name, row in rows.items()}
    return result if _safe(result, path_keys=True) else None


def _stop_family(rows):
    stopped = None
    for family, row in rows.items():
        cursor = row.get('cursor')
        if (row['status'] == 'budget' and isinstance(cursor, dict)
                and not {'phase', 'next_unit', 'max_steps', 'middle_digest'}.intersection(cursor)):
            stopped = family
    return stopped


def _require(condition):
    if not condition:
        raise ValueError('invalid kernel record')


def _check_record(doc, snapshot):
    _require(doc.get('format') == 'r22d25-run/1' and doc.get('catalogue') == CATALOGUE)
    start = doc.get('start')
    _require(isinstance(start, dict) and start.get('digest') == snapshot['digest']
             and type(start.get('off')) is int and start['off'] == snapshot['off'])
    _require(isinstance(doc.get('steps'), list) and len(doc['steps']) <= 1)
    _require(_ledger_status(doc) is not None and set(FAMILIES) <= doc['ledger'].keys())
    _require(doc.get('limits') == LIMITS and all(type(v) is int for v in doc['limits'].values()))
    _require(_number(doc.get('seconds')))


def _read_step(doc, directory, snapshot):
    _require(len(doc['steps']) == 1 and isinstance(doc['steps'][0], dict))
    row = doc['steps'][0]
    _require(type(row.get('index')) is int and row['index'] == 0 and row.get('word') == 'words/00000.json')
    _require(row.get('family') in ('F1', 'F2', 'F3', 'd19', 'ST', 'T1'))
    _require(row.get('kind') in ('landing', 'lowering') and type(row.get('high_exact')) is bool)
    target = row.get('target')
    _require(isinstance(target, list) and target and all(type(p) is int for p in target)
             and target == sorted(target))
    _require(type(row.get('off_before')) is int and row['off_before'] == snapshot['off']
             and row.get('input_digest') == snapshot['digest'])
    _require(type(row.get('off_after')) is int and row['off_after'] >= 0)
    _require(isinstance(row.get('output_digest'), str)
             and re.fullmatch(r'[0-9a-f]{64}', row['output_digest']))
    _require(type(row.get('letters')) is int and row['letters'] > 0 and _number(row.get('seconds')))
    heights = row.get('heights_after')
    _require(isinstance(heights, dict) and all(isinstance(h, str) and h.isdigit()
             and type(n) is int and n >= 0 for h, n in heights.items()))
    _require(isinstance(row.get('params'), dict) and _safe({k: row[k] for k in STEP_FIELDS}))
    if row['kind'] == 'landing':
        _require(row['off_after'] < row['off_before'])
    else:
        _require(heights.get('6', 0) < snapshot['heights'].get('6', 0))
    word = json.loads((directory / 'words/00000.json').read_bytes())
    _require(isinstance(word, dict) and set(word) == {'word', 'input_digest', 'output_digest'})
    _require(word['input_digest'] == row['input_digest'] and word['output_digest'] == row['output_digest'])
    _require(isinstance(word['word'], list) and len(word['word']) == row['letters'])
    for letter in word['word']:
        _require(isinstance(letter, list) and len(letter) == 2)
        api.ia16_code(*letter)
    rule = 'r22d25/' + row['family']
    if row['family'] == 'ST':
        s, t = row['params'].get('S_length'), row['params'].get('T_length')
        _require(type(s) is int and type(t) is int and s >= 1 and t >= 1 and s + t == row['letters'])
        tree = api.flat(rule, [('S', s), ('T', t)])
    else:
        tree = api.single(rule, row['letters'])
    return row, word, tree


def next_step(ctx):
    snapshot = ctx.snapshot()
    # A previous call (even an interrupted one) can never supply this call's evidence.
    suffix = 0
    while True:
        directory = ctx.work_dir / ('kernel' if suffix == 0 else f'kernel-{suffix}')
        try:
            directory.mkdir(exist_ok=False)
            break
        except FileExistsError:
            suffix += 1
    relative = directory.relative_to(ctx.work_dir.parents[1]).as_posix()
    details = dict(kernel_result=None, kernel_reason=None, ledger_status=None,
                   record_sha256=None, dir=relative)
    kernel_seconds, stopped, call_seconds = None, None, 0.0

    def end(result, reason):
        return api.End(result, reason, details=details, attempts=[{
            'rule': 'r22d25', 'catalogue': CATALOGUE, 'outcome': result, 'reason': reason,
            'timed_out': reason in ('kernel_wall_clock', 'kernel_clock'), 'seconds': call_seconds,
            'kernel_seconds': kernel_seconds, 'ledger_status': details['ledger_status'],
            'stop_family': stopped}])

    seconds = ctx.child_clock('kernel_seconds')
    wall = ctx.wall('kernel_seconds')
    if wall is not None:
        wall += ctx.budgets['kernel_wall_margin']
    try:
        d19_sha256 = hashlib.sha256((ROOT / D19).read_bytes()).hexdigest()
    except FileNotFoundError:
        d19_sha256 = None
    argv = [sys.executable, '-I', '-B', ROOT / KERNEL, 'run', '--table', ctx.table,
            '--out', directory, '--cache', ctx.cache, '--seconds', seconds,
            '--d19', ROOT / D19, '--max-steps', '1']
    started = time.monotonic()
    try:
        child = ctx.execute(argv, ctx.work_dir.name + '-' + directory.name, wall)
    except OSError:
        call_seconds = time.monotonic() - started
        return end('error', 'kernel_exit')
    call_seconds = child.seconds
    if child.timeout:
        return end('timeout', 'kernel_wall_clock')
    if child.cancelled:
        return end('error', 'cancelled')
    try:
        raw = (directory / 'run.json').read_bytes()
        details['record_sha256'] = hashlib.sha256(raw).hexdigest()
        doc = json.loads(raw)
    except (OSError, ValueError):
        return end('error', 'kernel_exit' if child.rc else 'kernel_record_missing')
    if isinstance(doc, dict):
        details.update(kernel_result=_token(doc.get('result')), kernel_reason=_token(doc.get('reason')),
                       ledger_status=_ledger_status(doc))
        kernel_seconds = doc.get('seconds') if _number(doc.get('seconds')) else None
    if child.rc:
        return end('error', 'kernel_error' if details['kernel_result'] == 'error' else 'kernel_exit')
    try:
        _require(isinstance(doc, dict))
        _check_record(doc, snapshot)
        rows, steps, result = doc['ledger'], doc['steps'], doc.get('result')
        stopped = _stop_family(rows)
        max_steps = any(isinstance(row.get('cursor'), dict) and 'max_steps' in row['cursor']
                        for row in rows.values())
        if result == 'lattice' or (result == 'budget' and steps):
            _require(result == 'lattice' or max_steps)
            row, word, tree = _read_step(doc, directory, snapshot)
            _require(result != 'lattice' or row['off_after'] == 0)
            kernel = dict(catalogue=doc['catalogue'], limits=doc['limits'], result=result,
                          seconds=seconds, max_steps=1, d19_sha256=d19_sha256,
                          record_sha256=details['record_sha256'], dir=relative,
                          step_seconds=row['seconds'], call_seconds=call_seconds)
            return api.Proposal('high', None, 'r22d25/' + row['family'], word,
                                'kernel_' + row['kind'], 'r22d25', tree=tree,
                                fields={**{k: row[k] for k in STEP_FIELDS}, 'kernel': kernel})
        if result == 'budget':
            _require(not steps and not max_steps and stopped is not None)
            return end('budget', 'kernel_clock')
        if result == 'stuck':
            _require(not steps and all(row['status'] == 'exhausted' for row in rows.values()))
            details['catalogue'] = CATALOGUE
            return end('stuck', 'kernel_exhausted')
        if result == 'unsupported':
            reason = {'d19_missing': 'd19_missing', 'height_above_6': 'invariant_height_above_6'}
            return end('error', reason.get(doc.get('reason'), 'kernel_unsupported'))
        return end('error', 'kernel_record_invalid')
    except (OSError, ValueError, KeyError, TypeError):
        return end('error', 'kernel_record_invalid')
