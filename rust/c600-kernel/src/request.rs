//! Exact rotation requests of state-contract amendment A6 at the byte level (KN3.4, KN3.7 to KN3.9; LP2).
//!
//! A fresh rotation request is exact bytes (KN3.4). This module holds the part that needs nothing beyond the standard
//! library: the wire limits and the refusal codes of KN3.7 and KN3.8, the binding that travels beside the bytes
//! (KN3.7's FP.5 object), the request digest of KN3.8, builders for the three wire forms, and the token writer the
//! shell's producer uses (KN3.9). The meaning of a request (canonical decoding, the exact rotation, free mode and the
//! menu chooser) needs unbounded integers and lives in `c600-jumble`'s `rotation` module.
//!
//! # Drafts (LP2)
//!
//! - Placement: this crate holds no big integers (KN3.2 is split), so the bytes live here and their meaning in
//!   `c600-jumble`. The shell, which depends on this crate only, can build and digest requests without arithmetic.
//! - KN3.9: the float site stays in the shell. The shell passes a binary64 value's bit pattern (`to_bits`) to
//!   [`token_of_binary64`], so no float type enters the kernel (CT-KN2-05); the writer reads the bit pattern exactly,
//!   refuses a non-finite pattern and writes both signed zeros as `0:0`. A value of 2^53 or more is written in
//!   decimal by repeated doubling of base-10^9 digits, which is formatting, not a big-integer type.
//! - The producer's refusal of a non-finite value uses `KN2:input.rotation_request_malformed`: chapter 13 names no
//!   producer code, and no request bytes exist for it.
//! - A malformed binding object (a missing or mistyped field, `menu` not null exactly for free) is refused with
//!   `KN2:input.rotation_request_malformed`; a Nat bound above `u128` is outside the search limit and refused with
//!   `KN2:resource_limit.rotation_search_bound`, as any bound outside the limit is.

use std::collections::BTreeMap;

use crate::json::Value;
use crate::Refusal;

/// The grammar prefix of KN3.7; a later grammar takes a new prefix.
pub const PREFIX: &str = "LX9B1";
/// The longest request line, final LF included (draft admission limit, KN3.7).
pub const MAX_REQUEST_BYTES: usize = 2048;
/// The most magnitude bits of a token's m (draft admission limit, KN3.7).
pub const MAX_MAGNITUDE_BITS: u64 = 1024;
/// The largest exponent k of a token m:k (draft admission limit, KN3.7).
pub const MAX_EXPONENT: u32 = 1074;
/// The trial search limit of free mode: (2N+1)³·D ≤ 2^40 (KN3.7).
pub const SEARCH_LIMIT: u128 = 1 << 40;
/// A C request with ω·ω at least this has half-turn candidates in free mode (KN3.7; cot²(π/360) ≈ 13130.56).
pub const HALF_TURN_THRESHOLD: u64 = 13130;

/// The refusal codes of chapter 13 for rotation requests.
pub mod codes {
    /// Request bytes fail the exact request format, canonical form or size bound (KN3.7).
    pub const ROTATION_REQUEST_MALFORMED: &str = "KN2:input.rotation_request_malformed";
    /// A menu letter or drag names other menu digests, or an M request arrives in free mode (KN3.7, KN3.8).
    pub const MENU_BINDING_MISMATCH: &str = "KN2:input.menu_binding_mismatch";
    /// A direct C or H request that is not exactly a menu member (KN3.8).
    pub const TWIST_OFF_MENU: &str = "KN2:input.twist_off_menu";
    /// Free-mode bounds outside the search limit; no search runs (KN3.7).
    pub const ROTATION_SEARCH_BOUND: &str = "KN2:resource_limit.rotation_search_bound";
}

pub(crate) fn refuse(code: &'static str, message: impl Into<String>) -> Refusal {
    Refusal { code, message: message.into(), fact: None }
}

/// Which digest of a menu binding disagrees, in KN3.7's order: the menu set, then the menu order, or that the request
/// and the state do not agree on whether a menu is pinned at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuMismatch {
    /// The set digest differs.
    Set,
    /// The order digest differs (the set agrees).
    Order,
    /// Free mode pins no menu, so a menu operation or an M request has none to name.
    FreeModePinsNoMenu,
    /// The state pins a menu, and a free binding names none.
    PinnedMenuNotNamed,
}

