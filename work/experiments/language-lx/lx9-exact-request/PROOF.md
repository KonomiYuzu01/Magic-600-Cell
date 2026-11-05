# Certified comparisons and the proposed repair of D

Experiment only; decision **open**, owner. All proof arithmetic uses integers
and `Fraction`, including decoding binary64 inputs. The float capture in
`freeze.py` is separate from the proof. No numerical approximation is accepted
as equality.

## 1. The request behind the six platform differences

The degree literal is captured as binary64 `4051e429cc698771`, exactly
5035942778341233 / 70368744177664. The `math.radians(math.remainder(d,360.0))`
operation used by `twists.py` produces `3ff3fc176b7a8560`, exactly
175787564848171 / 140737488355328. `INPUTS.json` also captures the binary64
radians-per-degree constant. The checker verifies both decimal-to-binary64
rounding and multiplication rounding against exact adjacent-value midpoints.
It does not treat the decimal spelling as an exact decimal degree request.

For a candidate quaternion q=(d,p), let S=(q.u)^2/(q.q), where u is the unit
target quaternion. Squared Frobenius distance is 8(1-S). Thus larger S means
smaller distance. The fixed fourth-space pole contributes zero distance.
For signed coordinate axis a and r=p.a,

```text
S = A + B cos(t) + C sin(t)
A = (d^2+r^2)/(2(d^2+p.p))
B = (d^2-r^2)/(2(d^2+p.p))
C = d*r/(d^2+p.p).
```

Use d=0 for a half-turn's score. For the candidates (1,a) and (2,a),
S1-S2=(sin(t)-3 cos(t))/10. The following are rational closed enclosures;
**divide every endpoint in the table by 2^96**. They are reproduced identically
by the two independent routes in `case.json`.

| Value | Lower numerator | Upper numerator |
| --- | ---: | ---: |
| pi | 248902613312231085230521944622 | 248902613312231085230521944623 |
| t, exact degrees | 98959601443323727856071533110 | 98959601443323727856071533111 |
| sin(t), exact degrees | 75162434512514379647738207253 | 75162434512514379647738207254 |
| cos(t), exact degrees | 25054144837504792243199179809 | 25054144837504792243199179810 |
| S1-S2, exact degrees | 291814066782 | 291814066783 |
| t, binary64 radians | 98959601443323727107275620352 | 98959601443323727107275620352 |
| sin(t), binary64 radians | 75162434512514379410948148559 | 75162434512514379410948148560 |
| cos(t), binary64 radians | 25054144837504792953569355892 | 25054144837504792953569355893 |
| S1-S2, binary64 radians | 55024008088 | 55024008089 |

Both score differences are **strictly positive**. The corresponding squared
distance differences are -8 times those intervals and strictly negative.
The 90-degree candidate (1,a) is the unique global winner in both semantics.
Every one of the 432 raw tuples for N=1, D=16 is compared; `case_axes.tsv`
repeats the global certificate for +x, -x, +y, -y, +z, -z. This is one input
event under coordinate symmetry, not six independent platform events.

Primary enclosures (`intervals.py`):

- pi = 16 atan(1/5) - 4 atan(1/239). Each arctangent uses
  sum((-1)^j x^(2j+1)/(2j+1)); its alternating, decreasing tail is between
  zero and the next signed term. For the identity, tan(4 atan(1/5))=120/119;
  subtracting atan(1/239) gives tangent 1 in the first quadrant.
- Sine and cosine use degree-m Maclaurin polynomials with Lagrange bound
  |x|^(m+1)/(m+1)!, since every real derivative has magnitude at most 1.
  Argument intervals use the midpoint plus their radius by the 1-Lipschitz
  property of sine and cosine. All endpoints round outward to dyadics.

Independent enclosures (`oracle.py`):

- pi = 4(atan(1/2)+atan(1/3)); the addition formula gives tangent 1 in
  the first quadrant. Separately accumulated rational series bound the tails.
- Sine/cosine use alternating series and the next signed term after the tail
  terms are decreasing. Interval argument uncertainty again has a proven
  Lipschitz bound. The score is constructed from half-angle quaternion squares:
  (c+s)^2/2 - (2c+s)^2/5. Full-angle sine/cosine are independently obtained
  by double-angle identities. No primary enclosure or score routine is called.

