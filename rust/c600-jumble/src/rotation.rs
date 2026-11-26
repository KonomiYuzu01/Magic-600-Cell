//! Exact rotation requests (state-contract amendment A6, in force since 15 November 2026; KN3.4, KN3.7 to KN3.9).
//!
//! `c600_kernel::request` holds the bytes: the limits, the codes, the binding and the producer's token writer. This
//! module gives them their meaning with unbounded integers. [`Request::decode`] reads a request line under KN3.7's
//! canonical rules and re-encodes it to compare bytes; [`resolve`] builds the exact rotation in A1's frame at the named
//! cap and chooses: in free mode the nearest point of A1's rational grid, in menu mode a direct member or a drag's
//! nearest item. The [`Resolution`] is what the journal records, and [`replay`] reads the recorded rotation back,
//! running neither the producer nor the chooser (KN3.9). This is `twist.apply`'s input on both puzzles: on the jumbling
//! puzzle the resolved [`Twist`] goes to [`crate::state::State::apply`]; on 600-cell-Full [`retained_word`] dispatches
//! a retained item to its retained word (JI.35).
//!
//! Requests have no constructor besides [`Request::decode`]: no float, text helper or cast makes one, and a drag is
//! chosen only by [`resolve`], which records its choice (CT-LG2-16, kernel half).
//!
//! # Drafts (LP2)
//!
//! 1. Free mode's half-turn candidates are the canonical integer axes (first nonzero component positive, gcd 1) with
//!    every |component| ≤ N, each at denominator 1, so each half-turn is one candidate. Its tie key is
//!    (1, axis, after the Cayley point (1, axis)), which is A1's order with half-turns after Cayley points.
//! 2. Free mode enumerates every grid candidate in A1's tie order: (2N+1)³·D Cayley points and, when eligible,
//!    the half-turn axes. Its cost is that count of exact comparisons (KN3.6), reported as `comparisons` and not
//!    journalled; nothing is pruned, except at N = 0, where the grid is the identity alone and the search stops at
//!    denominator 1 after one comparison. Free mode is research-only (JI.35), and no budget or cancellation (KN7) is
//!    built for it here, so a search for N ≥ 1 near the limit runs its whole count.
//! 3. Distances: free mode records the squared Frobenius distance in Q, 8(1 − (q·c)²/(|q|²|c|²)) for the request and
//!    candidate quaternions q and c; a drag scores the 4 × 4 matrices at the cap by Σ(R − M)² in Q(√5).
//! 4. Order of refusals: decoding (`rotation_request_malformed`), the binding's shape, a cap outside 0..599
//!    (`rotation_request_malformed`), then by operation. Free: a pinned menu or an M request
//!    (`menu_binding_mismatch`), then the search limit (`rotation_search_bound`). Menu: M with `menu_drag`
//!    (`rotation_request_malformed`), the binding's digests, then M's digests (`menu_binding_mismatch`), M's index
//!    (`rotation_request_malformed`), a direct non-member (`twist_off_menu`).
//! 5. 600-cell-Full resolves against the pinned menu, as JI.35 says for Jumble off: a retained item (an element of
//!    A4_c) dispatches to the shortest retained word for it ([`KPlus::a4_word`](crate::kplus::KPlus::a4_word)), a
//!    jumble item is refused with `KN5:input.menu_word_needs_jumble_on`, and a free binding with
//!    `KN2:input.menu_binding_mismatch`, since free mode is research on the jumbling puzzle only.

use std::cell::Cell;

use c600_kernel::json::{self, Value};
use c600_kernel::request::{malformed_at, Binding, Fact, MenuMismatch, Operation, HALF_TURN_THRESHOLD};
use c600_kernel::request::{MAX_EXPONENT, MAX_MAGNITUDE_BITS};
use c600_kernel::request::{MAX_REQUEST_BYTES, PREFIX};
use c600_kernel::Refusal;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::codes;
use crate::model::POLES;
use crate::q5::{matrix_from_json, matrix_json, Mat4, Vec4, Q5};
use crate::twist::{cap_frame, cayley_matrix, Menu, RecordError, Twist};
use crate::Context;

fn refuse(code: &'static str, message: impl Into<String>) -> Refusal {
    Refusal { code, message: message.into(), fact: None }
}

