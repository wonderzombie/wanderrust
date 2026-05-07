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
    tiles::{self, MapTile, Revealed, TileIdx},
};

#[derive(Component, Copy, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct WorldId(pub Entity);

#[derive(Resource, Default, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct WorldSpec {
    pub id: Option<WorldId>,
    pub maps: HashMap<LevelId, LevelSpec>,
    pub spawn_point: SpawnCell,
    pub light_level: LightLevel,
}

type TileSpec = (TileIdx, Cell);
type PortalSpec = (Portal, Cell);
type InterxSpec = (Interactable, Cell);
type EmitterSpec = (Emitter, Cell);

#[derive(Debug, Default, Resource, PartialEq, Reflect, Clone)]
#[reflect(Resource)]
pub struct LevelSpec {
    pub id: Option<Level>,
    pub identifier: String,
    pub dimensions: Dimensions,
    pub world_pos: Vec2,
    pub depth: i32,

    pub tiles: Vec<TileSpec>,
    pub emitters: Vec<EmitterSpec>,
    pub interxs: Vec<InterxSpec>,
    pub portals: Vec<PortalSpec>,

    pub spawn_point: Option<SpawnCell>,
    pub light_level: LightLevel,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LevelId(pub i32);

impl From<i32> for LevelId {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl Display for LevelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Level: {}", self.0)
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub struct Level(pub Entity, pub LevelId);

#[derive(Component, Debug, Reflect)]
#[relationship_target(relationship = DenizenOf)]
#[reflect(Component)]
pub struct Zone(Vec<Entity>);

#[derive(Component, Debug, Reflect)]
#[relationship(relationship_target = Zone)]
#[require(Actor)]
#[reflect(Component)]
pub struct DenizenOf(pub Entity);

/// Syncs the presentation with gameplay concept of a Denizens of a Zone.
pub fn snapshot_denizens(
    mut commands: Commands,
    query: Populated<(Entity, &ChildOf), (With<Actor>, Without<DenizenOf>)>,
) {
    for (actor, child_of) in query {
        commands.entity(actor).insert(DenizenOf(child_of.parent()));
    }
}

/// TileCell is a pair of (TileIdx, Cell). Together with a LevelId, it should be
/// enough to uniquely identify a tile.
pub type TileCell = (TileIdx, Cell);
/// PortalCell is a triple of (Portal, TileIdx, Cell). Together with a LevelId,
/// it should be enough to uniquely identify a tile.
pub type PortalCell = (Portal, TileIdx, Cell);

pub type InterxCell = (Interactable, TileIdx, Cell);
pub type EmitterCell = (Emitter, TileIdx, Cell);

pub type SpawnCell = (LevelId, Cell);

#[derive(Component, Debug, Clone, Reflect, PartialEq)]
pub struct ActiveLevel;

#[derive(
    Component, Serialize, Deref, Deserialize, Default, Debug, Clone, Copy, PartialEq, Reflect,
)]
#[reflect(Component)]
pub struct TilemapLayer(pub f32);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
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

