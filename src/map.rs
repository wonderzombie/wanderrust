use std::collections::HashMap;

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
        }
    }

    pub fn from_procedure(fx: impl Fn(&Cell) -> TileIdx, size: (u32, u32)) -> Self {
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

        MapSpec {
            size: TilemapSize {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            pieces,
            layer: DEFAULT_LAYER,
        }
    }
}

pub fn stable_tile_hash(cell: &Cell, tile_indices: &[TileIdx]) -> TileIdx {
    let max = tile_indices.len() as u32;
    tile_indices
        .get(stable_hash(cell, max))
        .copied()
        .unwrap_or_default()
}

pub fn tile_idx_for_cell(cell: &Cell) -> TileIdx {
    stable_tile_hash(cell, &[TileIdx::Dirt, TileIdx::GreenTree1])
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
