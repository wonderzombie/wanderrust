use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use std::{fmt::Display, ops::Neg};

use crate::{
    actors::{Actor, PieceBundle, Player},
    atlas::SpriteAtlas,
    cell::Cell,
    interactions::Interactable,
    ldtk_loader::{LdtkActor, LdtkEntity, LdtkEntityExt},
    light::{Emitter, LightLevel},
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct WorldId(pub Entity);

#[derive(Resource, Default, Debug, PartialEq)]
pub struct WorldSpec {
    pub id: Option<WorldId>,
    pub maps: HashMap<StratumId, StratumSpec>,
    pub spawn_point: SpawnCell,
}

type TileSpec = (TileIdx, Cell);
type PortalSpec = (Portal, Cell);
type InterxSpec = (Interactable, Cell);
type EmitterSpec = (Emitter, Cell);

#[derive(Debug, Default, Resource, PartialEq, Reflect, Clone)]
pub struct StratumSpec {
    pub id: Option<Stratum>,
    pub size: Dimensions,

    pub tiles: Vec<TileSpec>,
    pub emitters: Vec<EmitterSpec>,
    pub interxs: Vec<InterxSpec>,
    pub portals: Vec<PortalSpec>,

    pub spawn_point: Option<SpawnCell>,
    pub light_level: LightLevel,
}

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

/// TileCell is a pair of (TileIdx, Cell). Together with a StratumId, it should be enough to uniquely identify a tile.
pub type TileCell = (TileIdx, Cell);
/// PortalCell is a triple of (Portal, TileIdx, Cell). Together with a StratumId, it should be enough to uniquely identify a tile.
pub type PortalCell = (Portal, TileIdx, Cell);

pub type InterxCell = (Interactable, TileIdx, Cell);
pub type EmitterCell = (Emitter, TileIdx, Cell);

pub type SpawnCell = (StratumId, Cell);

pub type StratTiles = HashMap<StratumId, Vec<TileCell>>;
pub type StratPortals = HashMap<StratumId, Vec<PortalCell>>;
pub type StratInterxs = HashMap<StratumId, Vec<InterxCell>>;
pub type StratEmitters = HashMap<StratumId, Vec<EmitterCell>>;

/// A resource representing the specification of the map, including its size, default tile type, and any special pieces defined by the ASCII map.
#[derive(Resource, Default, Debug, Clone, Reflect, Serialize, Deserialize, PartialEq)]
pub struct StratumTileSpec {
    /// Stratum entities will be created as children of this entity.
    #[serde(skip)]
    pub id: TilemapId,
    pub size: Dimensions,
    /// Tiles and portals keyed by StratumId drive tilemap creation.
    pub all_tiles: StratTiles,
    pub all_portals: StratPortals,
    pub all_interxs: StratInterxs,
    pub all_emitters: StratEmitters,
    /// Starting point for the player.
    pub spawn_point: SpawnCell,
    /// The minimum light level for the area.
    pub light_level: LightLevel,
}

#[derive(Resource, Debug, Clone, Reflect, PartialEq)]
pub struct ActiveStratum(pub Stratum);

#[derive(
    Component, Serialize, Deref, Deserialize, Default, Debug, Clone, Copy, PartialEq, Reflect,
)]
pub struct TilemapLayer(pub f32);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
            tile_size: tiles::TILE_SIZE_PX as u32,
        }
    }
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
    pub fn idx_to_cell(&self, idx: usize) -> Cell {
        Cell::from_idx(self.width, idx)
    }

    #[inline]
    fn ntiles(&self) -> usize {
        (self.width * self.height) as usize
    }

    #[inline]
    fn cell_to_idx(&self, cell: &Cell) -> usize {
        (cell.y * self.width as i32 + cell.x) as usize
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
#[derive(
    Component, Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
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
#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash, Eq, Reflect)]
pub struct EntryId(pub String);

impl PartialEq for EntryId {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0.as_str())
    }
}

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
    pub tile_idx: TileIdx,
}

