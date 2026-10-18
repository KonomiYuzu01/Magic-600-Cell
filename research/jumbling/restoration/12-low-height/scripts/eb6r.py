"""E22-2: E-B6 after one retained letter, with the exact R22D-01 cap update.

Run from the repository root:
    python3 -I -B eb6r.py record <census.json> <out.json> [--grips c,...]
        [--units first,...] [--identity] [--shard i/k] [--seconds S]

The search retains E-B6's common-final-pose filter for the whole moved unit;
that filter is a property of this search, not a hypothesis of strict-prime
landing. Every word kept by Comm4 (including its non-lowering word list) is
replayed as r C r^-1. Failed replays report heights at the last admissible
prefix and false endpoint flags. Sharding is zero-based, after grip filtering.
Exact synthetic geometry and source only; no native or performance claim.
"""
import argparse
import json
import math
import os
import sys
import time
from pathlib import Path

# Also inherited by the acceptance check's command-line child. No search uses
# worker processes, and NumPy's small matrix screens need only one thread.
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[_name] = '1'

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import eb6
from eb6 import R, T, sim, stable_pose


class ZData:
    """Immutable classifications at Z; predictions never apply a twist."""

    def __init__(self, st, M):
        self.st, self.M, self.kp = st, M, st.ctx.kplus
        digest0 = st.digest()
        self.inside, self.strad = [], []
        self.clean = np.zeros(600, bool)
        self._labels = np.zeros((600, len(st.pose_id)), np.uint8)
        self._retained = {}
        self._identity = self.kp.index_of_matrix(R.I4)
        for x in range(600):
            cl = st.classify(x, need_inside=True, stop_at_straddle=False)
            assert not cl.uncertain, ('uncertain classification', x)
            inside = np.asarray(cl.inside, np.int64)
            strad = eb6.straddler_set(st, x) if cl.straddle is not None else np.zeros(0, np.int64)
            self.inside.append(inside)
            self.strad.append(strad)
            self.clean[x] = len(strad) == 0
            assert bool(self.clean[x]) == (cl.straddle is None)
            self._labels[x, inside] = 1
            self._labels[x, strad] = 2
        assert st.digest() == digest0, 'building ZData changed the state'

    def retained(self, c):
        """Sorted non-identity retained menu items at grip c, recognized exactly."""
        if not 0 <= c < 600:
            raise ValueError('grip out of range')
        if c not in self._retained:
            items = {}
            for j in range(len(self.M.items)):
                k = self.kp.index_of_matrix(R.element(self.M, c, j))
                if k >= 0 and k != self._identity:
                    items[j] = k
            self._retained[c] = items
        return sorted(self._retained[c])

    def _letter_index(self, c, j):
        if j not in self.retained(c):
            raise ValueError(f'letter ({c}, {j}) is not non-identity retained')
        return self._retained[c][j]

    def predict(self, c, j):
        """Class of a moved piece at x becomes its old class at k^-1(x)."""
        k = self._letter_index(c, j)
        if not self.clean[c]:
            raise ValueError(f'grip {c} is blocked at Z')
        moved = self.inside[c]
        moving = np.zeros(len(self.st.pose_id), bool)
        moving[moved] = True
        old = self._labels[:, moved]
        new = old[self.kp.perms[self.kp.inv[k]]]
        inside, strad, clean = list(self.inside), list(self.strad), self.clean.copy()
        # Only these rows can change. All pieces outside c keep their exact
        # classifications; the two concatenated sets below are disjoint.
        for x in np.flatnonzero(np.any(old != new, axis=1)).tolist():
            inside[x] = np.sort(np.concatenate((self.inside[x][~moving[self.inside[x]]],
                                               moved[new[x] == 1])))
            strad[x] = np.sort(np.concatenate((self.strad[x][~moving[self.strad[x]]],
                                              moved[new[x] == 2])))
            clean[x] = len(strad[x]) == 0
        return inside, strad, clean

    def prepared_caps(self, c, j, unit):
        U = unit['piece_ids']
        inside_c = self._labels[c, U] == 1
        assert not inside_c.any() or inside_c.all(), 'a retained letter split a Z unit'
        caps = unit['caps_whole']
        if inside_c.all():
            k = self._letter_index(c, j)
            return sorted(int(x) for x in self.kp.perms[k, caps])
        return caps

    def prunable(self, c, prediction, units):
        """Exactly the packet's no-moved-piece/no-classification-change rule."""
        inside_y, strad_y, clean_y = prediction
        moving = np.zeros(len(self.st.pose_id), bool)
        moving[self.inside[c]] = True
        if any(moving[u['piece_ids']].any() for u in units):
            return False
        # No unit moves, so its whole-cap vector is unchanged. Clean flags can
        # still change, including at a formerly blocked whole cap.
        clean_whole = {x for u in units for x in u['caps_whole'] if self.clean[x] or clean_y[x]}
        for x in region(clean_whole):
            if (moving[self.inside[x]].any() or moving[self.strad[x]].any()
                    or moving[inside_y[x]].any() or moving[strad_y[x]].any()):
                return False
            if (not np.array_equal(inside_y[x], self.inside[x])
                    or not np.array_equal(strad_y[x], self.strad[x])):
                return False
        return True


