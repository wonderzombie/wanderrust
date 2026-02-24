use std::collections::HashMap;
use std::ops::Div;

use crate::cell::Cell;
use crate::colors;
use crate::tilemap::TilemapSize;
use crate::tiles::{Highlighted, MapTile, Opaque, Revealed, TileIdx, TilePreview, Walkable};

use bevy::prelude::*;

pub const DEFAULT_LAYER: u32 = 0;

pub const MAP: &str = r#"
####################
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#...###...........#
#.................#
#.b.w.D.O. .......#
#.................#
#....#............D
#.X..#...#....###.#
#.............###.#
###################"#;

/// Key for the map:
/// - `#` = wall
/// - `.` = floor
/// - `X` = player start position
/// - `D` = door (closed)
/// - `O` = door (open)
/// - ` ` = empty space (not walkable)
/// - `b` = brown chest
/// - `w` = white chest

#[derive(Resource, Debug)]
/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
pub struct MapSpec {
    pub size: TilemapSize,
    pub layer: u32,
    pub pieces: HashMap<TileIdx, Vec<Cell>>,
    pub start: Cell,
}

pub const DEFAULT_TILE_SIZE: u32 = 16;

impl MapSpec {
    pub fn from_str(map_str: &str) -> Self {
        let lines: Vec<&str> = map_str.lines().collect();
        let height = lines.len() as u32;
        let width = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as u32;

        let pieces: HashMap<TileIdx, Vec<Cell>> = lines
            .iter()
            .enumerate()
            .flat_map(|(y, line)| {
                line.chars().enumerate().filter_map(move |(x, ch)| {
                    let tile_idx = match ch {
                        '#' => Some(TileIdx::StoneWall),
                        '.' => Some(TileIdx::Blank),
                        'X' => Some(TileIdx::Blank),
                        'D' => Some(TileIdx::DoorBrownThickClosed1),
                        'O' => Some(TileIdx::DoorwayBrownThick),
                        'b' => Some(TileIdx::ChestBrownClosed),
                        'w' => Some(TileIdx::ChestWhiteClosed),
                        'T' => Some(TileIdx::GreenTree1),
                        't' => Some(TileIdx::GreenTree2),
                        'U' => Some(TileIdx::DoubleGreenTree1),
                        'u' => Some(TileIdx::DoubleGreenTree2),
                        ' ' => None, // Empty space, not walkable
                        _ => None,   // Ignore unknown characters
                    };
                    tile_idx.map(|idx| {
                        (
                            idx,
                            Cell {
                                x: x as i32,
                                y: y as i32,
                            },
                        )
                    })
                })
            })
            .fold(HashMap::new(), |mut acc, (idx, cell)| {
                acc.entry(idx).or_default().push(cell);
                acc
            });

        MapSpec {
            size: TilemapSize {
                width,
                height,
                tile_size: DEFAULT_TILE_SIZE,
            },
            pieces,
            layer: DEFAULT_LAYER,
            start: Cell { x: 5, y: 5 },
        }
    }

    pub fn from_procedure(fx: impl Fn(&Cell) -> TileIdx, size: (u32, u32)) -> Self {
        let start = Cell {
            x: size.0 as i32 / 2,
            y: size.1 as i32 / 2,
        };
        info!("map from procedure; start {:?}", start);
        let tiles = size.0 * size.1;

        let pieces: HashMap<TileIdx, Vec<Cell>> = (0..tiles)
            .map(|i| {
                let cell = Cell::from_idx(size.0, i as usize);
                let tile_idx = fx(&cell);
                (tile_idx, cell)
            })
            .fold(HashMap::new(), |mut acc, (idx, cell)| {
                acc.entry(idx).or_default().push(cell);
                acc
            });

        info!("map_spec: generated tile distribution:");
        for (tile_idx, cells) in &pieces {
            info!("tile {:?}: {}", tile_idx, cells.len());
        }

        MapSpec {
            size: TilemapSize {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            pieces,
            layer: DEFAULT_LAYER,
            start: Cell {
                x: size.0 as i32 / 2,
                y: size.1 as i32 / 2,
            },
        }
    }
}

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

const REGION_SIZE: i32 = 8;

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

/// Updates the sprites of map tiles when their tile index changes.
pub fn sync_tiles(
    mut commands: Commands,
    mut tiles: Query<
        (Entity, &mut Sprite, &TileIdx, Option<&TilePreview>),
        (With<MapTile>, Or<(Changed<TileIdx>, Changed<TilePreview>)>),
    >,
) {
    for (entity, mut sprite, tile_idx, preview_opt) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);
        // If there's a preview, we should apply that tile index instead.
        let preview_opt = preview_opt.and_then(|it| it.get());
        let next_idx = preview_opt.unwrap_or(*tile_idx);

        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = next_idx.into();
        }

        if tile_idx.is_walkable() {
            entity_command.insert(Walkable);
        } else {
            entity_command.remove::<Walkable>();
        }

        if tile_idx.is_transparent() {
            entity_command.remove::<Opaque>();
        } else {
            entity_command.insert(Opaque);
        }
    }
}

pub fn update_map_tile_visuals(
    mut tiles: Query<
        (
            &mut Sprite,
            Option<&Highlighted>,
            Option<&Revealed>,
            Option<&TilePreview>,
        ),
        (
            With<MapTile>,
            Or<(
                Changed<Highlighted>,
                Changed<Revealed>,
                Changed<TilePreview>,
            )>,
        ),
    >,
) {
    for (mut sprite, highlighted, revealed, preview_opt) in tiles.iter_mut() {
        let revealed = revealed.map_or(false, |it| it.0);
        let highlighted = highlighted.map_or(false, |it| it.0);

        sprite.color = if highlighted {
            colors::KENNEY_GOLD
        } else if revealed {
            Color::WHITE
        } else {
            Color::NONE
        };

        if preview_opt.is_some_and(TilePreview::is_active) {
            sprite.color.set_alpha(0.5);
        }
    }
}
