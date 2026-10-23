"""Runner acceptance (RUN-3; runner-run/3, plan part I0). All generated evidence and caches live in temporary storage.

python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_runner.py [Class[.test] ...]
Set RUNNER_SLOW=1 to include the depth-20 run through the lattice phase. The tests that call
check_run.py on a record say so where they call independent_check.
"""
import ast
import copy
from contextlib import nullcontext, redirect_stdout
import gzip
import hashlib
import importlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
spec = importlib.util.spec_from_file_location('tutorial_runner', HERE / 'runner.py')
R = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = R
spec.loader.exec_module(R)
api = R.api

BUDGETS = {'bulk_steps', 'bulk_step_seconds', 'bulk_seconds', 'tail_steps', 'high_steps', 'rule_seconds',
           'greedy_seconds', 'cert_seconds', 'height_seconds', 'height_cpu', 'kernel_seconds', 'pair_shards',
           'fallback_max_length', 'par'}
STEP_KEYS = {'index', 'phase', 'subphase', 'rule', 'word', 'letters', 'encoding', 'word_sha256', 'tree',
             'standing', 'before', 'after', 'seconds'}
# Steps come in the fixed phase order; the tail and its fallback share one place.
PHASE_RANK = {'bulk': 0, 'height': 1, 'high': 2, 'tail': 3, 'fallback': 3, 'lattice': 4}
STOPS = ('budget', 'timeout', 'error')
SEED_REASON = 'seed: no small-support word found in single letters and bounded short products'
# R22D-36 (test_r22d25.py:39-42), from solved: 5,800 off, heights {2: 5799, 6: 1}.
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]


def entry(name, snap):
    """The entry test of a phase or height sub-phase (plan section 0), from a recorded snapshot."""
    heights = {int(h): n for h, n in snap['heights'].items()}
    top, n4, n6 = max(heights, default=0), heights.get(4, 0), heights.get(6, 0)
    return {'r22d75': top <= 6 and n4 + n6 > 0, 'r22d81': top <= 6 and n6 > 0, 'h4': top <= 4 and n4 > 0,
            'high': top <= 6 and n4 + n6 > 0, 'tail': snap['off'] > 0, 'lattice': snap['off'] == 0}[name]


def outer(results):
    """The height phase's result from its sub-phases' results, in the fixed order: the first
    run-ending result, else the result of the last sub-phase that applied."""
    for result in ('error', 'budget', 'timeout', 'lattice'):
        if result in results:
            return result
    applicable = [r for r in results if r != 'not_applicable']
    return applicable[-1] if applicable else 'not_applicable'


def expected_verdict(record):
    """The acceptance verdict in the plan's order (section 0), restated from the record alone."""
    block, result, steps = record['acceptance'], record['result'], record['steps']
    final = steps[-1]['after']['digest'] if steps else record['phases']['bulk']['end']['digest']
    ended = list(record['phases']['height']['sub'].values()) + [record['phases']['high']]
    at_end = [a for unit in (record['stuck'] or {}).get('units', []) for a in unit['attempts']]
    at_end += [a for data in ended if data['end'] is not None and data['end']['digest'] == final
               for a in data.get('attempts', [])]
    at_end += record['phases']['lattice'].get('attempts', [])
    if result == 'error':
        return 'error'
    if record['config']['component'] is not None:
        return 'component'
    if any(s['standing'] == 'certificate' for s in steps):
        return 'stuck'
    if any(s['standing'] == 'fallback' for s in steps) or block['probe']:
        return 'coverage_failure'
    if result == 'budget' or (result == 'stuck' and any(a['timed_out'] for a in at_end)):
        return 'budget'
    return result


def check_record(test, out, record):
    """Check a runner-run/3 record and its files, including every committed checkpoint.

    A step tree is checked with runner_api.check_tree and by its sha over the uncompressed
    bytes; that it expands to exactly the replayed word is check_run.py's job."""
    test.assertEqual(set(record) - {'resumes'}, {'format', 'menu', 'start', 'config', 'phases', 'steps', 'stuck',
                                  'lattice', 'result', 'error', 'acceptance', 'totals'})
    test.assertEqual(record['format'], 'runner-run/3')
    test.assertEqual(record['menu'], 'I_a')
    test.assertIn(record['result'], ('solved', 'stuck', 'budget', 'error'))
    if record['result'] == 'error':
        test.assertIsInstance(record['error'], str)
    else:
        test.assertIsNone(record['error'])
    config = record['config']
    test.assertEqual(set(config), {'literal_only', 'mode', 'component', 'bulk_order', 'phase_order',
        'subphase_order', 'tail_rules', 'budgets', 'clock_sentinel', 'kernel', 'lattice', 'literal_limit',
        'seeds', 'acceptance_config', 'source', 'scripts', 'code', 'no_lattice'})
    test.assertIs(config['literal_only'], True)
    test.assertIn(config['mode'], api.MODES)
    acceptance = config['mode'] == 'acceptance'
    test.assertTrue(config['component'] is None or (isinstance(config['component'], str) and config['component']))
    test.assertEqual(config['bulk_order'], R.BULK_ORDER)
    test.assertNotIn('reopen', config['bulk_order'])
    test.assertEqual(config['phase_order'], list(api.PHASE_ORDER))
    test.assertEqual(config['subphase_order'], {'height': ['r22d75', 'r22d81', 'h4']})
    test.assertLessEqual(set(config['tail_rules']), set(R.TAIL_RULES))
    budgets = config['budgets']
    test.assertEqual(set(budgets), BUDGETS)
    for name in R.CLOCKS:
        # Acceptance mode lifts every clock (plan section 0); development records its values.
        if acceptance:
            test.assertIsNone(budgets[name], name)
        elif budgets[name] is not None:
            test.assertGreaterEqual(budgets[name], 0, name)
    if not acceptance:
        test.assertGreater(budgets['cert_seconds'], 0)
    test.assertEqual(config['clock_sentinel'], api.CLOCK_SENTINEL if acceptance else None)
    test.assertLessEqual({'seconds', 'wall_margin', 'max_steps'}, set(config['kernel']))
    test.assertEqual((config['kernel']['seconds'], config['kernel']['max_steps']), (budgets['kernel_seconds'], 1))
    test.assertEqual(set(config['lattice']), {'restorer'})
    test.assertIn(config['lattice']['restorer'], ('late', 'schedule11'))
    test.assertIs(type(config['literal_limit']), int)
    test.assertGreaterEqual(config['literal_limit'], 0)
    test.assertIn(config['seeds'], ('atlas', 'default'))
    frozen = config['acceptance_config']
    if acceptance:
        # The options acceptance mode checked against acceptance-config.json (plan revision 3e, E3).
        test.assertEqual(set(frozen), {'file', 'sha256', 'options'})
        test.assertEqual(set(frozen['options']), set(R.ACCEPTANCE_OPTIONS))
        test.assertEqual((config['literal_limit'], config['seeds'], config['tail_rules']),
                         tuple(frozen['options'][k] for k in ('literal_limit', 'seeds', 'tail_rules')))
    else:
        test.assertIsNone(frozen)
    source = config['source']
    test.assertEqual(set(source), {'commit', 'status_sha256', 'dirty'})
    if source['commit'] is not None:
        test.assertRegex(source['commit'], r'^[0-9a-f]{40}$')
    if source['status_sha256'] is not None:
        test.assertRegex(source['status_sha256'], r'^[0-9a-f]{64}$')
        test.assertIs(type(source['dirty']), bool)

    def digest(value):
        test.assertRegex(value, r'^[0-9a-f]{64}$')

    def snapshot(value):
        digest(value['digest'])
        test.assertGreaterEqual(value['off'], 0)
        test.assertEqual(sum(value['heights'].values()), value['off'])
        for key, count in value['heights'].items():
            test.assertRegex(key, r'^\d+$')
            test.assertGreater(count, 0)

    def relative(value):
        test.assertFalse(Path(value).is_absolute(), value)
        test.assertNotIn('..', Path(value).parts)
        test.assertNotIn('\\', value)
        test.assertTrue((out / value).is_file(), value)

    def no_program(value):
        if isinstance(value, dict):
            for key, child in value.items():
                test.assertNotIn(key, api.FORBIDDEN_KEYS)
                no_program(child)
        elif isinstance(value, list):
            for child in value:
                no_program(child)
        elif isinstance(value, str):
            test.assertFalse(value.startswith('/'), value)

    def tree_file(step):
        tree = step['tree']
        test.assertEqual(set(tree), {'file', 'sha256', 'size'})
        test.assertEqual(tree['file'], f'step-trees/{step["index"]:05d}.json.gz')
        relative(tree['file'])
        raw = (out / tree['file']).read_bytes()
        test.assertEqual(raw[4:8], b'\0' * 4)       # gzip with mtime 0
        data = gzip.decompress(raw)
        test.assertEqual(hashlib.sha256(data).hexdigest(), tree['sha256'], 'tree sha over the uncompressed bytes')
        try:
            size = api.check_tree(json.loads(data), step['letters'])
        except ValueError as exc:
            test.fail(f'step {step["index"]}: {exc}')
        test.assertEqual(size, tree['size'])

    def lattice_step(step, last):
        """A played lattice row (P3): its ia16 word file, tree, lattice snapshots and, last, the end table."""
        test.assertEqual((step['subphase'], step['encoding'], step['contract'], step['gate'], step['standing']),
                         (None, 'ia16', 'lattice_literal', 'player', 'rule'))
        test.assertTrue(step['rule'].startswith('lattice/'))
        test.assertEqual(step['word'], f'words/{step["index"]:05d}.ia16.gz')
        raw = (out / step['word']).read_bytes()
        test.assertEqual(raw[4:8], b'\0' * 4)       # gzip with mtime 0
        data = gzip.decompress(raw)
        test.assertEqual(hashlib.sha256(data).hexdigest(), step['word_sha256'], 'word sha over the uncompressed bytes')
        test.assertEqual(len(data), 2 * step['letters'])
        test.assertTrue(all(int.from_bytes(data[k:k + 2], 'little') < 36000 for k in range(0, len(data), 2)))
        tree_file(step)
        for value in (step['before'], step['after']):
            test.assertEqual(value, {'digest': value['digest'], 'off': 0, 'heights': {}})
        test.assertEqual(step['attempts'], [])
        test.assertGreaterEqual(step['seconds'], 0)
        if last:
            test.assertIs(step['labels_identity'], True)
            test.assertEqual(step['table'], f'tables/{step["index"]:05d}.json')
            relative(step['table'])
            test.assertEqual(R.read_json(out / step['table'])['digest'], step['after']['digest'])
        else:
            test.assertNotIn('table', step)

    no_program(record)
    snapshot(record['start'])
    if record['start']['kind'] == 'table':
        test.assertEqual(record['start']['file'], 'start-table.json')
        relative(record['start']['file'])
    else:
        test.assertEqual(record['start']['kind'], 'scramble')
        test.assertIsInstance(record['start']['k'], int)
        test.assertIsInstance(record['start']['seed'], int)
    test.assertIn(R.SCRIPTS[0].as_posix(), config['scripts'])
    for path, sha in config['scripts'].items():
        test.assertFalse(Path(path).is_absolute())
        digest(sha)
        test.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), sha)

    steps = record['steps']
    current, rank = record['start']['digest'], (0, 0)
    for index, step in enumerate(steps):
        test.assertEqual(step['index'], index)
        test.assertIn(step['phase'], PHASE_RANK)
        test.assertLessEqual(STEP_KEYS, set(step))
        if step['phase'] == 'height':
            test.assertIn(step['subphase'], api.SUBPHASES['height'])
            place = (PHASE_RANK['height'], api.SUBPHASES['height'].index(step['subphase']))
        else:
            test.assertIsNone(step['subphase'])
            place = (PHASE_RANK[step['phase']], 0)
        test.assertGreaterEqual(place, rank, 'phases and sub-phases come in the fixed order')
        rank = place
        test.assertEqual(step['before']['digest'], current)
        digest(step['after']['digest'])
        relative(step['word'])
        if step['phase'] == 'lattice':
            lattice_step(step, index == len(steps) - 1)
            current = step['after']['digest']
            continue
        test.assertEqual(step['encoding'], 'json')     # ia16 words come only with the played lattice (P3)
        word = R.read_json(out / step['word'])
        test.assertEqual(set(word), {'word', 'input_digest', 'output_digest'})
        test.assertEqual(word['input_digest'], current)
        test.assertEqual(word['output_digest'], step['after']['digest'])
        test.assertEqual(step['letters'], len(word['word']))
        test.assertEqual(step['word'].endswith('.json.gz'), step['letters'] > 100000)
        if step['letters'] > 100000:
            test.assertEqual((out / step['word']).read_bytes()[4:8], b'\0' * 4)
        R.literal(word)
        test.assertEqual(step['word_sha256'], R.word_sha256(word['word']))
        tree_file(step)
        current = word['output_digest']
        test.assertGreaterEqual(step['seconds'], 0)
        certificate = step.get('gate') == 'slp_apply'
        test.assertEqual(step.get('construction'), 'certificate' if certificate else None)
        if certificate:
            digest(step['certificate_sha256'])
        test.assertEqual(step['standing'], {'bulk': 'rule', 'fallback': 'fallback'}.get(
            step['phase'], 'certificate' if certificate and not acceptance else 'rule'))
        if step['phase'] == 'bulk':
            test.assertEqual(set(step), STEP_KEYS)
            test.assertIn(step['rule'], config['bulk_order'])
            test.assertLess(step['after']['N'], step['before']['N'])
            continue
        snapshot(step['before'])
        snapshot(step['after'])
        test.assertTrue(isinstance(step['contract'], str) and step['contract'])
        test.assertTrue(isinstance(step['gate'], str) and step['gate'])
        test.assertEqual(step['table'], f'tables/{index:05d}.json')
        relative(step['table'])
        test.assertEqual(R.read_json(out / step['table'])['digest'], step['after']['digest'])
        for attempt in step['attempts']:
            test.assertIs(type(attempt['timed_out']), bool)
        if step['phase'] not in ('tail', 'fallback'):
            continue
        relative(step['unit'])
        test.assertEqual(len(R.read_json(out / step['unit'])['pieces']), step['unit_size'])
        test.assertEqual(step['landed'], step['unit_size'])
        test.assertEqual(step['before']['off'] - step['after']['off'], step['landed'] + step['others_landed'])
        test.assertEqual(step['contract'], 'fallback' if step['phase'] == 'fallback' else 'strict_prime')
        test.assertIn(step['gate'], ('stepchk', 'slp_apply'))
        test.assertEqual(certificate, step.get('V_source') == 'certificate')
        if 'parts' in step:
            test.assertEqual(sum(n for _, n in step['parts']), step['letters'])
        closure = step['phase'] == 'tail' and (step['rule'] == 'close' or
                   (step['rule'] == 't1prime' and len(step['parts']) > 1))
        if closure:
            test.assertIn(step['V_source'], ('tclose2', 'certificate'))
            n = step['V_letters']
            test.assertGreaterEqual(n, 0)
            parts = step['parts']
            prefix = parts[0][1] if step['rule'] == 't1prime' else 0
            tail_parts = parts[1:] if step['rule'] == 't1prime' else parts
            test.assertEqual([name for name, _ in tail_parts], ['V', 'middle', 'V_inverse'])
            test.assertEqual(tail_parts[0][1], n)
            test.assertEqual(tail_parts[-1][1], n)
            v = word['word'][prefix:prefix + n]
            test.assertEqual(R.word_sha256(v), step['V_sha256'])
            if step['V_source'] == 'certificate':
                test.assertIs(step['N1'], True)
                test.assertIs(step['N2'], True)
                test.assertGreaterEqual(step['T_size'], 0)
                digest(step['T_digest'])
            else:
                test.assertTrue({'T_size', 'T_digest', 'N1', 'N2'}.isdisjoint(step))
        else:
            test.assertTrue({'V_source', 'V_letters', 'V_sha256', 'T_size', 'T_digest',
                             'N1', 'N2'}.isdisjoint(step))
            if step['rule'] == 't1prime':
                test.assertEqual(step['parts'], [['W', step['letters']]])
            else:
                test.assertNotIn('parts', step)

    def own(phase, sub=None):
        return [s for s in steps if s['phase'] == phase and s['subphase'] == sub]

    def ended(name, data, rows):
        """A finished phase or sub-phase: its result agrees with its own recorded snapshots."""
        test.assertIn(data['result'], api.RESULTS)
        snapshot(data['start'])
        snapshot(data['end'])
        test.assertEqual(data['steps'], len(rows), name)
        if rows:
            test.assertEqual(rows[0]['before']['digest'], data['start']['digest'])
            test.assertEqual(rows[-1]['after']['digest'], data['end']['digest'])
        else:
            test.assertEqual(data['start'], data['end'])
        test.assertEqual(entry(name, data['start']), data['result'] != 'not_applicable', name)
        if data['result'] == 'handoff':
            test.assertTrue(rows and not entry(name, data['end']), name)
        elif data['result'] == 'lattice':
            test.assertTrue(rows and data['end']['off'] == 0, name)
        elif data['result'] != 'not_applicable':
            test.assertTrue(entry(name, data['end']), name)
        test.assertTrue(data.get('reason') is None or isinstance(data['reason'], str))
        for attempt in data.get('attempts', []):
            test.assertIs(type(attempt['timed_out']), bool)

    phases = record['phases']
    test.assertEqual(list(phases), list(api.PHASE_ORDER))
    bulk = phases['bulk']
    test.assertEqual(set(bulk) - {'completed'}, {'result', 'steps', 'twists', 'end'})
    if 'completed' in bulk:
        test.assertIs(type(bulk['completed']), bool)
    snapshot(bulk['end'])
    relative(bulk['end']['table'])
    test.assertIn(bulk['result'], ('lattice', 'stuck', 'timeout', 'budget'))
    test.assertEqual(bulk['steps'], len(own('bulk')))
    test.assertEqual(bulk['twists'], sum(s['letters'] for s in own('bulk')))
    height = phases['height']
    test.assertEqual(set(height), {'result', 'steps', 'twists', 'start', 'end', 'sub'})
    test.assertEqual(list(height['sub']), list(api.SUBPHASES['height']))
    if height['sub']['r22d75']['result'] is not None and bulk['result'] in ('budget', 'timeout'):
        # A bulk ended by a budget or clock leaves a component run (plan, result table).
        test.assertIsNotNone(config['component'])
    results, closed = [], False
    for name, data in height['sub'].items():
        test.assertLessEqual({'result', 'steps', 'start', 'end'}, set(data))
        test.assertLessEqual(set(data), {'result', 'steps', 'twists', 'start', 'end', 'reason', 'details', 'attempts'})
        if data['result'] is None:
            closed = True
            continue
        test.assertFalse(closed, 'height sub-phases run in the fixed order and stop at a budget')
        ended(name, data, own('height', name))
        results.append(data['result'])
        closed = data['result'] in STOPS
    rows = [s for s in steps if s['phase'] == 'height']
    if height['result'] is not None:
        test.assertEqual(height['result'], outer(results))
        test.assertEqual(height['steps'], len(rows))
        test.assertEqual(height['twists'], sum(s['letters'] for s in rows))
        ran = [d for d in height['sub'].values() if d['result'] is not None]
        test.assertEqual((height['start'], height['end']), (ran[0]['start'], ran[-1]['end']))
    high = phases['high']
    test.assertLessEqual({'result', 'steps', 'twists', 'start', 'end'}, set(high))
    if high['result'] is not None:
        test.assertNotIn(height['result'], (None,) + STOPS)
        ended('high', high, own('high'))
        test.assertEqual(high['twists'], sum(s['letters'] for s in own('high')))
    tail = phases['tail']
    test.assertEqual(set(tail), {'result', 'steps', 'twists'})
    test.assertIn(tail['result'], (None, 'lattice', 'stuck', 'timeout', 'budget', 'not_applicable'))
    test.assertEqual(tail['steps'], sum(1 for s in steps if s['phase'] in ('tail', 'fallback')))
    if tail['result'] is not None:
        test.assertNotIn(high['result'], (None,) + STOPS)
    if tail['result'] == 'not_applicable':
        test.assertEqual(tail['steps'], 0)
    if tail['result'] in ('stuck', 'timeout'):
        test.assertEqual(record['stuck']['reason'], 'no_rule' if tail['result'] == 'stuck' else 'timeout')
        test.assertIn(record['result'], ('stuck' if tail['result'] == 'stuck' else 'budget', 'error'))
    lattice_phase, played = phases['lattice'], own('lattice')
    lattice_ends = ('stuck', 'budget', 'timeout', 'error')
    test.assertIn(lattice_phase['result'], (None, 'solved') + lattice_ends)
    test.assertEqual(lattice_phase['steps'], len(played))
    report = lattice_phase.get('report')
    if report is not None:
        # Played by P3's player, solved by identity labels, with no step when the restorer scheduled
        # nothing (plan revision 3e, E2); the composition path is not run.
        test.assertEqual(set(lattice_phase), {'result', 'steps', 'twists', 'report'})
        test.assertEqual((lattice_phase['result'], record['result']), ('solved', 'solved'))
        test.assertEqual(lattice_phase['twists'], sum(s['letters'] for s in played))
        test.assertIsNone(record['lattice'])
        test.assertEqual(set(report), {'file', 'sha256'})
        relative(report['file'])
        test.assertEqual(hashlib.sha256((out / report['file']).read_bytes()).hexdigest(), report['sha256'])
        test.assertEqual(R.read_json(out / report['file'])['player']['played']['total'], lattice_phase['twists'])
    elif 'reason' in lattice_phase:
        # A recorded End: nothing played, the run result follows the End.
        test.assertEqual(set(lattice_phase), {'result', 'steps', 'twists', 'reason', 'details', 'attempts'})
        test.assertIn(lattice_phase['result'], lattice_ends)
        test.assertEqual((played, lattice_phase['twists'], record['lattice']), ([], None, None))
        test.assertEqual(record['result'], {'stuck': 'stuck', 'error': 'error'}.get(lattice_phase['result'], 'budget'))
        for attempt in lattice_phase['attempts']:
            test.assertIs(type(attempt['timed_out']), bool)
    else:
        # Not entered, interrupted by a signal (attempts kept, P3-robust-01), or composed (stub only).
        test.assertLessEqual({'result', 'steps', 'twists'}, set(lattice_phase))
        test.assertLessEqual(set(lattice_phase), {'result', 'steps', 'twists', 'attempts'})
        test.assertEqual(played, [])

    totals = record['totals']
    test.assertEqual(set(totals), {'bulk_twists', 'height_twists', 'high_twists', 'tail_twists',
                                   'fallback_twists', 'lattice_twists', 'seconds'})
    for phase in ('bulk', 'height', 'high', 'tail', 'fallback'):
        test.assertEqual(totals[phase + '_twists'], sum(s['letters'] for s in steps if s['phase'] == phase))
    composed = record['lattice']['twists'] if record['lattice'] else None
    test.assertEqual(totals['lattice_twists'], lattice_phase['twists'] if report is not None else composed)
    stuck = record['stuck']
    if record['result'] == 'stuck':
        test.assertIsNotNone(stuck)
    if stuck is not None:
        test.assertLessEqual({'digest', 'off', 'heights', 'phase', 'reason', 'units'}, set(stuck))
        test.assertLessEqual(set(stuck), {'digest', 'off', 'heights', 'phase', 'reason', 'units', 'probe'})
        snapshot(stuck)
        test.assertEqual(stuck['digest'], current)
        test.assertIn(stuck['phase'], ('tail', 'lattice'))
        if stuck['phase'] == 'lattice':
            test.assertEqual((lattice_phase['result'], stuck['reason'], stuck['units']),
                             ('stuck', lattice_phase['reason'], []))
            test.assertNotIn('probe', stuck)
        timed_out = False
        for unit in stuck['units']:
            test.assertEqual(set(unit), {'unit_key', 'size', 'attempts'})
            for attempt in unit['attempts']:
                test.assertEqual(attempt['unit_key'], unit['unit_key'])
                test.assertTrue({'rule', 'outcome', 'seconds'} <= set(attempt))
                test.assertIs(type(attempt['timed_out']), bool)
                timed_out = timed_out or attempt['timed_out']
        if stuck['phase'] == 'tail':
            test.assertEqual(stuck['reason'], 'timeout' if timed_out else 'no_rule')
        if 'probe' in stuck:
            # Acceptance mode keeps a fallback word as evidence and never plays it.
            test.assertTrue(acceptance)
            probe = stuck['probe']
            test.assertEqual(set(probe), {'unit', 'unit_key', 'word', 'letters', 'input_digest', 'output_digest'})
            relative(probe['unit'])
            relative(probe['word'])
            word = R.literal(R.read_json(out / probe['word']), stuck['digest'], probe['output_digest'])
            test.assertEqual((probe['input_digest'], probe['letters']), (stuck['digest'], len(word['word'])))
    block = record['acceptance']
    test.assertEqual(set(block), {'verdict', 'certificate_steps', 'fallback_steps', 'probe', 'timed_out_attempts'})
    test.assertIn(block['probe'], (None, True, False))
    test.assertEqual(block['probe'] is True, stuck is not None and 'probe' in stuck)
    if block['verdict'] is not None:            # set when the run finishes
        ended_phases = list(height['sub'].values()) + [high, lattice_phase]
        attempts = [a for s in steps for a in s.get('attempts', [])]
        attempts += [a for unit in (stuck or {}).get('units', []) for a in unit['attempts']]
        attempts += [a for data in ended_phases for a in data.get('attempts', [])]
        test.assertEqual(block['certificate_steps'], sum(s['standing'] == 'certificate' for s in steps))
        test.assertEqual(block['fallback_steps'], sum(s['standing'] == 'fallback' for s in steps))
        test.assertEqual(block['timed_out_attempts'], sum(a['timed_out'] for a in attempts))
        test.assertEqual(block['verdict'], expected_verdict(record))
    if record['result'] == 'solved':
        test.assertTrue(current == R.SOLVED or record['lattice'] is not None or report is not None)
    if record['lattice'] is not None:
        lattice = record['lattice']
        # Until the literal lattice phase is integrated, a composed lattice is a component run.
        test.assertIsNotNone(config['component'])
        test.assertEqual(set(lattice), {'input_digest', 'library', 'library_sha256', 'out',
            'restore_rc', 'verify_rc', 'verify_last_line', 'chain', 'input', 'twists', 'seconds'})
        test.assertIs(lattice['chain'], False)
        test.assertTrue(lattice['input'].startswith('table:'))
        relative(lattice['input'][6:])
        digest(lattice['input_digest'])
        relative(lattice['library'])
        test.assertEqual(hashlib.sha256((out / lattice['library']).read_bytes()).hexdigest(),
                         lattice['library_sha256'])


