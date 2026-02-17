use std::collections::HashMap;

use crate::cell::Cell;
use crate::tiles::{MapTile, Opaque, TileIdx, Walkable};
use crate::{PieceBundle, SpriteAtlas, TILE_SIZE_PX};

use bevy::prelude::*;

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
    pub size: UVec2,
    pub pieces: HashMap<TileIdx, Vec<Cell>>,
}

impl MapSpec {
    pub fn from_str(map_str: &str) -> Self {
        let lines: Vec<&str> = map_str.lines().collect();
        let height = lines.len() as u32;
        let width = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as u32;

        let pieces = lines
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
                acc.entry(idx).or_insert_with(Vec::new).push(cell);
                acc
            });

        MapSpec {
            size: UVec2::new(width, height),
            pieces: pieces,
        }
    }
}

/// Generates a random number using the cell as the seed s/t the number is the same for each cell.
/// This ensures that a specific cell will yield the same random result for the same `max`.
#[allow(dead_code)]
fn stable_hash(cell: &Cell, max: u32) -> u32 {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    cell.hash(&mut hasher);
    let hash = hasher.finish();

    let mut rng = StdRng::seed_from_u64(hash);
    rng.next_u32() % max
}

pub fn draw_ascii_map(mut commands: Commands, atlas: Res<SpriteAtlas>, spec: Res<MapSpec>) {
    for (tile_idx, cells) in spec.pieces.iter() {
        for cell in cells.iter() {
            commands.spawn((
                MapTile,
                PieceBundle {
                    sprite: atlas.sprite(),
                    cell: *cell,
                    transform: Transform::from_xyz(
                        cell.x as f32 * TILE_SIZE_PX,
                        cell.y as f32 * TILE_SIZE_PX,
                        -2.0,
                    ),
                },
                *tile_idx,
            ));
        }
    }
}

/// Updates the sprites of map tiles when their tile index changes.
pub fn update_map_tiles(
    mut commands: Commands,
    mut tiles: Query<(Entity, &mut Sprite, &TileIdx), (With<MapTile>, Changed<TileIdx>)>,
) {
    for (entity, mut sprite, tile_idx) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = (*tile_idx).into();
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
