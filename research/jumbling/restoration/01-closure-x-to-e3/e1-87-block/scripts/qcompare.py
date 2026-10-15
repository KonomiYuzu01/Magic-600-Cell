"""Compare two dcomm2.py runs for the same block at two states (here X and E1): candidate sets, |Bad(Q)| on the common
candidates, whether a given Q is present, and the grips used by the candidates found only in the second run.

Usage: python -I qcompare.py <run A jsonl> <run B jsonl> <W json with 'Q'> <resources A json> <resources B json> <out>
"""
import json
import sys
from pathlib import Path


def main():
    a = sys.argv[1:]
    ra = {json.dumps(json.loads(l)['Q']): json.loads(l)['bad'] for l in open(a[0])}
    rb = {json.dumps(json.loads(l)['Q']): json.loads(l)['bad'] for l in open(a[1])}
    q = json.dumps(json.loads(Path(a[2]).read_text())['Q'])
    sa = set(json.loads(Path(a[3]).read_text())['straddling_grips'])
    sb = set(json.loads(Path(a[4]).read_text())['straddling_grips'])
    extra = [k for k in rb if k not in ra]
    grips = set()
    for k in extra:
        grips |= {c for c, _ in json.loads(k)}
    res = {'A_candidates': len(ra), 'B_candidates': len(rb), 'A_subset_of_B': all(k in rb for k in ra),
           'bad_equal_on_A': all(rb.get(k) == v for k, v in ra.items()), 'only_in_B': len(extra),
           'only_in_B_grips_straddled_at_A': sorted(grips & sa),
           'only_in_B_all_use_one_of_them': all({c for c, _ in json.loads(k)} & sa for k in extra),
           'straddled_at_A': sorted(sa), 'straddled_at_B': sorted(sb),
           'Q_in_A': q in ra, 'Q_in_B': q in rb, 'min_bad_A': min(ra.values()), 'min_bad_B': min(rb.values()),
           'min_bad_only_in_B': min(rb[k] for k in extra) if extra else None}
    Path(a[5]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
