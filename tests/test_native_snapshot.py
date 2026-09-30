"""Real HTTP + full model, using an explicitly synthetic native enumeration.
Does not load MPUlt or substitute for the user's live executable bridge.
"""
from pathlib import Path
import sys,json,time,base64,urllib.request,urllib.error
from urllib.parse import urlparse,parse_qs
import numpy as np
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT))
from core import Model,canonical
from test_native_bridge import make_fixture
u=urlparse(sys.argv[1]);assert u.hostname=='127.0.0.1'
base=f'{u.scheme}://{u.netloc}';token=parse_qs(u.query)['token'][0]
def call(path,body=None):
 req=urllib.request.Request(base+'/api/'+path,data=None if body is None else canonical(body).encode(),headers={'X-C600-Token':token,'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=180) as r:result=json.load(r)
 if isinstance(result,dict) and 'job' in result:
  while True:
   job=call('job/'+result['job'])
   if job['done']:
    if 'error' in job:raise RuntimeError(job['error'])
    return job['result']
   time.sleep(.05)
 return result
start=time.perf_counter();m=Model();d,mapping=make_fixture(m);p=call('native/handshake',d);assert p['matched_stickers']==m.n and p['matched_generators']==1200
profile=call('native/map');initial=call('status')['state_hash'];expected=m.ids.copy();checks=[]
def snap():
 q=call('native/snapshot');assert q['format']=='C600-native-snapshot-v1';assert q['profile_sha256']==profile['profile_sha256']
 c=np.frombuffer(base64.b64decode(q['colors']),'<i2');styles=np.frombuffer(base64.b64decode(q['styles']),'u1')
 assert len(c)==m.n and len(styles)==m.n and np.all((styles>=0)&(styles<=6))
 inv=np.argsort(profile['native_face_to_lab']);assert np.array_equal(c,inv[expected[mapping]//433])
 return q
assert snap()['state']['state_hash']==initial
checks.append('Atomic status/color/style snapshot matches synthetic mapping and solved state')
for word in [[2,4,6],[1,8,-4],[10,12,-2,5]]:
 pr=call('preview',dict(recipe=[dict(kind='word',moves=word)]));call('commit',dict(token=pr['token']))
 for move in word:
  src,dst=m.move(move);expected[dst]=expected[src]
 snap()
checks.append('Snapshots after legal commits exactly match every mapped sticker color')
call('prefs',dict(rules=[dict(expr='O33',style='solid')],pin_safety=False,selected=None))
q=snap();styles=np.frombuffer(base64.b64decode(q['styles']),'u1');assert np.all((styles>0)==(m.oid[m.sp[mapping]]==33))
checks.append('Visibility and status are produced under one session lock, independent of hidden mechanics')
pr=call('preview',dict(recipe=[dict(kind='word',moves=[2])]))
call('commit',dict(token=pr['token']));old_hash=q['state']['state_hash']
try:call('native/turn',dict(pre_state=old_hash,tokens=['0:0:1:1']));raise AssertionError('Stale native turn accepted')
except urllib.error.HTTPError as e:assert e.code==400
checks.append('Stale native turn is rejected without an extra commit')
call('checkout',dict(head=0));expected=m.ids.copy();assert snap()['state']['state_hash']==initial
# Concurrent updates must not combine one state's metadata with another state's
# colors. Compare each atomic reply against two independently computed states.
from concurrent.futures import ThreadPoolExecutor
p0=call('native/snapshot');c0=base64.b64decode(p0['colors']);h0=p0['state']['state_hash']
pr=call('preview',dict(recipe=[dict(kind='word',moves=[2])]))
call('commit',dict(token=pr['token']));p1=call('native/snapshot');h1=p1['state']['state_hash'];c1=base64.b64decode(p1['colors'])
known={h0:c0,h1:c1}
def toggle():
 for i in range(12):
  call('undo',{});call('prefs',dict(rules=[dict(expr='O27',style='solid')]))
  call('redo',{});call('prefs',dict(rules=[dict(expr='O33',style='solid')]))
with ThreadPoolExecutor(max_workers=1) as ex:
 future=ex.submit(toggle)
 for i in range(24):
  value=call('native/snapshot');assert base64.b64decode(value['colors'])==known[value['state']['state_hash']]
  expr=value['state']['prefs']['rules'][0]['expr'];o=int(expr[1:]);vis=np.frombuffer(base64.b64decode(value['styles']),'u1')>0
  assert np.array_equal(vis,m.oid[m.sp[mapping]]==o)
 future.result()
checks.append('24 atomic snapshots stayed consistent during concurrent undo/redo and filter writes')
call('checkout',dict(head=0))
call('prefs',dict(rules=[dict(expr='active',style='solid')],pin_safety=True))
report=dict(passed=True,scope='Live HTTP with synthetic full native geometry; no actual MPUlt runtime execution',checks=checks,matched_generators=1200,matched_stickers=m.n,seconds=time.perf_counter()-start)
(ROOT/'tests/v022').mkdir(parents=True,exist_ok=True)
(ROOT/'tests/v022/native_snapshot.json').write_text(json.dumps(report,indent=2),encoding='utf-8');print(json.dumps(report,indent=2),flush=True)
