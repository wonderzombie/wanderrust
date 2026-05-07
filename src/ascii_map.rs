use crate::{cell::Cell, light::LightLevel, ptable::ProbabilityTable, tilemap::*, tiles::TileIdx};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub type LevelTiles = HashMap<LevelId, Vec<TileCell>>;
pub type LevelPortals = HashMap<LevelId, Vec<PortalCell>>;
pub type LevelInterxs = HashMap<LevelId, Vec<InterxCell>>;
pub type LevelEmitters = HashMap<LevelId, Vec<EmitterCell>>;

/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
/// Deprecated in favor of LevelSpec.
#[derive(Resource, Default, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Resource)]
pub struct AsciiMapSpec {
    pub size: Dimensions,
    /// Tiles and portals keyed by LevelId drive tilemap creation.
    pub all_tiles: LevelTiles,
    pub all_portals: LevelPortals,
    pub all_interxs: LevelInterxs,
    pub all_emitters: LevelEmitters,
    /// Starting point for the player.
    pub spawn_point: SpawnCell,
    /// The minimum light level for the area.
    pub light_level: LightLevel,
}

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

#[allow(dead_code)]
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

pub const DEFAULT_TILE_SIZE: u32 = 16;

impl From<AsciiMapSpec> for WorldSpec {
    fn from(value: AsciiMapSpec) -> Self {
        let mut out = WorldSpec::default();

        let incoming_levels = value.all_tiles.keys();

        for level_id in incoming_levels {
            let outgoing_map: &mut LevelSpec = out.maps.entry(*level_id).or_default();
            if let Some(tiles) = value.all_tiles.get(level_id) {
                outgoing_map.tiles.extend(tiles);
            }

            if let Some(portals) = value.all_portals.get(level_id) {
                outgoing_map.portals.extend(portals.iter().map(|(p, t, c)| {
                    let mut p = p.clone();
                    p.tile_idx = *t;
                    (p, *c)
                }));
            }

            if let Some(emitters) = value.all_emitters.get(level_id) {
                outgoing_map
                    .emitters
                    .extend(emitters.iter().map(|(e, t, c)| {
                        let mut e = *e;
                        e.tile_idx = *t;
                        (e, *c)
                    }));
            }

            if let Some(interxs) = value.all_interxs.get(level_id) {
                outgoing_map.interxs.extend(interxs.iter().map(|(i, t, c)| {
                    let i = i.set_tile(*t);
                    (i, *c)
                }));
            }

            outgoing_map.light_level = value.light_level;
            outgoing_map.dimensions = value.size;
        }

        dbg!(out)
    }
}

impl AsciiMapSpec {
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
        AsciiMapSpec::KEY
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

        let id = LevelId(0);
        let all_tiles: HashMap<LevelId, Vec<TileCell>> =
            vec![(id, AsciiMapSpec::parse_map_str(map_str))]
                .into_iter()
                .collect();

        let all_portals = vec![(id, AsciiMapSpec::parse_portals(&all_tiles[&id]))]
            .into_iter()
            .collect();

