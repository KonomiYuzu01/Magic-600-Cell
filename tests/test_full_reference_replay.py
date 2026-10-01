"""Replay all saved seed-600 stars through the actual Studio Model.star_net.
This is an intentionally long model regression, not a Windows UI test.
"""
from pathlib import Path
import sys,json,gzip,time
import numpy as np
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT))
from core import Model, state_hash
m=Model();record=json.load(gzip.open(ROOT/'tests/reference_solve.json.gz','rt'))
assert record['format']=='C600-TRIANGULAR-SOLVE-v1'
for c in record['controllers']:
 o=c['orbit'];assert c['seed']==m.trees[o]['seed']
a=m.ids.copy();start=time.perf_counter()
for move in record['scramble']:
 src,dst=m.move(move);a[dst]=a[src]
if 'scrambled_state_sha256' in record['report']:assert state_hash(a)==record['report']['scrambled_state_sha256']
total=0
for i,(o,n,sign) in enumerate(record['commands']):
 src,dst=m.star_net(o,n,sign);a[dst]=a[src]
 t=m.trees[o];total+=len(t['seed'])+2*len(t['relocation'])+2*len(m.path(o,n))
 if (i+1)%25000==0:print(i+1,'stars',round(time.perf_counter()-start,2),'seconds',flush=True)
assert np.array_equal(a,m.ids)
assert total==int(record['report']['expanded_primitive_count'])
out=dict(passed=True,scope='All saved seed-600 operations replayed using Studio full-collateral star permutations; not native GUI replay',stars=len(record['commands']),scramble_moves=len(record['scramble']),primitive_solution_moves=total,all_labelled_stickers_solved=m.n,seconds=time.perf_counter()-start)
(ROOT/'tests/v021').mkdir(parents=True,exist_ok=True)
(ROOT/'tests/v021/full_reference_replay.json').write_text(json.dumps(out,indent=2));print(json.dumps(out,indent=2),flush=True)
