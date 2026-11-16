//! Tokens and literal text: LX-8's shared lexical grammar (`syntax.py`) in the same order of alternatives, with `.`
//! added for field access. Spaces, tabs, newlines and `#` comments separate tokens and are not kept.

use crate::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Str,
    Rat,
    /// A decimal number without a precision marker. It is never a value (LG1.7).
    BadNumber,
    Int,
    Id,
    Punct,
    /// Any other character.
    Bad,
    Eof,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub kind: Kind,
    pub text: String,
    pub span: Span,
}

const PUNCT2: [&str; 5] = ["->", "=>", ":=", "<-", "|>"];
const PUNCT1: &[u8] = b"{}()[],;:<>!=@~|.";

/// The tokens of pinned source text, ending with an end-of-file token.
pub(crate) fn lex(source: &str) -> Vec<Token> {
    let b = source.as_bytes();
    let mut tokens = Vec::new();
    let (mut i, mut line, mut column) = (0, 1, 1);
    // Where the last scan for an unclosed string stopped. Every quote before that point was escaped in that scan, so
    // its own scan would resume in step and stop there too: it is not scanned again, which keeps lexing linear.
    let mut unclosed = 0;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' => {
                column += 1;
                i += 1;
            }
            b'\n' => {
                line += 1;
                column = 1;
                i += 1;
            }
            b'#' => {
                let end = b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |n| i + n);
                column += source[i..end].chars().count();
                i = end;
            }
            _ => {
                let (kind, end) = scan(source, i, &mut unclosed);
                let text = &source[i..end];
                tokens.push(Token { kind, text: text.to_string(), span: Span { start: i, end, line, column } });
                column += text.chars().count();
                i = end;
            }
        }
    }
    tokens.push(Token { kind: Kind::Eof, text: String::new(), span: Span { start: i, end: i, line, column } });
    tokens
}

/// The kind and end of the token at `i`, trying the alternatives in LX-8's order. A quote before `unclosed` cannot
/// start a string (see `lex`).
fn scan(s: &str, i: usize, unclosed: &mut usize) -> (Kind, usize) {
    let b = s.as_bytes();
    if b[i] == b'"' && i >= *unclosed {
        match string_end(b, i) {
            Ok(end) => return (Kind::Str, end),
            Err(stop) => *unclosed = stop,
        }
    }
    if let Some(n) = signed_digits(b, i) {
        if b.get(n) == Some(&b'/') {
            if let Some(d) = signed_digits(b, n + 1) {
                return (Kind::Rat, d);
            }
        }
        if b.get(n) == Some(&b'.') {
            let fraction = digits(b, n + 1);
            let mut end = fraction;
            if matches!(b.get(fraction), Some(b'e' | b'E')) {
                if let Some(exponent) = signed_digits(b, fraction + 1) {
                    end = exponent;
                }
            }
            return (Kind::BadNumber, end);
        }
        return (Kind::Int, n);
    }
    if b[i].is_ascii_alphabetic() || b[i] == b'_' {
        let mut j = i + 1;
        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
            j += 1;
        }
        return (Kind::Id, j);
    }
    if PUNCT2.iter().any(|p| s[i..].starts_with(p)) {
        return (Kind::Punct, i + 2);
    }
    if PUNCT1.contains(&b[i]) {
        return (Kind::Punct, i + 1);
    }
    (Kind::Bad, i + s[i..].chars().next().map_or(1, char::len_utf8))
}

fn digits(b: &[u8], from: usize) -> usize {
    let mut j = from;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    j
}

/// The end of `[+-]?[0-9]+` at `from`.
fn signed_digits(b: &[u8], from: usize) -> Option<usize> {
    let start = if matches!(b.get(from), Some(b'+' | b'-')) { from + 1 } else { from };
    let end = digits(b, start);
    (end > start).then_some(end)
}

/// The end of a quoted string on one line, with any character escaped by a backslash; or, when the line ends first,
/// a backslash ends it or the text ends, `Err` with where the scan stopped.
fn string_end(b: &[u8], i: usize) -> Result<usize, usize> {
    let mut j = i + 1;
    let end = loop {
        match b.get(j) {
            Some(b'"') => break Ok(j + 1),
            Some(b'\\') if b.get(j + 1).is_some_and(|&c| c != b'\n') => j += 2,
            Some(b'\n' | b'\\') | None => break Err(j),
            Some(_) => j += 1,
        }
    };
    #[cfg(test)]
    tests::EXAMINED.with(|n| n.set(n.get() + (j - i)));
    end
}

/// The value of a string token under JSON's escapes: no raw control character and no lone surrogate.
pub(crate) fn unescape(token: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = token[1..token.len() - 1].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let high = hex4(&mut chars)?;
                    let code = match high {
                        0xd800..=0xdbff => {
                            if chars.next()? != '\\' || chars.next()? != 'u' {
                                return None;
                            }
                            let low = hex4(&mut chars)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return None;
                            }
                            0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                        }
                        0xdc00..=0xdfff => return None,
                        _ => high,
                    };
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            },
            c if (c as u32) < 0x20 => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

fn hex4(chars: &mut std::str::Chars) -> Option<u32> {
    let mut value = 0;
    for _ in 0..4 {
        value = value * 16 + chars.next()?.to_digit(16)?;
    }
    Some(value)
}

/// A string in the canonical escaped form: JSON's short escapes, `\u00XX` for other control characters, and every
/// other character as itself.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The canonical decimal text of `[+-]?[0-9]+`: no leading zeros, and a sign only when negative.
pub(crate) fn canonical(text: &str) -> String {
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        "0".into()
    } else if negative {
        format!("-{digits}")
    } else {
        digits.into()
    }
}