def replay(test, out, record, m=None):
    """The test's own replay of every recorded word from the start table."""
    m = m or FakeMechanics()
    state = m.load(out / 'start-table.json')
    for step in record['steps']:
        m.apply_word(state, R.read_json(out / step['word']))
        test.assertEqual(state.digest(), step['after']['digest'])
    return state


CHECKER = None


def checker():
    """check_run.py, loaded once as its own module."""
    global CHECKER
    if CHECKER is None:
        spec = importlib.util.spec_from_file_location('tutorial_check_run', HERE / 'check_run.py')
        CHECKER = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(CHECKER)
    return CHECKER


def independent_check(out, allow_unchecked=False):
    """check_run.py on a record. ``allow_unchecked`` lets a record pass whose contracts a test leaves
    unchecked (they are still counted)."""
    rows = []
    if allow_unchecked:
        final = checker().check_run(out, rows.append, allow_unchecked=True)
    else:
        final = checker().check_run(out, rows.append)
    return final, rows


REAL_ACCEPTANCE_CONFIG = R.acceptance_config


def any_acceptance_config(args):
    """Stands in for the committed acceptance options, so a test may run acceptance mode with its own."""
    return {'file': R.ACCEPTANCE_CONFIG.relative_to(ROOT).as_posix(), 'sha256': '0' * 64,
            'options': {name: getattr(args, name) for name in R.ACCEPTANCE_OPTIONS}}


class TemporaryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='tutorial-runner-test-'))
        self.addCleanup(shutil.rmtree, self.tmp)
        environment = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(self.tmp / 'cache/home_centroids.npy'))
        environment.start()
        self.addCleanup(environment.stop)
        # AcceptanceConfigTests checks the committed options; the other tests choose their own.
        frozen = patch.object(R, 'acceptance_config', side_effect=any_acceptance_config)
        frozen.start()
        self.addCleanup(frozen.stop)

    def args(self, out='run', *extra):
        return R.parse_args(['--scramble', '1', '7', '--out', str(self.tmp / out),
            '--cache', str(self.tmp / 'cache'), '--bulk-steps', '0',
            '--fallback-max-length', '0', '--no-lattice', *extra])

    def assert_processes_exited(self, pids):
        for pid in pids:
            path = Path('/proc') / str(pid) / 'stat'
            # A zombie has exited; the environment's PID 1 owns its reaping.
            try:
                state = path.read_text().rsplit(')', 1)[1].split()[0]
            except FileNotFoundError:
                continue
            self.assertIn(state, ('Z', 'X'), f'process {pid} still running')


class FakeState:
    """Off pieces with even heights; a piece not named in ``heights`` is at height 2. ``turn``
    stands for a rotation the labels do not see, so a lattice state can differ from solved."""
    def __init__(self, off=(1, 2), heights=None, turn=0):
        self.off = set(off)
        self.height = {p: h for p, h in (heights or {}).items() if p in self.off and h != 2}
        self.turn = turn
        self.journal = []

    def digest(self):
        if not self.off and not self.turn:
            return R.SOLVED
        key = [sorted(self.off), sorted(self.height.items())] if self.height else sorted(self.off)
        return hashlib.sha256(json.dumps([key, self.turn] if self.turn else key).encode()).hexdigest()

    def off_lattice_count(self):
        return len(self.off)


class FakeMechanics:
    def __init__(self, start=None):
        self.start = start

    def inverse_word(self, word):
        return [list(pair) for pair in reversed(word)]

    def scramble(self, k, seed):
        return FakeState(self.start.off, self.start.height, self.start.turn) if self.start else FakeState()

    def load(self, path):
        doc = R.read_json(path)
        return FakeState(doc['off_pieces'], {int(p): h for p, h in doc.get('heights', {}).items()}, doc.get('turn', 0))

    def export(self, state, path):
        doc = {'digest': state.digest(), 'off_pieces': sorted(state.off),
               'heights': {str(p): h for p, h in sorted(state.height.items())}}
        if state.turn:
            doc['turn'] = state.turn
        R.atomic_json(path, doc)

    def heights(self, state):
        histogram = {}
        for piece in state.off:
            h = state.height.get(piece, 2)
            histogram[h] = histogram.get(h, 0) + 1
        return {str(h): histogram[h] for h in sorted(histogram)}

    def snapshot(self, state):
        return {'digest': state.digest(), 'off': len(state.off), 'heights': self.heights(state)}

    def labels_identity(self, state):
        # The turn is a rotation the labels do not see.
        return not state.off

    def off_set(self, state):
        return state.off.copy()

    def play(self, state, word):
        """Item 0 moves nothing, item 1 lands off piece c, item 2, 4 or 6 sets its height."""
        for c, i in word:
            if i == 0:
                continue
            if c not in state.off or i not in (1, 2, 4, 6):
                raise ValueError('inadmissible fake letter')
            if i == 1:
                state.off.remove(c)
            if i in (1, 2):
                state.height.pop(c, None)
            else:
                state.height[c] = i

    def apply_word(self, state, doc):
        doc = R.literal(doc, state.digest())
        self.play(state, doc['word'])
        if state.digest() != doc['output_digest']:
            raise ValueError('word output digest mismatch on own replay')


def play_real(m, state, word):
    for c, i in word:
        outcome = state.apply(m.sim.Twist(m.restore.ctx, c, m.restore.element(m.menu, c, i),
                                          'menu', {'menu': m.menu.name}))
        if not outcome.applied or outcome.uncertain:
            raise ValueError('inadmissible or uncertain letter')


def end_digest(m, table, word):
    """The digest a word reaches from a table, computed by the test rather than the runner."""
    state = m.load(table)
    if isinstance(m, FakeMechanics):
        m.play(state, word)
    else:
        play_real(m, state, word)
    return state.digest()


class Plugin:
    """A scripted phase plugin. ``script`` maps a sub-phase (or the phase) to a list of
    answers: an End, or a step (rule, word) or (rule, word, tree) proposed at the current state."""
    STUB = False
    SCRIPTS = []

    def __init__(self, phase, script=None, stub=False, fields=None):
        self.phase, self.calls, self.fields = phase, [], fields or {}
        self.script = {name: list(answers) for name, answers in (script or {}).items()}
        if stub:
            self.STUB = True

    def next_step(self, ctx):
        name = ctx.subphase or self.phase
        self.calls.append(name)
        answers = self.script.get(name)
        answer = answers.pop(0) if answers else api.End('stuck', 'no scripted step')
        if isinstance(answer, BaseException):
            raise answer
        if isinstance(answer, api.End):
            return answer
        rule, word, *tree = answer
        doc = {'word': word, 'input_digest': ctx.state.digest(),
               'output_digest': end_digest(ctx.mechanics, ctx.table, word)}
        contract, gate = ('kernel_landing', 'r22d25') if self.phase == 'high' else ('height', name)
        return api.Proposal(self.phase, ctx.subphase, rule, doc, contract, gate,
                            tree=tree[0] if tree else None, fields=dict(self.fields))


def plugins(**given):
    """The runner's own plugin modules (stubs until P1-P3), with the given ones in their place."""
    modules = {phase: importlib.import_module(name) for phase, name in R.PLUGINS.items()}
    modules.update(given)
    return modules


def stub_lattice():
    """A lattice plugin that plays nothing, so the runner composes the program instead (R22D-57); the
    composition path stays for such a stub only (plan revision 3e, E2)."""
    return SimpleNamespace(STUB=True, SCRIPTS=[], play=lambda ctx: api.End('not_applicable'))


class LatticePlay:
    """A lattice plugin with packet P3's interface whose child runs in process: it plays the
    scripted steps (rule, word, tree) on a copy of the current state and writes their ia16 words,
    trees and the end table where P3's child writes them. ``edit`` then changes the result."""
    STUB = False
    SCRIPTS = []

    def __init__(self, script, edit=None):
        self.script, self.edit, self.work = script, edit, None

    def play(self, ctx):
        m, self.work = ctx.mechanics, ctx.work_dir / 'play'
        state, before, steps = m.load(ctx.table), ctx.snapshot(), []
        start = before['digest']
        for k, (rule, word, tree) in enumerate(self.script):
            if isinstance(m, FakeMechanics):
                m.play(state, word)
            else:
                play_real(m, state, word)
            writer = api.Ia16Writer(self.work / 'words' / f'{k:05d}.ia16.gz')
            writer.write_letters(word)
            written = writer.close()
            tree_file = self.work / 'step-trees' / f'{k:05d}.json.gz'
            tree = api.write_tree(tree_file, tree or api.single(rule, len(word)), len(word))
            after = m.snapshot(state)
            steps.append(api.LatticeStep(rule, writer.path, written['sha256'], written['letters'], tree_file,
                                         tree['sha256'], tree['size'], before, after, {'orbit': k}, 0.0))
            before = dict(after)
        if steps:
            steps[-1].fields['labels_identity'] = True
        m.export(state, self.work / 'end-table.json')
        # The parts of P3's report the runner and the checker read (I1R-01): the run's restorer, the start and end
        # digests, a solved player, and played and scheduled counts that add up to the steps' twists.
        corrections = sum(s.letters for s in steps if s.rule.startswith('lattice/correction'))
        retained = sum(s.letters for s in steps) - corrections
        report = {'restorer': ctx.budgets.get('lattice_restorer'), 'player': {
            'input': {'digest': start}, 'result': 'solved', 'failure': None,
            'played': {'retained': retained, 'corrections': corrections, 'total': retained + corrections},
            'scheduled': {'retained': retained, 'corrections': corrections},
            'final': {'digest': m.snapshot(state)['digest'], 'labels_identity': True}}}
        result = api.LatticeResult(steps, self.work / 'end-table.json', report)
        if self.edit is not None:
            self.edit(result)
        return result


def bulk_finds_nothing(runner):
    """Stand in for a bulk child that finds no step at the start state, so the run is no component."""
    def bulk():
        runner.record['phases']['bulk']['result'] = 'stuck'
        runner.record['phases']['bulk']['completed'] = True
        runner.save()
    return patch.object(runner, 'bulk', side_effect=bulk)


def fake_units(state, work):
    work.mkdir(parents=True, exist_ok=True)
    units = []
    for key in sorted(state.off):
        path = work / f'u{key}.json'
        R.atomic_json(path, {'pieces': [key]})
        units.append(R.Unit(key, 1, path))
    return units


class StubRunner(R.Runner):
    def __init__(self, args, plugins=None, start=None):
        super().__init__(args, FakeMechanics(start), plugins)
        self.fail = set()
        self.delays = {}
        self.finished = []

    def units(self, table, work):
        return fake_units(self.state, work)

    def make_candidate(self, table, unit, work, rule='pure'):
        work.mkdir(parents=True, exist_ok=True)
        state = self.m.load(table)
        before = state.digest()
        off_before = len(state.off)
        self.m.play(state, [[unit.key, 1]])
        word = {'word': [[unit.key, 1]], 'input_digest': before, 'output_digest': state.digest()}
        check = {'start_digest': before, 'end_digest': state.digest(), 'unit_size': 1,
                 'unit_on_lattice': 1, 'others_landed': 0, 'others_exact': len(state.off),
                 'others': len(state.off), 'off_before': off_before, 'off_after': len(state.off),
                 'collateral': 0, 'strict_prime': True}
        end = work / 'end.json'
        self.m.export(state, end)
        return R.Candidate(rule, word, check, end)

    def try_unit(self, table, unit, work):
        time.sleep(self.delays.get(unit.key, 0))
        self.finished.append(unit.key)
        if unit.key in self.fail:
            return R.Trial(attempts=[{'unit_key': unit.key, 'rule': 'pure', 'outcome': 'no word',
                                      'reason': 'stub exhausted', 'seconds': 0.0, 'timed_out': False}])
        return R.Trial(self.make_candidate(table, unit, work))


class PowerLoss(BaseException):
    """A test SIGKILL: bypass the exception handler and suppress finish()."""


def without_times(doc):
    if isinstance(doc, dict):
        return {k: without_times(v) for k, v in doc.items() if k not in ('seconds', 'resumes')}
    if isinstance(doc, list):
        return [without_times(v) for v in doc]
    return doc


