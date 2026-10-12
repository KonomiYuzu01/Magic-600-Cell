"""Check the lock lemma and Corollary L along a word, exactly.

At each checkpoint prefix, the rigid off blocks and the chambers locked to each block B (I_B within caps(x)
within I_B + S_B; ``blocks.Blocks``) are registered with their current content (a lattice piece or a hole).
The word is then replayed, and for every registered block:

(a) until B's first move by an element outside K, all members of B move together, and the content of every
    locked chamber x is in the chamber k_B(x), where k_B is the product of the K elements that moved B since
    the checkpoint (lock lemma);
(b) at that first move outside K, every lattice piece in a locked chamber goes off the lattice (Corollary L).

A violation of (a) or (b) is reported, never raised. Every forcing event is listed with its block (trackers);
for each piece forced off by (b), ``forced_off`` lists every forcing event in order (its prefix and the trackers
that forced it there) with the first later prefix at which the piece is on the lattice again, and whether the
piece is on the lattice at the end. The moved set of each twist is the set the simulator certifies for it
(captured at commit); pose ids are never compared across twists. The exit status is 0 only when there is no
violation and the end digest equals the word's output digest.

Usage (from the repository root)::

    python research/jumbling/theory/lockcheck.py --word <word file> --checkpoints 0,17670 --out <file>

The word file holds {'word', 'input_digest', 'output_digest'}; it is replayed from the E2 end of I_a@40
(``orient.start``), or from ``--table``. Evidence: exact simulator replay, not native host evidence.
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import blocks as BL  # noqa: E402
from exact import identity  # noqa: E402

CL, O, restore = BL.CL, BL.O, BL.restore
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--word', required=True)
    ap.add_argument('--table', help='current-only pose table of the word input (default: E2 end)')
    ap.add_argument('--checkpoints', required=True, help='comma-separated prefixes')
    ap.add_argument('--out', required=True)
    a = ap.parse_args(argv)
    doc = json.loads(Path(a.word).read_text())
    word = doc['word']
    cps = sorted({int(x) for x in a.checkpoints.split(',')})
    if any(not 0 <= x <= len(word) for x in cps):
        raise ValueError('checkpoint outside the word')
    R, M = O.start(a.table)
    st = R.st
    if st.digest() != doc['input_digest']:
        raise ValueError('word input digest differs from the start state')
    NP = len(st.pose_id)
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    kid = kp.index_of_matrix(BL.to_tuple(identity(4)))
    ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of

    def chamber(pieces, ks):
        return POF[ORB[pieces], kp.compose_vec(np.asarray(ks), TRN[pieces])]

    def occupancy():
        K = st._kpose()[st.pose_id]
        L = np.flatnonzero(K >= 0)
        occ = np.full(NP, -1, np.int64)
        ch = chamber(L, K[L])
        if len(np.unique(ch)) != len(ch):
            raise AssertionError('two lattice pieces occupy the same chamber')
        occ[ch] = L
        return occ

    kcache = {}

    def kindex(c, i):
        if (c, i) not in kcache:
            kcache[(c, i)] = int(kp.index_of_matrix(restore.element(M, c, i)))
        return kcache[(c, i)]

    last = {}
    commit = st._commit

    def capture(twist, inside, journal):
        last['inside'] = np.asarray(inside, np.int64).copy()
        return commit(twist, inside, journal)

    st._commit = capture
    occ = occupancy()
    trackers, report, forced, violations = [], {'checkpoints': []}, {}, []
    cpset = set(cps)
    for n in range(len(word) + 1):
        if n in cpset:
            bl = BL.Blocks(R, M)
            rec = {'prefix': n, 'digest': st.digest(), 'off': int(len(bl.off)), 'blocks': []}
            for b, members in enumerate(bl.blocks):
                lk = np.asarray(bl.locked[b], np.int64)
                t = {'id': len(trackers), 'cp': n, 'members': np.asarray(members, np.int64), 'k': kid,
                     'locked': lk, 'content': occ[lk].copy(), 'alive': True, 'k_moves': 0, 'event': None}
                trackers.append(t)
                rec['blocks'].append({'tracker': t['id'], 'size': int(len(members)), 'smallest_piece': int(members[0]),
                                      'locked': lk.tolist(), 'occupied_by': [int(q) for q in t['content'].tolist()]})
            report['checkpoints'].append(rec)
            log(checkpoint=n, blocks=len(bl.blocks), locked=int(sum(len(x) for x in bl.locked)),
                occupied=int(sum(int((occ[np.asarray(x, np.int64)] >= 0).sum()) for x in bl.locked)))
        if n == len(word):
            break
        c, i = int(word[n][0]), int(word[n][1])
        mk = kindex(c, i)
        K_before = st._kpose()[st.pose_id]
        last.clear()
        R.apply(c, i)
        moved = np.sort(last['inside'])
        K_after = st._kpose()[st.pose_id]
        Ko, Kn = K_before[moved], K_after[moved]
        a_ = moved[Ko >= 0]
        ya = chamber(a_, Ko[Ko >= 0])
        if not (occ[ya] == a_).all():
            raise AssertionError(f'occupancy bookkeeping broke at prefix {n + 1}')
        occ[ya] = -1
        b_ = moved[Kn >= 0]
        yb = chamber(b_, Kn[Kn >= 0])
        if not (occ[yb] == -1).all():
            raise AssertionError(f'two lattice pieces in one chamber at prefix {n + 1}')
        occ[yb] = b_
        for t in trackers:
            if not t['alive']:
                continue
            mv = np.isin(t['members'], moved)
            if mv.any():
                if not mv.all():
                    violations.append({'tracker': t['id'], 'prefix': n + 1, 'kind': 'block split'})
                    t['alive'] = False
                    continue
                if mk >= 0 and mk != kid:
                    t['k'] = int(kp.compose(mk, t['k']))
                    t['k_moves'] += 1
                elif mk < 0:
                    qs = t['content'][t['content'] >= 0]
                    went = [int(q) for q in qs.tolist() if K_after[q] < 0]
                    stayed = [int(q) for q in qs.tolist() if K_after[q] >= 0]
                    t['event'] = {'prefix': n + 1, 'grip': c, 'menu': i, 'retained_moves_before': t['k_moves'],
                                  'occupied_locked': int(len(qs)), 'went_off': went, 'stayed_on': stayed}
                    if stayed:
                        violations.append({'tracker': t['id'], 'prefix': n + 1, 'kind': 'locked piece stayed on'})
                    for q in went:
                        ev = forced.setdefault(q, [])
                        if ev and ev[-1]['off_at'] == n + 1:
                            ev[-1]['trackers'].append(t['id'])
                        else:
                            ev.append({'off_at': n + 1, 'trackers': [t['id']], 'back_at': None})
                    t['alive'] = False
                    continue
            if len(t['locked']):
                y = chamber(t['locked'], np.full(len(t['locked']), t['k']))
                if not np.array_equal(occ[y], t['content']):
                    violations.append({'tracker': t['id'], 'prefix': n + 1, 'kind': 'locked content moved'})
                    t['alive'] = False
        for q in set(moved.tolist()) & set(forced):
            f = forced[q][-1]
            if f['back_at'] is None and K_after[q] >= 0:
                f['back_at'] = n + 1
        if (n + 1) % 2000 == 0:
            log(prefix=n + 1, alive=sum(t['alive'] for t in trackers), forced=len(forced), violations=len(violations))
    end = st.digest()
    if not np.array_equal(occ, occupancy()):
        raise AssertionError('occupancy bookkeeping differs from the final state')
    K_end = st._kpose()[st.pose_id]
    report.update({'word_length': len(word), 'input_digest': doc['input_digest'], 'end_digest': end,
                   'end_digest_ok': end == doc.get('output_digest'), 'off_at_end': int((K_end < 0).sum()),
                   'violations': violations,
                   'trackers': [{'id': t['id'], 'checkpoint': t['cp'], 'size': int(len(t['members'])),
                                 'locked': int(len(t['locked'])), 'occupied_locked': int((t['content'] >= 0).sum()),
                                 'retained_moves': t['k_moves'], 'first_move_outside_K': t['event'],
                                 'alive_at_end': t['alive']} for t in trackers],
                   'forcing_events': sum(len(ev) for ev in forced.values()),
                   'forced_off': {str(q): {'events': ev, 'on_lattice_at_end': bool(K_end[q] >= 0)}
                                  for q, ev in sorted(forced.items())},
                   'evidence': 'exact simulator replay', 'seconds': round(time.time() - T0)})
    Path(a.out).write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({'violations': len(violations), 'forced_off': len(forced),
                      'forcing_events': report['forcing_events'],
                      'end_digest_ok': report['end_digest_ok'], 'off_at_end': report['off_at_end']}))
    return 0 if not violations and report['end_digest_ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
