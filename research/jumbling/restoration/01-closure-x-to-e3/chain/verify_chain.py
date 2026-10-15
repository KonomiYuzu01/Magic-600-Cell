"""Check the X -> E1 -> E2 -> E3 bundle with the Python standard library only (no simulator, no NumPy).

Usage, from the bundle root:
    python -I verify_chain.py [--out report.json]

Exit status 0 when every check passes, 1 otherwise. It reads the three package zips in place and extracts nothing.

Checks:
1. Each zip under packages/ has the sha256 that chain.json records.
2. Inside each zip, every SHA256SUMS entry matches its file, and no file is missing or unlisted.
3. Links: the end table of each segment and the start table of the next are byte-identical.
4. Every pose table's digest is recomputed here from its content (canonical exact matrices, identity skipped,
   sha256 over sorted pairs of matrix JSON and little-endian int32 member lists). It must equal the table's digest
   field and the digests that the word files, part files, claims and replay reports name, along X -> E1 -> E2 -> E3.
5. Each segment's word is its five parts concatenated, and the parts chain by digest.
6. Each replay report passed: every twist admissible, depth tested at every prefix with no failure, every claim met.
7. Every value that chain.json records equals the value read here.

Not checked here: the replay itself, which needs the exact simulator. The commands are in section 复核 of each
package's README. Off counts are the simulator's, as recorded in the replay reports.
"""
import argparse
import hashlib
import json
import struct
import sys
import zipfile
from math import gcd
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PARTS = ['V', 'P', 'Q', 'Pinv', 'Vinv']


def sha(b):
    return hashlib.sha256(b).hexdigest()


def norm_entry(e):
    if not (isinstance(e, list) and len(e) == 3 and all(type(x) is int for x in e)):
        raise ValueError(f'bad matrix entry {e!r}')
    a, b, d = e
    if d == 0:
        raise ValueError('zero denominator')
    if d < 0:
        a, b, d = -a, -b, -d
    g = gcd(gcd(a, b), d)
    if g > 1:
        a, b, d = a // g, b // g, d // g
    return a, b, d


def canon(m):
    if not (isinstance(m, list) and len(m) == 4 and all(isinstance(r, list) and len(r) == 4 for r in m)):
        raise ValueError('matrix must be 4 x 4')
    return tuple(tuple(norm_entry(e)) for r in m for e in r)


IDENTITY = tuple((1, 0, 1) if i == j else (0, 0, 1) for i in range(4) for j in range(4))


def table_digest(doc):
    poses, pose_id = doc['poses'], doc['pose_id']
    keys = [canon(m) for m in poses]
    groups = {}
    for piece, i in enumerate(pose_id):
        if type(i) is not int or not 0 <= i < len(keys):
            raise ValueError(f'bad pose id {i!r} for piece {piece}')
        groups.setdefault(keys[i], []).append(piece)
    items = []
    for k, members in groups.items():
        if k == IDENTITY:
            continue
        mj = json.dumps([[list(k[4 * i + j]) for j in range(4)] for i in range(4)])
        items.append((mj, struct.pack(f'<{len(members)}i', *sorted(members))))
    items.sort()
    h = hashlib.sha256()
    for mj, b in items:
        h.update(mj.encode())
        h.update(b)
    return h.hexdigest()


