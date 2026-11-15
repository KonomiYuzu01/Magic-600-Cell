// Scoped full-model mechanics port. No session, persistence, UI or geometry code.
use crate::json::{self, J, num, object, string};
use crate::sha256;

pub const SLOTS: usize = 259_800;
pub struct Tree { seed:Vec<i32>, relocation:Vec<i32>, parent:Vec<i32>, moves:Vec<i32> }
pub struct Model {
    pub identity:J,
    pub ids:Vec<i32>,
    sp:Vec<i32>, oid:Vec<i32>, first:Vec<usize>,
    offsets:Vec<i32>, source:Vec<i32>, destination:Vec<i32>,
    trees:Vec<Tree>, census:Vec<J>,
}
pub enum Part { Word(Vec<i32>), Star { orbit:usize, path:Vec<i32>, sign:i32 } }
pub type Net = (Vec<i32>,Vec<i32>);

struct Reader<'a> { data:&'a [u8], pos:usize }
impl Reader<'_> {
    fn bytes(&mut self, n:usize) -> Result<&[u8],String> {
        let end=self.pos.checked_add(n).ok_or("Model size overflow")?;
        let bytes=self.data.get(self.pos..end).ok_or("Truncated model")?;
        self.pos=end; Ok(bytes)
    }
    fn count(&mut self) -> Result<usize,String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()) as usize)
    }
    fn vector(&mut self) -> Result<Vec<i32>,String> {
        let n=self.count()?;
        Ok(self.bytes(n.checked_mul(4).ok_or("Model size overflow")?)?
           .chunks_exact(4).map(|b| i32::from_le_bytes(b.try_into().unwrap())).collect())
    }
}

impl Model {
    pub fn load(data:&[u8]) -> Result<Model,String> {
        let mut r=Reader { data, pos:0 };
        if r.bytes(8)?!=b"LX6MOD1\0" { return Err("Wrong model format".into()); }
        let len=r.count()?; let meta=json::parse(r.bytes(len)?)?;
        let sp=r.vector()?; let oid=r.vector()?;
        let offsets=r.vector()?; let source=r.vector()?; let destination=r.vector()?;
        let mut trees=Vec::new();
        for _ in 0..r.count()? {
            trees.push(Tree { seed:r.vector()?, relocation:r.vector()?, parent:r.vector()?, moves:r.vector()? });
        }
        if r.pos!=data.len() || sp.len()!=SLOTS || oid.len()!=177_120 || offsets.len()!=1201
            || source.len()!=destination.len() || offsets[0]!=0
            || offsets[1200] as usize!=source.len() || offsets.windows(2).any(|w| w[0]>w[1])
            || trees.len()!=35 || meta.get("census")?.array()?.len()!=35
            || sp.iter().any(|&p| p<0 || p as usize>=oid.len())
            || oid.iter().any(|&o| !(-1..35).contains(&o))
            || source.iter().chain(&destination).any(|&s| s<0 || s as usize>=SLOTS) {
            return Err("Invalid full-model arrays".into());
        }
        let mut first=vec![usize::MAX;oid.len()];
        for (slot,&p) in sp.iter().enumerate() { first[p as usize]=first[p as usize].min(slot); }
        if first.contains(&usize::MAX) { return Err("Missing physical piece".into()); }
        for t in &trees {
            if t.parent.len()!=t.moves.len() || t.parent.is_empty()
                || t.parent.iter().any(|&p| p < -1 || p as usize>=t.parent.len() && p>=0)
                || t.seed.iter().chain(&t.relocation).any(|&m| !valid_move(m as i64)) {
                return Err("Invalid execution tree".into());
            }
        }
        Ok(Model { identity:object([("model_id",meta.get("model_id")?.clone()),
                                   ("manifest_sha256",meta.get("manifest_sha256")?.clone())]),
                   ids:(0..SLOTS as i32).collect(), sp,oid,first,offsets,source,destination,trees,
                   census:meta.get("census")?.array()?.to_vec() })
    }

