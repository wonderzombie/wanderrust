use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use std::{fmt::Display, ops::Neg};

use crate::{
    actors::Player,
    atlas::SpriteAtlas,
    cell::Cell,
    light::LightLevel,
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(
    Component,
    Copy,
    Clone,
    Default,
    Debug,
    Deref,
    DerefMut,
    Reflect,
    Serialize,
    Deserialize,
    PartialEq,
)]
pub struct TilemapId(Option<Entity>);

impl TilemapId {
    pub fn set(&mut self, id: Entity) {
        self.0.replace(id);
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct Stratum(pub Entity, pub StratumId);

pub type TileCell = (TileIdx, Cell);
pub type PortalCell = (Portal, Cell);

#[derive(
    Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ActiveStratum(Stratum);

impl ActiveStratum {
    pub fn entity(&self) -> Entity {
        self.0.0
    }

    pub fn id(&self) -> StratumId {
        self.0.1
    }
}

/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
#[derive(Resource, Default, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
pub struct TilemapSpec {
    /// Stratum entities will be created as children of this entity.
    #[serde(skip)]
    pub id: TilemapId,
    pub size: Dimensions,
    /// Tiles and portals keyed by StratumId drive tilemap creation.
    pub all_tiles: StratTiles,
    #[serde(skip)]
    pub all_portals: StratPortals,
    /// Starting point for the player.
    pub spawn_point: Cell,
    /// The minimum light level for the area.
    pub light_level: LightLevel,
}

#[derive(
    Component, Serialize, Deref, Deserialize, Default, Debug, Clone, Copy, PartialEq, Reflect,
)]
pub struct TilemapLayer(pub f32);

#[derive(
    Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
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

    // Silences a warning re: `len()` but not `is_empty()`.
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
#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash, Eq, PartialEq, Reflect)]
pub struct EntryId(pub String);

impl From<&str> for EntryId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

/// A Portal is a bidirectional link between two [`Cell`]s in the map.
#[derive(
    Component, Serialize, Deserialize, Default, Debug, Hash, Clone, Eq, PartialEq, Reflect,
)]
pub struct Portal {
    pub id: EntryId,
    pub arrive_at: EntryId,
}

pub type StratPortals = HashMap<StratumId, Vec<(Portal, Cell)>>;
pub type StratTiles = HashMap<StratumId, Vec<(TileIdx, Cell)>>;

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
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}

pub(crate) const MAP_LAYER: TilemapLayer = TilemapLayer(-6.0);
pub(crate) const ACTOR_LAYER: TilemapLayer = TilemapLayer(-2.0);
pub(crate) const PLAYER_LAYER: TilemapLayer = TilemapLayer(-1.0);

/// Spawns a tilemap, a constituency of [`MapTile`] entities, from a [`TilemapSpec`].
/// It creates one entity with [`TilemapBundle`] and many with [`TileBundle`].
pub fn spawn_tilemap(
    mut commands: Commands,
    mut spec: ResMut<TilemapSpec>,
    sheet: Res<SpriteAtlas>,
) {
    info!(
        "initializing tilemap: {:?} ({}) {:?}",
        spec.id, spec.size, spec.light_level
    );

    let tilemap_bundle = TilemapBundle::default();

    let map_entity = commands.spawn(tilemap_bundle).id();
    spec.id.set(map_entity);

    for (strat_id, tile_cells) in spec.all_tiles.iter() {
        // Use the ID as a negative index on the tilemap layer.
        // That puts the 0th item at MAP_LAYER, the 1st at MAP_LAYER - 1.
        let strat_id = strat_id.0.neg();
        // Avoid warnings that children of this entity are missing components.
        let strat_entity = commands
            .spawn((Visibility::Visible, Transform::default()))
            .id();
        let bundles = generate_tile_bundles(
            strat_entity,
            &spec.size,
            tile_cells,
            strat_id as f32 + *MAP_LAYER,
            &sheet,
        );
        commands.spawn_batch(bundles);
        commands
            .entity(strat_entity)
            .insert(Stratum(strat_entity, strat_id.into()))
            .insert(Name::new(format!("Stratum: {}", strat_entity)));
    }
    commands
        .entity(map_entity)
        .insert(spec.id)
        .insert(Visibility::Visible)
        .insert(Name::new("Tilemap"));

    info!("✅\tdone spawning tilemap\t✅")
}

pub fn despawn_tilemap(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    strata: Query<Entity, With<Stratum>>,
) {
    commands.entity(*player).remove::<ChildOf>();
    for stratum in strata.iter() {
        commands.entity(stratum).despawn();
    }
}

/// Generates [`MapTile`] entities from a [`TilemapSpec`] in a batch as children of a parent Entity.
fn generate_tile_bundles(
    parent: Entity,
    dim: &Dimensions,
    tiles: &[(TileIdx, Cell)],
    layer: f32,
    sheet: &SpriteAtlas,
) -> Vec<TileBundle> {
    tiles
        .iter()
        .map(|(tile_idx, cell)| {
            let pos = dim.cell_to_pos(cell);

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
        .collect()
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
            "✅\tstratum {}: set {}/{} tile entities\t✅",
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
    for (Stratum(strat_entity, id), storage) in strat_storage.iter() {
        if let Some(portal_cells) = spec.all_portals.get(id) {
            for (portal, cell) in portal_cells {
                if let Some(tile_entity) = storage.get(cell) {
                    commands
                        .entity(tile_entity)
                        .insert(portal.clone())
                        .insert(ChildOf(*strat_entity))
                        .insert(Name::new(format!("Portal: {:#?}", portal)));
                    info!("inserted portal {:?} at {:?}", portal, cell);
                }
            }
        }
    }
}

pub fn get_live_tiles(
    size: &Dimensions,
    strat_storage: &Query<(&Stratum, &TileStorage)>,
    live_tiles: &Query<&TileIdx>,
) -> HashMap<StratumId, Vec<(TileIdx, Cell)>> {
    get_live_storage_items(size, strat_storage, live_tiles)
}

pub fn get_live_portals(
    strat_storage: &Query<&Stratum>,
    live_portals: &Query<(&Portal, &Cell, &ChildOf)>,
) -> StratPortals {
    get_item_cells(strat_storage, live_portals)
}

pub fn get_item_cells<T>(
    strata: &Query<&Stratum>,
    live_items: &Query<(&T, &Cell, &ChildOf)>,
) -> HashMap<StratumId, Vec<(T, Cell)>>
where
    T: Component + Clone + Default + PartialEq,
{
    let mut out = HashMap::new();
    for (item, cell, child_of) in live_items.iter() {
        let Ok(stratum) = strata.get(child_of.parent()) else {
            continue;
        };
        out.entry(stratum.1.clone())
            .or_insert(Vec::new())
            .push((item.clone(), cell.clone()));
    }

    out
}

pub fn get_live_storage_items<T>(
    size: &Dimensions,
    strat_storage: &Query<(&Stratum, &TileStorage)>,
    live_items: &Query<&T>,
) -> HashMap<StratumId, Vec<(T, Cell)>>
where
    T: Component + Clone + Default + PartialEq,
{
    let mut out = HashMap::new();
    for (stratum, storage) in strat_storage.iter() {
        for (i, entity_opt) in storage.tiles.iter().enumerate() {
            let cell = size.idx_to_cell(i as u32);
            if let Some(entity) = entity_opt
                && let Ok(item) = live_items.get(*entity)
            {
                if *item != T::default() {
                    out.entry(stratum.1.clone())
                        .or_insert(Vec::new())
                        .push((item.clone(), cell))
                }
            }
        }
    }
    out
}
