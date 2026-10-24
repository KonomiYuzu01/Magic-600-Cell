"""Compare two runner records step by step: rule, letters, word sha256, before and after digest, N.

Usage: python3 -I -B compare_runs.py <run 1 run.json> <run 2 run.json>
"""
import json
import sys


def key(step):
    return (step['phase'], step['subphase'], step['rule'], step['letters'], step['word_sha256'],
            step['before']['digest'], step['after']['digest'], step['before'].get('N'), step['after'].get('N'))


def main(first, second):
    a = json.load(open(first))['steps']
    b = json.load(open(second))['steps']
    n = min(len(a), len(b))
    diff = next((i for i in range(n) if key(a[i]) != key(b[i])), None)
    print(f'run 1 steps: {len(a)}; run 2 steps: {len(b)}; compared: {n}; first difference at index: {diff}')
    if diff is not None:
        print('run 1:', key(a[diff]))
        print('run 2:', key(b[diff]))
    secs = [(a[i]['seconds'], b[i]['seconds']) for i in range(n)]
    if secs:
        print(f'seconds over the compared steps: run 1 {sum(x for x, _ in secs):.0f}, run 2 {sum(y for _, y in secs):.0f}')
    return 1 if diff is not None else 0


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:]))