impl LdtkEntityExt<Portal> for Portal {
    fn from_ldtk(entity: &LdtkEntity) -> Option<Portal> {
        if entity.ty().is_none_or(|it| it != LdtkActor::Portal) {
            return None;
        }

        // TODO: use EntityRef field.
        let id = EntryId(entity.get_string("id")?);
        let arrive_at = EntryId(entity.get_string("arrive_at")?);
        let tile_idx = entity.get_tile_field("tile").unwrap_or(TileIdx::Blank);

        Some(Portal {
            id,
            arrive_at,
            tile_idx,
        })
    }
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
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}

pub(crate) const MAP_LAYER: TilemapLayer = TilemapLayer(-6.0);
pub(crate) const ACTOR_LAYER: TilemapLayer = TilemapLayer(-2.0);
pub(crate) const PLAYER_LAYER: TilemapLayer = TilemapLayer(-1.0);

pub fn spawn_worldmap(
    mut commands: Commands,
    mut world_spec: ResMut<WorldSpec>,
    atlas: Res<SpriteAtlas>,
) {
    info!("🕹️  initializing worldmap");

    let world_entity = commands
        .spawn((Visibility::Visible, Transform::default()))
        .id();
    let world_id = WorldId(world_entity);
    world_spec.id.replace(world_id);

    let (start_strat_id, cell) = world_spec.spawn_point;

    for (stratum_id, strat_spec) in world_spec.maps.iter_mut() {
        let layer = stratum_id.0.neg() as f32 + *MAP_LAYER;

        let strat_entity = commands.spawn(TilemapBundle::default()).id();
        let stratum = Stratum(strat_entity, *stratum_id);
        strat_spec.id.replace(stratum);

        let mut cells = vec![TileIdx::Blank; strat_spec.size.ntiles()];

        let mut count = 0;
        strat_spec.tiles.iter().for_each(|(tile, cell)| {
            let idx = strat_spec.size.cell_to_idx(cell);
            cells[idx] = *tile;
            count += 1;
        });

        let bundles = generate_tile_bundles(strat_entity, &strat_spec.size, &cells, layer, &atlas);

        info!(
            "📍 {:?}: {} tiles; {} bundles; {} mapped tiles",
            stratum_id,
            count,
            cells.len(),
            bundles.len(),
        );
        commands.spawn_batch(bundles);

        if start_strat_id == *stratum_id {
            info!(
                "🕹️ found spawn stratum: {:?} and cell {:?}",
                start_strat_id, cell
            );
            commands.insert_resource(ActiveStratum(stratum));
            commands.spawn(WorldSpawn::new(strat_entity, cell));
        }

        commands
            .entity(strat_entity)
            .insert(Name::new(format!("Stratum {:?}", stratum)))
            .insert(strat_spec.size)
            .insert(stratum);
    }

    commands
        .entity(world_entity)
        .insert(world_id)
        .insert(Name::new("World"));
}

#[derive(Component)]
pub struct WorldSpawn {
    pub strat_entity: Entity,
    pub cell: Cell,
}

impl WorldSpawn {
    pub fn new(strat_entity: Entity, cell: Cell) -> Self {
        Self { strat_entity, cell }
    }
}