impl MenuMismatch {
    pub fn name(self) -> &'static str {
        match self {
            MenuMismatch::Set => "set",
            MenuMismatch::Order => "order",
            MenuMismatch::FreeModePinsNoMenu => "free_mode_pins_no_menu",
            MenuMismatch::PinnedMenuNotNamed => "pinned_menu_not_named",
        }
    }
}

/// The typed failed fact of ERR.8 for the three rotation-request refusals (CT-ERR-16; draft until the freeze).
///
/// - [`Fact::MalformedField`]: the part of the request or its binding that failed, by a dotted name. From the request
///   line: `request.length`, `request.line_end`, `request.charset`, `request.field_count`, `request.prefix`,
///   `request.form`, `request.token.<0..2>`, `request.axis`, `request.set`, `request.order`, `request.index`,
///   `request.canonical`, and `request` for an exact rotation that fails its check. From the binding: `binding`,
///   `binding.fields`, `binding.request`, `binding.cap`, `binding.operation`, `binding.operation.kind`,
///   `binding.operation.n`, `binding.operation.d`, `binding.menu`, `binding.shape`; and, once resolved, `cap`,
///   `operation` and `menu`. `value` names a non-finite binary64 value with no exact request token. A twist row's
///   argument is named by its own name, `rotation` or `letter`, which `c600-command` emits.
/// - [`Fact::MenuBinding`]: which digest differs (or that free mode pins no menu), with the pinned menu's digest as
///   `expected` and the digest the binding or the request named as `held`, each `None` when there is none. A digest is
///   echoed only when it has a digest's shape (64 lowercase hexadecimal digits), so the fact stays bounded whatever
///   the binding names; otherwise it is `None`. `source` says whether the binding or the M request named it.
/// - [`Fact::SearchBound`]: the free-mode bounds N and D in decimal against the limit of (2N+1)^3*D (draft limit
///   [`SEARCH_LIMIT`]), both bounds read.
/// - [`Fact::SearchBoundNat`]: the bound `n` or `d` that is a Nat above `u128`, with its digit count; the bound that
///   follows it is not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    MalformedField { field: String },
    MenuBinding { source: &'static str, differs: MenuMismatch, expected: Option<String>, held: Option<String> },
    SearchBound { n: String, d: String, limit: u128 },
    SearchBoundNat { field: &'static str, digits: usize },
}

impl Fact {
    /// The fact as a JSON object with a `kind` key (the receipt's `fact`). Its keys sort by name, as every
    /// [`Value::Object`] does, so `kind` is not first.
    pub fn to_json(&self) -> Value {
        let text = |s: &str| Value::String(s.to_owned());
        let opt = |s: &Option<String>| s.as_deref().map_or(Value::Null, |s| Value::String(s.to_owned()));
        match self {
            Fact::MalformedField { field } => {
                crate::json::object([("kind", text("malformed_field")), ("field", text(field))])
            }
            Fact::MenuBinding { source, differs, expected, held } => crate::json::object([
                ("kind", text("menu_binding")),
                ("source", text(source)),
                ("differs", text(differs.name())),
                ("expected", opt(expected)),
                ("held", opt(held)),
            ]),
            Fact::SearchBound { n, d, limit } => crate::json::object([
                ("kind", text("search_bound")),
                ("n", text(n)),
                ("d", text(d)),
                ("limit", nat(*limit)),
            ]),
            Fact::SearchBoundNat { field, digits } => crate::json::object([
                ("kind", text("search_bound_nat")),
                ("field", text(field)),
                ("digits", Value::Int(*digits as i64)),
            ]),
        }
    }
}

/// A rotation-request refusal for the part named `field` (CT-ERR-16, ERR.8; see [`Fact::MalformedField`]).
pub fn malformed_at(field: impl Into<String>, message: impl Into<String>) -> Refusal {
    refuse(codes::ROTATION_REQUEST_MALFORMED, message).with_fact(Fact::MalformedField { field: field.into() })
}

fn malformed(field: &str, message: impl Into<String>) -> Refusal {
    malformed_at(field, message)
}

/// The request digest of KN3.8: the SHA-256 of the raw request bytes, in lowercase hexadecimal.
pub fn request_digest(bytes: &[u8]) -> String {
    c600_digest::sha256_hex(bytes)
}

fn line(form: &str, fields: [&str; 3]) -> Vec<u8> {
    format!("{PREFIX}|{form}|{}|{}|{}\n", fields[0], fields[1], fields[2]).into_bytes()
}

/// The bytes of a C request from three tokens `m:k`. Nothing is checked here; the decoder checks every byte.
pub fn cayley_request(omega: [&str; 3]) -> Vec<u8> {
    line("C", omega)
}

/// The bytes of an H request from three tokens `x:0`. Nothing is checked here.
pub fn half_turn_request(axis: [&str; 3]) -> Vec<u8> {
    line("H", axis)
}

/// The bytes of an M request. Nothing is checked here.
pub fn menu_request(set: &str, order: &str, index: u64) -> Vec<u8> {
    line("M", [set, order, &index.to_string()])
}

/// The token `m:k` of a binary64 value read from its bit pattern (KN3.9's producer, kernel half).
///
/// The value is read exactly: m/2^k with m odd when k > 0, both zeros as `0:0`. A non-finite pattern is refused.
pub fn token_of_binary64(bits: u64) -> Result<String, Refusal> {
    let negative = bits >> 63 == 1;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    if exponent == 0x7ff {
        return Err(malformed("value", "a non-finite binary64 value has no exact request token"));
    }
    if exponent == 0 && fraction == 0 {
        return Ok("0:0".to_owned());
    }
    let (mut m, mut e) = if exponent == 0 { (fraction, -1074) } else { (fraction | 1 << 52, exponent - 1075) };
    if e < 0 {
        let shift = (m.trailing_zeros() as i32).min(-e);
        m >>= shift;
        e += shift;
    }
    let sign = if negative { "-" } else { "" };
    if e < 0 {
        return Ok(format!("{sign}{m}:{}", -e));
    }
    Ok(format!("{sign}{}:0", decimal_shifted(m, e as u32)))
}

/// m·2^e in decimal, by doubling base-10^9 digits (least significant first).
fn decimal_shifted(m: u64, e: u32) -> String {
    const BASE: u64 = 1_000_000_000;
    let mut digits = vec![(m % BASE) as u32, ((m / BASE) % BASE) as u32, (m / BASE / BASE) as u32];
    let mut left = e;
    while left > 0 {
        let step = left.min(29);
        let mut carry = 0u64;
        for digit in digits.iter_mut() {
            let value = ((*digit as u64) << step) + carry;
            *digit = (value % BASE) as u32;
            carry = value / BASE;
        }
        while carry > 0 {
            digits.push((carry % BASE) as u32);
            carry /= BASE;
        }
        left -= step;
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let mut out = digits.last().expect("at least one digit").to_string();
    for digit in digits.iter().rev().skip(1) {
        out.push_str(&format!("{digit:09}"));
    }
    out
}

/// The operation a request carries (KN3.7): free with bounds N and D, menu direct or menu drag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Free { n: u128, d: u128 },
    MenuDirect,
    MenuDrag,
}

/// The pinned menu's two digests, as a binding names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuPin {
    pub set: String,
    pub order: String,
}

