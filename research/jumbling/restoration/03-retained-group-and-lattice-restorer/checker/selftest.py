"""Tamper tests: the certificate checker must reject each altered certificate file.

usage: python3 -I selftest.py <repo root> <certificates.json> <work dir>"""
import copy
import json
import subprocess
import sys
from pathlib import Path

root, cert_path, work = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
work.mkdir(parents=True, exist_ok=True)
checker = Path(__file__).resolve().parent / 'check_certs.py'
base = json.loads(cert_path.read_text())


def tamper_seed(d):
    # drop the last move of the seed word of orbit 4 (the word under its c_o)
    n = d['nodes'][d['P'][4]['c_node']]
    x = n['x']
    while d['nodes'][x]['op'] != 'word':
        x = d['nodes'][x]['x'] if d['nodes'][x]['op'] == 'conj' else d['nodes'][x]['f'][0][0]
    d['nodes'][x]['w'] = d['nodes'][x]['w'][:-1]


def tamper_setup(d):
    s = d['P'][1]['setup_tree_nodes']
    k = sorted(s)
    s[k[0]], s[k[1]] = s[k[1]], s[k[0]]


def tamper_frame(d):
    F = next(F for F in d['F'] if F['orbit'] == 25)
    F['node'] = next(F2['node'] for F2 in d['F'] if F2['orbit'] == 32)


def tamper_missing_setup(d):
    s = d['P'][7]['setup_tree_nodes']
    del s[sorted(s)[5]]


cases = {'seed-word': tamper_seed, 'setup-swap': tamper_setup, 'frame-word': tamper_frame,
         'missing-setup': tamper_missing_setup}
ok = True
for name, fn in cases.items():
    d = copy.deepcopy(base)
    fn(d)
    p = work / ('tampered-%s.json' % name)
    p.write_text(json.dumps(d))
    r = subprocess.run([sys.executable, '-I', str(checker), root, str(p), str(work / ('report-%s.json' % name))],
                       capture_output=True, text=True)
    rejected = r.stdout.startswith('FAIL')
    ok &= rejected
    print(name, 'rejected' if rejected else 'ACCEPTED (bad)', r.stdout.strip()[:200])
print('SELFTEST', 'PASS' if ok else 'FAIL')