/// A malformed-request refusal for the part named `field` of the request or its binding (CT-ERR-16, ERR.8; the names
/// are [`Fact::MalformedField`]'s).
fn malformed(field: &str, message: impl Into<String>) -> Refusal {
    malformed_at(field, message)
}

/// A binding refusal with its typed fact (CT-ERR-16, ERR.8): which digest differs, the pinned menu's and the named one.
fn binding_mismatch(
    source: &'static str,
    differs: MenuMismatch,
    expected: Option<&str>,
    held: Option<&str>,
    message: &str,
) -> Refusal {
    let (expected, held) = (expected.map(str::to_owned), held.map(str::to_owned));
    refuse(codes::MENU_BINDING_MISMATCH, message).with_fact(Fact::MenuBinding { source, differs, expected, held })
}

/// The first of the menu's two digests that a named pair disagrees with: the set, then the order (KN3.7).
fn first_digest_difference(menu: &Menu, set: &str, order: &str) -> Option<(MenuMismatch, String, String)> {
    if set != menu.identity {
        Some((MenuMismatch::Set, menu.identity.clone(), set.to_owned()))
    } else if order != menu.order {
        Some((MenuMismatch::Order, menu.order.clone(), order.to_owned()))
    } else {
        None
    }
}

thread_local! {
    static CHOOSER_RUNS: Cell<u64> = const { Cell::new(0) };
}

/// How often this thread has run a chooser (free search, direct membership or drag). Replay never runs one.
pub fn chooser_runs() -> u64 {
    CHOOSER_RUNS.with(Cell::get)
}

fn count_chooser_run() {
    CHOOSER_RUNS.with(|c| c.set(c.get() + 1));
}

/// A dyadic m/2^k.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Dyadic {
    m: BigInt,
    k: u32,
}

impl Dyadic {
    fn value(&self) -> Q5 {
        Q5::new(self.m.clone(), BigInt::zero(), BigInt::one() << self.k as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Form {
    Cayley([Dyadic; 3]),
    HalfTurn([BigInt; 3]),
    Menu { set: String, order: String, index: u64 },
}

/// A decoded rotation request (KN3.7). The only way to make one is [`Request::decode`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    form: Form,
}

/// The form letter of a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormKind {
    Cayley,
    HalfTurn,
    Menu,
}

fn canonical_decimal(text: &str, what: &str, field: &str) -> Result<(), Refusal> {
    if text.is_empty() || !text.bytes().all(|c| c.is_ascii_digit()) {
        return Err(malformed(field, format!("{what} is not a decimal integer")));
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(malformed(field, format!("{what} has a leading zero")));
    }
    Ok(())
}

fn token(text: &str, at: usize) -> Result<Dyadic, Refusal> {
    let field = format!("request.token.{at}");
    let field = field.as_str();
    let (m, k) = text.split_once(':').ok_or_else(|| malformed(field, "a token is m:k"))?;
    let (negative, digits) = match m.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, m),
    };
    canonical_decimal(digits, "m", field)?;
    if negative && digits == "0" {
        return Err(malformed(field, "negative zero is spelled 0:0"));
    }
    canonical_decimal(k, "k", field)?;
    let k: u32 = if k.len() <= 4 { k.parse().expect("four digits") } else { u32::MAX };
    if k > MAX_EXPONENT {
        return Err(malformed(field, format!("k exceeds {MAX_EXPONENT}")));
    }
    // 2^1024 has 309 decimal digits, so a longer m is out of bounds before parsing.
    if digits.len() > 309 {
        return Err(malformed(field, format!("m exceeds {MAX_MAGNITUDE_BITS} magnitude bits")));
    }
    let magnitude = BigInt::parse_bytes(digits.as_bytes(), 10).expect("decimal digits");
    if magnitude.bits() > MAX_MAGNITUDE_BITS {
        return Err(malformed(field, format!("m exceeds {MAX_MAGNITUDE_BITS} magnitude bits")));
    }
    if k > 0 && magnitude.is_even() {
        return Err(malformed(field, "m must be odd when k > 0"));
    }
    Ok(Dyadic { m: if negative { -magnitude } else { magnitude }, k })
}

fn digest(text: &str, field: &str) -> Result<String, Refusal> {
    if text.len() != 64 || !text.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
        return Err(malformed(field, "a digest is 64 lowercase hexadecimal digits"));
    }
    Ok(text.to_owned())
}

