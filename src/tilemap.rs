use bevy::{
    camera::visibility::InheritedVisibility,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        system::{Commands, Res},
    },
    log::info,
    prelude::Deref,
    sprite::Sprite,
    transform::components::{GlobalTransform, Transform},
};
use bevy_egui::egui::Vec2;

use crate::{
    SpriteAtlas,
    cell::Cell,
    map::MapSpec,
    tiles::{MapTile, TileIdx},
};

#[derive(Component, Deref, Clone, Copy)]
pub struct TilemapId(pub Entity);

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct TilemapLayer(pub f32);

#[derive(Debug, Default, Component, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Default, Component, Clone, PartialEq, Eq, Hash)]
/// Attach this to an entity with TilemapId.
pub struct TilemapStorage {
    tiles: Vec<Option<Entity>>,
    pub size: TilemapSize,
}

impl TilemapStorage {
    pub fn empty(width: u32, height: u32, tile_size: u32) -> TilemapStorage {
        TilemapStorage {
            tiles: vec![None; (width * height) as usize],
            size: TilemapSize {
                width,
                height,
                tile_size,
            },
        }
    }

    pub fn get(&self, cell: &Cell) -> Option<Entity> {
        let idx = cell.to_idx(self.size.width);
        self.tiles[idx as usize]
    }

    pub fn set(&mut self, cell: &Cell, entity: Entity) {
        let idx = cell.to_idx(self.size.width);
        self.tiles[idx as usize] = Some(entity);
    }

    pub fn remove(&mut self, cell: &Cell) {
        let idx = cell.to_idx(self.size.width);
        self.tiles[idx as usize] = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = &Option<Entity>> {
        self.tiles.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Option<Entity>> {
        self.tiles.iter_mut()
    }
}

#[derive(Component, Clone, Copy)]
pub struct TileTextureIdx(pub usize);

#[derive(Bundle)]
pub struct TileBundle {
    pub tilemap_id: TilemapId,
    pub tile_idx: TileIdx,
    pub cell: Cell,
    pub transform: Transform,
    pub sprite: Sprite,
}

#[derive(Bundle, Default)]
pub struct TileMapBundle {
    pub size: TilemapSize,
    pub storage: TilemapStorage,
    pub layer: TilemapLayer,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: InheritedVisibility,
}

pub fn setup_tilemap(mut commands: Commands, spec: Res<MapSpec>, sheet: Res<SpriteAtlas>) {
    let map_entity = commands.spawn_empty().id();

    let tilemap_id = TilemapId(map_entity);
    let mut tilemap_storage = TilemapStorage::empty(spec.size.x, spec.size.y, spec.tile_size);
    let size = TilemapSize {
        width: spec.size.x,
        height: spec.size.y,
        tile_size: spec.tile_size,
    };
    let layer = TilemapLayer(spec.layer as f32 - 3.);

    info!(
        "initializing tilemap with size {:?} and layer {:?}",
        size, layer
    );

    let tile_idx = TileIdx::Dirt;

    fill_tilemap(
        tile_idx,
        tilemap_id,
        size,
        layer,
        &sheet,
        &mut commands,
        &mut tilemap_storage,
    );

    let tilemap_bundle = TileMapBundle {
        size: size,
        storage: tilemap_storage,
        layer: layer,
        ..Default::default()
    };

    commands
        .entity(map_entity)
        .insert((tilemap_id, tilemap_bundle));
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
    commands.entity(tilemap_id.0).with_children(|parent| {
        for x in 0..size.width {
            for y in 0..size.height {
                let cell = Cell::new(x as i32, y as i32);
                let pos = size.cell_to_pos(&cell);
                let entity = parent
                    .spawn((
                        MapTile,
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
    });
}