class Package:
    def __init__(self, path):
        self.zip = zipfile.ZipFile(path)
        names = [n for n in self.zip.namelist() if not n.endswith('/')]
        tops = {n.split('/', 1)[0] for n in names}
        if len(tops) != 1:
            raise ValueError(f'{path.name}: expected one top directory, found {sorted(tops)}')
        self.top = tops.pop()
        self.files = {n.split('/', 1)[1] for n in names}

    def read(self, rel):
        return self.zip.read(f'{self.top}/{rel}')

    def json(self, rel):
        return json.loads(self.read(rel))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--out', help='also write the report to this file')
    a = ap.parse_args()
    chain = json.loads((ROOT / 'chain.json').read_text())
    checks = []

    def check(name, ok, **detail):
        checks.append({'check': name, 'ok': bool(ok), **detail})
        return ok

    digests = {}
    tables = {}
    for seg in chain['segments']:
        zpath = ROOT / 'packages' / seg['zip']
        zbytes = zpath.read_bytes()
        check(f"{seg['name']}: zip sha256", sha(zbytes) == seg['zip_sha256'], zip=seg['zip'], sha256=sha(zbytes))
        pk = Package(zpath)
        sums = {}
        for line in pk.read('SHA256SUMS').decode().splitlines():
            if line.strip():
                # sha256sum format: 64 hex digits, a space, ' ' (text) or '*' (binary), the path; './' is optional.
                if len(line) < 67 or line[64] != ' ' or line[65] not in ' *':
                    raise ValueError(f'{seg["zip"]}: bad SHA256SUMS line {line!r}')
                h, rel = line[:64], line[66:]
                sums[rel[2:] if rel.startswith('./') else rel] = h
        bad = sorted(rel for rel, h in sums.items() if rel not in pk.files or sha(pk.read(rel)) != h)
        unlisted = sorted(pk.files - set(sums) - {'SHA256SUMS'})
        check(f"{seg['name']}: SHA256SUMS", not bad and not unlisted, entries=len(sums), mismatched=bad[:20],
              unlisted=unlisted[:20])

        # tables and their content digests
        for role in ('start', 'end'):
            rel = seg[f'{role}_table']
            raw = pk.read(rel)
            doc = json.loads(raw)
            dig = table_digest(doc)
            tables[(seg['name'], role)] = sha(raw)
            check(f"{seg['name']}: {role} table digest recomputed", dig == doc.get('digest') == seg[f'{role}_digest'],
                  table=rel, recomputed=dig, field=doc.get('digest'), recorded=seg[f'{role}_digest'])
            check(f"{seg['name']}: {role} table sha256", sha(raw) == seg[f'{role}_table_sha256'], table=rel)
            digests[(seg['name'], role)] = dig

        # word, parts, claims, replay
        d = seg['dir']
        wf = pk.json(f'{d}/word-full.json')
        word = wf['word']
        check(f"{seg['name']}: word digests", wf['input_digest'] == seg['start_digest']
              and wf['output_digest'] == seg['end_digest'])
        check(f"{seg['name']}: word length", len(word) == seg['twists'], twists=len(word))
        parts = [pk.json(f'{d}/parts/{p}.json') for p in PARTS]
        cat = [t for p in parts for t in p['word']]
        linked = (parts[0]['input_digest'] == seg['start_digest'] and parts[-1]['output_digest'] == seg['end_digest']
                  and all(parts[i]['output_digest'] == parts[i + 1]['input_digest'] for i in range(len(parts) - 1)))
        check(f"{seg['name']}: parts concatenate to the word and chain by digest", cat == word and linked,
              part_lengths=[len(p['word']) for p in parts])
        check(f"{seg['name']}: part lengths as recorded", [len(p['word']) for p in parts] == seg['part_twists'])
        cl = pk.json(f'{d}/claims.json')
        rp = pk.json(f'{d}/replay.json')
        tot = rp['total']
        claims_ok = all(v.get('ok') is True for v in rp['claims'].values())
        check(f"{seg['name']}: replay report passed", rp.get('pass') is True and claims_ok
              and tot['inadmissible'] is None and tot['twists'] == len(word)
              and tot['depth_tested'] == len(word) and tot['depth_failures'] == 0
              and tot['input_digest_ok'] is True and tot['output_digest_ok'] is True,
              twists=tot['twists'], depth_tested=tot['depth_tested'], depth_failures=tot['depth_failures'])
        check(f"{seg['name']}: replay and claims digests", rp['start_digest'] == seg['start_digest']
              and tot['end_digest'] == cl['end_digest'] == seg['end_digest'])
        rec = {'start_off': rp['start_off'], 'end_off': tot['off_end'], 'restored': tot['restored'],
               'collateral_end': tot['collateral'], 'off_max': tot['off_max']}
        check(f"{seg['name']}: counts as recorded", all(seg[k] == v for k, v in rec.items()), **rec)
        check(f"{seg['name']}: claims as recorded", cl['restored'] == seg['restored']
              and cl['collateral'] == seg['collateral_end'] and cl['off_at_end'] == seg['end_off'])

    segs = chain['segments']
    for s, t in zip(segs, segs[1:]):
        check(f"link {s['name']} -> {t['name']}: same table bytes", tables[(s['name'], 'end')]
              == tables[(t['name'], 'start')] == chain_link(chain, s['name'], t['name']),
              sha256=tables[(s['name'], 'end')])
        check(f"link {s['name']} -> {t['name']}: same digest", digests[(s['name'], 'end')]
              == digests[(t['name'], 'start')])
    check('total twists as recorded', sum(s['twists'] for s in segs) == chain['total_twists'],
          total=sum(s['twists'] for s in segs))
    check('off along the chain as recorded', [segs[0]['start_off']] + [s['end_off'] for s in segs]
          == chain['off_along_chain'])

    ok = all(c['ok'] for c in checks)
    report = {'ok': ok, 'checks': len(checks), 'failed': [c for c in checks if not c['ok']], 'all': checks}
    text = json.dumps(report, indent=1, ensure_ascii=False)
    if a.out:
        Path(a.out).write_text(text + '\n')
    print(json.dumps({'ok': ok, 'checks': len(checks), 'failed': len(report['failed'])}))
    for c in report['failed']:
        print(json.dumps(c, ensure_ascii=False))
    return 0 if ok else 1


def chain_link(chain, a, b):
    for link in chain['links']:
        if link['from'] == a and link['to'] == b:
            return link['table_sha256']
    return None


if __name__ == '__main__':
    sys.exit(main())