impl Request {
    /// Decodes one request line, refusing malformed, noncanonical and oversize bytes; nothing is repaired or
    /// rounded. The decoder encodes what it read and compares bytes.
    pub fn decode(bytes: &[u8]) -> Result<Request, Refusal> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(malformed("request.length", format!("a request has at most {MAX_REQUEST_BYTES} bytes")));
        }
        let Some((&b'\n', body)) = bytes.split_last() else {
            return Err(malformed("request.line_end", "a request ends with exactly one LF"));
        };
        if body.iter().any(|&c| !(b'!'..=b'~').contains(&c)) {
            return Err(malformed(
                "request.charset",
                "a request is printable ASCII with no whitespace before its final LF",
            ));
        }
        let text = std::str::from_utf8(body).expect("ASCII");
        let fields: Vec<&str> = text.split('|').collect();
        if fields.len() != 5 {
            return Err(malformed("request.field_count", "a request has five |-separated fields"));
        }
        if fields[0] != PREFIX {
            return Err(malformed("request.prefix", format!("unknown grammar prefix {:?}", fields[0])));
        }
        let form = match fields[1] {
            "C" => Form::Cayley([token(fields[2], 0)?, token(fields[3], 1)?, token(fields[4], 2)?]),
            "H" => {
                let mut axis = Vec::with_capacity(3);
                for (at, field) in fields[2..].iter().enumerate() {
                    let t = token(field, at)?;
                    if t.k != 0 {
                        return Err(malformed(&format!("request.token.{at}"), "H tokens have k = 0"));
                    }
                    axis.push(t.m);
                }
                let axis: [BigInt; 3] = axis.try_into().expect("three tokens");
                let first = axis
                    .iter()
                    .find(|x| !x.is_zero())
                    .ok_or_else(|| malformed("request.axis", "an H axis is nonzero"))?;
                if first.is_negative() {
                    return Err(malformed("request.axis", "an H axis has its first nonzero component positive"));
                }
                if !axis[0].gcd(&axis[1]).gcd(&axis[2]).is_one() {
                    return Err(malformed("request.axis", "an H axis is primitive"));
                }
                Form::HalfTurn(axis)
            }
            "M" => {
                canonical_decimal(fields[4], "the index", "request.index")?;
                let index =
                    fields[4].parse::<u64>().map_err(|_| malformed("request.index", "the index exceeds every menu"))?;
                Form::Menu { set: digest(fields[2], "request.set")?, order: digest(fields[3], "request.order")?, index }
            }
            other => return Err(malformed("request.form", format!("unknown request form {other:?}"))),
        };
        let request = Request { form };
        if request.encode() != bytes {
            return Err(malformed("request.canonical", "the request is not in canonical form"));
        }
        Ok(request)
    }

    /// The canonical bytes.
    pub fn encode(&self) -> Vec<u8> {
        let fields = match &self.form {
            Form::Cayley(w) => ("C", w.iter().map(|t| format!("{}:{}", t.m, t.k)).collect::<Vec<_>>()),
            Form::HalfTurn(a) => ("H", a.iter().map(|x| format!("{x}:0")).collect()),
            Form::Menu { set, order, index } => ("M", vec![set.clone(), order.clone(), index.to_string()]),
        };
        format!("{PREFIX}|{}|{}\n", fields.0, fields.1.join("|")).into_bytes()
    }

    pub fn kind(&self) -> FormKind {
        match self.form {
            Form::Cayley(_) => FormKind::Cayley,
            Form::HalfTurn(_) => FormKind::HalfTurn,
            Form::Menu { .. } => FormKind::Menu,
        }
    }

    /// The exact Cayley vector ω of a C request.
    pub fn omega(&self) -> Option<[Q5; 3]> {
        match &self.form {
            Form::Cayley(w) => Some([w[0].value(), w[1].value(), w[2].value()]),
            _ => None,
        }
    }

    /// The integer axis of an H request.
    pub fn axis(&self) -> Option<&[BigInt; 3]> {
        match &self.form {
            Form::HalfTurn(a) => Some(a),
            _ => None,
        }
    }

    /// The projective quaternion (q₀, q⃗) of a C or H request, in integers: C gives (2^K, 2^K ω), H gives (0, axis).
    fn quaternion(&self) -> Option<[BigInt; 4]> {
        match &self.form {
            Form::Cayley(w) => {
                let k = w.iter().map(|t| t.k).max().expect("three tokens");
                let scaled = |t: &Dyadic| &t.m << (k - t.k) as usize;
                Some([BigInt::one() << k as usize, scaled(&w[0]), scaled(&w[1]), scaled(&w[2])])
            }
            Form::HalfTurn(a) => Some([BigInt::zero(), a[0].clone(), a[1].clone(), a[2].clone()]),
            Form::Menu { .. } => None,
        }
    }

    /// The exact rotation of a C or H request at cap c, in A1's frame (state contract A1).
    pub fn matrix(&self, ctx: &Context, cap: usize) -> Option<Mat4> {
        let n = &ctx.model.poles[cap];
        match &self.form {
            Form::Cayley(_) => Some(cayley_matrix(n, &self.omega()?, &ctx.model.nn)),
            Form::HalfTurn(a) => {
                let axis = [Q5::new(a[0].clone(), BigInt::zero(), BigInt::one()), int_q5(&a[1]), int_q5(&a[2])];
                Some(half_turn_matrix(n, &axis, &ctx.model.nn))
            }
            Form::Menu { .. } => None,
        }
    }
}