class ResumeTests(TemporaryTest):
    """P6: every saved boundary and an uncommitted word, using state-only stub steps."""
    SCRIPT = {'r22d75': [api.End('stuck', '75 exhausted')],
              'r22d81': [('r22d81/H6', [[1, 2]])],
              'h4': [('h4/L4', [[2, 2]])],
              'high': [api.End('stuck', 'high exhausted')]}

    def runner(self, name, resume=False, script=None, start=None, extra=()):
        args = self.args(name, '--resume', *extra) if resume else self.args(name, *extra)
        saved = R.read_json(args.out / 'run.json') if resume else {'steps': []}
        script = copy.deepcopy(self.SCRIPT if script is None else script)
        for name in script:
            count = sum((s['subphase'] or s['phase']) == name for s in saved['steps'])
            script[name] = script[name][count:]
        modules = plugins(height=Plugin('height', {k: v for k, v in script.items() if k != 'high'}),
                          high=Plugin('high', {'high': script.get('high', [])}))
        return StubRunner(args, modules, start or FakeState({1, 2, 3}, {1: 6, 2: 4}))

    def run_stub(self, runner):
        with bulk_finds_nothing(runner):
            return runner.run()

    def interrupted(self, runner, at_step, inside=False):
        write = R.atomic_json

        def crash(path, doc):
            write(path, doc)
            path = Path(path)
            if inside:
                stop = path.parent == runner.out / 'words' and int(path.name.split('.')[0]) == at_step
            else:
                stop = path == runner.out / 'run.json' and len(doc['steps']) == at_step
            if stop:
                raise PowerLoss()

        with bulk_finds_nothing(runner), patch.object(runner, 'finish'), patch.object(R, 'atomic_json', side_effect=crash):
            with self.assertRaises(PowerLoss):
                runner.run()
        return R.read_json(runner.out / 'run.json')

    def test_every_step_boundary_and_uncommitted_word(self):
        original = self.run_stub(self.runner('original'))
        self.assertEqual(original['result'], 'solved')
        self.assertEqual(len(original['steps']), 5)
        for inside in (False, True):
            for index in range(5 if inside else 6):
                name = f'cut-{inside}-{index}'
                with self.subTest(inside=inside, index=index):
                    cut = self.runner(name)
                    before = self.interrupted(cut, index, inside)
                    self.assertEqual(len(before['steps']), index)
                    for directory in ('words', 'step-trees', 'tables', 'attempts', 'units'):
                        R.atomic_json(cut.out / directory / '00100.json', {})
                    R.atomic_json(cut.out / 'bulk-progress.json', {})
                    (cut.out / 'run.json.tmp').write_text('partial')
                    resumed = self.runner(name, resume=True)
                    result = self.run_stub(resumed)
                    self.assertEqual(without_times(original), without_times(result))
                    self.assertEqual(result['resumes'][0]['at_step'], index)
                    self.assertGreaterEqual(result['totals']['seconds'], before['totals']['seconds'])
                    self.assertEqual(result['resumes'][0]['seconds'], before['totals']['seconds'])
                    self.assertEqual(replay(self, resumed.out, result).digest(), R.SOLVED)
                    check_record(self, resumed.out, result)
                    self.assertFalse(list(resumed.out.rglob('*.tmp')))
                    self.assertFalse((resumed.out / 'bulk-progress.json').exists())
                    self.assertFalse(any((resumed.out / d / '00100.json').exists() for d in
                                         ('words', 'step-trees', 'tables', 'attempts', 'units')))

    def test_saved_height_and_high_counters_and_high_budget(self):
        cases = [('r22d75', FakeState({1, 2}, {1: 6}), {'r22d75': [('r22d75/T0', [[1, 4]])]}),
                 ('r22d81', FakeState({1, 2}, {1: 6}), {'r22d81': [('r22d81/H6', [[1, 4]])]}),
                 ('h4', FakeState({1, 2}, {1: 4}), {'h4': [('h4/L4', [[1, 4]])]}),
                 ('high', FakeState({1, 2}, {1: 6}), {'high': [('r22d25/F1', [[1, 4]])]})]
        for phase, start, script in cases:
            with self.subTest(phase=phase):
                extra = ('--high-steps', '1') if phase == 'high' else ()
                original = self.run_stub(self.runner('whole-' + phase, script=script, start=start, extra=extra))
                cut = self.runner('cut-' + phase, script=script, start=start, extra=extra)
                self.interrupted(cut, 1)
                resumed = self.runner('cut-' + phase, True, script=script, start=start, extra=extra)
                result = self.run_stub(resumed)
                self.assertEqual(without_times(original), without_times(result))
                data = result['phases']['high'] if phase == 'high' else result['phases']['height']['sub'][phase]
                self.assertEqual((data['steps'], data['twists']), (1, 1))
                if phase == 'high':
                    self.assertEqual((result['result'], data['result']), ('budget', 'budget'))

    def test_signal_resume_and_multiple_resumes(self):
        cut = self.runner('signal')
        record = self.interrupted(cut, 1)
        record.update(result='error', error='signal 15')
        R.atomic_json(cut.out / 'run.json', record)
        resumed = self.runner('signal', True)
        self.interrupted(resumed, 2)
        final = self.run_stub(self.runner('signal', True))
        self.assertEqual((final['result'], final['error']), ('solved', None))
        self.assertEqual([r['at_step'] for r in final['resumes']], [1, 2])
        self.assertLessEqual(final['resumes'][0]['seconds'], final['resumes'][1]['seconds'])

    def test_stuck_high_is_not_reentered_after_tail_step(self):
        start = FakeState({1, 2}, {1: 6})
        original = self.run_stub(self.runner('whole-high-stuck', script={}, start=start))
        cut = self.runner('cut-high-stuck', script={}, start=start)
        self.interrupted(cut, 1)
        resumed = self.runner('cut-high-stuck', True, script={}, start=start)
        actual = self.run_stub(resumed)
        self.assertEqual(actual['phases']['high']['result'], 'stuck')
        self.assertEqual(resumed.plugins['high'].calls, [])
        self.assertEqual(without_times(actual), without_times(original))

    def test_finished_phase_files_at_the_resume_count_survive(self):
        # The D2 shape: high ends stuck with its kernel record in attempts/<count>-high, and the
        # container dies in the tail's unit listing before a tail row. Resume keeps that record.
        start = FakeState({1, 2}, {1: 6})
        cut = self.runner('cut-high-files', script={}, start=start)

        def stuck(ctx):
            kernel = ctx.work_dir / 'kernel'
            kernel.mkdir(parents=True)
            R.atomic_json(kernel / 'run.json', {'result': 'stuck'})
            return api.End('stuck', 'kernel_exhausted', details={'dir': cut.rel(kernel)})

        cut.plugins['high'].next_step = stuck
        with bulk_finds_nothing(cut), patch.object(cut, 'finish'), patch.object(cut, 'units', side_effect=PowerLoss()):
            with self.assertRaises(PowerLoss):
                cut.run()
        saved = R.read_json(cut.out / 'run.json')
        self.assertEqual((len(saved['steps']), saved['phases']['high']['details']['dir']),
                         (0, 'attempts/00000-high/kernel'))
        (cut.out / 'attempts/00000-tail-stray').mkdir()
        resumed = self.runner('cut-high-files', True, script={}, start=start)
        result = self.run_stub(resumed)
        self.assertEqual(resumed.plugins['high'].calls, [])
        self.assertEqual(result['phases']['high'], saved['phases']['high'])
        self.assertTrue((resumed.out / 'attempts/00000-high/kernel/run.json').is_file())
        self.assertFalse((resumed.out / 'attempts/00000-tail-stray').exists())

    def test_interrupted_lattice_restarts_with_clean_work_directory(self):
        args = self.args('lattice-restart')
        args.no_lattice = False
        start = FakeState((), turn=1)

        def interrupted_play(ctx):
            (ctx.work_dir / 'restore').mkdir()
            runner.on_signal(signal.SIGTERM, None)
            return api.End('error', 'cancelled', attempts=[{'rule': 'lattice', 'timed_out': False}])

        modules = plugins(lattice=SimpleNamespace(SCRIPTS=[], play=interrupted_play))
        runner = StubRunner(args, modules, start)
        record = runner.run()
        self.assertEqual(record['error'], 'signal 15')
        self.assertTrue((args.out / 'attempts/00000-lattice/restore').exists())
        args.resume = True
        result = StubRunner(args, plugins(lattice=LatticePlay([])), start).run()
        self.assertEqual(result['result'], 'solved')
        check_record(self, args.out, result)
        self.assertFalse((args.out / 'attempts/00000-lattice/restore').exists())

    def test_interrupted_plugin_attempt_is_not_a_completed_phase_attempt(self):
        start = FakeState({1, 2}, {1: 6})
        script = {'r22d75': [('r22d75/T0', [[1, 2]])]}
        original = self.run_stub(self.runner('whole-cancel', script=script, start=start))
        cut = self.runner('cut-cancel', script=script, start=start)

        def cancelled(ctx):
            cut.on_signal(signal.SIGTERM, None)
            return api.End('error', 'cancelled', attempts=[{'rule': 'height', 'timed_out': False}])

        cut.plugins['height'].next_step = cancelled
        record = self.run_stub(cut)
        self.assertEqual(record['error'], 'signal 15')
        self.assertTrue(record['phases']['height']['sub']['r22d75']['attempts'])
        result = self.run_stub(self.runner('cut-cancel', True, script=script, start=start))
        self.assertEqual(without_times(result), without_times(original))

    def tree(self, root):
        return {p.relative_to(root).as_posix(): p.read_bytes() if p.is_file() else None
                for p in root.rglob('*')}

    def refused(self, runner):
        before = self.tree(runner.out)
        with self.assertRaises(R.ResumeError):
            runner.run()
        self.assertEqual(before, self.tree(runner.out), 'a refusal must not write or clean up')

    def test_resume_checks_all_committed_tables_before_writing(self):
        cut = self.runner('committed-tables')
        saved = self.interrupted(cut, 2)
        self.assertEqual([s['phase'] for s in saved['steps']], ['height', 'height'])
        for table in ('tables/00000.json', 'tables/00001.json', 'tables/bulk-end.json'):
            for damage in ('missing', 'unreadable', 'digest', 'state'):
                name = Path(table).stem + '-' + damage
                with self.subTest(table=table, damage=damage):
                    out = self.tmp / name
                    shutil.copytree(cut.out, out)
                    path = out / table
                    if damage == 'missing':
                        path.unlink()
                    elif damage == 'unreadable':
                        path.write_text('incomplete JSON {')
                    else:
                        doc = R.read_json(path)
                        doc.update({'digest': '0' * 64} if damage == 'digest' else {'off_pieces': []})
                        R.atomic_json(path, doc)
                    (out / 'run.json.tmp').write_text('retain on refusal')
                    resumed = self.runner(name, True)
                    before = self.tree(out)
                    with self.assertRaisesRegex(R.ResumeError, 'resume committed table'):
                        resumed.run()
                    self.assertEqual(before, self.tree(out), 'refusal must precede cleanup or writes')

    def test_resume_rebuilds_the_mutable_in_flight_bulk_table(self):
        for damage in ('missing', 'unreadable', 'digest'):
            with self.subTest(damage=damage):
                cut = self.runner('mutable-' + damage)
                cut.initialize()
                path = cut.out / 'tables/bulk-end.json'
                if damage == 'missing':
                    path.unlink()
                elif damage == 'unreadable':
                    path.write_text('incomplete JSON {')
                else:
                    cut.m.export(FakeState(()), path)
                resumed = self.runner('mutable-' + damage, True)
                self.assertEqual(resumed.resume(), 'bulk')
                self.assertEqual(resumed.m.load(path).digest(), cut.state.digest())

    def test_completed_bulk_is_not_reentered_before_height(self):
        for label in ('zero-budget', 'timeout', 'stuck', 'lattice'):
            extra = (() if label in ('zero-budget', 'lattice') else ('--bulk-steps', '2'))
            if label == 'timeout':
                extra += ('--bulk-seconds', '0.000000001')
            start = FakeState((), turn=1) if label == 'lattice' else None
            name = 'completed-bulk-' + label
            cut = self.runner(name, start=start, extra=extra)
            cut.initialize()

            def child(*args, **kwargs):
                if label == 'timeout':
                    return SimpleNamespace(poll=lambda: None, returncode=-9)
                record = R.read_json(cut.out / 'run.json')
                record['phases']['bulk']['result'] = 'stuck'
                R.atomic_json(cut.out / 'run.json', record)
                return SimpleNamespace(poll=lambda: 0, returncode=0)

            with self.subTest(result=label), patch.object(R.subprocess, 'Popen', side_effect=child), \
                    patch.object(R, 'kill_child'), patch.object(cut, 'initialize'), patch.object(cut, 'finish'), \
                    patch.object(cut, 'phases_between', side_effect=PowerLoss()):
                with self.assertRaises(PowerLoss):
                    cut.run()
            saved = R.read_json(cut.out / 'run.json')
            self.assertIsNone(saved['phases']['height']['start'])
            resumed = self.runner(name, True, start=start, extra=extra)
            with patch.object(resumed, 'bulk') as bulk:
                result = resumed.run()
            with self.subTest(result=label):
                bulk.assert_not_called()
                self.assertIs(saved['phases']['bulk']['completed'], True)
                self.assertEqual(result['phases']['bulk'], saved['phases']['bulk'])
                # The explicit marker is optional: old records keep their old classification.
                saved['phases']['bulk'].pop('completed')
                self.assertEqual(R.resume_point(saved, cut.m.snapshot(cut.state)), 'bulk')

    def test_bulk_child_distinguishes_final_budget_from_interim_steps(self):
        class Restorer:
            def __init__(self, state, menu, order):
                self.state, self.order, self.moves = state, order, []

            def misalignment(self):
                return len(self.state.off)

            def seam(self):
                piece = min(self.state.off)
                self.state.off.remove(piece)
                self.moves.append((piece, 1))
                return [self.moves[-1]]

            cover = level1 = level2 = improve = shift = conj = seam

        cut = self.runner('bulk-final-budget', start=FakeState({1, 2}), extra=('--bulk-steps', '1'))
        cut.initialize()
        cut.m.menu = None
        cut.m.seams = SimpleNamespace(SeamRestorer=Restorer)
        saves, save = [], R.Runner.save

        def observe(runner):
            save(runner)
            saves.append(copy.deepcopy(runner.record))

        with patch.object(R, 'Mechanics', return_value=cut.m), patch.object(R.threading, 'Thread'), \
                patch.object(R.Runner, 'save', observe):
            R.bulk_child(cut.out, self.tmp / 'cache')
        resumed = self.runner('bulk-final-budget', True, extra=('--bulk-steps', '1'))
        tables, resume = [], resumed.resume

        def capture_resume_table():
            point = resume()
            tables.append(resumed.table)
            return point

        with patch.object(resumed, 'resume', side_effect=capture_resume_table), patch.object(resumed, 'bulk') as bulk:
            result = resumed.run()
        bulk.assert_not_called()
        self.assertEqual(tables, [resumed.out / 'tables/bulk-end.json'])
        self.assertEqual(result['result'], 'solved')
        self.assertEqual([r['phases']['bulk']['completed'] for r in saves], [False, True])
        self.assertEqual([r['phases']['bulk']['result'] for r in saves], ['budget', 'budget'])
        self.assertTrue(all(r['config']['component'] is None for r in saves))
        self.assertEqual(result['config']['component'], 'bulk_budget')

    def test_resume_refusals_write_nothing(self):
        cut = self.runner('source')
        original = self.interrupted(cut, 1)
        changes = {
            'format': lambda r: r.update(format='runner-run/2'),
            'start': lambda r: r['start'].update(digest='0' * 64),
            'config': lambda r: r['config'].update(literal_limit=0),
            'scripts': lambda r: r['config']['scripts'].update({next(iter(r['config']['scripts'])): '0' * 64}),
            'code missing': lambda r: r['config'].pop('code'),
            'code changed': lambda r: r['config']['code'].update(sha256='0' * 64),
            'bulk completion type': lambda r: r['phases']['bulk'].update(completed='yes'),
            'source seed': lambda r: r['start'].update(seed=8),
            'source depth': lambda r: r['start'].update(k=2),
            'solved': lambda r: r.update(result='solved'),
            'stuck': lambda r: r.update(result='stuck'),
            'stuck block': lambda r: r.update(stuck={'phase': 'tail'}),
            'error': lambda r: r.update(result='error', error='rule crashed'),
            'step before': lambda r: r['steps'][0]['before'].update(digest='0' * 64),
            'step after': lambda r: r['steps'][0]['after'].update(digest='0' * 64),
            'word hash': lambda r: r['steps'][0].update(word_sha256='0' * 64),
            'lattice word': lambda r: r['steps'][0].update(encoding='ia16'),
        }
        for name in api.SUBPHASES['height']:
            for result in ('budget', 'timeout', 'error'):
                changes[name + result] = lambda r, n=name, v=result: r['phases']['height']['sub'][n].update(result=v)
        for name in ('high', 'tail', 'lattice'):
            for result in ('budget', 'timeout', 'error'):
                changes[name + result] = lambda r, n=name, v=result: r['phases'][n].update(result=v)
        for label, edit in changes.items():
            name = label.replace(' ', '-')
            with self.subTest(label=label):
                out = self.tmp / name
                shutil.copytree(cut.out, out)
                record = copy.deepcopy(original)
                edit(record)
                R.atomic_json(out / 'run.json', record)
                self.refused(self.runner(name, True))
        bad = self.runner('bad-word')
        self.interrupted(bad, 1)
        word = R.read_json(bad.out / 'run.json')['steps'][0]['word']
        (bad.out / word).write_bytes(b'\x1f\x8b' + b'broken gzip' * 5)
        self.refused(self.runner('bad-word', True))
        record = copy.deepcopy(original)
        record['format'] = 'runner-run/2'
        R.atomic_json(cut.out / 'run.json', record)
        before = self.tree(cut.out)
        with patch('sys.stderr', new=io.StringIO()) as stderr:
            self.assertEqual(R.main(['--scramble', '1', '7', '--out', str(cut.out), '--resume']), 2)
            self.assertIn('resume refused', stderr.getvalue())
        self.assertEqual(before, self.tree(cut.out))

    def test_source_table_and_cli_refusals(self):
        source = self.tmp / 'table.json'
        FakeMechanics().export(FakeState({1, 2}, {1: 6}), source)
        args = R.parse_args([
            '--table', str(source), '--out', str(self.tmp / 'table'), '--cache', str(self.tmp / 'cache'),
            '--bulk-steps', '0', '--no-lattice'])
        runner = StubRunner(args)
        runner.initialize()
        args.resume = True
        StubRunner(args).resume()                # same table loads to the same digest
        FakeMechanics().export(FakeState({2}), source)
        self.refused(StubRunner(args))
        missing = self.tmp / 'missing'
        with patch('sys.stderr', new=io.StringIO()):
            self.assertEqual(R.main(['--scramble', '1', '7', '--out', str(missing), '--resume']), 2)
        self.assertFalse(missing.exists())
        with patch('sys.stderr', new=io.StringIO()), self.assertRaises(SystemExit):
            R.main(['--scramble', '1', '7', '--out', str(runner.out)])

    def test_output_and_checkpoint_excluded_but_simulator_and_versions_pinned(self):
        root = self.tmp / 'copy'
        source = root / 'research/jumbling/sim/twists.py'
        source.parent.mkdir(parents=True)
        source.write_text('original')
        driver = source.parents[1] / 'driver.py'
        driver.write_text('pinned entry point')
        subprocess.run(['git', 'init', '-b', 'fingerprint-fixture', str(root)], check=True, capture_output=True)
        out = root / 'research/jumbling/restoration/ordinary-output'
        args = self.args('unused')
        args.out = out
        modules = plugins(height=Plugin('height'), high=Plugin('high'), lattice=stub_lattice())
        with patch.object(R, 'ROOT', root), patch.object(R, 'SCRIPTS', [driver.relative_to(root)]):
            for name in ('runs', 'cache', '__pycache__'):
                R.atomic_json(source.parent / name / 'generated.json', {})
            runner = StubRunner(args, modules)
            runner.initialize()
            initial = R.code_identity(out)
            R.atomic_json(out / 'extra.json', {})
            R.atomic_json(out.with_name(out.name + '-checkpoint') / 'run.json', {})
            for name in ('runs', 'cache', '__pycache__'):
                R.atomic_json(source.parent / name / 'generated.json', {'changed': True})
            self.assertEqual(initial, R.code_identity(out))
            args.resume = True
            resumed = StubRunner(args, modules)
            resumed.resume()
            self.assertEqual(resumed.record['config']['code'], initial)
            excluded = (out, out.with_name(out.name + '-checkpoint'))
            self.assertEqual(resumed.record['resumes'][0]['source'], R.source_identity(excluded))
            self.assertEqual(runner.record['config']['source'], R.source_identity(excluded))
            source.write_text('changed simulator')
            self.assertNotIn(source.relative_to(root).as_posix(), resumed.record['config']['scripts'])
            self.refused(StubRunner(args, modules))
            source.write_text('original')
            import numpy as np
            with patch.object(np, '__version__', 'different'):
                self.refused(StubRunner(args, modules))
            with patch.object(R.sys, 'version', 'different'):
                self.refused(StubRunner(args, modules))

    def test_signal_after_saved_lattice_is_terminal_without_writes(self):
        args = self.args('lattice')
        args.no_lattice = False
        start = FakeState((), turn=1)
        runner = StubRunner(args, plugins(lattice=LatticePlay([])), start)
        save = runner.save

        def signal_after_result():
            save()
            if runner.record['phases']['lattice']['result'] is not None:
                runner.on_signal(signal.SIGTERM, None)

        with patch.object(runner, 'save', side_effect=signal_after_result):
            result = runner.run()
        self.assertEqual((result['result'], result['error']), ('error', 'signal 15'))
        args.resume = True
        self.refused(StubRunner(args, plugins(lattice=LatticePlay([])), start))

    def test_bulk_child_rebuilds_before_constructing_state_only_restorer(self):
        class Restorer:
            def __init__(self, state, menu, order):
                self.state, self.order, self.moves = state, order, []

            def misalignment(self):
                return len(self.state.off)

            def seam(self):
                piece = min(self.state.off)
                self.state.off.remove(piece)
                self.moves.append((piece, 1))
                return [self.moves[-1]]

            cover = level1 = level2 = improve = shift = conj = seam

        mechanics = FakeMechanics()
        mechanics.menu = None
        mechanics.seams = SimpleNamespace(SeamRestorer=Restorer)
        args = self.args('bulk', '--bulk-steps', '2')
        runner = R.Runner(args, mechanics, plugins())
        runner.initialize()
        whole = self.tmp / 'bulk-whole'
        shutil.copytree(runner.out, whole)
        with patch.object(R, 'Mechanics', return_value=mechanics), patch.object(R.threading, 'Thread'):
            R.bulk_child(whole, self.tmp / 'cache')
        expected = R.read_json(whole / 'run.json')
        first = copy.deepcopy(expected)
        first['steps'] = first['steps'][:1]
        state = mechanics.load(runner.out / 'start-table.json')
        mechanics.apply_word(state, R.read_json(whole / first['steps'][0]['word']))
        first['phases']['bulk'].update(steps=1, twists=1,
            end=dict(mechanics.snapshot(state), table='tables/bulk-end.json'))
        first['totals']['bulk_twists'] = 1
        R.atomic_json(runner.out / 'run.json', first)
        shutil.copyfile(whole / first['steps'][0]['word'], runner.out / first['steps'][0]['word'])
        tree = first['steps'][0]['tree']['file']
        shutil.copyfile(whole / tree, runner.out / tree)
        # A partial step overwrote the current table: only replay can recover it.
        mechanics.export(FakeState(()), runner.out / 'tables/bulk-end.json')
        constructed = []

        class ObservedRestorer(Restorer):
            def __init__(self, state, menu, order):
                constructed.append(state.off.copy())
                super().__init__(state, menu, order)

        mechanics.seams.SeamRestorer = ObservedRestorer
        with patch.object(R, 'Mechanics', return_value=mechanics), patch.object(R.threading, 'Thread'):
            R.bulk_child(runner.out, self.tmp / 'cache')
        actual = R.read_json(runner.out / 'run.json')
        self.assertEqual(constructed, [{2}])
        self.assertEqual([(s['before'], s['after']) for s in actual['steps']],
                         [(s['before'], s['after']) for s in expected['steps']])
        self.assertEqual((actual['phases']['bulk']['steps'], actual['phases']['bulk']['twists']), (2, 2))


class RecordTests(TemporaryTest):
    def test_source_identity_matches_worktree(self):
        runner = StubRunner(self.args())
        runner.initialize()
        source = runner.record['config']['source']
        self.assertEqual(source['commit'], subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip())
        status = subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain=v1', '--untracked-files=all', '--',
                                          'research/jumbling'], cwd=ROOT)
        self.assertEqual(source['status_sha256'], hashlib.sha256(status).hexdigest())
        self.assertEqual(source['dirty'], bool(status))
        with patch.object(R.subprocess, 'check_output', side_effect=FileNotFoundError):
            self.assertEqual(R.source_identity(), {'commit': None, 'status_sha256': None, 'dirty': None})

    def test_out_with_json_anywhere_is_rejected_before_writes(self):
        for out in (self.tmp / 'x.json', self.tmp / 'a.json/run'):
            with self.subTest(out=out), patch('sys.stderr', new=io.StringIO()), self.assertRaises(SystemExit):
                R.parse_args(['--scramble', '1', '7', '--out', str(out)])
            self.assertFalse(out.exists())
        parent = self.tmp / 'cwd.json'
        parent.mkdir()
        argv = [sys.executable, '-I', '-B', HERE / 'runner.py', '--scramble', '1', '7', '--out', 'run']
        child = subprocess.run([str(a) for a in argv], cwd=parent, capture_output=True, text=True)
        self.assertEqual(child.returncode, 2)
        self.assertIn('out path must not contain .json', child.stderr)
        self.assertEqual(list(parent.iterdir()), [])
        self.assertEqual(sorted(p.name for p in self.tmp.iterdir()), ['cwd.json'])

    def test_checkpoints_and_word_files(self):
        runner = StubRunner(self.args())
        original = R.atomic_json
        checkpoints = []

        def write(path, doc):
            original(path, doc)
            if Path(path) == runner.out / 'run.json':
                check_record(self, runner.out, R.read_json(path))
                checkpoints.append(len(doc['steps']))

        with patch.object(R, 'atomic_json', side_effect=write):
            record = runner.run()
        self.assertEqual(record['result'], 'solved')
        self.assertTrue({0, 1, 2} <= set(checkpoints))
        self.assertEqual(record['totals']['tail_twists'], 2)
        # The tail ran to the lattice; the phases before it found nothing to do.
        self.assertEqual(record['phases']['tail']['result'], 'lattice')
        self.assertEqual(record['phases']['height']['result'], 'not_applicable')
        self.assertEqual(record['phases']['high']['result'], 'not_applicable')
        self.assertEqual(record['config']['component'], 'bulk_budget')
        self.assertEqual(record['acceptance']['verdict'], 'component')

    def test_stuck_lists_every_unit_and_attempt(self):
        runner = StubRunner(self.args())
        runner.fail = {1, 2}
        record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'stuck')
        self.assertEqual((record['stuck']['phase'], record['stuck']['reason']), ('tail', 'no_rule'))
        self.assertEqual([u['unit_key'] for u in record['stuck']['units']], [1, 2])
        self.assertTrue(all(u['attempts'] for u in record['stuck']['units']))

    def test_failed_prefix_is_recorded_on_step(self):
        runner = StubRunner(self.args('run', '--tail-steps', '1'))
        runner.fail = {1}
        record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'budget')
        self.assertEqual(record['phases']['tail']['result'], 'budget')
        self.assertEqual(record['steps'][0]['unit_key'], 2)
        self.assertEqual(record['steps'][0]['attempts'][0]['unit_key'], 1)

    def test_parallelism_preserves_first_success(self):
        words = []
        for par in (1, 2):
            runner = StubRunner(self.args(f'par{par}', '--par', str(par), '--tail-steps', '1'))
            runner.delays = {1: 0.15}
            record = runner.run()
            check_record(self, runner.out, record)
            self.assertEqual(record['steps'][0]['unit_key'], 1)
            words.append(R.read_json(runner.out / record['steps'][0]['word']))
            if par == 2:
                self.assertEqual(runner.finished[:2], [2, 1])
        self.assertEqual(words[0], words[1])

    def test_parallel_timeout_retries_earlier_units_alone(self):
        runner = StubRunner(self.args('run', '--par', '2', '--tail-steps', '1'))
        calls, original = [], runner.try_unit

        def trial(table, unit, work):
            calls.append((unit.key, work.name))
            if unit.key == 1 and not work.name.endswith('-serial'):
                return R.Trial(attempts=[{'unit_key': 1, 'rule': 'pure', 'outcome': 'timeout',
                    'reason': 'busy machine', 'seconds': 0.1, 'timed_out': True}])
            if unit.key == 1:
                self.assertTrue(all(f[0] in (1, 2) for f in calls))
                candidate = original(table, unit, work).candidate
                return R.Trial(candidate, [{'unit_key': 1, 'rule': 'zero_bad', 'outcome': 'no word',
                                            'reason': 'solo retry', 'seconds': 0.0, 'timed_out': False}])
            return original(table, unit, work)

        with patch.object(runner, 'try_unit', side_effect=trial):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['steps'][0]['unit_key'], 1)
        self.assertIn((1, 'u1-serial'), calls)
        self.assertEqual([a['reason'] for a in record['steps'][0]['attempts']], ['busy machine', 'solo retry'])
        self.assertEqual(record['acceptance']['timed_out_attempts'], 1)

    def test_rule_timeout_is_a_failed_attempt(self):
        runner = R.Runner(self.args('run', '--tail-rules', 'pure', '--rule-seconds', '0.12'), FakeMechanics())
        runner.initialize()
        target = runner.out / 'units/test.json'
        R.atomic_json(target, {'pieces': [1]})

        def slow(script, argv, tag, **kwargs):
            return runner.execute([sys.executable, '-I', '-B', '-c',
                'import time; print("started", flush=True); time.sleep(10)'], tag, runner.args.rule_seconds)

        with patch.object(runner, 'script', side_effect=slow):
            started = time.monotonic()
            result = runner.try_unit(runner.table, R.Unit(1, 1, target), runner.out / 'attempts/timeout')
        self.assertLess(time.monotonic() - started, 3)
        self.assertIsNone(result.candidate)
        self.assertEqual(result.attempts[0]['outcome'], 'timeout')
        self.assertIs(result.attempts[0]['timed_out'], True)
        self.assertTrue(list((runner.out / 'logs').glob('*.log')))

    def test_lattice_pass_requires_exactly_the_right_run(self):
        runner = StubRunner(self.args(), plugins(lattice=stub_lattice()))
        runner.initialize()
        runner.state.off.clear()
        # Keep a nonidentity lattice digest, without loading actual geometry.
        runner.state.digest = lambda: 'a' * 64
        runner.args.no_lattice = False

        for kind in ('zero', 'two', 'wrong input', 'not identity', 'fail line', 'good'):
            with self.subTest(kind=kind):
                def script(path, argv, tag, **kwargs):
                    log = runner.out / 'logs' / (tag + '.log')
                    out = runner.out / 'lattice'
                    spec = 'table:' + runner.rel(runner.table)
                    if path == R.CHECKER:
                        self.assertNotIn('--chain', argv)
                        checked = [{'input': spec, 'restored_to_identity': True}]
                        if kind == 'zero':
                            checked = []
                        elif kind == 'two':
                            checked *= 2
                        elif kind == 'wrong input':
                            checked[0]['input'] = 'table:wrong.json'
                        elif kind == 'not identity':
                            checked[0]['restored_to_identity'] = False
                        R.atomic_json(out / 'report.json', {'pass': True, 'runs': checked})
                        log.write_text('FAIL 0.0\n' if kind == 'fail line' else 'PASS 0.0\n')
                    else:
                        R.atomic_json(out / 'restore-summary.json', {'runs': [
                            {'input': spec, 'status': 'restored', 'retained_length': 3, 'correction_twists': 2}]})
                        log.write_text('restored\n')
                    return R.Child(0, 0.0, log)

                with patch.object(runner, 'script', side_effect=script):
                    runner.lattice()
                self.assertEqual(runner.record['result'], 'solved' if kind == 'good' else 'error')
                self.assertEqual(runner.record['lattice']['twists'], 5)
                # The lattice stub keeps the composition path, which makes the run a component.
                self.assertEqual(runner.record['config']['component'], 'lattice_composition')

    def test_lattice_uses_relative_paths_without_chain_after_tail(self):
        runner = StubRunner(self.args('run', '--tail-steps', '1'), plugins(lattice=stub_lattice()))
        runner.run()
        runner.state.off.clear()
        runner.state.digest = lambda: 'a' * 64
        runner.args.no_lattice = False
        commands = []

        def script(path, argv, tag, **kwargs):
            commands.append((path, argv))
            self.assertEqual(kwargs['cwd'], runner.out)
            log = runner.out / 'logs' / (tag + '.log')
            log.write_text('FAIL 0.0\n')
            return R.Child(1, 0.0, log)

        with patch.object(runner, 'script', side_effect=script):
            runner.lattice()
        checker = commands[-1][1]
        self.assertNotIn('--chain', checker)
        self.assertEqual(checker[2:], ['lattice/library-merged.json', 'lattice', 'lattice/report.json'])
        self.assertFalse(runner.record['lattice']['chain'])
        self.assertEqual(runner.record['lattice']['input'], 'table:tables/00000.json')

    def test_fallback_is_recorded_separately(self):
        runner = StubRunner(self.args('run', '--fallback-max-length', '1', '--tail-steps', '1'))
        runner.fail = {1, 2}
        with patch.object(runner, 'fallback', side_effect=lambda unit, work:
                          runner.make_candidate(runner.table, unit, work, 'fallback')):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['result'], 'budget')
        self.assertEqual(record['steps'][0]['phase'], 'fallback')
        self.assertEqual(record['steps'][0]['standing'], 'fallback')
        self.assertEqual(record['totals']['fallback_twists'], 1)
        self.assertEqual(record['totals']['tail_twists'], 0)
        self.assertEqual(record['acceptance']['fallback_steps'], 1)

    def test_failed_fallback_is_in_stuck_attempts(self):
        runner = StubRunner(self.args('run', '--fallback-max-length', '1'))
        runner.fail = {1, 2}
        with patch.object(runner, 'fallback', side_effect=R.RuleFailure('bound exhausted', 'stub bound')):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['stuck']['units'][0]['attempts'][-1]['rule'], 'fallback')
        self.assertIs(record['stuck']['units'][0]['attempts'][-1]['timed_out'], False)

    def test_closure_failure_preserves_vacancy_premises(self):
        runner = StubRunner(self.args())
        runner.initialize()
        unit = R.Unit(1, 1, runner.out / 'units/u.json')
        R.atomic_json(unit.target, {'pieces': [1]})

        def script(path, argv, tag, **kwargs):
            out = Path(argv[5])
            out.mkdir(parents=True, exist_ok=True)
            R.atomic_json(out / 'pre.json', {'vacancy': {'N1': False, 'N2': False, 'deficits': {'2': [2, 1]}}})
            R.atomic_json(out / 'result.json', {'result': 'no V'})
            log = runner.out / 'logs/failure.log'
            log.write_text('no V\n')
            return R.Child(0, 0.0, log)

        with patch.object(runner, 'script', side_effect=script):
            with self.assertRaises(R.RuleFailure) as raised:
                runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/close')
        self.assertEqual(raised.exception.details, {'N1': False, 'N2': False, 'deficits': {'2': [2, 1]}})

    def test_rejects_programs_and_digest_mismatches(self):
        with self.assertRaises(ValueError):
            R.literal({'segments': []})
        with self.assertRaises(ValueError):
            R.literal({'word': [[True, 1]]})
        with self.assertRaises(ValueError):
            R.literal({'word': [], 'input_digest': 'a' * 64}, 'b' * 64)
        runner = StubRunner(self.args())
        runner.initialize()
        unit = runner.units(runner.table, runner.out / 'attempts/units')[0]
        candidate = runner.make_candidate(runner.table, unit, runner.out / 'attempts/bad')
        candidate.word['output_digest'] = 'a' * 64
        with self.assertRaises(ValueError):
            runner.advance(unit, candidate, [], 0.0)
        self.assertEqual(runner.record['steps'], [])

    def test_timeout_with_partial_premises_is_still_a_failed_attempt(self):
        runner = StubRunner(self.args())
        runner.initialize()
        unit = runner.units(runner.table, runner.out / 'attempts/units')[0]

        def script(path, argv, tag, **kwargs):
            out = Path(argv[5])
            out.mkdir(parents=True, exist_ok=True)
            (out / 'pre.json').write_text('{"vacancy":')
            log = runner.out / 'logs/partial.log'
            log.write_text('interrupted\n')
            return R.Child(-9, 0.1, log, timeout=True)

        with patch.object(runner, 'script', side_effect=script):
            with self.assertRaises(R.RuleFailure) as raised:
                runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/partial')
        self.assertEqual(raised.exception.outcome, 'timeout')
        self.assertIs(raised.exception.details['timed_out'], True)

    def test_bulk_order_has_no_reopen(self):
        # reopen reads the solver's earlier twists (plan D2); the bulk takes SeamRestorer's order without it.
        source = ast.parse((ROOT / R.TH / 'seams.py').read_text())
        order = next(ast.literal_eval(node.value) for node in ast.walk(source)
                     if isinstance(node, ast.Assign) and isinstance(node.value, ast.Tuple)
                     and any(getattr(target, 'id', None) == 'ORDER' for target in node.targets))
        self.assertIn('reopen', order)
        self.assertEqual(R.BULK_ORDER, [rule for rule in order if rule != 'reopen'])
        runner = StubRunner(self.args())
        runner.initialize()
        self.assertEqual(runner.record['config']['bulk_order'], R.BULK_ORDER)
        check_record(self, runner.out, runner.record)


