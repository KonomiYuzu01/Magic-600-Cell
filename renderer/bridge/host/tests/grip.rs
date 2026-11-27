use std::fs;
use std::path::Path;
use std::sync::Arc;

use c600_jumble::q5::{matmul, transpose};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::menu_request;
use c600_kernel::Kernel;
use c600_shell_host::turns::{Press, Puzzle, Turns};

fn number(row: &Value, field: &str) -> usize {
    row.get(field).unwrap().as_i64().unwrap() as usize
}

fn rows<'a>(row: &'a Value, field: &str) -> &'a [Value] {
    row.get(field).unwrap().as_array().unwrap()
}

fn corners(frame: &Value) -> Vec<usize> {
    rows(frame, "marks").iter().map(|mark| number(mark, "corner")).collect()
}

#[test]
fn exact_grip_frames_rebind_the_router_without_writing_the_journal() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let assets = root.join("assets");
    let menu_file = root.join("research/jumbling/fixtures/wj-I_a.json");
    let menu_record = json::parse(&fs::read(&menu_file).unwrap()).unwrap();
    let exact = Arc::new(Context::load(&assets).unwrap());
    let menu = Menu::from_record(&exact, menu_record.get("menu").unwrap()).unwrap();
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("grip-{}", std::process::id()));
    let kernel = Arc::new(Kernel::load(&assets).unwrap());
    let mut turns = Turns::open_with(kernel, || Ok(exact.clone()), &folder, &menu_file, Puzzle::JumbleOn).unwrap();
    let initial = turns.grip_report().unwrap();
    let journal = fs::read(folder.join("journal")).unwrap();
    assert_eq!((number(&initial, "cap"), number(&initial, "frame")), (0, 0));
    let pinned = [("KeyJ", 1), ("Shift+KeyJ", 1), ("KeyK", 2), ("Shift+KeyK", 5), ("KeyL", 3), ("Shift+KeyL", 9)];
    let base_keys = rows(&rows(&initial, "frames")[0], "keys");
    let actual: Vec<_> =
        base_keys.iter().map(|row| (row.get("chord").unwrap().as_str().unwrap(), number(row, "item"))).collect();
    assert_eq!(actual, pinned);

    let mut chosen = None;
    for cap in [0, 91, 599, 150] {
        turns.set_grip(cap, 0).unwrap();
        let report = turns.grip_report().unwrap();
        let frames = rows(&report, "frames");
        let base = corners(&frames[0]);
        let a4 = exact.kplus.a4(cap);
        let twists = menu.for_cap(&exact, cap).unwrap();
        let mut orders = std::collections::BTreeSet::new();
        assert_eq!(frames.len(), 12);
        for (frame, row) in frames.iter().enumerate() {
            assert_eq!(number(row, "frame"), frame);
            assert!(row.get("reason").is_none(), "cap {cap}, frame {frame}: {row:?}");
            let order = corners(row);
            assert!(orders.insert(order.clone()));
            let permutation: Vec<_> = order.iter().map(|c| base.iter().position(|b| c == b).unwrap()).collect();
            let inversions = (0..4)
                .flat_map(|i| (i + 1..4).map(move |j| (i, j)))
                .filter(|&(i, j)| permutation[i] > permutation[j])
                .count();
            assert_eq!(inversions % 2, 0);
            let g = exact.kplus.matrix(a4[frame], &exact.model);
            let keys = rows(row, "keys");
            for (key, (chord, base_item)) in keys.iter().zip(pinned) {
                assert_eq!(key.get("chord").and_then(Value::as_str), Some(chord));
                let wanted = matmul(&matmul(g, &twists[base_item].matrix), &transpose(g));
                assert_eq!(twists[number(key, "item")].matrix.as_ref(), &wanted);
                assert_eq!(
                    key.get("meaning"),
                    base_keys.iter().find(|r| r.get("chord") == key.get("chord")).unwrap().get("meaning")
                );
            }
            let j = number(&keys[0], "item");
            if cap == 150 && j != 1 {
                chosen = Some((frame, j));
            }
        }
        assert_eq!(orders.len(), 12);
    }

    let (frame, item) = chosen.expect("a frame changes KeyJ's menu item");
    turns.set_grip(150, frame).unwrap();
    let reason = turns.set_grip(150, 12).unwrap_err();
    assert!(reason.contains("0 to 11"), "{reason}");
    let reason = turns.set_grip(600, 0).unwrap_err();
    assert!(reason.contains("0 to 599"), "{reason}");
    let report = turns.grip_report().unwrap();
    assert_eq!((number(&report, "cap"), number(&report, "frame")), (150, frame));
    assert_eq!(turns.presses(), (0, 0));
    assert_eq!(report.get("state_digest"), initial.get("state_digest"));
    assert_eq!(fs::read(folder.join("journal")).unwrap(), journal);

    assert_eq!(turns.key_down("KeyJ", false).unwrap(), Press::Committed);
    turns.key_up("KeyJ").unwrap();
    let args = &turns.committed()[0];
    assert_eq!(number(args, "cap"), 150);
    let request = menu_request(&menu.identity, &menu.order, item as u64);
    assert_eq!(args.get("letter").unwrap().get("request").unwrap().as_str().unwrap().as_bytes(), request);
    assert_eq!(turns.presses(), (1, 0));
    let report = turns.grip_report().unwrap();
    assert_eq!(number(report.get("last").unwrap(), "item"), item);
    let digest = report.get("state_digest").unwrap().as_str().unwrap().to_owned();
    assert_ne!(report.get("state_digest"), initial.get("state_digest"));
    let control = turns.check(Some((Puzzle::JumbleOn.opened_at() + 1, digest)));
    assert_eq!(control.get("pass"), Some(&Value::Bool(true)), "{control:?}");
    fs::remove_dir_all(folder).unwrap();
}