fn int_q5(x: &BigInt) -> Q5 {
    Q5::new(x.clone(), BigInt::zero(), BigInt::one())
}

/// A1's half-turn branch H_u = 2(P_c + P_u) − I for the axis u = Σ a_i u_i of the cap frame.
pub fn half_turn_matrix(n: &[Q5], axis: &[Q5; 3], nn: &Q5) -> Mat4 {
    let frame = cap_frame(n);
    let u: Vec4 = std::array::from_fn(|j| (0..3).fold(Q5::zero(), |s, i| s.add(&axis[i].mul(&frame[i][j]))));
    let uu = crate::q5::dot(&u, &u);
    let (inv_n, inv_u) = (nn.inv().expect("|n|^2 > 0"), uu.inv().expect("a nonzero axis"));
    std::array::from_fn(|a| {
        std::array::from_fn(|b| {
            let p = n[a].mul(&n[b]).mul(&inv_n).add(&u[a].mul(&u[b]).mul(&inv_u));
            let two_p = p.add(&p);
            if a == b {
                two_p.sub(&Q5::one())
            } else {
                two_p
            }
        })
    })
}

/// The squared Frobenius distance Σ(a − b)² of two matrices.
pub fn frobenius2(a: &Mat4, b: &Mat4) -> Q5 {
    let mut sum = Q5::zero();
    for i in 0..4 {
        for j in 0..4 {
            let d = a[i][j].sub(&b[i][j]);
            sum = sum.add(&d.mul(&d));
        }
    }
    sum
}

/// A free-mode grid candidate: a Cayley point ω = numerators / denominator, or a half-turn about the axis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub half_turn: bool,
    pub denominator: u128,
    pub numerators: [i64; 3],
}

impl Candidate {
    fn quaternion(&self) -> [BigInt; 4] {
        let d = if self.half_turn { BigInt::zero() } else { BigInt::from(self.denominator) };
        let [x, y, z] = self.numerators.map(BigInt::from);
        [d, x, y, z]
    }

    /// The exact rotation at cap c.
    pub fn matrix(&self, ctx: &Context, cap: usize) -> Mat4 {
        let n = &ctx.model.poles[cap];
        let d = BigInt::from(self.denominator);
        if self.half_turn {
            let axis = self.numerators.map(|x| Q5::new(BigInt::from(x), BigInt::zero(), BigInt::one()));
            half_turn_matrix(n, &axis, &ctx.model.nn)
        } else {
            let omega = self.numerators.map(|x| Q5::new(BigInt::from(x), BigInt::zero(), d.clone()));
            cayley_matrix(n, &omega, &ctx.model.nn)
        }
    }

    fn to_json(&self) -> Value {
        json::object([
            ("family", Value::String(if self.half_turn { "half_turn" } else { "cayley" }.into())),
            ("denominator", Value::Number(self.denominator.to_string())),
            ("numerators", Value::Array(self.numerators.iter().map(|&x| Value::Int(x)).collect())),
        ])
    }
}

