use std::collections::HashMap;

use crate::{
    actors::PieceBundle,
    atlas::SpriteAtlas,
    cell::Cell,
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    tilemap::{Stratum, StratumTileSpec, TileStorage},
    tiles::{MapTile, Revealed, TileIdx},
};
use bevy::{platform::collections::HashSet, prelude::*};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(
    Component,
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
)]
pub enum LightLevel {
    Dark, // underground default — render nothing
    #[default]
    Night, // default for nighttime; not quite dark
    Dim,  // the outer edge of a lantern or torch
    Light, // normal non-magical light
    Bright, // noon sun, magical light source
}

impl LightLevel {
    pub fn from_str(s: impl AsRef<str>) -> Option<LightLevel> {
        use LightLevel::*;
        match s.as_ref() {
            "dark" => Some(Dark),
            "night" => Some(Night),
            "dim" => Some(Dim),
            "light" => Some(Light),
            "bright" => Some(Bright),
            _ => None,
        }
    }
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
struct LightRing {
    level: LightLevel,
    thickness: i32,
}

impl From<(LightLevel, i32)> for LightRing {
    fn from((level, thickness): (LightLevel, i32)) -> Self {
        Self { level, thickness }
    }
}

#[derive(
    Component, Default, Debug, Copy, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
pub struct Emitter {
    inner: LightRing,
    outer: LightRing,
    default_tile_idx: TileIdx,
}

impl Emitter {
    pub fn new(tile_idx: TileIdx, inner: (LightLevel, i32), outer: (LightLevel, i32)) -> Self {
        Emitter::from_rings(tile_idx, LightRing::from(inner), LightRing::from(outer))
    }

    fn from_rings(tile_idx: TileIdx, inner: LightRing, outer: LightRing) -> Self {
        Self {
            inner,
            outer,
            default_tile_idx: tile_idx,
        }
    }