class PhaseTests(TemporaryTest):
    """The phase dispatcher with scripted plugins and fake geometry: one test per row of the
    plan's result table (section 0) and the verdict order."""
    def run_phases(self, start, height=None, high=None, extra=(), bulk=True, fail=()):
        modules = plugins(height=height or Plugin('height'), high=high or Plugin('high'))
        runner = StubRunner(self.args('run', *extra), modules, start)
        runner.fail = set(fail)
        with bulk_finds_nothing(runner) if bulk else nullcontext():
            record = runner.run()
        check_record(self, runner.out, record)
        replay(self, runner.out, record)
        return runner, record

    def results(self, record):
        sub = record['phases']['height']['sub']
        return ([sub[name]['result'] for name in api.SUBPHASES['height']] +
                [record['phases'][phase]['result'] for phase in ('high', 'tail')])

    def test_lattice_row(self):
        # A height step that lands every piece: the phases in between are not applicable.
        height = Plugin('height', {'r22d75': [('r22d75/T1', [[1, 1], [2, 1]])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height)
        self.assertEqual(self.results(record), ['lattice', 'not_applicable', 'not_applicable',
                                                'not_applicable', 'not_applicable'])
        self.assertEqual(record['phases']['height']['result'], 'lattice')
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('solved', 'solved'))
        # A kernel call whose single step lands every remaining piece.
        high = Plugin('high', {'high': [('r22d25/F1', [[1, 1], [2, 1]])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 6}), Plugin('height'), high)
        self.assertEqual(self.results(record), ['stuck', 'stuck', 'not_applicable', 'lattice', 'not_applicable'])
        self.assertEqual(record['steps'][0]['phase'], 'high')
        self.assertEqual(record['totals']['high_twists'], 2)
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('solved', 'solved'))

    def test_handoff_row(self):
        height = Plugin('height', {'r22d75': [api.End('stuck', 'no tier fires')],
                                   'r22d81': [('r22d81/H6', [[1, 2]])], 'h4': [('h4/L4', [[2, 2]])]})
        _, record = self.run_phases(FakeState({1, 2, 3}, {1: 6, 2: 4}), height)
        self.assertEqual(self.results(record), ['stuck', 'handoff', 'handoff', 'not_applicable', 'lattice'])
        # r22d75's entry test holds again after the r22d81 step; the order never returns (D3).
        self.assertTrue(entry('r22d75', record['steps'][0]['after']))
        self.assertEqual(height.calls, ['r22d75', 'r22d81', 'h4'])
        self.assertEqual([(s['phase'], s['subphase']) for s in record['steps'][:2]],
                         [('height', 'r22d81'), ('height', 'h4')])
        self.assertEqual(record['totals']['height_twists'], 2)
        self.assertEqual(record['result'], 'solved')
        # The runner decides handoff and lattice from the state; a false claim is an error.
        for claim in ('handoff', 'lattice'):
            with self.subTest(claim=claim):
                height = Plugin('height', {'r22d75': [api.End(claim)]})
                runner, record = self.run_phases(FakeState({1, 2}, {1: 4}), height)
                self.assertEqual(record['result'], 'error')
                self.assertIn('claims ' + claim, record['error'])
                self.assertIsNone(record['phases']['tail']['result'])

    def test_not_applicable_row(self):
        height, high = Plugin('height'), Plugin('high')
        _, record = self.run_phases(FakeState({1, 2}), height, high)
        self.assertEqual(self.results(record)[:4], ['not_applicable'] * 4)
        self.assertEqual(record['phases']['height']['result'], 'not_applicable')
        self.assertEqual(height.calls + high.calls, [])
        self.assertEqual(record['result'], 'solved')
        # A stub's not_applicable where the entry test holds is no step, and the run is a component.
        stub = Plugin('height', {'r22d75': [api.End('not_applicable', 'stub')]}, stub=True)
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), stub)
        self.assertEqual(self.results(record)[:3], ['stuck', 'not_applicable', 'stuck'])
        self.assertEqual(record['config']['component'], 'height_stub')
        self.assertEqual(record['acceptance']['verdict'], 'component')
        # The runner's own plugins (P1 to P3) are not stubs, so none of them can make a component run.
        for name in R.PLUGINS.values():
            self.assertIs(getattr(importlib.import_module(name), 'STUB', False), False, name)
        # Any other plugin claiming not_applicable against its entry test is an error.
        height = Plugin('height', {'r22d75': [api.End('not_applicable')]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height)
        self.assertEqual(record['result'], 'error')
        self.assertIn('not_applicable against its entry test', record['error'])

    def test_stuck_row(self):
        attempt = {'rule': 'r22d75/T1', 'outcome': 'no step', 'timed_out': False}
        height = Plugin('height', {'r22d75': [api.End('stuck', 'no tier fires', {'tiers': 3}, [attempt])]})
        high = Plugin('high', {'high': [api.End('stuck', 'kernel_exhausted')]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height, high, fail={1, 2})
        self.assertEqual(self.results(record), ['stuck', 'not_applicable', 'stuck', 'stuck', 'stuck'])
        sub = record['phases']['height']['sub']['r22d75']
        self.assertEqual((sub['reason'], sub['details'], sub['attempts']), ('no tier fires', {'tiers': 3}, [attempt]))
        self.assertEqual(record['phases']['high']['reason'], 'kernel_exhausted')
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('stuck', 'stuck'))
        self.assertEqual(record['acceptance']['timed_out_attempts'], 0)
        # Every plugin attempt must say whether a clock ended it.
        height = Plugin('height', {'r22d75': [api.End('stuck', attempts=[{'rule': 'r22d75/T1'}])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height)
        self.assertEqual(record['result'], 'error')
        self.assertIn('timed_out', record['error'])

    def test_budget_row(self):
        # After the bulk, a budget continues and labels the run a component.
        _, record = self.run_phases(FakeState({1, 2}), bulk=False)
        self.assertEqual((record['phases']['bulk']['result'], record['result']), ('budget', 'solved'))
        self.assertEqual(record['config']['component'], 'bulk_budget')
        self.assertEqual(record['acceptance']['verdict'], 'component')
        # After any other phase it ends the run (R22D-55).
        height = Plugin('height', {'r22d75': [api.End('budget', 'BUDGET')]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height)
        self.assertEqual(self.results(record), ['budget', None, None, None, None])
        self.assertEqual(record['phases']['height']['result'], 'budget')
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('budget', 'budget'))
        # --high-steps ends the high phase after its own steps.
        high = Plugin('high', {'high': [('r22d25/F1', [[1, 4]])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 6}), Plugin('height'), high, ('--high-steps', '1'))
        self.assertEqual(self.results(record)[3:], ['budget', None])
        self.assertEqual(record['phases']['high']['reason'], 'high_steps')
        self.assertEqual(high.calls, ['high'])
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('budget', 'budget'))

    def test_timeout_row(self):
        # The wall-clock kill of a height call: same as budget.
        attempt = {'rule': 'r22d81/H6', 'outcome': 'timeout', 'timed_out': True}
        height = Plugin('height', {'r22d81': [api.End('timeout', 'wall-clock kill', attempts=[attempt])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 6}), height)
        self.assertEqual(self.results(record), ['stuck', 'timeout', None, None, None])
        self.assertEqual(record['phases']['height']['result'], 'timeout')
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('budget', 'budget'))
        self.assertEqual(record['acceptance']['timed_out_attempts'], 1)

    def test_tail_timeout_row(self):
        runner = StubRunner(self.args('run', '--fallback-max-length', '1'), plugins())
        runner.fail = {2}
        original, fallbacks = runner.try_unit, []

        def trial(table, unit, work):
            if unit.key == 1:
                return R.Trial(attempts=[{'unit_key': 1, 'rule': 'close', 'outcome': 'timeout',
                                          'reason': 'rule clock', 'seconds': 0.1, 'timed_out': True}])
            return original(table, unit, work)

        def fallback(unit, work):
            fallbacks.append(unit.key)
            raise R.RuleFailure('bound exhausted', 'stub bound')

        with bulk_finds_nothing(runner), patch.object(runner, 'try_unit', side_effect=trial), \
                patch.object(runner, 'fallback', side_effect=fallback):
            record = runner.run()
        check_record(self, runner.out, record)
        # In development the fallback still runs first; it takes no step, so the run ends budget.
        self.assertEqual(fallbacks, [1])
        self.assertEqual((record['phases']['tail']['result'], record['stuck']['reason']), ('timeout', 'timeout'))
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('budget', 'budget'))
        self.assertEqual(record['acceptance']['timed_out_attempts'], 1)

    def test_error_row(self):
        tail = []
        for name, answer in (('kernel error', api.End('error', 'd19_missing')),
                             ('exception', RuntimeError('plugin crashed'))):
            with self.subTest(name=name):
                high = Plugin('high', {'high': [answer]})
                runner = StubRunner(self.args(name.replace(' ', '-')), plugins(height=Plugin('height'), high=high),
                                    FakeState({1, 2}, {1: 6}))
                with bulk_finds_nothing(runner), patch.object(runner, 'tail', side_effect=tail.append):
                    record = runner.run()
                check_record(self, runner.out, record)
                self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))
                self.assertIsNone(record['phases']['tail']['result'])
                self.assertEqual(record['phases']['high']['result'], 'error' if name == 'kernel error' else None)
        self.assertEqual(tail, [], 'no later phase runs after an error')

    def test_component_run_that_ends_stuck_is_component(self):
        _, record = self.run_phases(FakeState({1, 2}), bulk=False, fail={1, 2})
        self.assertEqual((record['result'], record['config']['component']), ('stuck', 'bulk_budget'))
        self.assertEqual(record['acceptance']['verdict'], 'component')

    def test_stuck_run_with_a_timed_out_attempt_at_its_end_is_budget(self):
        attempt = {'rule': 'h4/pair', 'outcome': 'timeout', 'timed_out': True}
        height = Plugin('height', {'h4': [api.End('stuck', 'pair shards stopped', attempts=[attempt])]})
        _, record = self.run_phases(FakeState({1, 2}, {1: 4}), height, fail={1, 2})
        self.assertEqual(record['phases']['height']['sub']['h4']['result'], 'stuck')
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('stuck', 'budget'))
        # A timed-out attempt at an earlier state does not make the final stop a budget end.
        runner = StubRunner(self.args('later', '--tail-steps', '2'), plugins())
        runner.fail = {1}
        attempts = iter([[{'unit_key': 1, 'rule': 'pure', 'outcome': 'timeout', 'reason': 'clock',
                           'seconds': 0.1, 'timed_out': True}], []])
        original = runner.try_unit

        def trial(table, unit, work):
            result = original(table, unit, work)
            if unit.key == 1:
                result.attempts = next(attempts) or result.attempts
            return result

        with bulk_finds_nothing(runner), patch.object(runner, 'try_unit', side_effect=trial):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual(record['steps'][0]['attempts'][0]['timed_out'], True)
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('stuck', 'stuck'))
        self.assertEqual(record['acceptance']['timed_out_attempts'], 1)

    def test_verdict_order(self):
        def record(result='stuck', component=None, standings=(), probe=None, timed_out=False):
            stuck = {'units': [{'attempts': [{'timed_out': timed_out}]}]}
            return {'result': result, 'config': {'component': component}, 'stuck': stuck,
                    'acceptance': {'certificate_steps': standings.count('certificate'),
                                   'fallback_steps': standings.count('fallback'), 'probe': probe}}

        cases = [(record('error', 'bulk_budget', ('certificate',)), 'error'),
                 (record('stuck', 'bulk_budget', ('certificate', 'fallback')), 'component'),
                 (record('solved', None, ('certificate', 'fallback')), 'stuck'),
                 (record('solved', None, ('fallback',)), 'coverage_failure'),
                 (record('stuck', None, (), True, True), 'coverage_failure'),
                 (record('stuck', None, (), False, True), 'budget'),
                 (record('budget'), 'budget'),
                 (record('stuck'), 'stuck'),
                 (record('solved'), 'solved')]
        for doc, expected in cases:
            with self.subTest(expected=expected, doc=doc):
                final = any(a['timed_out'] for u in doc['stuck']['units'] for a in u['attempts'])
                self.assertEqual(R.verdict(doc, final), expected)

    def test_probe_word_is_coverage_failure(self):
        for found in (True, False):
            with self.subTest(found=found):
                runner = StubRunner(self.args(f'probe{found}', '--mode', 'acceptance', '--fallback-max-length', '1'),
                                    plugins())
                runner.fail = {1, 2}

                def fallback(unit, work):
                    if not found:
                        raise R.RuleFailure('bound exhausted', 'stub bound')
                    return runner.make_candidate(runner.table, unit, work, 'fallback')

                with bulk_finds_nothing(runner), patch.object(runner, 'fallback', side_effect=fallback):
                    record = runner.run()
                check_record(self, runner.out, record)
                # Acceptance mode never plays the fallback word; a found word is a coverage gap.
                self.assertEqual(record['steps'], [])
                self.assertEqual(record['result'], 'stuck')
                self.assertEqual(record['acceptance']['probe'], found)
                self.assertEqual('probe' in record['stuck'], found)
                self.assertEqual(record['acceptance']['verdict'], 'coverage_failure' if found else 'stuck')
                if found:
                    self.assertEqual(record['stuck']['probe']['unit_key'], 1)
        # In development the fallback step is played, and the verdict is the same.
        runner = StubRunner(self.args('development', '--fallback-max-length', '1'), plugins())
        runner.fail = {1, 2}
        with bulk_finds_nothing(runner), patch.object(runner, 'fallback', side_effect=lambda unit, work:
                                                       runner.make_candidate(runner.table, unit, work, 'fallback')):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual([s['standing'] for s in record['steps']], ['fallback', 'fallback'])
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('solved', 'coverage_failure'))

    def test_record_step_exports_the_table_and_writes_a_tree(self):
        def fresh(name):
            runner = R.Runner(self.args(name), FakeMechanics(FakeState({1, 2}, {1: 6})), plugins())
            runner.initialize()
            return runner

        def proposal(runner, letters, **changes):
            doc = {'word': letters, 'input_digest': runner.state.digest(),
                   'output_digest': end_digest(runner.m, runner.table, letters)}
            values = dict(phase='high', subphase=None, rule='r22d25/F1', word=doc, contract='kernel_lowering',
                          gate='r22d25', fields={'family': 'F1', 'kind': 'lowering'})
            values.update(changes)
            return api.Proposal(**values)

        runner = fresh('run')
        runner.record_step(proposal(runner, [[1, 4]]), 'high', None)
        tree = api.flat('r22d25/ST', [('S', 1), ('T', 1), ('S⁻¹', 1)], inverse={2: 0})
        runner.record_step(proposal(runner, [[7, 0], [1, 1], [7, 0]], rule='r22d25/ST', tree=tree,
                                    contract='kernel_landing'), 'high', None)
        steps = runner.record['steps']
        check_record(self, runner.out, runner.record)
        replay(self, runner.out, runner.record)
        for index, step in enumerate(steps):
            self.assertEqual(step['table'], f'tables/{index:05d}.json')
            self.assertEqual(runner.m.load(runner.out / step['table']).digest(), step['after']['digest'])
            self.assertEqual((step['family'], step['standing'], step['subphase']), ('F1', 'rule', None))
        self.assertEqual(runner.table, runner.out / 'tables/00001.json')
        self.assertEqual(steps[0]['after']['heights'], {'2': 1, '4': 1})
        written = [json.loads(gzip.decompress((runner.out / s['tree']['file']).read_bytes())) for s in steps]
        self.assertEqual(written[0], {'name': 'r22d25/F1', 'letters': 1, 'span': [0, 1]})
        self.assertEqual(written[1], tree)
        self.assertEqual([s['tree']['size'] for s in steps], [1, 4])
        # Rejected proposals record nothing.
        rejected = {'phase': dict(phase='height'), 'subphase': dict(subphase='h4'),
                    'standing': dict(standing='proved'), 'record key': dict(fields={'gate': 'x'}),
                    'forbidden key': dict(fields={'slp': 1}), 'tree': dict(tree=api.single('r22d25/F1', 2))}
        for name, changes in rejected.items():
            with self.subTest(name=name):
                runner = fresh(name.replace(' ', '-'))
                with self.assertRaises(ValueError):
                    runner.record_step(proposal(runner, [[1, 4]], **changes), 'high', None)
                self.assertEqual(runner.record['steps'], [])
        for name, doc in (('no end digest', {'word': [[1, 4]]}),
                          ('wrong end digest', {'word': [[1, 4]], 'output_digest': 'a' * 64})):
            with self.subTest(name=name):
                runner = fresh(name.replace(' ', '-'))
                with self.assertRaises(ValueError):
                    runner.record_step(proposal(runner, [[1, 4]], word=doc), 'high', None)
                self.assertEqual(runner.record['steps'], [])


class LatticeTests(TemporaryTest):
    """The runner's side of the played lattice phase (plan section 2) with fake geometry: the start
    is on the lattice, turned where the labels do not see, so the lattice phase runs."""
    SCRIPT = [('lattice/orbit-3', [[3, 0], [5, 0]], api.flat('orbit 3', [('word 0', 1), ('word 1', 1)])),
              ('lattice/correction-0-0', [[7, 0]], None)]

    def lattice_run(self, name, plugin, script=None):
        args = self.args(name)
        args.no_lattice = False
        runner = StubRunner(args, plugins(lattice=plugin), FakeState((), turn=1))
        with patch.object(runner, 'script', side_effect=script) if script else nullcontext():
            record = runner.run()
        check_record(self, runner.out, record)
        return runner, record

    def test_played_lattice_is_recorded_with_its_end_table(self):
        plugin = LatticePlay(self.SCRIPT)
        runner, record = self.lattice_run('run', plugin)
        start = record['start']['digest']
        self.assertNotEqual(start, R.SOLVED)
        self.assertEqual((record['result'], record['error'], record['acceptance']['verdict']), ('solved', None, 'solved'))
        self.assertEqual((record['config']['component'], record['lattice']), (None, None))
        self.assertEqual([(s['index'], s['phase'], s['rule'], s['word'], s['letters'], s['orbit']) for s in record['steps']],
                         [(0, 'lattice', 'lattice/orbit-3', 'words/00000.ia16.gz', 2, 0),
                          (1, 'lattice', 'lattice/correction-0-0', 'words/00001.ia16.gz', 1, 1)])
        report = runner.out / 'lattice/report.json'
        self.assertEqual(record['phases']['lattice'], {'result': 'solved', 'steps': 2, 'twists': 3, 'report': {
            'file': 'lattice/report.json', 'sha256': hashlib.sha256(report.read_bytes()).hexdigest()}})
        self.assertEqual(R.read_json(report)['player']['played']['total'], 3)
        self.assertEqual(record['totals']['lattice_twists'], 3)
        # The plugin's files move into the record unchanged: codes 60·grip + item, the scripted tree.
        data = gzip.decompress((runner.out / 'words/00000.ia16.gz').read_bytes())
        self.assertEqual([int.from_bytes(data[k:k + 2], 'little') for k in (0, 2)], [180, 300])
        tree = json.loads(gzip.decompress((runner.out / 'step-trees/00000.json.gz').read_bytes()))
        self.assertEqual(tree, self.SCRIPT[0][2])
        self.assertEqual(list((plugin.work / 'words').iterdir()), [])
        # Solved by identity labels at the played digest, which the labels do not tell from SOLVED (R3-05).
        self.assertEqual(runner.table, runner.out / 'tables/00001.json')
        self.assertEqual(runner.state.digest(), start)

    def test_mismatches_end_the_run_error_before_any_change(self):
        def tree(result):
            # A tree that does not expand to its step's word, under its own sha.
            step = result.steps[1]
            data = (json.dumps({'name': 'correction 0', 'letters': 2, 'span': [0, 2]}) + '\n').encode()
            step.tree_file.write_bytes(gzip.compress(data, mtime=0))
            step.tree_sha256 = hashlib.sha256(data).hexdigest()

        cases = {
            'end table': (lambda r: FakeMechanics().export(FakeState((), turn=2), r.end_table), 'end table digest'),
            'malformed tree': (tree, 'tree length'),
            'word sha256': (lambda r: setattr(r.steps[0], 'word_sha256', 'a' * 64), 'word file'),
            'digest chain': (lambda r: setattr(r.steps[1], 'before', dict(r.steps[1].before, digest='a' * 64)),
                             'digest chain'),
            'labels': (lambda r: r.steps[-1].fields.pop('labels_identity'), 'identity labels'),
            'no report': (lambda r: setattr(r, 'report', None), 'a report'),
            # No step although the restorer scheduled three twists.
            'no step': (lambda r: r.steps.clear(), 'schedule nothing')}
        for name, (edit, reason) in cases.items():
            with self.subTest(name=name):
                plugin = LatticePlay(self.SCRIPT, edit)
                runner, record = self.lattice_run(name.replace(' ', '-'), plugin)
                self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))
                self.assertIn(reason, record['error'])
                # Nothing changed: no row, no file moved, the lattice phase and the state as they were.
                self.assertEqual(record['steps'], [])
                self.assertEqual(record['phases']['lattice'], {'result': None, 'steps': 0, 'twists': None})
                self.assertIsNone(record['totals']['lattice_twists'])
                self.assertEqual([*(runner.out / 'words').iterdir(), *(runner.out / 'step-trees').iterdir()], [])
                self.assertEqual(len(list((plugin.work / 'words').iterdir())), 2)
                self.assertEqual(runner.state.digest(), record['start']['digest'])

    def test_lattice_with_no_step(self):
        # The restorer scheduled nothing (plan revision 3e, E2): solved with no row, after the runner
        # tests the labels itself. A report or end table that says otherwise, or labels that are not
        # the identity, end the run error with the lattice phase unchanged.
        def empty(edit=None):
            return LatticePlay([], edit)

        runner, record = self.lattice_run('run', empty())
        self.assertEqual((record['result'], record['error'], record['acceptance']['verdict']), ('solved', None, 'solved'))
        self.assertEqual(record['steps'], [])
        report = runner.out / 'lattice/report.json'
        self.assertEqual(record['phases']['lattice'], {'result': 'solved', 'steps': 0, 'twists': 0, 'report': {
            'file': 'lattice/report.json', 'sha256': hashlib.sha256(report.read_bytes()).hexdigest()}})
        self.assertEqual(record['totals']['lattice_twists'], 0)
        self.assertEqual(runner.state.digest(), record['start']['digest'])

        def played(result):
            result.report['player']['played']['total'] = 1

        def final(result):
            result.report['player']['final']['labels_identity'] = False

        cases = {'scheduled': (empty(played), 'schedule nothing'),
                 'final labels': (empty(final), 'schedule nothing'),
                 'end table': (empty(lambda r: FakeMechanics().export(FakeState((), turn=2), r.end_table)),
                               'schedule nothing'),
                 'labels not the identity': (empty(), 'not the identity')}
        for name, (plugin, reason) in cases.items():
            with self.subTest(name=name):
                args = self.args(name.replace(' ', '-'))
                args.no_lattice = False
                runner = StubRunner(args, plugins(lattice=plugin), FakeState((), turn=1))
                if name == 'labels not the identity':
                    runner.m.labels_identity = lambda state: False
                record = runner.run()
                check_record(self, runner.out, record)
                self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))
                self.assertIn(reason, record['error'])
                self.assertEqual(record['phases']['lattice'], {'result': None, 'steps': 0, 'twists': None})
                self.assertIsNone(record['totals']['lattice_twists'])
                self.assertFalse((runner.out / 'lattice/report.json').exists())

    def test_lattice_ends_are_recorded(self):
        # An End of the real plugin is recorded with its reason, details and attempts (plan revision 3e,
        # E2): stuck ends the run stuck, budget and timeout end it budget, error ends it error. Nothing
        # was played, so the phase keeps steps 0 and twists null. An End that claims another result
        # with every piece on the lattice ends the run error and leaves the phase unchanged.
        clock = {'rule': 'lattice/late', 'outcome': 'timeout', 'reason': 'lattice_wall_clock', 'timed_out': True}
        ends = {'stuck': (api.End('stuck', 'restorer_no_solution', {'status': 'failure'}), 'stuck', 'stuck'),
                'stuck with a timed-out attempt': (api.End('stuck', 'restorer_no_solution', attempts=[clock]),
                                                   'stuck', 'budget'),
                'timeout': (api.End('timeout', 'lattice_wall_clock', attempts=[clock]), 'budget', 'budget'),
                'budget': (api.End('budget', 'lattice_steps'), 'budget', 'budget'),
                'error': (api.End('error', 'player_failed', {'failure': 'final_labels_identity'}), 'error', 'error')}
        for name, (end, result, verdict) in ends.items():
            with self.subTest(name=name):
                plugin = SimpleNamespace(STUB=False, SCRIPTS=[], play=lambda ctx, end=end: end)
                _, record = self.lattice_run(name.replace(' ', '-'), plugin)
                self.assertEqual((record['result'], record['acceptance']['verdict']), (result, verdict))
                self.assertEqual(record['phases']['lattice'], {'result': end.result, 'steps': 0, 'twists': None,
                    'reason': end.reason, 'details': end.details, 'attempts': end.attempts})
                self.assertEqual(record['acceptance']['timed_out_attempts'], len(end.attempts))
                self.assertEqual((record['steps'], record['config']['component']), ([], None))
                if result == 'error':
                    self.assertIn('lattice phase error: player_failed', record['error'])
                if end.result == 'stuck':
                    self.assertEqual((record['stuck']['phase'], record['stuck']['reason'], record['stuck']['units']),
                                     ('lattice', 'restorer_no_solution', []))
                    self.assertEqual(record['stuck']['digest'], record['start']['digest'])
                else:
                    self.assertIsNone(record['stuck'])
        claims = {'not_applicable': (False, api.End('not_applicable')),
                  'handoff': (False, api.End('handoff')),
                  'lattice': (False, api.End('lattice')),
                  'stub stuck': (True, api.End('stuck', 'stub'))}
        for name, (stub, end) in claims.items():
            with self.subTest(name=name):
                plugin = SimpleNamespace(STUB=stub, SCRIPTS=[], play=lambda ctx, end=end: end)
                _, record = self.lattice_run(name.replace(' ', '-'), plugin)
                if stub:
                    # A stub's stuck is recorded like the real plugin's.
                    self.assertEqual((record['result'], record['phases']['lattice']['result']), ('stuck', 'stuck'))
                    continue
                self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))
                self.assertIn(f'lattice claims {end.result} ', record['error'])
                self.assertEqual(record['phases']['lattice'], {'result': None, 'steps': 0, 'twists': None})
                self.assertEqual((record['steps'], record['stuck'], record['config']['component']), ([], None, None))
        # The stub keeps the composition path (R22D-57), which makes the run a component; here its
        # restorer fails, so the run ends error.
        def script(path, argv, tag, **kwargs):
            log = self.tmp / 'stub' / 'logs' / (tag + '.log')
            log.write_text('FAIL 0.0\n')
            return R.Child(1, 0.0, log)

        _, record = self.lattice_run('stub', stub_lattice(), script)
        self.assertEqual((record['config']['component'], record['lattice']['restore_rc']), ('lattice_composition', 1))
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))


