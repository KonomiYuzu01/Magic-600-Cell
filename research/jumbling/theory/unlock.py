"""Dissect unlock windows of a word: from a block's first move outside K until the excursion it opens is closed.

Prefix p of the word is the state after its first p twists; twist p (counted from 1) is word[p - 1]. A window s:n:b
names an anchor prefix s <= n - 1 (chosen by the caller, for instance the last prefix before n with the fewest off
pieces), a forcing twist n, which moves a rigid off block by an element outside K and so takes the lattice pieces in
the block's occupied locked chambers off the lattice (Corollary L), and the first prefix b >= n at which all of these
forced pieces are on the lattice again. The forced pieces being back at b does not close the excursion: other
lattice pieces taken off on the way may still be off. The script therefore also finds the closing prefix e: the
first prefix >= n at which every lattice piece of the anchor prefix s is on the lattice again, and reports where the
blocks moved by the forcing twist are then. The witness word of the window is word[s:e]: twists s + 1 to e, e - s
twists, the forcing twist included, from prefix s to prefix e.

For every window the report gives
- the anchor, prefix s: digest, off count, the pose table (with ``--tables``) and the ``--pre`` twists before it;
- at prefix n - 1: digest, off count, the rigid off blocks (``blocks.Blocks``) that the forcing twist moves, with
  their locked chambers and the content of each, and the forced pieces;
- every twist from s + 1 to e: grip, menu index, the index of its element in K (null outside K), the number of moved
  pieces, the rigid off blocks of the anchor it moves and, from twist n on, those of prefix n - 1, the numbers of
  lattice pieces it takes off and of off pieces it puts on the lattice, the number of off pieces it moves that stay off, the off count and the digest
  after it, and for the pieces it takes off, how many are first on the lattice again at each later prefix (``back``,
  [prefix, count] pairs, prefix null if never before the end of the word); for the twists s + 1 to n + ``--detail``
  - 1 also the pieces themselves;
- every piece that goes off the lattice in the twists s + 1 to n + ``--detail`` - 1, with the first later prefix at
  which it is on the lattice again (null if never before the end of the word);
- at b and at e: digest, off count, and against the anchor: pieces restored, pieces newly off, and the off pieces at
  both prefixes, at the same exact pose or moved, and the members of the forcing blocks on the lattice (b is
  recorded even when it comes after e);
- the first prefix >= n at which all forced pieces are on the lattice together (tracked to the end of the word, after
  e too), and whether it is b.
A window not closed within ``--horizon`` (at least 1) twists of its forcing twist is reported with e null.

The moved set of each twist is the set the simulator certifies for it (captured at commit); pose ids are never
compared across twists, exact matrices are. The whole word is replayed, every twist checked admissible.

Usage (from the repository root)::

    python research/jumbling/theory/unlock.py --word <word file> --windows 9649:9654:9659,17882:17884:17886 \\
        --pre 8 [--tables <dir>] --out <file>

The word file holds {'word', 'input_digest', 'output_digest'}; it is replayed from the E2 end of I_a@40
(``orient.start``), or from ``--table``. The exit status is 0 only when the word starts at its input digest and ends
at its output digest, every window has forced pieces that the forcing twist takes off, b is the first prefix at which
they are all back, and every window closes within ``--horizon`` twists. Evidence: exact simulator replay, not native host evidence.
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
import buffer as B  # noqa: E402

CL, O, restore = BL.CL, BL.O, BL.restore
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--word', required=True)
    ap.add_argument('--table', help='current-only pose table of the word input (default: E2 end)')
    ap.add_argument('--windows', required=True, help='comma-separated s:n:b triples of executed-prefix counts')
    ap.add_argument('--pre', type=int, default=8, help='twists listed before each window')
    ap.add_argument('--horizon', type=int, default=4000, help='twists after which an unclosed window is reported')
    ap.add_argument('--detail', type=int, default=16, help='twists of each window listed piece by piece')
    ap.add_argument('--tables', help='directory for the pose table at each window start')
    ap.add_argument('--out', required=True)
    a = ap.parse_args(argv)
    if a.horizon < 1 or a.detail < 0 or a.pre < 0:
        raise ValueError('--horizon must be at least 1, --detail and --pre at least 0')
    doc = json.loads(Path(a.word).read_text())
    word = [(int(c), int(i)) for c, i in doc['word']]
    wins = sorted({tuple(int(x) for x in w.split(':')) for w in a.windows.split(',')})
    if any(len(w) != 3 or not 0 <= w[0] < w[1] <= w[2] <= len(word) for w in wins):
        raise ValueError('a window is not s:n:b with 0 <= s < n <= b <= word length')
    if len({w[0] for w in wins}) != len(wins) or len({w[1] for w in wins}) != len(wins):
        raise ValueError('two windows share an anchor or a forcing twist')
    R, M = O.start(a.table)
    st = R.st
    if st.digest() != doc['input_digest']:
        raise ValueError('word input digest differs from the start state')
    NP = len(st.pose_id)
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
    kcache = {}

    def kindex(c, i):
        if (c, i) not in kcache:
            kcache[(c, i)] = int(kp.index_of_matrix(restore.element(M, c, i)))
        return kcache[(c, i)]

    def kpose():
        return st._kpose()[st.pose_id]

    def occupancy():
        K = kpose()
        L = np.flatnonzero(K >= 0)
        occ = np.full(NP, -1, np.int64)
        occ[POF[ORB[L], kp.compose_vec(K[L], TRN[L])]] = L
        return occ

    def off_poses():
        return {int(q): B.matrix_json(st._pose_matrix(int(st.pose_id[q]))) for q in np.flatnonzero(kpose() < 0)}

    def compare(s, p):
        off0, off1 = s['off0'], off_poses()
        both = sorted(set(off0) & set(off1))
        moved = [q for q in both if off0[q] != off1[q]]
        return {'prefix': p, 'digest': st.digest(), 'off': len(off1), 'restored': sorted(set(off0) - set(off1)),
                'newly_off': sorted(set(off1) - set(off0)), 'off_at_both_same_pose': len(both) - len(moved),
                'off_at_both_moved': moved, 'forcing_members': int(s['members'].sum()),
                'forcing_members_on': int((kpose()[s['members']] >= 0).sum())}

    last = {}
    commit = st._commit

    def capture(twist, inside, journal):
        last['inside'] = np.asarray(inside, np.int64).copy()
        return commit(twist, inside, journal)

    st._commit = capture
    anchor_at = {s: (s, n, b) for s, n, b in wins}
    before_at = {n - 1: n for s, n, b in wins}
    report, live, pending, blocks_at = {}, {}, {}, {}

    def blocks(p):
        if p not in blocks_at:
            blocks_at.clear()
            blocks_at[p] = BL.Blocks(R, M)
        return blocks_at[p]

    def moved_of(bl, mv):
        out = []
        for j, m in enumerate(bl):
            inside = np.isin(m, mv)
            if inside.any():
                out.append({'block': j, 'size': int(len(m)), 'whole': bool(inside.all())})
        return out

    for p in range(len(word) + 1):
        if p in anchor_at:
            s0, n, b = anchor_at[p]
            w = {'window': [s0, n, b], 'start': {'prefix': p, 'digest': st.digest(), 'off': int((kpose() < 0).sum())},
                 'context_before': [{'prefix': q + 1, 'grip': word[q][0], 'menu': word[q][1],
                                     'k': kindex(*word[q]) if kindex(*word[q]) >= 0 else None}
                                    for q in range(max(0, p - a.pre), p)],
                 'twists': [], 'taken_off': [], 'at_b': None, 'closed_at': None, 'at_close': None}
            if a.tables:
                name = Path(a.tables) / f'prefix-{p}.json'
                name.parent.mkdir(parents=True, exist_ok=True)
                B.export_pose_table(st, name)
                w['start']['table'] = name.name
            off0 = off_poses()
            mask0 = np.zeros(NP, bool)
            mask0[list(off0)] = True
            ab = [np.asarray(m, np.int64) for m in blocks(p).blocks]
            w['start']['blocks'] = [int(len(m)) for m in ab]
            live[n] = {'w': w, 'off0': off0, 'mask0': mask0, 'members': np.zeros(NP, bool), 'lat_off': 0,
                       'open': True, 'ablocks': ab, 'forced': None, 'forced_back': None}
        if p in before_at:
            s = live[before_at[p]]
            bl = blocks(p)
            s['blocks'] = [np.asarray(m, np.int64) for m in bl.blocks]
            s['locked'], s['occ'] = bl.locked, occupancy()
            s['w']['before_forcing'] = {'prefix': p, 'digest': st.digest(), 'off': int(len(bl.off))}
        for n, s in live.items():
            w = s['w']
            if p == w['window'][2]:
                w['at_b'] = compare(s, p)
            f = s['forced']
            if f is not None and s['forced_back'] is None and len(f) and bool((kpose()[f] >= 0).all()):
                s['forced_back'] = p
            if not s['open']:
                continue
            if p >= n and s['lat_off'] == 0:
                w['closed_at'] = p
            if w['closed_at'] is not None or p - (n - 1) >= a.horizon:
                w['at_close'] = compare(s, p)
                s['open'] = False
                log(window=w['window'], closed_at=w['closed_at'], prefix=p, off=w['at_close']['off'])
        if p == len(word):
            break
        c, i = word[p]
        k = kindex(c, i)
        Kb = kpose()
        last.clear()
        R.apply(c, i)
        mv = np.sort(last['inside'])
        Ka = kpose()
        went = mv[(Kb[mv] >= 0) & (Ka[mv] < 0)]
        came = mv[(Kb[mv] < 0) & (Ka[mv] >= 0)]
        offmoved = int(((Kb[mv] < 0) & (Ka[mv] < 0)).sum())
        for q in came.tolist():
            for e in pending.pop(q, ()):
                e['back_at'] = p + 1
        digest = None
        for n, s in live.items():
            if not s['open']:
                continue
            w = s['w']
            digest = digest or st.digest()
            if p + 1 == n:
                forced, info = [], []
                for j, m in enumerate(s['blocks']):
                    inside = np.isin(m, mv)
                    if not inside.any():
                        continue
                    s['members'][m] = True
                    lk = np.asarray(s['locked'][j], np.int64)
                    content = s['occ'][lk]
                    info.append({'block': j, 'size': int(len(m)), 'smallest_piece': int(m[0]),
                                 'whole': bool(inside.all()), 'came_on': int((Ka[m] >= 0).sum()),
                                 'locked': lk.tolist(), 'content': [int(q) for q in content.tolist()]})
                    forced.extend(int(q) for q in content[content >= 0].tolist())
                w['forcing'] = {'element_outside_K': k < 0, 'blocks': info, 'forced': sorted(set(forced)),
                                'forced_went_off': sorted(set(forced) & set(went.tolist()))}
                s['forced'] = np.asarray(w['forcing']['forced'], np.int64)
            s['lat_off'] += int((~s['mask0'][went]).sum()) - int((~s['mask0'][came]).sum())
            detail = p + 1 < n + a.detail
            row = {'prefix': p + 1, 'grip': c, 'menu': i, 'k': k if k >= 0 else None, 'moved': int(len(mv)),
                   'anchor_blocks_moved': moved_of(s['ablocks'], mv),
                   'blocks_moved': moved_of(s['blocks'], mv) if p + 1 >= n else None,
                   'went_off': len(went), 'came_on': len(came),
                   'off_moved_staying_off': offmoved, 'off_after': int((Ka < 0).sum()), 'digest': digest}
            events = []
            for q in went.tolist():
                e = {'piece': int(q), 'off_at': p + 1, 'back_at': None}
                events.append(e)
                pending.setdefault(int(q), []).append(e)
            row['_events'] = events
            if detail:
                row['went_off_pieces'], row['came_on_pieces'] = went.tolist(), came.tolist()
                w['taken_off'].extend(events)
            w['twists'].append(row)
        if (p + 1) % 2000 == 0:
            log(prefix=p + 1, windows_open=sum(s['open'] for s in live.values()))
    end_ok = st.digest() == doc['output_digest']
    ok = end_ok
    for n, s in live.items():
        w = s['w']
        if s['open']:
            w['at_close'] = compare(s, len(word))
        for row in w['twists']:
            hist = {}
            for e in row.pop('_events'):
                hist[e['back_at']] = hist.get(e['back_at'], 0) + 1
            row['back'] = sorted(([b, m] for b, m in hist.items()), key=lambda x: (x[0] is None, x[0] or 0))
        forced = set(w['forcing']['forced'])
        first = s['forced_back']
        w['forced_back_at'] = first
        w['b_is_first_return'] = first is not None and first == w['window'][2]
        w['forced_pieces_ok'] = bool(forced) and set(w['forcing']['forced_went_off']) == forced
        e, s0 = w['closed_at'], w['window'][0]
        w['witness'] = None if e is None else {'from_prefix': s0, 'to_prefix': e, 'length': e - s0,
                                               'word': [list(x) for x in word[s0:e]]}
        ok = ok and w['b_is_first_return'] and w['forced_pieces_ok'] and e is not None
        report[str(n)] = w
    out = {'word_length': len(word), 'input_digest': doc['input_digest'], 'end_digest_ok': end_ok,
           'prefix_convention': 'prefix p = state after the first p twists; twist p = word[p - 1]; witness of '
                                'window s:n:b = word[s:e] (twists s + 1 to e, forcing twist n included), e the closing prefix',
           'windows': report, 'ok': ok, 'evidence': 'exact simulator replay', 'seconds': round(time.time() - T0)}
    Path(a.out).write_text(json.dumps(out) + '\n')
    print(json.dumps({'ok': ok, 'end_digest_ok': end_ok,
                      'windows': {n: {'b_first': w['b_is_first_return'], 'closed_at': w['closed_at'],
                                      'restored': len(w['at_close']['restored']),
                                      'newly_off': len(w['at_close']['newly_off']),
                                      'moved': len(w['at_close']['off_at_both_moved']),
                                      'members_on': [w['at_close']['forcing_members_on'],
                                                     w['at_close']['forcing_members']]}
                                  for n, w in report.items()}}))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
