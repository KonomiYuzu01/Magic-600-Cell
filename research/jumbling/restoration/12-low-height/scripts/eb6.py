# E-B6, height 4 (template T4 of option B): the high units of a state and the four-letter moves that lower them. A
# high unit is a set of off pieces of left height at least 4 with one pose and one inside vector over the 600 caps
# (E21-3). For each one:
# - the caps that hold it whole, which of them are clean (no piece straddles the cut) and which lie in its own tree
#   branch; at each own-branch cap the 12 menu items that lower it by 2 (R21F-03) and what else the cap holds, since
#   one lowering letter there moves the whole cap; for a blocked cap, how many off pieces straddle its cut (record and
#   witness inputs);
# - the lens pairs of R21F-02: an admissible lowering cap x and another clean cap y that both hold it whole, with K
#   the set of pieces whole in both caps;
# - every strict four-letter commutator (x, a)(y, b)(x, a^-1)(y, b^-1): every twist admissible, every piece outside the
#   unit back at its exact pose (lattice pieces included), every piece of the unit moved, to one common pose. The lens
#   words P1 and P2 are such commutators (P2 at (x, y) is the commutator at (y, x)). Exact necessary conditions, proved
#   in the stage 12 README (section 9, Lemma E6), prune the search: x is clean at Z; the straddlers of y at Z belong to
#   the unit; the unit lies whole in x or whole in y; the caps share a point of the ball that holds every piece (then
#   y lies within 55.7 degrees of x, eb6_meet.py); a piece outside the unit that lies whole in both caps returns only
#   when the two items commute; and if the unit is not whole in y, the first letter carries it whole into y, and if it
#   is not whole in x, the second letter carries it whole into x. So x runs over the clean caps that hold the unit
#   whole with y within 55.7 degrees of x, and y over the clean caps that hold it whole with x a clean cap within 55.7
#   degrees of y that does not; the letters that must carry the unit into the other cap are tested exactly first. For
#   a pair whose common pieces all belong to the unit every remaining item pair is replayed (after one more exact test
#   after the first letter), and for any other pair only the commuting item pairs are (a float screen confirmed
#   exactly). Each word is undone after its check, and the state's digest is checked at the end. The output keeps
#   every strict word that lowers the unit (up to 500), the first 20 other strict words, the left heights after all
#   strict words, and the counts per cap pair.
# Exact geometry; research only, headless Linux; no Windows, DirectX, input or performance claim.
# Run from the repository root:
#   python3 -I -B eb6.py record <census record json> <out json> [--comm4] [--units <first,...>] [--comm4-seconds S]
#   python3 -I -B eb6.py fixture <records> <out json> [...]
#   python3 -I -B eb6.py witness <stage 10 witness json> <prefix length> <out json> [...]
# The record form rebuilds the end of a stage 11 census record from its seed and moves; the fixture form the first
# <records> records of the W-J fixture of I_a; the witness form the first <prefix length> letters of a stage 10
# witness from the solved state (the positive control: X is the prefix 156 of H6030). --comm4 runs the commutator
# search, on every high unit or on the units named by their first piece, within S seconds in all (default 5400);
# units it does not reach are listed as not run.
import json
import sys
import time
from pathlib import Path

import numpy as np

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures'), str(Path(__file__).resolve().parent)]
import restore as R  # noqa: E402
import sim  # noqa: E402
import trees as T  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from sim.state import Classification  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def options(argv):
    opt = {'comm4': False, 'units': None, 'comm4_seconds': 5400.0}
    rest = []
    i = 1
    while i < len(argv):
        a = argv[i]
        if a == '--comm4':
            opt['comm4'] = True
        elif a == '--units':
            opt['units'] = {int(v) for v in argv[i + 1].split(',')}
            i += 1
        elif a == '--comm4-seconds':
            opt['comm4_seconds'] = float(argv[i + 1])
            i += 1
        elif a.startswith('--'):
            raise SystemExit(f'unknown option {a}')
        else:
            rest.append(a)
        i += 1
    return rest, opt