/// KN3.7's binding: `{request, cap, operation, menu}`, named arguments that LI.5's request digest covers and the
/// journal records. `menu` is `None` exactly for free mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub request: Vec<u8>,
    pub cap: u32,
    pub operation: Operation,
    pub menu: Option<MenuPin>,
}

fn nat(value: u128) -> Value {
    match i64::try_from(value) {
        Ok(v) => Value::Int(v),
        Err(_) => Value::Number(value.to_string()),
    }
}

fn read_nat(value: Option<&Value>, name: &'static str) -> Result<u128, Refusal> {
    let field = format!("binding.operation.{name}");
    match value {
        Some(Value::Int(v)) if *v >= 0 => Ok(*v as u128),
        Some(Value::Number(lexeme)) if !lexeme.is_empty() && lexeme.bytes().all(|c| c.is_ascii_digit()) => {
            if lexeme.len() > 1 && lexeme.starts_with('0') {
                return Err(malformed(&field, format!("{name} is not a canonical Nat")));
            }
            // A Nat above u128 is outside every search limit.
            lexeme.parse::<u128>().map_err(|_| {
                refuse(codes::ROTATION_SEARCH_BOUND, format!("{name} is outside the search limit; no search ran"))
                    .with_fact(Fact::SearchBoundNat { field: name, digits: lexeme.len() })
            })
        }
        _ => Err(malformed(&field, format!("the binding needs {name} as a Nat"))),
    }
}

