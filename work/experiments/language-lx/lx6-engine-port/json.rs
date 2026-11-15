// Strict JSON reader: numeric lexemes retain integer/float distinctions from Python json.loads.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum J { Null, Bool(bool), Number(String), String(String), Array(Vec<J>), Object(BTreeMap<String,J>) }

impl J {
    pub fn get(&self, key: &str) -> Result<&J, String> {
        match self { J::Object(o) => o.get(key).ok_or_else(|| format!("Missing {key}")),
                     _ => Err("Expected object".into()) }
    }
    pub fn array(&self) -> Result<&[J], String> {
        match self { J::Array(a) => Ok(a), _ => Err("Expected array".into()) }
    }
    pub fn text(&self) -> Result<&str, String> {
        match self { J::String(s) => Ok(s), _ => Err("Expected string".into()) }
    }
    pub fn int(&self) -> Result<i64, String> {
        match self {
            J::Number(s) if !s.contains(['.', 'e', 'E']) => s.parse().map_err(|_| "Integer out of range".into()),
            _ => Err("Expected integer (not float, boolean or string)".into()),
        }
    }
    pub fn canonical(&self) -> String {
        match self {
            J::Null => "null".into(), J::Bool(b) => b.to_string(), J::Number(n) => n.clone(),
            J::String(s) => quote(s),
            J::Array(a) => format!("[{}]", a.iter().map(J::canonical).collect::<Vec<_>>().join(",")),
            J::Object(o) => format!("{{{}}}", o.iter().map(|(k,v)| format!("{}:{}",quote(k),v.canonical())).collect::<Vec<_>>().join(",")),
        }
    }
}

pub fn num(value: impl ToString) -> J { J::Number(value.to_string()) }
pub fn string(value: impl Into<String>) -> J { J::String(value.into()) }
pub fn object<const N: usize>(pairs: [(&str,J);N]) -> J {
    J::Object(pairs.into_iter().map(|(k,v)| (k.to_owned(),v)).collect())
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"), '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
            c if (' '..='~').contains(&c) => out.push(c),
            c => { for unit in c.encode_utf16(&mut [0;2]) { out.push_str(&format!("\\u{unit:04x}")); } }
        }
    }
    out.push('"'); out
}

pub fn parse(bytes: &[u8]) -> Result<J,String> {
    std::str::from_utf8(bytes).map_err(|_| "Non-UTF8 JSON")?;
    let mut p = Parser { bytes, pos: 0 };
    let value = p.value(0)?;
    p.space();
    if p.pos != bytes.len() { return Err("Trailing JSON bytes".into()); }
    Ok(value)
}

