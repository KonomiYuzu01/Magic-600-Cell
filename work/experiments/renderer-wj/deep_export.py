"""Replay an I_a census scramble, descent and letters into render-contract arrays."""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import hashlib
import json
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path[:0] = [str(ROOT / 'research/jumbling/theory'), str(ROOT / 'research/jumbling'),
                str(ROOT / 'research/jumbling/fixtures')]

import numpy as np
import restore
import rim
import sim
import trees as T
import wj

FORMAT = 'magic600-wj-deep-export/1'


class Refused(ValueError):
    """An invalid request, reported by field name without filesystem paths."""


class _Parser(argparse.ArgumentParser):
    def error(self, message):
        raise Refused('arguments')


def draw(seed, twists):
    """Use exactly the census draw, including its default retained_mix of zero."""
    if type(seed) is not int:
        raise Refused('seed')
    if type(twists) is not int or twists < 0:
        raise Refused('twists')
    M = restore.menu('I_a')
    st, hist = restore.scramble(M, twists, seed)
    return st, M, hist


def _heights(st):
    """Left-height histogram over off-lattice pieces and the number of off poses."""
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    heights, off_poses = {}, 0
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        off_poses += 1
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        heights[d] = heights.get(d, 0) + c
    return {str(a): heights[a] for a in sorted(heights)}, off_poses


def facts(st, M, hist):
    """Summary facts, plus history lists kept in memory for exact record comparison."""
    heights, off_poses = _heights(st)
    history = [list(h) for h in hist]
    h, d = wj.height(st)
    return {'format': FORMAT, 'menu': M.name, 'digest': st.digest(),
            'pieces': int(len(st.pose_id)), 'poses': len(st._poses),
            'kplus_poses': sum(p.kidx >= 0 for p in st._poses),
            'off_lattice': int(st.off_lattice_count()), 'off_poses': off_poses,
            'left_heights': heights,
            'N': int(rim.Domains(st).total_misalignment()),
            'max_entry_height': h, 'max_denominator': d, 'files': wj.array_refs(st),
            'history': history,
            'history_sha256': hashlib.sha256(
                json.dumps(history, separators=(',', ':')).encode('utf-8')).hexdigest()}


def _record_identity(record, seed, twists):
    if not isinstance(record, dict):
        raise Refused('record')
    for field, value in (('menu', 'I_a'), ('seed', seed), ('k', twists),
                         ('continued_from_moves', 0)):
        if record.get(field) != value:
            raise Refused(field)


def check_record(record, seed, twists, facts):
    """Compare the fresh census scramble's recorded start."""
    _record_identity(record, seed, twists)
    expected = {'history': facts['history'], 'off_scramble': facts['off_lattice'],
                'N_start': facts['N'], 'off_start': facts['off_lattice'],
                'heights_start': facts['left_heights'], 'off_poses_start': facts['off_poses']}
    return [field for field, value in expected.items() if record.get(field) != value]


def _pairs(value, M, field):
    if not isinstance(value, list):
        raise Refused(field)
    for pair in value:
        if (not isinstance(pair, (list, tuple)) or len(pair) != 2
                or any(type(v) is not int for v in pair)
                or not 0 <= pair[0] < 600 or not 0 <= pair[1] < len(M.items)):
            raise Refused(field)


def _record_fields(record, M, descent=False):
    _pairs(record.get('history'), M, 'history')
    counts = ['seed', 'k', 'continued_from_moves', 'off_scramble', 'N_start',
              'off_start', 'off_poses_start']
    heights = ['heights_start']
    if descent:
        _pairs(record.get('moves'), M, 'moves')
        counts += ['twists', 'N_now', 'off_now']
        if 'heights_end' in record:
            heights.append('heights_end')
        if 'off_poses_end' in record:
            counts.append('off_poses_end')
    for field in counts:
        value = record.get(field)
        if type(value) is not int or (field != 'seed' and value < 0):
            raise Refused(field)
    for field in heights:
        value = record.get(field)
        if (not isinstance(value, dict)
                or any(not isinstance(k, str) or not k.isascii() or not k.isdecimal()
                       or type(v) is not int or v < 0 for k, v in value.items())):
            raise Refused(field)


