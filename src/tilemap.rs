use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    SpriteAtlas,
    cell::Cell,
    map::MapSpec,
    tiles::{self, MapTile, Revealed, TileIdx},
};

#[derive(Component, Deref, Clone, Copy)]
pub struct TilemapId(pub Entity);

#[derive(Component, Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct TilemapLayer(pub f32);

#[derive(Debug, Default, Component, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Default, Component, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Attach this to an entity with TilemapId.
pub struct TilemapStorage {
    tiles: Vec<Option<Entity>>,
    pub size: TilemapSize,
}

impl TilemapStorage {
    pub fn empty(size: TilemapSize) -> TilemapStorage {
        TilemapStorage {
            tiles: vec![None; (size.width * size.height) as usize],
            size: size,
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
    pub tiles: Vec<tiles::TileIdx>,
    pub size: TilemapSize,
    pub layer: TilemapLayer,
}

#[derive(Bundle)]
pub struct TileBundle {
    pub tilemap_id: TilemapId,
    pub tile_idx: TileIdx,
    pub cell: Cell,
    pub transform: Transform,
    pub sprite: Sprite,
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
        width: spec.size.x,
        height: spec.size.y,
        tile_size: spec.tile_size,
    };
    let layer = TilemapLayer(spec.layer as f32 - 3.);
    let tilemap_bundle = TilemapBundle {
        size: size.clone(),
        layer: layer,
        ..Default::default()
    };

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        size, layer
    );

    let map_entity = commands.spawn(tilemap_bundle).id();
    let tilemap_id = TilemapId(map_entity);
    let mut storage = TilemapStorage::empty(size);

    fill_tilemap(
        TileIdx::Dirt,
        tilemap_id,
        size,
        layer,
        &sheet,
        &mut commands,
        &mut storage,
    );

    commands.entity(map_entity).insert((tilemap_id, storage));
}

pub fn fill_tilemap(
    tile_idx: TileIdx,
    tilemap_id: TilemapId,
    size: TilemapSize,
    layer: TilemapLayer,
    sheet: &SpriteAtlas,
    commands: &mut Commands,
    storage: &mut TilemapStorage,
) {
    for x in 0..size.width {
        for y in 0..size.height {
            let cell = Cell::new(x as i32, y as i32);
            let pos = size.cell_to_pos(&cell);
            let entity = commands
                .spawn((
                    MapTile,
                    Revealed(false),
                    TileBundle {
                        tilemap_id,
                        tile_idx,
                        cell: cell,
                        transform: Transform::from_xyz(pos.x, pos.y, layer.0),
                        sprite: sheet.sprite_from_idx(tile_idx),
                    },
                ))
                .id();
            storage.set(&cell, entity);
        }
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
            if let Some(tile) = storage.get(&cell) {
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
