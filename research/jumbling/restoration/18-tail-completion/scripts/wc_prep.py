"""Stage 18 (tail completion): the SAFE-ALL preparation X of a whole-cap instance, as a literal word.

For a pose table Z, an eligible cap c and a landing item i: the middle group with the inside mask empty (every hole
moved out of cap c; COV-10, COV-12, COV-32), the integer flow from the source SAFE-ALL orbits of Z (their holes) to the
middle demand orbits (the landing chambers), and from it a prepared hole set J outside the cap with the same hole count
in every source orbit, chosen to keep as many current holes as it can. X is the SAFE-ALL word that makes every chamber
of J a hole: an R22B-10 certificate for the target J at Z (r22b10_cert.py, checked by check_r22b10.py), expanded into
one literal word (expand_slp.py) and played twist by twist from Z (slp_apply.py, a word_file segment), after which the
hole set must be exactly J and every off piece must keep its exact pose. Writes into a new directory: target.json,
x-word.json (the word with its input and output digests), zx.json (the table of Z.X), free.json (no free pieces:
every off piece inside the cap lands), unit.json (the off pieces inside the cap) and prep.json (the record).
Research only, headless; exact geometry. Usage (from the repository root): OPENBLAS_NUM_THREADS=1 python3 -I -B
<this script> <pose table> <grip> <item> <new out dir> [--seeds atlas] [--literal-limit N]
"""
import argparse
import hashlib
import json
import sys
from collections import deque
from math import lcm
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
sys.path.insert(0, str(Path('research/jumbling/restoration/12-low-height/scripts/round22').resolve()))
import screen as s  # noqa: E402
import replay_slp as RS  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from sim.regions import transport_rows  # noqa: E402
from sim.kernel import sign5_vec, KAPPA_FORM  # noqa: E402

ctx = s.CL.ctx
kp, reg = ctx.kplus, ctx.regions
S = RS.S