class AcceptanceConfigTests(TemporaryTest):
    """The committed acceptance options (plan revision 3e, E3) and the D19 provenance (E2)."""
    def acceptance_args(self, out, *extra):
        return R.parse_args(['--scramble', '1', '7', '--out', str(self.tmp / out), '--cache', str(self.tmp / 'cache'),
                             '--mode', 'acceptance', *extra])

    def test_committed_options_are_the_defaults(self):
        raw = R.ACCEPTANCE_CONFIG.read_bytes()
        doc = json.loads(raw)
        self.assertEqual(doc['format'], 'acceptance-config/1')
        self.assertEqual(set(doc['options']), set(R.ACCEPTANCE_OPTIONS))
        # An acceptance run needs no option beyond --mode acceptance; the record keeps the file's sha.
        frozen = REAL_ACCEPTANCE_CONFIG(self.acceptance_args('run'))
        self.assertEqual(frozen, {'file': R.ACCEPTANCE_CONFIG.relative_to(ROOT).as_posix(),
                                  'sha256': hashlib.sha256(raw).hexdigest(), 'options': doc['options']})
        # Every clock is lifted and no step budget is set; the lattice phase runs.
        self.assertEqual([doc['options'][k] for k in ('bulk_steps', 'tail_steps', 'high_steps', 'no_lattice')],
                         [None, 0, None, False])

    def test_acceptance_mode_refuses_other_options_before_any_write(self):
        restorer = 'schedule11' if json.loads(R.ACCEPTANCE_CONFIG.read_bytes())['options']['lattice_restorer'] == 'late' \
            else 'late'
        cases = {'par': ('--par', '2'), 'tail_rules': ('--tail-rules', 'pure'), 'bulk_steps': ('--bulk-steps', '3'),
                 'high_steps': ('--high-steps', '0'), 'no_lattice': ('--no-lattice',),
                 'lattice_restorer': ('--lattice-restorer', restorer), 'literal_limit': ('--literal-limit', '40499'),
                 'seeds': ('--seeds', 'default'), 'fallback_max_length': ('--fallback-max-length', '0'),
                 'pair_shards': ('--pair-shards', '2'), 'tail_steps': ('--tail-steps', '5')}
        self.assertEqual(set(cases), set(R.ACCEPTANCE_OPTIONS))
        for name, extra in cases.items():
            with self.subTest(name=name), patch.object(R, 'acceptance_config', REAL_ACCEPTANCE_CONFIG):
                runner = StubRunner(self.acceptance_args(name, *extra))
                with self.assertRaisesRegex(ValueError, f'differ from acceptance-config.json: {name}$'):
                    runner.run()
                self.assertIsNone(runner.record)
                # The runner made its empty directories and wrote no file.
                self.assertEqual([p for p in (self.tmp / name).rglob('*') if not p.is_dir()], [])
        # Development mode reads nothing and records null.
        runner = StubRunner(self.args('development', '--par', '2'))
        runner.initialize()
        self.assertIsNone(runner.record['config']['acceptance_config'])

    def test_kernel_d19_is_recorded(self):
        runner = StubRunner(self.args())
        runner.initialize()
        kernel = importlib.import_module('rules_kernel')
        self.assertEqual(runner.record['config']['kernel']['d19'], {'file': kernel.D19.as_posix(),
            'sha256': hashlib.sha256((ROOT / kernel.D19).read_bytes()).hexdigest()})
        # A scripted high plugin names no D19.
        runner = StubRunner(self.args('scripted'), plugins(high=Plugin('high')))
        runner.initialize()
        self.assertIsNone(runner.record['config']['kernel']['d19'])


class TreeTests(TemporaryTest):
    def closure(self):
        return R.Runner.step_tree('close', [['V', 2], ['middle', 1], ['V_inverse', 2]], 5)

    def test_closure_trees_name_v_and_its_inverse(self):
        tree = self.closure()
        self.assertEqual([(p['name'], p['span']) for p in tree['parts']], [('V', [0, 2]), ('middle', [2, 3]), ('V⁻¹', [3, 5])])
        self.assertEqual(tree['parts'][2]['inverse_of'], [0])
        self.assertEqual(api.check_tree(tree, 5), 4)
        t1 = R.Runner.step_tree('t1prime', [['W', 1], ['V', 2], ['middle', 1], ['V_inverse', 2]], 6)
        self.assertEqual(t1['parts'][3]['inverse_of'], [1])
        self.assertEqual(t1['parts'][2]['inverse_of'], [0])      # T1''s middle is W⁻¹
        self.assertEqual(api.check_tree(t1, 6), 5)
        self.assertEqual(R.Runner.step_tree('pure', None, 3), {'name': 'pure', 'letters': 3, 'span': [0, 3]})
        self.assertEqual(R.Runner.step_tree('t1prime', [['W', 3]], 3)['parts'], [{'name': 'W', 'letters': 1 * 3, 'span': [0, 3]}])
        same = api.flat('conj', [('seed', 1), ('x', 1), ('seed', 1)], same={2: 0})
        self.assertEqual(api.check_tree(same, 3), 4)

    def test_malformed_trees_are_rejected(self):
        def change(edit):
            tree = json.loads(json.dumps(self.closure()))
            edit(tree)
            return tree

        def later(tree):
            tree['parts'][0]['inverse_of'] = [2]
            del tree['parts'][2]['inverse_of']

        deep = {'name': 'leaf', 'letters': 5, 'span': [0, 5]}
        for _ in range(64):
            deep = {'name': 'level', 'letters': 5, 'parts': [deep]}
        faults = {
            'part sum': (change(lambda t: t.update(letters=6)), 5),
            'gap': (change(lambda t: t['parts'][1].update(span=[3, 4])), 5),
            'overlap': (change(lambda t: t['parts'][1].update(span=[1, 2])), 5),
            'root shorter than the word': (self.closure(), 6),
            'span letters': (change(lambda t: t['parts'][1].update(letters=2)), 5),
            'reference to a later part': (change(later), 5),
            'reference of another length': (change(lambda t: t['parts'][2].update(inverse_of=[1])), 5),
            'same_as of another name': (change(lambda t: t['parts'][2].update(same_as=[0])), 5),
            'span and parts': (change(lambda t: t['parts'][1].update(parts=[api.span('x', 2, 3)])), 5),
            'no name': (change(lambda t: t['parts'][1].update(name='')), 5),
            'forbidden key': (change(lambda t: t['parts'][1].update(slp=[])), 5),
            'depth over 64': (deep, 5)}
        for name, (tree, letters) in faults.items():
            with self.subTest(name=name), self.assertRaises(ValueError):
                api.check_tree(tree, letters)
        with self.assertRaises(ValueError):
            api.flat('close', [('V', 2), ('V⁻¹', 2)], inverse={0: 1})
        with self.assertRaises(ValueError):
            api.write_tree(self.tmp / 'bad.json.gz', self.closure(), 6)
        self.assertFalse((self.tmp / 'bad.json.gz').exists())

    def test_written_trees_and_their_sha(self):
        written = api.write_tree(self.tmp / 'tree.json.gz', self.closure(), 5)
        raw = (self.tmp / 'tree.json.gz').read_bytes()
        self.assertEqual(raw[4:8], b'\0' * 4)
        self.assertEqual(written, {'sha256': hashlib.sha256(gzip.decompress(raw)).hexdigest(), 'size': 4})
        self.assertEqual(json.loads(gzip.decompress(raw)), self.closure())
        # check_record finds a tree file that does not match its recorded sha, or is malformed.
        for kind in ('sha', 'file'):
            with self.subTest(kind=kind):
                runner = StubRunner(self.args(kind))
                record = runner.run()
                check_record(self, runner.out, record)
                step = record['steps'][0]
                if kind == 'sha':
                    step['tree']['sha256'] = 'a' * 64
                else:
                    path = runner.out / step['tree']['file']
                    data = (json.dumps({'name': 'pure', 'letters': 2, 'span': [0, 2]}) + '\n').encode()
                    path.write_bytes(gzip.compress(data, mtime=0))
                    step['tree']['sha256'] = hashlib.sha256(data).hexdigest()
                with self.assertRaises(self.failureException):
                    check_record(self, runner.out, record)


class ExecuteTests(TemporaryTest):
    def runner(self):
        runner = StubRunner(self.args())
        runner.initialize()
        return runner

    def test_stop_when_kills_the_group_after_the_first_step_line(self):
        runner = self.runner()
        ready = self.tmp / 'pids.json'
        # The STEP line arrives in two writes; only a complete line counts.
        code = ('import json, os, pathlib, subprocess, sys, time; '
                'child = subprocess.Popen([sys.executable, "-I", "-B", "-c", "import time; time.sleep(120)"]); '
                'pathlib.Path(sys.argv[1]).write_text(json.dumps([os.getpid(), child.pid])); '
                'print("START", flush=True); print("ST", end="", flush=True); time.sleep(0.4); '
                'print("EP 1", flush=True); print("STEP 2", flush=True); time.sleep(120)')
        seen = []

        def stop(line):
            seen.append(line)
            return line.startswith('STEP')

        started = time.monotonic()
        child = runner.execute([sys.executable, '-I', '-B', '-c', code, ready], 'stop-when', None, stop_when=stop)
        elapsed = time.monotonic() - started
        self.assertTrue(child.stopped)
        self.assertFalse(child.timeout or child.cancelled)
        self.assertTrue(0.4 <= elapsed < 10, elapsed)
        self.assertEqual(seen, ['START', 'STEP 1'])
        self.assertIn('STEP 1', child.log.read_text())
        self.assert_processes_exited(R.read_json(ready))

    def test_cancel_still_works_with_stop_when(self):
        runner = self.runner()
        ready = self.tmp / 'pids.json'
        code = ('import json, os, pathlib, subprocess, sys, time; '
                'child = subprocess.Popen([sys.executable, "-I", "-B", "-c", "import time; time.sleep(120)"]); '
                'pathlib.Path(sys.argv[1]).write_text(json.dumps([os.getpid(), child.pid])); '
                'print("WORKING", flush=True); time.sleep(120)')
        with R.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(runner.execute, [sys.executable, '-I', '-B', '-c', code, ready], 'cancel', None,
                                 stop_when=lambda line: False)
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(ready.exists())
            runner.cancel.set()
            child = future.result(timeout=5)
        self.assertTrue(child.cancelled)
        self.assertFalse(child.stopped or child.timeout)
        self.assert_processes_exited(R.read_json(ready))

    def test_no_clock_lets_a_child_finish(self):
        runner = self.runner()
        child = runner.execute([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(0.3); print("DONE")'],
                               'no-clock', None, stop_when=lambda line: line.startswith('STEP'))
        self.assertEqual((child.rc, child.stopped, child.timeout, child.cancelled), (0, False, False, False))
        self.assertEqual(child.log.read_text(), 'DONE\n')


class CertificateStub(R.Runner):
    """Exercise dispatch and gates without importing geometry or the searchers."""
    def __init__(self, args):
        super().__init__(args, FakeMechanics())
        self.calls = []
        self.argv = {}
        self.fault = None
        self.n1 = self.n2 = True
        self.pre_vacancy = {}
        self.greedy_timed_out = False
        self.ghost = True
        self.cert_format = 'r22b10/2'
        # Fields merged into the builder's record, the builder's log rows and its measured wall time.
        self.certificate = {}
        self.cert_log = []
        self.cert_wall = 0.0
        self.emitted = None
        self.v_length = 2
        self.v = [[9, 0], [8, 0]]
        self.full = None
        # Script name to (exit code, seconds, log): that child ends so and writes nothing else.
        self.exits = {}

    def units(self, table, work):
        return fake_units(self.state, work)

    def script(self, path, argv, tag, **kwargs):
        name = Path(path).name
        self.calls.append(name)
        self.argv[name] = list(argv)
        log = self.out / 'logs' / (tag.replace('/', '-') + '.log')
        rows = []
        if name in self.exits:
            rc, seconds, text = self.exits[name]
            log.write_text(text)
            return R.Child(rc, seconds, log)
        if name == 'cellpure5.py':
            out = Path(argv[3])
            piece = R.read_json(argv[2])['pieces'][0]
            R.atomic_json(out.with_name(out.stem + '-word.json'), {'word': [[piece, 1]]})
        elif name == 'transport_census.py':
            R.atomic_json(argv[1], {'units': {Path(argv[2]).stem: {}}})
        elif name == 'tclose2.py':
            out = Path(argv[5])
            target = [10, 11]
            pre = {'vacancy': self.pre_vacancy}
            if self.ghost:
                pre['ghost'] = {'T_on_lattice_at_end': 1, 'T_size': 1, 'protected_exact_at_end': True,
                                'badW': target, 'badW_digest': R.word_sha256(target)}
            if self.greedy_timed_out:
                pre['greedy'] = {'timed_out': True}
            R.atomic_json(out / 'pre.json', pre)
            R.atomic_json(out / 'result.json', {'result': 'no V'})
            if self.full is not None:
                R.atomic_json(out / 'word-full.json', {'word': self.full})
        elif name == 'jordan_target.py':
            if self.fault in ('jordan cut', 'jordan exit 124', 'jordan silent', 'jordan killed', 'jordan killed early'):
                # jordan_target's own CPU cap can stop it before any CONDITION line; its hard limit SIGKILLs.
                log.write_text('CUT_SHORT_CPU_840_SECONDS\n' if self.fault == 'jordan cut' else '')
                rc = {'jordan silent': 0, 'jordan killed': -signal.SIGKILL,
                      'jordan killed early': -signal.SIGKILL}.get(self.fault, 124)
                return R.Child(rc, {'jordan killed': 850.0, 'jordan killed early': 5.0}.get(self.fault, 0.0), log)
            log.write_text('CONDITION ' + json.dumps({'fixed_vacant': self.n1, 'necessary': self.n2,
                                                     'necessary_deficits': {'2': [2, 1]}}) + '\n')
            return R.Child(124 if self.fault == 'jordan CPU limit' else 0, 0.0, log)
        elif name == 'r22b10_cert.py':
            self.assert_seed_args = argv[-2:]
            if self.emitted is not None:
                # A record and log that the unchanged builder wrote (see emitted_certificate).
                evidence, text = self.emitted
                R.atomic_json(argv[3], evidence)
                log.write_text(text)
                return R.Child(0, self.cert_wall, log)
            evidence = {'format': self.cert_format, 'complete': self.fault != 'incomplete',
                'V_length': self.v_length,
                'state_digest': 'a' * 64 if self.fault == 'certificate digest' else R.read_json(argv[1])['digest']}
            evidence.update(self.certificate)
            R.atomic_json(argv[3], evidence)
            rows = self.cert_log
            log.write_text(''.join(json.dumps(row) + '\n' for row in rows))
            return R.Child(0, self.cert_wall, log)
        elif name == 'check_r22b10.py':
            # A rejection exits 1 with a last line 'FAIL: <reason>' (check_r22b10.py:483-485).
            text = {'check': 'FAIL: V_length differs from expanded length\n',
                    'check digest': 'FAIL: state digest mismatch\n',
                    'check truncated': 'PASS extra\n'}.get(self.fault, 'checked PASS\n')
            log.write_text(text)
            return R.Child(1 if text.startswith('FAIL') else 0, 0.0, log)
        elif name == 'expand_slp.py':
            if self.fault == 'expansion over limit':
                log.write_text(json.dumps({'over_limit': True, 'length': str(self.v_length)}) + '\n')
                return R.Child(3, 0.0, log)
            R.atomic_json(argv[1], {'word': self.v[:-1] if self.fault == 'expanded word length' else self.v,
                                   'input_digest': R.read_json(argv[0])['state_digest']})
            rows = [{'letters': len(self.v) + (self.fault == 'expand'), 'word_sha256': R.word_sha256(self.v)}]
        elif name == 'slp_apply.py':
            state = self.m.load(argv[1])
            before = self.m.snapshot(state)
            pieces = R.read_json(argv[-1])['pieces']
            state.off.difference_update(pieces)
            self.m.export(state, argv[3])
            for i, segment in enumerate(R.read_json(argv[2])['segments']):
                kind, wf = next(iter(segment.items()))
                word = R.read_json(wf)['word']
                row = {'segment': i, 'kind': kind, 'length': len(word), 'digest': state.digest(), 'off': len(state.off)}
                if kind == 'safe_word_file':
                    inverse = Path(wf).stem == 'V_inverse'
                    if self.fault != 'missing hash' and not (inverse and self.fault == 'inverse missing hash'):
                        wrong_hash = self.fault == 'hash' or (inverse and self.fault == 'inverse hash')
                        row['word_sha256'] = 'a' * 64 if wrong_hash else R.word_sha256(word)
                rows.append(row)
            if self.fault == 'segment length':
                rows[0]['length'] += 1
            elif self.fault == 'segment count':
                rows.pop()
            elif self.fault == 'segment digest':
                rows[0]['digest'] = 'z' * 64
            elif self.fault == 'final segment digest':
                rows[-1]['digest'] = 'a' * 64
            if self.fault == 'strict summary':
                rows.append({'strict_prime': False})
            elif self.fault != 'summary':
                rows.append({'start_digest': 'a' * 64 if self.fault in ('start digest', 'strict start digest')
                                            else before['digest'],
                    'end_digest': 'a' * 64 if self.fault == 'end digest' else state.digest(),
                    'strict_prime': self.fault not in ('strict', 'strict start digest'),
                    'unit_size': len(pieces), 'unit_on_lattice': len(pieces), 'collateral': 0,
                    'others': len(state.off), 'others_exact': len(state.off), 'others_landed': 0,
                    'off_before': before['off'], 'off_after': len(state.off)})
        elif name == 'stepchk.py':
            # Lands piece 1: a valid literal closure in the contract tests, or unit 1's word.
            state = self.m.load(argv[0])
            before = self.m.snapshot(state)
            state.off.remove(1)
            self.m.export(state, argv[4])
            R.atomic_json(argv[3], {'start_digest': before['digest'],
                'end_digest': 'a' * 64 if self.fault == 'check end digest' else state.digest(),
                'strict_prime': True, 'unit_size': 1, 'unit_on_lattice': 1, 'collateral': 0,
                'others': 1, 'others_exact': 1, 'others_landed': 0, 'off_before': 2, 'off_after': 1})
        elif name == 'fallback.py':
            piece = R.read_json(argv[3])['pieces'][0]
            R.atomic_json(Path(argv[4]) / 'word-full.json', {'word': [[piece, 1]]})
        else:
            raise AssertionError(name)
        log.write_text(''.join(json.dumps(row) + '\n' for row in rows))
        return R.Child(0, 0.0, log)


class CertificateTests(TemporaryTest):
    def setup_runner(self, *extra):
        runner = CertificateStub(self.args('run', '--tail-rules', 'close', *extra))
        runner.initialize()
        target = runner.out / 'units/u.json'
        R.atomic_json(target, {'pieces': [1]})
        return runner, R.Unit(1, 1, target)

    def trial(self, runner, unit):
        return runner.try_unit(runner.table, unit, runner.out / 'attempts/trial')

    def test_complete_certificate_formats_dispatch_and_advance(self):
        for fmt, seeds in (('r22b10/1', 'default'), ('r22b10/2', 'atlas')):
            with self.subTest(fmt=fmt):
                runner = CertificateStub(self.args(seeds, '--seeds', seeds))
                runner.cert_format = fmt
                runner.initialize()
                unit = R.Unit(1, 1, runner.out / 'units/u.json')
                R.atomic_json(unit.target, {'pieces': [1]})
                candidate = runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/close')
                self.assertEqual(candidate.gate, 'slp_apply')
                self.assertEqual(candidate.parts, [['V', 2], ['middle', 1], ['V_inverse', 2]])
                self.assertEqual(candidate.metadata['V_source'], 'certificate')
                self.assertEqual(runner.assert_seed_args, ['--seeds', seeds])
                with patch.object(runner.m, 'apply_word', side_effect=AssertionError('must load the gated end table')):
                    runner.advance(unit, candidate, [], 0.0)
                check_record(self, runner.out, runner.record)
                # The ordinary middle has no word_sha256 in the stub's segment row.
                rows = R.json_rows(runner.out / 'logs/attempts-close-whole.log')
                self.assertNotIn('word_sha256', rows[1])

    def test_strict_landing_t1prime_has_only_w_parts(self):
        runner, unit = self.setup_runner()
        wfile = runner.out / 'attempts/W.json'
        R.atomic_json(wfile, {'word': [[1, 1]]})
        candidate = runner.t1prime(runner.table, unit, wfile, runner.out / 'attempts/t1')
        self.assertEqual(candidate.parts, [['W', 1]])
        self.assertEqual(candidate.gate, 'stepchk')
        self.assertEqual(candidate.metadata, {})
        self.assertEqual(runner.calls, ['stepchk.py'])
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)

    def test_literal_limit_zero_never_builds_certificate(self):
        runner, unit = self.setup_runner('--literal-limit', '0')
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'no literal closure')
        self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_false_pre_vacancy_never_builds_certificate(self):
        runner, unit = self.setup_runner()
        for key in ('N1', 'N2'):
            with self.subTest(key=key):
                runner.calls.clear()
                runner.pre_vacancy = {key: False}
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertIs(trial.attempts[0][key], False)
                self.assertNotIn('jordan_target.py', runner.calls)
                self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_target_failing_n1_or_n2_is_recorded_without_certificate(self):
        runner, unit = self.setup_runner()
        for n1, n2 in ((False, True), (True, False)):
            with self.subTest(N1=n1, N2=n2):
                runner.calls.clear()
                runner.n1, runner.n2 = n1, n2
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertEqual((trial.attempts[0]['N1'], trial.attempts[0]['N2']), (n1, n2))
                self.assertEqual(trial.attempts[0]['T_size'], 2)
                self.assertIn('deficits', trial.attempts[0])
                self.assertNotIn('r22b10_cert.py', runner.calls)

    def test_failed_certificate_gate_is_an_attempt_and_bad_evidence_an_error(self):
        runner, unit = self.setup_runner()
        # The checker's rejection, an incomplete certificate and a false strict_prime are rule outcomes.
        for fault in ('check', 'incomplete', 'strict'):
            with self.subTest(fault=fault):
                runner.fault = fault
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                self.assertEqual(trial.attempts[0]['T_size'], 2)
                self.assertIs(trial.attempts[0]['timed_out'], False)
                if fault == 'strict':
                    self.assertEqual(trial.attempts[0]['outcome'], 'not strict_prime')
                self.assertEqual(runner.record['steps'], [])
        # Digest, hash and length mismatches, a rejection for a digest and truncated output are
        # integrity failures: they leave the rule, and the run ends error (plan, result table).
        for fault in ('check digest', 'check truncated', 'certificate digest', 'expand', 'expanded word length',
                      'hash', 'missing hash', 'inverse hash', 'inverse missing hash', 'segment length',
                      'segment count', 'segment digest', 'final segment digest', 'start digest', 'end digest',
                      'summary', 'strict start digest', 'strict summary'):
            with self.subTest(fault=fault):
                runner.fault = fault
                with self.assertRaises(R.RunError):
                    self.trial(runner, unit)
                self.assertEqual(runner.record['steps'], [])

    def test_jordan_condition_survives_its_later_cpu_limit(self):
        runner, unit = self.setup_runner()
        runner.fault = 'jordan CPU limit'
        trial = self.trial(runner, unit)
        self.assertIsNotNone(trial.candidate)
        self.assertEqual(trial.candidate.gate, 'slp_apply')

    def test_expansion_limit_exit_is_a_failed_attempt(self):
        runner, unit = self.setup_runner()
        runner.fault = 'expansion over limit'
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'over literal limit')
        self.assertEqual(trial.attempts[0]['V_length'], str(runner.v_length))
        self.assertNotIn('slp_apply.py', runner.calls)

    def test_over_limit_records_exact_decimal_without_expanding(self):
        runner, unit = self.setup_runner('--literal-limit', '5')
        runner.v_length = 10 ** 180
        trial = self.trial(runner, unit)
        self.assertIsNone(trial.candidate)
        self.assertEqual(trial.attempts[0]['outcome'], 'over literal limit')
        self.assertEqual(trial.attempts[0]['V_length'], str(10 ** 180))
        self.assertNotIn('expand_slp.py', runner.calls)
        # W contributes to the whole-step preflight, even though it is outside V W_inverse V_inverse.
        runner.v_length = 2
        with self.assertRaises(R.RuleFailure) as raised:
            runner.closure(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/prefix', prefix_length=1)
        self.assertEqual(raised.exception.outcome, 'over literal limit')
        self.assertNotIn('expand_slp.py', runner.calls)

    def test_tclose2_parts_are_checked_against_core_and_inverse(self):
        runner, unit = self.setup_runner()
        runner.full = [[9, 0], [1, 1], [9, 0]]
        candidate = runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/good')
        self.assertEqual(candidate.metadata['V_source'], 'tclose2')
        self.assertEqual(candidate.gate, 'stepchk')
        # tclose2 always writes V P Q P^-1 V^-1 (tclose2.py:203, :240-242): any other word is malformed output.
        for word in ([[9, 0], [2, 1], [9, 0]], [[9, 0], [1, 1], [8, 0]], [[1, 1], [9, 0]]):
            with self.subTest(word=word):
                runner.full = word
                with self.assertRaises(R.RunError):
                    runner.close(runner.table, unit, {'P': [], 'Q': [[1, 1]]}, runner.out / 'attempts/bad')

    def full_run(self, runner):
        with bulk_finds_nothing(runner):
            record = runner.run()
        check_record(self, runner.out, record)
        return record

    def test_acceptance_plays_a_complete_certificate_closure(self):
        # A2: the default stub's closure V·middle·V⁻¹ (5 letters, below --literal-limit) is played.
        runner = CertificateStub(self.args('run', '--tail-rules', 'close', '--mode', 'acceptance'))
        loaded, played = [], []
        load, apply_word = runner.m.load, runner.m.apply_word

        def recording_load(path):
            loaded.append(Path(path).name)
            return load(path)

        def recording_apply(state, doc):
            played.append(len(doc['word']))
            apply_word(state, doc)

        with patch.object(runner.m, 'load', side_effect=recording_load), \
                patch.object(runner.m, 'apply_word', side_effect=recording_apply):
            record = self.full_run(runner)
        self.assertEqual(record['result'], 'solved')
        self.assertEqual(played, [5, 5])
        # The slp_apply gate still checks the contract; its end table is never loaded.
        self.assertIn('slp_apply.py', runner.calls)
        self.assertIn('expand_slp.py', runner.calls)
        self.assertNotIn('end-table.json', loaded)
        for step in record['steps']:
            self.assertEqual((step['gate'], step['standing'], step['construction']), ('slp_apply', 'rule', 'certificate'))
            self.assertEqual((step['V_source'], step['letters']), ('certificate', 5))
        self.assertEqual(record['acceptance']['certificate_steps'], 0)
        self.assertEqual(record['acceptance']['verdict'], 'solved')
        # A replay check of the recorded words. The checker's own replay of a certificate-built
        # closure needs real geometry: the b20 step of plan I1.
        replay(self, runner.out, record)

    def test_acceptance_without_a_literal_word_is_certificate_needed(self):
        cases = {'over limit': (('--literal-limit', '4'), None, {}),
                 'incomplete': ((), 'incomplete', {'missing': [{'least': 0, 'reason': SEED_REASON}]}),
                 'check': ((), 'check', {})}
        for name, (extra, fault, certificate) in cases.items():
            with self.subTest(name=name):
                runner, unit = self.setup_runner('--mode', 'acceptance', *extra)
                runner.fault, runner.certificate = fault, certificate
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                attempt = trial.attempts[0]
                self.assertEqual((attempt['outcome'], attempt['timed_out']), ('certificate_needed', False))
                self.assertEqual((attempt['T_size'], attempt['T_digest']), (2, R.word_sha256([10, 11])))
                if name == 'over limit':
                    self.assertEqual(attempt['V_length'], '2')        # 2 + 1 + 2 > 4
                    self.assertNotIn('expand_slp.py', runner.calls)

    def test_development_keeps_the_gated_end_table(self):
        runner = CertificateStub(self.args('run', '--tail-rules', 'close'))
        loaded = []
        load = runner.m.load
        with patch.object(runner.m, 'load', side_effect=lambda path: loaded.append(Path(path).name) or load(path)), \
                patch.object(runner.m, 'apply_word', side_effect=AssertionError('must load the gated end table')):
            record = self.full_run(runner)
        self.assertEqual(loaded.count('end-table.json'), 2)
        self.assertEqual([s['standing'] for s in record['steps']], ['certificate', 'certificate'])
        self.assertEqual(record['acceptance']['certificate_steps'], 2)
        # A certificate step makes the verdict stuck even when the run reaches solved.
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('solved', 'stuck'))


