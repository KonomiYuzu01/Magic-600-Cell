# Replay check of census records: rebuild each scramble from its seed, apply the recorded moves step by step on the
# exact simulator (every twist must be admissible and certain), and compare the off count and the total misalignment N
# after every step, and the end heights, with the record. Prints one JSON line per record with its end digest.
# Run from the repository root:  python3 -I -B check_records.py <result json>...
import sys, json
from pathlib import Path
import numpy as np
REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore, rim, sim, trees as T


def heights(st):
    pid, cnt = np.unique(st.pose_id, return_counts=True)
    h = {}
    for p, c in zip(pid.tolist(), cnt.tolist()):
        if st._poses[p].kidx >= 0:
            continue
        d = T.dist(T.factors(st._pose_matrix(p))[0])
        h[d] = h.get(d, 0) + c
    return {str(a): h[a] for a in sorted(h)}


M = restore.menu('I_a')
bad = 0
for f in sys.argv[1:]:
    r = json.loads(Path(f).read_text())
    fails = []
    st, hist = restore.scramble(M, r['k'], r['seed'])
    if [list(h) for h in hist] != [list(h) for h in r['history']]:
        fails.append('history')
    if int(st.off_lattice_count()) != r['off_scramble']:
        fails.append('off_scramble')
    if heights(st) != r['heights_start'] and not r['continued_from_moves']:
        fails.append('heights_start')
    N = rim.Domains(st).total_misalignment()
    moves = [tuple(m) for m in r['moves']]
    if sum(s['twists'] for s in r['steps']) != len(moves):
        fails.append('step twists do not add up to the moves')
    pos = 0
    for j, s in enumerate(r['steps']):
        if s['N_before'] != N:
            fails.append(f'step {j}: N_before {s["N_before"]} != {N}')
        for c, i in moves[pos:pos + s['twists']]:
            o = st.apply(sim.Twist(restore.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name}))
            if not o.applied or o.uncertain:
                fails.append(f'step {j}: twist {(c, i)} not admissible')
        pos += s['twists']
        N = rim.Domains(st).total_misalignment()
        off = int(st.off_lattice_count())
        if (N, off) != (s['N_after'], s['off_after']):
            fails.append(f'step {j}: (N, off) {(N, off)} != {(s["N_after"], s["off_after"])}')
        if r['continued_from_moves'] and pos == r['continued_from_moves']:
            # the start of this round: the state after the replayed moves of the earlier rounds
            if (N, off, heights(st)) != (r['N_start'], r['off_start'], r['heights_start']):
                fails.append('start of this round: N, off or heights')
    if (N, int(st.off_lattice_count())) != (r['N_now'], r['off_now']):
        fails.append('end N or off')
    if 'heights_end' in r and heights(st) != r['heights_end']:
        fails.append('heights_end')
    if r.get('solved') is not None:
        solved = int(st.off_lattice_count()) == 0 and bool(np.array_equal(st.lattice_stickers()['labels'],
                                                                          np.arange(restore.NS)))
        if solved != r['solved']:
            fails.append('solved')
    out = {'run': r['run'], 'record': Path(f).name, 'result': r['result'], 'steps': len(r['steps']), 'twists': len(moves),
           'off_now': r['off_now'], 'N_now': r['N_now'], 'heights_now': heights(st), 'digest': st.digest(),
           'pass': not fails, 'fails': fails[:5]}
    bad += bool(fails)
    print(json.dumps(out), flush=True)
sys.exit(1 if bad else 0)
