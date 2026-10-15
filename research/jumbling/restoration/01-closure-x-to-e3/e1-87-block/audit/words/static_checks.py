"""Audit 'words': structure and inverse checks of the native word E1 -> E2 (read-only on inputs).

Usage: python3 -I static_checks.py <out.json>
"""
import hashlib
import json
import re
import sys
from pathlib import Path

SP = Path('<work>')
V26 = SP / 'v26'
C0 = V26 / 'E2' / 'c0'
TH = SP / 'wt-e3' / 'research' / 'jumbling' / 'theory'
REPORT = V26 / 'report' / 'D2-E1-E2.md'
E1_DIGEST = '903997457de665c2846e00ba62b71c3d3ccb7df79c78391f5a1b9f1757afc4e8'
E2_DIGEST = '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff'

sys.dont_write_bytecode = True
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import numpy as np  # noqa: E402
import blocks as BL  # noqa: E402
import restore  # noqa: E402
from exact import identity, matmul, transpose, ZERO  # noqa: E402
from sim.kplus import to_tuple, det4  # noqa: E402

out = {'checks': [], 'fail': []}


def check(name, ok, **ev):
    rec = dict(check=name, ok=bool(ok), **ev)
    out['checks'].append(rec)
    if not ok:
        out['fail'].append(rec)
    print(json.dumps(rec, default=str)[:600], flush=True)


def load(p):
    return json.loads(Path(p).read_text())


parts = {k: load(C0 / 'parts' / f'{k}.json') for k in ['V', 'P', 'Q', 'Pinv', 'Vinv']}
W = {k: [tuple(x) for x in v['word']] for k, v in parts.items()}
full = load(C0 / 'word-full.json')
c0 = load(V26 / 'E2' / 'w87-c0.json')
c2 = load(V26 / 'E2' / 'w87-c2.json')

# ---- 1. lengths, digest chain, concatenation, P/Q vs w87-c0
lens = {k: len(v) for k, v in W.items()}
check('1a lengths 590/10/10/10/590', lens == {'V': 590, 'P': 10, 'Q': 10, 'Pinv': 10, 'Vinv': 590}, lens=lens)
order = ['V', 'P', 'Q', 'Pinv', 'Vinv']
check('1b V.input_digest = E1', parts['V']['input_digest'] == E1_DIGEST, got=parts['V']['input_digest'])
for a, b in zip(order, order[1:]):
    check(f'1c {a}.output = {b}.input', parts[a]['output_digest'] == parts[b]['input_digest'],
          a=parts[a]['output_digest'], b=parts[b]['input_digest'])
check('1d Vinv.output_digest = E2', parts['Vinv']['output_digest'] == E2_DIGEST, got=parts['Vinv']['output_digest'])
cat = [x for k in order for x in W[k]]
check('1e word-full = concatenation', [tuple(x) for x in full['word']] == cat, n_full=len(full['word']), n_cat=len(cat))
check('1f word-full digests', full['input_digest'] == E1_DIGEST and full['output_digest'] == E2_DIGEST,
      inp=full['input_digest'], outp=full['output_digest'])
check('1g P = w87-c0.P', W['P'] == [tuple(x) for x in c0['P']])
check('1h Q = w87-c0.Q', W['Q'] == [tuple(x) for x in c0['Q']])
check('1i total 1210', len(full['word']) == 1210, n=len(full['word']))
for f in ['E1-table.json', 'E2-table.json']:
    d = load(C0 / f)
    print(f, 'stored digest', d['digest'], 'sha256', hashlib.sha256((C0 / f).read_bytes()).hexdigest())

# ---- simulator at E1
R, M = BL.O.start(str(C0 / 'E1-table.json'))
st = R.st
dig = st.digest()
check('1j recomputed digest of E1-table = E1', dig == E1_DIGEST, got=dig)
kp = BL.CL.ctx.kplus
data = BL.CL.ctx.data
I4 = to_tuple(identity(4))
POLES = set(data.POLE.keys())

_E = {}


def E(c, i):
    if (c, i) not in _E:
        _E[(c, i)] = to_tuple(restore.element(M, c, i))
    return _E[(c, i)]


