"""Compare the runner's bulk steps with a census record, step by step.

Usage: python3 -I -B compare.py <run dir> <census record> <markdown out>
"""
import gzip
import json
import sys
from pathlib import Path


def read(path):
    opener = gzip.open if str(path).endswith('.gz') else open
    with opener(path, 'rt') as f:
        return json.load(f)


def main(run_dir, census_file, out):
    run_dir = Path(run_dir)
    run = read(run_dir / 'run.json')
    census = read(census_file)
    bulk = [s for s in run['steps'] if s['phase'] == 'bulk']
    rows, pos, first_diff, matched_twists = [], 0, None, 0
    for i in range(min(len(bulk), len(census['steps']))):
        r, c = bulk[i], census['steps'][i]
        word = [list(x) for x in read(run_dir / r['word'])['word']]
        cw = [list(x) for x in census['moves'][pos:pos + c['twists']]]
        pos += c['twists']
        got = (r['rule'], r['letters'], r['before']['N'], r['after']['N'], r['after']['off'])
        want = (c['rule'], c['twists'], c['N_before'], c['N_after'], c['off_after'])
        same = got == want and word == cw
        if same:
            matched_twists += len(word)
        elif first_diff is None:
            first_diff = i
        rows.append(f"| {i} | {got[0]} / {want[0]} | {got[1]} / {want[1]} | {got[2]} / {want[2]} | "
                    f"{got[3]} / {want[3]} | {got[4]} / {want[4]} | {'yes' if word == cw else 'no'} | "
                    f"{r['seconds']:.1f} / {c['sec']} | {'match' if same else 'DIFFERENT'} |")
    head = [f"runner bulk steps: {len(bulk)}; census steps: {len(census['steps'])}; compared: {len(rows)}",
            f"first difference at index: {first_diff}; twists in matching steps: {matched_twists}", '',
            '| index | rule (runner / census) | twists | N before | N after | off after | same word '
            '| seconds (runner step / census cumulative) | |', '|---|---|---|---|---|---|---|---|---|']
    Path(out).write_text('\n'.join(head + rows) + '\n')
    print('\n'.join(head[:2]))


if __name__ == '__main__':
    main(*sys.argv[1:])
