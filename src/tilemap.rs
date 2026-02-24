use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    SpriteAtlas,
    cell::Cell,
    map::{self, MapSpec},
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Component, Default, Clone, Copy)]
pub struct TilemapId(usize);

#[derive(Component, Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct TilemapLayer(pub f32);

#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TilemapSize {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
}

impl TilemapSize {
    #[inline]
    pub fn cell_to_pos(&self, cell: &Cell) -> Vec2 {
        Vec2::new(
            cell.x as f32 * self.tile_size as f32,
            cell.y as f32 * self.tile_size as f32,
        )
    }
}

#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Attach this to an entity with TilemapId.
pub struct TilemapStorage {
    tiles: Vec<Option<Entity>>,
    pub size: TilemapSize,
}

impl TilemapStorage {
    pub fn empty(size: TilemapSize) -> TilemapStorage {
        TilemapStorage {
            tiles: vec![None; (size.width * size.height) as usize],
            size,
        }
    }

    pub fn get(&self, cell: &Cell) -> Option<Entity> {
        let idx = cell.to_idx(self.size.width) as usize;
        self.tiles.get(idx).copied().flatten()
    }

    pub fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = cell.to_idx(self.size.width) as usize;
        if let Some(slot) = self.tiles.get_mut(idx) {
            *slot = Some(entity);
        }
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SavedTilemap {
    pub tiles: Vec<TileIdx>,
    pub size: TilemapSize,
    pub layer: TilemapLayer,
}

#[derive(Bundle, Clone, Default)]
pub struct TileBundle {
    pub map_tile: MapTile,
    pub tilemap_id: TilemapId,
    pub tile_idx: TileIdx,
    pub cell: Cell,
    pub transform: Transform,
    pub sprite: Sprite,
    pub revealed: Revealed,
}

#[derive(Bundle, Default)]
pub struct TilemapBundle {
    pub size: TilemapSize,
    pub layer: TilemapLayer,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}

pub fn setup_tilemap(mut commands: Commands, spec: Res<MapSpec>, sheet: Res<SpriteAtlas>) {
    let size = TilemapSize {
        width: spec.size.width,
        height: spec.size.height,
        tile_size: map::DEFAULT_TILE_SIZE,
    };
    // TODO: settle z-order and use this as a constant
    let layer = TilemapLayer(spec.layer as f32 - 3.);
    let tilemap_bundle = TilemapBundle {
        size,
        layer,
        ..Default::default()
    };

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        size, layer
    );

    let map_entity = commands.spawn(tilemap_bundle).id();
    let tilemap_id = TilemapId(0);
    let mut storage = TilemapStorage::empty(size);

    fill_tilemap_fn(
        |_| TileIdx::Blank,
        tilemap_id,
        size,
        layer,
        &sheet,
        &mut commands,
        &mut storage,
    );

    commands.entity(map_entity).insert((tilemap_id, storage));
}

pub fn fill_tilemap_fn(
    fx: impl Fn(&Cell) -> TileIdx,
    tilemap_id: TilemapId,
    size: TilemapSize,
    layer: TilemapLayer,
    sheet: &SpriteAtlas,
    commands: &mut Commands,
    storage: &mut TilemapStorage,
) {
    let tiles = size.width * size.height;
    let mut tally: HashMap<TileIdx, usize> = HashMap::new();
    for i in 0..tiles {
        let cell = Cell::from_idx(size.width, i as usize);
        let pos = size.cell_to_pos(&cell);
        let tile_idx = fx(&cell);
        let entity = commands
            .spawn((TileBundle {
                map_tile: MapTile,
                tilemap_id,
                tile_idx,
                cell,
                transform: Transform::from_xyz(pos.x, pos.y, layer.0),
                sprite: sheet.sprite_from_idx(tile_idx),
                revealed: Revealed(false),
            },))
            .id();
        storage.set(&cell, entity);

        tally
            .entry(tile_idx)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    info!("tilemap: initialized tile distribution:");
    for (tile_idx, count) in tally.iter() {
        info!("\t{:?}: {}", tile_idx, count);
    }
}

pub fn load_ascii_map(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tilemaps: Query<(&TilemapId, &TilemapStorage)>,
) {
    let (_, storage) = tilemaps.single().unwrap();

    for (tile_idx, cells) in spec.pieces.iter() {
        for cell in cells.iter() {
            // We're going to reuse the tiles from the existing tilemap via Storage.
            if let Some(tile) = storage.get(cell) {
                commands.entity(tile).insert(*tile_idx);
            } else {
                warn!("Tilemap is missing a tile at {:?}", cell);
                continue;
            }
        }
    }
}

pub fn save_map(
    storage: &mut TilemapStorage,
    all_tiles: &Query<&TileIdx, With<MapTile>>,
) -> SavedTilemap {
    let tiles = storage
        .tiles
        .iter()
        .map(|entity_opt| entity_opt.and_then(|entity| all_tiles.get(entity).ok().copied()))
        .map(|tile_idx| tile_idx.unwrap_or_default())
        .collect::<Vec<_>>();

    SavedTilemap {
        tiles: tiles.clone(),
        size: storage.size,
        layer: TilemapLayer::default(),
    }
}

pub fn load_map(commands: &mut Commands, saved: &SavedTilemap, storage: &mut TilemapStorage) {
    storage
        .tiles
        .iter()
        .zip(saved.tiles.iter())
        .for_each(|(maybe_entity, maybe_tile_idx)| {
            if let (Some(entity), tile_idx) = (maybe_entity, maybe_tile_idx) {
                commands.entity(*entity).insert(*tile_idx);
            }
        });
}
