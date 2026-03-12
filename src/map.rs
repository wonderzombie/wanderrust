use crate::cell::Cell;
use crate::colors;
use crate::ptable::ProbabilityTable;
use crate::tilemap::MapDimensions;
use crate::tiles::{Highlighted, MapTile, Opaque, Revealed, TileIdx, TilePreview, Walkable};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub const DEFAULT_LAYER: u32 = 0;

#[allow(dead_code)]
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

#[derive(Component, Copy, Clone, Default, Debug, Deref, DerefMut)]
pub struct TilemapId(Option<Entity>);

impl TilemapId {
    pub fn get(&self) -> Option<Entity> {
        self.0
    }

    pub fn set(&mut self, id: Entity) {
        self.0.replace(id);
    }
}

#[derive(Resource, Default, Debug)]
/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
pub struct TilemapSpec {
    pub id: TilemapId,
    pub size: MapDimensions,
    pub layer: u32,
    /// A vector of tile indices and their corresponding cell positions. This will drive tilemap creation.
    pub tiles: Vec<(TileIdx, Cell)>,
    pub start: Cell,
}

pub const DEFAULT_TILE_SIZE: u32 = 16;

impl TilemapSpec {
    #[allow(dead_code)]
    pub fn from_str(map_str: &str) -> Self {
        let lines: Vec<&str> = map_str.lines().collect();
        let height = lines.len() as u32;
        let width = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as u32;

        let flat_pieces = lines
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
            .collect::<Vec<_>>();

        TilemapSpec {
            id: TilemapId::default(),
            size: MapDimensions {
                width,
                height,
                tile_size: DEFAULT_TILE_SIZE,
            },
            tiles: flat_pieces,
            layer: DEFAULT_LAYER,
            start: Cell { x: 5, y: 5 },
        }
    }

    pub fn with_ptable(
        table: ProbabilityTable,
        fx: impl Fn(&Cell, &ProbabilityTable) -> TileIdx,
        size: (u32, u32),
    ) -> Self {
        let start = Cell {
            x: size.0 as i32 / 2,
            y: size.1 as i32 / 2,
        };
        info!("=== map from procedure ===");
        let tiles = size.0 * size.1;
        info!("start: {:?}", start);
        info!("size: {:?}", size);

        let mut tally: HashMap<TileIdx, usize> = HashMap::new();

        let tiles = (0..tiles)
            .map(|i| {
                let cell = Cell::from_idx(size.0, i as usize);
                let tile_idx = fx(&cell, &table);
                tally.entry(tile_idx).and_modify(|e| *e += 1).or_insert(1);
                (tile_idx, cell)
            })
            .collect();

        info!("tile breakdown: {:#?}", tally);

        TilemapSpec {
            id: TilemapId::default(),
            size: MapDimensions {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            tiles,
            layer: DEFAULT_LAYER,
            start,
        }
    }
}

/// Sync [TileIdx] and [Sprite] visuals along with their gameplay properties.
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

/// Sync [MapTile] [Sprite] visual effects with the tile's logical state. This is orthogonal to [TileIdx].
/// TODO: consider whether or how function signature might be simplified.
pub fn update_tile_visuals(
    mut tiles: Query<
        (
            &mut Sprite,
            Option<&Highlighted>,
            &Revealed,
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
        let revealed = revealed.0;
        let highlighted = highlighted.is_some();

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