def in_kplus_indep(m):
    """Independent test: orthogonal, det 1, and every one of the 600 poles maps onto a pole."""
    if to_tuple(matmul(transpose(m), m)) != I4 or det4(m) != 1:
        return False
    for v in data.N:
        img = tuple(sum((m[r][c] * v[c] for c in range(4)), ZERO) for r in range(4))
        if img not in POLES:
            return False
    return True


def in_kplus(m):
    a = kp.index_of_matrix(m) >= 0
    b = in_kplus_indep(m)
    return a, b


def mul(a, b):
    return to_tuple(matmul(a, b))


# sanity of the independent K+ test
check('2z identity is in K+ (both tests)', in_kplus(I4) == (True, True))
nk = [in_kplus_indep(kp.matrix(j)) for j in range(0, 7200, 97)]
check('2z sample of K+ elements pass the independent test', all(nk), n=len(nk))

# ---- 2. letters
L = {'X': (115, 26), 'Xi': (115, 58), 'Y': (190, 3), 'Yi': (190, 9), 'Z': (317, 48), 'Zi': (317, 17),
     'A': (346, 10), 'B': (314, 1)}
memb = {k: in_kplus(E(*v)) for k, v in L.items()}
print('membership (sim index_of_matrix, independent pole test):', memb)
for k, v in memb.items():
    check(f'2a membership tests agree for {k}', v[0] == v[1], v=v)
check('2b X outside K+', memb['X'] == (False, False))
check('2b X^-1 outside K+', memb['Xi'] == (False, False))
check('2c X X^-1 = I', mul(E(*L['X']), E(*L['Xi'])) == I4 and mul(E(*L['Xi']), E(*L['X'])) == I4)
check('2d X not an involution (X^2 != I, X != X^-1)', mul(E(*L['X']), E(*L['X'])) != I4 and E(*L['X']) != E(*L['Xi']))
check('2e Y in K+', memb['Y'] == (True, True))
check('2e Y^-1 in K+', memb['Yi'] == (True, True))
check('2f Y Y^-1 = I', mul(E(*L['Y']), E(*L['Yi'])) == I4)
check('2f Y not identity', E(*L['Y']) != I4)
print('Y involution?', mul(E(*L['Y']), E(*L['Y'])) == I4)
check('2g Z outside K+', memb['Z'] == (False, False))
check('2g Z^-1 outside K+', memb['Zi'] == (False, False))
check('2h Z Z^-1 = I', mul(E(*L['Z']), E(*L['Zi'])) == I4)
print('Z involution?', mul(E(*L['Z']), E(*L['Z'])) == I4)
for k in 'AB':
    check(f'2i {k} in K+', memb[k] == (True, True))
    check(f'2i {k} involution (k^2 = I, k != I)', mul(E(*L[k]), E(*L[k])) == I4 and E(*L[k]) != I4)
# order of each letter (exact)
orders = {}
for k, v in L.items():
    m, n = E(*v), 1
    while m != I4 and n < 200:
        m, n = mul(m, E(*v)), n + 1
    orders[k] = n if m == I4 else None
print('orders', orders)
out['orders'] = orders
# uniqueness of the inverse letter in the menu at that grip
for k, inv in [('X', 'Xi'), ('Y', 'Yi'), ('Z', 'Zi'), ('A', 'A'), ('B', 'B')]:
    c, i = L[k]
    hits = [j for j in range(len(M.items)) if mul(E(c, i), E(c, j)) == I4]
    check(f'2j menu index of {k}^-1 at grip {c} is {L[inv][1]} (unique)', hits == [L[inv][1]], hits=hits)

# ---- 3. shapes, letter by letter
Pexp = [L[x] for x in ['X', 'Y', 'Xi', 'Yi', 'Z', 'Y', 'X', 'Yi', 'Xi', 'Zi']]
Qexp = [L[x] for x in ['A', 'B', 'X', 'B', 'Xi', 'A', 'X', 'B', 'Xi', 'B']]
check('3a P = X Y X^-1 Y^-1 Z Y X Y^-1 X^-1 Z^-1', W['P'] == Pexp,
      diff=[(n, a, b) for n, (a, b) in enumerate(zip(W['P'], Pexp)) if a != b])