/// TileStorage is used to manipulate the tiles in a tilemap, typically living
/// on the same entity as [LevelId]. Tiles are stored as a flat vector of
/// `Option<Entity>`, indexed by `cell.to_idx(map_size.width)`. In this way, a
/// cell may be empty of any tile entity.
#[derive(
    Component, Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect,
)]
#[reflect(Component)]
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

    pub fn into_iter(&self) -> impl Iterator<Item = Cell> {
        (0..self.len())
            .into_iter()
            .map(|i| self.size.idx_to_cell(i))
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
#[reflect(Component)]
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
pub struct LevelBundle {
    pub transform: Transform,
    pub visibility: Visibility,
}

pub(crate) const MAP_LAYER: TilemapLayer = TilemapLayer(-6.0);
pub(crate) const ACTOR_LAYER: TilemapLayer = TilemapLayer(-2.0);
pub(crate) const PLAYER_LAYER: TilemapLayer = TilemapLayer(-1.0);

pub fn spawn_worldmap(
    mut commands: Commands,
    mut world_spec: ResMut<WorldSpec>,
    atlas: Res<SpriteAtlas>,
) {
    info!("📍 initializing worldmap");

    let world_entity = commands
        .spawn((Visibility::Inherited, Transform::default()))
        .id();
    let world_id = WorldId(world_entity);
    world_spec.id.replace(world_id);

    let (start_level_id, cell) = world_spec.spawn_point;

    let mut grand_tally: HashMap<TileIdx, usize> = HashMap::new();

    for (level_id, level_spec) in world_spec.maps.iter_mut() {
        let layer = level_spec.depth.neg() as f32 + *MAP_LAYER;
        let level_entity = commands
            .spawn(LevelBundle {
                transform: Transform::from_xyz(
                    level_spec.world_pos.x,
                    level_spec.world_pos.y,
                    layer,
                ),
                ..default()
            })
            .id();
        let level = Level(level_entity, *level_id);
        level_spec.id.replace(level);

        let mut tally: HashMap<TileIdx, usize> = HashMap::new();
        let mut count = 0;
        let mut cells = vec![TileIdx::Blank; level_spec.dimensions.ntiles()];
        level_spec.tiles.iter().for_each(|(tile_idx, cell)| {
            let idx = level_spec.dimensions.cell_to_idx(cell);
            cells[idx] = *tile_idx;
            tally.entry(*tile_idx).and_modify(|e| *e += 1).or_insert(1);
            count += 1;
        });

        for (k, v) in tally.iter() {
            grand_tally
                .entry(*k)
                .and_modify(|vv| *vv += v)
                .or_insert(*v);
        }

        let bundles =
            generate_tile_bundles(level_entity, &level_spec.dimensions, &cells, layer, &atlas);

        info!(
            "📍 {:?}: {} tiles; {} bundles; {} mapped tiles",
            level,
            level_spec.tiles.len(),
            bundles.len(),
            count,
        );
        commands.spawn_batch(bundles);

        if start_level_id == *level_id {
            info!(
                "- 📍 found spawn level: {:?} and cell {:?}",
                start_level_id, cell
            );
            commands.spawn(WorldSpawn::new(level_entity, cell));
            commands
                .entity(level_entity)
                .insert((ActiveLevel, Visibility::Inherited));
        }

        info!("- 📍 {start_level_id} tally: {:?}", tally);

        commands
            .entity(level_entity)
            .insert(Name::new(format!("Level {:?}", level)))
            .insert(level_spec.dimensions)
            .insert(ChildOf(world_entity))
            .insert(level);
    }

    info!(
        "📍 world tiles: {}",
        grand_tally.values().copied().sum::<usize>()
    );

    commands
        .entity(world_entity)
        .insert(world_id)
        .insert(Name::new("World"));
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct WorldSpawn {
    pub level_entity: Entity,
    pub cell: Cell,
}

impl WorldSpawn {
    pub fn new(level_entity: Entity, cell: Cell) -> Self {
        Self { level_entity, cell }
    }
}

pub fn despawn_tilemap(
    mut commands: Commands,
    player: Single<Entity, With<Player>>,
    levels: Query<Entity, With<Level>>,
) {
    commands.entity(*player).remove::<ChildOf>();
    for level in levels.iter() {
        commands.entity(level).despawn();
    }
}

/// Generates [`MapTile`] entities from a [`TilemapSpec`] in a batch as children
/// of a parent Entity.
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
            if cell > Cell::ZERO {
                assert_ne!(
                    (cell.x as f32, cell.y as f32),
                    (pos.x, pos.y),
                    "expected non-zero cell to map to non-zero position"
                );
            }

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
    levels: Query<(&Level, &Dimensions, &Children)>,
    tiles: Query<&Cell, With<MapTile>>,
) {
    info!("📍 storing maps by cell by level");
    if levels.count() < 1 {
        panic!("zero levels found when initializing storage");
    }

    let mut zero_cells = 0;

    for (Level(level_entity, level_id), size, children) in levels {
        let mut num_cells = 0;
        let mut storage = TileStorage::new(*size);
        for entity in children.iter() {
            if let Ok(cell) = tiles.get(entity) {
                if cell == &Cell::ZERO {
                    zero_cells += 1;
                }
                storage.set(cell, entity);
                num_cells += 1;
            }
        }
        info!(
            "- 📍 level {}: set {}/{} tile entities ({} zero cells)",
            level_id,
            num_cells,
            storage.len(),
            zero_cells,
        );
        commands.entity(*level_entity).insert(storage);
    }
}

pub fn setup_portals(
    mut commands: Commands,
    world: Res<WorldSpec>,
    levels: Query<&Level>,
    atlas: Res<SpriteAtlas>,
) {
    for Level(level_entity, id) in levels.iter() {
        if let Some(spec) = world.maps.get(id) {
            for (portal, cell) in spec.portals.iter() {
                commands.spawn((
                    Actor,
                    portal.clone(),
                    portal.tile_idx,
                    ChildOf(*level_entity),
                    PieceBundle {
                        sprite: atlas.sprite_from_idx(portal.tile_idx),
                        cell: *cell,
                        transform: Transform::from_xyz(0., 0., *ACTOR_LAYER),
                        ..default()
                    },
                ));
                info!("- 📍 inserted portal {:?} at {:?}", portal, cell);
            }
        }
    }
}
