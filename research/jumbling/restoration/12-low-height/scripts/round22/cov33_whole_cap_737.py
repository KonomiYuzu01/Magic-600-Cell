# Theory round 22, coverage round 5 (COV-33, at the 737 state of the line (a) replay): the all-content-landing
# whole-cap test with SAFE preparation. The setup sorts the 600 caps (empty, straddled, mixed, no landing item,
# eligible: ELIGIBILITY_737). Part A tests every landing item of each eligible cap at its actual middle (PASS by a
# global Jordan certificate) and, failing that, excludes every preparation by an optimistic middle group built from the
# mandatory displaced regions (NO_PREP) or leaves the cap open (OPEN_PREP) (CAP_RESULT rows, SUMMARY_737). Part B
# resolves the one open cap, 91 with item 14: an exact integer flow from source SAFE orbits to middle demand orbits
# chooses a prepared hole set, and Jordan certificates on both sides prove it (FLOW, FLOW_JORDAN,
# PREPARED_CAP_91_PASS).
# The senior reviewer's exact computation, kept as written apart from this header: the common setup, then part A, then
# part B (the reviewer ran each part after its own copy of the setup; here they run in one process, so part B's CPU
# figure is cumulative). The integrator re-ran it and got the output in runs/round22/cov33_whole_cap_737.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov33_whole_cap_737.py
import sys,time,json,resource,signal
from pathlib import Path
from collections import Counter
from math import lcm
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
from sim.regions import transport_rows
from sim.kernel import sign5_vec,KAPPA_FORM
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
def im(m):
 d=lcm(*(v.d for row in m for v in row))
 return np.array([[v.a*(d//v.d) for v in row] for row in m],np.int64),np.array([[v.b*(d//v.d) for v in row] for row in m],np.int64),d
def labels(mi,y):
 e,v=transport_rows(mi,reg.vform(y));v=np.array(v,np.int64);aa=125*(v[:,:4]@s.CL.data.N1.T+5*v[:,4:]@s.CL.data.N2.T)-2*e*KAPPA_FORM[0];bb=125*(v[:,4:]@s.CL.data.N1.T+v[:,:4]@s.CL.data.N2.T)-2*e*KAPPA_FORM[1]
 assert max(int(abs(aa).max()),int(abs(bb).max()))<2**29
 ss=sign5_vec(aa,bb);return np.where((ss>=0).all(axis=0),1,np.where((ss<=0).all(axis=0),0,2))
st,M,_=s.load('table','research/jumbling/restoration/12-low-height/runs/round22/line-a/replay/tables/737.json.gz')
off=s.B.off_pieces(st);D=s.rim.Domains(st);mi={int(pid):im(st._pose_matrix(int(pid))) for pid in np.unique(st.pose_id[off])};ls=np.array([labels(mi[int(st.pose_id[p])],int(p)) for p in off])
ct=Counter();eligible=[]
for c in range(600):
 if (ls[:,c]==2).any():ct['straddled']+=1;continue
 u=off[ls[:,c]==1]
 if not len(u):ct['empty']+=1;continue
 if len(set(D.dom[u]))!=1:ct['mixed']+=1;continue
 pose=st.pose(int(u[0]));items=[i for i in range(12,60) if kp.index_of_matrix(to_tuple(matmul(s.restore.element(M,c,i),pose)))>=0]
 if items:ct['eligible']+=1;eligible.append([c,len(u),items])
 else:ct['no_menu']+=1
print('ELIGIBILITY_737',st.digest(),dict(ct),eligible,'CPU',round(time.process_time(),1),flush=True)
from types import SimpleNamespace
s.B.log=lambda **kw:None
home=s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)));typ=np.asarray(reg.orbit_of);NP=len(home)
allcaps=[np.flatnonzero(home[:,c]) for c in range(600)];two={}
for c in range(600):
 rows=[]
 for k in kp.gen_idx[2*c:2*c+2]:
  xs=allcaps[c];ys=reg.piece_of[typ[xs],kp.compose_vec(np.full(len(xs),k),reg.transport[xs])]
  i=next(i for i in range(1,12) if kp.index_of_matrix(s.restore.element(M,c,i))==k)
  rows.append((c,i,int(k),ys))
 two[c]=rows
def group(safe):
 g=s.B.Alphabet.__new__(s.B.Alphabet);g.NP=NP;g.home=home;g.type=typ;g.safe=list(map(int,safe));g.letters=[l for c in g.safe for l in two[c]];g.capx={c:allcaps[c] for c in g.safe};g.index={(l[0],l[1]):j for j,l in enumerate(g.letters)};g.orbits();return g
G0=group(np.flatnonzero((ls==0).all(axis=0)));H=s.B.hole_set(st);o0,z0=G0.orbits();hc=np.bincount(o0[H],minlength=len(z0));possible=hc[o0]>0
# Part A: eligibility, actual middles, and preparation-independent exclusions.
totals=Counter();records=[]
for c,nu,items in eligible:
 cap=allcaps[c];u=off[ls[:,c]==1];otherls=ls[ls[:,c]!=1];otherbad=(otherls!=0).any(axis=0)
 qs=cap[~np.isin(cap,H)];mandatory=~possible[qs];rows=[];cap_pass=False;notexcluded=False
 for i in items:
  g=s.restore.element(M,c,i);mi=im(g);qbad=np.zeros(600,bool);qmust=np.zeros(600,bool)
  for q,must in zip(qs,mandatory):
   bad=labels(mi,int(q))!=0;qbad|=bad
   if must:qmust|=bad
  dst=[]
  for p in u:
   k=kp.index_of_matrix(to_tuple(matmul(g,st.pose(int(p)))));assert k>=0
   dst.append(int(reg.piece_of[typ[p],kp.compose(k,int(reg.transport[p]))]))
  dst=np.array(dst);G1=group(np.flatnonzero(~(otherbad|qbad)));o1,z1=G1.orbits();have=np.bincount(o1[H[~home[H,c]]],minlength=len(z1));need=np.bincount(o1[dst],minlength=len(z1));deficit=int(np.maximum(need-have,0).sum());fixed=int((z1[o1[dst]]==1).sum())
  proved=False
  if deficit==0:
   cert=s.B.jordan(G1,dst,seed=29)
   if not cert['missing'] and not cert['uncertifiable_below_8']:
    for o,v in cert['certificates'].items():assert s.B.check_jordan(G1,v['word'],int(o),v['prime'])['verified']
    proved=True
  if proved:
   rows.append([i,fixed,deficit,'PASS',len(cert['certificates'])]);cap_pass=True;break
  GM=group(np.flatnonzero(~(otherbad|qmust)));om,zm=GM.orbits();mustfixed=int((zm[om[dst]]==1).sum());upperfail=0
  for oid in set(om[dst]):
   delta=np.flatnonzero((om==oid)&(~home[:,c]));counts=np.bincount(o0[delta],minlength=len(z0));upper=int(np.minimum(hc,counts).sum());req=int((om[dst]==oid).sum())
   if upper<req:upperfail+=req-upper
  verdict='NO_PREP' if mustfixed or upperfail else 'OPEN_PREP';notexcluded|=verdict=='OPEN_PREP';rows.append([i,fixed,deficit,verdict,mustfixed,upperfail])
 status='PASS' if cap_pass else ('OPEN_PREP' if notexcluded else 'NO_PREP')
 totals[status]+=1;records.append([c,status,rows]);print('CAP_RESULT',c,status,rows,'CPU',round(time.process_time(),1),flush=True)
print('SUMMARY_737',dict(totals),'CPU',round(time.process_time(),1),flush=True)
# Part B: resolving cap 91.
from collections import deque
c,i=91,14;cap=allcaps[c];u=off[ls[:,c]==1];g=s.restore.element(M,c,i);mi=im(g)
bad=(ls[ls[:,c]!=1]!=0).any(axis=0)
for q in cap:bad|=labels(mi,int(q))!=0
G1=group(np.flatnonzero(~bad));o1,z1=G1.orbits()
dst=[]
for p in u:
 k=kp.index_of_matrix(to_tuple(matmul(g,st.pose(int(p)))));assert k>=0
 dst.append(int(reg.piece_of[typ[p],kp.compose(k,int(reg.transport[p]))]))
dst=np.array(dst);ais=np.flatnonzero(hc);js=np.unique(o1[dst]);want=np.bincount(o1[dst],minlength=len(z1));outs=[np.flatnonzero((o0==a)&(~home[:,c])) for a in ais]
print('ZERO_MASK',len(u),'fixed',int((z1[o1[dst]]==1).sum()),'source_capacity_deficit',sum(max(0,int(hc[a])-len(xs)) for a,xs in zip(ais,outs)),flush=True)
na,nj=len(ais),len(js);sink=na+nj+1;capmat=np.zeros((sink+1,sink+1),np.int64)
for a,(ai,xs) in enumerate(zip(ais,outs)):
 capmat[0,1+a]=hc[ai]
 for b,j in enumerate(js):capmat[1+a,1+na+b]=int((o1[xs]==j).sum())
for b,j in enumerate(js):capmat[1+na+b,sink]=want[j]
res=capmat.copy();value=0
while True:
 prev=np.full(sink+1,-1);prev[0]=0;queue=deque([0])
 while queue and prev[sink]<0:
  a=queue.popleft()
  for b in np.flatnonzero(res[a]>0):
   if prev[b]<0:prev[b]=a;queue.append(int(b))
 if prev[sink]<0:break
 v=sink;d=10**9
 while v:a=prev[v];d=min(d,int(res[a,v]));v=a
 v=sink
 while v:a=prev[v];res[a,v]-=d;res[v,a]+=d;v=a
 value+=d
J=[]
for a,(ai,xs) in enumerate(zip(ais,outs)):
 chosen=[]
 for b,j in enumerate(js):
  num=int(capmat[1+a,1+na+b]-res[1+a,1+na+b]);chosen.extend(xs[o1[xs]==j][:num].tolist())
 chosen.extend(np.setdiff1d(xs,np.array(chosen,dtype=int))[:int(hc[ai])-len(chosen)].tolist());J+=chosen
J=np.array(J);assert len(J)==len(H) and not home[J,c].any()
print('FLOW',value,'demand',len(dst),'source_orbits',na,'middle_demand_orbits',nj,flush=True)
assert value==len(dst)
assert (np.bincount(o1[J],minlength=len(z1))>=want).all()
for label,GG,targets in [('source',G0,np.array([int(np.flatnonzero(o0==a)[0]) for a in ais if 0<hc[a]<z0[a]])),('middle',G1,dst)]:
 cert=s.B.jordan(GG,targets,seed=29)
 for o,v in cert['certificates'].items():assert s.B.check_jordan(GG,v['word'],int(o),v['prime'])['verified']
 print('FLOW_JORDAN',label,len(cert['certificates']),'missing',cert['missing'],'small',cert['uncertifiable_below_8'],flush=True)
 assert not cert['missing'] and not cert['uncertifiable_below_8']
print('PREPARED_CAP_91_PASS','holes',len(J),'off_landed',len(u),'CPU',round(time.process_time(),1),flush=True)