check('3b Q = A B X B^-1 X^-1 A^-1 X B X^-1 B^-1 (A^-1=A, B^-1=B)', W['Q'] == Qexp,
      diff=[(n, a, b) for n, (a, b) in enumerate(zip(W['Q'], Qexp)) if a != b])


# symbolic: build [[X,Y],Z] and [A,[B,X]] from a free-group word expansion and compare
def inv_word(w):
    return [(s[:-1] if s.endswith('^') else s + '^') for s in reversed(w)]


def comm(a, b):
    return a + b + inv_word(a) + inv_word(b)


sym = {'X': L['X'], 'X^': L['Xi'], 'Y': L['Y'], 'Y^': L['Yi'], 'Z': L['Z'], 'Z^': L['Zi'],
       'A': L['A'], 'A^': L['A'], 'B': L['B'], 'B^': L['B']}
Psym = [sym[s] for s in comm(comm(['X'], ['Y']), ['Z'])]
Qsym = [sym[s] for s in comm(['A'], comm(['B'], ['X']))]
check('3c P equals free-group expansion of [[X,Y],Z]', W['P'] == Psym)
check('3d Q equals free-group expansion of [A,[B,X]]', W['Q'] == Qsym)
check('3e Pinv equals free-group expansion of [[X,Y],Z]^-1', W['Pinv'] == [sym[s] for s in inv_word(comm(comm(['X'], ['Y']), ['Z']))])

# ---- 4. mirror inverses
def mirror_check(name, w, winv):
    n = len(w)
    bad_grip, bad_inv = [], []
    for k in range(n):
        a, b = winv[k], w[n - 1 - k]
        if a[0] != b[0]:
            bad_grip.append((k, a, b))
        if mul(E(*a), E(*b)) != I4 or mul(E(*b), E(*a)) != I4:
            bad_inv.append((k, a, b))
    check(f'4 {name}: same length', len(w) == len(winv), n=len(w), ninv=len(winv))
    check(f'4 {name}: grips mirror', not bad_grip, bad=bad_grip[:5], n_bad=len(bad_grip))
    check(f'4 {name}: exact inverse matrices letter by letter', not bad_inv, bad=bad_inv[:5], n_bad=len(bad_inv))


mirror_check('Pinv vs P', W['P'], W['Pinv'])
mirror_check('Vinv vs V', W['V'], W['Vinv'])
# product of all V matrices times product of all Vinv matrices (as an abstract group word) is I
prodV = I4
for c, i in W['V']:
    prodV = mul(E(c, i), prodV)
prodVi = I4
for c, i in W['Vinv']:
    prodVi = mul(E(c, i), prodVi)
check('4 abstract product Vinv*V = I (left action, chronological)', mul(prodVi, prodV) == I4)

# ---- 5. V letters
pre = load(C0 / 'pre.json')
safe = set(map(int, pre['menu']['safe_grips']))
check('5 pre.json state digest is E1', pre.get('state_digest') == E1_DIGEST, got=pre.get('state_digest'))
notsafe = [(n, x) for n, x in enumerate(W['V']) if x[0] not in safe]
check('5a every V grip in pre.menu.safe_grips', not notsafe, bad=notsafe[:5], n_bad=len(notsafe))
vk = {}
for x in set(W['V']):
    m = E(*x)
    vk[x] = (in_kplus(m), m == I4)
bad_k = sorted(x for x, (k, idn) in vk.items() if k != (True, True))
bad_id = sorted(x for x, (k, idn) in vk.items() if idn)
check('5b every V letter in K+ (both tests)', not bad_k, bad=bad_k[:5], n_distinct=len(vk))
check('5c no V letter is the identity', not bad_id, bad=bad_id[:5])
vinvk = [x for x in set(W['Vinv']) if in_kplus(E(*x)) != (True, True) or E(*x) == I4 or x[0] not in safe]
check('5d every Vinv letter: safe grip, in K+, not identity', not vinvk, bad=vinvk[:5])

