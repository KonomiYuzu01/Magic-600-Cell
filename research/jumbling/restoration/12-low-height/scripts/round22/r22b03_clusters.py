# Theory round 22, R22B-03: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22b03_clusters.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22b03_clusters.py
import sys,json,resource,signal
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(300,310))
def stop(*a):
 print('CUT_SHORT_CPU_300_SECONDS',flush=True); raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path[:0]=[str(Path(p).resolve()) for p in ['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/09-general-method/scripts/echain','research/jumbling/restoration/09-general-method/scripts/gm']]
import screen,buffer as B,fastghost as FG,vacate2
import numpy as np
st,M,_=screen.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
assert st.digest()=='7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818'
G=B.Alphabet(SimpleNamespace(st=st),M)
W=json.loads(Path('research/jumbling/restoration/12-low-height/runs/lc13-k40/transport-b00-word.json').read_text())['word']
U=np.flatnonzero(G.home[:,sorted({c for c,i in W})].any(axis=1))
g=FG.Ghost(FG.Frames(),np.zeros(len(U),np.int64),U)
for c,i in W:g.apply(c,screen.restore.element(M,c,i),(c,i))
T=np.sort(U[~g.alive|(g.dom!=0)])
mask=np.zeros(G.NP,bool);mask[T]=True
va=vacate2.Vacate2(G,mask,G.holes)
original=va.apply
counter=0
def checked_apply(w):
 global counter
 counter+=1
 before=va.occ.copy()
 original(w)
 p=G.perm(w)
 lab,lens=B.cycles(p)
 sizes,cts=np.unique(lens[lens>1],return_counts=True)
 doc={'step':counter,'word':G.pairs(w),'gained':int((mask&before&~va.occ).sum()),'lost':int((mask&~before&va.occ).sum()),'old_vacant_moved':int((mask&~before&(p!=np.arange(G.NP))).sum()),'cycle_counts':list(zip(sizes.tolist(),cts.tolist()))}
 if len(w)==4 and w[2:]==[int(G.inv[w[0]]),int(G.inv[w[1]])]:
  a,b=G.perm(w[:1]),G.perm(w[1:2]);I=np.flatnonzero((a!=np.arange(G.NP))&(b!=np.arange(G.NP)))
  doc.update(overlap=len(I),a_overlap_returns=int(np.isin(a[I],I).sum()),b_overlap_returns=int(np.isin(b[I],I).sum()))
 print('CLUSTER',json.dumps(doc),flush=True)
va.apply=checked_apply
word,steps,ok=va.run(max_steps=2)
print('BOUNDED_PREFIX',json.dumps({'steps':steps,'length':len(word),'ok':ok,'left':int((va.occ&mask).sum())}),flush=True)