def emitted_certificate(directory, seconds, serialization_deadline):
    """Run the unchanged r22b10_cert.main in memory (plan section 0, test 6, fixtures 7 and 8).

    Its loaders are stubbed, build finds no orbit and leaves one non-deadline reason, and the
    module's clock is a fake. With ``serialization_deadline`` the first write moves the clock past
    ``finish`` and raises Deadline, so main writes its serialization fallback. Returns the record
    and the log the builder printed. The builder arms SIGALRM, so this runs in the main thread."""
    spec = importlib.util.spec_from_file_location('r22b10_cert_fixture', ROOT / R.LOW / 'r22b10_cert.py')
    builder = importlib.util.module_from_spec(spec)
    digits = sys.get_int_max_str_digits()
    try:
        spec.loader.exec_module(builder)
    finally:
        sys.set_int_max_str_digits(digits)      # the builder lifts the limit for its own process
    clock = SimpleNamespace(now=1000.0)
    clock.monotonic = lambda: clock.now
    write, writes = builder.write_result, []

    def build(G, target, **kwargs):
        kwargs['result'].update(complete=False, missing=[{'least': 0, 'reason': SEED_REASON}], orbits=[], steps=[])
        return kwargs['result']

    def write_result(path, result):
        writes.append(path)
        if serialization_deadline and len(writes) == 1:
            clock.now = builder.T0 + seconds
            raise builder.Deadline('wall-clock budget exhausted')
        write(path, result)

    target, out = directory / 'target.json', directory / 'certificate.json.gz'
    R.atomic_json(target, {'T': [0], 'T_digest': R.word_sha256([0]), 'state_digest': 'a' * 64})
    state = SimpleNamespace(digest=lambda: 'a' * 64, pose_id=[0])
    loaders = {'screen': SimpleNamespace(load=lambda mode, path: (state, None, None)),
               'buffer': SimpleNamespace(Alphabet=lambda st, M: SimpleNamespace())}
    log = io.StringIO()
    with patch.multiple(builder, time=clock, T0=clock.now, modules=lambda: None, target_check=lambda *a: None,
                        build=build, write_result=write_result), patch.dict(sys.modules, loaders), redirect_stdout(log):
        rc = builder.main(['table', str(directory / 'state.json'), str(target), str(out),
                           '--seconds', repr(seconds), '--seeds', 'atlas'])
    assert rc == 0 and len(writes) == (2 if serialization_deadline else 1)
    return R.read_json(out), log.getvalue()


class ClockTests(TemporaryTest):
    """The clock tests 1, 4, 5 and 6 of plan section 0 (I0's runner side)."""
    def setup_runner(self, mode, *extra):
        runner = CertificateStub(self.args(mode, '--tail-rules', 'close', '--mode', mode, *extra))
        runner.initialize()
        target = runner.out / 'units/u.json'
        R.atomic_json(target, {'pieces': [1]})
        return runner, R.Unit(1, 1, target)

    def trial(self, runner, unit):
        return runner.try_unit(runner.table, unit, runner.out / 'attempts/trial')

    def full_run(self, runner):
        with bulk_finds_nothing(runner):
            record = runner.run()
        check_record(self, runner.out, record)
        return record

    def test_1_adapter_gives_the_sentinel_in_acceptance_mode(self):
        self.assertEqual(api.CLOCK_SENTINEL, 1e9)
        for value in (0.5, 60, 3600.0):
            self.assertEqual(api.child_clock('acceptance', value), api.CLOCK_SENTINEL)
            self.assertEqual(api.child_clock('development', value), value)
        self.assertEqual(api.child_clock('development', None), api.CLOCK_SENTINEL)
        with self.assertRaises(ValueError):
            api.child_clock('fast', 1)
        values = {'greedy_seconds': 7, 'cert_seconds': 11, 'height_cpu': 13, 'height_seconds': 17,
                  'kernel_seconds': 19, 'rule_seconds': 23, 'bulk_step_seconds': 29, 'bulk_seconds': 31}
        extra = [x for name, value in values.items() for x in ('--' + name.replace('_', '-'), str(value))]
        for mode in api.MODES:
            with self.subTest(mode=mode):
                lifted = mode == 'acceptance'
                runner, unit = self.setup_runner(mode, *extra)
                config = runner.record['config']
                check_record(self, runner.out, runner.record)
                for name, value in values.items():
                    self.assertEqual(config['budgets'][name], None if lifted else value, name)
                self.assertEqual(config['clock_sentinel'], api.CLOCK_SENTINEL if lifted else None)
                self.assertEqual(config['kernel']['seconds'], None if lifted else 19)
                # Plugins: a child clock is the sentinel, and the runner's own wall clock is off.
                ctx = runner.context('height', 'r22d81')
                for name in ('height_cpu', 'height_seconds', 'kernel_seconds', 'greedy_seconds', 'cert_seconds'):
                    self.assertEqual(ctx.child_clock(name), api.CLOCK_SENTINEL if lifted else values[name])
                    self.assertEqual(ctx.wall(name), None if lifted else values[name])
                # The runner's own child clock arguments.
                self.assertIsNotNone(self.trial(runner, unit).candidate)
                self.assertEqual(runner.argv['tclose2.py'][6:8], ['--greedy-seconds', api.CLOCK_SENTINEL if lifted else 7])
                self.assertEqual(runner.argv['r22b10_cert.py'][4:6], ['--seconds', api.CLOCK_SENTINEL if lifted else 11])
                # And the runner's own rule clock.
                with patch.object(runner, 'execute') as execute:
                    R.Runner.script(runner, R.GM / 'fallback.py', [], 'clock')
                self.assertEqual(execute.call_args.args[2], None if lifted else 23)

    def test_4_greedy_clock_is_a_timed_out_attempt(self):
        # Implemented as the I0 brief allows: the tclose2 stub reports that its greedy vacancy search
        # stopped on its clock (pre.greedy.timed_out), and the attempt records timed_out. The child's
        # clock itself is not mocked; acceptance passes it the sentinel and development the value.
        for mode in api.MODES:
            with self.subTest(mode=mode):
                runner, unit = self.setup_runner(mode, '--greedy-seconds', '5')
                runner.greedy_timed_out, runner.ghost = True, False
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                attempt = trial.attempts[0]
                self.assertEqual((attempt['outcome'], attempt['timed_out']), ('no literal closure', True))
                self.assertEqual(runner.argv['tclose2.py'][7], api.CLOCK_SENTINEL if mode == 'acceptance' else 5)
                # Without the greedy timeout the same failure is not timed out.
                runner.greedy_timed_out = False
                self.assertIs(self.trial(runner, unit).attempts[0]['timed_out'], False)
        runner = CertificateStub(self.args('run', '--tail-rules', 'close', '--mode', 'acceptance'))
        runner.greedy_timed_out, runner.ghost = True, False
        record = self.full_run(runner)
        self.assertEqual((record['phases']['tail']['result'], record['result']), ('timeout', 'budget'))
        self.assertEqual(record['acceptance']['verdict'], 'budget')

    def test_5_jordan_cpu_cap_before_condition_is_a_timed_out_attempt(self):
        for fault in ('jordan cut', 'jordan exit 124', 'jordan killed'):
            with self.subTest(fault=fault):
                runner, unit = self.setup_runner('acceptance')
                runner.fault = fault
                trial = self.trial(runner, unit)
                self.assertIsNone(trial.candidate)
                attempt = trial.attempts[0]
                self.assertEqual((attempt['outcome'], attempt['timed_out']), ('timeout', True))
                self.assertNotIn('r22b10_cert.py', runner.calls)
        # Exit 0 without CONDITION is truncated output; a SIGKILL before the cap's 840 s is a crash.
        for fault in ('jordan silent', 'jordan killed early'):
            with self.subTest(fault=fault):
                runner, unit = self.setup_runner('acceptance')
                runner.fault = fault
                with self.assertRaises(R.RunError):
                    self.trial(runner, unit)
                self.assertNotIn('r22b10_cert.py', runner.calls)
        runner = CertificateStub(self.args('run', '--tail-rules', 'close', '--mode', 'acceptance'))
        runner.fault = 'jordan cut'
        record = self.full_run(runner)
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('budget', 'budget'))
        self.assertEqual(record['acceptance']['timed_out_attempts'], 2)

    def certificate_run(self, name, certificate=None, log=(), wall=0.0, emitted=None):
        runner = CertificateStub(self.args(name.replace(' ', '-'), '--tail-rules', 'close', '--mode', 'acceptance'))
        runner.fault = None if emitted else 'incomplete'
        runner.certificate, runner.cert_log, runner.cert_wall = certificate or {}, list(log), wall
        runner.emitted = emitted
        record = self.full_run(runner)
        self.assertEqual(runner.argv['r22b10_cert.py'][4:6], ['--seconds', api.CLOCK_SENTINEL])
        attempts = [a for unit in record['stuck']['units'] for a in unit['attempts'] if a['rule'] == 'close'
                    and a['reason'].startswith('pure:')]
        self.assertEqual(len(attempts), 2)
        return record, attempts

    def test_6_certificate_deadlines(self):
        deadline = 'seed: wall-clock budget exhausted'
        timed_out = {
            'per-orbit': ({'missing': [{'least': 0, 'reason': deadline}]}, ()),
            'serialization': ({'missing': [{'least': 0, 'reason':
                               'serialization budget exhausted; constructed program omitted'}]}, ()),
            'mixed': ({'missing': [{'least': 0, 'reason': deadline}, {'least': 5, 'reason': SEED_REASON}]}, ()),
            'timeout row': ({'missing': [{'least': 0, 'reason': SEED_REASON}]}, ({'timeout': True, 'phase': 'x'},)),
            'recorded seconds': ({'missing': [{'least': 0, 'reason': SEED_REASON}], 'seconds': 1e9 - 5}, ())}
        for name, (certificate, log) in timed_out.items():
            with self.subTest(name=name):
                record, attempts = self.certificate_run(name, certificate, log)
                self.assertEqual({(a['outcome'], a['timed_out']) for a in attempts}, {('timeout', True)})
                self.assertEqual((record['phases']['tail']['result'], record['result']), ('timeout', 'budget'))
                self.assertEqual(record['acceptance']['verdict'], 'budget')
        # No seconds in the record: the runner's measured wall time of the child is used.
        record, attempts = self.certificate_run('measured', {'missing': [{'least': 0, 'reason': SEED_REASON}]},
                                                wall=api.CLOCK_SENTINEL)
        self.assertEqual({a['timed_out'] for a in attempts}, {True})
        not_timed_out = {
            'seed reason': {'missing': [{'least': 0, 'reason': SEED_REASON}], 'seconds': 12.5},
            'size cap': {'missing': [{'least': 0, 'reason':
                         'word tables: word-table generator memory budget reached before connectivity'}]}}
        for name, certificate in not_timed_out.items():
            with self.subTest(name=name):
                record, attempts = self.certificate_run(name, certificate)
                self.assertEqual({(a['outcome'], a['timed_out']) for a in attempts}, {('certificate_needed', False)})
                self.assertEqual((record['phases']['tail']['result'], record['result']), ('stuck', 'stuck'))
                self.assertEqual(record['acceptance']['verdict'], 'stuck')

    def test_6_emitted_certificates(self):
        # Fixtures 7 and 8: the unchanged builder writes them with the runner's acceptance --seconds.
        for expired in (True, False):
            with self.subTest(serialization_deadline=expired):
                directory = self.tmp / f'emitter-{expired}'
                directory.mkdir()
                evidence, text = emitted_certificate(directory, api.CLOCK_SENTINEL, expired)
                rows = [json.loads(line) for line in text.splitlines()]
                self.assertEqual(evidence['missing'], [{'least': 0, 'reason': SEED_REASON}])
                self.assertIs(evidence['complete'], False)
                self.assertFalse(any(row.get('timeout') for row in rows))
                self.assertEqual(evidence['seconds'] >= api.CLOCK_SENTINEL - 10, expired)
                self.assertIs(R.cert_timed_out(rows, evidence, api.CLOCK_SENTINEL, 0.0), expired)
                record, attempts = self.certificate_run(f'emitted-{expired}', emitted=(evidence, text))
                outcome = ('timeout', True) if expired else ('certificate_needed', False)
                self.assertEqual({(a['outcome'], a['timed_out']) for a in attempts}, {outcome})
                self.assertEqual(record['acceptance']['verdict'], 'budget' if expired else 'stuck')


def crash_log(directory, script, line, statement='assert False'):
    """The log of a child that ``statement`` at ``line`` of ``script`` ended, as CPython prints it when
    the adapter's runpy runs the script; the output the script had buffered follows the traceback."""
    path = directory / script
    path.write_text('print("progress")\n' + '\n' * (line - 2) + statement + '\n')
    log = directory / (script + '.log')
    with log.open('w') as f:
        rc = subprocess.run([sys.executable, '-I', '-B', '-c', 'import runpy, sys; '
                             'runpy.run_path(sys.argv[1], run_name="__main__")', str(path)],
                            stdout=f, stderr=subprocess.STDOUT).returncode
    assert rc == 1
    return log.read_text()


class IntegrityTests(TemporaryTest):
    """I0-01: a crash, truncated or malformed child output, or a digest mismatch ends the run error,
    and no later rule, unit or phase runs (plan, result table). A rule's own no-word outcome stays a
    failed attempt, and the next rule or unit is tried."""
    def run_stub(self, name, *extra, exits=None, fault=None):
        runner = CertificateStub(self.args(name, *extra))
        runner.exits, runner.fault = dict(exits or {}), fault
        lattice = []
        with bulk_finds_nothing(runner), patch.object(runner, 'lattice', side_effect=lambda: lattice.append(1)):
            record = runner.run()
        check_record(self, runner.out, record)
        return runner, record, lattice

    def assert_error(self, record, lattice, message):
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('error', 'error'))
        self.assertIn(message, record['error'])
        self.assertEqual((record['steps'], record['stuck']), ([], None))
        self.assertIsNone(record['phases']['tail']['result'])
        self.assertIsNone(record['phases']['lattice']['result'])
        self.assertEqual(lattice, [], 'no later phase runs after an error')

    def test_a_crashing_checker_ends_the_run_error(self):
        # stepchk's own digest assert (stepchk.py:43) fails with a traceback and exit 1.
        crash = crash_log(self.tmp, 'stepchk.py', 43, "assert 'a' == 'b', 'not a word at this state'")
        for mode, par in (('development', 1), ('development', 2), ('acceptance', 1)):
            with self.subTest(mode=mode, par=par):
                runner, record, lattice = self.run_stub(
                    f'{mode}-{par}', '--tail-rules', 'zero_bad,pure,close', '--mode', mode, '--par', str(par),
                    '--fallback-max-length', '1', exits={'stepchk.py': (1, 0.0, crash)})
                self.assert_error(record, lattice, 'child exit 1')
                # zero_bad's no word was an attempt; pure's crashed check ends the run before close or the
                # fallback. A later unit's trial may have started, but it is never consumed or recorded.
                self.assertTrue({'transport_census.py', 'cellpure5.py', 'stepchk.py'} <= set(runner.calls))
                self.assertTrue({'tclose2.py', 'fallback.py'}.isdisjoint(runner.calls))
        # A failed assert of cellpure5 that is not an applicability screen is a crash too.
        crash = crash_log(self.tmp, 'cellpure5.py', 349, 'assert False, "home point"')
        runner, record, lattice = self.run_stub('invariant', '--tail-rules', 'pure,close',
                                                exits={'cellpure5.py': (1, 0.0, crash)})
        self.assert_error(record, lattice, 'child exit 1')
        self.assertEqual(set(runner.calls), {'cellpure5.py'})

    def test_b_mismatched_check_digest_ends_the_run_error(self):
        cases = {'pure rule': ((), 'check end digest', 'stepchk digest mismatch'),
                 'certificate gate': (('--tail-rules', 'close'), 'end digest', 'slp_apply digest mismatch'),
                 # A false strict_prime does not hide a digest mismatch (scoped verification, I0-01).
                 'certificate gate not strict prime': (('--tail-rules', 'close'), 'strict start digest',
                                                       'slp_apply digest mismatch'),
                 'fallback': (('--tail-rules', 'zero_bad', '--fallback-max-length', '1'), 'check end digest',
                              'stepchk digest mismatch'),
                 'probe': (('--tail-rules', 'zero_bad', '--fallback-max-length', '1', '--mode', 'acceptance'),
                           'check end digest', 'stepchk digest mismatch')}
        for name, (extra, fault, message) in cases.items():
            with self.subTest(name=name):
                extra = extra or ('--tail-rules', 'zero_bad,pure,close')
                runner, record, lattice = self.run_stub(name.replace(' ', '-'), *extra, fault=fault)
                self.assert_error(record, lattice, message)
                self.assertIsNone(record['acceptance']['probe'])
                if name == 'pure rule':
                    self.assertNotIn('tclose2.py', runner.calls)
                if name.startswith('certificate gate'):
                    # The transport seed source of close is not tried.
                    self.assertIn('slp_apply.py', runner.calls)
                    self.assertNotIn('transport_census.py', runner.calls)

    def test_c_no_word_stays_recoverable(self):
        screen = crash_log(self.tmp, 'cellpure5.py', 191, 'assert len([(0, 1), (0, 2)]) == 1, [(0, 1), (0, 2)]')
        self.assertEqual(R.failed_assert(self.tmp / 'cellpure5.py.log'), ('cellpure5.py', 191))
        self.assertTrue(screen.rstrip().endswith('progress'))
        for name, exit_code, text, outcome in (('no word', 0, '', 'no word'),
                                               ('applicability screen', 1, screen, 'not applicable')):
            with self.subTest(name=name):
                runner, record, _ = self.run_stub(name.replace(' ', '-'), '--tail-rules', 'zero_bad,pure,close',
                                                  exits={'cellpure5.py': (exit_code, 0.0, text)})
                self.assertEqual((record['result'], record['acceptance']['verdict']), ('stuck', 'stuck'))
                # Every rule was tried at every unit, the close rule from both seed sources.
                self.assertEqual([u['unit_key'] for u in record['stuck']['units']], [1, 2])
                for unit in record['stuck']['units']:
                    self.assertEqual([(a['rule'], a['outcome']) for a in unit['attempts']],
                                     [('zero_bad', 'no word'), ('pure', outcome), ('close', outcome),
                                      ('close', 'no word')])
                if exit_code:
                    self.assertEqual(record['stuck']['units'][0]['attempts'][1]['reason'],
                                     'applicability screen cellpure5.py:191')

    def test_an_error_leaves_choose_and_its_serial_retry(self):
        for where in ('parallel', 'serial retry'):
            with self.subTest(where=where):
                runner = StubRunner(self.args(where.replace(' ', '-'), '--par', '2'), plugins())
                original = runner.try_unit

                def trial(table, unit, work):
                    if unit.key == 1 and (where == 'parallel' or work.name.endswith('-serial')):
                        raise R.RunError('stepchk digest mismatch')
                    if unit.key == 1:
                        return R.Trial(attempts=[{'unit_key': 1, 'rule': 'pure', 'outcome': 'timeout',
                                                  'reason': 'busy machine', 'seconds': 0.1, 'timed_out': True}])
                    return original(table, unit, work)

                with bulk_finds_nothing(runner), patch.object(runner, 'try_unit', side_effect=trial):
                    record = runner.run()
                check_record(self, runner.out, record)
                # Unit 2's step is never taken: unit 1 comes first in a serial run.
                self.assertEqual((record['result'], record['error'], record['steps']),
                                 ('error', 'stepchk digest mismatch', []))
                self.assertEqual(record['acceptance']['verdict'], 'error')

    def test_a_later_unit_error_waits_for_the_earlier_serial_retry(self):
        # Unit 1 times out beside unit 2 and is tried again alone before unit 2's error is taken,
        # as --par 1 would (README section 1, rule 3), so the outcome does not depend on --par.
        for alone in ('step', 'no word'):
            outcomes = []
            for par in (1, 2):
                runner = StubRunner(self.args(f'{alone.replace(" ", "-")}-{par}', '--par', str(par),
                                              '--tail-steps', '1'), plugins())
                original = runner.try_unit

                def trial(table, unit, work, par=par, alone=alone, original=original):
                    if unit.key == 2:
                        raise R.RunError('stepchk digest mismatch')
                    if par > 1 and not work.name.endswith('-serial'):
                        return R.Trial(attempts=[{'unit_key': 1, 'rule': 'pure', 'outcome': 'timeout',
                                                  'reason': 'busy machine', 'seconds': 0.1, 'timed_out': True}])
                    if alone == 'no word':
                        return R.Trial(attempts=[{'unit_key': 1, 'rule': 'pure', 'outcome': 'no word',
                                                  'reason': 'stub exhausted', 'seconds': 0.0, 'timed_out': False}])
                    return original(table, unit, work)

                with bulk_finds_nothing(runner), patch.object(runner, 'try_unit', side_effect=trial):
                    record = runner.run()
                check_record(self, runner.out, record)
                outcomes.append((record['result'], record['acceptance']['verdict'], record['error'],
                                 [s['unit_key'] for s in record['steps']]))
            with self.subTest(alone=alone):
                self.assertEqual(outcomes[0], outcomes[1])
                self.assertEqual(outcomes[0], ('budget', 'budget', None, [1]) if alone == 'step' else
                                 ('error', 'error', 'stepchk digest mismatch', []))

    def test_an_error_after_a_tail_step_keeps_the_step(self):
        # The steps before the error stand; the tail keeps their count and has no result.
        runner = StubRunner(self.args('after-a-step', '--tail-steps', '0'), plugins())
        original = runner.try_unit

        def trial(table, unit, work):
            if runner.record['steps']:
                raise R.RunError('stepchk digest mismatch')
            return original(table, unit, work)

        with bulk_finds_nothing(runner), patch.object(runner, 'try_unit', side_effect=trial):
            record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual((record['result'], record['acceptance']['verdict'], record['error']),
                         ('error', 'error', 'stepchk digest mismatch'))
        self.assertEqual([s['unit_key'] for s in record['steps']], [1])
        self.assertEqual(record['phases']['tail'], {'result': None, 'steps': 1, 'twists': 1})
        self.assertIsNone(record['stuck'])


