"""Type-S restoration of B' by bubble surgery: U = P (x r y x^-1) P^-1 (reads only the pose table).

After P: B' on K in chambers L, target T = Bad(P^-1) = L + BH, good holes GH outside T. The only SAFE-ALL caps holding
L also hold bad holes, so L sits in a bubble of holes. x is a SAFE-ALL letter that carries the bubble's chambers in its
cap (lens = cap(x) ∩ T) out of T: bubble = x(lens), B' at x(L). With H0 = SAFE-ALL grips avoiding T (they fix T
pointwise) the script looks for:
- y: an H0 letter whose cap holds x(L), maps the bubble holes in its cap, B_y, into B_y + x(L), and moves x(L) out of
  the bubble. Then N = y^-1(B_y + x(L)) minus B_y are 3 chambers outside the bubble that must hold holes before y.
- r: a word of H1 = H0 grips whose caps avoid the bubble holes x(BH ∩ lens), routing GH onto N (peeling certificate and
  greedy of Theorem B on H1, target N).
Then x r y x^-1 ends with holes exactly T, and P^-1 returns the parked pieces. Every premise is recorded before any
twist after P; then the word is replayed on the simulator (every twist checked admissible) and the end compared with
the start.

Usage: python -I bubble.py <theory dir> <pose table> <P json word> <out dir> [--max-y N]
"""
import copy
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def restrict(G, grips, kp):
    gs = set(grips)
    G0 = copy.copy(G)
    for attr in ('orbit', 'orbit_size', 'orbit_least'):
        G0.__dict__.pop(attr, None)
    G0.safe = [c for c in G.safe if c in gs]
    G0.capx = {c: G.capx[c] for c in G0.safe}
    G0.letters = [l for l in G.letters if l[0] in gs]
    G0.by_grip = {}
    for n, l in enumerate(G0.letters):
        G0.by_grip.setdefault(l[0], []).append(n)
    G0.index = {(l[0], l[1]): n for n, l in enumerate(G0.letters)}
    kindex = {(l[0], l[2]): n for n, l in enumerate(G0.letters)}
    G0.inv = np.array([kindex[(l[0], int(kp.inv[l[2]]))] for l in G0.letters], np.int64)
    G0.orbits()
    return G0


