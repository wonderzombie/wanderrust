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
#[allow(dead_code)]
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
