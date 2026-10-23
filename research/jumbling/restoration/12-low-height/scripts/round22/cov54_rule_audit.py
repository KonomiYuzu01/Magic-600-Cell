# Theory round 22, coverage round 8 (COV-54): an instrumented audit of the lock and shift rules of the existing
# descent at COV-43's table (y, h) = (12, 12). Through the head of descent_cost.py it builds the table, then gives the
# cut faces of the hole chamber 12, whether the off piece's rotated normals at those faces are grip poles, and the faces
# where the piece blocks the grip (FACES); the descent's grip survey with each grip's score and the smallest and largest
# score after one letter (SURVEY); and, for level2 (lock) and shift, an in-memory copy of the rule from
# research/jumbling/theory/restore.py that counts every opening by kind (retained or jumble) and score change and every
# scored continuation by its net change (AUDIT), asserting that the rule returns None and leaves the digest unchanged.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov54_rule_audit.txt.
# It writes no file and does not change restore.py. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov54_rule_audit.py
from pathlib import Path
import sys,time,collections,inspect,textwrap
sys.argv=['descent_cost.py','12','12','900'];p=Path('research/jumbling/restoration/12-low-height/scripts/round22/descent_cost.py');n={};exec(compile(p.read_text().split('t0=time.process_time()')[0],str(p),'exec'),n)
s,M,st,np=[n[k] for k in ('s','M','root','np')];r=s.restore.Restorer(st,M);D=r.domains();d=st.digest()
from exact import matvec
home=s.CL.data.sig_matrix(np.array([12]))[0];rim=s.restore.rim
facets=[c for c in range(600) if rim.partner([12],[c],not bool(home[c]))[0]>=0]
print('FACES',facets,'rotated_poles',[s.CL.pole(matvec(st.pose(12),s.CL.N[c])) for c in facets],'blocked_hole_faces',[c for c in facets if r.inside(c) is None],flush=True)
print('SURVEY',{c:(a,min(b.values()),max(b.values())) for c,(a,b) in r.survey().items()},flush=True)
for name in ('level2','shift'):
 code=textwrap.dedent(inspect.getsource(getattr(s.restore.Restorer,name)))
 code=code.replace('d_open = after_c[i] - now_c',"d_open = after_c[i] - now_c; self.audit['openings'][(i<12,int(d_open))]+=1")
 if name=='level2':
  code=code.replace('if d_open - n_d - (cl[0][0] if cl else 0) < 0:',"self.audit['tests'][int(d_open - n_d - (cl[0][0] if cl else 0))]+=1\n                if d_open - n_d - (cl[0][0] if cl else 0) < 0:")
 else:
  code=code.replace('low = min(a_d.values())',"low = min(a_d.values()); self.audit['tests'][int(d_open - (n_d-low))]+=1")
 env=dict(s.restore.__dict__);exec(compile(code,'<audited-rule>','exec'),env)
 r.audit={'openings':collections.Counter(),'tests':collections.Counter()};t=time.process_time();out=env[name](r)
 print('AUDIT',name,'result',out,'openings',sorted(r.audit['openings'].items()),'candidate_count',sum(r.audit['tests'].values()),'min_net_change',min(r.audit['tests'],default=None),'net_changes',sorted(r.audit['tests'].items()),'unchanged',st.digest()==d,'cpu',round(time.process_time()-t,2),flush=True)
 assert out is None and st.digest()==d
