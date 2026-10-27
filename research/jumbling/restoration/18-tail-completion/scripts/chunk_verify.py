"""Stage 18: check a chunked twist-by-twist replay (chunk_prep.py, chunk_run.py). The manifest must match the record
(its start and end digests are the step's in steps.json, and the chunk words concatenate to the step's full word,
rebuilt from the step's files). Every chunk must have exit 0, the manifest's word hash for the word handed to stepchk
and for its word file, and a report whose start digest is the chunk's start digest and whose end digest is the next
chunk's start digest (the last: the recorded end), with its letter count; the end tables must carry the reported
digests. Then the strict' contract of the whole step is computed between the start table and the last chunk's
twist-by-twist end table, each checked against its digest (and, in the four-segment form, of the step without V'^-1
at its mark, and that V'^-1 takes no piece off the lattice). One JSON line per manifest, then {"all_ok": ...}; exit 0
only when every manifest passes. Research only, headless; exact geometry.
Usage (from the repository root): python3 -I -B <this script> <manifest dir> ..."""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import chunk_prep as C  # noqa: E402

A = C.A


def state(path):
    B, screen = A.modules()
    st, M, _ = screen.load('table', str(path))
    off = set(B.off_pieces(st).tolist())
    return (st.digest(), off, {p: st._poses[int(st.pose_id[p])].key for p in off}), M


def strict(s0, s1, unit):
    (_, off0, pose0), (_, off1, pose1) = s0, s1
    others = off0 - unit
    kept = sum(p in off1 and pose1[p] == pose0[p] for p in others)
    landed = len(others - off1)
    row = dict(unit_size=len(unit), unit_on_lattice=len(unit - off1), collateral=len(off1 - off0),
               others=len(others), others_exact=kept, others_landed=landed, off_before=len(off0), off_after=len(off1))
    row['strict_prime'] = (row['unit_on_lattice'] == len(unit) and row['collateral'] == 0
                           and kept + landed == len(others))
    return row


def check(d):
    m = json.loads((d / 'manifest.json').read_text())
    step = Path(m['step_dir'])
    unit = set(json.loads((step / 'unit.json').read_text())['pieces'])
    rec = next(r for r in json.loads((step.parent / 'steps.json').read_text())['steps'] if r['dir'] == step.name)
    s0, M = state(m['start_table'])
    full = [x for _, w in C.spans_of(step, m['form'], M) for x in w]
    problems, cpu, letters, words = [], 0.0, 0, []
    if rec['start_digest'] != m['start_digest'] or rec['end_digest'] != m['end_digest']:
        problems.append('the manifest is not the recorded step')
    if s0[0] != m['start_digest'] or m['chunks'][0]['start_digest'] != m['start_digest']:
        problems.append('the start table is not the manifest start')
    chunks = m['chunks']
    for k, c in enumerate(chunks):
        j = c['j']
        try:
            done = json.loads((d / f'done{j}.json').read_text())
            r = json.loads((d / f'r{j}.json').read_text())
            w = json.loads((d / c['word']).read_text())['word']
        except OSError:
            problems.append(f'chunk {j} not done')
            continue
        words += w
        cpu += done['cpu_s']
        letters += r['letters']
        nxt = chunks[k + 1]['start_digest'] if k + 1 < len(chunks) else m['end_digest']
        if (done['rc'] != 0 or done.get('word_sha256') != c['word_sha256'] or C.sha(w) != c['word_sha256']
                or r['start_digest'] != c['start_digest'] or r['end_digest'] != nxt or r['letters'] != c['letters']
                or len(w) != c['letters'] or r.get('end_table_digest') != r['end_digest']):
            problems.append(f'chunk {j} differs')
    if not problems and (C.sha(words) != m['word_sha256'] or C.sha(full) != m['word_sha256']):
        problems.append('the chunk words are not the step word')
    row = {'dir': d.name, 'step': step.name, 'chunks': len(chunks), 'letters': letters,
           'expected_letters': len(full), 'start_digest': m['start_digest'], 'end_digest': m['end_digest'],
           'word_sha256': m['word_sha256'], 'cpu_s': round(cpu), 'problems': problems}
    if problems or letters != len(full):
        row['ok'] = False
        return row
    s1, _ = state(d / f'e{chunks[-1]["j"]}.json')
    row['end_check'] = s1[0] == m['end_digest']
    row['whole'] = strict(s0, s1, unit)
    ok = row['end_check'] and row['whole']['strict_prime']
    if 'short_end' in m['marks']:
        mark = m['marks']['short_end']
        j = next(c['j'] for c in chunks if c['to'] == mark['at'])
        sm, _ = state(d / f'e{j}.json')
        r = json.loads((d / f'r{j}.json').read_text())
        row['short'] = dict(strict(s0, sm, unit), end_digest=sm[0],
                            digest_check=sm[0] == mark['digest'] == r['end_digest'])
        row['bridge'] = dict(strict(sm, s1, set()), start_digest=sm[0], end_digest=s1[0])
        ok = (ok and row['short']['digest_check'] and row['short']['strict_prime']
              and row['bridge']['strict_prime'])
    row['ok'] = ok
    return row


def main(argv):
    ok_all = True
    for d in [Path(a) for a in argv]:
        row = check(d)
        ok_all &= row['ok']
        print(json.dumps(row), flush=True)
    print(json.dumps({'all_ok': ok_all}))
    return 0 if ok_all else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