fn dot4(a: &[BigInt; 4], b: &[BigInt; 4]) -> BigInt {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The chosen grid point, its exact squared distance and the number of exact comparisons.
fn free_search(q: &[BigInt; 4], n: i64, d: u128, half_turns: bool) -> (Candidate, Q5, u64) {
    count_chooser_run();
    let qq = dot4(q, q);
    // best: (candidate, (q·c)², |c|²); a candidate wins on a larger (q·c)²/|c|², the first in A1's order on a tie.
    let mut best: Option<(Candidate, BigInt, BigInt)> = None;
    let mut comparisons = 0u64;
    let mut consider = |cand: Candidate| {
        let c = cand.quaternion();
        let (dot, cc) = (dot4(q, &c), dot4(&c, &c));
        let score = &dot * &dot;
        comparisons += 1;
        let better = match &best {
            None => true,
            Some((_, s, n2)) => &score * n2 > s * &cc,
        };
        if better {
            best = Some((cand, score, cc));
        }
    };
    // At N = 0 every grid point is the identity, which A1's order takes at denominator 1, and no half-turn axis
    // exists: the larger denominators only repeat it and lose the tie, so they are not compared.
    let last = if n == 0 { 1 } else { d };
    for den in 1..=last {
        for x in -n..=n {
            for y in -n..=n {
                for z in -n..=n {
                    consider(Candidate { half_turn: false, denominator: den, numerators: [x, y, z] });
                    let first = [x, y, z].into_iter().find(|&v| v != 0);
                    let primitive = (x.unsigned_abs()).gcd(&y.unsigned_abs()).gcd(&z.unsigned_abs()) == 1;
                    if den == 1 && half_turns && first.is_some_and(|f| f > 0) && primitive {
                        consider(Candidate { half_turn: true, denominator: 1, numerators: [x, y, z] });
                    }
                }
            }
        }
    }
    let (cand, score, cc) = best.expect("the grid has the identity");
    let total = &qq * &cc;
    let distance = Q5::new(BigInt::from(8) * (&total - &score), BigInt::zero(), total);
    (cand, distance, comparisons)
}

/// How a request was resolved; part of the journal record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Free mode: the chosen grid point and its exact squared distance (in Q).
    Free { chosen: Candidate, distance: Q5, comparisons: u64 },
    /// A direct menu request: the member's index in the pinned order.
    MenuDirect { index: usize },
    /// A drag: the selected index, the complete tie list and the exact squared distance (in Q(√5)).
    MenuDrag { index: usize, ties: Vec<usize>, distance: Q5 },
}

/// A resolved request: what the journal records next to the request bytes (KN3.8, KN3.9).
#[derive(Clone, Debug)]
pub struct Resolution {
    pub binding: Binding,
    pub request: Request,
    /// The SHA-256 of the raw request bytes (KN3.8).
    pub request_digest: String,
    pub choice: Choice,
    /// The exact rotation at the binding's cap.
    pub twist: Twist,
}

impl Resolution {
    /// The journal record: the binding, the request digest, the choice and the exact rotation.
    pub fn to_json(&self) -> Value {
        let choice = match &self.choice {
            Choice::Free { chosen, distance, .. } => json::object([
                ("kind", Value::String("free".into())),
                ("chosen", chosen.to_json()),
                ("distance2", distance.to_json()),
            ]),
            Choice::MenuDirect { index } => {
                json::object([("kind", Value::String("menu_direct".into())), ("index", Value::Int(*index as i64))])
            }
            Choice::MenuDrag { index, ties, distance } => json::object([
                ("kind", Value::String("menu_drag".into())),
                ("index", Value::Int(*index as i64)),
                ("ties", Value::Array(ties.iter().map(|&t| Value::Int(t as i64)).collect())),
                ("distance2", distance.to_json()),
            ]),
        };
        json::object([
            ("binding", self.binding.to_json()),
            ("request_digest", Value::String(self.request_digest.clone())),
            ("choice", choice),
            ("cap", Value::Int(self.twist.grip as i64)),
            ("matrix", matrix_json(&self.twist.matrix)),
        ])
    }
}

fn twist_at(ctx: &Context, cap: usize, matrix: Mat4) -> Result<Twist, Refusal> {
    Twist::new(ctx, cap, matrix).map_err(|e| malformed("request", format!("the exact rotation failed its check: {e}")))
}

