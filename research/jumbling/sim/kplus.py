"""The rotation group K+ of the 600-cell (order 7200), exactly.

An element is stored as its pole permutation (perm[x] = image of pole x) and identified by the
images of four independent basis poles. Its exact matrix is the unique linear map sending the
basis poles to those images, computed in Q(sqrt5) on demand. The map from matrices to pole
permutations is a homomorphism, so products of generator matrices have exactly the matrices
computed here; `verify_all` checks every element against all 600 poles anyway.
"""
from collections import deque
from math import gcd

import numpy as np

from exact import ONE, ZERO, Q5, identity, matmul, solve, transpose

ORDER = 7200


def to_tuple(m):
    return tuple(tuple(row) for row in m)


def is_rotation(m):
    return matmul(transpose(m), m) == identity(4) and det4(m) == ONE


def det4(m):
    def det3(a):
        return (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
    s = ZERO
    for j in range(4):
        minor = [[m[i][k] for k in range(4) if k != j] for i in range(1, 4)]
        t = m[0][j] * det3(minor)
        s = s + t if j % 2 == 0 else s - t
    return s


class KPlus:
    def __init__(self, data):
        self.data = data
        self.basis = list(data.basis)
        rp = data.rotperms
        gens = [rp[0], rp[1], rp[2], rp[3]]
        ident = np.arange(600, dtype=np.int64)
        seen = {self._key_of(ident[self.basis]): 0}
        elems = [ident]
        queue = deque([0])
        while queue:
            i = queue.popleft()
            for g in gens:
                q = g[elems[i]]
                k = self._key_of(q[self.basis])
                if k not in seen:
                    seen[k] = len(elems)
                    elems.append(q)
                    queue.append(len(elems) - 1)
        if len(elems) != ORDER:
            raise ValueError(f'K+ closure has order {len(elems)}, expected {ORDER}')
        self.perms = np.array(elems, dtype=np.int16)
        keys = np.array([self._key_of(p[self.basis]) for p in elems], dtype=np.int64)
        self._order = np.argsort(keys)
        self._keys = keys[self._order]
        inv_perm = np.argsort(self.perms, axis=1)
        self.inv = self.index_of_images(inv_perm[:, self.basis])
        assert np.all(self.inv >= 0)
        self.frame_idx = self.index_of_images(data.frameperms[:, self.basis])
        self.gen_idx = self.index_of_images(rp[:, self.basis])
        if np.any(self.frame_idx < 0) or np.any(self.gen_idx < 0):
            raise ValueError('a retained frame or generator is not in K+')
        self.stab0 = [int(i) for i in np.nonzero(self.perms[:, 0] == 0)[0]]
        assert len(self.stab0) == 12
        # exact basis inverse: M B = T with B the basis poles as columns
        bmat = [[data.N[b][r] for b in self.basis] for r in range(4)]
        self._binv = solve(bmat, identity(4))
        self._mat = {}
        self._int = {}
        self._a4 = {}
        self._words = {}

    @staticmethod
    def _key_of(img):
        a, b, c, d = (int(x) for x in img)
        return ((a * 600 + b) * 600 + c) * 600 + d

    def index_of_images(self, imgs):
        """K+ indices of elements with the given basis images (shape (n, 4)); -1 if none."""
        imgs = np.asarray(imgs, np.int64)
        keys = ((imgs[:, 0] * 600 + imgs[:, 1]) * 600 + imgs[:, 2]) * 600 + imgs[:, 3]
        pos = np.searchsorted(self._keys, keys)
        pos = np.minimum(pos, ORDER - 1)
        hit = self._keys[pos] == keys
        return np.where(hit, self._order[pos], -1)

    def index_of_perm(self, perm):
        i = int(self.index_of_images(np.asarray(perm)[self.basis][None, :])[0])
        if i >= 0 and not np.array_equal(self.perms[i], perm):
            return -1
        return i

    def compose(self, i, j):
        """Index of (element i) o (element j): j acts first."""
        return int(self.compose_vec(np.array([i]), np.array([j]))[0])

    def compose_vec(self, i, j):
        i = np.asarray(i, np.int64)
        j = np.asarray(j, np.int64)
        basis = np.asarray(self.basis, np.int64)[None, :]
        imgs = self.perms[i[:, None], self.perms[j[:, None], basis].astype(np.int64)]
        out = self.index_of_images(imgs)
        assert np.all(out >= 0)
        return out

    def matrix(self, i):
        """Exact matrix of element i as a tuple of row tuples of Q5."""
        m = self._mat.get(i)
        if m is None:
            perm = self.perms[i]
            t = [[self.data.N[int(perm[b])][r] for b in self.basis] for r in range(4)]
            m = to_tuple(matmul(t, self._binv))
            self._mat[i] = m
        return m

    def int_matrix(self, i):
        """(G1, G2, D) int64 arrays with matrix = (G1 + G2 sqrt5) / D."""
        f = self._int.get(i)
        if f is None:
            m = self.matrix(i)
            d = 1
            for row in m:
                for x in row:
                    d = d * x.d // gcd(d, x.d)
            g1 = np.array([[x.a * (d // x.d) for x in row] for row in m], np.int64)
            g2 = np.array([[x.b * (d // x.d) for x in row] for row in m], np.int64)
            f = (g1, g2, int(d))
            self._int[i] = f
        return f

    def pole_images_ok(self, i):
        """Exact check that the matrix of element i maps every pole n_x onto n_perm[x]."""
        g1, g2, d = self.int_matrix(i)
        n1, n2 = self.data.N1, self.data.N2
        r1 = n1 @ g1.T + 5 * (n2 @ g2.T)          # numerators of G n over 2d
        r2 = n2 @ g1.T + n1 @ g2.T
        perm = self.perms[i].astype(np.int64)
        return bool(np.array_equal(r1, d * n1[perm]) and np.array_equal(r2, d * n2[perm]))

    def index_of_matrix(self, m):
        """K+ index of an exact matrix, or -1 when it is not in K+ (exact test)."""
        imgs = []
        for b in self.basis:
            v = tuple(sum((m[r][c] * self.data.N[b][c] for c in range(4)), ZERO) for r in range(4))
            j = self.data.POLE.get(v)
            if j is None:
                return -1
            imgs.append(j)
        i = int(self.index_of_images(np.array([imgs]))[0])
        if i < 0 or self.matrix(i) != to_tuple(m):
            return -1
        return i

    def a4(self, c):
        """The 12 elements fixing pole c (the cap rotation group A4_c), frame-conjugated from
        the stabiliser of pole 0 in a fixed order."""
        out = self._a4.get(c)
        if out is None:
            f = int(self.frame_idx[c])
            finv = int(self.inv[f])
            out = [self.compose(f, self.compose(s, finv)) for s in self.stab0]
            assert all(int(self.perms[k][c]) == c for k in out) and len(set(out)) == 12
            self._a4[c] = out
        return out

    def a4_word(self, c, k):
        """Shortest word in the retained generators H_c, T_c, T_c^-1 for element k of A4_c, as
        signed 1-based primitive ids (generator j is id j + 1; a negative id is the inverse)."""
        words = self._words.get(c)
        if words is None:
            h, t = int(self.gen_idx[2 * c]), int(self.gen_idx[2 * c + 1])
            moves = [(h, 2 * c + 1), (t, 2 * c + 2), (int(self.inv[t]), -(2 * c + 2))]
            words = {0: []}
            queue = deque([0])
            while queue:
                x = queue.popleft()
                for g, mid in moves:
                    y = self.compose(g, x)
                    if y not in words:
                        words[y] = words[x] + [mid]
                        queue.append(y)
            assert sorted(words) == sorted(self.a4(c)), 'H_c and T_c do not generate A4_c'
            self._words[c] = words
        return list(words[k])

    def verify_all(self):
        """Exact checks over the whole group: every element is a rotation and maps every pole
        as its permutation says. Returns a summary dict."""
        bad_rot = [i for i in range(ORDER) if not is_rotation(self.matrix(i))]
        bad_img = [i for i in range(ORDER) if not self.pole_images_ok(i)]
        return {'order': ORDER, 'non_rotations': len(bad_rot), 'pole_image_failures': len(bad_img)}


def generator_word_matrix(kp, word):
    """Exact matrix of a word of signed 1-based primitive ids, applied left to right."""
    m = identity(4)
    for mid in word:
        k = int(kp.gen_idx[abs(mid) - 1])
        if mid < 0:
            k = int(kp.inv[k])
        m = matmul(kp.matrix(k), m)
    return to_tuple(m)


def q5_json(x):
    return [x.a, x.b, x.d]


def q5_from_json(v):
    a, b, d = v
    if any(type(x) is not int for x in (a, b, d)):
        raise ValueError('Q5 components must be integers (not booleans)')
    if d <= 0:
        raise ValueError('denominator must be positive')
    x = Q5(a, b, d)
    if [x.a, x.b, x.d] != [a, b, d]:
        raise ValueError('Q5 record is not in lowest terms')
    return x


def matrix_json(m):
    return [[q5_json(x) for x in row] for row in m]


def matrix_from_json(v):
    m = tuple(tuple(q5_from_json(x) for x in row) for row in v)
    if len(m) != 4 or any(len(r) != 4 for r in m):
        raise ValueError('matrix record must be 4 x 4')
    return m
