use std::ops::Div;

use bevy::prelude::FloatExt;

use crate::cell::Cell;
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

pub fn get_bilinear_sample(size: u32, cell: &Cell) -> f32 {
    let points = get_sample_rect_cells(size, cell);

    let size = size as f32;
    // tx and ty will be a fraction of `size`.
    let tx = (cell.x as f32).rem_euclid(size) / size;
    let ty = (cell.y as f32).rem_euclid(size) / size;

    let [top_left, top_right, bot_right, bot_left] = points.map(|c| sample(&c));

    // upper + t * (upper - lower)
    let top = top_left.lerp(top_right, tx);
    let bot = bot_left.lerp(bot_right, tx);
    let value = top.lerp(bot, ty);
    value
}

pub fn sample(cell: &Cell) -> f32 {
    stable_hash(cell, 100) as f32 / 100.
}

pub fn tile_idx_for_cell(cell: &Cell) -> TileIdx {
    let sample = get_bilinear_sample(REGION_SIZE as u32, cell);

    match sample {
        0.0..=0.5 => TileIdx::Dirt,
        0.5..1.0 => TileIdx::GreenTree1,
        _ => TileIdx::GridSquare,
    }
}

/// Generates a random number using the cell as the seed s/t the number is the same for each cell.
/// This ensures that a specific cell will yield the same random result for the same `max`.
pub fn stable_hash(cell: &Cell, max: u32) -> usize {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    cell.hash(&mut hasher);
    let hash = hasher.finish();

    let mut rng = StdRng::seed_from_u64(hash);
    (rng.next_u32() % max) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- stable_hash ---

    #[test]
    fn stable_hash_is_deterministic() {
        let cell = Cell::new(3, 7);
        assert_eq!(stable_hash(&cell, 100), stable_hash(&cell, 100));
    }

    #[test]
    fn stable_hash_different_cells_differ() {
        // Collect hashes for a small grid and assert they're not all identical.
        let hashes: Vec<usize> = (0..5)
            .flat_map(|x| (0..5).map(move |y| stable_hash(&Cell::new(x, y), 1000)))
            .collect();
        let first = hashes[0];
        assert!(
            hashes.iter().any(|&h| h != first),
            "all cells produced the same hash"
        );
    }

    #[test]
    fn stable_hash_within_range() {
        for x in 0..10_i32 {
            for y in 0..10_i32 {
                let result = stable_hash(&Cell::new(x, y), 100);
                assert!(
                    result < 100,
                    "hash {result} out of range for cell ({x},{y})"
                );
            }
        }
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
                let v = get_bilinear_sample(size, &Cell::new(x, y));
                assert!(
                    (0.0..=1.0).contains(&v),
                    "bilinear sample {v} out of [0,1] at ({x},{y})"
                );
            }
        }
    }
}
