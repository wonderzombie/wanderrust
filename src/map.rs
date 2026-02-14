use std::collections::HashMap;

use crate::{Fov, PieceBundle, SpriteAtlas, TILE_SIZE_PX};
use crate::cell::Cell;
use crate::tiles::{AtlasIdx, MapTile, Opaque, TileIdx, Walkable};

use bevy::prelude::*;
use itertools::iproduct;
use mrpas::Mrpas;

pub const MAP : &str = r#"
####################
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#.................#
#.................D
#.X...............#
#.................#
###################"#;

/// Key for the map legend:
/// - `#` = wall
/// - `.` = floor
/// - `X` = player start position
/// - `D` = door (closed)
/// - `O` = door (open)
/// - ` ` = empty space (not walkable)
/// - `b` = brown chest
/// - `w` = white chest


#[derive(Resource, Debug)]
pub struct MapSpec {
    pub size: UVec2,
    pub default_tile: TileIdx,
    pub pieces: HashMap<TileIdx, Vec<Cell>>,
    content: String,
}

impl MapSpec {
    pub fn from_str(map_str: &str) -> Self {
        let lines: Vec<&str> = map_str.lines().collect();
        let height = lines.len() as u32;
        let width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0) as u32;

        let pieces = lines.iter().enumerate().flat_map(|(y, line)| {
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
                tile_idx.map(|idx| (idx, Cell { x: x as i32, y: y as i32 }))
            })
        }).fold(HashMap::new(), |mut acc, (idx, cell)| {
            acc.entry(idx).or_insert_with(Vec::new).push(cell);
            acc
        });

        MapSpec {
            size: UVec2::new(width, height),
            default_tile: TileIdx::Blank,
            pieces: pieces,
            content: map_str.to_string(),
        }
    }
}


/// Initializes the map by spawning entities for each cell with the default tile sprite.
pub fn init_map(mut commands: Commands, atlas: Res<SpriteAtlas>, spec: Res<MapSpec>) {
    let fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));
    commands.insert_resource(fov);

    let sprite = atlas.sprite_from_idx(spec.default_tile.into());
    let default: AtlasIdx = spec.default_tile.into();

    for (x, y) in iproduct!(0..spec.size.x, 0..spec.size.y) {
        commands.spawn((
            MapTile,
            PieceBundle {
                sprite: sprite.clone(),
                cell: Cell::at_coords(x, y),
                atlas_idx: default,
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

pub fn draw_structures(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    let wall_sprite = atlas.sprite_from_idx(AtlasIdx(1)); // Example wall sprite index

    // Example structure: a simple 3x3 building in the center of the map
    let structure_cells = [
        Cell::at_coords(14, 12),
        Cell::at_coords(15, 12),
        Cell::at_coords(16, 12),
        Cell::at_coords(14, 13),
        Cell::at_coords(15, 13),
        Cell::at_coords(16, 13),
        Cell::at_coords(14, 14),
        Cell::at_coords(15, 14),
        Cell::at_coords(16, 14),
    ];

    for cell in structure_cells.iter() {
        commands.spawn((
            MapTile,
            PieceBundle {
                sprite: wall_sprite.clone(),
                cell: *cell,
                atlas_idx: AtlasIdx(1), // Wall tile index
                transform: Transform::from_xyz(
                    cell.x as f32 * TILE_SIZE_PX,
                    cell.y as f32 * TILE_SIZE_PX,
                    -2.0,
                ),
            },
            TileIdx::StoneWall,
        ));
    }
}

/// Updates the sprites of map tiles when their atlas index changes.
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
            entity_command.insert(Opaque);
        } else {
            entity_command.remove::<Opaque>();
        }
    }
}
