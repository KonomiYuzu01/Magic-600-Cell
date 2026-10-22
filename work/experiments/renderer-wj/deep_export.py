"""Rebuild an I_a census scramble and optionally export its render-contract arrays."""
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


def facts(st, M, hist):
    """Summary facts, plus history lists kept in memory for exact record comparison."""
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    heights, off_poses = {}, 0
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        off_poses += 1
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        heights[d] = heights.get(d, 0) + c
    history = [list(h) for h in hist]
    h, d = wj.height(st)
    return {'format': FORMAT, 'menu': M.name, 'digest': st.digest(),
            'pieces': int(len(st.pose_id)), 'poses': len(st._poses),
            'kplus_poses': sum(p.kidx >= 0 for p in st._poses),
            'off_lattice': int(st.off_lattice_count()), 'off_poses': off_poses,
            'left_heights': {str(a): heights[a] for a in sorted(heights)},
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
    """Compare only the fresh census scramble's recorded start, never its descent."""
    _record_identity(record, seed, twists)
    expected = {'history': facts['history'], 'off_scramble': facts['off_lattice'],
                'N_start': facts['N'], 'off_start': facts['off_lattice'],
                'heights_start': facts['left_heights'], 'off_poses_start': facts['off_poses']}
    return [field for field, value in expected.items() if record.get(field) != value]


def header(M, seed, twists, facts):
    """The W-J render header, with census provenance in place of a fixture stage."""
    return {'format': wj.RENDER_FORMAT, 'dimension': wj.DIMENSION,
            'pieces': facts['pieces'], 'poses': facts['poses'], 'revision': twists,
            'digest': facts['digest'], 'model_identity': restore.ctx.data.identity,
            'menu_identity': M.identity, 'contract_revision': sim.model.CONTRACT_REVISION,
            'source': {'kind': 'census scramble',
                       'draw': 'research/jumbling/theory/restore.py scramble',
                       'menu': 'I_a', 'seed': seed, 'twists': twists},
            'convention': "x' = M x; M row-major float32; pose 0 is the identity",
            'files': facts['files']}


def main(argv=None):
    started = time.perf_counter()
    ap = _Parser(prog='deep_export.py', description=__doc__, allow_abbrev=False)
    ap.add_argument('--seed', type=int, required=True)
    ap.add_argument('--twists', type=int, required=True)
    ap.add_argument('--record', type=Path)
    ap.add_argument('--out', type=Path)
    ap.add_argument('--summary', type=Path)
    field = 'arguments'
    try:
        args = ap.parse_args(argv)
        if args.twists < 0:
            raise Refused('twists')
        field = 'out'
        if args.out is not None and args.out.exists():
            if not args.out.is_dir() or any(args.out.iterdir()):
                raise Refused('out')
        field = 'record'
        record = None
        if args.record is not None:
            record = json.loads(args.record.read_text(encoding='utf-8'))
            _record_identity(record, args.seed, args.twists)
        field = 'state'
        st, M, hist = draw(args.seed, args.twists)
        info = facts(st, M, hist)
        mismatches = check_record(record, args.seed, args.twists, info) if record is not None else []
        info.pop('history')
        info.update(seed=args.seed, twists=args.twists, mismatches=mismatches,
                    record_check=('not run' if record is None else 'fail' if mismatches else 'pass'))
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
        print('refused: ' + field, file=sys.stderr)
        return 2
    except SystemExit as exc:
        return int(exc.code)
    except Exception:
        print('failed: ' + field, file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
