use crate::{
    cell::Cell,
    colors,
    light::LightLevel,
    ptable::ProbabilityTable,
    tilemap::{
        Dimensions, EntryId, Portal, PortalCell, StratumId, TileCell, TilemapLayer, TilemapSpec,
    },
    tiles::{Highlighted, MapTile, Occupied, Opaque, Revealed, TileIdx, TilePreview, Walkable},
};

use bevy::ecs::query::QueryData;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Key for the map:
/// - `#` = wall
/// - `.` = floor
/// - `X` = player start position
/// - `D` = door (closed)
/// - `O` = door (open)
/// - ` ` = empty space (not walkable)
/// - `b` = brown chest
/// - `w` = white chest
///
/// See also [`TilemapSpec::KEY`].
#[allow(dead_code)]
pub const MAP_ZERO: &str = r#"
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

pub const MAP_ONE: &str = r#"
###################
#.................#
#.................#
#.................#
#.................#
#.................#
#....I............#
#...###...........#
#.......i.........#
#.b.w.D.O. .......#
#.................#
#.i...............D
#.X.....I.....###.#
#.i...........###.#
###################"#;

/// MAP_LAYER is the layer for the map tilemap.
pub const MAP_LAYER: TilemapLayer = TilemapLayer(-6.);

pub const DEFAULT_TILE_SIZE: u32 = 16;

impl TilemapSpec {
    const KEY: &[(char, TileIdx)] = &[
        ('#', TileIdx::StoneWall),
        ('.', TileIdx::Blank),
        ('X', TileIdx::Blank),
        ('D', TileIdx::DoorBrownThickClosed1),
        ('O', TileIdx::DoorwayBrownThick),
        ('b', TileIdx::ChestBrownClosed),
        ('w', TileIdx::ChestWhiteClosed),
        ('T', TileIdx::GreenTree1),
        ('t', TileIdx::GreenTree2),
        ('U', TileIdx::DoubleGreenTree1),
        ('u', TileIdx::DoubleGreenTree2),
        ('I', TileIdx::Torch),
        ('i', TileIdx::Candle),
        (' ', TileIdx::Blank),
        ('s', TileIdx::StairsDown),
        ('S', TileIdx::StairsUp),
    ];

    fn tile_for(c: char) -> Option<TileIdx> {
        TilemapSpec::KEY
            .iter()
            .find(|(k, _)| *k == c)
            .map(|(_, v)| *v)
    }

    #[allow(dead_code)]
    pub fn from_str(map_str: &str) -> Self {
        let lines: Vec<&str> = map_str.lines().collect();
        let height = lines.len() as u32;
        let width = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as u32;

        let id = StratumId(0);
        let all_tiles: HashMap<StratumId, Vec<TileCell>> =
            vec![(id, TilemapSpec::parse_map_str(map_str))]
                .into_iter()
                .collect();

        let all_portals = vec![(id, TilemapSpec::parse_portals(&all_tiles[&id]))]
            .into_iter()
            .collect();

        TilemapSpec {
            size: Dimensions {
                width,
                height,
                tile_size: DEFAULT_TILE_SIZE,
            },
            all_tiles,
            layer: MAP_LAYER,
            start: Cell { x: 5, y: 5 },
            light_level: LightLevel::Bright,
            all_portals,
            ..default()
        }
    }

