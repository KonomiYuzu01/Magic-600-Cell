# Theory round 22, X22C-09 (line (c)), experiment C: FC-shaped two-letter readiness at the depth 60 stop. The senior
# reviewer's command, kept as written apart from this header (the unit is read from the command line); the integrator
# ran it for units 27, 36, 44 and 59 and got the outputs in runs/round22/line-c/r2-<unit>.txt. Research only, headless
# Linux; exact geometry. Run from the repository root:
# python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/x22c09_r2_ready.py <unit>
import sys,resource
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1000,1000))
target=int(sys.argv[1])
p=Path('research/jumbling/restoration/12-low-height/scripts/eb4.py')
src=p.read_text();cut=src.index('    resZ, passing = stage1')
tail=r'''    target=int(sys.argv[4])
    left=[target]
    baseU=U
    L0=labels.copy();C0=rcls.copy();D0=rdom.copy();refs0=list(D.reps)
    adm=np.flatnonzero(~(L0==2).any(axis=0)).tolist()
    straddlers=set()
    for c in bl.I[U[target]['block']]:
        straddlers.update(int(unit_of[r]) for r in np.flatnonzero(L0[:,c]==2))
    first=sorted(g for g in adm if any((L0[units[q],g]==1).all() for q in straddlers))
    print(json.dumps({'unit':target,'first_grips':first,'first_letters':11*len(first)}),flush=True)
    tried=0
    for g in first:
      for i,e in retained(g):
        tried+=1
        mv=L0[:,g]==1;rows=np.flatnonzero(mv)
        labels=L0.copy();labels[rows]=L0[np.ix_(rows,perms[int(inv[e])])]
        rcls=C0.copy();good=C0[rows]>=0;rcls[rows[good]]=cmap(e)[C0[rows[good]]]
        rdom=D0.copy();refs=list(refs0)
        for d in np.unique(D0[rows]).tolist():
            ER=matmul(kp.matrix(e),refs0[d])
            j=next((j for j in range(1,len(refs)) if kp.index_of_matrix(to_tuple(matmul(transpose(refs[j]),ER)))>=0),None)
            if j is None:j=len(refs);refs.append(ER)
            rdom[mv&(D0==d)]=j
        D=SimpleNamespace(reps=refs);ykey_cache.clear()
        inside=labels==1;straddle=labels==2
        U={k:dict(v) for k,v in baseU.items()}
        if mv[units[target]].all():
            u=U[target];b=baseU[target]
            u['class']=int(cmap(e)[b['class']])
            u['cell']=[int(perms[e][c]) for c in b['cell']]
            u['needed']={'all':[int(perms[e][c]) for c in b['needed']['all']],'one_of':[[int(perms[e][c]) for c in s] for s in b['needed']['one_of']]}
        result,passing=stage1(range(len(units)),'r2')
        r=result['units'][0]
        if r.get('ready_without_letter') or passing:
            print(json.dumps({'unit':target,'A1':[g,i],'ready_after_A1':r.get('ready_without_letter'),'A2':[passing[0][1]['grip'],passing[0][1]['item']] if passing else None,'first_letters_tried':tried,'complete':False}),flush=True)
            return
    print(json.dumps({'unit':target,'first_letters_tried':tried,'ready_pair':None,'complete_FC_shape':True}),flush=True)
'''
ns={'__name__':'eb4_r2_memory','__file__':str(p)}
exec(compile(src[:cut]+tail,str(p),'exec'),ns)
ns['log']=lambda **kw:None
sys.argv=[str(p),'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json','research/jumbling/restoration/12-low-height/runs/screen-k60.json','UNUSED',str(target)]
ns['main']()
