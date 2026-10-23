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
from types import SimpleNamespace
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

    def invoke(argv):
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = E.main(argv)
        return code, stdout.getvalue(), stderr.getvalue()

    def clone(st):
        result = E.sim.State(st.ctx, filtered=st.filtered, menu=st.menu)
        result.pose_id = st.pose_id.copy()
        result._poses, result._lookup = list(st._poses), dict(st._lookup)
        result.journal = list(st.journal)
        return result

    def word_sha(word):
        raw = json.dumps([list(p) for p in word], separators=(',', ':')).encode('utf-8')
        return hashlib.sha256(raw).hexdigest()

    def summary_line(stdout):
        lines = stdout.splitlines()
        assert len(lines) == 1
        info = json.loads(lines[0])
        assert lines[0] == json.dumps(info, sort_keys=True)
        for root in (str(ROOT), ROOT.as_posix(), json.dumps(str(ROOT))[1:-1]):
            assert root not in lines[0]
        return info

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
        hd = E.header(M, 7001, 10, dict(f, revision=10, descent='not run', letters='not run'))
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
        assert summary['start_digest'] == f['digest'] and summary['revision'] == 10
        assert summary['descent'] == summary['letters'] == summary['readback'] == 'not run'
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

    def refuse_summary_inside_out():
        out = HERE / 'deep' / 'no-such-export-directory'
        assert not out.exists()
        for summary in (out, out / 'poses.f32', out / 'sub' / '..' / 'header.json'):
            with patch.object(E, 'draw', side_effect=AssertionError('draw must not run')) as mocked_draw:
                code, stdout, stderr = invoke(['--seed', '7001', '--twists', '0', '--out', str(out),
                                              '--summary', str(summary)])
            mocked_draw.assert_not_called()
            assert code == 2 and not stdout and stderr == 'refused: summary\n'
        with patch.object(E, 'draw', side_effect=RuntimeError('stop after the argument checks')) as mocked_draw:
            code, stdout, stderr = invoke(['--seed', '7001', '--twists', '0', '--out', str(out),
                                          '--summary', str(out.parent / 'no-such-export-summary.json')])
        mocked_draw.assert_called_once_with(7001, 0)
        assert code == 1 and not stdout and stderr == 'failed: state\n'

    case('summary inside out refused before drawing, no writes', refuse_summary_inside_out)

    replay = {}

    def census_descent():
        end = clone(st)
        problems, count = E.descend(end, M, record)
        assert problems == [] and count == 9
        df = E.facts(end, M, hist)
        assert df['off_lattice'] == df['N'] == df['off_poses'] == 0
        assert df['left_heights'] == record['heights_end'] == {}
        assert end.journal == st.journal == []
        replay['descent_facts'] = df

    case('committed descent, nine certain moves to the lattice', census_descent)

    def blocked_descent():
        altered = copy.deepcopy(record)
        altered['moves'][0] = [2, 18]
        end = clone(st)
        problems, count = E.descend(end, M, altered)
        assert problems == ['moves.0'] and count == 0
        assert end.digest() == f['digest'] and end.journal == []

    case('mutated descent move blocked at moves.0', blocked_descent)

    for field, value in [('twists', record['twists'] + 1), ('N_now', record['N_now'] + 1),
                         ('off_now', record['off_now'] + 1), ('heights_end', {'2': 1}),
                         ('off_poses_end', record['off_poses_end'] + 1)]:
        def mutant(field=field, value=value):
            altered = copy.deepcopy(record)
            altered[field] = value
            problems, count = E.descend(clone(st), M, altered)
            assert problems == [field] and count == 9
        case('mutated descent ' + field, mutant)

    def descent_header():
        provenance = {'moves': 9, 'moves_sha256': word_sha(record['moves']),
                      'digest': replay['descent_facts']['digest']}
        info = dict(replay['descent_facts'], revision=19, descent=provenance, letters='not run')
        hd = E.header(M, 7001, 10, info)
        assert hd['revision'] == 19 and hd['digest'] == info['digest']
        assert hd['files'] == info['files']
        assert hd['source']['descent'] == {'record_twists': 9,
                                            'moves_sha256': word_sha(record['moves'])}
        assert 'letters' not in hd['source']

    case('descent header revision and provenance', descent_header)

    def main_descent():
        with patch.object(E, 'draw', return_value=(clone(st), M, hist)) as mocked_draw:
            code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10',
                                          '--record', str(RECORD), '--descent'])
        mocked_draw.assert_called_once_with(7001, 10)
        assert code == 0 and not stderr
        info = summary_line(stdout)
        assert info['record_check'] == 'pass' and info['mismatches'] == []
        assert info['off_lattice'] == info['N'] == 0 and info['left_heights'] == {}
        assert info['start_digest'] == f['digest'] and info['revision'] == 19
        assert info['descent'] == {'moves': 9, 'moves_sha256': word_sha(record['moves']),
                                   'digest': info['digest']}
        assert info['letters'] == info['readback'] == 'not run'
        for field, value in replay['descent_facts'].items():
            if field != 'history':
                assert info[field] == value

    case('main descent, one sanitized final-state JSON line', main_descent)

    def missing_record():
        with patch.object(E, 'draw', side_effect=AssertionError('draw must not run')) as draw:
            code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10', '--descent'])
        draw.assert_not_called()
        assert code == 2 and not stdout and stderr == 'refused: descent\n'

    case('descent without record refused before drawing', missing_record)

    solved, _, _ = E.draw(7001, 0)
    letters_doc = {'format': 'magic600-wj-letters/1', 'letters': [list(h) for h in hist],
                   'checkpoints': {'0': solved.digest(), '10': f['digest']},
                   'source': 'ignored provenance'}

    def letters_replay():
        end = clone(solved)
        problems, count, checked = E.play_letters(end, M, letters_doc)
        assert problems == [] and count == 10 and checked == {'0': 'pass', '10': 'pass'}
        lf = E.facts(end, M, [])
        assert lf['digest'] == f['digest'] and lf['files'] == f['files']
        assert end.journal == []
        replay['letters_facts'] = lf

    case('ten history letters reproduce scramble digest and array hashes', letters_replay)

    def wrong_checkpoint():
        doc = copy.deepcopy(letters_doc)
        doc['checkpoints']['10'] = '0' * 64
        problems, count, checked = E.play_letters(clone(solved), M, doc)
        assert problems == ['letters.checkpoint.10'] and count == 10
        assert checked == {'0': 'pass', '10': 'fail'}

    case('wrong letter checkpoint detected at letters.checkpoint.10', wrong_checkpoint)

    def blocked_letter():
        doc = {'format': 'magic600-wj-letters/1', 'letters': [[2, 18], hist[0]],
               'checkpoints': {'0': f['digest'], '1': '0' * 64}}
        end = clone(st)
        problems, count, checked = E.play_letters(end, M, doc)
        assert problems == ['letters.1'] and count == 0 and checked == {'0': 'pass'}
        assert end.digest() == f['digest'] and end.journal == []

    case('blocked letter stops at letters.1 without later checkpoints', blocked_letter)

    def uncertain_replay():
        end = clone(st)
        uncertain = SimpleNamespace(applied=True, uncertain=1)
        with patch.object(end, 'apply', return_value=uncertain):
            assert E.descend(end, M, record) == (['moves.0'], 0)
            doc = {'format': 'magic600-wj-letters/1', 'letters': hist, 'checkpoints': {}}
            assert E.play_letters(end, M, doc) == (['letters.1'], 0, {})

    case('uncertain moves and letters are failures', uncertain_replay)

    def letters_header():
        provenance = {'count': 10, 'sha256': word_sha(hist),
                      'checkpoints': {'0': 'pass', '10': 'pass'}}
        info = dict(replay['letters_facts'], revision=10, descent='not run', letters=provenance)
        hd = E.header(M, 7001, 0, info)
        assert hd['revision'] == 10 and hd['digest'] == f['digest']
        assert hd['source']['twists'] == 0
        assert hd['source']['letters'] == {'count': 10, 'sha256': word_sha(hist)}
        assert 'descent' not in hd['source']

    case('letters header revision and provenance', letters_header)

    def main_letters():
        with patch.object(E, 'draw', return_value=(clone(solved), M, [])) as draw:
            with patch.object(Path, 'read_text', return_value=json.dumps(letters_doc)):
                code, stdout, stderr = invoke(['--seed', '7001', '--twists', '0',
                                              '--letters', 'letters.json'])
        draw.assert_called_once_with(7001, 0)
        assert code == 0 and not stderr
        info = summary_line(stdout)
        assert info['digest'] == f['digest'] and info['files'] == f['files']
        assert info['start_digest'] == solved.digest() and info['revision'] == 10
        assert info['letters'] == {'count': 10, 'sha256': word_sha(hist),
                                   'checkpoints': {'0': 'pass', '10': 'pass'}}
        assert info['history_sha256'] == word_sha([]) and info['mismatches'] == []
        assert info['record_check'] == info['descent'] == info['readback'] == 'not run'

    case('main letters, final arrays and scramble provenance', main_letters)

    bad_letters = [('format', dict(letters_doc, format='unknown')),
                   ('grip 600', dict(letters_doc, letters=[[600, 0]])),
                   ('item range', dict(letters_doc, letters=[[0, len(M.items)]])),
                   ('boolean grip', dict(letters_doc, letters=[[True, 0]])),
                   ('checkpoint range', dict(letters_doc, checkpoints={'11': f['digest']})),
                   ('large checkpoint', dict(letters_doc, checkpoints={'9' * 5000: f['digest']})),
                   ('non-hex digest', dict(letters_doc, checkpoints={'0': 'g' * 64})),
                   ('checkpoint key', dict(letters_doc, checkpoints={'-1': f['digest']})),
                   ('object shape', []), ('JSON null', None)]
    for name, doc in bad_letters:
        def malformed(doc=doc):
            with patch.object(E, 'draw', side_effect=AssertionError('draw must not run')) as draw:
                with patch.object(Path, 'read_text', return_value=json.dumps(doc)):
                    code, stdout, stderr = invoke(['--seed', '7001', '--twists', '0',
                                                  '--letters', 'letters.json'])
            draw.assert_not_called()
            assert code == 2 and not stdout and stderr == 'refused: letters\n'
        case('malformed letters ' + name + ' refused before drawing', malformed)

    def malformed_records():
        for field, value in [('moves', [[600, 0]]), ('twists', '9'), ('N_now', None),
                             ('history', None), ('heights_end', [])]:
            altered = dict(record, **{field: value})
            with patch.object(E, 'draw', side_effect=AssertionError('draw must not run')) as draw:
                with patch.object(Path, 'read_text', return_value=json.dumps(altered)):
                    code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10',
                                                  '--record', str(RECORD), '--descent'])
            draw.assert_not_called()
            assert code == 2 and not stdout and stderr == 'refused: ' + field + '\n'

    case('malformed record fields refused before drawing', malformed_records)

    def phase_order():
        for field in ('N_start', 'N_now'):
            altered = dict(record, **{field: record[field] + 1})

            def reader(path, *args, **kwargs):
                return json.dumps(altered if path == RECORD else letters_doc)

            with patch.object(Path, 'read_text', autospec=True, side_effect=reader):
                with patch.object(E, 'draw', return_value=(clone(st), M, hist)):
                    with patch.object(E, 'play_letters') as letters:
                        with patch.object(E, 'descend', wraps=E.descend) as descent:
                            code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10',
                                                          '--record', str(RECORD), '--descent',
                                                          '--letters', 'letters.json'])
            assert code == 1 and not stderr
            info = summary_line(stdout)
            assert info['mismatches'] == [field] and info['record_check'] == 'fail'
            assert info['letters'] == info['readback'] == 'not run'
            letters.assert_not_called()
            if field == 'N_start':
                descent.assert_not_called()
                assert info['revision'] == 10 and info['descent'] == 'not run'
                assert info['digest'] == f['digest']
            else:
                descent.assert_called_once()
                assert info['revision'] == 19 and info['descent']['moves'] == 9
                assert info['digest'] == replay['descent_facts']['digest']

    case('start and descent mismatches prevent later phases', phase_order)

    def letters_after_descent():
        doc = {'format': 'magic600-wj-letters/1', 'letters': [hist[0]],
               'checkpoints': {'0': replay['descent_facts']['digest']}}

        def reader(path, *args, **kwargs):
            return json.dumps(record if path == RECORD else doc)

        with patch.object(Path, 'read_text', autospec=True, side_effect=reader):
            with patch.object(E, 'draw', return_value=(clone(st), M, hist)):
                code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10',
                                              '--record', str(RECORD), '--descent',
                                              '--letters', 'letters.json'])
        assert code == 0 and not stderr
        info = summary_line(stdout)
        assert info['revision'] == 20 and info['record_check'] == 'pass'
        assert info['letters']['count'] == 1 and info['letters']['checkpoints'] == {'0': 'pass'}
        assert info['descent']['digest'] == replay['descent_facts']['digest']
        hd = E.header(M, 7001, 10, info)
        assert hd['revision'] == 20
        assert hd['source']['letters'] == {'count': 1, 'sha256': word_sha(doc['letters'])}
        assert hd['source']['descent']['record_twists'] == 9

    case('letters run after descent with combined revision and provenance', letters_after_descent)

    def checkpoint_failure_summary():
        doc = dict(letters_doc, checkpoints={'10': '0' * 64})
        with patch.object(Path, 'read_text', return_value=json.dumps(doc)):
            with patch.object(E, 'draw', return_value=(clone(solved), M, [])):
                code, stdout, stderr = invoke(['--seed', '7001', '--twists', '0',
                                              '--letters', 'letters.json'])
        assert code == 1 and not stderr
        info = summary_line(stdout)
        assert info['record_check'] == info['readback'] == 'not run'
        assert info['mismatches'] == ['letters.checkpoint.10']
        assert info['letters']['checkpoints'] == {'10': 'fail'} and info['revision'] == 10

    case('main letter checkpoint failure is a state mismatch', checkpoint_failure_summary)

    def simulator_failures():
        for error in (ValueError, TypeError, RuntimeError):
            with patch.object(E, 'draw', side_effect=error('private simulator diagnostic')):
                code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10'])
            assert code == 1 and not stdout and stderr == 'failed: state\n'
            with patch.object(E, 'draw', return_value=(clone(st), M, hist)):
                with patch.object(E, 'play', side_effect=error('private simulator diagnostic')):
                    code, stdout, stderr = invoke(['--seed', '7001', '--twists', '10',
                                                  '--record', str(RECORD), '--descent'])
            assert code == 1 and not stdout and stderr == 'failed: state\n'

    case('simulator draw and play exceptions fail without leaking diagnostics', simulator_failures)
    return int(bool(failures))


if __name__ == '__main__':
    raise SystemExit(main())
