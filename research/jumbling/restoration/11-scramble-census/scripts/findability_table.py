# The numbers of the section "What a person would have to try" from the outputs in runs/findability/:
# - <run>-steps.jsonl (findability.py): single-twist steps, the chosen cap's place among the candidates; repair
#   steps, the openings that the rule's order tries before the one that works;
# - <run>-density.jsonl.gz (density.py): at the chosen cap of a repair step, how many (opening, neighbour) pairs make
#   a repair that lowers N;
# - <run>-single.jsonl (single_twist.py): whether a single twist that lowers N existed at a repair step;
# - repair-items.jsonl (repair_items.py): what the repairs of the census records are made of.
# python3 -I -B findability_table.py <runs/findability dir> <run>
import gzip
import json
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

D, RUN = Path(sys.argv[1]), sys.argv[2]


def lines(p):
    with (gzip.open(p, 'rt') if p.suffix == '.gz' else open(p)) as f:
        return [json.loads(x) for x in f if x.strip()]


steps = [r for r in lines(D / f'{RUN}-steps.jsonl') if 'rule' in r]
assert lines(D / f'{RUN}-steps.jsonl')[-1] == {'ok': True}
print(f'{RUN}: {len(steps)} steps', dict(Counter(r['rule'] for r in steps)))

print('\nSingle-twist steps: the chosen cap among the caps that pass the test')
for rule in ('seam', 'cover'):
    R = [r for r in steps if r['rule'] == rule]
    print(f"  {rule}: {len(R)} steps, chosen cap first in {sum(r['rank'] == 1 for r in R)}, candidates median "
          f"{statistics.median(r['candidates'] for r in R)} max {max(r['candidates'] for r in R)}")

C = [r for r in steps if r['rule'] == 'conj']
ob = [r['openings_before'] for r in C]
print(f'\nRepair steps: {len(C)}; openings tried before the one that worked: min {min(ob)}, median '
      f'{statistics.median(ob)}, max {max(ob)}; at most 10 in {sum(o <= 10 for o in ob)} steps')
print(f"  chosen cap by decreasing N_c: {dict(sorted(Counter(r['rank_c'] for r in C).items()))}")
print(f"  opening retained (items 1 to 11): {sum(r['x'] < 12 for r in C)}; z undoes x: "
      f"{sum(r['z_inverse_of_x'] for r in C)}; c covers d: {sum(r['c_covers_d'] for r in C)}; d covers c: "
      f"{sum(r['d_covers_c'] for r in C)}")
print(f"  face match after the opening, at d: {sum(r['d_face_match_after_x'] for r in C)}; at c after y: "
      f"{sum(r['c_face_match_after_xy'] for r in C)}")
print(f"  scramble labels: x undoes the last scramble twist of c {sum(r['x_undoes_last_scramble_twist_of_c'] for r in C)}"
      f", y undoes that of d {sum(r['y_undoes_last_scramble_twist_of_d'] for r in C)}")
print(f"  search seconds: median {statistics.median(r['search_sec'] for r in C)}, max {max(r['search_sec'] for r in C)}")

dens = defaultdict(list)
drows = lines(D / f'{RUN}-density.jsonl.gz')
assert drows[-1] == {'done': True}
for r in drows[:-1]:
    dens[r['step']].append(r)
print('\n| Step | N | Cap c | Its place | N_c | Openings | Neighbours | Pairs | Lowering N | Share | Retained openings among them |')
print('| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
byst = {r['step']: r for r in C}
for s in sorted(dens):
    R, rc = dens[s], byst[s]
    low = [r for r in R if r['delta'] is not None and r['delta'] < 0]
    assert sum(r['chosen'] for r in R) == 1 and any(r['chosen'] for r in low)
    print(f"| {s} | {rc['N_before']:,} | {rc['c']} | {rc['rank_c']} | {rc['N_c']} | {len({r['x'] for r in R})} | "
          f"{len({r['d'] for r in R})} | {len(R):,} | {len(low)} | {100 * len(low) / len(R):.2f}% | "
          f"{sum(r['x'] < 12 for r in low)} |")

single = [r for r in lines(D / f'{RUN}-single.jsonl') if 'step' in r]
print(f"\nSingle twists that lower N, at the {len(single)} repair steps: some cap had one at "
      f"{sum(r['caps_with_single_drop'] > 0 for r in single)} steps; from step 40 on at "
      f"{sum(r['caps_with_single_drop'] > 0 for r in single if r['step'] >= 40)} of "
      f"{sum(r['step'] >= 40 for r in single)}, at most {max(r['caps_with_single_drop'] for r in single if r['step'] >= 40)} "
      f"cap at a time, largest drop {max(r['best_single_drop'] for r in single if r['step'] >= 40)}")

items = lines(D / 'repair-items.jsonl')
n = sum(r['repairs'] for r in items)
rrr = sum(r['kinds_xyz'].get('RRR', 0) for r in items)
print(f"\nRepairs in {len(items)} census records: {n}; all three twists retained {rrr} ({100 * rrr / n:.0f}%), z undoes x "
      f"{sum(r['z_undoes_x'] for r in items)}, net turn of c retained {sum(r['net_turn_of_c_retained'] for r in items)}")
for r in items:
    print(f"  {r['run']}: {r['repairs']} repairs, RRR {r['kinds_xyz'].get('RRR', 0)}, z undoes x {r['z_undoes_x']}")