def main():
    a = sys.argv[1:]
    out = Path(a[3])
    out.mkdir(parents=True, exist_ok=True)
    C = GL.Ctx(a[1])
    R, M, st = C.R, C.M, C.st
    from blocks import CL
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    codes = CC.Codes()
    c0 = codes.of(st)
    d0 = st.digest()
    off0 = np.flatnonzero(c0 < 0)
    lat0 = c0 >= 0
    P = [tuple(x) for x in json.loads(a[2])]
    Pinv = CP.inverse_word(M, P)
    badPinv, _, _ = C.bad(Pinv)
    t = time.time()
    for c, i in P:
        R.apply(c, i)
    d_p = st.digest()
    G = B.Alphabet(R, M)
    G.orbits()
    holes = np.asarray(G.holes)
    T = np.zeros(G.NP, bool)
    T[badPinv] = True
    Kp = st._kpose()[st.pose_id]
    L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
    GH = holes[~T[holes]]
    isL = np.zeros(G.NP, bool)
    isL[L] = True
    h0 = [c for c in G.safe if not T[G.capx[c]].any()]
    pre = {'state_digest': d0, 'P': [list(x) for x in P], 'after_P_digest': d_p, 'T': int(T.sum()),
           'holes': int(len(holes)), 'GH': GH.tolist(), 'L': L.tolist(), 'h0_grips': len(h0),
           'setup_sec': round(time.time() - t, 1), 'x_rows': []}
    log(**{k: v for k, v in pre.items() if k != 'x_rows'})
    lgrips = [c for c in G.safe if isL[G.capx[c]].any()]
    found = None
    for c in lgrips:
        for x in G.by_grip[c]:
            t = time.time()
            px = G.perm([x])
            lens = G.capx[c][T[G.capx[c]]]
            bub = px[lens]
            if T[bub].any() or T[px[GH]].any():
                pre['x_rows'].append({'x': G.pairs([x])[0], 'skip': 'bubble or GH meets T after x'})
                continue
            inbub = np.zeros(G.NP, bool)
            inbub[bub] = True
            xL = px[L]
            bh = bub[~np.isin(bub, xL)]
            isbh = np.zeros(G.NP, bool)
            isbh[bh] = True
            ys = [n for cc in h0 if G.home[xL, cc].all() for n in G.by_grip[cc]]
            row = {'x': G.pairs([x])[0], 'bubble': int(len(bub)), 'y_letters_holding_xL': len(ys), 'y_ok': []}
            for y in ys:
                cy = G.letters[y][0]
                cap = G.capx[cy]
                img = G.letters[y][3]
                By = cap[isbh[cap]]
                imgBy = img[np.searchsorted(cap, By)]
                if not inbub[imgBy].all():
                    continue
                imgL = img[np.searchsorted(cap, np.sort(xL))]
                if inbub[imgL].any():
                    continue
                want = np.concatenate([By, xL])
                pre_img = cap[np.isin(img, want)]
                N = pre_img[~isbh[pre_img]]
                if len(N) != 3 or inbub[N].any() or T[N].any():
                    continue
                row['y_ok'].append({'y': G.pairs([y])[0], 'B_y': int(len(By)), 'N': np.sort(N).tolist()})
            if row['y_ok']:
                h1 = [cc for cc in h0 if not isbh[G.capx[cc]].any()]
                G1 = restrict(G, h1, kp)
                row['h1_grips'] = len(h1)
                row['GH_h1_orbits'] = sorted(int(G1.orbit[g]) for g in px[GH])
                hmask = np.zeros(G.NP, bool)
                hmask[px[holes]] = True
                for yr in row['y_ok'][:int(a[a.index('--max-y') + 1]) if '--max-y' in a else 20]:
                    N = np.asarray(yr['N'], np.int64)
                    yr['N_h1_orbits'] = sorted(int(G1.orbit[z]) for z in N)
                    yr['N_h1_orbit_sizes'] = [int(G1.orbit_size[G1.orbit[z]]) for z in N]
                    cond = B.condition(G1, N, None, np.flatnonzero(hmask))
                    yr['N1'], yr['N2'] = cond['fixed_vacant'], cond['necessary']
                    if not cond['necessary'] or found is not None:
                        continue
                    try:
                        cert = CP.generic_certificate(G1, N)
                        prep, _, deficits = CP.generic_prepare(G1, cert, N, np.flatnonzero(hmask))
                        yr['certificate'] = {'peeled': cert['peeled'], 'of': cert['of']}
                        yr['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
                        if prep is None:
                            continue
                        r = [tuple(z) for z in prep['word']]
                        Qp = ([tuple(G.pairs([x])[0])] + r + [tuple(yr['y'])]
                              + [tuple(G.pairs([int(G.inv[x])])[0])])
                        pq = G.permutation([list(z) for z in Qp])
                        yr['r_length'] = len(r)
                        yr['predicted_holes_equal_T'] = bool(np.array_equal(np.sort(pq[holes]), np.sort(badPinv)))
                        if yr['predicted_holes_equal_T']:
                            found = (row['x'], yr['y'], Qp, cert)
                    except Exception as exc:  # noqa: BLE001
                        yr['construct_error'] = str(exc)[:300]
            row['sec'] = round(time.time() - t, 1)
            pre['x_rows'].append(row)
            log(x=row['x'], bubble=row['bubble'], ys=row['y_letters_holding_xL'], y_ok=len(row['y_ok']),
                first=row['y_ok'][:3], sec=row['sec'])
            if found is not None and '--all' not in a:
                break
        if found is not None and '--all' not in a:
            break
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    for _ in P:
        R.undo()
    if st.digest() != d0:
        raise AssertionError('state not restored after undo')
    if found is None:
        (out / 'result.json').write_text(json.dumps({'result': 'no word'}, indent=1) + '\n')
        log(result='no word')
        return
    xw, yw, Qp, cert = found
    (out / 'certificate-r.json').write_text(json.dumps(cert) + '\n')
    t = time.time()
    parts = [('P', P), ('Qprime', Qp), ('Pinv', Pinv)]
    rows = []
    (out / 'parts').mkdir(exist_ok=True)
    for name, part in parts:
        d_in = st.digest()
        for c, i in part:
            R.apply(c, i)
        cn = codes.of(st)
        rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': st.digest(),
                     'off_end': int((cn < 0).sum()), 'lattice_of_start_off': int((lat0 & (cn < 0)).sum())})
        (out / 'parts' / f'{name}.json').write_text(json.dumps({'word': [list(z) for z in part], 'input_digest': d_in,
                                                                'output_digest': rows[-1]['output_digest']}) + '\n')
        if name == 'Qprime':
            rows[-1]['holes_equal_T'] = bool(np.array_equal(np.sort(B.hole_set(st)), np.sort(badPinv)))
        log(part=name, twists=len(part), off=rows[-1]['off_end'])
    ce = codes.of(st)
    end = {'digest': st.digest(), 'off': int((ce < 0).sum()), 'bprime_restored': int((ce[off0] >= 0).sum()),
           'collateral': int((lat0 & (ce < 0)).sum()), 'lattice': int(lat0.sum()),
           'lattice_on_K_at_end': int((lat0 & (ce >= 0)).sum()),
           'lattice_same_exact_pose': int((lat0 & (ce == c0)).sum())}
    full = [list(z) for _, part in parts for z in part]
    (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d0,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'input_digest': d0, 'x': xw, 'y': yw, 'parts': rows, 'end': end, 'length': len(full),
           'replay_sec': round(time.time() - t, 1), 'seconds': round(time.time() - T0, 1)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    log(end=end, length=len(full))


if __name__ == '__main__':
    main()
