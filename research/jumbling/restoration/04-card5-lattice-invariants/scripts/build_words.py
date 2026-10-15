"""Collect every generator word into one jsonl for verify_words.py (name, source, letters or records, claimed nu).
usage: build_words.py <out jsonl> <library json> <handoff work-output dir> <jsonl:label ...>"""
import json, sys
out = open(sys.argv[1], 'w')
lib = json.load(open(sys.argv[2]))
for m in lib['macros']:
    out.write(json.dumps({'name': f"M{m['id']}", 'source': f"lattice-correction library v1, family {m['family']}",
                          'records': m['records'], 'claimed': m['nu']}) + '\n')
W = sys.argv[3]
fr = json.load(open(W + '/C5-VACANCY-FRAME-RETURN.json'))
for k, row in enumerate(fr['rows']):
    out.write(json.dumps({'name': f'WA{k}', 'source': 'handoff accepted fact d0c9bda0a984e204 (C5-VACANCY-FRAME-RETURN.json rows[%d])' % k,
                          'letters': row['word'], 'claimed': {'32': [0, 1]}}) + '\n')
th = json.load(open(W + '/C5-TWO-HOLE-SIGN-RETURN.json'))
out.write(json.dumps({'name': 'Wstar', 'source': 'handoff unreviewed candidate (C5-TWO-HOLE-SIGN-RETURN.json), replayed here',
                      'letters': th['word'], 'claimed': {'32': [1, 0]}}) + '\n')
for arg in sys.argv[4:]:
    path, label = arg.rsplit(':', 1)
    for k, line in enumerate(open(path)):
        if not line.strip():
            continue
        r = json.loads(line)
        out.write(json.dumps({'name': f'{label}{k}', 'source': f'{label} (this work)', 'letters': r['word'],
                              'claimed': r['nu']}) + '\n')
out.close()