    fn path(&self, orbit:usize, node:i64) -> Result<Vec<i32>,String> {
        let t=&self.trees[orbit];
        if node<0 || node as usize>=t.parent.len() { return Err("Frame node out of range".into()); }
        let mut n=node as usize; let mut path=Vec::new();
        while t.parent[n]>=0 {
            if path.len()>=t.parent.len() { return Err("Cyclic execution tree".into()); }
            path.push(t.moves[n]); n=t.parent[n] as usize;
        }
        path.reverse(); Ok(path)
    }

    pub fn normalize(&self, recipe:&J) -> Result<Vec<Part>,String> {
        let steps=recipe.array()?;
        if !(1..=256).contains(&steps.len()) { return Err("Recipe length".into()); }
        let mut out=Vec::new(); let mut length=0usize;
        for step in steps {
            match step.get("kind")?.text()? {
                "word" => {
                    let moves=step.get("moves")?.array()?;
                    if !(1..=100_000).contains(&moves.len()) { return Err("Word length".into()); }
                    let mut word=Vec::with_capacity(moves.len());
                    for m in moves {
                        let m=m.int()?;
                        if !valid_move(m) { return Err("Primitive out of range".into()); }
                        word.push(m as i32);
                    }
                    length+=word.len(); out.push(Part::Word(word));
                },
                "star" => {
                    let o=step.get("orbit")?.int()?;
                    let sign=match step.get("sign") { Ok(s)=>s.int()?, Err(_)=>1 };
                    if !(0..35).contains(&o) || ![-1,1].contains(&sign) { return Err("Bad star fields".into()); }
                    let o=o as usize; let path=self.path(o,step.get("node")?.int()?)?;
                    length+=self.trees[o].seed.len()+2*self.trees[o].relocation.len()+2*path.len();
                    out.push(Part::Star { orbit:o,path,sign:sign as i32 });
                },
                _ => return Err("Only word and star steps are accepted".into()),
            }
        }
        if length>3_000_000 { return Err("Expansion limit".into()); }
        Ok(out)
    }

    pub fn expand(&self, parts:&[Part]) -> Vec<i32> {
        let mut out=Vec::new();
        for part in parts {
            match part {
                Part::Word(w) => out.extend_from_slice(w),
                Part::Star { orbit,path,sign } => {
                    let t=&self.trees[*orbit]; let start=out.len();
                    out.extend(path.iter().rev().map(|m| -*m));
                    out.extend(t.relocation.iter().rev().map(|m| -*m));
                    out.extend_from_slice(&t.seed);
                    out.extend_from_slice(&t.relocation);
                    out.extend_from_slice(path);
                    if *sign<0 {
                        out[start..].reverse();
                        for m in &mut out[start..] { *m = -*m; }
                    }
                },
            }
        }
        out
    }

    pub fn move_arrays(&self, m:i32) -> (&[i32],&[i32]) {
        let i=m.unsigned_abs() as usize-1;
        let range=self.offsets[i] as usize..self.offsets[i+1] as usize;
        let (s,d)=(&self.source[range.clone()],&self.destination[range]);
        // Swapping endpoints is sufficient for simultaneous application; net() sorts by source.
        if m>0 { (s,d) } else { (d,s) }
    }

    pub fn apply_word(&self, labels:&mut [i32], word:&[i32], scratch:&mut Vec<i32>) {
        for &m in word { let (s,d)=self.move_arrays(m); apply(labels,s,d,scratch); }
    }

    pub fn net(&self, recipe:&J) -> Result<Net,String> {
        let word=self.expand(&self.normalize(recipe)?);
        Ok(self.word_net(&word))
    }

    pub fn word_net(&self, word:&[i32]) -> Net {
        let mut map=self.ids.clone();
        self.apply_word(&mut map,word,&mut Vec::new());
        net_from_map(&map)
    }

    fn support(&self, source:&[i32]) -> (J,usize) {
        let mut stickers=[0usize;36]; let mut pieces=[0usize;36];
        let mut seen=vec![false;self.oid.len()];
        for &s in source {
            let p=self.sp[s as usize] as usize; let o=(self.oid[p]+1) as usize;
            stickers[o]+=1;
            if !seen[p] { pieces[o]+=1; seen[p]=true; }
        }
        let rows=(0..36).filter(|&o| stickers[o]>0).map(|o| object([
            ("orbit",num(o as i32-1)),("stickers",num(stickers[o])),("pieces",num(pieces[o])),
        ])).collect();
        (J::Array(rows),pieces.iter().sum())
    }

