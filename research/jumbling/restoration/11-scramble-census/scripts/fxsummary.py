# Summary of a template test (a fxrun.py log): the groups with a template shape, the groups restored, and for every
# other group what stops its best carried word, checked in this order: a grip of the word is blocked, the word
# disturbs other off pieces, or chambers of Bad(W) are occupied.
# python3 -I -B fxsummary.py <fxrun jsonl>...
import sys, json
from collections import Counter

for f in sys.argv[1:]:
    ev = [json.loads(line) for line in open(f)]
    targets = next(e for e in ev if e['event'] == 'targets')
    steps = [e for e in ev if e['event'] == 'step']
    stop = [e for e in ev if e['event'] == 'stop'][-1]
    why, occupied = Counter(), []
    for g, words in stop['remaining'].items():
        if not words:
            why['no carried word'] += 1
            continue
        best = min(words, key=lambda w: (len(w['blocked']) > 0, w['disturbed'] > 0 or len(w['conflict_blocks']) > 0,
                                         w['badW_occupied']))
        occupied.append(min(w['badW_occupied'] for w in words))
        if best['T_on'] < best['T']:
            why['does not put the group on the lattice'] += 1
        elif best['blocked']:
            why['grips blocked'] += 1
        elif best['disturbed'] or best['conflict_blocks']:
            why['disturbs other off pieces'] += 1
        elif best['badW_occupied']:
            why['Bad(W) occupied'] += 1
        else:
            why['other'] += 1
    print(json.dumps({'log': f.rsplit('/', 1)[-1], 'groups_with_template_shape': len(targets['with_template']),
                      'carried_words': targets['words'],
                      'restored': [{'group': s['block'], 'pieces': s['size'], 'letters': s['length'],
                                    'off_after': s['end']['off'], 'collateral': s['end']['collateral']} for s in steps],
                      'not_restored': dict(why), 'least_occupied_chambers': sorted(occupied)}))
