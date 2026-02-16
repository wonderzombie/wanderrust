use std::collections::HashMap;

use crate::cell::Cell;
use crate::tiles::{MapTile, Opaque, TileIdx, Walkable};
use crate::{Fov, PieceBundle, Player, SpriteAtlas, TILE_SIZE_PX};

use bevy::prelude::*;
use itertools::iproduct;
use mrpas::Mrpas;

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
    pub default_tile: TileIdx,
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
            default_tile: TileIdx::Blank,
            pieces: pieces,
        }
    }
}

/// Initializes the map by spawning entities for each cell with the default tile sprite.
pub fn init_map(mut commands: Commands, atlas: Res<SpriteAtlas>, spec: Res<MapSpec>) {
    let fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));
    commands.insert_resource(fov);

    for (x, y) in iproduct!(0..spec.size.x, 0..spec.size.y) {
        commands.spawn((
            MapTile,
            PieceBundle {
                sprite: atlas.sprite(),
                cell: Cell::at_coords(x, y),
                transform: Transform::from_xyz(
                    x as f32 * TILE_SIZE_PX,
                    y as f32 * TILE_SIZE_PX,
                    -3.0,
                ),
            },
            spec.default_tile,
        ));
    }
}

/// Decorates the map by assigning a random ground tile to each cell based on its coordinates.
pub fn decorate_map(mut tiles: Query<(&mut TileIdx, &Cell), With<MapTile>>) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let ground_tile_types = [
        TileIdx::Blank,
        TileIdx::Dirt,
        TileIdx::Gravel,
        TileIdx::Grass,
    ];

    for (mut tile_idx, cell) in tiles.iter_mut() {
        let mut hasher = DefaultHasher::new();
        cell.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = StdRng::seed_from_u64(hash);
        let result = rng.next_u32() % (ground_tile_types.len() as u32);
        *tile_idx = ground_tile_types[result as usize];
    }
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

/// Updates the field of view model based on the transparency of tiles when their atlas index changes.
pub fn update_fov_model(mut fov: ResMut<Fov>, query: Query<(&Cell, &TileIdx), With<MapTile>>) {
    for (cell, tile_idx) in query.iter() {
        let (x, y) = (*cell).into();
        fov.set_transparent((x, y), tile_idx.is_transparent());
    }
}

/// Updates the visibility of map tiles based on the player's field of view.
pub fn update_vision(
    mut fov: ResMut<Fov>,
    player_query: Query<&Cell, With<Player>>,
    mut tiles: Query<(&Cell, &mut Sprite), With<MapTile>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    fov.clear_field_of_view();
    fov.compute_field_of_view((*player_cell).into(), 5);
    for (cell, mut sprite) in tiles.iter_mut() {
        let (x, y) = (*cell).into();
        if fov.is_in_view((x, y)) {
            sprite.color = Color::WHITE;
        } else {
            sprite.color = Color::BLACK.with_alpha(0.0);
        }
    }
}
