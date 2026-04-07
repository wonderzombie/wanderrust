use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use std::{fmt::Display, ops::Neg};

use crate::{
    atlas::SpriteAtlas,
    cell::Cell,
    light::LightLevel,
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Component, Copy, Clone, Default, Debug, Deref, DerefMut)]
pub struct TilemapId(Option<Entity>);

impl TilemapId {
    pub fn set(&mut self, id: Entity) {
        self.0.replace(id);
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StratumId(pub i32);

impl From<i32> for StratumId {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl Display for StratumId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stratum: {}", self.0)
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stratum(pub Entity, pub StratumId);

pub type TileCell = (TileIdx, Cell);
pub type PortalCell = (Portal, Cell);

/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
#[derive(Resource, Default, Debug)]
pub struct TilemapSpec {
    /// Stratum entities will be created as children of this entity.
    pub id: TilemapId,
    pub size: Dimensions,
    pub layer: TilemapLayer,
    /// Tiles and portals keyed by StratumId drive tilemap creation.
    pub all_tiles: HashMap<StratumId, Vec<TileCell>>,
    pub all_portals: HashMap<StratumId, Vec<PortalCell>>,
    /// Starting point for the player.
    pub start: Cell,
    /// The minimum light level for the area.
    pub light_level: LightLevel,
}

#[derive(Component, Serialize, Deref, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct TilemapLayer(pub f32);

#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
}

impl Dimensions {
    #[inline]
    pub fn cell_to_pos(&self, cell: &Cell) -> Vec2 {
        Vec2::new(
            cell.x as f32 * self.tile_size as f32,
            cell.y as f32 * self.tile_size as f32,
        )
    }

    #[inline]
    pub fn idx_to_cell(&self, idx: u32) -> Cell {
        // TODO: use Cell's impl
        Cell {
            x: (idx % self.width) as i32,
            y: (idx / self.width) as i32,
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn cell_to_idx(&self, cell: &Cell) -> u32 {
        cell.x as u32 + cell.y as u32 * self.width
    }

    #[inline]
    #[allow(dead_code)]
    pub const fn ntiles(&self) -> u32 {
        self.width * self.height
    }
}

impl PartialOrd for Dimensions {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dimensions {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.width
            .cmp(&other.width)
            .then(self.height.cmp(&other.height))
            .then(self.tile_size.cmp(&other.tile_size))
    }
}

impl Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// TileStorage is used to manipulate the tiles in a tilemap, typically living on the same entity as [TilemapId].
/// Tiles are stored as a flat vector of `Option<Entity>`, indexed by `cell.to_idx(map_size.width)`. In this way,
/// a cell may be empty of any tile entity.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileStorage {
    tiles: Vec<Option<Entity>>,
    pub size: Dimensions,
}

impl TileStorage {
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

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    fn new(size: Dimensions) -> Self {
        Self {
            tiles: vec![None; (size.width * size.height) as usize],
            size,
        }
    }
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
    pub tiles: Vec<TileIdx>,
    pub size: Dimensions,
    pub layer: TilemapLayer,
    pub portals: Vec<(Portal, Cell)>,
    pub light_level: LightLevel,
    pub flip_v: bool,
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
    pub vis: Visibility,
}

#[derive(Bundle, Default)]
pub struct TilemapBundle {
    pub size: Dimensions,
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
        ..default()
    };

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        spec.size, spec.layer
    );

    let map_entity = commands.spawn(tilemap_bundle).id();
    spec.id.set(map_entity);

    for (id, tile_cells) in spec.all_tiles.iter() {
        let i = id.0.neg();
        let strat_id = commands
            .spawn((Visibility::Visible, Transform::default()))
            .id();
        spawn_maptiles_from_spec(
            strat_id,
            &spec.size,
            tile_cells,
            i as f32,
            &sheet,
            &mut commands,
        );
        commands
            .entity(strat_id)
            .insert(Stratum(strat_id, i.into()));
    }
    commands
        .entity(map_entity)
        .insert(spec.id)
        .insert(Visibility::Visible);

    info!("ℹ️\tdone spawning tilemap")
}

/// Spawns [`MapTile`] entities from a [`TilemapSpec`] in a batch.
fn spawn_maptiles_from_spec(
    parent: Entity,
    size: &Dimensions,
    tiles: &Vec<(TileIdx, Cell)>,
    layer: f32,
    sheet: &SpriteAtlas,
    commands: &mut Commands,
) {
    let bundles: Vec<TileBundle> = tiles
        .iter()
        .map(|(tile_idx, cell)| {
            let pos = size.cell_to_pos(cell);

            TileBundle {
                map_tile: MapTile,
                tile_idx: *tile_idx,
                cell: *cell,
                // This puts the tile at the correct z-order based on the layer.
                transform: Transform::from_xyz(pos.x, pos.y, layer),
                sprite: sheet.sprite_from_idx(*tile_idx),
                revealed: Revealed(false),
                child_of: ChildOf(parent),
                vis: Visibility::Inherited,
            }
        })
        .collect();

    commands.spawn_batch(bundles);
}

/// Adds all [`MapTile`] entities to [`TileStorage`] for quick lookup by [`Cell`].
pub fn initialize_tile_storage(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    strata: Query<(&Stratum, &Children)>,
    tiles: Query<&Cell, With<MapTile>>,
) {
    info!("storing maps by cell by stratum");
    info!("there are {} strata", strata.iter().len());

    let mut num_cells = 0;
    for (stratum, children) in strata {
        let mut storage = TileStorage::new(spec.size);
        for entity in children.iter() {
            if let Ok(cell) = tiles.get(entity) {
                storage.set(cell, entity);
                num_cells += 1;
            }
        }
        info!(
            "✅\tstratum {}: set {} cells of {} tile entities",
            stratum.1,
            num_cells,
            storage.len(),
        );
        commands.entity(stratum.0).insert(storage);
    }
}

pub fn setup_portals(
    mut commands: Commands,
    spec: Res<TilemapSpec>,
    strat_storage: Query<(&Stratum, &TileStorage)>,
) {
    for (Stratum(_, id), storage) in strat_storage.iter() {
        if let Some(portals_cells) = spec.all_portals.get(id) {
            for (portal, cell) in portals_cells {
                if let Some(entity) = storage.get(cell) {
                    commands.entity(entity).insert(portal.clone());
                    info!("inserted portal {:?} at {:?}", portal, cell);
                }
            }
        }
    }
}

/// Saves the current state [`TileStorage`] as a [`SavedTilemap`].
pub fn save_map(
    spec: &Res<TilemapSpec>,
    storage: &TileStorage,
    all_tiles: &Query<&TileIdx, With<MapTile>>,
    all_portals: &Query<(&Portal, &Cell)>,
) -> SavedTilemap {
    // Use storage to drive iteration and using all_tiles to resolve [`TileIdx`] for each entity.
    // We don't need to store coordinates since the map size is fixed and known at load time
    // AND because we provide a default, never skipping empty cells.
    let tiles = storage
        .tiles
        .iter()
        // If there's an entity in storage, use that entity as a lookup into the [`TileIdx`] query.
        .map(|&entity_opt| {
            entity_opt
                .and_then(|e| all_tiles.get(e).ok().copied())
                .unwrap_or_default()
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
        light_level: spec.light_level,
        ..default()
    }
}

/// Loads a [`SavedTilemap`] into [`TileStorage`].
pub fn load_map(commands: &mut Commands, saved: &SavedTilemap, storage: &mut TileStorage) {
    if saved.size > storage.size {
        error!(
            "saved map size {:?} exceeds storage size {:?}",
            saved.size, storage.size
        );
        return;
    } else if saved.size != storage.size {
        warn!(
            "saved map size {:?} does not match storage size {:?}",
            saved.size, storage.size
        );
    }

    let mut tally = HashMap::<TileIdx, usize>::new();
    let mut missing: usize = 0;

    // We can derive cell from the source using its Dimensions and then
    // pull the entity from storage thus to insert its new tile components.
    for (source_idx, source_tile) in saved.tiles.iter().enumerate() {
        let orig_cell = saved.size.idx_to_cell(source_idx as u32);
        let mut flipped_cell = orig_cell;

        if saved.flip_v {
            flipped_cell.y = storage.size.height as i32 - 1 - flipped_cell.y;
            println!(
                "orig: {} => {}",
                orig_cell,
                storage.size.cell_to_pos(&orig_cell)
            );
            println!(
                "{} => {}",
                flipped_cell,
                storage.size.cell_to_pos(&flipped_cell)
            );
        }

        if let Some(entity) = storage.get(&flipped_cell) {
            commands.entity(entity).insert(*source_tile);
            tally
                .entry(*source_tile)
                .and_modify(|v| *v += 1)
                .or_insert(1);
        } else {
            missing += 1;
        }
    }

    info!("ℹ️ tile breakdown ({} types) {:#?}", tally.len(), tally);

    if missing > 0 {
        warn!("{} tiles in storage are not entities", missing);
    }

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
