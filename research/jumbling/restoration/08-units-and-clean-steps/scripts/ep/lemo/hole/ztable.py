"""Table of the Theorem Z run at 233 from z1-<unit>.json and .full.jsonl."""
import json
import sys
from pathlib import Path

D = Path(sys.argv[1])
rows = []
for u in ['1040', '23955', '64184']:
    d = json.loads((D / f'z1-{u}.json').read_text())
    full = [json.loads(x) for x in open(D / f'z1-{u}.full.jsonl') if x.strip()]
    h = dict((int(k), v) for k, v in d['w1_excess_histogram'])
    lens = {k[len('root_len_'):]: v for k, v in d.items() if k.startswith('root_len_')}
    rows.append({'unit': u, 'roots_listed': d['roots_this_part'], 'roots_from_files': d['roots_from_files'],
                 'root_landing': d.get('root_landing', 0), 'root_lens': lens,
                 'root_type29_free': d.get('root_excess0', 0), 'root_dead': d.get('root_closed_by_dead', 0),
                 'w1_scored': d.get('refinements_scored', 0), 'w1_landing': d.get('refinements_landing', 0),
                 'w1_type29_free': h.get(0, 0), 'w1_min_type29': min(h) if h else None,
                 'w1_dead': d.get('w1_closed_by_dead', 0), 'w1_open': d.get('w1_open', 0),
                 'full_rows': len(full), 'full_deficit0': sum(1 for r in full if r['deficit'] == 0),
                 'seconds': d['seconds']})
print(json.dumps(rows, indent=1))
