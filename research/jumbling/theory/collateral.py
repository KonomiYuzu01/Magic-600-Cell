"""The state after a separating loop: glued pairs, collateral cells and vacant chambers (R1, card 4, E, part B).

A loop J C J^-1 C^-1 of split_pairs.py separates the glued pair and can put the whole fragment on the lattice, but
it takes lattice pieces off it (the collateral). Before any word is built to return the collateral, this script
reads, for each selected loop applied on the exact simulator at the start state:
- glued pairs after the loop, with the test of split_pairs.py (an off-lattice piece whose inside signature over
  the 600 cuts is a chamber's home signature, the chamber held by a lattice piece, and both pieces strictly on the
  same side of every cut). No retained word separates such a pair;
- the off-lattice domains and their classes;
- the collateral by the cell of its class that holds it now: pieces, chambers of that cell, lattice pieces in it now
  (a shared cell: no word of that class alone returns the collateral and keeps them), and the cells of the same
  class whose chambers the collateral came from. A cell without chambers lies outside the outline;
- for every class, the cells of the chambers the collateral came from: how many of each cell's occupied chambers
  (before the loop) the collateral held (all of them: a whole cell), and whether the fragment landed in that cell;
- vacant chambers. A chamber holds at most one piece and every piece type has as many chambers as pieces, so the
  vacant chambers of a type number the off-lattice pieces of that type. The script lists the chambers the fragment
  landed in and the chambers the collateral left, and counts the chambers the loop fills (vacant before, occupied
  after), empties (occupied before, vacant after) and leaves vacant.
- what a clean run of the loop needs. A twist moves a region exactly when the region lies inside its cap, whatever
  the other pieces are, so the way a lattice piece goes through the loop depends only on its chamber. For every
  vacant chamber inside a cap of the loop the script follows the chamber's region through the four twists (exact, as
  the simulator does) and finds whether a piece there would stay on the lattice, leave it, or straddle a cut on the
  way and block the twist; a chamber outside both caps never moves. These paths are checked against the simulator on
  the chambers the collateral left, on the landing chambers (backwards) and on every 25th other lattice piece inside
  a cap (both ways). Whenever the loop runs admissibly and takes no piece off the lattice, the chambers whose pieces
  it would take off or that would block it are vacant: the chambers the collateral left and the vacant chambers
  found here to leave or block. So at that time at least that many pieces are off the lattice
  (`off_lattice_needed_for_a_clean_run`). The other vacant chambers are the ones that can stand in for the chambers
  the collateral left, and vacancies move only within a piece type: if the pieces off the lattice now stay off, then
  at least as many further pieces as the collateral's chambers that they cannot match by type must be off the
  lattice when the loop runs (`collateral_origin_unmatched_by_type`). Conversely, if everything were as here except
  that the chambers the collateral left were vacant and their pieces lay outside both caps, the loop would move
  every other piece as it does here and take nothing off the lattice (removing pieces never makes a cut straddled).
- what the route L M L^-1 needs (run the loop, move the pieces it put on the lattice, run the loop backwards; the
  backward loop returns the collateral if its pieces are where the loop left them). Run on the state after the
  loop, L^-1 takes exactly the pieces the loop put on the lattice back off. So whenever L^-1 runs admissibly and
  takes no piece off, their landing chambers are vacant, and so is every chamber whose piece L^-1 would take off or
  that would block it; the script follows every chamber vacant after the loop inside a cap through L^-1 as above.
  If M keeps the number of pieces off the lattice, the landing chambers' pieces must then fit, type by type, in the
  chambers vacant after the loop that L^-1 keeps on the lattice; the script counts the shortfall (further pieces that
  would have to be off the lattice), and the shortfall if they are moved to vacant chambers outside both caps. The
  loop maps the vacant chambers before it that it keeps on the lattice one to one onto the vacant chambers after it
  that L^-1 keeps (a path read backwards is the backward path), so the shortfall is at most the number of vacant
  chambers before the loop that it would take a piece off from or that would block it: the count is a check, and
  what this route needs is the word M.
Each loop is undone afterwards and the start digest checked.

The loops are read from a split_pairs.py result (results file or --full file): the legal loops that put --joined
pieces on the lattice (default: the most any legal loop puts). By default a first pass applies every such loop and
reads only the vacant chambers, and the --limit loops that leave the fewest collateral chambers unmatched by type
are analysed in full (ties: fewest pieces taken out). --order left skips that pass and takes the loops with the
fewest pieces taken out.

Run from the repository root:
    python research/jumbling/theory/collateral.py --words research/jumbling/theory/results/e-chain/*.json \
        research/jumbling/theory/results/e-chain-104/*.json \
        --loops research/jumbling/theory/results/split-pairs-104.json --limit 60 --out <file>
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import orient as O  # noqa: E402
import split_pairs as SP  # noqa: E402
from exact import identity, matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


class After:
    def __init__(self, R, members, cls):
        self.R, self.st, self.members, self.cls = R, R.st, members, cls
        st = self.st
        self.NP = NP = len(st.pose_id)
        self.home = home = CL.data.sig_matrix(np.arange(NP))
        self.lookup = {r.tobytes(): x for x, r in enumerate(np.packbits(home, axis=1))}
        assert len(self.lookup) == NP, 'two chambers with one home signature'
        self.cell, self.nchambers, self.caps, self.cell_of_key = [], [], [], []
        for k in range(5):
            keys = np.packbits(home[:, members[k]], axis=1)
            u, inv, cnt = np.unique(keys, axis=0, return_inverse=True, return_counts=True)
            self.cell.append(inv.ravel())
            self.nchambers.append(cnt)
            self.caps.append([[int(members[k][j]) for j in np.flatnonzero(np.unpackbits(r)[:120])] for r in u])
            self.cell_of_key.append({r.tobytes(): n for n, r in enumerate(u)})
        self.type = np.asarray(CL.ctx.regions.orbit_of)      # chamber s has the type of piece s
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        self.identity = to_tuple(identity(4))
        k = kp.index_of_matrix(self.identity)
        at_home = reg.piece_of[reg.orbit_of, kp.compose_vec(np.full(NP, k), reg.transport)]
        assert np.array_equal(at_home, np.arange(NP)), 'piece x at the identity pose is not in chamber x'
        self.digest0 = st.digest()
        self.K0 = st._kpose()[st.pose_id]
        self.lat0 = self.K0 >= 0
        self.ch0 = self.chambers(self.K0)
        self.vacant0 = np.ones(NP, bool)
        self.vacant0[self.ch0[self.lat0]] = False
        self.occupied0 = [np.bincount(self.cell[k][self.ch0[self.lat0]], minlength=len(self.nchambers[k]))
                          for k in range(5)]

    def chambers(self, K):
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        L = np.flatnonzero(K >= 0)
        out = np.full(self.NP, -1, np.int64)
        out[L] = reg.piece_of[reg.orbit_of[L], kp.compose_vec(K[L], reg.transport[L])]
        return out

    def types(self, chambers):
        u, c = np.unique(self.type[np.asarray(chambers, np.int64)], return_counts=True)
        return {int(a): int(b) for a, b in zip(u, c)}

    @staticmethod
    def matched(t1, t2):
        return sum(min(v, t2.get(k, 0)) for k, v in t1.items())

    def path(self, x, word):
        """Chamber that a lattice piece in chamber x would reach under the word: -1 when its region leaves the lattice,
        -2 when the region straddles the cut of a twist on the way (the twist would not be admissible). A twist moves
        a region exactly when the region lies inside the cap, so this holds for the piece in chamber x of any state
        (piece x at the identity pose fills chamber x)."""
        pose = self.identity
        for c, i in word:
            s = SP.side(pose, x, c)
            if s == 'straddle':
                return -2
            if s == 'in':
                pose = matmul(self.R.g(c, i), pose)
            elif s != 'out':
                raise AssertionError(f'uncertain side of chamber {x} at grip {c}')
        reg, kp = CL.ctx.regions, CL.ctx.kplus
        k = kp.index_of_matrix(pose)
        if k < 0:
            return -1
        return int(reg.piece_of[reg.orbit_of[x], kp.compose(k, int(reg.transport[x]))])

    def inside_caps(self, word):
        return self.home[:, sorted({int(c) for c, _ in word})].any(axis=1)

    def loop(self, rec, full=True):
        """Apply a loop, read the state after it (everything, or only the vacant chambers) and undo it."""
        R, st = self.R, self.st
        word = [tuple(s) for s in rec['word']]
        done = 0
        try:
            for s in word:
                R.apply(*s)                                 # raises if a twist is not admissible
                done += 1
            if full:
                return self._read(rec, word)
            K1 = st._kpose()[st.pose_id]
            lat1 = K1 >= 0
            return {'word': [list(s) for s in word], 'left_lattice': int((self.lat0 & ~lat1).sum()),
                    'off_lattice_after': int((~lat1).sum()),
                    **self._vacancies(word, self.chambers(K1), lat1, np.flatnonzero(self.lat0 & ~lat1),
                                      np.flatnonzero(~self.lat0 & lat1), lists=False)}
        finally:
            for _ in range(done):
                R.undo()
            if st.digest() != self.digest0:
                raise AssertionError('the loop was not undone')

    def _vacancies(self, word, ch1, lat1, left, joined, lists=True):
        occupied = np.zeros(self.NP, bool)
        occupied[ch1[lat1]] = True
        filled = np.flatnonzero(self.vacant0 & occupied)        # vacant before the loop, occupied after
        emptied = np.flatnonzero(~self.vacant0 & ~occupied)     # occupied before, vacant after
        other = np.flatnonzero(self.vacant0 & ~occupied)        # vacant before and after
        origin = self.ch0[left]
        # vacant chambers before the loop that a piece placed there would leave (or block the loop) from;
        # chambers outside both caps never move
        ends = {x: self.path(x, word) for x in np.flatnonzero(self.vacant0 & self.inside_caps(word)).tolist()}
        taken = sorted(x for x, y in ends.items() if y == -1)
        blocking = sorted(x for x, y in ends.items() if y == -2)
        kept = np.setdiff1d(np.flatnonzero(self.vacant0), np.array(taken + blocking, np.int64))
        to, tk = self.types(origin), self.types(kept)
        out = {'vacant_chambers_after': int(self.NP - lat1.sum()),
               'filled_chambers': int(len(filled)), 'emptied_chambers': int(len(emptied)),
               'emptied_are_the_collateral_origin': bool(np.array_equal(np.sort(emptied), np.sort(origin))),
               'vacant_before_and_after': int(len(other)),
               'vacant_before_kept_on_the_lattice': int(len(kept)),
               'vacant_before_taken_off': len(taken), 'vacant_before_blocking': len(blocking),
               'off_lattice_needed_for_a_clean_run': int(len(origin)) + len(taken) + len(blocking),
               'collateral_origin_matched_by_type': self.matched(to, tk),
               'collateral_origin_unmatched_by_type': int(len(origin)) - self.matched(to, tk)}
        if lists:
            out.update({'landing_chambers': sorted(ch1[joined].tolist()), 'collateral_origin_chambers': sorted(origin.tolist()),
                        'vacant_before_taken_off_chambers': taken, 'vacant_before_blocking_chambers': blocking,
                        'types': {'filled': self.types(filled), 'collateral_origin': to,
                                  'vacant_before_and_after': self.types(other), 'vacant_before_kept_on_the_lattice': tk}})
        return out

    def _check_paths(self, word, back, left, joined, ch1):
        """The paths agree with the simulator: every chamber the collateral left leads off the lattice under the loop
        and every landing chamber under the loop backwards, and every 25th other lattice piece inside a cap ends in the
        chamber its path gives, both ways."""
        for x in self.ch0[left].tolist():
            if self.path(int(x), word) != -1:
                raise AssertionError(f'path of chamber {x} disagrees with the simulator')
        for x in ch1[joined].tolist():
            if self.path(int(x), back) != -1:
                raise AssertionError(f'backward path of chamber {x} disagrees with the simulator')
        stay = np.flatnonzero(self.lat0 & (ch1 >= 0))
        stay = stay[self.inside_caps(word)[self.ch0[stay]]][::25]
        for p in stay.tolist():
            if self.path(int(self.ch0[p]), word) != int(ch1[p]) or self.path(int(ch1[p]), back) != int(self.ch0[p]):
                raise AssertionError(f'path of chamber {int(self.ch0[p])} disagrees with the simulator')
        return int(len(left) + len(joined) + 2 * len(stay))

    def backward(self, word):
        """L^-1 as menu pairs: the loop is [(d, i), (c, x), (d, i^-1), (c, x^-1)]."""
        g = [self.R.g(*s) for s in word]
        if to_tuple(matmul(g[2], g[0])) != self.identity or to_tuple(matmul(g[3], g[1])) != self.identity \
                or word[0][0] != word[2][0] \
                or word[1][0] != word[3][0]:
            raise AssertionError(f'{word} is not a loop J C J^-1 C^-1')
        return [word[1], word[0], word[3], word[2]]

    def _route_back(self, word, back, ch1, lat1, joined):
        occupied = np.zeros(self.NP, bool)
        occupied[ch1[lat1]] = True
        vacant1 = np.flatnonzero(~occupied)
        inside = self.inside_caps(word)
        ends = {x: self.path(x, back) for x in vacant1[inside[vacant1]].tolist()}
        taken = sorted(x for x, y in ends.items() if y == -1)
        blocking = sorted(x for x, y in ends.items() if y == -2)
        kept = np.setdiff1d(vacant1, np.array(taken + blocking, np.int64))
        tl, tk, to = self.types(ch1[joined]), self.types(kept), self.types(vacant1[~inside[vacant1]])
        return {'vacant_after_kept_on_the_lattice_by_it': int(len(kept)),
                'vacant_after_taken_off_by_it': len(taken), 'vacant_after_blocking_it': len(blocking),
                'vacant_after_taken_off_by_it_chambers': taken, 'vacant_after_blocking_it_chambers': blocking,
                'further_off_lattice_needed': sum(max(0, v - tk.get(t, 0)) for t, v in tl.items()),
                'further_off_lattice_needed_to_move_them_outside_both_caps': sum(max(0, v - to.get(t, 0)) for t, v in tl.items()),
                'types': {'landing': tl, 'vacant_after_kept_on_the_lattice_by_it': tk,
                          'vacant_after_outside_both_caps': to}}

    def _read(self, rec, word):
        st, members, NP = self.st, self.members, self.NP
        back = self.backward(word)
        K1 = st._kpose()[st.pose_id]
        lat1 = K1 >= 0
        ch1 = self.chambers(K1)
        off = np.flatnonzero(~lat1)
        row = np.full(NP, -1, np.int64)
        row[off] = np.arange(len(off))
        sig = np.zeros((len(off), 600), bool)                # inside signature of every off-lattice piece
        for c in range(600):
            cl = st.classify(int(c), need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            ins = np.asarray(cl.inside, np.int64)
            ins = ins[~lat1[ins]]
            sig[row[ins], c] = True
        occupant = np.full(NP, -1, np.int64)
        occupant[ch1[lat1]] = np.flatnonzero(lat1)
        glued = []
        for n, p in enumerate(off.tolist()):
            x = self.lookup.get(np.packbits(sig[n]).tobytes())
            if x is None or occupant[x] < 0:
                continue
            q = int(occupant[x])
            sp, sq = SP.sides(st.pose(p), p), SP.sides(st.pose(q), q)
            if sp == sq and 'straddle' not in sp:
                glued.append([p, q])
        D = self.R.domains()
        dom = np.asarray(D.dom)
        dcls = {}
        for d in range(1, len(D.reps)):
            h = CL.hub(D.reps[d], members, self.cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        domains = {}
        for p in off.tolist():
            d = int(dom[p])
            domains.setdefault(d, {'pieces': 0, 'class': dcls[d]})['pieces'] += 1
        left = np.flatnonzero(self.lat0 & ~lat1)
        joined = np.flatnonzero(~self.lat0 & lat1)
        occupied1 = [np.bincount(self.cell[k][ch1[lat1]], minlength=len(self.nchambers[k])) for k in range(5)]
        cells = {}
        for p in left.tolist():
            k = dcls[int(dom[p])]
            if k is None:
                g = cells.setdefault(('none', int(dom[p])), {'class': None, 'domain': int(dom[p]), 'pieces': 0})
                g['pieces'] += 1
                continue
            inside = sig[row[p]][members[k]]
            f = self.cell_of_key[k].get(np.packbits(inside).tobytes())
            caps = [int(members[k][j]) for j in np.flatnonzero(inside)]
            g = cells.setdefault((k, tuple(caps)), {
                'class': k, 'domain': int(dom[p]), 'cell': caps, 'pieces': 0,
                'chambers': 0 if f is None else int(self.nchambers[k][f]),
                'lattice_pieces_now': 0 if f is None else int(occupied1[k][f]), 'came_from': {}})
            g['pieces'] += 1
            src = str(self.caps[k][self.cell[k][self.ch0[p]]])
            g['came_from'][src] = g['came_from'].get(src, 0) + 1
        cells = sorted(cells.values(), key=lambda g: -g['pieces'])
        origin = {}
        for k in range(5):
            fs, cnt = np.unique(self.cell[k][self.ch0[left]], return_counts=True)
            landed = set(self.cell[k][ch1[joined]].tolist())
            rows = [{'cell': self.caps[k][f], 'collateral': int(n), 'occupied_before': int(self.occupied0[k][f]),
                     'chambers': int(self.nchambers[k][f]), 'fragment_landed_here': f in landed}
                    for f, n in zip(fs.tolist(), cnt.tolist())]
            origin[str(k)] = {'cells': len(rows),
                              'whole_cells': sum(r['collateral'] == r['occupied_before'] for r in rows),
                              'cells_where_the_fragment_landed': sum(r['fragment_landed_here'] for r in rows),
                              'rows': sorted(rows, key=lambda r: -r['collateral'])}
        out = {'word': [list(s) for s in word], 'end_digest': st.digest(),
               'retained': [bool(SP.retained(self.R.g(*word[0]))), bool(SP.retained(self.R.g(*word[1])))],
               'joined_lattice': int(len(joined)), 'left_lattice': int(len(left)), 'off_lattice_after': int(len(off)),
               'glued_pairs_after': glued, 'off_domains': domains,
               'collateral_by_cell_now': cells,
               'collateral_in_shared_cells': sum(g['pieces'] for g in cells if g.get('lattice_pieces_now', 0) > 0),
               'collateral_outside_the_outline': sum(g['pieces'] for g in cells
                                                     if g['class'] is not None and g['chambers'] == 0),
               'collateral_origin_by_class': origin,
               'paths_checked': self._check_paths(word, back, left, joined, ch1),
               **self._vacancies(word, ch1, lat1, left, joined),
               'backward_route': self._route_back(word, back, ch1, lat1, joined)}
        if 'end_digest' in rec and out['end_digest'] != rec['end_digest']:
            raise AssertionError(f"{rec['word']}: end digest differs from the split_pairs record")
        return out


def candidates(doc, joined):
    loops = [l for c in doc['candidates'] for l in c.get('loops', []) if l['twists_admissible'] == 4]
    top = max(l['joined_lattice'] for l in loops) if joined is None else joined
    return top, [l for l in loops if l['joined_lattice'] == top]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--table', help='current-only pose table to start from (default: the E2 end)')
    ap.add_argument('--words', nargs='*', default=[], help='word files replayed after the start')
    ap.add_argument('--loops', required=True, help='split_pairs.py result at the same state')
    ap.add_argument('--joined', type=int, help='loops that put this many pieces on the lattice (default: the most)')
    ap.add_argument('--order', choices=('unmatched', 'left'), default='unmatched',
                    help='unmatched: a vacancy pass over every such loop, then the loops that leave the fewest collateral '
                         'chambers unmatched by type; left: the loops with the fewest pieces taken out')
    ap.add_argument('--limit', type=int, default=60, help='loops analysed in full')
    ap.add_argument('--out', required=True)
    a = ap.parse_args()
    doc = json.loads(Path(a.loops).read_text())
    members, cls = CL.classes()
    R, M = O.start(a.table, a.words)
    if R.st.digest() != doc['source_digest']:
        raise AssertionError('the loops were found at another state')
    after = After(R, members, cls)
    top, loops = candidates(doc, a.joined)
    vac = []
    if a.order == 'unmatched':
        for n, l in enumerate(loops):
            vac.append(after.loop(l, full=False))
            if n % 100 == 99:
                log(vacancy_pass=n + 1, of=len(loops))
        key = {json.dumps(v['word']): (v['collateral_origin_unmatched_by_type'], v['left_lattice'], v['off_lattice_after'])
               for v in vac}
        pick = sorted(loops, key=lambda l: key[json.dumps(l['word'])] + (l['word'],))[:a.limit]
    else:
        pick = sorted(loops, key=lambda l: (l['left_lattice'], l['off_lattice_after'], l['word']))[:a.limit]
    rows = []
    for l in pick:
        rows.append(after.loop(l))
        r = rows[-1]
        log(loop=len(rows), word=r['word'], left=r['left_lattice'], off=r['off_lattice_after'],
            unmatched=r['collateral_origin_unmatched_by_type'], needed=r['off_lattice_needed_for_a_clean_run'],
            back=r['backward_route']['further_off_lattice_needed'], glued_after=len(r['glued_pairs_after']),
            shared=r['collateral_in_shared_cells'], outside=r['collateral_outside_the_outline'])
    out = {'start': 'E2 end of I_a@40' if a.table is None else 'current-only pose table',
           'words': [Path(w).name for w in a.words], 'source_digest': after.digest0,
           'off_lattice': int((~after.lat0).sum()), 'loops_from': Path(a.loops).name,
           'joined': top, 'loops_with_that_many_joined': len(loops), 'order': a.order, 'analysed': len(rows),
           'with_no_glued_pair_after': sum(not r['glued_pairs_after'] for r in rows),
           'fewest_glued_pairs_after': min((len(r['glued_pairs_after']) for r in rows), default=None),
           'fewest_unmatched': min((v['collateral_origin_unmatched_by_type'] for v in vac or rows), default=None),
           'fewest_off_lattice_needed_for_a_clean_run': min((v['off_lattice_needed_for_a_clean_run'] for v in vac or rows),
                                                            default=None),
           'backward_route_further_off_lattice_needed': sorted({r['backward_route']['further_off_lattice_needed'] for r in rows}),
           'loops': rows, 'vacancy_pass': vac, 'seconds': round(time.time() - T0)}
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps({k: out[k] for k in ('source_digest', 'joined', 'loops_with_that_many_joined', 'analysed',
                                          'with_no_glued_pair_after', 'fewest_glued_pairs_after', 'fewest_unmatched',
                                          'fewest_off_lattice_needed_for_a_clean_run',
                                          'backward_route_further_off_lattice_needed')}))


if __name__ == '__main__':
    main()
