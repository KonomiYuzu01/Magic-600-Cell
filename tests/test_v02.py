"""Tests for the 0.2 workflow additions. No Windows runtime is claimed."""
import sys,tempfile,json,time,threading
from pathlib import Path
import numpy as np
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from core import Model,PuzzleState,Filters,invrecipe,canonical
from session import Session
from enhanced import Workflow
from grips import grips
m=Model();checks=[]
def ok(s):checks.append(s);print('PASS',s,flush=True)
with tempfile.TemporaryDirectory() as td:
 s=Session(m,Path(td));wf=Workflow(s)
 assert np.array_equal(Filters(s.st).parse('cap(C013)'),m.cap_mask(13))
 assert np.array_equal(Filters(s.st).parse('shell(C013,0)'),m.cell_mask(13))
 wf.save_set('sample','O33 & current(C000)','identity');before=Filters(s.st,sets=s.prefs['named_sets']).parse('set(sample)');ids=s.st.at[before].copy()
 p=s.preview([{'kind':'word','moves':[2,4,6]}]);s.commit(p['token']);after=Filters(s.st,sets=s.prefs['named_sets']).parse('set(sample)');assert set(s.st.at[after])==set(ids)
 ok('Cap, graph shell and moving identity-set predicates')
 h1=s.head;hash1=s.st.hash;s.undo();s.close();s=Session(m,Path(td));s.redo();assert s.head==h1 and s.st.hash==hash1;wf=Workflow(s)
 ok('Redo survives process/session restart')
 a=[dict(kind='word',moves=[2])];b=[dict(kind='word',moves=[4])]
 p=wf.compose(a,b,'commutator');expected=m.word_net([2,4,-2,-4]);assert np.array_equal(s.pending['src'],expected[0]) and np.array_equal(s.pending['dst'],expected[1]);wf.save_macro('AB');s.commit(p['token']);h2=s.head
 wf.checkout(h1);assert s.st.hash==hash1;p=s.preview([dict(kind='word',moves=[6])]);s.commit(p['token']);h3=s.head;assert h3!=h2;wf.checkout(h2);assert s.head==h2
 ok('Macro algebra, saved recipes and branch checkout')
 p=s.preview(a);s.commit(p['token']);prior=s.head
 try:s.checkpoint('Solved root');raise AssertionError('Root overwrite accepted')
 except ValueError:pass
 wf.checkout(0);assert s.st.hash==PuzzleState(m).hash
 ok('Immutable solved-root checkpoint')
 wf.timer('start');time.sleep(.05);r=wf.timer('pause');assert r['seconds']>=.045;s.close()
for c in (0,1,57,299,599):
 g=grips(m,c);assert len(g['axes'])==7 and g['all_rotations']==11
 for x in g['axes']:
  for z in (x,x['inverse']):
   src,dst=m.word_net(z['word']);assert len(src)>0;u=np.array(z['u']);v=np.array(z['v']);th=z['angle'];P=np.eye(4)+(np.cos(th)-1)*(np.outer(u,u)+np.outer(v,v))+np.sin(th)*(np.outer(v,u)-np.outer(u,v))
   q=np.arange(600)
   for k in z['word']:
    r=m.z['rotperms'][abs(k)-1];q=(r if k>0 else np.argsort(r))[q]
   assert np.max(np.abs(m.normals@P.T-m.normals[q]))<1e-7
ok('All seven grip axes and analytic animation endpoint matrices in five cells')
cancel=threading.Event();cancel.set();m.cancel_event=cancel
try:m.word_net([2]);raise AssertionError('Cancellation ignored')
except InterruptedError:pass
m.cancel_event=None;ok('Cancelled operation refuses publication')
p=Path(__file__).parent/'v02'/'workflow_report.json';p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(dict(passed=True,checks=checks,windows_runtime_tested=False),indent=2))