/// A ratio token `n/d` in canonical form with the sign on the numerator, or `None` for a zero denominator. The ratio
/// is not reduced: that needs the interpreter's big integers (M2).
pub(crate) fn ratio(text: &str) -> Option<(String, String)> {
    let (n, d) = text.split_once('/')?;
    let (n, d) = (canonical(n), canonical(d));
    if d == "0" {
        None
    } else if let Some(magnitude) = d.strip_prefix('-') {
        Some((negate(&n), magnitude.to_string()))
    } else {
        Some((n, d))
    }
}

fn negate(n: &str) -> String {
    match n.strip_prefix('-') {
        Some(m) => m.to_string(),
        None if n == "0" => n.to_string(),
        None => format!("-{n}"),
    }
}

/// Whether `s` is a finite decimal number: `[+-]?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
pub(crate) fn is_decimal(s: &str) -> bool {
    let b = s.as_bytes();
    let Some(mut i) = signed_digits(b, 0) else { return false };
    if b.get(i) == Some(&b'.') {
        let end = digits(b, i + 1);
        if end == i + 1 {
            return false;
        }
        i = end;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        match signed_digits(b, i + 1) {
            Some(end) => i = end,
            None => return false,
        }
    }
    i == b.len()
}

/// Whether `s` is a SHA-256 in lowercase hexadecimal.
pub(crate) fn is_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        /// The bytes `string_end` has stepped over on this thread.
        pub(super) static EXAMINED: Cell<usize> = const { Cell::new(0) };
    }

    fn kinds(s: &str) -> Vec<(Kind, String)> {
        lex(s).into_iter().map(|t| (t.kind, t.text)).collect()
    }

    /// M7 lead L5: a quote and then escaped quotes with no closing one used to scan to the line's end from every quote.
    #[test]
    fn unclosed_strings_lex_in_linear_time() {
        use Kind::*;
        for n in [10, 100_000, 1_000_000] {
            let line = format!("\"{}", "\\\"".repeat(n / 2));
            EXAMINED.with(|e| e.set(0));
            let toks = lex(&line);
            assert_eq!(toks.len(), line.len() + 1);
            assert!(toks[..line.len()].iter().all(|t| t.kind == Bad && t.text.len() == 1), "{n}");
            assert!(EXAMINED.with(Cell::get) <= line.len(), "{n}");
        }
        // The skip ends with the line: a later line's string still closes.
        let toks: Vec<_> = kinds("\"a\\\"b\n\"c\"").into_iter().map(|(k, _)| k).collect();
        assert_eq!(toks, [Bad, Id, Bad, Bad, Id, Str, Eof]);
    }

    /// Why `lex` may skip them: a quote before an unclosed scan's stop does not close either, and stops at the same
    /// place. Checked on every text of up to eight bytes over a quote, a backslash, a letter and a newline.
    #[test]
    fn quotes_inside_an_unclosed_string_stop_at_the_same_place() {
        let alphabet = b"\"\\a\n";
        for len in 0..=8 {
            for code in 0..4usize.pow(len) {
                let text: Vec<u8> = (0..len).map(|d| alphabet[code / 4usize.pow(d) % 4]).collect();
                for i in (0..text.len()).filter(|&i| text[i] == b'"') {
                    if let Err(stop) = string_end(&text, i) {
                        for k in (i + 1..stop).filter(|&k| text[k] == b'"') {
                            assert_eq!(string_end(&text, k), Err(stop), "{text:?} {i} {k}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn alternatives_follow_lx8_order() {
        use Kind::*;
        let toks = kinds("1/2 1/x 1.5e 1.5e3 +-1 -> <- x.y \"a\\\"b\" \"open $");
        let expected = [
            (Rat, "1/2"),
            (Int, "1"),
            (Bad, "/"),
            (Id, "x"),
            (BadNumber, "1.5"),
            (Id, "e"),
            (BadNumber, "1.5e3"),
            (Bad, "+"),
            (Int, "-1"),
            (Punct, "->"),
            (Punct, "<-"),
            (Id, "x"),
            (Punct, "."),
            (Id, "y"),
            (Str, "\"a\\\"b\""),
            (Bad, "\""),
            (Id, "open"),
            (Bad, "$"),
            (Eof, ""),
        ];
        assert_eq!(toks, expected.iter().map(|(k, t)| (*k, t.to_string())).collect::<Vec<_>>());
    }

    #[test]
    fn columns_count_scalars() {
        let toks = lex("# λ\n\"λ\" x\n  y");
        assert_eq!((toks[0].span.line, toks[0].span.column, toks[0].span.start), (2, 1, 5));
        assert_eq!((toks[1].span.line, toks[1].span.column), (2, 5));
        assert_eq!((toks[2].span.line, toks[2].span.column), (3, 3));
    }

    #[test]
    fn literal_text() {
        assert_eq!(canonical("+007"), "7");
        assert_eq!(canonical("-0"), "0");
        assert_eq!(ratio("6/-8"), Some(("-6".into(), "8".into())));
        assert_eq!(ratio("-0/-5"), Some(("0".into(), "5".into())));
        assert_eq!(ratio("1/-00"), None);
        assert!(is_decimal("-1.25e-1") && is_decimal("7") && !is_decimal("1.") && !is_decimal("NaN"));
        assert_eq!(unescape("\"\\ud83d\\ude00\\u00e9\""), Some("\u{1f600}\u{e9}".into()));
        assert_eq!(unescape("\"\\ud800\""), None);
        assert_eq!(unescape("\"\\q\""), None);
        assert_eq!(unescape("\"a\tb\""), None);
        assert_eq!(quote("a\"\\\n\u{1}λ"), "\"a\\\"\\\\\\n\\u0001λ\"");
    }
}
