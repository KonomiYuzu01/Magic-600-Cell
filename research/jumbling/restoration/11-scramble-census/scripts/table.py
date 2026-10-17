# One row per census record: depth, outcome, twists, off and N at the start and at the stop, heights, rule counts,
# and the two phases of a descent: how many steps came before the first conj step and how many pieces were off
# then, the N drop of the single-twist steps (seam, cover), and for the conj steps of this round their N drop and
# search time (seconds between finished steps).
# python3 -I -B table.py <runs dir>
import sys, json, collections, statistics
from pathlib import Path

rows = []
for f in sorted(Path(sys.argv[1]).glob('ia-k*-s*.json'), key=lambda p: (int(p.stem.split('-')[1][1:]), p.stem)):
    r = json.loads(f.read_text())
    steps = r['steps']
    rc = collections.Counter(s['rule'] for s in steps)
    tw = collections.Counter()
    for s in steps:
        tw[s['rule']] += s['twists']
    # steps of earlier rounds: those whose twists make up the replayed moves
    done, prior = 0, 0
    while prior < len(steps) and done < r['continued_from_moves']:
        done += steps[prior]['twists']
        prior += 1
    assert done == r['continued_from_moves']
    first_conj = next((j for j, s in enumerate(steps) if s['rule'] == 'conj'), None)   # = steps before it
    single = [s['N_before'] - s['N_after'] for s in steps if s['rule'] in ('seam', 'cover')]
    prev, conj_sec, conj_drop = 0, [], []
    for s in steps[prior:]:
        if s['rule'] == 'conj':
            conj_sec.append(s['sec'] - prev)
            conj_drop.append(s['N_before'] - s['N_after'])
        prev = s['sec']
    rows.append({'run': r['run'], 'k': r['k'], 'seed': r['seed'], 'result': r['result'], 'twists': r['twists'],
                 'steps': len(steps), 'steps_this_round': len(steps) - prior,
                 'off_scramble': r['off_scramble'], 'off_start': r['off_start'], 'off_now': r['off_now'],
                 'N_start': r['N_start'], 'N_now': r['N_now'],
                 'heights_start': r.get('heights_start'), 'heights_end': r.get('heights_end'),
                 'off_poses_end': r.get('off_poses_end'), 'rules': dict(rc), 'rule_twists': dict(tw),
                 'steps_before_first_conj': first_conj,
                 'off_before_first_conj': None if first_conj is None else
                 (steps[first_conj - 1]['off_after'] if first_conj else r['off_scramble']),
                 'single_twist_N_drop_median': statistics.median(single) if single else None,
                 'conj_steps_this_round': len(conj_sec),
                 'conj_sec_median': statistics.median(conj_sec) if conj_sec else None,
                 'conj_sec_max': max(conj_sec) if conj_sec else None,
                 'conj_N_drop_median': statistics.median(conj_drop) if conj_drop else None,
                 'seconds': r['seconds'], 'solved': r.get('solved'), 'v_end_in_vG': r.get('v_end_in_vG')})
for x in rows:
    print(json.dumps(x))