def validate_letters(doc, M):
    """Validate letters before drawing and index checkpoints by played count."""
    if not isinstance(doc, dict) or doc.get('format') != 'magic600-wj-letters/1':
        raise Refused('letters')
    _pairs(doc.get('letters'), M, 'letters')
    checkpoints = doc.get('checkpoints')
    if not isinstance(checkpoints, dict):
        raise Refused('letters')
    indexed = {}
    limit = len(doc['letters'])
    for key, digest in checkpoints.items():
        if (not isinstance(key, str) or not key.isascii() or not key.isdecimal()
                or not isinstance(digest, str)
                or len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest)):
            raise Refused('letters')
        digits = key.lstrip('0') or '0'
        if len(digits) > len(str(limit)) or int(digits) > limit:
            raise Refused('letters')
        indexed.setdefault(int(digits), []).append((key, digest))
    return indexed


def play(st, M, c, j):
    """Play only the supplied menu twist, without retaining a journal."""
    out = st.apply(sim.Twist(restore.ctx, c, restore.element(M, c, j),
                             'menu', {'menu': M.name}), journal=False)
    return bool(out.applied and not out.uncertain)


def descend(st, M, record):
    """Return mismatches and the number of certain descent moves played."""
    _record_fields(record, M, descent=True)
    for i, (c, j) in enumerate(record['moves']):
        if not play(st, M, c, j):
            return ['moves.' + str(i)], i
    count = len(record['moves'])
    expected = {'twists': count, 'N_now': int(rim.Domains(st).total_misalignment()),
                'off_now': int(st.off_lattice_count())}
    if 'heights_end' in record or 'off_poses_end' in record:
        heights, off_poses = _heights(st)
        if 'heights_end' in record:
            expected['heights_end'] = heights
        if 'off_poses_end' in record:
            expected['off_poses_end'] = off_poses
    return [field for field, value in expected.items() if record.get(field) != value], count


def play_letters(st, M, doc):
    """Return mismatches, the played count and the results of reached checkpoints."""
    checkpoints = validate_letters(doc, M)
    mismatches, checked = [], {}

    def check(n):
        if n in checkpoints:
            digest = st.digest()
            for key, expected in checkpoints[n]:
                checked[key] = 'pass' if digest == expected else 'fail'
                if digest != expected:
                    mismatches.append('letters.checkpoint.' + str(n))

    count = 0
    check(0)
    for c, j in doc['letters']:
        if not play(st, M, c, j):
            mismatches.append('letters.' + str(count + 1))
            break
        count += 1
        check(count)
    return mismatches, count, checked


def _word_sha256(word):
    return hashlib.sha256(json.dumps([list(p) for p in word],
                                    separators=(',', ':')).encode('utf-8')).hexdigest()


def header(M, seed, twists, facts):
    """The W-J render header, with census provenance in place of a fixture stage."""
    source = {'kind': 'census scramble', 'draw': 'research/jumbling/theory/restore.py scramble',
              'menu': 'I_a', 'seed': seed, 'twists': twists}
    if facts['descent'] != 'not run':
        descent = facts['descent']
        source['descent'] = {'record_twists': descent['moves'],
                             'moves_sha256': descent['moves_sha256']}
    if facts['letters'] != 'not run':
        letters = facts['letters']
        source['letters'] = {'count': letters['count'], 'sha256': letters['sha256']}
    return {'format': wj.RENDER_FORMAT, 'dimension': wj.DIMENSION,
            'pieces': facts['pieces'], 'poses': facts['poses'], 'revision': facts['revision'],
            'digest': facts['digest'], 'model_identity': restore.ctx.data.identity,
            'menu_identity': M.identity, 'contract_revision': sim.model.CONTRACT_REVISION,
            'source': source,
            'convention': "x' = M x; M row-major float32; pose 0 is the identity",
            'files': facts['files']}


