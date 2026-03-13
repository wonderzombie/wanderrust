use crate::cell::Cell;
use crate::colors;
use crate::light::LightLevel;
use crate::ptable::ProbabilityTable;
use crate::tilemap::{MapDimensions, TilemapId, TilemapSpec};
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
        (' ', TileIdx::Blank),
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

        let tiles = lines
            .iter()
            .enumerate()
            .flat_map(|(y, line)| {
                line.chars().enumerate().filter_map(move |(x, ch)| {
                    TilemapSpec::tile_for(ch).map(|idx| {
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
            size: MapDimensions {
                width,
                height,
                tile_size: DEFAULT_TILE_SIZE,
            },
            tiles,
            layer: DEFAULT_LAYER,
            start: Cell { x: 5, y: 5 },
            light_level: LightLevel::Dark,
            ..Default::default()
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
            size: MapDimensions {
                width: size.0,
                height: size.1,
                tile_size: DEFAULT_TILE_SIZE,
            },
            tiles,
            layer: DEFAULT_LAYER,
            start,
            light_level: LightLevel::Light,
            ..Default::default()
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
        assert!(spec.tiles.is_empty());
    }

    #[test]
    fn spaces_and_unknown_chars_excluded() {
        // ' ' (empty space) and '?' (unknown) should produce no tiles
        let spec = TilemapSpec::from_str(" ?");
        assert!(spec.tiles.is_empty());
    }

    #[test]
    fn character_mappings() {
        // One of each known character on a single row; check tile indices in order
        let spec = TilemapSpec::from_str("#.XDObwTtUu");
        let tile_types: Vec<TileIdx> = spec.tiles.iter().map(|(idx, _)| *idx).collect();
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
        let tiles = &spec.tiles;
        assert_eq!(tiles[0], (TileIdx::StoneWall, Cell { x: 0, y: 0 }));
        assert_eq!(tiles[1], (TileIdx::Blank, Cell { x: 1, y: 0 }));
        assert_eq!(tiles[2], (TileIdx::Blank, Cell { x: 0, y: 1 }));
        assert_eq!(tiles[3], (TileIdx::StoneWall, Cell { x: 1, y: 1 }));
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
        assert_eq!(spec.layer, DEFAULT_LAYER);
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
    // This method only runs when [TileIdx] or [TilePreview] changes, so
    // we apply most changes in some unconditional fashion.
    for (entity, mut sprite, tile_idx, preview_opt) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);

        // If there's a preview, we should apply that tile index instead.
        let next_idx = preview_opt.and_then(|it| it.get()).unwrap_or(*tile_idx);
        // Apply the texture atlas index unconditionally.
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = next_idx.into();
        }

        // Walkable may be added/removed unconditionally based on [TileIdx].
        // TODO: consider whether to split this out or not.
        if tile_idx.is_walkable() {
            entity_command.insert(Walkable);
        } else {
            entity_command.remove::<Walkable>();
        }

        // Opaque may be added/removed unconditionally based on [TileIdx].
        // TODO: consider whether to split this out or not.
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
            &mut Visibility,
            Option<&Highlighted>,
            &Revealed,
            Option<&TilePreview>,
            Option<&LightLevel>,
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
    map_spec: Res<TilemapSpec>,
) {
    for (mut sprite, mut vis, highlighted, revealed, preview_opt, light_level) in tiles.iter_mut() {
        let revealed = revealed.0;
        let highlighted = highlighted.is_some();
        let light_level = light_level.copied().unwrap_or(map_spec.light_level);

        *vis = if revealed {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        sprite.color = if highlighted {
            colors::KENNEY_GOLD
        } else if revealed {
            match light_level {
                LightLevel::Bright => Color::WHITE,
                LightLevel::Light => Color::WHITE.with_alpha(0.75),
                LightLevel::Dim => Color::WHITE.with_alpha(0.5),
                LightLevel::Night => Color::WHITE.with_alpha(0.25),
                LightLevel::Dark => Color::WHITE.with_alpha(0.0),
            }
        } else {
            Color::NONE
        };

        if preview_opt.is_some_and(TilePreview::is_active) {
            sprite.color.set_alpha(0.5);
        }
    }
}
