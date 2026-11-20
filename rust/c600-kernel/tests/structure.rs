//! LP2, the structure rows of the 600 cells (LP3's C6, step 1 item 3): two cells compared (`Kernel::compare_cells`) and
//! the breadth-first layers around a cell with their pieces and slots (`Kernel::cell_layers`, `Kernel::layer_counts`,
//! `Kernel::layer_members`), which `structure.compare`, `filter.preview` and `structure.layer` read. The expected
//! values come from 0.4's `core.py` on the same assets: `Model.cell_distances`, `Model.adj` and `Model.layer_mask`,
//! with the pieces counted as `layer_mask` selects them and the slots as `np.diff(face_offsets)` sums them. The two
//! digests cover every cell and layer, and every pair of cells. A pass is source evidence.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::{codes, CellComparison, Kernel, LayerCounts, PIECES, SLOTS};

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| {
        let repository: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        match Kernel::load(&repository.join("assets")) {
            Ok(kernel) => kernel,
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        }
    })
}

/// 0.4's digest of every face step count: for each cell `a` from 0 to 599, the bytes of the steps from `a` to cells 0
/// to 599, hashed in that order.
const STEPS_SHA256: &str = "3297ac37a04f92c72bd509fe3be25bb367a3e559c1383bdf0d0965d58469f6e0";
/// 0.4's digest of every layer's counts: for each cell from 0 to 599 and each of its layers from 0, the line
/// `"{cell} {layer} {pieces} {slots}\n"`, hashed in that order. There are 9,600 lines.
const LAYERS_SHA256: &str = "dd6cedf8c5bdbe6d0105585808c63264c8f74fdaebda79764135e03949c72543";

/// Two cells compared: 0 steps from a cell to itself, 1 to a cell that shares a face, and every pair as 0.4 counts
/// it. Each cell shares a face with four cells, and the steps are symmetric. Covers no conformance ID.
#[test]
fn cells_compare_as_0_4_counts_their_face_steps() {
    let kernel = kernel();
    for (first, second, steps) in [(0, 0, 0), (0, 1, 1), (0, 599, 14), (17, 433, 9)] {
        let compared = kernel.compare_cells(first, second).unwrap();
        let expected = CellComparison { first: first as usize, second: second as usize, steps };
        assert_eq!(compared, expected);
        assert_eq!(compared.shares_face(), steps == 1);
    }

    let mut bytes = Vec::with_capacity(600 * 600);
    for a in 0..600 {
        let row: Vec<usize> = (0..600).map(|b| kernel.compare_cells(a, b).unwrap().steps).collect();
        assert_eq!(row.iter().filter(|&&steps| steps == 1).count(), 4, "cell {a}");
        for (b, &steps) in row.iter().enumerate() {
            assert_eq!(kernel.compare_cells(b as i64, a).unwrap().steps, steps, "{a} {b}");
            bytes.push(u8::try_from(steps).unwrap());
        }
    }
    assert_eq!(c600_digest::sha256_hex(&bytes), STEPS_SHA256);
}

/// The layers around a cell: a position is in a layer when one of its home slots lies in a cell that many face steps
/// away, so it counts whole and can be in several layers, as 0.4's `layer_mask` selects it. Layer 0 holds the cell's
/// 433 positions, every cell has 16 layers, a cell's layers together count at least every position and slot, and
/// `layer_counts` is `cell_layers`' row. Covers no conformance ID.
#[test]
fn layers_count_the_positions_and_slots_0_4_selects() {
    let kernel = kernel();
    let rows = [
        (0, 0, 433, 849),
        (0, 1, 1694, 3086),
        (0, 2, 4682, 8098),
        (0, 15, 433, 849),
        (17, 3, 9362, 16156),
        (599, 7, 28982, 49274),
    ];
    for (cell, layer, pieces, slots) in rows {
        let expected = LayerCounts { cell: cell as usize, layer: layer as usize, pieces, slots };
        assert_eq!(kernel.layer_counts(cell, layer), Ok(expected));
    }

    let mut lines = String::new();
    for cell in 0..600 {
        let layers = kernel.cell_layers(cell).unwrap();
        assert_eq!(layers.len(), 16, "cell {cell}");
        assert_eq!((layers[0].pieces, layers[0].layer), (433, 0), "cell {cell}");
        assert!(layers.iter().map(|row| row.pieces).sum::<usize>() >= PIECES, "cell {cell}");
        assert!(layers.iter().map(|row| row.slots).sum::<usize>() >= SLOTS, "cell {cell}");
        for row in &layers {
            assert_eq!(row.cell, cell as usize);
            lines.push_str(&format!("{} {} {} {}\n", row.cell, row.layer, row.pieces, row.slots));
        }
        assert_eq!(kernel.layer_counts(cell, 9), Ok(layers[9]), "cell {cell}");
    }
    assert_eq!(lines.lines().count(), 9_600);
    assert_eq!(c600_digest::sha256_hex(lines.as_bytes()), LAYERS_SHA256);
}

