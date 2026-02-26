use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    SpriteAtlas,
    cell::Cell,
    map::MapSpec,
    tiles::{MapTile, Revealed, TileIdx},
};

#[derive(Component, Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq)]
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
/// Attach this to an entity with TilemapId.
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

    // pub fn get(&self, cell: &Cell) -> Option<Entity> {
    //     let idx = cell.to_idx(self.size.width) as usize;
    //     self.tiles.get(idx).copied().flatten()
    // }

    pub fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = cell.to_idx(self.size.width) as usize;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SavedTilemap {
    pub tiles: Vec<TileIdx>,
    pub size: MapDimensions,
    pub layer: TilemapLayer,
}

#[derive(Bundle, Clone)]
pub struct TileBundle {
    pub map_tile: MapTile,
    pub tile_idx: TileIdx,
    pub cell: Cell,
    pub transform: Transform,
    pub sprite: Sprite,
    pub revealed: Revealed,
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

pub fn spawn_tilemap(mut commands: Commands, mut spec: ResMut<MapSpec>, sheet: Res<SpriteAtlas>) {
    let layer = TilemapLayer(spec.layer as f32 - 3.);
    let tilemap_bundle = TilemapBundle {
        size: spec.size,
        layer,
        ..Default::default()
    };

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        spec.size, layer
    );

    let map_entity = commands.spawn(tilemap_bundle).id();
    spec.id.set(map_entity);
    spawn_maptiles_from_spec(&spec, &sheet, &mut commands);
    commands.entity(map_entity).insert(spec.id);
}

fn spawn_maptiles_from_spec(spec: &MapSpec, sheet: &SpriteAtlas, commands: &mut Commands) {
    let bundles: Vec<TileBundle> = spec
        .flat_pieces
        .iter()
        .map(|(tile_idx, cell)| {
            let pos = spec.size.cell_to_pos(&cell);

            TileBundle {
                map_tile: MapTile,
                tile_idx: *tile_idx,
                cell: *cell,
                transform: Transform::from_xyz(pos.x, pos.y, spec.layer as f32 - 3.0),
                sprite: sheet.sprite_from_idx(*tile_idx),
                revealed: Revealed(false),
            }
        })
        .collect();

    commands.spawn_batch(bundles);
}

pub fn store_maptiles_by_cell(
    mut commands: Commands,
    spec: Res<MapSpec>,
    tiles: Query<(&Cell, Entity), With<MapTile>>,
) {
    let map_entity = spec.id.get().expect("MapSpec is missing an ID");

    info!("storing maps by cell");

    let mut storage = TileStorage::new(spec.size);
    for (cell, entity) in tiles.iter() {
        storage.set(cell, entity);
    }
    info!("stored tiles: {}", storage.len());
    commands.entity(map_entity).insert(storage);
}

pub fn save_map(storage: &TileStorage, all_tiles: &Query<&TileIdx, With<MapTile>>) -> SavedTilemap {
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

pub fn load_map(commands: &mut Commands, saved: &SavedTilemap, storage: &mut TileStorage) {
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