    fn parse_portals(tiles: &[(TileIdx, Cell)]) -> Vec<PortalCell> {
        tiles
            .iter()
            .filter_map(|(idx, cell)| match *idx {
                TileIdx::StairsDown => Some((
                    Portal {
                        id: EntryId(format!("{:?}", TileIdx::StairsDown)),
                        arrive_at: EntryId(format!("{:?}", TileIdx::StairsUp)),
                    },
                    *cell,
                )),
                TileIdx::StairsUp => Some((
                    Portal {
                        id: EntryId(format!("{:?}", TileIdx::StairsUp)),
                        arrive_at: EntryId(format!("{:?}", TileIdx::StairsDown)),
                    },
                    *cell,
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    fn parse_map_str(map_str: &str) -> Vec<TileCell> {
        let lines: Vec<&str> = map_str.lines().collect();
        lines
            .iter()
            .enumerate()
            .flat_map(|(y, line)| {
                line.char_indices().filter_map(move |(x, c)| {
                    TilemapSpec::tile_for(c).map(|idx| {
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
            .collect::<Vec<_>>()
    }

    pub fn from_strs(one: &str, two: &str, start: Cell, light_level: LightLevel) -> Self {
        let id1 = StratumId(0);
        let id2 = StratumId(1);
        let tiles1 = TilemapSpec::parse_map_str(one);
        let portals1 = TilemapSpec::parse_portals(&tiles1);
        let tiles2 = TilemapSpec::parse_map_str(two);
        let portals2 = TilemapSpec::parse_portals(&tiles2);
        TilemapSpec {
            all_tiles: vec![(id1, tiles1), (id2, tiles2)]
                .into_iter()
                .collect::<HashMap<StratumId, Vec<TileCell>>>(),
            all_portals: vec![(id1, portals1), (id2, portals2)]
                .into_iter()
                .collect::<HashMap<StratumId, Vec<PortalCell>>>(),
            start,
            light_level,
            ..default()
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

        let all_tiles = vec![(
            StratumId(0),
            (0..tiles)
                .map(|i| {
                    let cell = Cell::from_idx(size.0, i as usize);
                    let tile_idx = fx(&cell, &table);
                    tally.entry(tile_idx).and_modify(|e| *e += 1).or_insert(1);
                    (tile_idx, cell)
                })
                .collect(),
        )]
        .into_iter()
        .collect();

        info!("tile breakdown: {:#?}", tally);

        TilemapSpec {
            size: Dimensions {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            all_tiles,
            layer: MAP_LAYER,
            start,
            light_level: LightLevel::Night,
            ..default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_match_string() {
        let spec = TilemapSpec::from_str("###\n...\n");
        assert_eq!(spec.size.width, 3);
        assert_eq!(spec.size.height, 2);
        assert_eq!(spec.size.tile_size, DEFAULT_TILE_SIZE);
    }

    #[test]
    fn jagged_map_uses_widest_line() {
        let spec = TilemapSpec::from_str("#\n###\n##\n");
        assert_eq!(spec.size.width, 3);
        assert_eq!(spec.size.height, 3);
    }

    #[test]
    fn empty_string_produces_empty_spec() {
        let spec = TilemapSpec::from_str("");
        assert_eq!(spec.size.width, 0);
        assert_eq!(spec.size.height, 0);
        assert!(spec.all_tiles[&StratumId(0)].is_empty());
    }

    #[test]
    fn spaces_and_unknown_chars_excluded() {
        // '?' (unknown) should produce no tiles
        let spec = TilemapSpec::from_str("?");
        assert!(spec.all_tiles[&StratumId(0)].is_empty());
    }

    #[test]
    fn character_mappings() {
        // One of each known character on a single row; check tile indices in order
        let spec = TilemapSpec::from_str("#.XDObwTtUu");
        let tile_types: Vec<TileIdx> = spec
            .all_tiles
            .values()
            .flatten()
            .map(|(idx, _)| *idx)
            .collect();
        assert_eq!(
            tile_types,
            vec![
                TileIdx::StoneWall,
                TileIdx::Blank, // '.'
                TileIdx::Blank, // 'X' also maps to Blank
                TileIdx::DoorBrownThickClosed1,
                TileIdx::DoorwayBrownThick,
                TileIdx::ChestBrownClosed,
                TileIdx::ChestWhiteClosed,
                TileIdx::GreenTree1,
                TileIdx::GreenTree2,
                TileIdx::DoubleGreenTree1,
                TileIdx::DoubleGreenTree2,
            ]
        );
    }

    #[test]
    fn cell_coordinates_match_col_row() {
        // "#." on row 0 → wall at (0,0), blank at (1,0)
        // ".#" on row 1 → blank at (0,1), wall at (1,1)
        let spec = TilemapSpec::from_str("#.\n.#");
        let tiles = &spec.all_tiles;
        let id = StratumId(0);
        assert_eq!(tiles[&id][0], (TileIdx::StoneWall, Cell { x: 0, y: 0 }));
        assert_eq!(tiles[&id][1], (TileIdx::Blank, Cell { x: 1, y: 0 }));
        assert_eq!(tiles[&id][2], (TileIdx::Blank, Cell { x: 0, y: 1 }));
        assert_eq!(tiles[&id][3], (TileIdx::StoneWall, Cell { x: 1, y: 1 }));
    }

    #[test]
    fn start_is_hardcoded_regardless_of_x_position() {
        // 'X' marks the intended start in ASCII but from_str ignores its position;
        // start is always hardcoded to (5, 5).
        let spec = TilemapSpec::from_str("X..\n...\n...");
        assert_eq!(spec.start, Cell { x: 5, y: 5 });
    }

    #[test]
    fn layer_uses_default() {
        let spec = TilemapSpec::from_str("#");
        assert_eq!(spec.layer, MAP_LAYER);
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct SyncProps {
    _mt: &'static MapTile,
    tile_preview: Option<&'static TilePreview>,
    walkable: Option<&'static Walkable>,
    opaque: Option<&'static Opaque>,
    pickable: Option<&'static Pickable>,
}

/// Sync [TileIdx] and [Sprite] visuals along with their gameplay properties.
pub fn sync_tiles(
    mut commands: Commands,
    mut tiles: Query<
        (Entity, &mut Sprite, &TileIdx, SyncProps),
        Or<(Changed<TileIdx>, Changed<TilePreview>)>,
    >,
) {
    // This method only runs when [TileIdx] or [TilePreview] changes, so
    // we apply most changes in some unconditional fashion.
    for (entity, mut sprite, tile_idx, sync_props) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);

        let preview_opt = sync_props.tile_preview;
        let walkable_opt = sync_props.walkable;
        let opaque_opt = sync_props.opaque;
        let pickable_opt = sync_props.pickable;

        // If there's a preview, we should apply that tile index instead.
        let next_idx = preview_opt.and_then(|it| it.get()).unwrap_or(*tile_idx);
        // Apply the texture atlas index unconditionally.
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = next_idx.into();
        }

        // Update tile Walkable only when necessary.
        // TODO: consider whether to split this out or not.
        if tile_idx.is_walkable() && walkable_opt.is_none() {
            entity_command.insert(Walkable);
        } else if !tile_idx.is_walkable() && walkable_opt.is_some() {
            entity_command.remove::<Walkable>();
        }

        // Update tile Opaque only when necessary.
        // TODO: consider whether to split this out or not.
        if tile_idx.is_transparent() && opaque_opt.is_some() {
            entity_command.remove::<Opaque>();
        } else if !tile_idx.is_transparent() && opaque_opt.is_none() {
            entity_command.insert(Opaque);
        }

        if pickable_opt.is_none() {
            entity_command.insert(Pickable {
                should_block_lower: false,
                is_hoverable: true,
            });
        }
    }
}

/// Sync [MapTile] [Sprite] visual effects with the tile's logical state. This is orthogonal to [TileIdx].
/// TODO: consider whether or how function signature might be simplified.
pub fn update_tile_visuals(
    mut tiles: Query<(&mut Sprite, &mut Visibility, VisualProps)>,
    spec: Res<TilemapSpec>,
) {
    for (mut sprite, mut vis, p) in tiles.iter_mut() {
        *vis = if p.revealed() && !p.occupied() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        sprite.color = if p.highlighted() {
            colors::KENNEY_GOLD
        } else if p.revealed() && !p.occupied() {
            Color::WHITE.with_alpha(p.light_or(&spec.light_level).into())
        } else {
            Color::NONE
        };

        if p.preview_active() {
            sprite.color.set_alpha(0.5);
        }
    }
}

#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct VisualProps {
    _mt: &'static MapTile,
    occupied: Option<&'static Occupied>,
    highlighted: Option<&'static Highlighted>,
    revealed: Option<&'static Revealed>,
    tile_preview: Option<&'static TilePreview>,
    light_level: Option<&'static LightLevel>,
}

impl<'w, 's> VisualPropsItem<'w, 's> {
    pub fn revealed(&self) -> bool {
        self.revealed.is_some_and(|r| r.0)
    }

    pub const fn highlighted(&self) -> bool {
        self.highlighted.is_some()
    }

    pub fn preview_active(&self) -> bool {
        self.tile_preview.is_some_and(TilePreview::is_active)
    }

    pub fn light_or(&self, other: &LightLevel) -> LightLevel {
        *self.light_level.unwrap_or(other)
    }

    pub const fn occupied(&self) -> bool {
        self.occupied.is_some()
    }
}
