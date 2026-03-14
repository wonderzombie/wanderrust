use std::collections::HashMap;

use crate::Actor;
use crate::tiles::{MapTile, Revealed};
use crate::{cell::Cell, tilemap::TileStorage};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LightLevel {
    Dark, // underground default — render nothing
    #[default]
    Night, // default for nighttime; not quite dark
    Dim,  // the outer edge of a lantern or torch
    Light, // normal non-magical light
    Bright, // noon sun, magical light source
}

impl From<LightLevel> for f32 {
    fn from(value: LightLevel) -> Self {
        match value {
            LightLevel::Dark => 0.0,
            LightLevel::Night => 0.1,
            LightLevel::Dim => 0.4,
            LightLevel::Light => 0.7,
            LightLevel::Bright => 1.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LightRing {
    level: LightLevel,
    thickness: i32,
}

impl From<(LightLevel, i32)> for LightRing {
    fn from((level, thickness): (LightLevel, i32)) -> Self {
        Self { level, thickness }
    }
}

#[derive(Component, Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Emitter {
    inner: LightRing,
    outer: LightRing,
}

impl From<(LightRing, LightRing)> for Emitter {
    fn from(value: (LightRing, LightRing)) -> Self {
        Emitter::from_rings(value.0, value.1)
    }
}

impl Emitter {
    pub fn new(inner: (LightLevel, i32), outer: (LightLevel, i32)) -> Self {
        Emitter::from_rings(LightRing::from(inner), LightRing::from(outer))
    }

    fn from_rings(inner: LightRing, outer: LightRing) -> Self {
        debug_assert!(
            inner.level >= outer.level,
            "inner ring level must be >= outer ring level"
        );
        Self { inner, outer }
    }

    #[allow(dead_code)]
    fn total_radius(&self) -> i32 {
        self.inner.thickness + self.outer.thickness
    }

    /// Returns all cells lit by this emitter, paired with their light level.
    /// Follows D&D 5e light semantics: inner covers radius `inner.thickness` from the origin,
    /// outer covers an *additional* `outer.thickness` beyond that.
    /// E.g. "bright light for 1 tile, dim light for an additional 1 tile" →
    /// `Emitter::new((Bright, 1), (Dim, 1))`.
    pub fn light_cells(&self, origin: Cell) -> LightMap {
        let outer_radius = self.inner.thickness + self.outer.thickness;
        let mut cell_map = HashMap::default();
        for dx in -outer_radius..=outer_radius {
            for dy in -outer_radius..=outer_radius {
                let cell = Cell::new(origin.x + dx, origin.y + dy);
                let dist = cell.as_vec().distance(origin.as_vec());
                if dist <= outer_radius as f32 {
                    let level = if dist <= self.inner.thickness as f32 {
                        self.inner.level
                    } else {
                        self.outer.level
                    };
                    cell_map
                        .entry(cell)
                        .and_modify(|prev| {
                            *prev = level.max(*prev);
                        })
                        .or_insert(level);
                }
            }
        }
        LightMap(cell_map)
    }
}

#[derive(Component, Default, Deref, Debug, Clone)]
pub struct LightMap(pub HashMap<Cell, LightLevel>);

impl LightMap {
    /// Combines two light maps, choosing the brighter level for each cell.
    pub fn merge_with(&mut self, other: LightMap) {
        other.0.into_iter().for_each(|(cell, level)| {
            self.0
                .entry(cell)
                .and_modify(|prev| {
                    *prev = level.max(*prev);
                })
                .or_insert(level);
        });
    }
}

pub fn update_emitter_lights(
    mut commands: Commands,
    changed_emitters: Query<&Emitter, Changed<Cell>>,
    all_emitters: Query<(Entity, &Emitter, &Cell)>,
    storage: Single<&TileStorage>,
    mut prev_map: Local<LightMap>,
) {
    if changed_emitters.is_empty() {
        return;
    }

    let mut new_combined_map = LightMap(HashMap::new());
    for (entity, emitter, &cell) in all_emitters.iter() {
        let next_light_map = emitter.light_cells(cell);
        // Important: this is a deferred operation. We are storing the light map directly on the entity
        // but this won't take effect immediately.
        commands.entity(entity).insert(next_light_map.clone());

        // This populates the map combining each emitter's output, choosing the
        // brighter value when two emitters address the same cell.
        new_combined_map.merge_with(next_light_map);
    }

    let old_map_cells: HashSet<Cell> = prev_map.0.keys().copied().collect();
    let new_map_cells: HashSet<Cell> = new_combined_map.keys().copied().collect();

    // Any cells in the old map that aren't in the new one are 1) lit and 2) should not be lit.
    old_map_cells
        .difference(&new_map_cells)
        .filter_map(|c| {
            let tile = storage.get(c)?;
            Some(tile)
        })
        .for_each(|tile| {
            commands.entity(tile).remove::<LightLevel>();
        });

    // Any cells in the new map that aren't in the old one are 1) unlit and 2) should be lit.
    new_map_cells
        .difference(&old_map_cells)
        .filter_map(|c| {
            let tile = storage.get(c)?;
            let level = new_combined_map.get(c)?;
            Some((level, tile))
        })
        .for_each(|(level, tile)| {
            commands.entity(tile).insert(*level);
        });

    // Any cells that are still lit may need to be updated; these are 1) lit, and 2) should be lit *differently*.
    // Tiles here may need to be updated with a different light intensity.
    old_map_cells
        .intersection(&new_map_cells)
        .filter_map(|c| {
            let tile = storage.get(c)?;
            let new_level = new_combined_map.get(c)?;
            let old_level = prev_map.0.get(c)?;

            return (old_level != new_level).then_some((tile, new_level));
        })
        .for_each(|(tile, new_level)| {
            commands.entity(tile).insert(*new_level);
        });

    *prev_map = new_combined_map;
}

pub fn sync_tile_light_levels(
    mut commands: Commands,
    light_maps: Query<&LightMap, Changed<LightMap>>,
    tiles: Single<&TileStorage>,
) {
    for light_map in light_maps.iter() {
        for (cell, level) in light_map.0.iter() {
            if let Some(tile) = tiles.get(cell) {
                commands.entity(tile).insert(*level);
            }
        }
    }
}

pub fn sync_actor_light_levels(
    storage: Single<&TileStorage>,
    light_levels: Query<(&LightLevel, &Revealed), With<MapTile>>,
    actors: Query<(&mut Sprite, &Cell), Without<MapTile>>,
) {
    // Actor entities should have the same LightLevel as the tile they are standing on.
    for (mut sprite, cell) in actors {
        let Some(tile) = storage.get(cell) else {
            continue;
        };
        let Ok((level, revealed)) = light_levels.get(tile) else {
            continue;
        };

        if revealed.0 {
            sprite.color = Color::WHITE.with_alpha((*level).into());
        } else {
            sprite.color = Color::BLACK.with_alpha(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LightLevel::*;

    fn emitter(inner_r: i32, outer_r: i32) -> Emitter {
        Emitter::new((Bright, inner_r), (Dim, outer_r))
    }

    fn cells_with_level(emitter: &Emitter, origin: Cell, level: LightLevel) -> Vec<Cell> {
        emitter
            .light_cells(origin)
            .iter()
            .filter(|&(_, l)| *l == level)
            .map(|(&c, _)| c)
            .collect()
    }

    #[test]
    fn origin_always_receives_inner_level() {
        let e = emitter(1, 1);
        let lit = e.light_cells(Cell::new(0, 0));
        let origin_level = lit
            .iter()
            .find(|&(c, _)| c == &Cell::new(0, 0))
            .map(|(_, l)| *l);
        assert_eq!(origin_level, Some(Bright));
    }

    #[test]
    fn inner_radius_zero_lights_only_origin() {
        // inner=0 means only the origin tile is Bright; outer=1 adds one ring of Dim.
        let e = emitter(0, 1);
        let bright = cells_with_level(&e, Cell::new(0, 0), Bright);
        assert_eq!(bright, vec![Cell::new(0, 0)]);
    }

    #[test]
    fn outer_is_additive_beyond_inner() {
        // inner=1 covers dist ≤ 1; outer=1 adds dist ≤ 2.
        // Any cell at dist > 1 and ≤ 2 must be Dim, not Bright.
        let e = emitter(1, 1);
        let origin = Cell::new(5, 5);
        let lit = e.light_cells(origin);

        for (cell, level) in lit.0 {
            let dist = cell.as_vec().distance(origin.as_vec());
            let expected = if dist <= 1.0 { Bright } else { Dim };
            assert_eq!(
                level, expected,
                "cell {:?} at dist {:.2} should be {:?}",
                cell, dist, expected
            );
        }
    }

    #[test]
    fn no_duplicate_cells() {
        let e = emitter(2, 2);
        let lit = e.light_cells(Cell::new(0, 0));
        let mut seen = std::collections::HashSet::new();
        for (cell, _) in lit.0 {
            assert!(seen.insert(cell), "duplicate cell {:?}", cell);
        }
    }

    #[test]
    fn all_cells_within_total_radius() {
        let e = emitter(1, 2); // total radius = 3
        let origin = Cell::new(0, 0);
        let lit = e.light_cells(origin);
        for (cell, _) in lit.0 {
            let dist = cell.as_vec().distance(origin.as_vec());
            assert!(
                dist <= e.total_radius() as f32,
                "cell {:?} at dist {:.2} exceeds total radius",
                cell,
                dist
            );
        }
    }

    #[test]
    fn inner_only_emitter_has_no_outer_cells() {
        // outer thickness = 0 means no dim ring at all.
        let e = emitter(2, 0);
        let dim = cells_with_level(&e, Cell::new(0, 0), Dim);
        assert!(dim.is_empty(), "expected no Dim cells but got {:?}", dim);
    }

    #[test]
    fn light_cells_offset_by_origin() {
        // Results should be the same shape regardless of where the origin is.
        let e = emitter(1, 1);
        let at_zero = e.light_cells(Cell::new(0, 0));
        let at_ten = e.light_cells(Cell::new(10, 10));

        assert_eq!(at_zero.len(), at_ten.len());

        // Every cell from at_zero shifted by (10,10) should match at_ten.
        let shifted: HashMap<Cell, LightLevel> = at_zero
            .iter()
            .map(|(c, l)| (Cell::new(c.x + 10, c.y + 10), *l))
            .collect();
        let reference = at_ten.clone();
        assert_eq!(shifted, reference.0);
    }
}