/// A cell outside 0 to 599 is refused with 0.4's `Cell must be 0..599`, and a layer past the cell's last with 0.4's
/// `Layer must be L0..L15 for the chosen color`, both as `KN2:input.named_object_invalid`. Covers no conformance ID.
#[test]
fn a_cell_or_layer_outside_the_model_is_refused() {
    let kernel = kernel();
    let cell = (codes::NAMED_OBJECT_INVALID, "Cell must be 0..599");
    for bad in [-1, 600, i64::MAX, i64::MIN] {
        for refusal in [
            kernel.compare_cells(bad, 0).unwrap_err(),
            kernel.compare_cells(0, bad).unwrap_err(),
            kernel.cell_layers(bad).unwrap_err(),
            kernel.layer_counts(bad, 0).unwrap_err(),
        ] {
            assert_eq!((refusal.code, refusal.message.as_str()), cell, "{bad}");
        }
    }
    let layer = (codes::NAMED_OBJECT_INVALID, "Layer must be L0..L15 for the chosen color");
    for bad in [-1, 16, i64::MAX] {
        let refusal = kernel.layer_counts(0, bad).unwrap_err();
        assert_eq!((refusal.code, refusal.message.as_str()), layer, "{bad}");
    }
}

/// A layer's members are what its counts count: `layer_members` names as many positions and slots as `layer_counts`
/// counts, each list strictly increasing, with the same refusals. The slots are the positions' home slots, layer 0
/// holds every slot of the cell itself, and a cell's layers together name every position. Covers no conformance ID.
#[test]
fn a_layers_members_are_what_its_counts_count() {
    let kernel = kernel();
    for cell in [0, 17, 599] {
        let mut named = vec![false; PIECES];
        for counted in kernel.cell_layers(cell).unwrap() {
            let members = kernel.layer_members(cell, counted.layer as i64).unwrap();
            assert_eq!((members.cell, members.layer), (counted.cell, counted.layer));
            assert_eq!((members.pieces.len(), members.slots.len()), (counted.pieces, counted.slots), "{counted:?}");
            assert!(members.pieces.windows(2).all(|pair| pair[0] < pair[1]), "{counted:?}");
            assert!(members.slots.windows(2).all(|pair| pair[0] < pair[1]), "{counted:?}");
            assert!(members.pieces.iter().all(|&piece| piece < PIECES), "{counted:?}");
            assert!(members.slots.iter().all(|&slot| slot < SLOTS), "{counted:?}");
            for &piece in &members.pieces {
                named[piece] = true;
            }
        }
        assert!(named.iter().all(|&named| named), "cell {cell}: its layers name every position");
    }

    let members = kernel.layer_members(17, 0).unwrap();
    let pieces: Vec<i64> = members.pieces.iter().map(|&piece| piece as i64).collect();
    let rows = kernel.positions(&kernel.solved(), &pieces).unwrap();
    let mut home: Vec<usize> = rows.iter().flat_map(|row| row.home.iter().map(|&slot| slot as usize)).collect();
    home.sort_unstable();
    assert_eq!(home, members.slots, "the positions' home slots");
    assert!((17 * 433..18 * 433).all(|slot| members.slots.binary_search(&slot).is_ok()), "the cell's own slots");

    for (cell, layer) in [(-1, 0), (600, 0), (i64::MAX, 0), (0, -1), (0, 16), (0, i64::MAX)] {
        let refusal = kernel.layer_members(cell, layer).unwrap_err();
        assert_eq!(refusal, kernel.layer_counts(cell, layer).unwrap_err(), "{cell} {layer}");
    }
}