def region(clean_whole):
    """R_U: clean whole caps and every cap in their NEAR sets."""
    return sorted({int(y) for x in clean_whole for y in R.NEAR[x]} | set(clean_whole))


def high_units(st, Z, firsts=None):
    """The same pose/inside-vector grouping and ordering as E-B6 at Z."""
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    info = {}
    for pid in np.unique(st.pose_id[off]).tolist():
        hl, hr, bl, _ = T.pose_tree(st._pose_matrix(pid))
        assert hr == 0, 'a right height is not 0'
        info[pid] = (int(hl), int(bl))
    groups = {}
    for p in off.tolist():
        pid = int(st.pose_id[p])
        if info[pid][0] >= 4:
            vector = (Z._labels[:, p] == 1).tobytes()
            groups.setdefault((pid, vector), []).append(p)
    units = []
    for (pid, _), pieces in sorted(groups.items(), key=lambda kv: (-info[kv[0][0]][0], kv[1][0])):
        hl, bl = info[pid]
        units.append({'first': pieces[0], 'pieces': len(pieces), 'piece_ids': pieces,
                      'height': hl, 'branch': bl,
                      'caps_whole': np.flatnonzero(Z._labels[:, pieces[0]] == 1).tolist()})
    if firsts is not None:
        missing = set(firsts) - {u['first'] for u in units}
        if missing:
            raise ValueError(f'unknown high units: {sorted(missing)}')
        units = [u for u in units if u['first'] in firsts]
    return units


def verify_prediction(st, prediction, caps):
    inside, strad, clean = prediction
    for x in caps:
        cl = st.classify(x, need_inside=True, stop_at_straddle=False)
        assert not cl.uncertain, ('uncertain prediction check', x)
        assert np.array_equal(inside[x], cl.inside), ('predicted inside differs', x)
        assert bool(clean[x]) == (cl.straddle is None), ('predicted clean differs', x)
        direct_strad = eb6.straddler_set(st, x) if cl.straddle is not None else np.zeros(0, np.int64)
        assert np.array_equal(strad[x], direct_strad), ('predicted straddlers differ', x)


def twist(st, M, c, j):
    return sim.Twist(st.ctx, c, R.element(M, c, j), 'menu', {'menu': M.name})


def rewind(st, depth, digest):
    assert len(st.journal) >= depth, 'search consumed a preparation journal entry'
    while len(st.journal) > depth:
        st.undo()
    assert st.digest() == digest, 'the state changed'


def replay_word(st, M, U, word, pid0, digest0):
    """Replay all six prefixes and compare every piece by exact stable pose."""
    U = np.asarray(U, np.int64)
    depth = len(st.journal)
    admissible = True
    try:
        for c, j in word:
            outcome = st.apply(twist(st, M, c, j))
            if not outcome.applied or outcome.uncertain:
                admissible = False
                break
        heights = {}
        for pid in np.unique(st.pose_id[U]).tolist():
            heights[pid] = int(T.pose_tree(st._pose_matrix(pid))[0])
        outside = np.ones(len(st.pose_id), bool)
        outside[U] = False
        same = stable_pose(st) == pid0
        lattice = st._kpose()[st.pose_id] >= 0
        return {'admissible': bool(admissible), 'heights': [heights[int(st.pose_id[p])] for p in U],
                'lands': bool(admissible and lattice[U].all()),
                'strict': bool(admissible and same[outside].all()),
                'strict_prime': bool(admissible and (same | lattice)[outside].all())}
    finally:
        rewind(st, depth, digest0)


def search_letter(st, M, Z, units, c, j, prediction, pid_z, digest_z):
    inside_y, strad_y, clean_y = prediction
    depth = len(st.journal)
    kept, summaries = [], {}
    try:
        outcome = st.apply(twist(st, M, c, j))
        assert outcome.applied and not outcome.uncertain, ('retained preparation failed', c, j)
        caps_y = {u['first']: Z.prepared_caps(c, j, u) for u in units}
        touched = region({x for caps in caps_y.values() for x in caps if clean_y[x]})
        verify_prediction(st, prediction, touched)
        pid_y = stable_pose(st)
        comm = eb6.Comm4(st, M, inside_y, clean_y, pid_y)
        comm.strad = dict(enumerate(strad_y))
        inverse = comm.inv[j]
        for u in units:
            summary = comm.run(u['piece_ids'], u['height'], caps_y[u['first']])
            assert len(st.journal) == depth + 1 and np.array_equal(stable_pose(st), pid_y)
            for core in summary['strict_words'] + summary['strict_words_other']:
                kept.append({'unit': u['first'], 'word': [[c, j]] + core['word'] + [[c, inverse]],
                             'core_height_after': core['height_after']})
            summaries[str(u['first'])] = {k: v for k, v in summary.items()
                                          if k not in ('strict_words', 'strict_words_other')}
    finally:
        rewind(st, depth, digest_z)
    for row in kept:
        u = next(u for u in units if u['first'] == row['unit'])
        row['replay'] = replay_word(st, M, u['piece_ids'], row['word'], pid_z, digest_z)
    return {'grip': c, 'item': j, 'status': 'searched', 'units': summaries, 'words': kept}


