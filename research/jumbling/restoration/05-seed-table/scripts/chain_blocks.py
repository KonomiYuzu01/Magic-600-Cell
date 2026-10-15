"""Block-wise structure of the E chain X -> E1 -> E2 -> E3 (menu I_a): for each step, the off pieces that remain are a
subset of the off pieces before, each at the identical pose (exact matrix entries of the pose tables), and the pieces
that left form whole blocks of blocks.py. Pure comparison of the given files.
usage: chain_blocks.py <out json> <name>=<pose table> ... -- <name>=<blocks json> ..."""
import json
import sys

out = sys.argv[1]
i = sys.argv.index('--')
tabs = dict(a.split('=', 1) for a in sys.argv[2:i])
blks = dict(a.split('=', 1) for a in sys.argv[i + 1:])
names = list(tabs)
T = {k: json.load(open(v)) for k, v in tabs.items()}


def pose(k, p):
    t = T[k]
    return json.dumps(t['poses'][t['pose_id'][p]])


B = {k: json.load(open(v)) for k, v in blks.items()}
off = {k: {p for b in B[k]['blocks'] for p in b['pieces']} if k in B else set() for k in names}
rows = []
for a, b in zip(names, names[1:]):
    left = off[a] - off[b]
    row = {'step': f'{a} -> {b}', 'off_before': len(off[a]), 'off_after': len(off[b]),
           'remaining_subset': off[b] <= off[a], 'remaining_same_pose': all(pose(a, p) == pose(b, p) for p in off[b]),
           'left_pieces': len(left),
           'left_is_union_of_blocks': all(set(x['pieces']) <= left or not (set(x['pieces']) & left) for x in B[a]['blocks']),
           'blocks_left': sorted(x['size'] for x in B[a]['blocks'] if set(x['pieces']) <= left)}
    if b in T and 'digest' in T[b]:
        row['digest_after'] = T[b]['digest']
    rows.append(row)
    print(row)
json.dump({'note': __doc__.split('usage')[0], 'steps': rows}, open(out, 'w'), indent=1)
