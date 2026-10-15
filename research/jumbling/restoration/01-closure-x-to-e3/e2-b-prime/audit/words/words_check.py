"""Audit 'words': structure, inverse and provenance checks of the native word W3 = V3 P4 F P4^-1 V3^-1 (segment 3).

Independent of the producing scripts: reads the part files, the word files, the report text and the pose tables;
recomputes exact letter matrices (restore.element), inverses, the iterated-conjugation shape with an own mirror
inverse, the SAFE-ALL alphabets (buffer.Alphabet) at E2 and at Y4 = E2 V3 P4, and replays the word on the simulator.

Usage: python3 -I -B words_check.py <out json>
"""
import hashlib
import json
import re
import sys
import time
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
SP = Path('<work>')
TH = SP / 'wt-e3' / 'research' / 'jumbling' / 'theory'
B4 = SP / 'v26' / 'B4'
C3 = B4 / 'c3'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
from exact import identity, matmul, matvec, transpose  # noqa: E402
from sim.kplus import det4, to_tuple  # noqa: E402

O, restore, CL = BL.O, BL.restore, BL.CL
kp, data = CL.ctx.kplus, CL.ctx.data
T0 = time.time()
I4 = to_tuple(identity(4))
OUT = {}
FAIL = []


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def check(name, ok, **ev):
    OUT.setdefault('checks', []).append(dict(check=name, ok=bool(ok), **ev))
    if not ok:
        FAIL.append(name)
    log(check=name, ok=bool(ok))


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def tup(w):
    return [tuple(int(v) for v in x) for x in w]


# ---------------------------------------------------------------- own exact arithmetic (independent of exact.matmul)
def fr(x):
    return (Fraction(x.a, x.d), Fraction(x.b, x.d))


def own_mul(A, Bm):
    out = []
    for r in range(4):
        row = []
        for c in range(4):
            s0, s1 = Fraction(0), Fraction(0)
            for k in range(4):
                a0, a1 = fr(A[r][k])
                b0, b1 = fr(Bm[k][c])
                s0 += a0 * b0 + 5 * a1 * b1
                s1 += a0 * b1 + a1 * b0
            row.append((s0, s1))
        out.append(row)
    return out


def own_is_identity(P):
    return all(P[r][c] == ((Fraction(1) if r == c else Fraction(0)), Fraction(0)) for r in range(4) for c in range(4))