struct Parser<'a> { bytes: &'a [u8], pos: usize }
impl Parser<'_> {
    fn peek(&self) -> u8 { *self.bytes.get(self.pos).unwrap_or(&0) }
    fn space(&mut self) { while matches!(self.peek(), b' '|b'\t'|b'\r'|b'\n') { self.pos+=1; } }
    fn take(&mut self, c:u8) -> Result<(),String> {
        if self.peek()!=c { return Err("Malformed JSON".into()); } self.pos+=1; Ok(())
    }
    fn literal(&mut self, s:&[u8], value:J) -> Result<J,String> {
        if !self.bytes[self.pos..].starts_with(s) { return Err("Malformed JSON literal".into()); }
        self.pos+=s.len(); Ok(value)
    }
    fn value(&mut self, depth:usize) -> Result<J,String> {
        if depth>128 { return Err("JSON nesting limit".into()); }
        self.space();
        match self.peek() {
            b'n' => self.literal(b"null",J::Null), b't' => self.literal(b"true",J::Bool(true)),
            b'f' => self.literal(b"false",J::Bool(false)), b'"' => Ok(J::String(self.str()?)),
            b'[' => {
                self.pos+=1; self.space(); let mut a=Vec::new();
                if self.peek()!=b']' { loop {
                    a.push(self.value(depth+1)?); self.space();
                    if self.peek()!=b',' { break; } self.pos+=1;
                }}
                self.take(b']')?; Ok(J::Array(a))
            },
            b'{' => {
                self.pos+=1; self.space(); let mut o=BTreeMap::new();
                if self.peek()!=b'}' { loop {
                    self.space(); let key=self.str()?; self.space(); self.take(b':')?;
                    if o.insert(key,self.value(depth+1)?).is_some() { return Err("Duplicate JSON key".into()); }
                    self.space(); if self.peek()!=b',' { break; } self.pos+=1;
                }}
                self.take(b'}')?; Ok(J::Object(o))
            },
            b'-'|b'0'..=b'9' => self.number(),
            _ => Err("Malformed JSON value".into()),
        }
    }
    fn digits(&mut self) -> Result<(),String> {
        let start=self.pos;
        while self.peek().is_ascii_digit() { self.pos+=1; }
        if self.pos==start { return Err("Missing JSON digits".into()); } Ok(())
    }
    fn number(&mut self) -> Result<J,String> {
        let start=self.pos;
        if self.peek()==b'-' { self.pos+=1; }
        if self.peek()==b'0' { self.pos+=1; } else { self.digits()?; }
        if self.peek()==b'.' { self.pos+=1; self.digits()?; }
        if matches!(self.peek(),b'e'|b'E') {
            self.pos+=1; if matches!(self.peek(),b'+'|b'-') { self.pos+=1; } self.digits()?;
        }
        Ok(J::Number(std::str::from_utf8(&self.bytes[start..self.pos]).unwrap().into()))
    }
    fn hex4(&mut self) -> Result<u32,String> {
        let mut value=0;
        for _ in 0..4 {
            value=value*16+(self.peek() as char).to_digit(16).ok_or("Bad Unicode escape")?; self.pos+=1;
        }
        Ok(value)
    }
    fn str(&mut self) -> Result<String,String> {
        self.take(b'"')?; let mut out=Vec::new();
        loop {
            let c=self.peek(); self.pos+=1;
            match c {
                b'"' => break,
                b'\\' => {
                    let escape=self.peek(); self.pos+=1;
                    match escape {
                        b'"'|b'\\'|b'/' => out.push(escape),
                        b'b' => out.push(8), b'f' => out.push(12), b'n' => out.push(10),
                        b'r' => out.push(13), b't' => out.push(9),
                        b'u' => {
                            let mut cp=self.hex4()?;
                            if (0xd800..=0xdbff).contains(&cp) {
                                self.take(b'\\')?; self.take(b'u')?; let low=self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&low) { return Err("Bad surrogate pair".into()); }
                                cp=0x10000+((cp-0xd800)<<10)+(low-0xdc00);
                            }
                            let ch=char::from_u32(cp).ok_or("Bad Unicode scalar")?;
                            out.extend_from_slice(ch.encode_utf8(&mut [0;4]).as_bytes());
                        },
                        _ => return Err("Bad JSON escape".into()),
                    }
                },
                0..=31 => return Err("Unterminated string or control character".into()),
                _ => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| "Bad UTF8 string".into())
    }
}

pub fn self_test() {
    let values=parse(br#"[1,1.0,1e0,true,"1",-1,null]"#).unwrap();
    let a=values.array().unwrap();
    assert_eq!(a[0].int(),Ok(1)); assert_eq!(a[5].int(),Ok(-1));
    for i in [1,2,3,4,6] { assert!(a[i].int().is_err()); }
    for bad in ["01", "1.", "1e", "NaN", "[1,]", "{\"x\":1,\"x\":2}", "\"\\uD800\""] {
        assert!(parse(bad.as_bytes()).is_err());
    }
    assert_eq!(parse(br#"{"z":"\ud83d\ude00","a":"\n\u00e9"}"#).unwrap().canonical(),
               r#"{"a":"\n\u00e9","z":"\ud83d\ude00"}"#);
}