    pub fn progress(&self, labels:&[i32]) -> J {
        let mut correct=vec![true;self.oid.len()];
        let mut color=correct.clone();
        for (slot,&label) in labels.iter().enumerate() {
            let piece=self.sp[slot] as usize;
            if label as usize!=slot { correct[piece]=false; }
            if label as usize/433!=slot/433 { color[piece]=false; }
        }
        let mut counts=[[0usize;5];35];
        for (p,&o) in self.oid.iter().enumerate() {
            if o<0 { continue; }
            let position=self.sp[labels[self.first[p]] as usize] as usize==p;
            let row=&mut counts[o as usize];
            row[0]+=1; row[1]+=usize::from(correct[p]); row[2]+=usize::from(!position);
            row[3]+=usize::from(position && !correct[p]); row[4]+=usize::from(color[p]);
        }
        J::Array(counts.iter().enumerate().map(|(o,c)| object([
            ("orbit",num(o)),("pieces",num(c[0])),("solved",num(c[1])),
            ("position_wrong",num(c[2])),("orientation_wrong",num(c[3])),("color_solved",num(c[4])),
            ("rank",self.census[o].get("rank").unwrap().clone()),
            ("stickers",self.census[o].get("colors").unwrap().clone()),
            ("orientation",self.census[o].get("orientation_group").unwrap().clone()),
        ])).collect())
    }

    pub fn replay(&self, case:&J) -> Result<J,String> {
        if case.get("format")?.text()? != "C600-ORACLE-CASE-draft" || case.get("model")? != &self.identity {
            return Err("Case format or model identity differs".into());
        }
        let mode=case.get("mode")?.text()?;
        if mode!="independent" && mode!="chain" { return Err("Bad case mode".into()); }
        let initial=case.get("initial")?;
        let mut labels=self.ids.clone(); let mut scratch=Vec::new();
        for m in initial.get("moves")?.array()? {
            let m=m.int()?;
            if !valid_move(m) { return Err("Bad initial primitive".into()); }
            self.apply_word(&mut labels,&[m as i32],&mut scratch);
        }
        if sha256::ints(&labels)!=initial.get("state_hash")?.text()? { return Err("Initial state differs".into()); }
        let initial_labels=labels.clone();
        let protected=case.get("protected")?.array()?.iter().map(J::int).collect::<Result<Vec<_>,_>>()?;
        if protected.iter().any(|o| !(0..35).contains(o)) { return Err("Bad protected orbit".into()); }
        let mut steps=case.get("steps")?.array()?.iter().map(|s| Ok((s.get("index")?.int()?,s)))
            .collect::<Result<Vec<_>,String>>()?;
        steps.sort_by_key(|s| s.0);
        if steps.iter().any(|s| s.0<0) || steps.windows(2).any(|s| s[0].0==s[1].0) { return Err("Bad step index".into()); }
        let mut results=Vec::new();
        for (index,step) in steps {
            if mode=="independent" { labels.copy_from_slice(&initial_labels); }
            let result=match self.normalize(step.get("recipe")?) {
                Err(_) => object([
                    ("accepted",J::Bool(false)),("error",string("invalid-input")),
                    ("net_sha256",J::Null),("net_moved_stickers",J::Null),("net_moved_pieces",J::Null),
                    ("expansion_sha256",J::Null),("primitive_count",J::Null),
                    ("support",J::Null),("conflicts",J::Null),
                ]),
                Ok(parts) => {
                    let word=self.expand(&parts); let (s,d)=self.word_net(&word);
                    let (support,pieces)=self.support(&s);
                    let conflicts:Vec<J>=support.array()?.iter().filter(|row|
                        protected.contains(&row.get("orbit").unwrap().int().unwrap())).cloned().collect();
                    let accepted=conflicts.is_empty();
                    if accepted { apply(&mut labels,&s,&d,&mut scratch); }
                    object([
                        ("accepted",J::Bool(accepted)),("error",if accepted { J::Null } else { string("protected") }),
                        ("net_sha256",string(sha256::pair(&s,&d))),
                        ("net_moved_stickers",num(s.len())),("net_moved_pieces",num(pieces)),
                        ("expansion_sha256",string(sha256::ints(&word))),("primitive_count",num(word.len())),
                        ("support",support),("conflicts",J::Array(conflicts)),
                    ])
                },
            };
            let J::Object(mut result)=result else { unreachable!() };
            result.insert("index".into(),num(index));
            result.insert("state_hash".into(),string(sha256::ints(&labels)));
            result.insert("progress_sha256".into(),string(sha256::hex(self.progress(&labels).canonical().as_bytes())));
            results.push(J::Object(result));
        }
        Ok(object([
            ("format",string("C600-ORACLE-TRACE-draft")),("case_id",case.get("case_id")?.clone()),
            ("model",self.identity.clone()),("candidate",object([
                ("name",string("lx6-rust-primitive-port")),("build",string(env!("LX6_BUILD"))),
            ])),("steps",J::Array(results)),
        ]))
    }

