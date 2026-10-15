"""Rim survey of a pose table, and optionally the rim descent of seams.py from it (reads only the state).

Usage: python -I seamx.py <theory dir> <pose table> <out json> [--run MAX_TWISTS] [--order a,b,c]
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import orient as O  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


a = sys.argv[1:]
table, outp = a[1], Path(a[2])
R, M = O.start(table)
if '--order' in a:
    R.order = tuple(a[a.index('--order') + 1].split(','))
st = R.st
d0 = st.digest()
N0 = R.misalignment()
sv = R.survey()
gm = R.grip_misalignment()
log(digest=d0, off=st.off_lattice_count(), N=N0, grip_N=gm, misaligned_caps=len(sv))
res = {'input_digest': d0, 'off': st.off_lattice_count(), 'N': N0, 'grip_N': gm,
       'survey': {str(c): {'now': now, 'best_after': min(after.values()) if after else None}
                  for c, (now, after) in sv.items()}}
outp.write_text(json.dumps(res, indent=1) + '\n')
if '--run' in a:
    mx = int(a[a.index('--run') + 1])

    def steplog(step):
        log(**step, N_now=R.misalignment(), off=st.off_lattice_count())
        res['moves'] = [list(m) for m in R.moves]
        res['steps'] = res.get('steps', []) + [step]
        outp.write_text(json.dumps(res, indent=1) + '\n')
    out, steps = R.run(max_twists=mx, log=steplog)
    res.update({'result': out, 'moves': [list(m) for m in R.moves], 'N_end': R.misalignment(),
                'off_end': st.off_lattice_count(), 'digest_end': st.digest(), 'seconds': round(time.time() - T0)})
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(result=out, N_end=res['N_end'], off_end=res['off_end'], twists=len(R.moves))