def bulk_stub(out, cache):
    """Commit one step, corrupt only the uncommitted table, then block with a child."""
    runner = R.Runner(type('Args', (), {'out': out, 'cache': cache})(), FakeMechanics())
    runner.record = R.read_json(out / 'run.json')
    runner.started -= runner.record['totals']['seconds']
    state = runner.m.load(out / 'start-table.json')
    for step in runner.record['steps']:
        runner.m.apply_word(state, R.read_json(out / step['word']))
    index = len(runner.record['steps'])
    piece = min(state.off)
    before = dict(runner.m.snapshot(state), N=len(state.off))
    del before['heights']
    state.off.remove(piece)
    after = dict(runner.m.snapshot(state), N=len(state.off))
    del after['heights']
    word = {'word': [[piece, 1]], 'input_digest': before['digest'], 'output_digest': after['digest']}
    word_file = f'words/{index:05d}.json'
    R.atomic_json(out / word_file, word)
    runner.table = out / 'tables/bulk-end.json'
    runner.m.export(state, runner.table)
    runner.record['steps'].append({'index': index, 'phase': 'bulk', 'subphase': None, 'rule': 'close',
        'word': word_file, 'letters': 1, 'encoding': 'json', 'word_sha256': R.word_sha256(word['word']),
        'tree': runner.write_tree(index, api.single('close', 1), 1), 'standing': 'rule',
        'before': before, 'after': after, 'seconds': 0.0})
    runner.record['phases']['bulk'].update(steps=index + 1, twists=index + 1,
        end=dict(runner.m.snapshot(state), table='tables/bulk-end.json'))
    if index:
        runner.record['phases']['bulk']['result'] = 'lattice'
        runner.save()
        return
    runner.save()
    # Recovery has to replay the checkpoint; loading this partial table would be wrong.
    runner.m.export(FakeState(()), runner.table)
    child = subprocess.Popen([sys.executable, '-I', '-B', '-c', 'import time; time.sleep(120)'])
    R.atomic_json(out / 'bulk-progress.json', {'step_started': time.monotonic()})
    R.atomic_json(out / 'stub-ready.json', {'bulk': os.getpid(), 'descendant': child.pid})
    time.sleep(120)


class BulkTests(TemporaryTest):
    def test_signal_cancels_rule_group_even_after_event_is_cleared(self):
        runner = StubRunner(self.args())
        runner.initialize()
        ready = self.tmp / 'rule-ready.json'
        code = ('import json, os, pathlib, subprocess, sys, time; '
                'child = subprocess.Popen([sys.executable, "-I", "-B", "-c", "import time; time.sleep(120)"]); '
                'pathlib.Path(sys.argv[1]).write_text(json.dumps([os.getpid(), child.pid])); time.sleep(120)')
        with R.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(runner.execute, [sys.executable, '-I', '-B', '-c', code, ready], 'rule-signal', 10)
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(ready.exists())
            runner.on_signal(signal.SIGTERM, None)
            runner.cancel.clear()  # A scheduler reset cannot erase the signal.
            child = future.result(timeout=5)
        self.assertTrue(child.cancelled)
        self.assertFalse(child.timeout)
        self.assert_processes_exited(R.read_json(ready))
        with self.assertRaisesRegex(R.RunSignal, 'signal 15'):
            runner.require(child)

    def test_bulk_timeout_replays_last_completed_close_step(self):
        runner = StubRunner(self.args('run', '--bulk-steps', '2', '--bulk-step-seconds', '0.3'))
        original_save = runner.save

        def save():
            original_save()
            check_record(self, runner.out, runner.record)

        with patch.object(R, 'SELF', Path(__file__)), patch.object(runner, 'tail'), patch.object(runner, 'save', side_effect=save):
            record = runner.run()
        self.assertNotEqual(record['result'], 'error', record['error'])
        self.assertEqual(record['phases']['bulk']['result'], 'timeout')
        self.assertEqual(record['phases']['bulk']['steps'], 1)
        self.assertEqual(record['totals']['bulk_twists'], 1)
        self.assertEqual(runner.m.off_set(runner.state), {2})
        self.assertEqual(runner.m.load(runner.table).off, {2})
        self.assertEqual(record['steps'][0]['rule'], 'close')
        self.assertTrue({'V_source', 'V_letters', 'V_sha256', 'parts', 'gate'}.isdisjoint(record['steps'][0]))
        # A bulk clock leaves a component run, and the later phases still run.
        self.assertEqual(record['config']['component'], 'bulk_budget')
        self.assertEqual(record['phases']['height']['result'], 'not_applicable')
        self.assertEqual(record['acceptance']['verdict'], 'component')
        self.assert_processes_exited(R.read_json(runner.out / 'stub-ready.json').values())

    def test_sigterm_and_sighup_preserve_child_checkpoint_and_kill_group(self):
        for number in (signal.SIGTERM, signal.SIGHUP):
            with self.subTest(signal=number):
                out = self.tmp / f'signal{number}'
                log = self.tmp / f'signal{number}.log'
                with log.open('w') as f:
                    proc = subprocess.Popen([sys.executable, '-I', '-B', str(Path(__file__)),
                        '--_signal-parent', str(out), str(self.tmp / 'cache')], cwd=ROOT,
                        stdout=f, stderr=subprocess.STDOUT, start_new_session=True)
                    try:
                        deadline = time.monotonic() + 15
                        while not (out / 'stub-ready.json').exists():
                            if proc.poll() is not None or time.monotonic() > deadline:
                                self.fail('stub did not reach committed checkpoint: ' + log.read_text())
                            time.sleep(0.02)
                        pids = R.read_json(out / 'stub-ready.json')
                        proc.send_signal(number)
                        self.assertEqual(proc.wait(timeout=10), 2, log.read_text())
                    finally:
                        R.kill_child(proc)
                record = R.read_json(out / 'run.json')
                check_record(self, out, record)
                self.assertEqual(record['result'], 'error')
                self.assertEqual(record['error'], f'signal {number}')
                self.assertEqual(record['acceptance']['verdict'], 'error')
                self.assertIsNone(record['config']['component'], 'an interrupted bulk has not ended on its budget')
                self.assertEqual(len(record['steps']), 1)
                self.assertEqual(record['totals']['bulk_twists'], 1)
                self.assertEqual(record['phases']['bulk']['twists'], 1)
                self.assertEqual(record['phases']['bulk']['steps'], 1)
                table = FakeMechanics().load(out / 'tables/bulk-end.json')
                self.assertEqual(table.off, {2})
                self.assertEqual(table.digest(), record['steps'][0]['after']['digest'])
                self.assert_processes_exited(pids.values())
                # Resume the actual signal checkpoint through the real parent/child pipeline.
                resumed = subprocess.run([sys.executable, '-I', '-B', str(Path(__file__)),
                    '--_signal-parent', str(out), str(self.tmp / 'cache'), '--resume'], cwd=ROOT,
                    capture_output=True, text=True, timeout=20, start_new_session=True)
                self.assertEqual(resumed.returncode, 0, resumed.stdout + resumed.stderr)
                continued = R.read_json(out / 'run.json')
                check_record(self, out, continued)
                self.assertEqual((continued['result'], continued['acceptance']['verdict']), ('solved', 'solved'))
                self.assertIsNone(continued['config']['component'])
                self.assertEqual([r['at_step'] for r in continued['resumes']], [1])
                self.assertEqual([R.read_json(out / s['word'])['word'] for s in continued['steps']],
                                 [[[1, 1]], [[2, 1]]])