def load(args):
    mode = args[0]
    if mode == 'record':
        from screen import load as screen_load
        st, M, src = screen_load('record', args[1])
        return st, M, src, args[2]
    C = sim.get_context()
    if mode == 'fixture':
        import wj
        fx, M = wj.load(C, 'I_a')
        n = int(args[1])
        st = sim.State(C, menu=M)
        for rec in fx['journal']['records'][:n]:
            assert st.apply(sim.Twist.from_record(C, rec)).applied
        return st, M, {'fixture': 'wj-I_a', 'records': n}, args[2]
    if mode == 'witness':
        M = R.menu('I_a')
        word = json.loads(Path(args[1]).read_text())['word']
        n = int(args[2])
        st = sim.State(C, filtered=False, menu=M)
        for c, j in word[:n]:
            o = st.apply(sim.Twist(C, c, R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
            assert o.applied and not o.uncertain
        return st, M, {'witness': Path(args[1]).name, 'prefix': n}, args[3]
    raise SystemExit(f'unknown mode {mode}')


_SID = {}


def stable_pose(st):
    """Per piece, an id of its exact pose that stays fixed across twists: the simulator renumbers its pose table
    after every twist (canonical order), so pose ids from different moments cannot be compared."""
    ids = np.array([_SID.setdefault(q.key, len(_SID)) for q in st._poses], np.int64)
    return ids[st.pose_id]


def straddler_set(st, e):
    """Every off piece whose pose straddles the cut of grip e (the simulator's classification, without stopping)."""
    D, reg = st.ctx.data, st.ctx.regions
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    cl = Classification(int(e))
    out = []
    for pid, members in st._groups(off):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), D.N[e]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, reg.cap_vform(anchor), lambda a=anchor: reg.cap_frows(a), cl, None)
            assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                continue
            for p in sub.tolist():
                r = st._eval(ev, reg.vform(p), lambda q=p: reg.frows(q), cl, None)
                assert r[0] != 'uncertain'
                if r[0] == 'straddle':
                    out.append(p)
    return np.array(sorted(out), np.int64)