impl Binding {
    /// The FP.5 object `{request:Text,cap:integer,operation:{...},menu:null|{set:Text,order:Text}}`.
    pub fn to_json(&self) -> Value {
        let operation = match &self.operation {
            Operation::Free { n, d } => {
                crate::json::object([("kind", Value::String("free".into())), ("n", nat(*n)), ("d", nat(*d))])
            }
            Operation::MenuDirect => crate::json::object([("kind", Value::String("menu_direct".into()))]),
            Operation::MenuDrag => crate::json::object([("kind", Value::String("menu_drag".into()))]),
        };
        let menu = match &self.menu {
            None => Value::Null,
            Some(pin) => crate::json::object([
                ("set", Value::String(pin.set.clone())),
                ("order", Value::String(pin.order.clone())),
            ]),
        };
        crate::json::object([
            ("request", Value::String(String::from_utf8_lossy(&self.request).into_owned())),
            ("cap", Value::Int(self.cap as i64)),
            ("operation", operation),
            ("menu", menu),
        ])
    }

    /// The canonical text of [`Binding::to_json`] (sorted keys, no spaces).
    pub fn canonical(&self) -> String {
        self.to_json().canonical()
    }

    /// Reads a binding object. The request text is kept as bytes; the decoder checks them.
    pub fn from_json(value: &Value) -> Result<Binding, Refusal> {
        let fields = value.as_object().ok_or_else(|| malformed("binding", "a binding must be an object"))?;
        const KEYS: [&str; 4] = ["cap", "menu", "operation", "request"];
        if fields.len() != KEYS.len() || !KEYS.iter().all(|k| fields.contains_key(*k)) {
            return Err(malformed(
                "binding.fields",
                "a binding has exactly the fields request, cap, operation and menu",
            ));
        }
        let request =
            fields["request"].as_str().ok_or_else(|| malformed("binding.request", "the request must be text"))?;
        let cap = match fields["cap"] {
            Value::Int(c) if (0..=u32::MAX as i64).contains(&c) => c as u32,
            _ => return Err(malformed("binding.cap", "the cap must be a nonnegative integer")),
        };
        let operation = read_operation(&fields["operation"])?;
        let menu = match &fields["menu"] {
            Value::Null => None,
            Value::Object(pin) => Some(read_pin(pin)?),
            _ => return Err(malformed("binding.menu", "the menu must be null or {set, order}")),
        };
        let binding = Binding { request: request.as_bytes().to_vec(), cap, operation, menu };
        binding.check_shape()?;
        Ok(binding)
    }

    /// `menu` is null exactly for free mode (KN3.7).
    pub fn check_shape(&self) -> Result<(), Refusal> {
        match (&self.operation, &self.menu) {
            (Operation::Free { .. }, None) => Ok(()),
            (Operation::MenuDirect | Operation::MenuDrag, Some(_)) => Ok(()),
            (Operation::Free { .. }, Some(_)) => Err(malformed("binding.shape", "a free binding pins no menu")),
            _ => Err(malformed("binding.shape", "a menu operation needs both menu digests")),
        }
    }

    /// The trial search limit of free mode: N ≥ 0, D ≥ 1 and (2N+1)³·D ≤ 2^40, checked before any search.
    pub fn check_search_bound(&self) -> Result<(), Refusal> {
        let Operation::Free { n, d } = self.operation else {
            return Ok(());
        };
        let size = (|| {
            let side = n.checked_mul(2)?.checked_add(1)?;
            side.checked_mul(side)?.checked_mul(side)?.checked_mul(d)
        })();
        match size {
            Some(size) if d >= 1 && size <= SEARCH_LIMIT => Ok(()),
            _ => Err(refuse(
                codes::ROTATION_SEARCH_BOUND,
                format!(
                    "bounds N = {n}, D = {d} are outside the search limit (2N+1)^3*D <= 2^40, D >= 1; no search ran"
                ),
            )
            .with_fact(Fact::SearchBound { n: n.to_string(), d: d.to_string(), limit: SEARCH_LIMIT })),
        }
    }
}

