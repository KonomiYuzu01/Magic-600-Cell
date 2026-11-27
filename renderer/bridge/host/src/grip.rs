//! ADR-D01.75: the retained corner action and the pinned menu share one proper grip frame.

use c600_input::TWIST_KEYS;
use c600_jumble::model::POLES;
use c600_jumble::q5::{dot, identity4, matmul, matvec, transpose, Mat4, Vec4, Q5};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::json::{self, Value};

struct Corner {
    piece: usize,
    point: Vec4,
}

// A retained vertex is incident to the twenty sticker hosts of its orbit-34 piece. Their pole sum is along the
// vertex ray, which meets the cap's facet at n_cap·x = |n_cap|². This is the same incidence used by 0.4.
fn corners(exact: &Context, cap: usize) -> Result<[Corner; 4], String> {
    let model = &exact.model;
    model
        .orbit_id
        .iter()
        .enumerate()
        .filter(|&(piece, orbit)| *orbit == 34 && model.hosts(piece).contains(&(cap as u16)))
        .map(|(piece, _)| {
            let sum = std::array::from_fn(|i| {
                model.hosts(piece).iter().fold(Q5::zero(), |s, &c| s.add(&model.poles[c as usize][i]))
            });
            let scale = model.nn.div(&dot(&sum, &model.poles[cap])).expect("a vertex is on the cap's facet");
            Corner { piece, point: sum.map(|x| x.mul(&scale)) }
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "The cap needs four retained corners".to_owned())
}

fn action(corners: &[&Corner; 4], matrix: &Mat4) -> Option<[usize; 4]> {
    let mut targets = [0; 4];
    for (i, corner) in corners.iter().enumerate() {
        let image = matvec(matrix, &corner.point);
        targets[i] = corners.iter().position(|c| c.point == image)?;
    }
    Some(targets)
}

// Port of grip_frames._table: T fixes mark 4, H takes it to mark 1, T then gives marks 3 and 2.
fn base_order(exact: &Context) -> Result<[Corner; 4], String> {
    let corners = corners(exact, 0)?;
    let refs = std::array::from_fn(|i| &corners[i]);
    let [h, t] = [0, 1].map(|i| action(&refs, exact.kplus.matrix(exact.kplus.gen_idx[i], &exact.model)));
    if let (Some(h), Some(t)) = (h, t) {
        if let Some(d) = (0..4).find(|&i| t[i] == i) {
            let a = h[d];
            let c = t[a];
            let b = t[c];
            if [a, b, c, d].into_iter().collect::<std::collections::BTreeSet<_>>().len() == 4
                && h[a] == d
                && h[b] == c
                && h[c] == b
                && t[b] == a
            {
                let mut corners = corners.map(Some);
                return Ok([a, b, c, d].map(|i| corners[i].take().unwrap()));
            }
        }
    }
    Err("The retained H/T actions do not order the corners".into())
}

// Port of grip_frames.notation, using the same mark numbers the views draw.
fn meaning(corners: &[&Corner; 4], matrix: &Mat4) -> String {
    if let Some(permutation) = action(corners, matrix) {
        let mut used = [false; 4];
        let mut label = String::new();
        for start in 0..4 {
            if used[start] || permutation[start] == start {
                continue;
            }
            label.push('(');
            let mut current = start;
            while !used[current] {
                label.push(b"1234"[current] as char);
                used[current] = true;
                current = permutation[current];
            }
            label.push(')');
        }
        return if label.is_empty() { "()".into() } else { label };
    }
    // A cap rotation is in SO(3). A finite order n has real cyclotomic degree φ(n)/2 at most two over Q.
    let identity = identity4();
    let mut power = identity.clone();
    for order in 1..=12 {
        power = matmul(matrix, &power);
        if power == identity {
            return format!("order {order}");
        }
    }
    "infinite order".into()
}

pub(crate) struct KeyMeaning {
    pub chord: String,
    pub item: u64,
    meaning: String,
}

pub(crate) struct Frame {
    marks: Value,
    pub keys: Vec<KeyMeaning>,
}

pub(crate) struct Grip {
    pub cap: usize,
    pub selected: usize,
    frames: Vec<Result<Frame, String>>,
}

impl Grip {
    pub fn build(exact: &Context, menu: &Menu, cap: usize) -> Result<Self, String> {
        if cap >= POLES {
            return Err("Caps are 0 to 599".into());
        }
        let transport = exact.kplus.matrix(exact.kplus.frame_idx[cap], &exact.model);
        let base = base_order(exact)?;
        let there = corners(exact, cap)?;
        let ordered = base
            .iter()
            .map(|c| {
                let point = matvec(transport, &c.point);
                there.iter().find(|c| c.point == point).ok_or("The retained frame misses a cap corner")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let ordered: [&Corner; 4] = ordered.try_into().ok().expect("four corners");
        let twists = menu.for_cap(exact, cap).map_err(|e| e.to_string())?;
        let frames = exact
            .kplus
            .a4(cap)
            .into_iter()
            .map(|k| {
                let rotation = exact.kplus.matrix(k, &exact.model);
                Self::frame(&ordered, &twists, rotation)
            })
            .collect();
        Ok(Self { cap, selected: 0, frames })
    }

    fn frame(base: &[&Corner; 4], twists: &[Twist], rotation: &Mat4) -> Result<Frame, String> {
        let item_of = |matrix: &Mat4| twists.iter().position(|twist| twist.matrix.as_ref() == matrix);
        if item_of(rotation).is_none() {
            return Err("Frame rotation is outside the menu".into());
        }
        let marks = action(base, rotation).ok_or("Frame rotation misses a cap corner")?.map(|i| base[i]);
        let inverse = transpose(rotation);
        let mut keys = Vec::new();
        for (key, forward, backward) in TWIST_KEYS {
            for (chord, item) in [(key.to_owned(), forward), (format!("Shift+{key}"), backward)] {
                let matrix = twists.get(item as usize).ok_or_else(|| format!("Menu omits {chord}"))?;
                let conjugate = matmul(&matmul(rotation, &matrix.matrix), &inverse);
                let item = item_of(&conjugate).ok_or_else(|| format!("Menu omits the {chord} conjugate"))? as u64;
                keys.push(KeyMeaning { chord, item, meaning: meaning(&marks, &conjugate) });
            }
        }
        let marks = Value::Array(
            marks
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    json::object([
                        ("index", Value::Int((i + 1) as i64)),
                        ("label", Value::String((i + 1).to_string())),
                        ("corner", Value::Int(c.piece as i64)),
                        ("point", Value::Array(c.point.iter().map(Q5::to_json).collect())),
                    ])
                })
                .collect(),
        );
        Ok(Frame { marks, keys })
    }

    pub fn at(&self, frame: usize) -> Result<&Frame, String> {
        self.frames.get(frame).ok_or("Frames are 0 to 11")?.as_ref().map_err(Clone::clone)
    }

    pub fn frames_json(&self) -> Value {
        Value::Array(
            self.frames
                .iter()
                .enumerate()
                .map(|(frame, data)| {
                    let index = Value::Int(frame as i64);
                    match data {
                        Ok(data) => json::object([
                            ("frame", index),
                            ("marks", data.marks.clone()),
                            (
                                "keys",
                                Value::Array(
                                    data.keys
                                        .iter()
                                        .map(|key| {
                                            json::object([
                                                ("chord", Value::String(key.chord.clone())),
                                                ("item", Value::Int(key.item as i64)),
                                                ("meaning", Value::String(key.meaning.clone())),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ]),
                        Err(reason) => json::object([("frame", index), ("reason", Value::String(reason.clone()))]),
                    }
                })
                .collect(),
        )
    }
}
