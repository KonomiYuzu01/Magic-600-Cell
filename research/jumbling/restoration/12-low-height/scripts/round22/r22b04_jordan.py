# Theory round 22, R22B-04: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22b04_jordan.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22b04_jordan.py
import sys, json, random, hashlib, resource, signal, time
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU, (840, 850))
def stop(*a):
    print('CUT_SHORT_CPU_840_SECONDS', flush=True)
    raise SystemExit(124)
signal.signal(signal.SIGXCPU, stop)
sys.path[:0] = [str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()), str(Path('research/jumbling/restoration/09-general-method/scripts/echain').resolve())]
import screen, buffer as B, fastghost as FG
import numpy as np
st, M, src = screen.load('record', 'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
print('STATE', st.digest(), int(st.off_lattice_count()), flush=True)
G = B.Alphabet(SimpleNamespace(st=st), M)
G.orbits()
base = Path('research/jumbling/restoration/12-low-height/runs/lc13-k40')
W = json.loads((base/'transport-b00-word.json').read_text())['word']
U = np.flatnonzero(G.home[:,sorted({c for c,i in W})].any(axis=1))
g = FG.Ghost(FG.Frames(), np.zeros(len(U), np.int64), U)
for c,i in W: g.apply(c, screen.restore.element(M,c,i), (c,i))
T = np.sort(U[~g.alive | (g.dom != 0)])
print('TARGET', len(T), int(G.occupied[T].sum()), hashlib.sha256(json.dumps(T.tolist()).encode()).hexdigest(), flush=True)
cond = B.condition(G,T)
print('CONDITION', json.dumps(cond), flush=True)
want = {int(o) for o in G.orbit[T] if G.orbit_size[o] > 1}
found = {}
for stage,(length,tries) in enumerate(((120,200),(400,400),(800,400),(1500,400))):
    rng = random.Random(1+stage)
    for trial in range(tries):
        if set(found) == want: break
        word = [rng.randrange(len(G.letters)) for _ in range(length)]
        lab,lens = B.cycles(G.perm(word))
        for o in sorted(want-set(found)):
            try: cert = B._jordan_from_cycles(G,lab,lens,o)
            except ValueError: continue
            pairs = G.pairs(word)
            assert B.check_jordan(G,pairs,o,cert['prime']) == cert
            found[o] = (stage,trial+1,cert)
            print('JORDAN',json.dumps(dict(cert,stage=stage,trial=trial+1,length=length,word_sha256=hashlib.sha256(json.dumps(pairs,separators=(',',':')).encode()).hexdigest())),flush=True)
        if trial % 20 == 19: print('PROGRESS',stage,trial+1,len(found),len(want),round(time.process_time(),1),flush=True)
    if set(found) == want: break
print('JORDAN_DONE', sorted(want-set(found)), 'cpu',round(time.process_time(),1),flush=True)
