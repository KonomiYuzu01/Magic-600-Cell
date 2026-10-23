# Theory round 22, R22B-09: the senior reviewer's exact computation, kept as written apart from this header; the
# integrator re-ran it and got the output in runs/round22/r22b09_bound.txt. It reruns R22B-04 (the 19 Jordan
# certificates of b00's target orbits), finds for each orbit the SAFE letter with the smallest support on it, and
# evaluates the construction bound of R22B-05 and R22B-08. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B
#   research/jumbling/restoration/12-low-height/scripts/round22/r22b09_bound.py   (about 1 minute)
import contextlib,io,runpy,random,math,json
buf=io.StringIO()
with contextlib.redirect_stdout(buf),contextlib.redirect_stderr(buf):
    r=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22b04_jordan.py')
G,B,found=r['G'],r['B'],r['found']
import numpy as np
assert len(found)==19 and r['cond']['necessary']
print('STATE',G.state_digest,'TARGET',len(r['T']),'ALL_OCCUPIED',bool(G.occupied[r['T']].all()),flush=True)
print('ALPHABET',len(G.safe),len(G.letters),'ORBITS',len(G.orbit_size),'SINGLETONS',int((G.orbit_size==1).sum()),flush=True)
minimum={o:(G.NP,None) for o in found}
for j,(c,item,k,ys) in enumerate(G.letters):
    xs=G.capx[c]; counts=np.bincount(G.orbit[xs[xs!=ys]],minlength=len(G.orbit_size))
    for o in found:
        q=int(counts[o])
        if 0<q<minimum[o][0]:minimum[o]=(q,j)
rng=random.Random(1); words=[[rng.randrange(len(G.letters)) for _ in range(120)] for _ in range(80)]
rows=[]; bound=0
for o,(stage,trial,cert) in sorted(found.items()):
    assert stage==0
    w=words[trial-1]; _,lens=B.cycles(G.perm(w)); p=cert['prime']
    m=math.lcm(*(int(x) for x in lens[lens>0] if x!=p)); assert m%p
    d=cert['orbit_size']; k=int((G.orbit[r['T']]==o).sum()); q,j=minimum[o]
    assert j is not None and 2*q-1<=d
    L=120*m+2*(d-p)
    s=4+4*L*(math.comb(d,q)-1)
    e=(d-2)*(2*d-1); f=(d-3)*(2*d-3)*(2*d-1)
    bound+=2*k*(s+2*e+2*f)
    rows.append([int(G.orbit_least[o]),d,k,p,q,G.pairs([j])[0]])
print('ROWS_least_d_k_prime_support_letter',json.dumps(rows),flush=True)
print('ALL_19_JORDAN_AND_SMALL_SUPPORT_CHECKS_PASS',flush=True)
print('BOUND',bound,flush=True)
print('BOUND_LE_10_POWER',len(str(bound)),flush=True)
print('NO_COMPLETE_B00_SEED_TABLE_OR_V_MATERIALIZED',flush=True)