def main():
    outp = Path(sys.argv[1])
    # ------------------------------------------------------------------------------------------------ item 1
    names = ['V', 'P', 'Q', 'Pinv', 'Vinv']
    parts = {n: json.loads((C3 / 'parts' / f'{n}.json').read_text()) for n in names}
    words = {n: tup(parts[n]['word']) for n in names}
    lens = {n: len(words[n]) for n in names}
    OUT['lengths'] = lens
    check('1.lengths 8,31,1,31,8', [lens[n] for n in names] == [8, 31, 1, 31, 8], lengths=lens, total=sum(lens.values()))
    e2_sha, e3_sha = sha(B4 / 'E2-table.json'), sha(C3 / 'E3-table.json')
    E2doc = json.loads((B4 / 'E2-table.json').read_text())
    E3doc = json.loads((C3 / 'E3-table.json').read_text())
    E2d, E3d = E2doc['digest'], E3doc['digest']
    del E2doc, E3doc
    check('1.E2 table sha256 and digest field', e2_sha == '649576e69ede65a6203dfbebd3b879a68e40250abaf18a0f08ad86084834d893'
          and E2d == '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff', sha256=e2_sha, digest=E2d)
    check('1.E3 table sha256 and digest field', e3_sha == '53cec2e37f3b858924c216d45d99f1563f0f1e57d1d8a78fa39946c1c965d889'
          and E3d == 'fb199d9997a240c69ce91b39953d2434939f094f183f566067ca1779fe10c441', sha256=e3_sha, digest=E3d)
    check('1.V.input_digest = E2 digest', parts['V']['input_digest'] == E2d, v_in=parts['V']['input_digest'])
    chain = [(names[j], names[j + 1], parts[names[j]]['output_digest'] == parts[names[j + 1]]['input_digest'])
             for j in range(4)]
    check('1.part output = next part input', all(x[2] for x in chain), chain=chain)
    check('1.Vinv.output_digest = E3 digest', parts['Vinv']['output_digest'] == E3d, vinv_out=parts['Vinv']['output_digest'])
    wf = json.loads((C3 / 'word-full.json').read_text())
    full = [x for n in names for x in words[n]]
    check('1.word-full = V+P+Q+Pinv+Vinv, digests E2 -> E3',
          tup(wf['word']) == full and wf['input_digest'] == E2d and wf['output_digest'] == E3d,
          word_full_length=len(wf['word']), input=wf['input_digest'], output=wf['output_digest'])
    P4file = tup(json.loads((B4 / 'P4.json').read_text()))
    w3 = json.loads((B4 / 'w3-P4-162-1.json').read_text())
    check('1.P = P4.json = w3-P4-162-1.json P', words['P'] == P4file == tup(w3['P']))
    check('1.Q = [(162,1)] = w3 Q', words['Q'] == [(162, 1)] == tup(w3['Q']), w3_keys=sorted(w3))
    pre = json.loads((C3 / 'pre.json').read_text())
    check('1.pre.json W equals parts (P, Q, P_inverse)',
          tup(pre['W']['P']) == words['P'] and tup(pre['W']['Q']) == words['Q']
          and tup(pre['W']['P_inverse']) == words['Pinv'])
    res = json.loads((C3 / 'result.json').read_text())
    rows = res['parts']
    check('1.result.json part digests equal part files',
          [r['part'] for r in rows] == names and all(r['input_digest'] == parts[r['part']]['input_digest']
                                                     and r['output_digest'] == parts[r['part']]['output_digest']
                                                     and r['twists'] == lens[r['part']] for r in rows))
    typeS = json.loads((B4 / 'cS-P4-162-1' / 'word-full.json').read_text())
    check('1.type-S word starts with P4 and ends with F P4^-1 (same P)',
          tup(typeS['word'][:31]) == words['P'] and tup(typeS['word'][-32:]) == [(162, 1)] + words['Pinv'],
          typeS_length=len(typeS['word']), typeS_input=typeS['input_digest'])
    # report text: printed V3 and P4
    rep = (B4 / 'report' / 'D3-E2-E3.md').read_text()
    m = re.search(r'Written out, P4 is\n(.*?)\.\n\(91,7\) is x0', rep, re.S)
    printedP4 = [(int(a), int(b)) for a, b in re.findall(r'\((\d+),(\d+)\)', m.group(1))] if m else None
    check('3.report printed P4 = parts/P.json', printedP4 == words['P'], printed_length=None if printedP4 is None else len(printedP4))
    m = re.search(r'V3 \(8 letters, SAFE-ALL at E2, from the peeling certificate below\):\n(.*?)\.\n', rep, re.S)
    printedV = [(int(a), int(b)) for a, b in re.findall(r'\((\d+),(\d+)\)', m.group(1))] if m else None
    check('1.report printed V3 = parts/V.json', printedV == words['V'])

    # ------------------------------------------------------------------------------------------------ item 2
    R, M = O.start(str(B4 / 'E2-table.json'))
    st = R.st
    check('2.E2 table loads to its digest', st.digest() == E2d, loaded=st.digest())
    OUT['menu_items'] = len(M.items)
    letters = sorted(set(full) | {(237, 13), (237, 47)})
    E = {w: restore.element(M, *w) for w in letters}
    info = {}
    for w in letters:
        c, i = w
        e = E[w]
        rot = to_tuple(matmul(transpose(e), e)) == I4 and det4(e) == 1
        fixes = matvec(e, data.N[c]) == list(data.N[c])
        k = kp.index_of_matrix(to_tuple(e))
        inv = [j for j in range(len(M.items)) if to_tuple(matmul(e, restore.element(M, c, j))) == I4]
        inv2 = [j for j in range(len(M.items)) if to_tuple(matmul(restore.element(M, c, j), e)) == I4]
        order, p = None, e
        for n in range(1, 61):
            if to_tuple(p) == I4:
                order = n
                break
            p = matmul(p, e)
        info[f'{c},{i}'] = {'label': M.items[i][0], 'rotation': rot, 'fixes_pole_c': fixes, 'kplus_index': int(k),
                           'in_K+': k >= 0, 'identity': to_tuple(e) == I4, 'inverse_items_right': inv,
                           'inverse_items_left': inv2, 'order': order}
    OUT['letters'] = info
    check('2.every letter an exact rotation fixing its pole', all(v['rotation'] and v['fixes_pole_c'] for v in info.values()))
    check('2.every letter has exactly one inverse item (both sides agree)',
          all(len(v['inverse_items_right']) == 1 and v['inverse_items_right'] == v['inverse_items_left'] for v in info.values()))

    def own_inverse(a, b):
        return own_is_identity(own_mul(E[a], E[b])) and own_is_identity(own_mul(E[b], E[a]))
    for a, b in (((237, 13), (237, 47)), ((91, 6), (91, 7)), ((154, 7), (154, 6))):
        check(f'2.{b} is the exact inverse of {a} (own arithmetic, both orders)', own_inverse(a, b))
    inv133 = info['133,1']['inverse_items_right']
    inv183 = info['183,6']['inverse_items_right']
    OUT['inverse_of_133_1'] = inv133
    OUT['inverse_of_183_6'] = inv183
    check('2.inverse of (133,1) determined', len(inv133) == 1 and own_inverse((133, 1), (133, inv133[0])),
          inverse=[133, inv133[0]], order=info['133,1']['order'])
    check('2.inverse of (183,6) determined', len(inv183) == 1 and own_inverse((183, 6), (183, inv183[0])),
          inverse=[183, inv183[0]], order=info['183,6']['order'])
    for w in ((91, 6), (154, 7), (133, 1), (183, 6), (162, 1)):
        v = info[f'{w[0]},{w[1]}']
        check(f'2.{w} in K+ and not the identity', v['in_K+'] and not v['identity'], kplus_index=v['kplus_index'],
              order=v['order'])
    check('2.(237,13) not in K+', not info['237,13']['in_K+'], order=info['237,13']['order'])
    check('2.(237,47) not in K+', not info['237,47']['in_K+'], order=info['237,47']['order'])

    # ------------------------------------------------------------------------------------------------ item 3
    MINV = {}

    def minv(w):
        if w in MINV:
            return MINV[w]
        c, i = w
        hits = [j for j in range(len(M.items)) if own_is_identity(own_mul(restore.element(M, c, i), restore.element(M, c, j)))]
        if len(hits) != 1:
            raise AssertionError(f'{w}: {len(hits)} inverse items')
        MINV[w] = (c, hits[0])
        return MINV[w]

    def mirror(word):
        return [minv(w) for w in reversed(word)]
    xs = [(91, 6), (154, 7), (133, 1), (183, 6)]
    levels = [[(237, 13)]]
    for x in xs:
        Pk = levels[-1]
        levels.append(Pk + [x] + mirror(Pk))
    OUT['level_lengths'] = [len(p) for p in levels]
    check('3.P4 = P3 x3 P3^-1, ..., P1 = R x0 R^-1 letter by letter', levels[4] == words['P'],
          lengths=[len(p) for p in levels])
    shrink = [json.loads(line) for line in (B4 / 'shrink-237-13.jsonl').read_text().splitlines() if line.strip()]
    check('3.shrink level words P(k) equal the recomputed P0..P4',
          len(shrink) == 5 and all(tup(r['P']) == levels[r['level']] for r in shrink),
          shrink_levels=[(r['level'], r['P_length']) for r in shrink])
    check('3.shrink chosen x_k = best[0] letter at levels 0..3, F=(162,1) least F letter at level 4',
          [tuple(shrink[k]['best'][0]['letter']) for k in range(4)] == xs
          and tuple(shrink[4]['best'][0]['letter']) == (162, 1) and shrink[4]['F_letters'][0] == [162, 1]
          and min(tuple(x) for x in shrink[4]['F_letters']) == (162, 1) and len(shrink[4]['F_letters']) == 55)
    # the report's identifications in the printed P4
    check('3.report: (91,7) is x0^-1 and (154,6) is x1^-1', minv((91, 6)) == (91, 7) and minv((154, 7)) == (154, 6))
    mid = levels[3][len(levels[3]) // 2]  # middle letter of P3 = x2
    mid_inv = mirror(levels[3])[len(levels[3]) // 2]  # middle letter of P3^-1 = x2^-1
    check('3.middle letter of P3^-1 inside P4 equals x2^-1', words['P'][16 + 7] == mid_inv,
          P4_letter_23=list(words['P'][23]), x2=list(mid), x2_inverse=list(mid_inv))

    # ------------------------------------------------------------------------------------------------ item 4
    def mirror_ok(a, b):
        n = len(a)
        rows_ = []
        for j in range(n):
            x, y = a[n - 1 - j], b[j]
            rows_.append(x[0] == y[0] and own_is_identity(own_mul(restore.element(M, *x), restore.element(M, *y)))
                         and own_is_identity(own_mul(restore.element(M, *y), restore.element(M, *x))))
        return len(a) == len(b) and all(rows_)
    check('4.Pinv is the mirror inverse of P (same grip, reversed, exact product I both orders)',
          mirror_ok(words['P'], words['Pinv']))
    check('4.Vinv is the mirror inverse of V (same grip, reversed, exact product I both orders)',
          mirror_ok(words['V'], words['Vinv']))
    check('4.Pinv equals own mirror(P); Vinv equals own mirror(V)',
          words['Pinv'] == mirror(words['P']) and words['Vinv'] == mirror(words['V']))
    # whole-word letter balance: W3 as a word is V P F P^-1 V^-1; check the full mirror symmetry around F
    check('4.full word = X F mirror(X) with X = V3 P4', full == words['V'] + words['P'] + [(162, 1)]
          + mirror(words['V'] + words['P']))

    # ------------------------------------------------------------------------------------------------ item 5
    G = B.Alphabet(R, M)
    safe_pre = pre['menu']['safe_grips']
    blocked = sorted(G.blocked)
    OUT['E2_alphabet'] = {'safe': len(G.safe), 'letters': len(G.letters), 'blocked': blocked,
                          'blocked_detail': {str(k): v for k, v in G.blocked.items()}}
    check('5.E2 SAFE-ALL alphabet rebuilt = pre.json safe grips (592 grips, 6512 letters)',
          list(map(int, G.safe)) == safe_pre and len(G.safe) == 592 and len(G.letters) == 6512,
          safe=len(G.safe), letters=len(G.letters), blocked=blocked)
    check('5.blocked grips at E2 = report list (232, 237, 250, 257, 273, 301, 307, 356)',
          blocked == [232, 237, 250, 257, 273, 301, 307, 356], blocked=blocked)
    vrows = []
    for w in words['V']:
        v = info[f'{w[0]},{w[1]}']
        vrows.append({'letter': list(w), 'grip_in_pre_safe': w[0] in safe_pre, 'letter_in_alphabet': w in G.index,
                      'in_K+': v['in_K+'], 'identity': v['identity']})
    OUT['V_letters'] = vrows
    check('5.every V letter: grip SAFE-ALL at E2 (pre.json), letter of the E2 alphabet, in K+, not identity',
          all(r['grip_in_pre_safe'] and r['letter_in_alphabet'] and r['in_K+'] and not r['identity'] for r in vrows))
    check('5.every Vinv letter is a letter of the E2 alphabet', all(w in G.index for w in words['Vinv']))
    del G
    # replay V, P; Alphabet at Y4 = E2 V3 P4
    dig = {'start': st.digest()}
    for w in words['V']:
        R.apply(*w)
    dig['V'] = st.digest()
    for w in words['P']:
        R.apply(*w)
    dig['P'] = st.digest()
    check('5.replay: V and P admissible, digests equal part files',
          dig['V'] == parts['V']['output_digest'] and dig['P'] == parts['P']['output_digest'], digests=dict(dig))
    G4 = B.Alphabet(R, M)
    check('5.F=(162,1) is a SAFE-ALL letter at Y4 = E2 V3 P4', (162, 1) in G4.index and 162 in G4.safe,
          safe=len(G4.safe), letters=len(G4.letters), blocked=sorted(G4.blocked),
          grip162=G4.blocked.get(162, 'safe'))
    off_before = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    poses_before = {int(p): to_tuple(st.pose(int(p))) for p in off_before}
    R.apply(162, 1)
    dig['Q'] = st.digest()
    off_after = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    same = np.array_equal(off_before, off_after) and all(to_tuple(st.pose(p)) == m for p, m in poses_before.items())
    check('5.F keeps the 10 off pieces at Y4 (same set, same exact poses); digest = Q file',
          same and len(off_before) == 10 and dig['Q'] == parts['Q']['output_digest'],
          off=off_before.tolist())
    del G4
    for w in words['Pinv']:
        R.apply(*w)
    dig['Pinv'] = st.digest()
    for w in words['Vinv']:
        R.apply(*w)
    dig['Vinv'] = st.digest()
    off_end = int((st._kpose()[st.pose_id] < 0).sum())
    check('1/5.replay of the whole word: every part digest as filed, end = E3 digest, 0 off',
          all(dig[n] == parts[n]['output_digest'] for n in names) and dig['Vinv'] == E3d and off_end == 0,
          digests=dict(dig), off_end=off_end)
    for _ in range(len(full)):
        R.undo()
    check('undo back to E2', st.digest() == E2d)
    # Alphabet at E2 P4 (the shrink's Y_4, without V3)
    for w in words['P']:
        R.apply(*w)
    dP = st.digest()
    G4b = B.Alphabet(R, M)
    check('5.(162,1) is also SAFE-ALL at E2 P4 (shrink Y_4; digest = shrink after_P_digest)',
          (162, 1) in G4b.index and dP == shrink[4]['after_P_digest'], digest=dP)
    for _ in words['P']:
        R.undo()
    # E3 table digest as loaded
    R3, _ = O.start(str(C3 / 'E3-table.json'))
    check('1.E3 table loads to its digest', R3.st.digest() == E3d, loaded=R3.st.digest())
    OUT['failed'] = FAIL
    OUT['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(OUT, indent=1) + '\n')
    print(json.dumps({'failed': FAIL, 'checks': len(OUT['checks']), 'seconds': OUT['seconds']}))


if __name__ == '__main__':
    main()