        AsciiMapSpec {
            size: Dimensions {
                width,
                height,
                tile_size: DEFAULT_TILE_SIZE,
            },
            all_tiles,
            spawn_point: (LevelId::default(), Cell { x: 5, y: 5 }),
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
                        tile_idx: *idx,
                    },
                    *idx,
                    *cell,
                )),
                TileIdx::StairsUp => Some((
                    Portal {
                        id: EntryId(format!("{:?}", TileIdx::StairsUp)),
                        arrive_at: EntryId(format!("{:?}", TileIdx::StairsDown)),
                        tile_idx: *idx,
                    },
                    *idx,
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
                    AsciiMapSpec::tile_for(c).map(|idx| {
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

    #[allow(dead_code)]
    pub fn from_strs(one: &str, two: &str, spawn_cell: Cell, light_level: LightLevel) -> Self {
        let id1 = LevelId(0);
        let id2 = LevelId(1);
        let tiles1 = AsciiMapSpec::parse_map_str(one);
        let portals1 = AsciiMapSpec::parse_portals(&tiles1);
        let tiles2 = AsciiMapSpec::parse_map_str(two);
        let portals2 = AsciiMapSpec::parse_portals(&tiles2);
        let spawn_point = (LevelId::default(), spawn_cell);
        AsciiMapSpec {
            all_tiles: vec![(id1, tiles1), (id2, tiles2)]
                .into_iter()
                .collect::<HashMap<LevelId, Vec<TileCell>>>(),
            all_portals: vec![(id1, portals1), (id2, portals2)]
                .into_iter()
                .collect::<HashMap<LevelId, Vec<PortalCell>>>(),
            spawn_point,
            light_level,
            ..default()
        }
    }

    pub fn with_ptable(
        table: ProbabilityTable,
        fx: impl Fn(&Cell, &ProbabilityTable) -> TileIdx,
        size: (u32, u32),
    ) -> Self {
        let spawn_cell = Cell {
            x: size.0 as i32 / 2,
            y: size.1 as i32 / 2,
        };
        info!("=== map from procedure ===");
        let tiles = size.0 * size.1;
        info!("spawn_point: {:?}", spawn_cell);
        info!("size: {:?}", size);

        let spawn_point = (LevelId::default(), spawn_cell);

        let mut tally: HashMap<TileIdx, usize> = HashMap::new();

        let all_tiles = vec![(
            LevelId(0),
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

        AsciiMapSpec {
            size: Dimensions {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            all_tiles,
            spawn_point,
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
        let spec = AsciiMapSpec::from_str("###\n...\n");
        assert_eq!(spec.size.width, 3);
        assert_eq!(spec.size.height, 2);
        assert_eq!(spec.size.tile_size, DEFAULT_TILE_SIZE);
    }

    #[test]
    fn jagged_map_uses_widest_line() {
        let spec = AsciiMapSpec::from_str("#\n###\n##\n");
        assert_eq!(spec.size.width, 3);
        assert_eq!(spec.size.height, 3);
    }

    #[test]
    fn empty_string_produces_empty_spec() {
        let spec = AsciiMapSpec::from_str("");
        assert_eq!(spec.size.width, 0);
        assert_eq!(spec.size.height, 0);
        assert!(spec.all_tiles[&LevelId(0)].is_empty());
    }

    #[test]
    fn spaces_and_unknown_chars_excluded() {
        // '?' (unknown) should produce no tiles
        let spec = AsciiMapSpec::from_str("?");
        assert!(spec.all_tiles[&LevelId(0)].is_empty());
    }

    #[test]
    fn character_mappings() {
        // One of each known character on a single row; check tile indices in order
        let spec = AsciiMapSpec::from_str("#.XDObwTtUu");
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
        let spec = AsciiMapSpec::from_str("#.\n.#");
        let tiles = &spec.all_tiles;
        let id = LevelId(0);
        assert_eq!(tiles[&id][0], (TileIdx::StoneWall, Cell { x: 0, y: 0 }));
        assert_eq!(tiles[&id][1], (TileIdx::Blank, Cell { x: 1, y: 0 }));
        assert_eq!(tiles[&id][2], (TileIdx::Blank, Cell { x: 0, y: 1 }));
        assert_eq!(tiles[&id][3], (TileIdx::StoneWall, Cell { x: 1, y: 1 }));
    }

    #[test]
    fn start_is_hardcoded_regardless_of_x_position() {
        // 'X' marks the intended start in ASCII but from_str ignores its position;
        // start is always hardcoded to (5, 5).
        let spec = AsciiMapSpec::from_str("X..\n...\n...");
        let (_, spawn_cell) = spec.spawn_point;
        assert_eq!(spawn_cell, Cell { x: 5, y: 5 });
    }
}
