# The numbers of the section "What a person would have to try" from the outputs in runs/findability/:
# - <run>-steps.jsonl (findability.py): every step, the cost of a face match search from the facts the program
#   shows; single-twist steps, the candidates; repair steps, the openings and pairs that the rule's order tries
#   before the one that works;
# - <run>-density.jsonl.gz and <run>-density-rest.jsonl.gz (density.py): at the chosen cap of a repair step, how many
#   (opening, neighbour) pairs make a repair that lowers N, at 15 tail steps and at the other repair steps; these are
#   the pairs the repair list shows (owner decision of 17 October 2026);
# - <run>-single.jsonl (single_twist.py): whether a single twist that lowers N existed at a repair step, and the caps
#   above their floor without one;
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

print('\nSingle-twist steps: the caps that pass the test (the rule takes the one with the largest drop)')
for rule in ('seam', 'cover'):
    R = [r for r in steps if r['rule'] == rule]
    assert all(r['rank'] == 1 for r in R)
    print(f"  {rule}: {len(R)} steps, candidates median {statistics.median(r['candidates'] for r in R)} max "
          f"{max(r['candidates'] for r in R)}; twists of the chosen cap that reach its best drop: median "
          f"{statistics.median(r['best_twists'] for r in R)}, one at {sum(r['best_twists'] == 1 for r in R)}")

print('\nFace match search from the facts the program shows (findability.py, face_search)')
assert all(r['face_found'] == (r['rule'] == 'seam') for r in steps)
for label, R in (('face match steps', [r for r in steps if r['face_found']]),
                 ('other steps, where it finds none', [r for r in steps if not r['face_found']])):
    print(f"  {label}: {len(R)}; checks median {statistics.median(r['face_checks'] for r in R)}, max "
          f"{max(r['face_checks'] for r in R)}; caps the cut slice allows median "
          f"{statistics.median(r['face_slice_caps'] for r in R)}, max {max(r['face_slice_caps'] for r in R)}")

C = [r for r in steps if r['rule'] == 'conj']
ob = [r['openings_before'] for r in C]
print(f'\nRepair steps: {len(C)}; openings tried before the one that worked: min {min(ob)}, median '
      f'{statistics.median(ob)}, max {max(ob)}; at most 10 in {sum(o <= 10 for o in ob)} steps')
few = [r for r in C if r['openings_before'] <= 10]
print(f"  the steps with at most 10: {[r['step'] for r in few]}, the chosen cap the first by decreasing N_c at "
      f"{sum(r['rank_c'] == 1 for r in few)}")
pb = [r['pairs_before'] for r in C]
print(f'  pairs (opening, neighbour) tried before the one that worked: median {statistics.median(pb)}, max {max(pb)}')
T = [r for r in C if r['step'] >= 44]
print(f"  from step 44 on, {len(T)} steps: openings median {statistics.median(r['openings_before'] for r in T)}, "
      f"pairs median {statistics.median(r['pairs_before'] for r in T)}, at least 59 openings at "
      f"{sum(r['openings_before'] >= 59 for r in T)}")
print(f"  chosen cap by decreasing N_c: {dict(sorted(Counter(r['rank_c'] for r in C).items()))}")
for label, R in (('before step 44', [r for r in C if r['step'] < 44]), ('from step 44 on', T)):
    print(f"    {label}, {len(R)} steps: {dict(sorted(Counter(r['rank_c'] for r in R).items()))}, median "
          f"{statistics.median(r['rank_c'] for r in R)}; misaligned caps median "
          f"{statistics.median(r['misaligned_caps'] for r in R)}")
print(f"  opening retained (items 1 to 11): {sum(r['x'] < 12 for r in C)}; z undoes x: "
      f"{sum(r['z_inverse_of_x'] for r in C)}; c covers d: {sum(r['c_covers_d'] for r in C)}; d covers c: "
      f"{sum(r['d_covers_c'] for r in C)}")
print(f"  face match after the opening, at d: {sum(r['d_face_match_after_x'] for r in C)}; at c after y: "
      f"{sum(r['c_face_match_after_xy'] for r in C)}")
print(f"  scramble labels: x undoes the last scramble twist of c {sum(r['x_undoes_last_scramble_twist_of_c'] for r in C)}"
      f", y undoes that of d {sum(r['y_undoes_last_scramble_twist_of_d'] for r in C)}")
print(f"  search seconds: median {statistics.median(r['search_sec'] for r in C)}, max {max(r['search_sec'] for r in C)}")

dens, table_steps = defaultdict(list), None
for name in ('density', 'density-rest'):
    drows = lines(D / f'{RUN}-{name}.jsonl.gz')
    assert drows[-1] == {'done': True}
    got = {r['step'] for r in drows[:-1]}
    assert not got & set(dens), 'a step is in both density files'
    for r in drows[:-1]:
        dens[r['step']].append(r)
    table_steps = table_steps or sorted(got)
byst = {r['step']: r for r in C}
assert sorted(dens) == sorted(byst), 'the density files do not cover every repair step'
low = {}
for s, R in dens.items():
    low[s] = [r for r in R if r['delta'] is not None and r['delta'] < 0]
    assert sum(r['chosen'] for r in R) == 1 and any(r['chosen'] for r in low[s]) and len({r['x'] for r in R}) == 59
    assert all(r['c'] == byst[s]['c'] for r in R)
print('\n| Step | N | Cap c | Its place | N_c | Neighbours | Pairs | Lowering N | Share | Of them retained openings |')
print('| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
for s in table_steps:
    R, rc = dens[s], byst[s]
    print(f"| {s} | {rc['N_before']:,} | {rc['c']} | {rc['rank_c']} | {rc['N_c']} | {len({r['d'] for r in R})} | "
          f"{len(R):,} | {len(low[s])} | {100 * len(low[s]) / len(R):.2f}% | {sum(r['x'] < 12 for r in low[s])} |")
print('\nRepair list at the chosen cap (the pairs whose repair, with the rule\'s y and z, lowers N), at every repair step')
for label, S in (('all', sorted(dens)), ('before step 44', [s for s in dens if s < 44]),
                 ('from step 44 on', [s for s in dens if s >= 44])):
    k = [len(low[s]) for s in S]
    print(f"  {label}, {len(S)} steps: pairs median {statistics.median(k)}, min {min(k)}, max {max(k)}; exactly one at "
          f"{sum(x == 1 for x in k)}; pairs tried at the cap median {statistics.median(len(dens[s]) for s in S)}")
print(f"  list sizes: {dict(sorted(Counter(len(low[s]) for s in dens).items()))}")

single = [r for r in lines(D / f'{RUN}-single.jsonl') if 'step' in r]
assert lines(D / f'{RUN}-single.jsonl')[-1] == {'done': True}
print('\nCaps with a single twist that lowers N, before step 40:',
      {r['step']: r['caps_with_single_drop'] for r in single if r['step'] < 40})
af = [r['above_floor_without_drop'] for r in single]
print(f"Caps above their floor with no lowering single twist, at the {len(single)} repair steps: at "
      f"{sum(a > 0 for a in af)} steps, median {statistics.median(af)}, max {max(af)}; at step 40: "
      f"{next(r['above_floor_without_drop'] for r in single if r['step'] == 40)}, the furthest above its floor "
      f"[cap, N_c, floor] {next(r['furthest_above_floor'] for r in single if r['step'] == 40)}")
print(f"Single twists that lower N, at the {len(single)} repair steps: some cap had one at "
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
