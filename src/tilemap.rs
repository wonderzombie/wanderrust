use bevy::{platform::collections::HashSet, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    atlas::SpriteAtlas,
    cell::Cell,
    light::LightLevel,
    tiles::{MapTile, Revealed, TileIdx},
};

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

#[derive(Component, Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stratum {
    Above,
    #[default]
    Below,
}

#[derive(Resource, Default, Debug)]
/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
pub struct TilemapSpec {
    /// MapTile entities will be created as children of this entity.
    pub id: TilemapId,
    pub size: MapDimensions,
    pub layer: TilemapLayer,
    /// A vector of tile indices and their corresponding cell positions. This will drive tilemap creation.
    pub tiles: Vec<(TileIdx, Cell, Stratum)>,
    pub start: Cell,
    pub light_level: LightLevel,
}

#[derive(Component, Serialize, Deref, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct TilemapLayer(pub f32);

#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapDimensions {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
}

impl MapDimensions {
    #[inline]
    pub fn cell_to_pos(&self, cell: &Cell) -> Vec2 {
        Vec2::new(
            cell.x as f32 * self.tile_size as f32,
            cell.y as f32 * self.tile_size as f32,
        )
    }
}

#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// TileStorage is used to manipulate the tiles in a tilemap, typically living on the same entity as [TilemapId].
/// Tiles are stored as a flat vector of `Option<Entity>`, indexed by `cell.to_idx(map_size.width)`. In this way,
/// a cell may be empty of any tile entity.
pub struct TileStorage {
    tiles: Vec<Option<Entity>>,
    pub size: MapDimensions,
}

impl TileStorage {
    // pub fn empty(size: MapDimensions) -> TileStorage {
    //     TileStorage {
    //         tiles: vec![None; (size.width * size.height) as usize],
    //         size,
    //     }
    // }

    pub fn get(&self, cell: &Cell) -> Option<Entity> {
        let idx = cell.to_idx(self.size.width);
        self.tiles.get(idx).copied().flatten()
    }

    pub fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = cell.to_idx(self.size.width);
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = Some(entity);
        }
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    fn new(size: MapDimensions) -> Self {
        Self {
            tiles: vec![None; (size.width * size.height) as usize],
            size,
        }
    }

    // /// Removes the cell-entity from storage and returns it, if any.
    // pub fn remove(&mut self, cell: &Cell) -> Option<Entity> {
    //     let idx = cell.to_idx(self.size.width);
    //     self.tiles[idx as usize].take()
    // }

    // pub fn iter(&self) -> impl Iterator<Item = &Option<Entity>> {
    //     self.tiles.iter()
    // }

    // pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Option<Entity>> {
    //     self.tiles.iter_mut()
    // }
}

/// EntryId uniquely identifies a [`Portal`].
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct EntryId(pub String);

impl From<&str> for EntryId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

/// A Portal is a bidirectional link between two [`Cell`]s in the map.
#[derive(Component, Serialize, Deserialize, Debug, Hash, Clone, Eq, PartialEq)]
pub struct Portal {
    pub id: EntryId,
    pub arrive_at: EntryId,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SavedTilemap {
    pub tiles: Vec<(TileIdx, Stratum)>,
    pub size: MapDimensions,
    pub layer: TilemapLayer,
    pub portals: Vec<(Portal, Cell)>,
}

#[derive(Bundle, Clone)]
pub struct TileBundle {
    pub map_tile: MapTile,
    pub tile_idx: TileIdx,
    pub cell: Cell,
    pub transform: Transform,
    pub sprite: Sprite,
    pub revealed: Revealed,
    pub child_of: ChildOf,
    pub stratum: Stratum,
}

#[derive(Bundle, Default)]
pub struct TilemapBundle {
    pub size: MapDimensions,
    pub layer: TilemapLayer,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}

/// Spawns a tilemap, a constituency of [`MapTile`] entities, from a [`TilemapSpec`].
/// It creates one entity with [`TilemapBundle`] and many with [`TileBundle`].
pub fn spawn_tilemap(
    mut commands: Commands,
    mut spec: ResMut<TilemapSpec>,
    sheet: Res<SpriteAtlas>,
) {
    let tilemap_bundle = TilemapBundle {
        size: spec.size,
        layer: spec.layer,
        ..Default::default()
    };

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        spec.size, spec.layer
    );

