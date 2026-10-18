# Theory round 22, coverage round 3 (COV-19): the 60 I_a matrices at grip 0 form a perfect group (all 60 elements are
# commutators), faithful on the class-0 poles, and every grip's menu is its conjugate by a frame in K+. So every
# homomorphism from the group the menu generates to an abelian group is trivial, and no weighted sum of such a
# homomorphism over the pieces can tell COV-01's state from a reachable one. It rebuilds that state first.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov19_perfect_menu.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov19_perfect_menu.py
import runpy,time
ns=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov01_isolated_triangle.py')
s,M,np=map(ns.get,('s','M','np'))
from exact import matvec
poles=s.CL.classes()[0][0]
from sim.kplus import det4
assert all(c in poles for c in [0,2,9,20])
assert not det4([s.CL.N[c] for c in [0,2,9,20]]).is_zero()
mats=[s.restore.element(M,0,i) for i in range(len(M.items))]
perms=[tuple(s.CL.pole(matvec(g,s.CL.N[c])) for c in poles) for g in mats]
assert len(set(perms))==60 and all(set(p)==set(poles) for p in perms)
pos={p:i for i,p in enumerate(poles)}
perms=[tuple(pos[x] for x in p) for p in perms];lookup={p:i for i,p in enumerate(perms)}
table=[[lookup[tuple(a[x] for x in b)] for b in perms] for a in perms]
e=lookup[tuple(range(120))];inv=[next(j for j in range(60) if table[i][j]==e) for i in range(60)]
comms={table[table[a][b]][table[inv[a]][inv[b]]] for a in range(60) for b in range(60)}
closure={e};queue=[e]
for a in queue:
 for b in comms:
  q=table[a][b]
  if q not in closure:closure.add(q);queue.append(q)
assert len(closure)==60
assert all(s.CL.ctx.kplus.index_of_matrix(M.frame(c))>=0 for c in range(600))
print('PERFECT_MENU',{'order':60,'distinct_commutators':len(comms),'derived_order':len(closure),'conjugating_frames_in_Kplus':600},flush=True)
print('VERSIONS',__import__('sys').version.split()[0],np.__version__)
print('COMPLETED_CPU',round(time.process_time(),1))
