"""Write-free acceptance checks for the census-to-render export tool."""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import copy
import hashlib
import io
import json
import os
import time
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(HERE))


def _no_writes(event, args):
    if event == 'open' and args[2] & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
        raise AssertionError('acceptance check attempted a file write')
    if event in ('os.mkdir', 'os.remove', 'os.rename', 'os.rmdir'):
        raise AssertionError('acceptance check attempted a filesystem change')


sys.addaudithook(_no_writes)

import deep_export as E
import wj

RECORD = ROOT / 'research/jumbling/restoration/11-scramble-census/runs/round1/ia-k10-s7001.json'


def main():
    failures = 0

    def case(name, check):
        nonlocal failures
        try:
            check()
        except Exception as exc:
            failures += 1
            print(f'{name}: FAIL ({type(exc).__name__})', flush=True)
        else:
            print(f'{name}: PASS', flush=True)

    started = time.perf_counter()
    try:
        st, M, hist = E.draw(7001, 10)
    except Exception as exc:
        print(f'draw seed 7001, 10 twists: FAIL ({type(exc).__name__})', flush=True)
        return 1
    print(f'draw seed 7001, 10 twists: PASS ({time.perf_counter() - started:.3f} s)', flush=True)
    try:
        f = E.facts(st, M, hist)
        record = json.loads(RECORD.read_text(encoding='utf-8'))
    except Exception as exc:
        print(f'facts and record: FAIL ({type(exc).__name__})', flush=True)
        return 1

    def record_passes():
        assert E.check_record(record, 7001, 10, f) == []

    case('committed record', record_passes)

    def counts():
        assert f['off_lattice'] == 26519
        assert f['left_heights'] == {'2': 26519}
        assert f['pieces'] == 177120
        assert f['off_poses'] == 9 and f['N'] == 13270
        assert f['poses'] == len(st._poses)
        assert f['kplus_poses'] == sum(p.kidx >= 0 for p in st._poses)
        assert f['digest'] == st.digest()
        assert (f['max_entry_height'], f['max_denominator']) == wj.height(st)

    case('census counts and exact digest', counts)

    def array_hashes():
        arrays = wj.arrays(st)
        assert set(f['files']) == {'pose_index.i32', 'poses.f32', 'pose_lattice.i32'}
        for name, a in arrays.items():
            assert f['files'][name] == {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
            assert a.dtype.str == ('<f4' if name == 'poses.f32' else '<i4')
        raw = json.dumps([list(h) for h in hist], separators=(',', ':')).encode('utf-8')
        assert f['history_sha256'] == hashlib.sha256(raw).hexdigest()

    case('render arrays and history SHA-256', array_hashes)

    changed_history = copy.deepcopy(record['history'])
    changed_history[0][1] += 1
    mutations = [('history', changed_history), ('off_scramble', record['off_scramble'] + 1),
                 ('N_start', record['N_start'] + 1), ('off_start', record['off_start'] + 1),
                 ('heights_start', {'2': 26518, '4': 1}),
                 ('off_poses_start', record['off_poses_start'] + 1),
                 ('seed', 7002), ('k', 11), ('menu', 'I_b'), ('continued_from_moves', 1)]
    for field, value in mutations:
        def mutant(field=field, value=value):
            altered = copy.deepcopy(record)
            altered[field] = value
            if field in ('seed', 'k', 'menu', 'continued_from_moves'):
                try:
                    E.check_record(altered, 7001, 10, f)
                except E.Refused as exc:
                    assert str(exc) == field
                else:
                    raise AssertionError('incompatible record was accepted')
            else:
                assert E.check_record(altered, 7001, 10, f) == [field]
        case('mutated ' + field, mutant)

    def render_header():
        hd = E.header(M, 7001, 10, f)
        assert hd['dimension'] == 4 and hd['format'] == 'magic600-jumbling-render/1'
        assert hd['revision'] == 10 and hd['digest'] == f['digest']
        assert hd['pieces'] == f['pieces'] and hd['poses'] == f['poses']
        assert hd['files'] == f['files']
        assert hd['model_identity'] == E.restore.ctx.data.identity
        assert hd['menu_identity'] == M.identity
        assert hd['contract_revision'] == E.sim.model.CONTRACT_REVISION
        assert hd['source'] == {'kind': 'census scramble',
                                'draw': 'research/jumbling/theory/restore.py scramble',
                                'menu': 'I_a', 'seed': 7001, 'twists': 10}
        assert 'fixture' not in hd

    case('render header', render_header)

    def command_line():
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(E, 'draw', return_value=(st, M, hist)) as mocked_draw:
            with redirect_stdout(stdout), redirect_stderr(stderr):
                code = E.main(['--seed', '7001', '--twists', '10', '--record', str(RECORD)])
        mocked_draw.assert_called_once_with(7001, 10)
        assert code == 0 and not stderr.getvalue()
        lines = stdout.getvalue().splitlines()
        assert len(lines) == 1
        summary = json.loads(lines[0])
        assert lines[0] == json.dumps(summary, sort_keys=True)
        assert summary['record_check'] == 'pass' and summary['mismatches'] == []
        assert summary['format'] == 'magic600-wj-deep-export/1'
        assert summary['menu'] == 'I_a' and summary['seed'] == 7001 and summary['twists'] == 10
        assert summary['seconds'] >= 0 and 'history' not in summary
        for field, value in f.items():
            if field != 'history':
                assert summary[field] == value
        for root in (str(ROOT), ROOT.as_posix(), json.dumps(str(ROOT))[1:-1]):
            assert root not in lines[0]

    case('main committed record, one sanitized JSON line', command_line)

    def refuse_nonempty():
        assert any(ROOT.iterdir())
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(E, 'draw', side_effect=AssertionError('draw must not run')) as mocked_draw:
            with redirect_stdout(stdout), redirect_stderr(stderr):
                code = E.main(['--seed', '7001', '--twists', '10', '--out', str(ROOT)])
        mocked_draw.assert_not_called()
        assert code == 2 and not stdout.getvalue()
        assert stderr.getvalue() == 'refused: out\n'

    case('non-empty out refused before drawing, no writes', refuse_nonempty)
    return int(bool(failures))


if __name__ == '__main__':
    raise SystemExit(main())