def main(argv=None):
    started = time.perf_counter()
    ap = _Parser(prog='deep_export.py', description=__doc__, allow_abbrev=False)
    ap.add_argument('--seed', type=int, required=True)
    ap.add_argument('--twists', type=int, required=True)
    ap.add_argument('--record', type=Path)
    ap.add_argument('--descent', action='store_true')
    ap.add_argument('--letters', type=Path)
    ap.add_argument('--out', type=Path)
    ap.add_argument('--summary', type=Path)
    field = 'arguments'
    try:
        args = ap.parse_args(argv)
        if args.twists < 0:
            raise Refused('twists')
        if args.descent and args.record is None:
            raise Refused('descent')
        field = 'out'
        if args.out is not None and args.out.exists():
            if not args.out.is_dir() or any(args.out.iterdir()):
                raise Refused('out')
        field = 'summary'
        if args.out is not None and args.summary is not None:
            out, summary = args.out.resolve(), args.summary.resolve()
            if summary == out or out in summary.parents:
                raise Refused('summary')
        field = 'record'
        record = None
        if args.record is not None:
            record = json.loads(args.record.read_text(encoding='utf-8'))
            _record_identity(record, args.seed, args.twists)
        field = 'letters'
        letters_doc = None
        if args.letters is not None:
            letters_doc = json.loads(args.letters.read_text(encoding='utf-8'))
        field = 'state'
        M = restore.menu('I_a')
        if record is not None:
            _record_fields(record, M, args.descent)
        if args.letters is not None:
            validate_letters(letters_doc, M)
        st, M, hist = draw(args.seed, args.twists)
        info = facts(st, M, hist)
        start_digest = info['digest']
        mismatches = check_record(record, args.seed, args.twists, info) if record is not None else []
        descent, letters = 'not run', 'not run'
        revision = args.twists
        if args.descent and not mismatches:
            problems, count = descend(st, M, record)
            mismatches.extend(problems)
            revision += count
            descent = {'moves': count, 'moves_sha256': _word_sha256(record['moves']),
                       'digest': st.digest()}
        record_check = 'not run' if record is None else 'fail' if mismatches else 'pass'
        if args.letters is not None and not mismatches:
            problems, count, checked = play_letters(st, M, letters_doc)
            mismatches.extend(problems)
            revision += count
            letters = {'count': count, 'sha256': _word_sha256(letters_doc['letters']),
                       'checkpoints': checked}
        if descent != 'not run' or letters != 'not run':
            info = facts(st, M, hist)
        info.pop('history')
        info.update(seed=args.seed, twists=args.twists, mismatches=mismatches,
                    start_digest=start_digest, revision=revision, descent=descent, letters=letters,
                    record_check=record_check, readback='not run')
        field = 'out'
        if args.out is not None and not mismatches:
            args.out.mkdir(parents=True, exist_ok=True)
            hd = header(M, args.seed, args.twists, info)
            for name, a in wj.arrays(st).items():
                (args.out / name).write_bytes(np.ascontiguousarray(a).tobytes())
            (args.out / 'header.json').write_text(
                json.dumps(hd, indent=1, sort_keys=True) + '\n', encoding='utf-8', newline='\n')
            for name, meta in hd['files'].items():
                actual = hashlib.sha256((args.out / name).read_bytes()).hexdigest()
                if actual != meta['sha256']:
                    mismatches.append('files.' + name + '.sha256')
            info['readback'] = 'fail' if mismatches else 'pass'
        info['seconds'] = round(time.perf_counter() - started, 3)
        field = 'summary'
        if args.summary is not None:
            args.summary.write_text(json.dumps(info, indent=1, sort_keys=True) + '\n', encoding='utf-8',
                                    newline='\n')
        print(json.dumps(info, sort_keys=True))
        return int(bool(mismatches))
    except Refused as exc:
        print('refused: ' + str(exc), file=sys.stderr)
        return 2
    except (OSError, ValueError, TypeError):
        if field == 'state':
            print('failed: state', file=sys.stderr)
            return 1
        print('refused: ' + field, file=sys.stderr)
        return 2
    except SystemExit as exc:
        return int(exc.code)
    except Exception:
        print('failed: ' + field, file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
