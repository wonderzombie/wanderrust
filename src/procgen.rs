use std::hash::{Hash, Hasher};
use std::ops::Div;

use bevy::prelude::FloatExt;

use crate::cell::Cell;
use crate::ptable::{ProbabilityTable, TableBuilder, WeightedEntry};
use crate::tiles::TileIdx;

use fxhash::FxHasher32;

const REGION_SIZE: i32 = 8;

/// Returns the sub-grid coordinates using a subgrid of `size`.
pub fn get_sample_rect_cells(size: u32, cell: &Cell) -> [Cell; 4] {
    let size = size as i32;
    let top_left = cell.clone().div(size);
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
    stable_hash(cell, depth)
}

pub fn tile_idx_for_cell(cell: &Cell, table: &ProbabilityTable) -> TileIdx {
    // Partially apply get_bilinear_sample() with the same cell and region size.
    let sampler = |depth| get_bilinear_sample(REGION_SIZE as u32, cell, depth);
    select_from_table(table, cell, &sampler, 1)
}

pub fn biome_ptable() -> ProbabilityTable {
    TableBuilder::new()
        // Grasslands.
        .table(0.7, |t| {
            t.table(0.5, |t| {
                t.tile(0.5, TileIdx::Blank)
                    .tile(0.005, TileIdx::Rocks)
                    .tile(0.05, TileIdx::Grass)
                    .tile(0.005, TileIdx::GrassFlowers)
                    .tile(0.005, TileIdx::GrassLong)
            })
            .table(0.5, |t| {
                t.tile(0.5, TileIdx::Blank)
                    .tile(0.25, TileIdx::GrassFlowers)
                    .tile(0.05, TileIdx::GrassLong)
                    .tile(0.025, TileIdx::Grass)
            })
        })
        .table(0.5, |t| {
            // Deciduous forest.
            t.table(0.5, |t| {
                t.tile(1.5, TileIdx::GreenTree1)
                    .tile(0.15, TileIdx::DoubleGreenTree1)
                    .tile(0.05, TileIdx::Blank)
            })
            // Coniferous forest.
            .table(0.5, |t| {
                t.tile(0.5, TileIdx::GreenTree2)
                    .tile(0.5, TileIdx::DoubleGreenTree2)
                    .tile(0.05, TileIdx::Blank)
            })
        })
        .build()
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
                        &subtable,
                        cell,
                        &|depth| sample_cell_with_depth(cell, depth),
                        depth + 1,
                    );
                }
            }
        }
    }

    TileIdx::GridSquare
}

/// Generates a random number using the cell and the given seed s/t the number is the same for each cell.
/// This ensures that a specific cell will yield the same random result as long as `seed` is the same.
pub fn stable_hash(cell: &Cell, seed: u64) -> f32 {
    let mut hasher = FxHasher32::default();
    cell.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish() as f32 / u32::MAX as f32
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
        let hashes: Vec<f32> = (0..5)
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

    // --- sample_cell_with_depth ---

    #[test]
    fn sample_cell_with_depth_stays_in_unit_range() {
        for x in 0..10_i32 {
            for y in 0..10_i32 {
                let v = sample_cell_with_depth(&Cell::new(x, y), 0);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "sample_cell_with_depth {v} out of [0,1] at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn sample_cell_with_depth_not_always_zero() {
        // Before the fix, dividing by f32::MAX produced values indistinguishable from 0.
        let values: Vec<f32> = (0..20_i32)
            .flat_map(|x| (0..20_i32).map(move |y| sample_cell_with_depth(&Cell::new(x, y), 1)))
            .collect();
        let nonzero = values.iter().filter(|&&v| v > 1e-10).count();
        assert!(
            nonzero > values.len() / 2,
            "most sample_cell_with_depth values were effectively zero ({nonzero}/{} nonzero)",
            values.len()
        );
    }

    // --- tile_idx_for_cell ---

    #[test]
    fn tile_idx_for_cell_is_deterministic() {
        let table = crate::ptable::TableBuilder::new()
            .tile(1.0, TileIdx::Blank)
            .tile(1.0, TileIdx::Grass)
            .tile(1.0, TileIdx::Rocks)
            .build();
        let cell = Cell::new(5, 7);
        let first = tile_idx_for_cell(&cell, &table);
        let second = tile_idx_for_cell(&cell, &table);
        assert_eq!(first, second, "tile_idx_for_cell is not deterministic");
    }

    #[test]
    fn tile_idx_for_cell_returns_entry_from_table() {
        let table = crate::ptable::TableBuilder::new()
            .tile(1.0, TileIdx::Blank)
            .tile(1.0, TileIdx::Grass)
            .tile(1.0, TileIdx::Rocks)
            .build();
        let valid = [TileIdx::Blank, TileIdx::Grass, TileIdx::Rocks];
        for x in 0..10_i32 {
            for y in 0..10_i32 {
                let idx = tile_idx_for_cell(&Cell::new(x, y), &table);
                assert!(
                    valid.contains(&idx),
                    "tile_idx_for_cell returned unexpected tile {idx:?} at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn tile_idx_for_cell_produces_varied_output() {
        // With a uniform table, we should see more than one distinct tile across a grid.
        let table = crate::ptable::TableBuilder::new()
            .tile(1.0, TileIdx::Blank)
            .tile(1.0, TileIdx::Grass)
            .tile(1.0, TileIdx::Rocks)
            .build();
        let mut tiles = std::collections::HashSet::new();
        for x in 0..20_i32 {
            for y in 0..20_i32 {
                tiles.insert(tile_idx_for_cell(&Cell::new(x, y), &table));
            }
        }
        assert!(
            tiles.len() > 1,
            "tile_idx_for_cell returned only one tile across a 20x20 grid: {:?}",
            tiles
        );
    }

    #[test]
    fn tile_idx_for_cell_with_subtable_produces_varied_output() {
        // Subtable recursion previously was broken by the double-division bug; verify it now works.
        let table = crate::ptable::TableBuilder::new()
            .table(0.5, |t| {
                t.tile(1.0, TileIdx::GreenTree1)
                    .tile(1.0, TileIdx::GreenTree2)
            })
            .table(0.5, |t| {
                t.tile(1.0, TileIdx::Grass).tile(1.0, TileIdx::Blank)
            })
            .build();
        let mut tiles = std::collections::HashSet::new();
        for x in 0..20_i32 {
            for y in 0..20_i32 {
                tiles.insert(tile_idx_for_cell(&Cell::new(x, y), &table));
            }
        }
        assert!(
            tiles.len() > 1,
            "tile_idx_for_cell with subtables returned only one tile: {:?}",
            tiles
        );
    }
}
