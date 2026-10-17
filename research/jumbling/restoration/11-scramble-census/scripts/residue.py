# Residue of a phase U census run: rebuild the stop state from the scramble seed and the solver's moves (no journal),
# export its pose table, list the restoration units with mkblocks2.py (stage 09: rigid blocks, sizes, admissible
# movers, frames), and summarize them: block sizes, blocks without an admissible mover, and the left heights of the
# pieces in each block. Run from the repository root:
#   python3 -I -B residue.py <census result json> <out dir>
import sys, json, subprocess
from collections import Counter
from pathlib import Path
import numpy as np
REPO = Path.cwd()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
import restore, sim, buffer as B, trees as T

res = json.loads(Path(sys.argv[1]).read_text())
out = Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True)
M = restore.menu('I_a')
st, hist = restore.scramble(M, res['k'], res['seed'])
assert [tuple(h) for h in hist] == [tuple(h) for h in res['history']], 'scramble differs from the recorded history'
for c, i in res['moves']:
    o = st.apply(sim.Twist(restore.ctx, c, restore.element(M, c, i), 'menu', {'menu': M.name}))
    assert o.applied and not o.uncertain, ('solver move not admissible', c, i)
name = f"{res['run']}-stop"
table = out / f'{name}-table.json'
B.export_pose_table(st, str(table))
assert int(st.off_lattice_count()) == res['off_now'], (st.off_lattice_count(), res['off_now'])
gm = REPO / 'research/jumbling/restoration/09-general-method/scripts'
p = subprocess.run([sys.executable, '-I', '-B', str(gm / 'gm/mkblocks2.py'), str(TH), str(gm / 'echain'), str(table), name,
                    str(out / f'{name}-units')], capture_output=True, text=True, cwd=REPO)
line = [l for l in p.stdout.splitlines() if l.startswith('{')]
if p.returncode or not line:
    print(json.dumps({'run': res['run'], 'error': p.stderr[-800:]})); sys.exit(1)
u = json.loads(line[-1])
blocks = u['blocks']

# left height of every off piece (tree distance of the left factor of its pose)
hpose = {}
for pid in np.unique(st.pose_id).tolist():
    if st._poses[pid].kidx < 0:
        hpose[pid] = int(T.dist(T.factors(st._pose_matrix(pid))[0]))
height = {int(q): hpose[int(st.pose_id[q])] for q in np.flatnonzero(np.isin(st.pose_id, list(hpose))).tolist()}
rows = []
for j, size, movers, first, frames in blocks:
    doc = json.loads((out / f'{name}-units' / f'{name}-b{j:02d}.json').read_text())
    assert doc['size'] == size and len(doc['pieces']) == size
    hs = Counter(height[q] for q in doc['pieces'])
    rows.append({'block': j, 'size': size, 'movers': movers, 'frames': frames, 'first': first,
                 'heights': {str(h): n for h, n in sorted(hs.items())}})
assert sum(r['size'] for r in rows) == len(height) == res['off_now']
sizes = Counter(r['size'] for r in rows)
top = sorted(sizes.items(), key=lambda x: -x[0] * x[1])
frozen = [r for r in rows if r['movers'] == 0]
high = [r for r in rows if max(int(h) for h in r['heights']) > 2]
summary = {'run': res['run'], 'digest': st.digest(), 'off': int(st.off_lattice_count()), 'blocks': len(rows),
           'units': len(u['units']), 'largest_blocks': sorted(sizes.elements(), reverse=True)[:10],
           'pieces_by_block_size': [[s, n, s * n] for s, n in top],
           'blocks_without_mover': len(frozen), 'pieces_without_mover': sum(r['size'] for r in frozen),
           'multi_frame_blocks': sum(1 for r in rows if r['frames'] > 1),
           'pieces_by_height': {str(h): n for h, n in sorted(Counter(height.values()).items())},
           'blocks_with_height_above_2': len(high), 'pieces_in_those_blocks': sum(r['size'] for r in high),
           'table': table.name}
(out / f'{name}-summary.json').write_text(json.dumps(summary) + '\n')
(out / f'{name}-blocks.json').write_text(json.dumps({'run': res['run'], 'digest': summary['digest'], 'blocks': rows}) + '\n')
print(json.dumps(summary))