    fn from_tile(tile_idx: &TileIdx) -> Option<Self> {
        if tile_idx.is_emitter() {
            match tile_idx {
                TileIdx::Torch => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Light, 1),
                        (LightLevel::Dim, 1),
                    ));
                }
                TileIdx::Candle => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Dim, 1),
                        (LightLevel::Night, 1),
                    ));
                }
                TileIdx::Brazier => {
                    return Some(Emitter::new(
                        *tile_idx,
                        (LightLevel::Light, 2),
                        (LightLevel::Dim, 1),
                    ));
                }
                _ => {
                    error!("tile is emitter but unrecognized: {:?}", tile_idx);
                    return None;
                }
            }
        }
        None
    }

    // Used in test.
    #[allow(dead_code)]
    fn total_radius(&self) -> i32 {
        self.inner.thickness + self.outer.thickness
    }

    /// Returns all cells lit by this emitter, paired with their light level.
    /// Follows D&D 5e light semantics: inner covers radius `inner.thickness` from the origin,
    /// outer covers an *additional* `outer.thickness` beyond that.
    /// E.g. "bright light for 1 tile, dim light for an additional 1 tile" →
    /// `Emitter::new((Bright, 1), (Dim, 1))`.
    pub fn light_cells(&self, origin: &Cell) -> LightMap {
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

impl LdtkEntityExt<Emitter> for Emitter {
    fn from_ldtk(entity: &LdtkEntity) -> Option<Emitter> {
        if entity.ty().is_none_or(|it| it != LdtkActor::Emitter) {
            warn!("entity unknown or not an emitter: {:?}", entity);
            return None;
        }
        Emitter::from_tile(&entity.get_tile())
    }
}

/// A map of cells to [`LightLevel`] values, representing the light emitted by an [`Emitter`].
#[derive(Component, Default, Deref, Debug, Clone, Reflect, PartialEq)]
pub struct LightMap(pub HashMap<Cell, LightLevel>);

impl LightMap {
    /// Combines [`LightMap`]s, choosing the brighter level for each cell.
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

#[derive(Component, Default, Debug, Reflect)]
pub struct StratumLightMap {
    pub curr: LightMap,
    pub prev: LightMap,
    pub default: LightLevel,
}

impl StratumLightMap {
    pub fn with_ambient(level: LightLevel) -> Self {
        Self {
            curr: LightMap::default(),
            prev: LightMap::default(),
            default: level,
        }
    }

    /// Applies this StratumLightMap to the given [`TileStorage`].
    pub fn apply(&self, commands: &mut Commands, storage: &TileStorage) {
        let prev: HashSet<Cell> = self.prev.keys().copied().collect();
        let curr: HashSet<Cell> = self.curr.keys().copied().collect();

        // Cells in the old [`LightMap`] that aren't in the new one are no longer
        // lit at all. We apply the [`light_level`] from [`TilemapSpec`]
        // accordingly.
        prev.difference(&curr)
            .filter_map(|c| {
                let tile = storage.get(c)?;
                Some(tile)
            })
            .for_each(|tile| {
                commands.entity(tile).insert(self.default);
            });

        // Cells in the new [`LightMap`] that aren't in the old one receive light
        // from the emitter. These cells probably had the default light level for
        // the area before this. The map has already handled overlapping emitters,
        // so we apply the map.
        curr.difference(&prev)
            .filter_map(|c| {
                let tile = storage.get(c)?;
                let level = self.curr.get(c)?;
                Some((level, tile))
            })
            .for_each(|(level, tile)| {
                commands.entity(tile).insert(*level);
            });

        // Cells in the old [`LightMap`] that *are* in the new map *may* need to
        // change intensity. When two overlapping emitters have different light
        // levels and one moves away, we restore tiles to the light level from the
        // lower-intensity emitter.
        prev.intersection(&curr)
            .filter_map(|c| {
                let tile = storage.get(c)?;
                let new_level = self.curr.get(c)?;
                let old_level = self.prev.get(c)?;

                (old_level != new_level).then_some((tile, new_level))
            })
            .for_each(|(tile, new_level)| {
                commands.entity(tile).insert(*new_level);
            });
    }
}

pub fn spawn(
    mut commands: Commands,
    spec: Res<StratumTileSpec>,
    atlas: Res<SpriteAtlas>,
    strata: Query<&Stratum>,
) {
    info!(
        "🔥 spawning emitters for {} strata",
        spec.all_emitters.len()
    );
    for (stratum_id, emitters) in spec.all_emitters.iter() {
        info!(
            "🔥 strat {:?} has {:?} emitters",
            stratum_id,
            emitters.len()
        );
        let Some(Stratum(strat_entity, _)) = strata.iter().find(|Stratum(_, id)| id == stratum_id)
        else {
            warn!("🔥 unable to find stratum with id {:?}", stratum_id);
            continue;
        };

        let mut count = 0;
        for (emitter, _, cell) in emitters.iter() {
            count += 1;
            trace!("🔥 spawning {:?} at {:?}", emitter, cell);
            commands.spawn((
                *emitter,
                emitter.default_tile_idx,
                ChildOf(*strat_entity),
                PieceBundle {
                    sprite: atlas.sprite_from_idx(emitter.default_tile_idx),
                    cell: *cell,
                    ..default()
                },
            ));
        }
        if count > 0 {
            info!("🔥 spawned {} emitters", count);
        }
    }
}

pub fn setup(
    mut commands: Commands,
    emitter_tiles: Query<(Entity, &TileIdx), Changed<TileIdx>>,
    storage: Query<Entity, With<TileStorage>>,
    spec: Res<StratumTileSpec>,
) {
    let mut count = 0;
    for (entity, tile_idx) in emitter_tiles {
        let Some(emitter) = Emitter::from_tile(tile_idx) else {
            continue;
        };
        commands.entity(entity).insert(emitter);
        count += 1;
    }
    if count > 0 {
        info!("🔥 set up {} emitter tiles", count);
    }

    count = 0;
    for entity in storage.iter() {
        commands
            .entity(entity)
            .insert(StratumLightMap::with_ambient(spec.light_level));
        count += 1;
    }
    if count > 0 {
        info!("🔥 set up {} strata light maps", count);
    }
}

pub fn update_emitter_maps(
    mut commands: Commands,
    emitters: Query<(Entity, &Emitter, &Cell), Or<(Changed<Emitter>, Changed<Cell>)>>,
) {
    let mut count = 0;
    for (entity, emitter, cell) in emitters.iter() {
        commands.entity(entity).insert(emitter.light_cells(cell));
        count += 1;
    }

    if count > 0 {
        trace!("🔥 updated {} emitter maps", count);
    }
}

pub fn update_strata_maps(
    mut all_strata: Query<&mut StratumLightMap>,
    all_emitter_maps: Query<(&ChildOf, &LightMap)>,
) {
    if all_emitter_maps.is_empty() {
        return;
    }

    let maps_by_stratum = all_emitter_maps
        .iter()
        .map(|(child_of, light_map)| (child_of.parent(), light_map))
        .into_group_map();

    for (stratum_entity, light_maps) in maps_by_stratum {
        let merged: LightMap = light_maps.into_iter().fold(
            LightMap(HashMap::new()),
            |mut stratum_map, emitter_map| {
                stratum_map.merge_with(emitter_map.clone());
                stratum_map
            },
        );

        // If the this new merged map isn't different than current, skip.
        if let Ok(stratum_map) = all_strata.get(stratum_entity) {
            if stratum_map.curr == merged {
                continue;
            }
        } else {
            panic!("unable to find stratum for {:?}", stratum_entity);
        }

        let mut stratum_map = all_strata.get_mut(stratum_entity).unwrap_or_else(|_| {
            panic!("unable to get stratum map for entity {:?}", stratum_entity)
        });
        stratum_map.prev = stratum_map.curr.clone();
        stratum_map.curr = merged;
    }
}

pub fn update_strata_light_levels(
    mut commands: Commands,
    all_strata_maps: Query<(&TileStorage, &StratumLightMap), Changed<StratumLightMap>>,
) {
    for (storage, light_map) in all_strata_maps.iter() {
        light_map.apply(&mut commands, storage);
    }
}

pub fn sync_actor_light_levels(
    spec: Res<StratumTileSpec>,
    storages: Query<&TileStorage>,
    lit_tiles: Query<&LightLevel, With<MapTile>>,
    revealed_tiles: Query<&Revealed, With<MapTile>>,
    actors: Query<(&mut Sprite, &Cell, &mut Visibility, &ChildOf), Without<MapTile>>,
) {
    // Actor entities should have the same LightLevel as the tile they are standing on.
    for (mut actor_sprite, actor_cell, mut actor_vis, child_of) in actors {
        let Some(actor_tile) = storages
            .get(child_of.0)
            .ok()
            .and_then(|v| v.get(actor_cell))
        else {
            warn!("no stratum or cell found for actor: {:?}", actor_cell);
            continue;
        };

        let revealed = revealed_tiles
            .get(actor_tile)
            .ok()
            .copied()
            .unwrap_or_default();

        let level = lit_tiles
            .get(actor_tile)
            .ok()
            .copied()
            .unwrap_or(spec.light_level);

        actor_vis.set_if_neq(if revealed.0 {
            actor_sprite.color = Color::WHITE.with_alpha(level.into());
            Visibility::Inherited
        } else {
            actor_sprite.color = Color::BLACK.with_alpha(0.0);
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LightLevel::*;

    fn emitter(inner_r: i32, outer_r: i32) -> Emitter {
        Emitter::new(TileIdx::Candle, (Bright, inner_r), (Dim, outer_r))
    }

    fn cells_with_level(emitter: &Emitter, origin: Cell, level: LightLevel) -> Vec<Cell> {
        emitter
            .light_cells(&origin)
            .iter()
            .filter(|&(_, l)| *l == level)
            .map(|(&c, _)| c)
            .collect()
    }

    #[test]
    fn origin_always_receives_inner_level() {
        let e = emitter(1, 1);
        let lit = e.light_cells(&Cell::new(0, 0));
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
        let lit = e.light_cells(&origin);

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
        let lit = e.light_cells(&Cell::new(0, 0));
        let mut seen = std::collections::HashSet::new();
        for (cell, _) in lit.0 {
            assert!(seen.insert(cell), "duplicate cell {:?}", cell);
        }
    }

    #[test]
    fn all_cells_within_total_radius() {
        let e = emitter(1, 2); // total radius = 3
        let origin = Cell::new(0, 0);
        let lit = e.light_cells(&origin);
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
        let at_zero = e.light_cells(&Cell::new(0, 0));
        let at_ten = e.light_cells(&Cell::new(10, 10));

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
