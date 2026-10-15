"""Collect the rigid-unit results (orbit 5) into one summary JSON and a Markdown table for PROOFS.md.
usage: build_units.py <b5 dir> <out json> <out md>"""
import json, re, sys
from collections import Counter
from pathlib import Path
B = Path(sys.argv[1])


def offcount(cap):
    if cap.suffix == '.json' and cap.exists():
        return json.load(open(cap))['off']
    return int(re.match(r'\{"off": (\d+)', open(cap).readline()).group(1))  # log line is truncated JSON


def row(label, res, cap, script, word=None):
    d = json.load(open(res))
    g = d['groups']
    return {'label': label, 'word': word, 'result_file': res.name, 'script': script, 'table_digest': d['table_digest'],
            'off_pieces': offcount(cap), 'orbit': d['orbit'], 'partition': d.get('partition', 'signature classes'),
            'splits': d.get('splits', 0), 'parts_with_orbit5_member': d['groups_with_wanted'],
            'all_self_locked': d['all_self_locked'], 'all_stabilizers_even': d['all_stabilizers_even'],
            'n_values': dict(Counter(x['n_wanted'] for x in g)), 'all_n_even': all(x['n_wanted'] % 2 == 0 for x in g),
            'max_stabilizer_order': max([x['stabilizer_order'] for x in g], default=0),
            'holds': d['all_self_locked'] and d['all_stabilizers_even'] and all(x['n_wanted'] % 2 == 0 for x in g)}


rows = [row('(0,12)', B / 'units5-u12.json', B / 'capsets-u12.json', 'units5.py', '0:12')]
for w in ['56-12', '56-20', '56-40', '0-13']:
    rows.append(row('(0,12)(%s)' % w.replace('-', ','), B / ('u5-0-12_%s.json' % w), B / ('c-0-12_%s.json' % w), 'units5.py',
                    '0:12,' + w.replace('-', ':')))
for w in ['26-12', '26-30', '100-12']:
    rows.append(row('(0,12)(%s)' % w.replace('-', ','), B / ('ub5-0-12_%s.json' % w), B / ('c-0-12_%s.json' % w), 'units5b.py',
                    '0:12,' + w.replace('-', ':')))
sample = json.load(open(B / 'sample2.json'))['words']
srows = []
for w in sample:
    t = w.replace(':', '-').replace(',', '_')
    srows.append(row('sample ' + w, B / 's2' / ('uc5-%s.json' % t), B / 's2' / ('c-%s.log' % t), 'units5c.py', w))
for lab in ['e2run24', 'stop62']:
    rows.append(row(lab, B / 'fz' / ('ub5-%s.json' % lab), B / 'fz' / ('c-%s.json' % lab), 'units5b.py'))
rows.append(row('I_a@40 fixture', B / 'fz' / 'uc5-fixture40.json', B / 'fz' / 'c-fixture40.json', 'units5c.py'))
for lab in ['X', 'E1', 'E2', 'I1', 'I2']:
    rows.append(row(lab, B / 'fz' / ('ub5-%s.json' % lab), B / 'fz' / ('c-%s.json' % lab), 'units5b.py'))
allrows = rows[:8] + srows + rows[8:]
out = {'orbit': 5, 'rows': allrows, 'all_hold': all(r['holds'] for r in allrows),
       'two_jumble_bases': 7 + len(srows), 'sample_size': len(srows)}
json.dump(out, open(sys.argv[2], 'w'), indent=1)


def fmt(n):
    return f'{n:,}'


md = ['| Base | Off pieces | Parts with an orbit-5 member | Splits | n | Largest stabilizer |', '|---|---|---|---|---|---|']
for r in rows[:8]:
    md.append(f"| {r['label']}{' (covers every single jumble)' if r['label'] == '(0,12)' else ''} | {fmt(r['off_pieces'])} | "
              f"{r['parts_with_orbit5_member']} | {r['splits']} | {', '.join(str(k) for k in sorted(r['n_values']))} | {r['max_stabilizer_order']} |")
lo = lambda k: min(r[k] for r in srows)
hi = lambda k: max(r[k] for r in srows)
ns = sorted({k for r in srows for k in r['n_values']})
md.append(f"| {len(srows)} random second jumbles after (0,12) | {fmt(lo('off_pieces'))} to {fmt(hi('off_pieces'))} | "
          f"{lo('parts_with_orbit5_member')} to {hi('parts_with_orbit5_member')} | {lo('splits')} to {hi('splits')} | "
          f"{', '.join(map(str, ns))} | {hi('max_stabilizer_order')} |")
for r in rows[8:11]:
    md.append(f"| {r['label']} | {fmt(r['off_pieces'])} | {r['parts_with_orbit5_member']} | {r['splits']} | "
              f"{', '.join(str(k) for k in sorted(r['n_values']))} | {r['max_stabilizer_order']} |")
fr = rows[11:]
md.append(f"| X, E1, E2, I1, I2 | {', '.join(fmt(r['off_pieces']) for r in fr)} | 0 | 0 | none | none |")
open(sys.argv[3], 'w').write('\n'.join(md) + '\n')
print('all hold', out['all_hold'], 'two-jumble bases', out['two_jumble_bases'])
print('\n'.join(md))