class Comm4:
    """The strict four-letter commutators of one unit at Z (see the head of this file)."""

    def __init__(self, st, M, inside, clean, pid0):
        self.st, self.M, self.C = st, M, st.ctx
        self.inside, self.clean, self.pid0 = inside, clean, pid0
        F = np.array([[[float(v) for v in row] for row in M.frame(c)] for c in range(600)])
        Im = np.array([[[float(v) for v in row] for row in m] for _, m in M.items])
        self.E = np.einsum('cij,njk,clk->cnil', F, Im, F)          # float item matrices E[c, n]
        items = [to_tuple(m) for _, m in M.items]
        self.inv = [next(j for j in range(60) if to_tuple(matmul(items[i], items[j])) == items[0]) for i in range(60)]
        self.strad = {}

    def letter(self, c, j):
        return sim.Twist(self.C, c, R.element(self.M, c, j), 'menu', {'menu': self.M.name})

    def apply(self, c, j):
        o = self.st.apply(self.letter(c, j))
        assert not o.uncertain
        return o.applied

    def commuting(self, x, y):
        """Item pairs (a, b), both non-identity, whose rotations at x and y commute (float screen, exact check)."""
        A = self.E[x, 1:]
        B = self.E[y, 1:]
        Cm = np.einsum('aij,bjk->abik', A, B) - np.einsum('bij,ajk->abik', B, A)
        out = []
        for a, b in zip(*np.nonzero(np.abs(Cm).max(axis=(2, 3)) < 1e-9)):
            a, b = int(a) + 1, int(b) + 1
            P, Q = R.element(self.M, x, a), R.element(self.M, y, b)
            if to_tuple(matmul(P, Q)) == to_tuple(matmul(Q, P)):
                out.append((a, b))
        return out

    def straddlers(self, y):
        if y not in self.strad:
            self.strad[y] = straddler_set(self.st, y)
        return self.strad[y]

    def carries_into(self, U, c, j, target):
        """True when the letter (c, j) at Z carries every piece of U whole into the cap of target (exact)."""
        st, reg = self.st, self.C.regions
        g = matmul(R.element(self.M, c, j), st._pose_matrix(int(st.pose_id[U[0]])))
        ev = Evaluator(pullback_form(g, self.C.data.N[target]))
        return all(ev.classify(reg.vform(p))[0] == 'in' for p in U.tolist())

    def run(self, U, dl, caps_whole):
        st, inside, clean = self.st, self.inside, self.clean
        U = np.asarray(U)
        cw = set(caps_whole)
        cwc = [x for x in caps_whole if clean[x]]
        cand = {}
        for x in cwc:                                    # the unit lies whole in the first cap
            for y in R.NEAR[x].tolist():
                if y != x:
                    cand[(x, y)] = 'A'
        for y in cwc:                                    # the unit lies whole in the last cap only
            for x in R.NEAR[y].tolist():
                if x != y and clean[x] and x not in cw:
                    cand[(x, y)] = 'B'
        out = {'clean_caps_whole': cwc, 'pairs': len(cand), 'pairs_last_cap_blocked': 0, 'pairs_unit_not_carried': 0,
               'pairs_all_items': 0, 'pairs_commuting_items': 0, 'commuting_item_pairs': 0, 'first_letters': 0,
               'words': 0, 'admissible': 0, 'strict': 0, 'strict_lowering': 0, 'height_after': {}, 'by_pair': [],
               'strict_words': [], 'strict_words_other': []}
        items = list(range(1, 60))
        for (x, y), case in sorted(cand.items()):
            if not clean[y] and not np.isin(self.straddlers(y), U).all():
                out['pairs_last_cap_blocked'] += 1
                continue
            first = second = items
            if y not in cw:                              # the first letter must carry the unit whole into y
                first = [a for a in items if self.carries_into(U, x, a, y)]
            if x not in cw:                              # the second letter must carry it whole into x
                second = [b for b in items if self.carries_into(U, y, b, x)]
            if not first or not second:
                out['pairs_unit_not_carried'] += 1
                continue
            both = np.intersect1d(inside[x], inside[y], assume_unique=True)
            pp = {'x': x, 'y': y, 'case': case, 'first_items': len(first), 'second_items': len(second), 'words': 0,
                  'admissible': 0, 'strict': 0, 'strict_lowering': 0}
            if len(both) == np.isin(both, U).sum():
                out['pairs_all_items'] += 1
                pp['items'] = 'all'
                self.all_items(x, y, U, dl, out, pp, first, second)
            else:
                fs, ss = set(first), set(second)
                pairs = [(a, b) for a, b in self.commuting(x, y) if a in fs and b in ss]
                out['pairs_commuting_items'] += 1
                out['commuting_item_pairs'] += len(pairs)
                pp['items'] = f'commuting ({len(pairs)})'
                for a, b in pairs:
                    if self.apply(x, a):
                        out['first_letters'] += 1
                        self.rest(x, y, a, b, U, dl, out, pp)
                        st.undo()
            out['by_pair'].append(pp)
        return out

    def all_items(self, x, y, U, dl, out, pp, first, second):
        st = self.st
        xu = np.setdiff1d(self.inside[x], U, assume_unique=True)
        for a in first:
            if not self.apply(x, a):
                continue
            out['first_letters'] += 1
            cl = st.classify(y, need_inside=True, stop_at_straddle=True)
            assert not cl.uncertain
            # the second letter must be admissible, and no piece of x other than the unit may enter y
            if cl.straddle is None and not np.isin(cl.inside, xu).any():
                for b in second:
                    self.rest(x, y, a, b, U, dl, out, pp)
            st.undo()

    def rest(self, x, y, a, b, U, dl, out, pp):
        st = self.st
        done = 0
        for c, j in ((y, b), (x, self.inv[a]), (y, self.inv[b])):
            if not self.apply(c, j):
                break
            done += 1
        out['words'] += 1
        pp['words'] += 1
        if done == 3:
            out['admissible'] += 1
            pp['admissible'] += 1
            changed = np.flatnonzero(stable_pose(st) != self.pid0)
            if np.array_equal(changed, U) and (st.pose_id[U] == st.pose_id[U[0]]).all():
                hl, hr, bl, _ = T.pose_tree(st._pose_matrix(int(st.pose_id[U[0]])))
                out['strict'] += 1
                pp['strict'] += 1
                h = str(int(hl))
                out['height_after'][h] = out['height_after'].get(h, 0) + 1
                w = {'word': [[x, a], [y, b], [x, self.inv[a]], [y, self.inv[b]]], 'height_after': int(hl),
                     'right_height_after': int(hr), 'branch_after': int(bl)}
                if hl < dl:
                    out['strict_lowering'] += 1
                    pp['strict_lowering'] += 1
                    if len(out['strict_words']) < 500:
                        out['strict_words'].append(w)
                elif len(out['strict_words_other']) < 20:
                    out['strict_words_other'].append(w)
        for _ in range(done):
            st.undo()


