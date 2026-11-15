// Minimal exact JSON transport for this experiment's strings, objects and counts.
use std::collections::BTreeMap;
use std::ops::Index;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum J { Null, Bool(bool), Num(u64), Str(String), Arr(Vec<J>), Obj(BTreeMap<String, J>) }
impl From<&str> for J { fn from(s: &str) -> Self { Self::Str(s.into()) } }
impl From<String> for J { fn from(s: String) -> Self { Self::Str(s) } }
impl From<usize> for J { fn from(n: usize) -> Self { Self::Num(n as u64) } }
impl From<Vec<J>> for J { fn from(v: Vec<J>) -> Self { Self::Arr(v) } }
impl Index<&str> for J {
    type Output = J;
    fn index(&self, key: &str) -> &J { self.obj().get(key).expect(key) }
}
impl J {
    pub fn s(&self) -> &str { if let Self::Str(s) = self { s } else { panic!("expected string") } }
    pub fn a(&self) -> &[J] { if let Self::Arr(a) = self { a } else { panic!("expected array") } }
    pub fn obj(&self) -> &BTreeMap<String, J> { if let Self::Obj(o) = self { o } else { panic!("expected object") } }
    pub fn n(&self) -> usize { if let Self::Num(n) = self { *n as usize } else { panic!("expected count") } }
    pub fn set(&mut self, key: &str, value: J) {
        if let Self::Obj(o) = self { o.insert(key.into(), value); } else { panic!("expected object") }
    }
    pub fn push(&mut self, key: &str, value: J) {
        if let Self::Obj(o) = self {
            if let Some(Self::Arr(a)) = o.get_mut(key) { a.push(value); return; }
        }
        panic!("expected array field")
    }
    pub fn has(&self, key: &str) -> bool { self.obj().contains_key(key) }
}
pub fn obj(fields: &[(&str, J)]) -> J {
    J::Obj(fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}
pub fn strings(values: &[&str]) -> J { J::Arr(values.iter().map(|s| J::from(*s)).collect()) }
pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"'); out
}
pub fn dump(j: &J) -> String {
    match j {
        J::Null => "null".into(), J::Bool(b) => b.to_string(), J::Num(n) => n.to_string(),
        J::Str(s) => quote(s),
        J::Arr(a) => format!("[{}]", a.iter().map(dump).collect::<Vec<_>>().join(",")),
        J::Obj(o) => format!("{{{}}}", o.iter().map(|(k, v)| format!("{}:{}", quote(k), dump(v))).collect::<Vec<_>>().join(",")),
    }
}
pub fn parse(s: &str) -> Result<J, String> {
    struct Cursor<'a> { s: &'a [u8], pos: usize }
    impl Cursor<'_> {
        fn ws(&mut self) { while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() { self.pos += 1; } }
        fn take(&mut self) -> Result<u8, String> {
            let c = *self.s.get(self.pos).ok_or("unexpected JSON end")?; self.pos += 1; Ok(c)
        }
        fn hex(&mut self) -> Result<u32, String> {
            let mut n = 0;
            for _ in 0..4 { n = n * 16 + (self.take()? as char).to_digit(16).ok_or("bad JSON escape")?; }
            Ok(n)
        }
        fn string(&mut self) -> Result<String, String> {
            let mut bytes = Vec::new();
            loop {
                match self.take()? {
                    b'"' => return String::from_utf8(bytes).map_err(|_| "invalid string UTF-8".into()),
                    b'\\' => match self.take()? {
                        b'"' => bytes.push(b'"'), b'\\' => bytes.push(b'\\'), b'/' => bytes.push(b'/'),
                        b'n' => bytes.push(b'\n'), b'r' => bytes.push(b'\r'), b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(8), b'f' => bytes.push(12),
                        b'u' => {
                            let mut n = self.hex()?;
                            if (0xd800..=0xdbff).contains(&n) {
                                if self.take()? != b'\\' || self.take()? != b'u' { return Err("unpaired surrogate".into()); }
                                let low = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&low) { return Err("unpaired surrogate".into()); }
                                n = 0x10000 + ((n - 0xd800) << 10) + low - 0xdc00;
                            }
                            let c = char::from_u32(n).ok_or("invalid Unicode scalar")?;
                            bytes.extend(c.to_string().as_bytes());
                        }
                        _ => return Err("invalid string escape".into()),
                    },
                    c if c < 32 => return Err("raw control in string".into()),
                    c => bytes.push(c),
                }
            }
        }
        fn value(&mut self) -> Result<J, String> {
            self.ws();
            match self.take()? {
                b'"' => Ok(J::Str(self.string()?)),
                b'[' => {
                    let mut a = vec![]; self.ws();
                    if self.s.get(self.pos) == Some(&b']') { self.pos += 1; return Ok(J::Arr(a)); }
                    loop {
                        a.push(self.value()?); self.ws();
                        match self.take()? { b']' => return Ok(J::Arr(a)), b',' => (), _ => return Err("JSON array delimiter".into()) }
                    }
                }
                b'{' => {
                    let mut o = BTreeMap::new(); self.ws();
                    if self.s.get(self.pos) == Some(&b'}') { self.pos += 1; return Ok(J::Obj(o)); }
                    loop {
                        self.ws(); if self.take()? != b'"' { return Err("JSON object key".into()); }
                        let k = self.string()?; self.ws();
                        if self.take()? != b':' { return Err("JSON colon".into()); }
                        if o.insert(k, self.value()?).is_some() { return Err("duplicate JSON key".into()); }
                        self.ws();
                        match self.take()? { b'}' => return Ok(J::Obj(o)), b',' => (), _ => return Err("JSON object delimiter".into()) }
                    }
                }
                c @ b'0'..=b'9' => {
                    let start = self.pos - 1;
                    while self.s.get(self.pos).is_some_and(u8::is_ascii_digit) { self.pos += 1; }
                    if c == b'0' && self.pos - start > 1 { return Err("leading JSON zero".into()); }
                    Ok(J::Num(std::str::from_utf8(&self.s[start..self.pos]).unwrap().parse().map_err(|_| "count overflow")?))
                }
                c => {
                    let (tail, v) = match c { b'n' => ("ull", J::Null), b't' => ("rue", J::Bool(true)), b'f' => ("alse", J::Bool(false)), _ => return Err("JSON value".into()) };
                    for expected in tail.bytes() { if self.take()? != expected { return Err("JSON keyword".into()); } }
                    Ok(v)
                }
            }
        }
    }
    let mut p = Cursor { s: s.as_bytes(), pos: 0 };
    let value = p.value()?; p.ws();
    if p.pos != p.s.len() { return Err("trailing JSON".into()); }
    Ok(value)
}
