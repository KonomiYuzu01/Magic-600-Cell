# Theory round 22, R22CF-02 (line (c)): the Shadow lemma's counts at the depth 40 stop.
# The solver's computation, kept as written apart from this header and the directory of its unit files (next to the
# output instead of a fixed path); the integrator re-ran it and got the output in runs/round22/line-c/shadow/.
# Research only, headless Linux; exact geometry. Run from the repository root:
# python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22cf02_shadow.py \
#     research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json \
#     research/jumbling/restoration/12-low-height/runs/screen-k40.json <out json>
# R22CF computation A2: shadowed chambers and the r = 1 hole test at the depth 40 stop. For each unit U with home
# chambers: x in Omega_C is U-shadowed when every cap containing x holds a piece of U's block or is straddled by it
# (block status 1 or 2 at every cap of x). Such an x is fixed by the SAFE-ALL group at every state where U's block
# keeps its exact pose. For a retained letter E at an admissible cap g holding U's block, the target at Y is E(Omega_C);
# E(x) is A(U)-shadowed at Y iff x is U-shadowed at Z, and E(x) is occupied at Y iff x is occupied (x in cap g) or
# E(x) is occupied at Z (x outside cap g). The count of shadowed occupied chambers at Y is therefore read from Z.
# Also: the 600 type-0 chambers (one per grip, on its axis) lie in one cap only and are fixed by its retained letters.
# Writes the output file given and, next to it, unit files unit-b<k>.json. Read-only otherwise.
import sys, json, time
from pathlib import Path
from types import SimpleNamespace
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures'), str(REPO / 'research/jumbling/restoration/12-low-height/scripts')]
import restore, sim, rim, buffer as B, blocks as BL, classes as CL
from exact import identity
from sim.kplus import to_tuple
from screen import load
T0 = time.time()
def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)
rec, scr_path, outp = sys.argv[1], sys.argv[2], Path(sys.argv[3])
st, M, src = load('record', rec)
NP = len(st.pose_id)
S = json.loads(Path(scr_path).read_text()); assert S['digest'] == st.digest()
U = {u['unit']: u for u in S['units']}
R = SimpleNamespace(st=st)
bl = BL.Blocks(R, M)
off, labels, status = bl.off, bl.labels, bl.status
D = rim.Domains(st); dom = np.asarray(D.dom)
members, cls = CL.classes()
home = CL.data.sig_matrix(np.arange(NP))
holes = B.hole_set(st); is_hole = np.zeros(NP, bool); is_hole[holes] = True
otype = np.asarray(CL.ctx.regions.orbit_of)
reg, kp = CL.ctx.regions, CL.ctx.kplus
kid = kp.index_of_matrix(to_tuple(identity(4)))
perms = np.asarray(kp.perms, np.int64)
ret = {}
for c in range(600):
    ks = [int(kp.index_of_matrix(restore.element(M, c, i))) for i in range(len(M.items))]
    ret[c] = [k for k in ks if k >= 0 and k != kid]
    assert len(ret[c]) == 11
def chamber_image(k, xs):
    xs = np.asarray(xs, np.int64)
    return reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
# type-0 chambers: in one cap only, fixed by that cap's retained letters
t0 = np.flatnonzero(otype == 0)
ncaps0 = home[t0].sum(axis=1)
own_cap = np.argmax(home[t0], axis=1)
fixed0 = all(int(chamber_image(k, [x])[0]) == int(x) for x, c in zip(t0.tolist(), own_cap.tolist()) for k in ret[int(c)])
axis = {'type0_chambers': int(len(t0)), 'all_in_exactly_one_cap': bool((ncaps0 == 1).all()),
        'one_per_grip': bool(len(set(own_cap.tolist())) == 600), 'fixed_by_all_retained_letters_of_its_cap': bool(fixed0)}
log(axis=axis)
# units, straddled grips (admissibility), G at Z
unit_rows, unit_pieces = {}, {}
for b, pieces in enumerate(bl.blocks):
    pieces = np.asarray(pieces)
    for d in np.unique(dom[pieces]).tolist():
        k = len(unit_rows)
        unit_pieces[k] = pieces[dom[pieces] == d]
        unit_rows[k] = (b, np.searchsorted(off, unit_pieces[k]))
