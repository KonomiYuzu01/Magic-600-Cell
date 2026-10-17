# Replay check of census records: rebuild each scramble from its seed and check the record's start (the history,
# the off count after the scramble, and the start of the round: N, off count, heights and off poses after the
# scramble, or for a continued round after the replayed moves, which must end at a step boundary); then apply the
# recorded moves step by step on the exact simulator (every twist must be admissible and certain) and compare the
# off count and the total misalignment N after every step (N must drop at every step), the twist count, the end
# state, the end heights and off poses, the result, and for a run that reached the lattice the solved flag and the
# invariant v with the record. Prints one JSON line per record with its end digest; exits 1 if any record fails.
# Run from the repository root:  python3 -I -B check_records.py <result json>...
import sys, json
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore, rim, sim, trees as T


def heights(st):
    """(histogram of left heights over the off pieces, number of off poses)."""
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    h, poses = {}, 0
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        poses += 1
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        h[d] = h.get(d, 0) + c
    return {str(a): h[a] for a in sorted(h)}, poses


M = restore.menu('I_a')
bad = 0
for f in sys.argv[1:]:
    r = json.loads(Path(f).read_text())
    fails = []
    if r['menu'] != M.name:
        fails.append('menu')
    st, hist = restore.scramble(M, r['k'], r['seed'])
    if [list(h) for h in hist] != [list(h) for h in r['history']]:
        fails.append('history')
    N, off = rim.Domains(st).total_misalignment(), int(st.off_lattice_count())
    if off != r['off_scramble']:
        fails.append('off_scramble')
    moves = [tuple(m) for m in r['moves']]
    if r['twists'] != len(moves):
        fails.append('twists is not the number of moves')
    bounds = [0]
    for s in r['steps']:
        bounds.append(bounds[-1] + s['twists'])
    if bounds[-1] != len(moves):
        fails.append('step twists do not add up to the moves')
    start = r['continued_from_moves']
    if start not in bounds:
        fails.append('continued_from_moves is not a step boundary')

    def check_start():
        if (N, off, heights(st)) != (r['N_start'], r['off_start'], (r['heights_start'], r['off_poses_start'])):
            fails.append('start of this round: N, off, heights or off poses')

    if start == 0:
        check_start()
    pos = 0
    for j, s in enumerate(r['steps']):
        if s['N_before'] != N:
            fails.append(f'step {j}: N_before {s["N_before"]} != {N}')
        if not s['N_after'] < s['N_before']:
            fails.append(f'step {j}: N does not drop')
        for c, i in moves[pos:pos + s['twists']]:
            o = st.apply(sim.Twist(restore.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name}))
            if not o.applied or o.uncertain:
                fails.append(f'step {j}: twist {(c, i)} not admissible')
        pos += s['twists']
        N, off = rim.Domains(st).total_misalignment(), int(st.off_lattice_count())
        if (N, off) != (s['N_after'], s['off_after']):
            fails.append(f'step {j}: (N, off) {(N, off)} != {(s["N_after"], s["off_after"])}')
        if start and pos == start:
            check_start()
    if (N, off) != (r['N_now'], r['off_now']):
        fails.append('end N or off')
    if (r['result'] == 'lattice') != (N == 0):
        fails.append(f'result {r["result"]} with N = {N}')
    hE = heights(st)
    if 'heights_end' in r and hE != (r['heights_end'], r['off_poses_end']):
        fails.append('heights_end or off_poses_end')
    if r['result'] == 'lattice' and off == 0:
        solved = bool(np.array_equal(st.lattice_stickers()['labels'], np.arange(restore.NS)))
        if solved != r.get('solved'):
            fails.append('solved')
        if bool(restore.end_invariant(st)[1]) != r.get('v_end_in_vG'):
            fails.append('v_end_in_vG')
    out = {'run': r['run'], 'record': Path(f).name, 'result': r['result'], 'continued_from_moves': start,
           'steps': len(r['steps']), 'twists': len(moves), 'off_now': off, 'N_now': N, 'heights_now': hE[0],
           'off_poses_now': hE[1], 'digest': st.digest(), 'pass': not fails, 'fails': fails[:5]}
    bad += bool(fails)
    print(json.dumps(out), flush=True)
sys.exit(1 if bad else 0)