def im(m):
    d = lcm(*(v.d for row in m for v in row))
    return (np.array([[v.a * (d // v.d) for v in row] for row in m], np.int64),
            np.array([[v.b * (d // v.d) for v in row] for row in m], np.int64), d)


def labels(mi, y):
    e, v = transport_rows(mi, reg.vform(y))
    v = np.array(v, np.int64)
    aa = 125 * (v[:, :4] @ s.CL.data.N1.T + 5 * v[:, 4:] @ s.CL.data.N2.T) - 2 * e * KAPPA_FORM[0]
    bb = 125 * (v[:, 4:] @ s.CL.data.N1.T + v[:, :4] @ s.CL.data.N2.T) - 2 * e * KAPPA_FORM[1]
    assert max(int(abs(aa).max()), int(abs(bb).max())) < 2 ** 29
    ss = sign5_vec(aa, bb)
    return np.where((ss >= 0).all(axis=0), 1, np.where((ss <= 0).all(axis=0), 0, 2))


def flow(caps_src, caps_mid, want_mid):
    na, nj = len(caps_src), len(want_mid)
    sink = na + nj + 1
    cap = np.zeros((sink + 1, sink + 1), np.int64)
    for a, h in enumerate(caps_src):
        cap[0, 1 + a] = h
        for b in range(nj):
            cap[1 + a, 1 + na + b] = caps_mid[a][b]
    for b, w in enumerate(want_mid):
        cap[1 + na + b, sink] = w
    res, value = cap.copy(), 0
    while True:
        prev = np.full(sink + 1, -1)
        prev[0] = 0
        queue = deque([0])
        while queue and prev[sink] < 0:
            a = queue.popleft()
            for b in np.flatnonzero(res[a] > 0):
                if prev[b] < 0:
                    prev[b] = a
                    queue.append(int(b))
        if prev[sink] < 0:
            break
        v, d = sink, 10 ** 9
        while v:
            a = prev[v]
            d = min(d, int(res[a, v]))
            v = a
        v = sink
        while v:
            a = prev[v]
            res[a, v] -= d
            res[v, a] += d
            v = a
        value += d
    used = cap - res
    return value, [[int(used[1 + a, 1 + na + b]) for b in range(nj)] for a in range(na)]


def main(argv=None):
    ap = argparse.ArgumentParser(description='The SAFE-ALL preparation of a whole-cap instance.')
    ap.add_argument('table', type=Path)
    ap.add_argument('grip', type=int)
    ap.add_argument('item', type=int)
    ap.add_argument('out', type=Path)
    RS.literal_options(ap)
    args = ap.parse_args(argv)
    out, c, i = args.out, args.grip, args.item
    out.mkdir(parents=True, exist_ok=False)
    st, M, _ = s.load('table', str(args.table))
    z = st.digest()
    off = s.B.off_pieces(st)
    mi = {int(pid): im(st._pose_matrix(int(pid))) for pid in np.unique(st.pose_id[off])}
    ls = np.array([labels(mi[int(st.pose_id[p])], int(p)) for p in off])
    assert not (ls[:, c] == 2).any(), 'the cap is straddled'
    u = off[ls[:, c] == 1]
    g = s.restore.element(M, c, i)
    assert all(kp.index_of_matrix(to_tuple(matmul(g, st.pose(int(p))))) >= 0 for p in u), 'the item does not land the cap'
    s.B.log = lambda **kw: None
    home = s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)))
    typ = np.asarray(reg.orbit_of)
    NP = len(home)
    allcaps = [np.flatnonzero(home[:, x]) for x in range(600)]
    two = {}
    for x in range(600):
        rows = []
        for k in kp.gen_idx[2 * x:2 * x + 2]:
            xs = allcaps[x]
            ys = reg.piece_of[typ[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
            j = next(j for j in range(1, 12) if kp.index_of_matrix(s.restore.element(M, x, j)) == k)
            rows.append((x, j, int(k), ys))
        two[x] = rows

    def group(safe):
        gg = s.B.Alphabet.__new__(s.B.Alphabet)
        gg.NP, gg.home, gg.type = NP, home, typ
        gg.safe = list(map(int, safe))
        gg.letters = [l for x in gg.safe for l in two[x]]
        gg.capx = {x: allcaps[x] for x in gg.safe}
        gg.index = {(l[0], l[1]): j for j, l in enumerate(gg.letters)}
        gg.orbits()
        return gg

    G0 = group(np.flatnonzero((ls == 0).all(axis=0)))
    o0, z0 = G0.orbits()
    H = s.B.hole_set(st)
    hc = np.bincount(o0[H], minlength=len(z0))
    otherbad = (ls[ls[:, c] != 1] != 0).any(axis=0)
    capx = allcaps[c]
    mg = im(g)
    bad = otherbad.copy()
    for q in capx:
        bad |= labels(mg, int(q)) != 0
    G1 = group(np.flatnonzero(~bad))
    o1, z1 = G1.orbits()
    dst = []
    for p in u:
        k = kp.index_of_matrix(to_tuple(matmul(g, st.pose(int(p)))))
        dst.append(int(reg.piece_of[typ[p], kp.compose(k, int(reg.transport[p]))]))
    dst = np.array(dst)
    assert not (z1[o1[dst]] == 1).any(), 'a landing chamber is fixed at the middle'
    ais = np.flatnonzero(hc)
    js = np.unique(o1[dst])
    want = np.bincount(o1[dst], minlength=len(z1))
    outs = [np.flatnonzero((o0 == a) & (~home[:, c])) for a in ais]
    assert all(len(xs) >= hc[a] for a, xs in zip(ais, outs)), 'a source orbit has too few chambers outside the cap'
    value, used = flow([int(hc[a]) for a in ais], [[int((o1[xs] == j).sum()) for j in js] for xs in outs],
                       [int(want[j]) for j in js])
    assert value == len(dst), ('flow', value, len(dst))
    isH = np.zeros(NP, bool)
    isH[H] = True
    Jset = []
    for a, (ai, xs) in enumerate(zip(ais, outs)):
        chosen = []
        for b, j in enumerate(js):
            pool = xs[o1[xs] == j]
            pool = np.concatenate([pool[isH[pool]], pool[~isH[pool]]])  # current holes first
            chosen.extend(pool[:used[a][b]].tolist())
        rest = np.setdiff1d(xs, np.array(chosen, dtype=int))
        rest = np.concatenate([rest[isH[rest]], rest[~isH[rest]]])
        chosen.extend(rest[:int(hc[ai]) - len(chosen)].tolist())
        Jset += chosen
    J = np.array(sorted(Jset), np.int64)
    assert len(J) == len(H) and not home[J, c].any()
    assert (np.bincount(o0[J], minlength=len(z0)) == hc).all()
    assert (np.bincount(o1[J], minlength=len(z1)) >= want).all()
    T = J.tolist()
    tgt = {'T': T, 'T_digest': hashlib.sha256(json.dumps(T).encode()).hexdigest(), 'state_digest': z}
    (out / 'target.json').write_text(json.dumps(tgt) + '\n')
    rec = {'state': z, 'grip': c, 'item': i, 'off': int(len(off)), 'cap_off': int(len(u)), 'holes': int(len(H)),
           'holes_in_cap': int(home[H, c].sum()), 'J_kept_holes': int(isH[J].sum()), 'flow': int(value),
           'source_orbits': int(len(ais)), 'middle_orbits': int(len(js)), 'target_digest': tgt['T_digest']}
    unit = {'pieces': sorted(int(p) for p in u)}
    (out / 'unit.json').write_text(json.dumps(unit) + '\n')
    (out / 'free.json').write_text(json.dumps({'pieces': []}) + '\n')
    if isH[J].all():
        rec.update(X=0)
        (out / 'x-word.json').write_text(json.dumps({'word': [], 'input_digest': z, 'output_digest': z}) + '\n')
        s.B.export_pose_table(st, out / 'zx.json')
        (out / 'prep.json').write_text(json.dumps(rec) + '\n')
        print(json.dumps(rec), flush=True)
        return
    cert = out / 'x-cert.json.gz'
    seed_args = ['--seeds', 'atlas'] if args.seeds == 'atlas' else []
    RS.run([S / 'r22b10_cert.py', 'table', args.table, out / 'target.json', cert, '--seconds', '3600'] + seed_args,
           out / 'x-build.log')
    RS.run([S / 'check_r22b10.py', 'table', args.table, out / 'target.json', cert], out / 'x-check.txt')
    lines = (out / 'x-check.txt').read_text().split()
    assert lines and lines[-1] == 'PASS', 'the preparation certificate is incomplete or fails its check'
    word = out / 'x-word-literal.json.gz'
    rc = RS.run([S / 'expand_slp.py', cert, word, '--limit', args.literal_limit or 10 ** 7], out / 'x-expand.log')
    assert rc == 0, 'the expansion failed or is over the limit'
    xw = RS.read_json(word)['word']
    xf = out / 'x-play.json'
    xf.write_text(json.dumps({'word': xw, 'input_digest': z}) + '\n')
    prog = out / 'x-program.json'
    prog.write_text(json.dumps({'segments': [{'word_file': str(xf)}]}) + '\n')
    zx = out / 'zx.json'
    rc = RS.run([S / 'slp_apply.py', 'table', args.table, prog, zx, '--unit', out / 'unit.json'], out / 'x-apply.log')
    rows = RS.json_lines(out / 'x-apply.log')
    summary = rows[-1] if rows else {}
    assert rc == 0 and summary.get('off_after') == len(off) and summary.get('collateral') == 0, summary
    assert summary.get('others_exact') == summary.get('others') and summary.get('unit_on_lattice') == 0, summary
    st2, _, _ = s.load('table', str(zx))
    H2 = np.sort(s.B.hole_set(st2))
    assert np.array_equal(H2, J), 'the holes after X are not J'
    assert s.B.off_fingerprint(st2) == s.B.off_fingerprint(st), 'X moved an off piece'
    zxd = st2.digest()
    (out / 'x-word.json').write_text(json.dumps({'word': xw, 'input_digest': z, 'output_digest': zxd}) + '\n')
    rec.update(X=len(xw), zx=zxd, X_sha256=hashlib.sha256(json.dumps(xw).encode()).hexdigest())
    (out / 'prep.json').write_text(json.dumps(rec) + '\n')
    print(json.dumps(rec), flush=True)


if __name__ == '__main__':
    main()