class ExactTests(TemporaryTest):
    @classmethod
    def setUpClass(cls):
        cls.shared = Path(tempfile.mkdtemp(prefix='tutorial-exact-cache-'))
        cls.addClassCleanup(shutil.rmtree, cls.shared)
        environment = patch.dict(os.environ, JUMBLING_HOME_CACHE=str(cls.shared / 'home_centroids.npy'))
        environment.start()
        cls.addClassCleanup(environment.stop)
        cls.mechanics = R.Mechanics(cls.shared)
        cls.runs = None

    def invoke(self, *arguments):
        out = self.tmp / 'run'
        command = [sys.executable, '-I', '-B', HERE / 'runner.py', '--out', out,
                   '--cache', self.shared, *arguments]
        log = self.tmp / 'runner-cli.log'
        with log.open('w') as f:
            child = subprocess.Popen([str(x) for x in command], cwd=ROOT,
                stdout=f, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                child.wait(timeout=60 * 60)
            except subprocess.TimeoutExpired:
                # Let the runner cancel its separately owned child groups before
                # reaping it. The suite still enforces its total 60-minute budget.
                child.send_signal(signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    R.kill_child(child)
                raise
            finally:
                R.kill_child(child)
        detail = log.read_text()
        if (out / 'logs/runner.log').exists():
            detail += (out / 'logs/runner.log').read_text()
        self.assertEqual(child.returncode, 0, detail)
        record = R.read_json(out / 'run.json')
        check_record(self, out, record)
        return out, record

    def test_01_scramble_10_matches_census(self):
        out, record = self.invoke('--scramble', '10', '7001')
        self.assertEqual(record['result'], 'solved')
        self.assertEqual(record['totals']['bulk_twists'], 9)
        self.assertEqual(record['phases']['tail']['steps'], 0)
        self.assertEqual(record['phases']['tail']['result'], 'not_applicable')
        self.assertEqual(record['phases']['height']['result'], 'not_applicable')
        self.assertEqual(record['acceptance']['verdict'], 'solved')
        self.assertIsNone(record['lattice'])
        census = R.read_json(ROOT / R.J / 'restoration/11-scramble-census/runs/round1/ia-k10-s7001.json')
        self.assertEqual([(s['rule'], s['letters']) for s in record['steps']],
                         [(s['rule'], s['twists']) for s in census['steps']])
        self.assertEqual(record['phases']['bulk']['end']['digest'], R.SOLVED)
        self.assertEqual(record['phases']['bulk']['end']['off'], 0)
        state = self.mechanics.load(out / 'start-table.json')
        for step in record['steps']:
            self.mechanics.apply_word(state, R.read_json(out / step['word']))
        self.assertEqual(state.digest(), R.SOLVED)

    def lattice_word(self):
        """Retained letters that take the solved state to a small lattice configuration."""
        m = self.mechanics
        retained = sorted(set(range(len(m.menu.items))) - set(m.restore.jumble_items(m.menu)) - {0})
        self.assertTrue(retained)
        # A short nested commutator at three caps keeps this lattice fixture small.
        item = next(i for i in retained if m.restore.to_tuple(m.restore.matmul(
            m.restore.matmul(m.menu.items[i][1], m.menu.items[i][1]),
            m.menu.items[i][1])) == m.restore.I4)
        square = m.restore.to_tuple(m.restore.matmul(m.menu.items[item][1], m.menu.items[item][1]))
        inverse = next(i for i in retained if m.restore.to_tuple(m.menu.items[i][1]) == square)
        second = next(int(c) for c in m.restore.NEAR[0] if int(c) != 0)
        third = min((set(map(int, m.restore.NEAR[0])) & set(map(int, m.restore.NEAR[second]))) - {0, second})
        comm = [(0, item), (second, item), (0, inverse), (second, inverse)]
        comm_inverse = [(second, item), (0, item), (second, inverse), (0, inverse)]
        return comm + [(third, item)] + comm_inverse + [(third, inverse)]

    def test_02_small_lattice_configuration(self):
        m = self.mechanics
        word = self.lattice_word()
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        for cap, item in word:
            outcome = state.apply(m.sim.Twist(m.restore.ctx, cap, m.restore.element(m.menu, cap, item),
                                             'menu', {'menu': m.menu.name}))
            self.assertTrue(outcome.applied)
            self.assertFalse(outcome.uncertain)
        self.assertEqual(state.off_lattice_count(), 0)
        self.assertNotEqual(state.digest(), R.SOLVED)
        self.assertLess(int(m.np.count_nonzero(state.pose_id)), 1000)
        table = self.tmp / 'lattice-input.json'
        m.export(state, table)
        # This same retained commutator has zero charge, so the existing w9
        # entry suffices for its exact restore and path checks. Replaying all
        # unused correction macros would spend the certificate tests' budget.
        # The opt-in depth-20 CLI test uses the complete production library.
        library = self.tmp / 'fixture-library.json.gz'
        w9 = R.read_json(ROOT / R.LIBRARY)['macros'][0]
        self.assertEqual(w9['family'], 'w9')
        R.atomic_json(library, {'macros': [w9]})
        args = R.parse_args(['--table', str(table), '--out', str(self.tmp / 'run'),
                             '--cache', str(self.shared)])
        # A stub lattice plugin keeps the verify_late path (plan revision 3e, E2): a component run. The
        # real plugin's play of this state is 16,184,022 retained twists (test_lattice_phase.py).
        runner = R.Runner(args, m, plugins(lattice=stub_lattice()))
        with patch.object(R, 'LIBRARY', library):
            record = runner.run()
        out = runner.out
        check_record(self, out, record)
        self.assertEqual(record['result'], 'solved', record['error'])
        self.assertEqual(record['steps'], [])
        self.assertEqual(record['config']['component'], 'lattice_composition')
        self.assertEqual(record['acceptance']['verdict'], 'component')
        self.assertEqual(record['phases']['lattice']['result'], 'solved')
        self.assertEqual(record['phases']['tail']['result'], 'not_applicable')
        self.assertFalse(record['lattice']['chain'])
        self.assertEqual(record['lattice']['input'], 'table:tables/bulk-end.json')
        self.assertEqual(record['lattice']['restore_rc'], 0)
        self.assertEqual(record['lattice']['verify_rc'], 0)
        self.assertTrue(record['lattice']['verify_last_line'].startswith('PASS '))
        report = R.read_json(out / 'lattice/report.json')
        self.assertTrue(report['pass'])
        self.assertEqual(len(report['runs']), 1)
        self.assertTrue(report['runs'][0]['restored_to_identity'])
        summary = R.read_json(out / 'lattice/restore-summary.json')['runs'][0]
        self.assertTrue(all(not any(value) for value in summary['nu_input'].values()))
        for path in (out / 'run.json', out / 'lattice/restore-summary.json', out / 'lattice/report.json'):
            def no_absolute(value):
                if isinstance(value, dict):
                    for key, child in value.items():
                        no_absolute(key)
                        no_absolute(child)
                elif isinstance(value, list):
                    for child in value:
                        no_absolute(child)
                elif isinstance(value, str):
                    self.assertFalse(value.startswith('/'), value)
                    self.assertNotIn(str(ROOT), value)
                    self.assertNotIn(str(self.tmp), value)
            no_absolute(R.read_json(path))
        print(f'  lattice retained solve: {summary["seconds"]}s; checker: {report["seconds"]}s',
              file=sys.stderr, flush=True)

    def test_03_stored_literal_t1prime(self):
        base = ROOT / R.J / 'restoration/12-low-height/runs/round22/line-a'
        table = self.tmp / 'input.json'
        R.atomic_json(table, R.read_json(base / 'replay2/run1/z-b21.json.gz'))
        self.assertEqual(R.read_json(table)['digest'],
                         'a3fef11ccce359a8a8456d0a0a1e82bdf26f296e08a717464b9c7b9d089f21df')
        self.assertEqual(self.mechanics.load(table).off_lattice_count(), 437)
        args = R.parse_args([
            '--table', str(table), '--out', str(self.tmp / 'run'), '--cache', str(self.shared),
            '--greedy-seconds', '0'])
        runner = R.Runner(args, self.mechanics)
        runner.initialize()
        unit_file = base / 'units/eb5-k40-k00-b04.json'
        self.assertEqual(len(R.read_json(unit_file)['pieces']), 45)
        unit = R.Unit(2198, 45, unit_file)
        wfile = base / 'replay2/inputs/b04-cp4-W.json'
        self.assertEqual(len(R.read_json(wfile)['word']), 288)
        candidate = runner.t1prime(table, unit, wfile, runner.out / 'attempts/t1')
        self.assertTrue(candidate.check['strict_prime'])
        expected = json.loads((base / 'replay2/run2/seq.jsonl').read_text().splitlines()[0])
        self.assertEqual(len(candidate.word['word']), int(expected['letters']))
        self.assertEqual(len(candidate.word['word']), 1254)
        self.assertEqual(candidate.word['output_digest'], expected['end_digest'])
        self.assertEqual(candidate.word['output_digest'],
                         'fee07e63a8cf8672ae81cbdf3c1d13b9ed83e61de758a7f2e94b13d28dfe7167')
        self.assertEqual(candidate.parts, [['W', 288], ['V', 339], ['middle', 288], ['V_inverse', 339]])
        self.assertEqual(candidate.metadata['V_source'], 'tclose2')
        self.assertEqual(candidate.gate, 'stepchk')
        self.assertEqual(set(candidate.word), {'word', 'input_digest', 'output_digest'})
        self.assertEqual(candidate.check['off_after'], 392)
        self.assertEqual(R.read_json(runner.out / 'attempts/t1/inverse/closure/result.json')['V_source'], 'certificate')
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), expected['end_digest'])
        self.assertEqual(runner.state.off_lattice_count(), 392)
        tree = json.loads(gzip.decompress((runner.out / runner.record['steps'][0]['tree']['file']).read_bytes()))
        self.assertEqual([(p['name'], p['letters'], p.get('inverse_of')) for p in tree['parts']],
                         [('W', 288, None), ('V', 339, None), ('middle', 288, [0]), ('V⁻¹', 339, [1])])

    def certificate_runner(self, table_name):
        base = ROOT / R.J / 'restoration/12-low-height/runs/round22/line-a'
        table = self.tmp / 'input.json'
        R.atomic_json(table, R.read_json(base / table_name))
        args = R.parse_args(['--table', str(table), '--out', str(self.tmp / 'run'),
            '--cache', str(self.shared), '--greedy-seconds', '0'])
        runner = R.Runner(args, self.mechanics)
        runner.initialize()
        return base, table, runner

    def test_05_certificate_closure_at_current_state(self):
        base, table, runner = self.certificate_runner('literal/run1/z-02-b16.json.gz')
        self.assertEqual(runner.state.digest(), 'fde9e0fde60bf0eb9ea3ac8e2a34a9d420601bb02d5d2fbbb86bf096fd1b0403')
        self.assertEqual(runner.state.off_lattice_count(), 507)
        target = base / 'units/eb5-k40-k00-b20.json'
        unit = R.Unit(1201, len(R.read_json(target)['pieces']), target)
        self.assertEqual(unit.size, 60)
        core = R.read_json(base / 'replay2/inputs/cores/b20-core.json')
        candidate = runner.close(table, unit, core, runner.out / 'attempts/close')
        self.assertEqual(candidate.parts, [['V', 20232], ['middle', 36], ['V_inverse', 20232]])
        self.assertEqual(candidate.metadata, {'V_source': 'certificate', 'V_letters': 20232,
            'V_sha256': '2a5e1a1c186dc18f808f27d7e3e32f39d1716908c65298ec3bd0274ac2ece71d',
            'T_size': 111, 'T_digest': '395d1d2125581bbe75ca86099c65f9f286ad103047cda4b9ecb22fb7393a6e21',
            'N1': True, 'N2': True})
        self.assertEqual(candidate.gate, 'slp_apply')
        self.assertEqual(len(candidate.word['word']), 40500)
        self.assertEqual(R.word_sha256(candidate.word['word']),
                         '197d582c4c4326a7267dc389deda4900172841f459e4269bf0814250360dccb2')
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), 'aab10f250f0aedbbcc0fbf0eb655cb0e4434a839c22fc7957559a27437f46e7c')
        self.assertEqual(runner.state.off_lattice_count(), 447)
        self.assertEqual(runner.record['steps'][0]['before']['off'], 507)
        self.assertEqual(runner.record['steps'][0]['standing'], 'certificate')

    def test_06_certificate_t1prime_at_z1(self):
        base, table, runner = self.certificate_runner('literal/run2/z-02-b18.json.gz')
        self.assertEqual(runner.state.digest(), 'b1a6b4ebbeddbd05b125f9973b3c77a111a65d89963ac3478b5965fb43f47baa')
        self.assertEqual(runner.state.off_lattice_count(), 324)
        target = base / 'units/eb5-k40-k00-b07.json'
        unit = R.Unit(7, len(R.read_json(target)['pieces']), target)
        candidate = runner.t1prime(table, unit, base / 'replay2/inputs/b07-W.json', runner.out / 'attempts/t1')
        self.assertEqual(candidate.parts, [['W', 48], ['V', 139774], ['middle', 48], ['V_inverse', 139774]])
        self.assertEqual(candidate.metadata, {'V_source': 'certificate', 'V_letters': 139774,
            'V_sha256': '384e924e20f20607849d3ed78c34d1a7016c40e669c7fcd5ddaf9a2c5327d0a8',
            'T_size': 198, 'T_digest': 'c74506ed672f5e7f278c800f09264683dda8821ea4dcb28cf62ec39338917ba3',
            'N1': True, 'N2': True})
        self.assertEqual(candidate.gate, 'slp_apply')
        self.assertEqual(len(candidate.word['word']), 279644)
        runner.advance(unit, candidate, [], 0.0)
        check_record(self, runner.out, runner.record)
        self.assertEqual(runner.state.digest(), 'cc03b2f9b38a946389b0f3f0473dfd6ccb8693244cdf612a054dc0992c835049')
        self.assertEqual(runner.state.off_lattice_count(), 213)
        self.assertEqual(runner.record['steps'][0]['before']['off'], 324)

    def table_runs(self):
        """Runs A and B of the plan's D3 test (section 2), made once per class at R22D-36.

        A: r22d75 is stuck, then an r22d81 step (12, 14) reaches S2 = 7cd868f6 (heights {2: 3097, 4: 1}),
        where r22d75's entry test holds again; the order goes on to h4. B starts at S2, and its first
        height step is an r22d75 step, (35, 13)(35, 13)⁻¹(14, 14), to heights {2: 395, 4: 1}. Its
        tree names the second letter as the inverse of the first. Both end at --high-steps 0, so no
        tail runs on these 3,098-piece states. The plugins are scripted (see scripted_check)."""
        if ExactTests.runs is not None:
            return ExactTests.runs
        m, root = self.mechanics, self.shared / 'table-runs'
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        play_real(m, state, WORD36)
        start = root / 'r22d36.json'
        root.mkdir()
        m.export(state, start)
        common = ['--cache', str(self.shared), '--bulk-steps', '0', '--high-steps', '0', '--fallback-max-length', '0']
        a = Plugin('height', {'r22d75': [api.End('stuck', 'no tier fires')], 'r22d81': [('r22d81/H6', [[12, 14]])],
                              'h4': [api.End('stuck', 'no h4 step')]})
        runner = R.Runner(R.parse_args(['--table', str(start), '--out', str(root / 'a'), *common]), m, plugins(height=a))
        record_a = runner.run()
        x = [[35, 13]]
        word = x + m.inverse_word(x) + [[14, 14]]
        tree = api.flat('r22d75/T2', [('x', 1), ('x⁻¹', 1), ('lowering', 1)], inverse={1: 0})
        b = Plugin('height', {'r22d75': [('r22d75/T2', word, tree), api.End('stuck', 'no tier fires')],
                              'h4': [api.End('stuck', 'no h4 step')]})
        s2 = runner.out / record_a['steps'][0]['table']
        runner = R.Runner(R.parse_args(['--table', str(s2), '--out', str(root / 'b'), *common]), m, plugins(height=b))
        record_b = runner.run()
        ExactTests.runs = (root / 'a', record_a, a.calls, root / 'b', record_b, b.calls)
        return ExactTests.runs

    def scripted_check(self, out):
        """check_run.py with the height contracts left unchecked. The scripted height steps of runs A and B
        are not rule steps: they have no tau or u fields, and B's r22d75 word has three letters. The real
        contracts reject them (test_07 asserts it; the contracts' own tests are test_check_contracts.py and
        test_check_run.py's test_v3_height_contracts), so test_07 and test_08 check the phase order and the
        step trees only."""
        C = checker().check_contracts
        with patch.object(C, 'prepare', lambda row, st: {}), patch.object(C, 'on_letter', lambda ctx, k, st: None), \
                patch.object(C, 'finish', lambda ctx, row, st: [C.UNCHECKED]):
            return independent_check(out, allow_unchecked=True)

    def test_07_table_start_keeps_the_fixed_phase_order(self):
        out_a, record_a, calls_a, out_b, record_b, calls_b = self.table_runs()
        for out, record in ((out_a, record_a), (out_b, record_b)):
            check_record(self, out, record)
            self.assertEqual(record['result'], 'budget', record['error'])
            self.assertEqual(record['phases']['high']['result'], 'budget')
            self.assertIsNone(record['phases']['tail']['result'])
            self.assertEqual(record['acceptance']['verdict'], 'component')
        sub = record_a['phases']['height']['sub']
        self.assertEqual([sub[k]['result'] for k in api.SUBPHASES['height']], ['stuck', 'handoff', 'stuck'])
        self.assertEqual(record_a['start']['heights'], {'2': 5799, '6': 1})
        step = record_a['steps'][0]
        self.assertEqual((step['phase'], step['subphase'], step['rule']), ('height', 'r22d81', 'r22d81/H6'))
        self.assertTrue(step['after']['digest'].startswith('7cd868f6'))
        self.assertEqual(step['after']['heights'], {'2': 3097, '4': 1})
        # r22d75 would fire at S2, but the order never returns to it (D3, M4).
        self.assertTrue(entry('r22d75', step['after']))
        self.assertEqual(calls_a, ['r22d75', 'r22d81', 'h4'])
        self.assertEqual(record_b['start']['digest'], step['after']['digest'])
        first = record_b['steps'][0]
        self.assertEqual((first['phase'], first['subphase']), ('height', 'r22d75'))
        self.assertEqual(first['after']['heights'], {'2': 395, '4': 1})
        sub = record_b['phases']['height']['sub']
        self.assertEqual([sub[k]['result'] for k in api.SUBPHASES['height']], ['stuck', 'not_applicable', 'stuck'])
        self.assertEqual(calls_b, ['r22d75', 'r22d75', 'h4'])
        # The real contracts reject the scripted height steps, and nothing else in the records fails.
        for out in (out_a, out_b):
            with self.subTest(out=out.name, contracts='real'):
                final, rows = independent_check(out)
                self.assertIs(final['pass'], False, rows)
                self.assertTrue(final['fails'] and all(f.startswith('height_') for f in final['fails']), final)
        # With the height contracts left unchecked, both records pass with the fixed phase order.
        for out in (out_a, out_b):
            with self.subTest(out=out.name):
                final, rows = self.scripted_check(out)
                self.assertIs(final['pass'], True, rows)
                self.assertEqual(final['unchecked_contracts'], 1)

    def test_08_checker_rejects_tampered_step_trees(self):
        # Needs the runner-run/3 checker. Run B's closure tree with inverse_of passes; each change fails.
        _, _, _, out_b, record_b, _ = self.table_runs()
        step = record_b['steps'][0]
        root = json.loads(gzip.decompress((out_b / step['tree']['file']).read_bytes()))
        self.assertEqual(root['parts'][1]['inverse_of'], [0])

        def edit(change):
            tree = json.loads(json.dumps(root))
            for part in tree['parts']:
                part.pop('inverse_of', None)
            change(tree)
            return tree

        def later(tree):
            tree['parts'][0]['inverse_of'] = [1]

        # The third letter (14, 14) is not the first one's inverse, so only the letters reject it.
        self.assertNotEqual(self.mechanics.inverse_word([[35, 13]]), [[14, 14]])
        cases = {'intact': root,
                 'part sum': dict(root, letters=4),
                 'gap': edit(lambda t: [p.update(span=[i + (i > 0), i + 1 + (i > 0)]) for i, p in enumerate(t['parts'])]),
                 'overlap': edit(lambda t: t['parts'][1].update(span=[0, 1])),
                 'root shorter than the word': edit(lambda t: (t['parts'].pop(), t.update(letters=2))),
                 'wrong inverse_of range': edit(lambda t: t['parts'][2].update(inverse_of=[0])),
                 'inverse_of names a later part': edit(later),
                 'wrong sha': root}
        for name, tree in cases.items():
            with self.subTest(name=name):
                out = self.tmp / name.replace(' ', '-')
                shutil.copytree(out_b, out)
                record = json.loads(json.dumps(record_b))
                data = (json.dumps(tree, separators=(',', ':')) + '\n').encode()
                (out / step['tree']['file']).write_bytes(gzip.compress(data, mtime=0))
                record['steps'][0]['tree'].update(sha256=hashlib.sha256(data).hexdigest(), size=api.tree_size(tree))
                if name == 'wrong sha':
                    record['steps'][0]['tree']['sha256'] = 'a' * 64
                R.atomic_json(out / 'run.json', record)
                final, rows = self.scripted_check(out)
                self.assertIs(final['pass'], name == 'intact', rows)
                if name != 'intact':
                    self.assertTrue(any('tree' in fail for fail in final['fails']), final['fails'])

    def test_09_played_lattice_passes_the_checker(self):
        # A scripted plugin in P3's interface plays the small lattice word's inverse W⁻¹ = t·C·t⁻¹·C⁻¹
        # in two steps; the runner records them and the checker replays their ia16 words.
        m = self.mechanics
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        play_real(m, state, self.lattice_word())
        table = self.tmp / 'lattice-input.json'
        m.export(state, table)
        inverse = m.inverse_word(self.lattice_word())
        self.assertEqual(inverse[5:6], m.inverse_word(inverse[:1]))
        tree = api.flat('orbit 0', [('t', 1), ('C', 4), ('t⁻¹', 1)], inverse={2: 0})
        plugin = LatticePlay([('lattice/orbit-0', inverse[:6], tree), ('lattice/correction-0-0', inverse[6:], None)])
        args = R.parse_args(['--table', str(table), '--out', str(self.tmp / 'run'), '--cache', str(self.shared)])
        runner = R.Runner(args, m, plugins(lattice=plugin))
        record = runner.run()
        check_record(self, runner.out, record)
        self.assertEqual((record['result'], record['acceptance']['verdict']), ('solved', 'solved'), record['error'])
        self.assertEqual([(s['rule'], s['letters']) for s in record['steps']],
                         [('lattice/orbit-0', 6), ('lattice/correction-0-0', 4)])
        self.assertEqual(runner.state.digest(), R.SOLVED)
        # check_lattice.py (P5) replays the words and checks the lattice contract of both rows.
        final, rows = independent_check(runner.out)
        self.assertIs(final['pass'], True, rows)
        self.assertEqual((final['end_digest'], final['lattice_twists'], final['verdict'], final['unchecked_contracts']),
                         (R.SOLVED, 10, 'solved', 0))

    def test_10_lattice_ends_pass_the_checker(self):
        # A recorded lattice End (plan revision 3e, E2) passes the checker, which recomputes the
        # verdict: stuck, budget when an attempt at the end state timed out, budget, error.
        m = self.mechanics
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        play_real(m, state, self.lattice_word())
        table = self.tmp / 'lattice-input.json'
        m.export(state, table)
        clock = {'rule': 'lattice/late', 'outcome': 'timeout', 'reason': 'lattice_wall_clock', 'timed_out': True}
        ends = ((api.End('stuck', 'restorer_no_solution'), 'stuck'),
                (api.End('stuck', 'restorer_no_solution', attempts=[clock]), 'budget'),
                (api.End('timeout', 'lattice_wall_clock', attempts=[clock]), 'budget'),
                (api.End('budget', 'lattice_steps'), 'budget'),
                (api.End('error', 'player_failed'), 'error'))
        for end, verdict in ends:
            with self.subTest(end=end.result, attempts=len(end.attempts)):
                out = self.tmp / f'run-{end.result}-{len(end.attempts)}'
                args = R.parse_args(['--table', str(table), '--out', str(out), '--cache', str(self.shared)])
                plugin = SimpleNamespace(STUB=False, SCRIPTS=[], play=lambda ctx, end=end: end)
                record = R.Runner(args, m, plugins(lattice=plugin)).run()
                check_record(self, out, record)
                self.assertEqual(record['phases']['lattice']['result'], end.result)
                self.assertEqual(record['acceptance']['verdict'], verdict)
                final, rows = independent_check(out)
                self.assertIs(final['pass'], True, rows)
                self.assertEqual(final['verdict'], verdict)

    def test_11_kernel_end_to_end(self):
        # A height stub ends at once at R22D-36 (n6 = 1, max 6), so r22d75 and r22d81 are stuck and h4 is
        # not applicable; the real kernel (P2) with D19 plays one step, and the checker applies P4's
        # kernel contract to its row (plan revision 3d, section 4).
        m = self.mechanics
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        play_real(m, state, WORD36)
        z36 = state.digest()
        table = self.tmp / 'r22d36.json'
        m.export(state, table)
        stub = Plugin('height', {name: [api.End('not_applicable', 'stub')] for name in ('r22d75', 'r22d81')},
                      stub=True)
        out = self.tmp / 'run'
        args = R.parse_args(['--table', str(table), '--out', str(out), '--cache', str(self.shared), '--bulk-steps', '0',
                             '--high-steps', '1', '--tail-rules', '', '--fallback-max-length', '0'])
        record = R.Runner(args, m, plugins(height=stub)).run()
        check_record(self, out, record)
        self.assertNotEqual(record['result'], 'error', record['error'])
        sub = record['phases']['height']['sub']
        self.assertEqual([sub[k]['result'] for k in api.SUBPHASES['height']], ['stuck', 'stuck', 'not_applicable'])
        # --bulk-steps 0 is the first reason the run is a component; the height stub is the second.
        self.assertEqual((record['config']['component'], record['acceptance']['verdict']), ('bulk_budget', 'component'))
        kernel = importlib.import_module('rules_kernel')
        self.assertEqual(record['config']['kernel']['d19']['file'], kernel.D19.as_posix())
        (row,) = record['steps']
        self.assertEqual((row['phase'], row['family'], row['before']['digest']), ('high', 'F1', z36))
        self.assertEqual(row['contract'], 'kernel_' + row['kind'])
        self.assertLessEqual(R.read_json(out / row['word'])['word'][0], [12, 45])
        final, rows = independent_check(out)
        self.assertIs(final['pass'], True, rows)
        self.assertEqual((final['verdict'], final['unchecked_contracts'], final['high_twists']),
                         ('component', 0, row['letters']))
        self.assertIs(rows[0]['unchecked'], False)
        # The other kind fails the kernel contract. (high_exact protects nothing here: the target is
        # the only piece above height 2.)
        mutated = self.tmp / 'mutated'
        shutil.copytree(out, mutated)
        changed = json.loads(json.dumps(record))
        kind = 'landing' if row['kind'] == 'lowering' else 'lowering'
        changed['steps'][0].update(kind=kind, contract='kernel_' + kind)
        R.atomic_json(mutated / 'run.json', changed)
        final, rows = independent_check(mutated)
        self.assertIs(final['pass'], False, rows)
        self.assertTrue(any(fail.startswith('kernel_') for fail in rows[0]['fails']), rows)
        # The D19 hash of the call must be the one config.kernel.d19 records (I1R-03).
        self.assertEqual(row['kernel']['d19_sha256'], record['config']['kernel']['d19']['sha256'])
        for label, edit in (('missing', lambda s: s['kernel'].pop('d19_sha256')),
                            ('null', lambda s: s['kernel'].update(d19_sha256=None)),
                            ('another', lambda s: s['kernel'].update(d19_sha256='0' * 64)),
                            ('no kernel block', lambda s: s.pop('kernel'))):
            with self.subTest(d19=label):
                changed = json.loads(json.dumps(record))
                edit(changed['steps'][0])
                R.atomic_json(mutated / 'run.json', changed)
                final, rows = independent_check(mutated)
                self.assertIs(final['pass'], False, rows)
                self.assertIn('kernel_d19', rows[0]['fails'], rows)

    def test_12_resume_pending_height_handoff_passes_independent_checker(self):
        """A1/A2: interruption after the real dispatcher's final row, before handoff is saved."""
        m = self.mechanics
        state = m.sim.State(m.restore.ctx, menu=m.menu)
        play_real(m, state, WORD36)
        source = self.tmp / 'source-table.json'
        m.export(state, source)

        def arguments(name, resume=False):
            return R.parse_args(['--table', str(source), '--out', str(self.tmp / name),
                                 '--cache', str(self.shared), '--bulk-steps', '0', '--tail-rules', '',
                                 '--fallback-max-length', '0', *(['--resume'] if resume else [])])

        original = R.Runner(arguments('whole'), m).run()
        self.assertEqual(len(original['steps']), 2)
        cut = R.Runner(arguments('cut'), m)
        save = cut.save

        def crash():
            save()
            if len(cut.record['steps']) == 2:
                raise PowerLoss()

        with patch.object(cut, 'save', side_effect=crash), patch.object(cut, 'finish'):
            with self.assertRaises(PowerLoss):
                cut.run()
        result = R.Runner(arguments('cut', True), m).run()
        self.assertEqual(without_times(result), without_times(original))
        final, rows = independent_check(cut.out)
        self.assertTrue(final['pass'], rows)
        self.assertEqual(result['phases']['height']['sub']['r22d75']['result'], 'handoff')
        self.assertEqual((final['height_twists'], final['verdict']), (2, 'component'))

    @unittest.skipUnless(os.environ.get('RUNNER_SLOW') == '1', 'set RUNNER_SLOW=1 for depth 20')
    def test_04_slow_scramble_20(self):
        _, record = self.invoke('--scramble', '20', '7106')
        self.assertEqual(record['result'], 'solved')
        self.assertEqual(record['phases']['lattice']['result'], 'solved')


class LatticeEndToEndTests(unittest.TestCase):
    """Packet P3's plugin through the runner's lattice phase and the v3 checker (plan revision 3d, section 4):
    test_lattice_phase.py's synthetic and w9 fixtures, whose restorer is a stub copying a fixed program."""
    @classmethod
    def setUpClass(cls):
        with patch.dict(os.environ):
            cls.T = importlib.import_module('test_lattice_phase')
        cls.addClassCleanup(shutil.rmtree, cls.T.TEMPORARY, True)
        cls.tmp = Path(tempfile.mkdtemp(prefix='tutorial-lattice-e2e-'))
        cls.addClassCleanup(shutil.rmtree, cls.tmp)

    def run_fixture(self, name, f):
        T = self.T
        out = self.tmp / name
        args = R.parse_args(['--table', str(f.table), '--out', str(out), '--cache', str(T.TEMPORARY / 'cache'),
                             '--lattice-restorer', 'late'])
        with patch.object(T.L, 'LIBRARY', T.shared().library), patch.dict(T.L.RESTORERS, {'late': f.stub}):
            runner = R.Runner(args, T.shared().mechanics, plugins())
            record = runner.run()
        check_record(self, out, record)
        self.assertEqual((record['result'], record['error'], record['acceptance']['verdict']), ('solved', None, 'solved'))
        self.assertEqual((record['config']['component'], record['lattice']), (None, None))
        return runner, record

    def test_signal_keeps_the_lattice_attempt(self):
        # A signal during the restorer's call or the player's: the run ends error, the phase records no
        # result, and the cancelled call keeps its attempt row with the time of both calls (I1R-04,
        # P3-robust-01).
        T = self.T
        for stage in ('restore', 'play'):
            with self.subTest(stage=stage):
                f = T.fixture()
                out = self.tmp / ('signal-' + stage)
                args = R.parse_args(['--table', str(f.table), '--out', str(out), '--cache', str(T.TEMPORARY / 'cache'),
                                     '--lattice-restorer', 'late'])
                with patch.object(T.L, 'LIBRARY', T.shared().library), patch.dict(T.L.RESTORERS, {'late': f.stub}):
                    runner = R.Runner(args, T.shared().mechanics, plugins())
                    real, earlier = runner.execute, []

                    def execute(argv, tag, seconds, **kwargs):
                        if not tag.startswith('lattice-' + stage + '-'):
                            child = real(argv, tag, seconds, **kwargs)
                            earlier.append(child.seconds)
                            return child
                        runner.on_signal(signal.SIGTERM, None)
                        return R.Child(-signal.SIGTERM, 2.0, out / 'logs' / 'cancelled.log', cancelled=True)

                    with patch.object(runner, 'execute', execute):
                        record = runner.run()
                self.assertEqual(len(earlier), {'restore': 0, 'play': 1}[stage])
                self.assertEqual((record['result'], record['error']), ('error', 'signal %d' % signal.SIGTERM))
                lattice = record['phases']['lattice']
                self.assertIsNone(lattice.get('result'))
                self.assertEqual(lattice['attempts'], [{'rule': 'lattice/late', 'outcome': 'error',
                                                        'reason': 'cancelled', 'timed_out': False,
                                                        'seconds': sum(earlier) + 2.0}])
                self.assertFalse([s for s in record['steps'] if s['phase'] == 'lattice'])

    def test_synthetic_program(self):
        runner, record = self.run_fixture('synthetic', self.T.fixture())
        self.assertEqual([s['rule'] for s in record['steps']], ['lattice/orbit-23', 'lattice/orbit-34'])
        self.assertEqual(runner.state.digest(), R.SOLVED)
        final, rows = independent_check(runner.out)
        self.assertIs(final['pass'], True, rows)
        self.assertEqual((final['end_digest'], final['verdict'], final['unchecked_contracts']), (R.SOLVED, 'solved', 0))
        self.assertEqual(final['lattice_twists'], record['totals']['lattice_twists'])

    def test_w9_invisible_rotation(self):
        # Solved by identity labels at a digest that is not SOLVED (R3-05); the checker replays to it.
        T = self.T
        f = T.fixture('invisible', nodes=[], factors=[], order='retained-first',
                      sequence=[{'macro': 0, 'times': 1, 'inverse': False}], cascade=[0], counts=[0], invisible=True)
        f.summary['runs'][0].pop('factors_per_orbit', None)
        T.write(f.template / 'restore-summary.json', f.summary)
        state = T.shared().mechanics.load(f.table)
        for twist in T.correction_word(f.sequence):
            self.assertTrue(state.apply(twist, journal=False).applied)
        expected = state.digest()
        self.assertNotEqual(expected, R.SOLVED)
        runner, record = self.run_fixture('w9', f)
        self.assertEqual([(s['rule'], s['letters']) for s in record['steps']], [('lattice/correction-0-0', 9)])
        self.assertEqual(runner.state.digest(), expected)
        final, rows = independent_check(runner.out)
        self.assertIs(final['pass'], True, rows)
        self.assertEqual((final['end_digest'], final['verdict'], final['unchecked_contracts'], final['lattice_twists']),
                         (expected, 'solved', 0, 9))
        # One retained letter more keeps every piece on the lattice but leaves labels that are not the
        # identity. Everything else in the copy is made to agree with the ten letters (the row's end, its
        # table, the report and the counts), so only the row's claim of identity labels fails (I1R-05).
        s = T.shared()
        retained = [i for i in range(len(s.menu.items)) if i not in s.restore.jumble_items(s.menu)
                    and s.restore.element(s.menu, 0, i) != s.restore.I4]
        out = self.tmp / 'w9-longer'
        shutil.copytree(runner.out, out)
        step = json.loads(json.dumps(record['steps'][0]))
        raw = gzip.decompress((out / step['word']).read_bytes()) + struct.pack('<H', retained[0])
        (out / step['word']).write_bytes(gzip.compress(raw, mtime=0))
        tree = api.write_tree(out / step['tree']['file'], api.single(step['rule'], 10), 10)
        state = s.mechanics.load(f.table)
        for twist in T.correction_word(f.sequence) + [
                s.sim.Twist(s.ctx, 0, s.restore.element(s.menu, 0, retained[0]), 'menu', {'menu': s.menu.name})]:
            self.assertTrue(state.apply(twist, journal=False).applied)
        after = s.mechanics.snapshot(state)
        self.assertEqual(after['off'], 0)
        s.mechanics.export(state, out / step['table'])
        step.update(letters=10, word_sha256=hashlib.sha256(raw).hexdigest(), after=after,
                    tree=dict(step['tree'], sha256=tree['sha256'], size=tree['size']))
        report = R.read_json(out / 'lattice/report.json')
        player = report['player']
        for counts in (player['played'], player['scheduled']):
            counts['corrections'] += 1
        player['played']['total'] = 10
        player['final']['digest'] = after['digest']
        R.atomic_json(out / 'lattice/report.json', report)
        longer = json.loads(json.dumps(record))
        longer['steps'][0] = step
        longer['phases']['lattice'].update(twists=10, report=dict(
            longer['phases']['lattice']['report'], sha256=hashlib.sha256((out / 'lattice/report.json').read_bytes()).hexdigest()))
        longer['totals']['lattice_twists'] = 10
        R.atomic_json(out / 'run.json', longer)
        final, rows = independent_check(out)
        self.assertIs(final['pass'], False, rows)
        self.assertEqual(final['fails'], ['labels_identity'], rows)


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        elapsed = time.monotonic() - self.started
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {elapsed:.3f}s')


# Git metadata and the private trees other processes write to while the suite runs.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
    """Include ignored files, and compare contents as well as Git's status text."""
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = sorted(d for d in dirs if (here / d).as_posix() not in SKIP)
        for name in sorted(files):
            path = Path(directory) / name
            if path == ROOT / '.git':
                continue
            try:
                value = os.readlink(path) if path.is_symlink() else hashlib.sha256(path.read_bytes()).hexdigest()
            except FileNotFoundError:
                continue
            result[path.relative_to(ROOT).as_posix()] = value
    return result


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP[1:]], cwd=ROOT)


def main(names=()):
    """Run the whole suite, or the named classes and tests, and check the worktree is unchanged."""
    started = time.monotonic()
    before_status, before_files = status(), inventory()
    module, loader = sys.modules[__name__], unittest.defaultTestLoader
    suite = loader.loadTestsFromNames(names, module) if names else loader.loadTestsFromModule(module)
    result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    after_status, after_files = status(), inventory()
    unchanged = before_status == after_status and before_files == after_files
    if not unchanged:
        changed = sorted(p for p in before_files.keys() | after_files.keys() if before_files.get(p) != after_files.get(p))
        print('FAIL: acceptance changed the worktree (including ignored files):', changed, file=sys.stderr)
    else:
        print('Worktree unchanged, including ignored files.', file=sys.stderr)
    elapsed = time.monotonic() - started
    print(f'Acceptance elapsed: {elapsed:.3f}s.', file=sys.stderr)
    if elapsed > 60 * 60 and os.environ.get('RUNNER_SLOW') != '1':
        print('Note: default acceptance took over 60 minutes.', file=sys.stderr)
    return 0 if result.wasSuccessful() and unchanged else 1


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--_bulk-child':
        bulk_stub(Path(sys.argv[2]), Path(sys.argv[4]))
        sys.exit(0)
    if len(sys.argv) > 1 and sys.argv[1] == '--_signal-parent':
        args = R.parse_args(['--scramble', '1', '7', '--out', sys.argv[2], '--cache', sys.argv[3],
                             '--bulk-steps', '2', '--no-lattice', *(['--resume'] if '--resume' in sys.argv[4:] else [])])
        with patch.object(R, 'SELF', Path(__file__)):
            record = StubRunner(args).run()
        sys.exit(2 if record['result'] == 'error' else 0)
    sys.exit(main(sys.argv[1:]))
