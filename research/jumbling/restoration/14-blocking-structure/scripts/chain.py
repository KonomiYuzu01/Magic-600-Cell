"""Greedy chaining of T3P rounds (Corollary I) with the committed tools only.

At each state: t3p.scan; take its best round (largest predicted descent; the scan's tie-break); check it with
t3p.check, which tests P0 to P4 and replays the round exactly (moved sets, each ΔΦ, predicted Φ); repeat until the
scan finds no round. Run from the repository root:

    python3 -I -B chain.py <repo> <out_dir> <name>:<table> [<name>:<table> ...] [--rounds N]
"""
import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

for _n in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_n] = '1'
args = sys.argv[1:]
max_rounds = 40
if '--rounds' in args:
    k = args.index('--rounds')
    max_rounds = int(args[k + 1])
    del args[k:k + 2]
root, out, specs = Path(args[0]), Path(args[1]), args[2:]
sys.path.insert(0, str(root / 'research/jumbling/restoration/14-blocking-structure/scripts'))
import jamcheck as J  # noqa: E402
import t3p as P  # noqa: E402


def statuses(scan):
    return dict(Counter(row['status'] for row in scan['rows']))


for spec in specs:
    name, table = spec.split(':', 1)
    t0 = time.process_time()
    state, _ = P.load_input(table, '', str(out / f'{name}.npz'))
    start_digest, start_phi = state.st.digest(), J.phi(state.h)
    start_n4 = int((state.h == 4).sum())
    end = (start_digest, start_phi, start_n4)
    rounds, word, stop = [], [], None
    for k in range(1, max_rounds + 1):
        scan = P.scan(state)
        cert = scan['best_certificate']
        if cert is None:
            stop = dict(reason='no round', digest=scan['digest'], phi=scan['phi'], statuses=statuses(scan),
                        single_letter_descents=len(scan['single_letter_descents']))
            break
        spec_round = cert['rounds'][0]
        result = P.check(state, cert)
        rd = result['rounds'][0] if result['rounds'] else {}
        row = dict(round=k, x=spec_round['x'], unit_piece=spec_round['unit_piece'], h=spec_round['h'],
                   helpers=spec_round['helpers'], order=rd.get('order'), S=rd.get('S'), F=rd.get('F'),
                   D=rd.get('D'), predicted_phi=rd.get('predicted_phi'), passed=result['passed'],
                   failures=result['failures'], statuses_before=statuses(scan),
                   phi_after=J.phi(state.h), digest_after=state.st.digest())
        rounds.append(row)
        print(json.dumps(dict(name=name, round=k, x=row['x'], F=row['F'], D=row['D'], passed=row['passed'],
                              phi=row['phi_after'])), flush=True)
        if not result['passed']:
            # A check can fail after part of its replay, so the end fields stay at the last passed round.
            stop = dict(reason='check failed', failures=result['failures'])
            break
        items = dict((c, i) for c, i in spec_round['helpers'])
        word += [[c, items[c]] for c in rd['order']] + [[spec_round['x'], spec_round['h']]]
        end = (state.st.digest(), J.phi(state.h), int((state.h == 4).sum()))
    else:
        stop = dict(reason='round limit', limit=max_rounds)
    doc = dict(format='t3p-chain/1', name=name, table=table, start_digest=start_digest, start_phi=start_phi,
               start_n4=start_n4, end_digest=end[0], end_phi=end[1], end_n4=end[2], rounds=rounds, word=word,
               stop=stop,
               policy='greedy: the best round of t3p.scan at each state (its tie-break), checked by t3p.check',
               cpu_s=round(time.process_time() - t0, 3))
    P.write_json(out / f'{name}-chain.json', doc)
    print(json.dumps(dict(name=name, done=True, start_phi=start_phi, end_phi=doc['end_phi'], rounds=len(rounds),
                          stop=stop.get('reason'))), flush=True)
