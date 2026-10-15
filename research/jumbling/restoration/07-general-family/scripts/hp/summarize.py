"""Summary of heap-peeling runs: per run the start off count, steps by kind, collateral, end state and replay result.

usage: summarize.py <out json> <run dir>[:<verify jsonl>] ...
"""
import json
import sys
from pathlib import Path

rows = []
for arg in sys.argv[2:]:
    run, _, ver = arg.partition(':')
    run = Path(run)
    steps = [json.loads(x) for x in (run / 'run.jsonl').read_text().splitlines()]
    end = None
    err = run.parent / (run.name + '.err')
    if err.exists():
        for x in err.read_text().splitlines():
            if x.startswith('{') and ('"end"' in x or '"stuck' in x):
                e = json.loads(x)
                end = e if end is None or e['event'] == 'end' else end
                if e['event'].startswith('stuck'):
                    end = e
    r = {'run': run.name, 'start_off': steps[0]['off_before'] if steps else None, 'steps': len(steps),
         'PS': sum(s['kind'] == 'PS' for s in steps), 'PSC': sum(s['kind'] == 'PSC' for s in steps),
         'collateral_total': sum(s['collateral'] for s in steps),
         'collateral_max': max((s['collateral'] for s in steps), default=0),
         'moved_total': sum(s['moved_off'] for s in steps),
         'gain_min': min((s['gain'] for s in steps), default=None),
         'end_off': steps[-1]['off_after'] if steps else None, 'all_steps_ok': all(s['ok'] for s in steps),
         'end_event': None if end is None else {k: v for k, v in end.items() if k in ('event', 'off', 'best', 'sec')}}
    if ver:
        v = [json.loads(x) for x in Path(ver).read_text().splitlines()]
        r['replay'] = {k: v[-1].get(k) for k in ('all_ok', 'steps', 'final_off', 'all_on_lattice', 'final_digest')}
        r['replay']['start_digest_ok'] = v[0].get('start_digest_ok')
    rows.append(r)
    print(json.dumps(r))
Path(sys.argv[1]).write_text(json.dumps(rows, indent=1) + '\n')