# independent SAFE-ALL recomputation for the grips used by V (classification at E1)
lat = st.lattice_flags()
offmask = st._kpose()[st.pose_id] < 0
check('5e lattice_flags == K-pose flags', np.array_equal(~lat, offmask), n_off=int(offmask.sum()))
grips = sorted({c for c, _ in W['V']} | {c for c, _ in W['P'] + W['Q']})
blocked = {}
for c in grips:
    cl = st.classify(c, need_inside=True, stop_at_straddle=False)
    off_in = int(offmask[np.asarray(cl.inside, np.int64)].sum())
    if cl.uncertain or cl.straddle is not None or off_in:
        blocked[c] = dict(uncertain=bool(cl.uncertain), straddle=cl.straddle is not None, off_inside=off_in)
vblocked = {c: v for c, v in blocked.items() if c in {g for g, _ in W['V']}}
check('5f V grips recomputed SAFE-ALL at E1 (no straddle, no off piece inside)', not vblocked,
      blocked=vblocked, n_grips=len({g for g, _ in W['V']}))
print('P/Q grips blocked at E1 (informational):', {c: v for c, v in blocked.items() if c in {115, 190, 317, 346, 314}})
out['pq_grips_at_E1'] = {str(c): blocked.get(c, 'safe') for c in [115, 190, 317, 346, 314]}

# ---- 6. report table vs parts
txt = REPORT.read_text()


def table_row(label):
    for line in txt.splitlines():
        if line.startswith('| ' + label + ' '):
            return [tuple(map(int, m)) for m in re.findall(r'\((\d+),(\d+)\)', line)], line
    return None, None


for label, key in [('P2 = [[X, Y], Z]', 'P'), ('Q2 = [A, [B, X]]', 'Q'), ('P2^-1', 'Pinv')]:
    row, line = table_row(label)
    check(f'6 report row "{label}" = parts/{key}.json', row == W[key], row=row, line=line)
# letter list lines 53-56
for pat, a, b in [(r'X = \((\d+), (\d+)\), with X\^-1 = \((\d+), (\d+)\)', 'X', 'Xi'),
                  (r'Y = \((\d+), (\d+)\), with Y\^-1 = \((\d+), (\d+)\)', 'Y', 'Yi'),
                  (r'Z = \((\d+), (\d+)\), with Z\^-1 = \((\d+), (\d+)\)', 'Z', 'Zi')]:
    m = re.search(pat, txt)
    g = tuple(map(int, m.groups())) if m else None
    check(f'6 report letter line {a}', g == L[a] + L[b], got=g)
m = re.search(r'A = \((\d+), (\d+)\) and B = \((\d+), (\d+)\)', txt)
check('6 report letter line A,B', m and tuple(map(int, m.groups())) == L['A'] + L['B'])

# ---- c2 note: shares Q, differs only in Y
Pc2 = [tuple(x) for x in c2['P']]
check('7a c2 shares Q2 with c0', [tuple(x) for x in c2['Q']] == W['Q'])
diffpos = [n for n in range(10) if Pc2[n] != W['P'][n]]
check('7b c2 P differs from c0 P only at Y/Y^-1 positions', diffpos == [1, 3, 5, 7], diffpos=diffpos)
Y2, Y2i = Pc2[1], Pc2[3]
check('7c c2 P is [[X,Y\'],Z] with Y\' = Pc2[1]',
      Pc2 == [L['X'], Y2, L['Xi'], Y2i, L['Z'], Y2, L['X'], Y2i, L['Xi'], L['Zi']], Y2=Y2, Y2i=Y2i)
check('7d c2 Y\' in K+ and Y\' Y\'^-1 = I and Y\' differs from Y',
      in_kplus(E(*Y2)) == (True, True) and mul(E(*Y2), E(*Y2i)) == I4 and E(*Y2) != E(*L['Y']))
try:
    r2 = load(V26 / 'E2' / 'c2' / 'result.json')
    flat = json.dumps(r2)
    print('c2 result mentions E2 digest:', E2_DIGEST in flat)
    out['c2_result_has_E2_digest'] = E2_DIGEST in flat
except Exception as e:  # noqa: BLE001
    print('c2 result read error', e)

Path(sys.argv[1]).write_text(json.dumps(out, indent=1, default=str))
print('FAILS', len(out['fail']))