/// Resolves a request under its binding against the state's pinned menu (`None` in free mode). Every refusal leaves
/// nothing chosen; a resolution is the exact rotation and its record.
pub fn resolve(ctx: &Context, menu: Option<&Menu>, binding: &Binding) -> Result<Resolution, Refusal> {
    let request = Request::decode(&binding.request)?;
    binding.check_shape()?;
    let cap = binding.cap as usize;
    if cap >= POLES {
        return Err(malformed("cap", "the cap is a pole index 0..599"));
    }
    let request_digest = c600_kernel::request::request_digest(&binding.request);
    let done = |choice: Choice, twist: Twist| Resolution {
        binding: binding.clone(),
        request: request.clone(),
        request_digest: request_digest.clone(),
        choice,
        twist,
    };
    match &binding.operation {
        Operation::Free { n, d } => {
            if let Some(menu) = menu {
                let message = "the state pins a menu, and a free request names none";
                let expected = Some(menu.identity.as_str());
                return Err(binding_mismatch("binding", MenuMismatch::PinnedMenuNotNamed, expected, None, message));
            }
            let q = request.quaternion().ok_or_else(|| {
                let held = match &request.form {
                    Form::Menu { set, .. } => Some(set.as_str()),
                    _ => None,
                };
                let message = "free mode pins no menu, so an M request refuses";
                binding_mismatch("request", MenuMismatch::FreeModePinsNoMenu, None, held, message)
            })?;
            binding.check_search_bound()?;
            let half_turns = match &request.form {
                Form::HalfTurn(_) => true,
                // ω·ω ≥ 13130 ⟺ |2^K ω|² ≥ 13130·(2^K)².
                _ => dot4(&q, &q) - &q[0] * &q[0] >= BigInt::from(HALF_TURN_THRESHOLD) * &q[0] * &q[0],
            };
            let n = i64::try_from(*n).expect("inside the search limit");
            let (chosen, distance, comparisons) = free_search(&q, n, *d, half_turns);
            let twist = twist_at(ctx, cap, chosen.matrix(ctx, cap))?;
            Ok(done(Choice::Free { chosen, distance, comparisons }, twist))
        }
        Operation::MenuDirect | Operation::MenuDrag => {
            let drag = binding.operation == Operation::MenuDrag;
            if drag && request.kind() == FormKind::Menu {
                return Err(malformed("operation", "a drag takes a C or H request, not M"));
            }
            let pin = binding.menu.as_ref().expect("checked shape");
            let menu = menu.ok_or_else(|| {
                let message = "free mode pins no menu";
                binding_mismatch("binding", MenuMismatch::FreeModePinsNoMenu, None, Some(&pin.set), message)
            })?;
            if let Some((differs, expected, held)) = first_digest_difference(menu, &pin.set, &pin.order) {
                let message = "the binding names another menu set or order";
                return Err(binding_mismatch("binding", differs, Some(&expected), Some(&held), message));
            }
            let twists =
                menu.for_cap(ctx, cap).map_err(|e| malformed("menu", format!("the pinned menu at the cap: {e}")))?;
            count_chooser_run();
            if let Form::Menu { set, order, index } = &request.form {
                if let Some((differs, expected, held)) = first_digest_difference(menu, set, order) {
                    let message = "the M request names another menu set or order";
                    return Err(binding_mismatch("request", differs, Some(&expected), Some(&held), message));
                }
                let index = usize::try_from(*index).ok().filter(|&i| i < twists.len());
                let index = index.ok_or_else(|| malformed("request.index", "the M index is outside the menu"))?;
                return Ok(done(Choice::MenuDirect { index }, twists[index].clone()));
            }
            let matrix = request.matrix(ctx, cap).expect("a C or H request");
            if !drag {
                let index = twists.iter().position(|t| *t.matrix == matrix).ok_or_else(|| {
                    refuse(codes::TWIST_OFF_MENU, "the requested rotation is not a member of the menu")
                })?;
                return Ok(done(Choice::MenuDirect { index }, twists[index].clone()));
            }
            let distances: Vec<Q5> = twists.iter().map(|t| frobenius2(&matrix, &t.matrix)).collect();
            let least = distances.iter().min().expect("a menu is not empty").clone();
            let ties: Vec<usize> = (0..distances.len()).filter(|&i| distances[i] == least).collect();
            let index = ties[0];
            Ok(done(Choice::MenuDrag { index, ties, distance: least }, twists[index].clone()))
        }
    }
}