The adaptive comparison schedule is 16,32,64,128,256,512 bits. Both global
checks needed at most **64 output enclosure bits**, with analytic guard bits
(up to 24 extra bits in these comparisons). The retained certificate table is
rounded to 96 bits for inspection. Exact rational operand sizes are not
bounded by that precision parameter. These are precision counts, not timings.

## 2. Exact ties and termination for exact dyadic degrees

At exactly 45 degrees about +x,

```text
S(2,1,0,0)-S(3,1,0,0) = (sin(t)-cos(t))/10 = 0.
```

The full 432-tuple enumeration proves that these two are globally nearest.
A1's denominator tie rule picks (2,1,0,0). Algebraically, in Q(zeta_8), sine
and cosine at this angle are the same positive root of 2x^2-1. The independent
computation squares half-angle quaternion products in Q(zeta_16) and obtains
the same equality and full minimiser list. Polynomial remainders are exactly
zero, not small. `tie_refinement.json` shows strict enclosures around zero at
16,32,64,128,256 bits in both routes. Since the true difference is zero, no
sound interval refinement alone can ever exclude zero. The largest **nondeciding
probe** was 256 bits; it is separate from the 64-bit deciding maximum above.

Here is the proposed repair, with its needed qualifications made explicit.

1. For a nonzero dyadic axis a, q=a.a is positive rational. Rodrigues' formula
   is aa^T/q + cos(t)(I-aa^T/q) + sin(t)[a]_cross/sqrt(q). Against exact
   Q(sqrt(5)) candidate matrices, a distance difference has
   F=A+B cos(t)+C sin(t), with A,B in Q(sqrt(5)) and C in
   K=Q(sqrt(5),sqrt(q)). In fact this construction has C in Q(sqrt(5))/sqrt(q).
   The larger field in the proposal is a valid bound.
2. **First detect A=B=C=0.** Distinct rotations can have identical scores for
   every angle on an axis: (1,0,1,0) and (1,0,-1,0) about +x are an explicit
   checked example. Duplicate parameter encodings are another example. Such
   ties occur outside any finite exceptional-angle list and use the tie rule
   immediately. Omitting this branch would invalidate the termination claim.
3. Otherwise, put z=exp(i*t). Multiplying F=0 by 2z gives
   (B-i*C)z^2+2A*z+(B+i*C)=0. This polynomial is nonzero. The coefficients
   are real before adjoining i, so simultaneous vanishing would force
   A=B=C=0. The degree is at most 2 over K(i). Since [K:Q]<=4 and
   [K(i):Q]<=8, [Q(z):Q]<=16. The tower/compositum inequality applies even
   when K is not contained in Q(z).
4. Write the degree input as m/2^k. Then t=pi*m/(180*2^k), and z is a root
   of unity with exact order n dividing 360*2^k. Its degree is phi(n).
   Thus a nontrivial zero requires phi(n)<=16. This uses the root-of-unity
   degree theorem, not the invalid Lindemann-Weierstrass degree-angle step.
5. n=2^a*3^b*5^c with b<=2, c<=1. Since phi(2^a)<=16, a<=5; hence every
   possible order divides 1440. Exhausting these exponents and independently
   counting coprime residues gives the following complete **necessary** list:

   ```text
   1,2,3,4,5,6,8,9,10,12,15,16,18,20,24,30,32,36,40,48,60
   ```

   The angle list is {360*j/n : listed n, 0<=j<n, gcd(j,n)=1}, with n=1
   contributing zero. There are **152 distinct angles modulo 360**, all
   dyadic degrees. Every angle, exact winner and complete minimiser list is
   retained in `exceptional.tsv`. Membership in this list does not imply a tie.
6. At these angles, trig values, i and sqrt(5) are in the fixed field
   L=Q(zeta_1440), of degree 384. **An arbitrary sqrt(q) need not be in L.**
   A literal claim that F always belongs to one fixed cyclotomic field is
   false; axes can introduce arbitrary quadratic extensions (sqrt(7), whose
   quadratic conductor is 28, is outside the conductor-1440 field). Nevertheless
   the equality test can run
   inside L: write F=X+Y sqrt(q), X,Y in L. If Y=0, test X=0. Otherwise test
   X^2=qY^2 exactly, then require opposite real signs of X and Y. Squaring
   without this sign guard is unsound. Signs of these nonzero algebraic
   elements terminate under rational interval refinement. `radical_zero`
   implements this test in the required subfield of L; four known radical
   cases check cancellation, the squaring false-positive guard, an outside
   radical and sqrt(5). Alternatively one could use the request-dependent
   compositum L(sqrt(q)), but that is not a fixed field independent of axes.