assert all(len(unit_pieces[k]) == U[k]['size'] for k in U)
for k in (3, 4, 5, 6, 12, 15, 18):
    (outp.parent / f'unit-b{k:02d}.json').write_text(json.dumps({'state': 'k40', 'digest': st.digest(), 'unit': k, 'block': unit_rows[k][0], 'size': int(len(unit_pieces[k])), 'pieces': [int(p) for p in unit_pieces[k]]}))
straddled = set(bl.straddling_grips.tolist())
G = B.Alphabet(R, M); G.orbits()
out = {'digest': st.digest(), 'axis_chambers': axis, 'units': {}}
for k, u in U.items():
    if not u.get('home_chambers'):
        continue
    b = unit_rows[k][0]
    P = u['class']; Pc = np.asarray(members[P], np.int64); want = np.isin(Pc, u['cell'])
    omega = np.flatnonzero((home[:, Pc] == want[None, :]).all(axis=1))
    cond = B.condition(G, omega)
    fixed_occ = set(int(x) for x in cond['fixed_occupied'])
    st_b = status[b]                                  # 0 outside, 1 inside, 2 straddle, 3 mixed
    shadow = np.array([bool((st_b[np.flatnonzero(home[x])] != 0).all()) for x in omega.tolist()])
    sh_occ = omega[shadow & ~is_hole[omega]]
    locked = set(bl.locked[b])
    info = {'kind': u['kind'], 'size': u['size'], 'class': P, 'cell': u['cell'], 'home_chambers': int(len(omega)),
            'home_occupied': int((~is_hole[omega]).sum()), 'fixed_occupied': len(fixed_occ),
            'shadowed': int(shadow.sum()), 'shadowed_occupied': int(len(sh_occ)),
            'fixed_occupied_all_shadowed': bool(fixed_occ <= set(sh_occ.tolist())),
            'shadowed_occupied_all_fixed': bool(set(sh_occ.tolist()) <= fixed_occ),
            'locked_own_block': int(sum(1 for x in omega.tolist() if x in locked)),
            'locked_own_block_occupied': int(sum(1 for x in sh_occ.tolist() if x in locked)),
            'I_block': [int(c) for c in bl.I[b]], 'S_block': [int(c) for c in bl.S[b]],
            'type0_chamber_in_cell': [int(x) for x in omega.tolist() if otype[x] == 0], 'r1': []}
    # r = 1: admissible caps holding U's block wholly
    for g in [int(c) for c in bl.I[b] if int(c) not in straddled]:
        ing = home[sh_occ, g]
        best = None
        for e in ret[g]:
            img = chamber_image(e, sh_occ)
            still = int(sum(1 for x, i, y in zip(sh_occ.tolist(), ing.tolist(), img.tolist()) if i or not is_hole[y]))
            info['r1'].append({'grip': g, 'class': int(cls[g]), 'element': int(e), 'moved_cell_pole_images': [int(perms[e][c]) for c in u['cell']],
                               'shadowed_occupied_at_Y': still, 'shadowed_in_cap': int(ing.sum()), 'shadowed_outside_cap': int((~ing).sum())})
    info['r1_min_shadowed_occupied_at_Y'] = min((r['shadowed_occupied_at_Y'] for r in info['r1']), default=None)
    info['r1_candidate_caps'] = sorted({r['grip'] for r in info['r1']})
    out['units'][str(k)] = info
    log(unit=k, kind=u['kind'], fixed_occ=len(fixed_occ), shadowed=int(shadow.sum()), shadowed_occ=int(len(sh_occ)),
        fixed_all_shadowed=info['fixed_occupied_all_shadowed'], locked_own_occ=info['locked_own_block_occupied'],
        r1_caps=info['r1_candidate_caps'], r1_min=info['r1_min_shadowed_occupied_at_Y'], type0=info['type0_chamber_in_cell'])
outp.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps({'axis': axis, 'summary': {k: {kk: v[kk] for kk in ('kind', 'fixed_occupied', 'shadowed_occupied', 'fixed_occupied_all_shadowed', 'locked_own_block_occupied', 'r1_candidate_caps', 'r1_min_shadowed_occupied_at_Y')} for k, v in out['units'].items()}}))