/// Replay (KN3.9): the recorded exact rotation at the recorded cap, checked exactly. Neither the producer nor the
/// chooser runs.
pub fn replay(ctx: &Context, record: &Value) -> Result<Twist, RecordError> {
    let invalid = |m: &str| RecordError::Invalid(m.to_owned());
    let cap = match record.get("cap") {
        Some(Value::Int(c)) if (0..POLES as i64).contains(c) => *c as usize,
        _ => return Err(invalid("a rotation record needs its cap")),
    };
    let matrix = matrix_from_json(record.get("matrix").ok_or_else(|| invalid("a rotation record needs its matrix"))?)
        .map_err(RecordError::Invalid)?;
    Twist::new(ctx, cap, matrix)
}

/// 600-cell-Full (JI.35): the retained word of a resolved menu item, as signed 1-based primitive ids, or the refusal
/// of a jumble item. A free resolution never reaches 600-cell-Full.
pub fn retained_word(ctx: &Context, resolution: &Resolution) -> Result<Vec<i64>, Refusal> {
    if matches!(resolution.choice, Choice::Free { .. }) {
        return Err(refuse(codes::MENU_BINDING_MISMATCH, "600-cell-Full resolves requests against the pinned menu"));
    }
    // A retained item is an element of A4_c, the K⁺ stabiliser of the pole, and only those have a retained word.
    let twist = &resolution.twist;
    let word = twist.kidx.and_then(|k| ctx.kplus.a4_word(twist.grip, k));
    word.ok_or_else(|| {
        refuse(codes::MENU_WORD_NEEDS_JUMBLE_ON, "the item is a jumble twist; 600-cell-Full takes retained items only")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(bytes: &[u8]) -> &'static str {
        Request::decode(bytes).expect_err(&String::from_utf8_lossy(bytes)).code
    }

    /// CT-KN3-04 (part: canonical C, H and M requests decode exactly and re-encode to the same bytes; malformed,
    /// noncanonical and oversize bytes refuse with `KN2:input.rotation_request_malformed`, C=(2^1024+1,0,0)
    /// included; not run here: free mode and the bindings, which `tests/kn3.rs` runs) and CT-LG2-16 (part: the
    /// kernel admits no approximate spelling, so no float, exponent, NaN or infinity becomes a request; not run: the
    /// language half, programs' opaque host values, literals and casts, which the runtime owns).
    #[test]
    fn requests_decode_exactly_and_refuse_every_other_spelling() {
        let pow1024 = BigInt::one() << 1024usize;
        let max = format!("LX9B1|C|{}:0|0:0|0:0\n", &pow1024 - 1);
        let digests = format!("{}|{}", "0123456789abcdef".repeat(4), "f".repeat(64));
        let good = [
            "LX9B1|C|1:1|0:0|0:0\n".to_owned(),
            "LX9B1|C|-3:3|5:0|0:0\n".to_owned(),
            "LX9B1|C|69264588888054800706886561:79|0:0|0:0\n".to_owned(),
            "LX9B1|C|1:1074|-1:1074|7:0\n".to_owned(),
            max.clone(),
            "LX9B1|H|1:0|0:0|0:0\n".to_owned(),
            "LX9B1|H|0:0|2:0|-3:0\n".to_owned(),
            format!("LX9B1|M|{digests}|0\n"),
            format!("LX9B1|M|{digests}|59\n"),
        ];
        for line in &good {
            let request = Request::decode(line.as_bytes()).expect(line);
            assert_eq!(request.encode(), line.as_bytes());
        }
        let omega = Request::decode(b"LX9B1|C|-3:3|5:0|0:0\n").expect("C").omega().expect("omega");
        assert_eq!(omega, [Q5::of(-3, 0, 8), Q5::int(5), Q5::zero()]);
        let big = Request::decode(max.as_bytes()).expect("1024 bits").omega().expect("omega");
        assert_eq!(big[0], Q5::new(&pow1024 - 1, BigInt::zero(), BigInt::one()));

        let long = format!("LX9B1|C|{}1:0|0:0|0:0\n", "1".repeat(2048));
        let bad: Vec<String> = vec![
            format!("LX9B1|C|{}:0|0:0|0:0\n", &pow1024 + 1),
            format!("LX9B1|C|{}:0|0:0|0:0\n", &pow1024),
            long,
            "LX9B1|C|1:1|0:0|0:0".into(),
            "LX9B1|C|1:1|0:0|0:0\n\n".into(),
            "LX9B1|C|1:1|0:0|0:0\r\n".into(),
            "LX9B1|C|1:1|0:0|0:0 \n".into(),
            " LX9B1|C|1:1|0:0|0:0\n".into(),
            "LX9B1|C|01:1|0:0|0:0\n".into(),
            "LX9B1|C|+1:1|0:0|0:0\n".into(),
            "LX9B1|C|-0:0|0:0|0:0\n".into(),
            "LX9B1|C|0:1|0:0|0:0\n".into(),
            "LX9B1|C|2:1|0:0|0:0\n".into(),
            "LX9B1|C|1:01|0:0|0:0\n".into(),
            "LX9B1|C|1:-1|0:0|0:0\n".into(),
            "LX9B1|C|1:1075|0:0|0:0\n".into(),
            "LX9B1|C|1.5:0|0:0|0:0\n".into(),
            "LX9B1|C|1e3:0|0:0|0:0\n".into(),
            "LX9B1|C|NaN|0:0|0:0\n".into(),
            "LX9B1|C|inf:0|0:0|0:0\n".into(),
            "LX9B1|C|0.5|0:0|0:0\n".into(),
            "LX9B1|C|1:1|0:0\n".into(),
            "LX9B1|C|1:1|0:0|0:0|0:0\n".into(),
            "LX9B2|C|1:1|0:0|0:0\n".into(),
            "LX9B1|X|1:1|0:0|0:0\n".into(),
            "LX9B1|c|1:1|0:0|0:0\n".into(),
            "LX9B1|H|1:1|0:0|0:0\n".into(),
            "LX9B1|H|2:0|0:0|0:0\n".into(),
            "LX9B1|H|-1:0|0:0|0:0\n".into(),
            "LX9B1|H|0:0|0:0|0:0\n".into(),
            "LX9B1|H|0:0|-2:0|3:0\n".into(),
            format!("LX9B1|M|{}|{}|0\n", "A".repeat(64), "f".repeat(64)),
            format!("LX9B1|M|{}|{}|0\n", "a".repeat(63), "f".repeat(64)),
            format!("LX9B1|M|{digests}|01\n"),
            format!("LX9B1|M|{digests}|-1\n"),
            format!("LX9B1|M|{digests}|99999999999999999999999\n"),
            "LX9B1|C|1:1|0:0|0:\u{e9}\n".into(),
        ];
        for line in &bad {
            assert_eq!(refused(line.as_bytes()), codes::ROTATION_REQUEST_MALFORMED, "{line:?}");
        }
    }

    /// CT-KN3-06 (part: bytes from the kernel half of the producer decode to exactly the binary64 value read; not
    /// run: the shell's gesture producer).
    #[test]
    fn producer_tokens_decode_to_the_exact_binary64_value() {
        // Bit patterns: the value nearest 1/10, -0, a subnormal, a large normal, -7.25, the largest finite value and
        // the smallest normal.
        let patterns = [
            0x3fb9_9999_9999_999a,
            0x8000_0000_0000_0000,
            0x0000_3727_dfa2_0b6c,
            0x7e37_e43c_8800_759c,
            0xc01d_0000_0000_0000,
            0x7fef_ffff_ffff_ffff,
            0x0010_0000_0000_0000u64,
        ];
        for bits in patterns {
            let t = c600_kernel::request::token_of_binary64(bits).expect("finite");
            let bytes = c600_kernel::request::cayley_request([&t, "0:0", "0:0"]);
            let w = Request::decode(&bytes).expect("canonical").omega().expect("omega");
            // The exact value: (-1)^s · M · 2^E from the bits, independently of the token writer.
            let (e, f) = (((bits >> 52) & 0x7ff) as i64, bits & ((1 << 52) - 1));
            let (m, exp) = if e == 0 { (f, -1074) } else { (f | 1 << 52, e - 1075) };
            let mut value = BigInt::from(m);
            if bits >> 63 == 1 {
                value = -value;
            }
            let exact = if exp >= 0 {
                Q5::new(value << exp as usize, BigInt::zero(), BigInt::one())
            } else {
                Q5::new(value, BigInt::zero(), BigInt::one() << (-exp) as usize)
            };
            assert_eq!(w[0], exact, "{bits:#018x}");
        }
    }
}