/// Spawns a tilemap, a constituency of [`MapTile`] entities, from a [`TilemapSpec`].
/// It creates one entity with [`TilemapBundle`] and many with [`TileBundle`].
pub fn spawn_tilemap(
    mut commands: Commands,
    mut spec: ResMut<StratumTileSpec>,
    sheet: Res<SpriteAtlas>,
) {
    info!(
        "📍 initializing tilemap: {:?} ({}) {:?}",
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

        let mut cells = vec![TileIdx::Blank; spec.size.ntiles()];

        let mut count = 0;
        tile_cells.iter().for_each(|(tile, cell)| {
            let i = spec.size.cell_to_idx(cell);
            cells[i] = *tile;
            count += 1;
        });

        let bundles = generate_tile_bundles(
            strat_entity,
            &spec.size,
            cells.as_ref(),
            strat_id as f32 + *MAP_LAYER,
            &sheet,
        );
        info!(
            "📍 {} tiles; {} bundles; {} mapped tiles",
            count,
            cells.len(),
            bundles.len(),
        );
        commands.spawn_batch(bundles);
        commands
            .entity(strat_entity)
            .insert(spec.size.clone())
            .insert(Stratum(strat_entity, strat_id.into()))
            .insert(Name::new(format!("Stratum: {}", strat_entity)));
    }
    commands
        .entity(map_entity)
        .insert(spec.id)
        .insert(Visibility::Visible)
        .insert(Name::new("Tilemap"));

    info!("📍 done spawning tilemap");
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
    tiles: &[TileIdx],
    layer: f32,
    sheet: &SpriteAtlas,
) -> Vec<TileBundle> {
    tiles
        .iter()
        .enumerate()
        .map(|(i, tile_idx)| {
            let cell = dim.idx_to_cell(i);
            let pos = dim.cell_to_pos(&cell);

            TileBundle {
                map_tile: MapTile,
                tile_idx: *tile_idx,
                cell,
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
    strata: Query<(&Stratum, &Dimensions, &Children)>,
    tiles: Query<&Cell, With<MapTile>>,
) {
    info!("📍 storing maps by cell by stratum");
    if strata.count() < 1 {
        panic!("zero strata found when initializing storage");
    }

    for (Stratum(stratum_entity, stratum_id), size, children) in strata {
        let mut num_cells = 0;
        let mut storage = TileStorage::new(size.clone());
        for entity in children.iter() {
            if let Ok(cell) = tiles.get(entity) {
                storage.set(cell, entity);
                num_cells += 1;
            }
        }
        info!(
            "📍 stratum {}: set {}/{} tile entities",
            stratum_id,
            num_cells,
            storage.len(),
        );
        commands.entity(*stratum_entity).insert(storage);
    }
}

pub fn setup_portals(
    mut commands: Commands,
    world: Res<WorldSpec>,
    strata: Query<&Stratum>,
    atlas: Res<SpriteAtlas>,
) {
    for Stratum(strat_entity, id) in strata.iter() {
        if let Some(spec) = world.maps.get(id) {
            for (portal, cell) in spec.portals.iter() {
                commands.spawn((
                    Actor,
                    portal.clone(),
                    portal.tile_idx,
                    ChildOf(*strat_entity),
                    PieceBundle {
                        sprite: atlas.sprite_from_idx(portal.tile_idx),
                        cell: *cell,
                        transform: Transform::from_xyz(0., 0., *ACTOR_LAYER),
                        ..default()
                    },
                ));
                info!("📍 inserted portal {:?} at {:?}", portal, cell);
            }
        }
    }
}

pub fn get_live_tiles(
    size: &Dimensions,
    strat_storage: &Query<(&Stratum, &TileStorage)>,
    live_tiles: &Query<&TileIdx>,
) -> HashMap<StratumId, Vec<TileCell>> {
    get_live_storage_items(size, strat_storage, live_tiles)
}

pub fn get_live_portals(
    strat_storage: &Query<&Stratum>,
    live_portals: &Query<(&Portal, &TileIdx, &Cell, &ChildOf)>,
) -> StratPortals {
    get_item_cells(strat_storage, live_portals)
}

pub fn get_item_cells<T>(
    strata: &Query<&Stratum>,
    live_items: &Query<(&T, &TileIdx, &Cell, &ChildOf)>,
) -> HashMap<StratumId, Vec<(T, TileIdx, Cell)>>
where
    T: Component + Clone + Default + PartialEq,
{
    let mut out = HashMap::new();
    for (item, tile_idx, cell, child_of) in live_items.iter() {
        let Ok(stratum) = strata.get(child_of.parent()) else {
            continue;
        };
        out.entry(stratum.1)
            .or_insert(Vec::new())
            .push((item.clone(), *tile_idx, *cell));
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
    for (Stratum(_, strat_id), storage) in strat_storage.iter() {
        for (i, entity_opt) in storage.tiles.iter().enumerate() {
            let cell = size.idx_to_cell(i);
            if let Some(entity) = entity_opt
                && let Ok(item) = live_items.get(*entity)
                && *item != T::default()
            {
                out.entry(*strat_id)
                    .or_insert(Vec::new())
                    .push((item.clone(), cell))
            }
        }
    }
    out
}