fn read_operation(value: &Value) -> Result<Operation, Refusal> {
    let fields = value.as_object().ok_or_else(|| malformed("binding.operation", "the operation must be an object"))?;
    let kind = fields
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("binding.operation.kind", "the operation needs a kind"))?;
    let expect = |keys: &[&str]| -> Result<(), Refusal> {
        if fields.len() == keys.len() && keys.iter().all(|k| fields.contains_key(*k)) {
            Ok(())
        } else {
            Err(malformed("binding.operation", format!("operation {kind} has the fields {}", keys.join(", "))))
        }
    };
    match kind {
        "free" => {
            expect(&["kind", "n", "d"])?;
            Ok(Operation::Free { n: read_nat(fields.get("n"), "n")?, d: read_nat(fields.get("d"), "d")? })
        }
        "menu_direct" => expect(&["kind"]).map(|_| Operation::MenuDirect),
        "menu_drag" => expect(&["kind"]).map(|_| Operation::MenuDrag),
        _ => Err(malformed("binding.operation.kind", format!("unknown operation kind {kind:?}"))),
    }
}

fn read_pin(pin: &BTreeMap<String, Value>) -> Result<MenuPin, Refusal> {
    match (pin.len(), pin.get("set").and_then(Value::as_str), pin.get("order").and_then(Value::as_str)) {
        (2, Some(set), Some(order)) => Ok(MenuPin { set: set.to_owned(), order: order.to_owned() }),
        _ => Err(malformed("binding.menu", "the menu must be {set, order} with two digests")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CT-KN3-06 (part: the kernel half of the producer, which reads binary64 bit patterns exactly, writes both
    /// signed zeros as `0:0` and refuses non-finite patterns; not run: the shell's gesture producer, which passes a
    /// value's bit pattern and lives in the shell).
    #[test]
    fn binary64_bit_patterns_become_exact_tokens() {
        let token = |bits: u64| token_of_binary64(bits).expect("a finite value");
        assert_eq!(token(0x0000_0000_0000_0000), "0:0", "+0");
        assert_eq!(token(0x8000_0000_0000_0000), "0:0", "-0");
        assert_eq!(token(0x3ff0_0000_0000_0000), "1:0", "1");
        assert_eq!(token(0xbff0_0000_0000_0000), "-1:0", "-1");
        assert_eq!(token(0x3fe0_0000_0000_0000), "1:1", "1/2");
        assert_eq!(token(0x3fd8_0000_0000_0000), "3:3", "3/8");
        assert_eq!(token(0xc018_0000_0000_0000), "-6:0", "-6");
        assert_eq!(token(0x3fb9_9999_9999_999a), "3602879701896397:55", "the binary64 value nearest 1/10");
        assert_eq!(token(0x0000_0000_0000_0001), "1:1074", "the smallest subnormal");
        assert_eq!(token(0x8000_0000_0000_0001), "-1:1074", "minus the smallest subnormal");
        assert_eq!(token(0x000f_ffff_ffff_ffff), "4503599627370495:1074", "the largest subnormal");
        assert_eq!(token(0x0010_0000_0000_0000), "1:1022", "the smallest normal");
        assert_eq!(token(0x43b0_0000_0000_0000), "1152921504606846976:0", "2^60");
        assert_eq!(token(0x4340_0000_0000_0000), "9007199254740992:0", "2^53");
        assert_eq!(token(0x4340_0000_0000_0001), "9007199254740994:0", "2^53 + 2");
        let max = token(0x7fef_ffff_ffff_ffff);
        assert_eq!(
            max,
            "179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368:0"
        );
        // Infinities, the quiet NaN and a signalling NaN.
        for bits in [0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000, 0x7ff8_0000_0000_0000, 0x7ff0_0000_0000_0001] {
            let refusal = token_of_binary64(bits).expect_err("a non-finite pattern");
            assert_eq!(refusal.code, codes::ROTATION_REQUEST_MALFORMED);
        }
    }

    /// CT-KN3-04 (part: the search limit of free mode refuses before any search, at and just past its edge; not run
    /// here: the search itself, which `c600-jumble`'s `rotation` tests run).
    #[test]
    fn the_search_limit_is_checked_exactly() {
        let free = |n: u128, d: u128| Binding {
            request: cayley_request(["1:1", "0:0", "0:0"]),
            cap: 0,
            operation: Operation::Free { n, d },
            menu: None,
        };
        assert!(free(0, 1).check_search_bound().is_ok());
        assert!(free(1, 16).check_search_bound().is_ok());
        // (2N+1)^3 * D = 2^40 exactly: N = 0, D = 2^40.
        assert!(free(0, 1 << 40).check_search_bound().is_ok());
        for (n, d) in [(0, 0), (0, (1 << 40) + 1), (5000, 1 << 20), (u128::MAX, 1), (1, u128::MAX)] {
            let refusal = free(n, d).check_search_bound().expect_err("outside the limit");
            assert_eq!(refusal.code, codes::ROTATION_SEARCH_BOUND, "N = {n}, D = {d}");
        }
    }

    /// CT-KN3-04 (part: the binding object round-trips through its canonical text, `menu` is null exactly for free,
    /// and a malformed binding is refused; not run here: request decoding, which `c600-jumble` runs).
    #[test]
    fn bindings_round_trip_and_keep_their_shape() {
        let pin = MenuPin { set: "a".repeat(64), order: "b".repeat(64) };
        let bindings = [
            Binding {
                request: cayley_request(["1:1", "0:0", "0:0"]),
                cap: 7,
                operation: Operation::Free { n: 1, d: 16 },
                menu: None,
            },
            Binding {
                request: menu_request(&pin.set, &pin.order, 3),
                cap: 0,
                operation: Operation::MenuDirect,
                menu: Some(pin.clone()),
            },
            Binding {
                request: half_turn_request(["1:0", "0:0", "0:0"]),
                cap: 599,
                operation: Operation::MenuDrag,
                menu: Some(pin.clone()),
            },
        ];
        for binding in &bindings {
            let text = binding.canonical();
            let back = Binding::from_json(&crate::json::parse(text.as_bytes()).expect("JSON")).expect("a binding");
            assert_eq!(&back, binding);
            assert_eq!(back.canonical(), text);
        }
        assert_eq!(
            bindings[0].canonical(),
            "{\"cap\":7,\"menu\":null,\"operation\":{\"d\":16,\"kind\":\"free\",\"n\":1},\"request\":\"LX9B1|C|1:1|0:0|0:0\\n\"}"
        );
        let bad = [
            "{\"cap\":0,\"menu\":{\"set\":\"a\",\"order\":\"b\"},\"operation\":{\"kind\":\"free\",\"n\":1,\"d\":1},\"request\":\"x\"}",
            "{\"cap\":0,\"menu\":null,\"operation\":{\"kind\":\"menu_drag\"},\"request\":\"x\"}",
            "{\"cap\":-1,\"menu\":null,\"operation\":{\"kind\":\"free\",\"n\":1,\"d\":1},\"request\":\"x\"}",
            "{\"cap\":0,\"menu\":null,\"operation\":{\"kind\":\"free\",\"n\":1},\"request\":\"x\"}",
            "{\"cap\":0,\"menu\":null,\"operation\":{\"kind\":\"free\",\"n\":1,\"d\":1},\"request\":\"x\",\"extra\":1}",
        ];
        for text in bad {
            let refusal = Binding::from_json(&crate::json::parse(text.as_bytes()).expect("JSON")).expect_err(text);
            assert_eq!(refusal.code, codes::ROTATION_REQUEST_MALFORMED, "{text}");
        }
        let huge = "{\"cap\":0,\"menu\":null,\"operation\":{\"kind\":\"free\",\"n\":1,\"d\":1000000000000000000000000000000000000000000},\"request\":\"x\"}";
        let refusal = Binding::from_json(&crate::json::parse(huge.as_bytes()).expect("JSON")).expect_err("huge D");
        assert_eq!(refusal.code, codes::ROTATION_SEARCH_BOUND);
    }

    #[test]
    fn request_digest_is_the_sha256_of_the_raw_bytes() {
        let bytes = cayley_request(["1:1", "0:0", "0:0"]);
        assert_eq!(bytes, b"LX9B1|C|1:1|0:0|0:0\n");
        assert_eq!(request_digest(&bytes), c600_digest::sha256_hex(b"LX9B1|C|1:1|0:0|0:0\n"));
        assert_eq!(request_digest(&bytes).len(), 64);
    }
}