7. Outside the list, every non-identically-zero comparison is nonzero, so
   convergent rational enclosures eventually separate its sign. Together
   with the exact exceptional test and the identically-zero branch, this
   proves termination for a finite candidate domain and valid dyadic axes
   and degree inputs. It does **not** supply an affordable precision bound,
   admission cost or implementation budget (EN1-Q3, KN7.8). The finite fixture
   caps fail loudly; they are not the unbounded mathematical algorithm.

All 152 listed-angle fixtures use axis +x, N=1, D=16, with half-turn axes only
at 180 degrees under A1's one-degree eligibility rule. The primary route uses
full-angle coefficients in Q(zeta_lcm(n,4)); the brute route uses squared
half-angle quaternions in Q(zeta_lcm(2n,4)). These are subfields of L. They
share rational polynomial arithmetic, but neither scoring nor sign-enclosure
code. Every winner and every tied raw tuple agrees. All nonzero comparisons
in this table resolved at 16 bits. Only 45 and 315 degrees tie distinct
winning rotations; zero and 180 have equivalent parameter encodings tied.

This checks the stated argument, not an implementation of a general product
D chooser for arbitrary axes. Its degree-angle reasoning does not describe
the rounded quaternion currently scored by the evaluator. No A1 interpretation
is silently changed by either experiment.

## 3. Default-grid coverage

`default_bisectors.json` certifies three global boundaries and the dyadic
coordinates on either side. `default_grid.json` covers six requests at N=16,
D=1000. They are exact synthetic C/H targets as defined in `REQUEST.md`, not
samples of natural gestures or the platform's float target-production path.

- Axial Cayley neighbours: 1/1000 and 1/999; 15/16 and 16/17.
- Half-turn axis slopes in the xy plane: 1/16 and 1/15.

The exact midpoint in orientation has coordinate

```text
w = (u+v)/(sqrt((1+u^2)(1+v^2)) + 1-u*v),
(u+v) w^2 + 2(1-u*v) w - (u+v) = 0.
```

Integer bisection gives two adjacent multiples of 2^-32 straddling this root;
each request is less than 2^-32 from it **in Cayley/axis-slope coordinates**.
Independent integer-square-root enclosures certify the same bracket. This is
not a binary64-degree-ulp claim. Both complete slope sets are enumerated;
coprime-pair counting and integer cross products independently verify their
sizes and that no eligible slope lies between each pair.

For the axial requests, orthogonal numerator components increase the norm
without changing the quaternion dot. Since 0<u<v<1, the requested half-angle
is between 0 and pi/4; opposite projective directions cannot beat even the
identity's score (>1/2). Thus adjacent axial slopes give the global bisector.
For half-turns, z components likewise worsen the score; Cayley d>0 only adds
to the norm. Vertical axes cannot beat the near-x neighbours. The bounded
xy-axis slopes therefore supply the global half-turn bisector.

`grid.rs` enumerates all 35,937,000 Cayley tuples per request, plus all 35,936
nonzero half-turn axes when eligible. Checked u128 products fail on overflow.
It maximises (t.q)^2/(q.q), with the target norm common to candidates.
The Python denominator-pruning route independently enumerates each numerator
triple and retains 1,D and clamped floor/ceil(t0*(p.p)/(t_vector.p)), exactly
the derivative candidates in `twists.py`. Counts deduplicate denominators
per numerator triple; no floating finalist screening is used or tested.

A third route uses the Frobenius residual |t|^2|q|^2-(t.q)^2. Since every
target has t3=0, z=-16,0,16 suffice to audit each complete z block: the dot
numerator is constant; norms decrease to z=0 then increase. They preserve
the winning tuple and the largest score-comparison operand. If a new leader
appears inside the block, any skipped left product is bounded by the sampled
right product at z=16; pre-existing-leader products are sampled at the ends.
For t0=0, d=1,1000 also preserve the leader and operand maximum because
scores only decrease as d increases. This is an independent exact audit of
the complete domain, not denominator pruning. It agrees with the Rust winner
and observed operand bits in every row. Full domain counts are checked by
their integer cardinality formula as well.

These six certificates do not prove that pruning preserves the global winner
for every input, and do not test NumPy's floating screening or natural input
frequency. Their largest score operand was 104 bits; no timing claim is made.