def csv_ints(value):
    try:
        return set(int(v) for v in value.split(','))
    except ValueError as exc:
        raise argparse.ArgumentTypeError('expected comma-separated integers') from exc


def shard(value):
    try:
        i, k = map(int, value.split('/'))
        if k > 0 and 0 <= i < k:
            return i, k
    except ValueError:
        pass
    raise argparse.ArgumentTypeError('expected i/k with 0 <= i < k')


def seconds(value):
    s = float(value)
    if not math.isfinite(s) or s < 0:
        raise argparse.ArgumentTypeError('seconds must be finite and nonnegative')
    return s


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['record'])
    parser.add_argument('record')
    parser.add_argument('out')
    parser.add_argument('--grips', type=csv_ints)
    parser.add_argument('--units', type=csv_ints)
    parser.add_argument('--identity', action='store_true')
    parser.add_argument('--shard', type=shard)
    parser.add_argument('--seconds', type=seconds, default=36000.0)
    opt = parser.parse_args()
    t0 = time.monotonic()

    def log(**kw):
        print(json.dumps(dict(kw, sec=round(time.monotonic() - t0, 3))), file=sys.stderr, flush=True)

    st, M, source, _ = eb6.load([opt.mode, opt.record, opt.out])
    st.journal = []
    digest_z = st.digest()
    log(loaded=source, digest=digest_z)
    Z = ZData(st, M)
    units = high_units(st, Z, opt.units)
    pid_z = stable_pose(st)
    grips = np.flatnonzero(Z.clean).tolist()
    if opt.grips is not None:
        if not opt.grips <= set(grips):
            parser.error('--grips must name clean grips of Z')
        grips = sorted(opt.grips)
    letters = [(c, j) for c in grips for j in Z.retained(c)]
    if opt.shard is not None:
        i, k = opt.shard
        letters = [letter for n, letter in enumerate(letters) if n % k == i]
    log(caps_classified=True, clean=int(Z.clean.sum()), units=len(units), letters=len(letters))
    result = {'source': source, 'digest': digest_z, 'units': units, 'letters': []}
    if opt.identity:
        comm = eb6.Comm4(st, M, Z.inside, Z.clean, pid_z)
        comm.strad = dict(enumerate(Z.strad))
        summaries = {}
        try:
            for u in units:
                summaries[str(u['first'])] = comm.run(u['piece_ids'], u['height'], u['caps_whole'])
        finally:
            rewind(st, 0, digest_z)
        result['identity'] = {'units': summaries}
        log(identity=True, units=len(summaries))
    totals = {'letters': 0, 'searched': 0, 'pruned': 0, 'strict_words': 0,
              'strict_lowering_words': 0, 'replayed_strict_prime': 0}
    for c, j in letters:
        if time.monotonic() - t0 >= opt.seconds:
            break
        t1 = time.monotonic()
        prediction = Z.predict(c, j)
        if Z.prunable(c, prediction, units):
            row = {'grip': c, 'item': j, 'status': 'pruned'}
        else:
            row = search_letter(st, M, Z, units, c, j, prediction, pid_z, digest_z)
            totals['strict_words'] += sum(s['strict'] for s in row['units'].values())
            totals['strict_lowering_words'] += sum(s['strict_lowering'] for s in row['units'].values())
            totals['replayed_strict_prime'] += sum(w['replay']['admissible'] and w['replay']['strict_prime']
                                                   for w in row['words'])
        row['seconds'] = round(time.monotonic() - t1, 6)
        result['letters'].append(row)
        totals['letters'] += 1
        totals[row['status']] += 1
        log(grip=c, item=j, status=row['status'], seconds=row['seconds'], totals=totals)
    not_run = [list(letter) for letter in letters[len(result['letters']):]]
    assert st.digest() == digest_z, 'the state changed'
    assert not st.journal, 'the journal changed'
    result.update(totals=totals, complete=not not_run, letters_not_run=not_run,
                  seconds=round(time.monotonic() - t0, 3))
    Path(opt.out).write_text(json.dumps(result) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k not in ('units', 'letters', 'identity')}), flush=True)


if __name__ == '__main__':
    main()
