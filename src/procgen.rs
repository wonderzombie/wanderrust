use std::ops::Div;

use bevy::prelude::FloatExt;

use crate::TILE_SIZE_PX;
use crate::cell::Cell;
use crate::ptable::{ProbabilityTable, TableBuilder, WeightedEntry};
use crate::tiles::TileIdx;

const REGION_SIZE: i32 = 8;

/// Returns the sub-grid coordinates using a subgrid of `size`.
pub fn get_sample_rect_cells(size: u32, cell: &Cell) -> [Cell; 4] {
    let size = size as i32;
    let top_left = cell.clone().div(REGION_SIZE);
    [
        top_left,
        top_left + Cell::new(size, 0),
        top_left + Cell::new(size, size),
        top_left + Cell::new(0, size),
    ]
}

pub fn get_bilinear_sample(size: u32, cell: &Cell, depth: u64) -> f32 {
    let points = get_sample_rect_cells(size, cell);

    let size = size as f32;
    // tx and ty will be a fraction of `size`.
    let tx = (cell.x as f32).rem_euclid(size) / size;
    let ty = (cell.y as f32).rem_euclid(size) / size;

    let [top_left, top_right, bot_right, bot_left] =
        points.map(|c| sample_cell_with_depth(&c, depth));

    // upper + t * (upper - lower)
    let top = top_left.lerp(top_right, tx);
    let bot = bot_left.lerp(bot_right, tx);
    let value = top.lerp(bot, ty);
    value
}

pub fn sample_cell_with_depth(cell: &Cell, depth: u64) -> f32 {
    stable_hash(cell, depth) as f32 / u32::MAX as f32
}

pub fn tile_idx_for_cell(cell: &Cell) -> TileIdx {
    // Partially apply get_bilinear_sample() with the same cell and region size.
    let sampler = |depth| get_bilinear_sample(REGION_SIZE as u32, cell, depth);
    select_from_table(&ptable_with_builder(), cell, &sampler, 1)
}

fn ptable_with_builder() -> ProbabilityTable {
    ProbabilityTable(
        TableBuilder::new()
            .table(0.5, |t| {
                t.tile(1.0, TileIdx::Blank)
                    .tile(0.01, TileIdx::Rocks)
                    .tile(0.3, TileIdx::GrassBrown)
                    .tile(0.3, TileIdx::Grass)
                    .tile(0.1, TileIdx::GrassFlowers)
                    .tile(0.3, TileIdx::GrassLong)
            })
            .table(0.5, |t| {
                t.table(0.5, |t| {
                    t.tile(1.5, TileIdx::GreenTree1)
                        .tile(0.05, TileIdx::DoubleGreenTree1)
                        .tile(0.01, TileIdx::Blank)
                        .tile(0.01, TileIdx::BigGreenTree1)
                        .tile(0.01, TileIdx::BigGreenTree2)
                })
                .table(0.5, |t| {
                    t.tile(2.0, TileIdx::GreenTree2)
                        .tile(0.5, TileIdx::DoubleGreenTree2)
                        .tile(0.01, TileIdx::Blank)
                })
            })
            .build(),
    )
}

fn select_from_table(
    table: &ProbabilityTable,
    cell: &Cell,
    sampler: &dyn Fn(u64) -> f32,
    depth: u64,
) -> TileIdx {
    let total: f32 = table.iter().map(|e| e.weight()).sum();

    let mut cursor = sampler(depth) * total;
    for entry in table.iter() {
        match entry {
            WeightedEntry::Tile(w, tile_idx) => {
                cursor -= w;
                if cursor <= 0.0 {
                    return *tile_idx;
                }
            }
            WeightedEntry::Table(w, subtable) => {
                cursor -= w;
                if cursor <= 0.0 {
                    return select_from_table(
                        &ProbabilityTable(subtable.clone()),
                        cell,
                        &|depth| sample_cell_with_depth(cell, depth),
                        depth + 1,
                    );
                }
            }
        }
    }

    if let Some(WeightedEntry::Tile(_, t)) = table.last() {
        return *t;
    }

    TileIdx::GridSquare
}

/// Generates a random number using the cell and the given seed s/t the number is the same for each cell.
/// This ensures that a specific cell will yield the same random result as long as `seed` is the same.
pub fn stable_hash(cell: &Cell, seed: u64) -> u32 {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    cell.hash(&mut hasher);
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    let mut rng = StdRng::seed_from_u64(hash);
    rng.next_u32()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- stable_hash ---

    #[test]
    fn stable_hash_is_deterministic() {
        let cell = Cell::new(3, 7);
        assert_eq!(stable_hash(&cell, 1), stable_hash(&cell, 1));
    }

    #[test]
    fn stable_hash_different_cells_differ() {
        // Collect hashes for a small grid and assert they're not all identical.
        let hashes: Vec<u32> = (0..5)
            .flat_map(|x| (0..5).map(move |y| stable_hash(&Cell::new(x, y), 1)))
            .collect();
        let first = hashes[0];
        assert!(
            hashes.iter().any(|&h| h != first),
            "all cells produced the same hash"
        );
    }

    // --- get_sample_rect_cells ---

    #[test]
    fn sample_rect_corners_have_correct_offsets() {
        let size: u32 = 8;
        let cell = Cell::new(16, 16); // cleanly divisible, top_left = (2, 2)
        let [tl, tr, br, bl] = get_sample_rect_cells(size, &cell);
        let s = size as i32;

        assert_eq!(tr, tl + Cell::new(s, 0));
        assert_eq!(br, tl + Cell::new(s, s));
        assert_eq!(bl, tl + Cell::new(0, s));
    }

    // --- get_bilinear_sample ---

    #[test]
    fn bilinear_sample_stays_in_unit_range() {
        let size = REGION_SIZE as u32;
        for x in 0..32_i32 {
            for y in 0..32_i32 {
                let v = get_bilinear_sample(size, &Cell::new(x, y), 0);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "bilinear sample {v} out of [0,1] at ({x},{y})"
                );
            }
        }
    }
}