def main():
    args, opt = options(sys.argv)
    st, M, src, out_path = load(args)
    st.journal = []
    C = st.ctx
    D = C.data
    digest0 = st.digest()
    log(loaded=src, digest=digest0)
    kp = st._kpose()[st.pose_id]
    lat = kp >= 0
    off = np.flatnonzero(~lat)
    info = {}
    for pid in np.unique(st.pose_id[off]).tolist():
        dl, dr, bl, br = T.pose_tree(st._pose_matrix(pid))
        assert dr == 0, 'a right height is not 0'
        info[pid] = (int(dl), int(bl))
    hgt = np.zeros(len(st.pose_id), np.int64)
    for pid, (dl, bl) in info.items():
        hgt[st.pose_id == pid] = dl
    heights = {}
    for h in hgt[off].tolist():
        heights[h] = heights.get(h, 0) + 1
    high = off[hgt[off] >= 4]
    log(off=int(len(off)), off_poses=len(info), heights=dict(sorted(heights.items())), high=int(len(high)))
    GB = [T.branch(tuple(D.N[x])) for x in range(600)]
    inside, clean = [], np.zeros(600, bool)
    for x in range(600):
        cl = st.classify(x, need_inside=True, stop_at_straddle=False)
        assert not cl.uncertain
        clean[x] = cl.straddle is None
        inside.append(cl.inside)
    log(caps_classified=True, clean=int(clean.sum()))
    ins = np.zeros((len(high), 600), bool)
    for x in range(600):
        ins[:, x] = np.isin(high, inside[x])
    groups = {}
    for r, p in enumerate(high.tolist()):
        groups.setdefault((int(st.pose_id[p]), ins[r].tobytes()), []).append(r)
    pid0 = stable_pose(st)
    count_straddlers = args[0] in ('record', 'witness')
    c4 = Comm4(st, M, inside, clean, pid0) if opt['comm4'] else None

    units, not_run = [], []
    for (pid, _), rows in sorted(groups.items(), key=lambda kv: (-info[kv[0][0]][0], int(high[kv[1][0]]))):
        dl, bl = info[pid]
        U = high[rows]
        Uset = set(U.tolist())
        caps = np.flatnonzero(ins[rows[0]]).tolist()
        Q = st._pose_matrix(pid)
        u = {'pieces': len(U), 'first': int(U[0]), 'piece_ids': U.tolist(), 'height': dl, 'branch': bl,
             'caps_whole': caps, 'clean_caps_whole': [x for x in caps if clean[x]], 'own_branch_caps': []}
        lowering = {}
        for x in caps:
            if GB[x] != bl:
                continue
            low = [j for j in range(60)
                   if T.dist(T.factors(to_tuple(matmul(R.element(M, x, j), Q)))[0]) == dl - 2]
            lowering[x] = low
            cap_off = int((~lat[inside[x]]).sum())
            row = {'grip': x, 'clean': bool(clean[x]), 'lowering_items': low, 'cap_pieces': len(inside[x]),
                   'cap_lattice': int(lat[inside[x]].sum()), 'cap_other_off': cap_off - len(U)}
            if not clean[x] and count_straddlers:
                row['straddlers'] = int(len(straddler_set(st, x)))
            u['own_branch_caps'].append(row)
        u['admissible_lowering_caps'] = [x for x in lowering if clean[x]]
        u['one_letter_strict'] = [x for x in u['admissible_lowering_caps'] if len(inside[x]) == len(U)]
        pairs = []
        for x in u['admissible_lowering_caps']:
            for y in u['clean_caps_whole']:
                if y == x:
                    continue
                Kxy = np.intersect1d(inside[x], inside[y])
                pairs.append({'x': x, 'y': y, 'K': len(Kxy), 'K_lattice': int(lat[Kxy].sum()),
                              'K_other_off': int((~lat[Kxy]).sum()) - len(U), 'K_is_unit': set(Kxy.tolist()) == Uset})
        u['lens_pairs'] = pairs
        if c4 is not None and (opt['units'] is None or u['first'] in opt['units']):
            if time.time() - T0 < opt['comm4_seconds']:
                t1 = time.time()
                u['comm4'] = c4.run(U, dl, caps)
                u['comm4']['seconds'] = round(time.time() - t1, 1)
            else:
                not_run.append(u['first'])
        units.append(u)
        log(unit=u['first'], pieces=len(U), height=dl, branch=bl, caps_whole=len(caps),
            clean_whole=len(u['clean_caps_whole']), own=[(r['grip'], r['clean'], r['cap_lattice'], r['cap_other_off'])
                                                         for r in u['own_branch_caps']],
            pairs=[(p['x'], p['y'], p['K'], p['K_is_unit']) for p in pairs],
            comm4={k: v for k, v in u.get('comm4', {}).items()
                   if k not in ('strict_words', 'strict_words_other', 'by_pair')})
    assert st.digest() == digest0, 'the state changed'
    ran = [u for u in units if 'comm4' in u]
    res = {'source': src, 'digest': digest0, 'off': int(len(off)), 'off_poses': len(info),
           'heights': {str(k): v for k, v in sorted(heights.items())}, 'clean_caps': int(clean.sum()),
           'high_units': len(units),
           'summary': {
               'without_admissible_lowering_cap': sum(1 for u in units if not u['admissible_lowering_caps']),
               'with_one_admissible_lowering_cap': sum(1 for u in units if len(u['admissible_lowering_caps']) == 1),
               'with_more': sum(1 for u in units if len(u['admissible_lowering_caps']) > 1),
               'without_clean_cap_whole': sum(1 for u in units if not u['clean_caps_whole']),
               'one_letter_strict': sum(1 for u in units if u['one_letter_strict']),
               'lens_pairs': sum(len(u['lens_pairs']) for u in units),
               'lens_pairs_K_is_unit': sum(1 for u in units for p in u['lens_pairs'] if p['K_is_unit'])},
           'comm4': None if c4 is None else {
               'units_run': len(ran), 'units_not_run': not_run,
               'units_with_a_strict_word': sum(1 for u in ran if u['comm4']['strict']),
               'units_with_a_strict_lowering_word': sum(1 for u in ran if u['comm4']['strict_lowering']),
               'words': sum(u['comm4']['words'] for u in ran),
               'strict_words': sum(u['comm4']['strict'] for u in ran)},
           'units': units, 'seconds': round(time.time() - T0, 1)}
    Path(out_path).write_text(json.dumps(res) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'units'}))


if __name__ == '__main__':
    main()
