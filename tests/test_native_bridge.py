"""Synthetic interface test. This does NOT count as a native executable test.
Builds a differently enumerated native-style export, then verifies all full generators.
"""
from pathlib import Path
import sys,tempfile,json,time
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from core import Model
from grips import grips
from native_bridge import verify_native

def make_fixture(m):
 rng=np.random.default_rng(600);face_order=rng.permutation(600)
 mapping=np.concatenate([c*433+rng.permutation(433) for c in face_order]).astype(np.int32);inverse=np.argsort(mapping)
 lookup={}
 for p in range(m.np):
  mask=sum(1<<c for c in m.caps(p))
  for h,s in zip(m.hosting(p),m.slots(p)):lookup[(mask,h)]=int(s)
 def rotate(slots,rho):
  masks={int(p):sum(1<<int(rho[c]) for c in m.caps(int(p))) for p in np.unique(m.sp[slots])}
  return np.array([lookup[masks[int(m.sp[s])],int(rho[s//433])] for s in slots],np.int32)
 opp=int(np.argmin(np.linalg.norm(m.normals+m.normals[0],axis=1)))
 base_slots=[]
 for c in (0,opp):
  a=m.ids[m.cap_mask(c)[m.sp]];base_slots.append(a[np.argsort(inverse[a])])
 twists=[]
 for g in grips(m,0)['axes']:
  rho=np.arange(600)
  for k in g['word']:
   r=m.z['rotperms'][abs(k)-1];rho=(r if k>0 else np.argsort(r))[rho]
  maps=[]
  for ss in base_slots:
   index={int(s):i for i,s in enumerate(ss)};maps.append([index[int(s)] for s in rotate(ss,rho)])
  twists.append(dict(order=g['order'],maps=[maps[0],None,maps[1]]))
 faces=[dict(id=i,pole=m.normals[c].tolist(),first=i*433,count=433) for i,c in enumerate(face_order)]
 axes=[];seen=set();start=time.perf_counter()
 for c in range(600):
  if c in seen:continue
  oc=int(np.argmin(np.linalg.norm(m.normals+m.normals[c],axis=1)));seen|={c,oc}
  F=m.cell_frames[c];rho=np.argmax((m.normals@F.T)@m.normals.T,axis=1)
  assert int(rho[0])==c and len(np.unique(rho))==600
  ss=[inverse[rotate(a,rho)].tolist() for a in base_slots]
  axes.append(dict(id=len(axes),base=0,dir=m.normals[c].tolist(),fixedMask=2,layers=[ss[0],None,ss[1]]))
  if len(axes)%50==0:print('Synthetic axes',len(axes),round(time.perf_counter()-start,2),flush=True)
 return dict(format='MPUlt-native600-v1',n=m.n,faces=faces,bases=[dict(id=0,cuts=[.968,-.968],twists=twists)],axes=axes,executable_sha256='SYNTHETIC-NOT-A-NATIVE-RUNTIME'),mapping

def main():
 m=Model();start=time.perf_counter();d,mapping=make_fixture(m)
 with tempfile.TemporaryDirectory() as td:
  result=verify_native(m,d,Path(td));assert np.array_equal(result['native_to_lab'],mapping);assert result['matched_generators']==1200
  expected=(Path(td)/'native_profile.json').read_bytes()
  d['bases'][0]['cuts'][0]=.967
  try:verify_native(m,d,Path(td));raise AssertionError('Bad cuts accepted')
  except ValueError:pass
  assert (Path(td)/'native_profile.json').read_bytes()==expected
  d['bases'][0]['cuts'][0]=.968;d['axes'][1]['id']=0
  try:verify_native(m,d,Path(td));raise AssertionError('Duplicate axes accepted')
  except ValueError:pass
 report=dict(passed=True,scope='Synthetic shuffled face/sticker enumeration and proper cap rotations, not an actual native export',matched_stickers=259800,matched_generators=1200,invalid_cuts_rejected=True,duplicate_axis_ids_rejected=True,existing_profile_unchanged_on_failure=True,seconds=time.perf_counter()-start)
 (Path(__file__).parent/'v02').mkdir(parents=True,exist_ok=True);(Path(__file__).parent/'v02'/'native_bridge_synthetic.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2),flush=True)
if __name__=='__main__':main()
