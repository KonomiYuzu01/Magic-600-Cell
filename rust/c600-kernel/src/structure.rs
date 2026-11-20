//! The structure of the 600 cells (LP2 draft for LP3's C6, step 1 item 3): which cells share a face, how many face
//! steps part two cells, and the breadth-first layers around a cell with their members, as 0.4's structure explorer and
//! filter read them (`core.py`: `Model.adj`, `cell_distances` and `layer_mask`).
//!
//! 0.4 takes a cell's four neighbours as the four nearest cell centres. Here two cells are neighbours when they share
//! three of their four vertices, read from the model's 120 vertex pieces (orbit 34): each holds one sticker in each of
//! the 20 cells that meet at its vertex, and a slot lies in cell `slot / 433`. On the shipped model both give the same
//! graph; `tests/structure.rs` checks every step count against 0.4's.

use crate::model::{Model, CELLS, CELL_STICKERS, PIECES};
use crate::LoadError;

/// The orbit of the pieces at the 120 vertices.
const VERTEX_ORBIT: i32 = 34;
const VERTICES: usize = 120;
/// The cells that meet at a vertex.
const VERTEX_CELLS: usize = 20;

/// The cells' face graph and the step counts between every two cells.
pub(crate) struct Structure {
    /// `steps[a * CELLS + b]`: the face steps from cell `a` to cell `b`.
    steps: Vec<u8>,
    /// The greatest step count from each cell, which is its last layer.
    last: Vec<u8>,
}

impl Structure {
    /// Builds the face graph from the vertex pieces and checks the facts the rows rely on: 120 vertices, each meeting
    /// 20 cells, four vertices and four face neighbours per cell, one connected graph, and at most 32 layers per cell.
    pub(crate) fn build(model: &Model) -> Result<Structure, LoadError> {
        let fail = |text: &str| LoadError(format!("cell structure: {text}"));
        let mut vertex_cells: Vec<Vec<usize>> = Vec::with_capacity(VERTICES);
        for piece in (0..PIECES).filter(|&piece| model.piece_orbit[piece] == VERTEX_ORBIT) {
            let mut cells: Vec<usize> =
                model.home_slots(piece).iter().map(|&slot| slot as usize / CELL_STICKERS).collect();
            cells.sort_unstable();
            cells.dedup();
            if cells.len() != VERTEX_CELLS {
                return Err(fail("a vertex piece must meet 20 distinct cells"));
            }
            vertex_cells.push(cells);
        }
        if vertex_cells.len() != VERTICES {
            return Err(fail("the vertex orbit must hold 120 pieces"));
        }
        let mut cell_vertices = vec![Vec::with_capacity(4); CELLS];
        for (vertex, cells) in vertex_cells.iter().enumerate() {
            for &cell in cells {
                cell_vertices[cell].push(vertex);
            }
        }
        if cell_vertices.iter().any(|vertices| vertices.len() != 4) {
            return Err(fail("every cell must meet four vertices"));
        }

        let mut neighbours = vec![[0usize; 4]; CELLS];
        for (cell, row) in neighbours.iter_mut().enumerate() {
            let mut shared = [0u8; CELLS];
            for &vertex in &cell_vertices[cell] {
                for &other in &vertex_cells[vertex] {
                    shared[other] += 1;
                }
            }
            shared[cell] = 0;
            let faces: Vec<usize> = (0..CELLS).filter(|&other| shared[other] == 3).collect();
            *row = faces.try_into().map_err(|_| fail("every cell must share a face with four cells"))?;
        }
        if (0..CELLS).any(|cell| neighbours[cell].iter().any(|&other| !neighbours[other].contains(&cell))) {
            return Err(fail("the face graph must be symmetric"));
        }

        let mut steps = vec![u8::MAX; CELLS * CELLS];
        let mut last = vec![0u8; CELLS];
        let mut queue = Vec::with_capacity(CELLS);
        for from in 0..CELLS {
            let row = &mut steps[from * CELLS..(from + 1) * CELLS];
            row[from] = 0;
            queue.clear();
            queue.push(from);
            let mut next = 0;
            while let Some(&cell) = queue.get(next) {
                next += 1;
                for &other in &neighbours[cell] {
                    if row[other] == u8::MAX {
                        row[other] = row[cell] + 1;
                        queue.push(other);
                    }
                }
            }
            if queue.len() != CELLS {
                return Err(fail("the face graph must be connected"));
            }
            last[from] = row[queue[CELLS - 1]];
            if last[from] >= 32 {
                return Err(fail("a cell must have at most 32 layers"));
            }
        }
        Ok(Structure { steps, last })
    }

    /// The face steps from cell `first` to cell `second`, both below 600.
    pub(crate) fn steps(&self, first: usize, second: usize) -> usize {
        self.steps[first * CELLS + second] as usize
    }

    /// The last layer around `cell`, below 600: its greatest step count to another cell.
    pub(crate) fn last_layer(&self, cell: usize) -> usize {
        self.last[cell] as usize
    }

    /// The pieces and slots of each layer around `cell`, below 600, from layer 0 to its last. A position is in a layer
    /// when one of its home slots lies in a cell of that layer, so it counts whole and can be in several layers.
    pub(crate) fn layers(&self, model: &Model, cell: usize) -> Vec<(usize, usize)> {
        let row = &self.steps[cell * CELLS..(cell + 1) * CELLS];
        let mut counts = vec![(0usize, 0usize); self.last_layer(cell) + 1];
        for piece in 0..PIECES {
            let home = model.home_slots(piece);
            let mut layers = 0u32;
            for &slot in home {
                layers |= 1 << row[slot as usize / CELL_STICKERS];
            }
            while layers != 0 {
                let layer = layers.trailing_zeros() as usize;
                counts[layer].0 += 1;
                counts[layer].1 += home.len();
                layers &= layers - 1;
            }
        }
        counts
    }

    /// The members of layer `layer` around `cell`, both in range, that [`Structure::layers`] counts for it: the
    /// positions with a home slot in a cell of that layer, in increasing order, and all their home slots, in
    /// increasing order. A slot is the home of one position, so no slot repeats.
    pub(crate) fn members(&self, model: &Model, cell: usize, layer: usize) -> (Vec<usize>, Vec<usize>) {
        let row = &self.steps[cell * CELLS..(cell + 1) * CELLS];
        let (mut pieces, mut slots) = (Vec::new(), Vec::new());
        for piece in 0..PIECES {
            let home = model.home_slots(piece);
            if home.iter().any(|&slot| usize::from(row[slot as usize / CELL_STICKERS]) == layer) {
                pieces.push(piece);
                slots.extend(home.iter().map(|&slot| slot as usize));
            }
        }
        slots.sort_unstable();
        (pieces, slots)
    }
}