    pub fn self_test(&self) {
        // Type preservation, default sign and complete validation at the 0.4 limits.
        for moves in ["[true]","[\"1\"]","[1.0]","[1e0]","[1,0]","[]","[9223372036854775808]"] {
            let recipe=json::parse(format!(r#"[{{"kind":"word","moves":{moves}}}]"#).as_bytes()).unwrap();
            assert!(self.normalize(&recipe).is_err());
        }
        for fields in [r#""orbit":true,"node":0"#,r#""orbit":0,"node":false"#,
                       r#""orbit":0,"node":0,"sign":true"#,r#""orbit":0,"node":0,"sign":1.0"#,
                       r#""orbit":0,"node":-1"#] {
            let recipe=json::parse(format!(r#"[{{"kind":"star",{fields}}}]"#).as_bytes()).unwrap();
            assert!(self.normalize(&recipe).is_err());
        }
        let star=json::parse(br#"[{"kind":"star","orbit":0,"node":0}]"#).unwrap();
        assert!(self.normalize(&star).is_ok());
        let word=object([("kind",string("word")),("moves",J::Array(vec![num(1)]))]);
        assert!(self.normalize(&J::Array(vec![word.clone();256])).is_ok());
        assert!(self.normalize(&J::Array(vec![word;257])).is_err());
        let long=object([("kind",string("word")),("moves",J::Array(vec![num(1);100_000]))]);
        assert!(self.normalize(&J::Array(vec![long.clone();30])).is_ok());
        assert!(self.normalize(&J::Array(vec![long;31])).is_err());
        let too_long=object([("kind",string("word")),("moves",J::Array(vec![num(1);100_001]))]);
        assert!(self.normalize(&J::Array(vec![too_long])).is_err());
        assert!(self.word_net(&[1,-1]).0.is_empty());
        let mut labels=self.ids.clone();
        let (s,d)=self.move_arrays(1); apply(&mut labels,s,d,&mut Vec::new());
        let (s,d)=self.move_arrays(-1); apply(&mut labels,s,d,&mut Vec::new());
        assert_eq!(labels,self.ids);
    }
}

fn valid_move(m:i64) -> bool { (1..=1200).contains(&m) || (-1200..=-1).contains(&m) }

pub fn apply(labels:&mut [i32], source:&[i32], destination:&[i32], scratch:&mut Vec<i32>) {
    scratch.clear();
    scratch.extend(source.iter().map(|&s| labels[s as usize]));
    for (&d,&value) in destination.iter().zip(scratch.iter()) { labels[d as usize]=value; }
}

pub fn net_from_map(map:&[i32]) -> Net {
    // map[d] = s. Invert only moved entries, then enumerate in ascending source order.
    let mut by_source=vec![-1;map.len()];
    for (d,&s) in map.iter().enumerate() { if s as usize!=d { by_source[s as usize]=d as i32; } }
    let mut source=Vec::new(); let mut destination=Vec::new();
    for (s,&d) in by_source.iter().enumerate() {
        if d>=0 { source.push(s as i32); destination.push(d); }
    }
    (source,destination)
}
