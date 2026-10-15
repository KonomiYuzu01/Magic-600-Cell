"""Summarize a transport run log: its steps and, at its stop event, why each remaining block has no clean word.

The stop event lists, per remaining block, its three best carried words, sorted by (number of blocked grips, other
off pieces disturbed, occupied chambers of Bad(W)). The first of them decides the class:
  blocked  - every carried word uses a grip that an off piece straddles;
  conflict - some carried word uses unblocked grips only, but each such word kills or moves other off pieces;
  T_off    - the best word leaves part of the block off the lattice;
  vacancy  - the best word has clean ghost premises, and only Bad(W) holds lattice pieces (needs a vacating word).

usage: fxstop.py <run.jsonl> <blocks json> [<out json>]
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

ev = [json.loads(x) for x in Path(sys.argv[1]).read_text().splitlines() if x.strip()]
blocks = json.loads(Path(sys.argv[2]).read_text())['blocks']
name = {i: f"{len(b['pieces'])}@{min(int(p) for p in b['pieces'])}" for i, b in enumerate(blocks)}
steps = [e for e in ev if e['event'] == 'step']
passes = [e for e in ev if e['event'] == 'pass']
stop = next((e for e in ev if e['event'] == 'stop'), None)
res = {'steps': [{'step': s['step'], 'block': s['block'], 'name': name[s['block']], 'template': s['template'],
                  'm': s['m'], 'length': s['length'], 'badW': s['badW'], 'off_after': s['end']['off'],
                  'collateral': s['end']['collateral'], 'ok': s['ok']} for s in steps],
       'passes': [{'step': p['step'], 'clean_blocks': p['clean_blocks'], 'remaining': p['remaining']} for p in passes]}
if stop:
    cls = defaultdict(list)
    for k, rows in stop['remaining'].items():
        r = rows[0]
        if r['blocked']:
            c = 'blocked'
        elif r['disturbed']:
            c = 'conflict'
        elif r['T_on'] < r['T']:
            c = 'T_off'
        else:
            c = 'vacancy'
        cls[c].append({'block': int(k), 'name': name[int(k)], 'disturbed': r['disturbed'],
                       'conflict_blocks': [name.get(j, str(j)) for j in r['conflict_blocks']],
                       'badW': r['badW'], 'badW_occupied': r['badW_occupied'], 'blocked_grips': len(r['blocked'])})
    res['stop'] = {'step': stop['step'], 'restored': stop['restored'],
                   'classes': {c: sorted(v, key=lambda x: x['block']) for c, v in sorted(cls.items())},
                   'counts': {c: [len(v), sum(int(x['name'].split('@')[0]) for x in v)] for c, v in sorted(cls.items())}}
txt = json.dumps(res, indent=1)
if len(sys.argv) > 3:
    Path(sys.argv[3]).write_text(txt + '\n')
for s in res['steps']:
    print('step', s['step'], s['name'], 'block', s['block'], 't', s['template'], 'm', s['m'], 'len', s['length'],
          'badW', s['badW'], 'off', s['off_after'], 'ok', s['ok'])
for p in res['passes']:
    print('pass at', p['step'], 'clean', p['clean_blocks'], 'remaining', p['remaining'])
if stop:
    print('stop counts (blocks, pieces):', res['stop']['counts'])
    for c, v in res['stop']['classes'].items():
        print(c, [(x['name'], x['badW_occupied'] if c == 'vacancy' else x['conflict_blocks'][:3]) for x in v])