    let map_entity = commands.spawn(tilemap_bundle).id();
    spec.id.set(map_entity);
    spawn_maptiles_from_spec(&spec, &sheet, &mut commands);
    commands.entity(map_entity).insert(spec.id);
}

/// Spawns [`MapTile`] entities from a [`TilemapSpec`] in a batch.
fn spawn_maptiles_from_spec(spec: &TilemapSpec, sheet: &SpriteAtlas, commands: &mut Commands) {
    let parent = spec.id.0.unwrap();
    let bundles: Vec<TileBundle> = spec
        .tiles
        .iter()
        .map(|(tile_idx, cell, stratum)| {
            let pos = spec.size.cell_to_pos(cell);

            // TODO: replace [MapTile] with [MapId] here and elsewhere.
            TileBundle {
                map_tile: MapTile,
                tile_idx: *tile_idx,
                cell: *cell,
                // This puts the tile at the correct z-order based on the layer.
                transform: Transform::from_xyz(pos.x, pos.y, *spec.layer),
                sprite: sheet.sprite_from_idx(*tile_idx),
                revealed: Revealed(false),
                child_of: ChildOf(parent),
                stratum: *stratum,
            }
        })
        .collect();

    commands.spawn_batch(bundles);
}

/// Adds all [`MapTile`] entities to [`TileStorage`] for quick lookup by [`Cell`].
pub fn initialize_tile_storage(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    tiles: Query<(&Cell, Entity), With<MapTile>>,
) {
    let map_entity = spec
        .id
        .get()
        .expect("TilemapSpec is missing a map entity ID");

    info!("storing maps by cell");

    let mut storage = TileStorage::new(spec.size);
    for (cell, entity) in tiles.iter() {
        storage.set(cell, entity);
    }
    info!("stored tiles: {}", storage.len());
    commands.entity(map_entity).insert(storage);
}

/// Saves the current state [`TileStorage`] as a [`SavedTilemap`].
pub fn save_map(
    storage: &TileStorage,
    all_tiles: &Query<&TileIdx, With<MapTile>>,
    all_portals: &Query<(&Portal, &Cell)>,
    strata: &Query<&Stratum, With<MapTile>>,
) -> SavedTilemap {
    // Use storage to drive iteration and using all_tiles to resolve [`TileIdx`] for each entity.
    // We don't need to store coordinates since the map size is fixed and known at load time
    // AND because we provide a default, never skipping empty cells.
    let tiles = storage
        .tiles
        .iter()
        // If there's an entity in storage, use that entity as a lookup into the [`TileIdx`] query.
        .map(|&entity_opt| {
            let Some(entity) = entity_opt else {
                return (TileIdx::default(), Stratum::default());
            };
            let tile_idx = all_tiles.get(entity).copied().unwrap_or_default();
            let stratum = strata.get(entity).copied().unwrap_or_default();

            (tile_idx, stratum)
        })
        .collect::<Vec<_>>();

    let portals = all_portals
        .iter()
        .map(|(portal, cell)| (portal.clone(), *cell))
        .collect::<Vec<_>>();

    SavedTilemap {
        tiles,
        portals,
        size: storage.size,
        ..Default::default()
    }
}

/// Loads a [`SavedTilemap`] into [`TileStorage`].
pub fn load_map(commands: &mut Commands, saved: &SavedTilemap, storage: &mut TileStorage) {
    storage
        .tiles
        .iter()
        .zip(saved.tiles.iter())
        .for_each(|(&maybe_entity, &idx_strat)| {
            if let Some(entity) = maybe_entity {
                commands.entity(entity).insert(idx_strat);
            }
        });

    let valid_ids = saved
        .portals
        .iter()
        .map(|(portal, _)| portal.id.clone())
        .collect::<HashSet<_>>();

    for (portal, cell) in saved.portals.iter() {
        // TODO: ensure that some validation occurs here and/or address the case where
        // there aren't already enough tiles.
        if let Some(entity) = storage.get(cell) {
            if valid_ids.contains(&portal.id) {
                commands.entity(entity).insert(portal.clone());
            } else {
                error!("portal id {:?} not found in valid_ids", portal.id);
            }
        }
    }
}
