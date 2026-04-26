use bevy::{
    asset::Handle,
    ecs::resource::Resource,
    image::{Image, TextureAtlas, TextureAtlasLayout},
    prelude::*,
    sprite::Sprite,
};

use crate::tiles;

/// The path to the spritesheet image.
pub const DEFAULT_SHEET: &str = "kenney_1-bit-pack/Tilesheet/colored_packed.png";
pub const TRANSPARENT_SHEET: &str = "kenney_1-bit-pack/Tilesheet/colored-transparent_packed.png";

/// A simple wrapper around an image handle and a texture atlas layout that provides helper methods for creating sprites from the atlas.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct SpriteAtlas {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub loaded: bool,
}

impl SpriteAtlas {
    pub fn sprite(&self) -> Sprite {
        Sprite {
            image: self.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: self.layout.clone(),
                ..default()
            }),
            ..default()
        }
    }

    pub fn sprite_from_idx(&self, index: impl Into<usize>) -> Sprite {
        Sprite {
            image: self.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: self.layout.clone(),
                index: index.into(),
            }),
            ..default()
        }
    }
}

/// Loads the spritesheet asset and creates a [SpriteAtlas] resource from it.
pub(crate) fn load_spritesheet(
    mut atlas: ResMut<SpriteAtlas>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture: Handle<Image> = asset_server.load(DEFAULT_SHEET);
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(tiles::TILE_SIZE_PX as u32),
        tiles::SHEET_SIZE_G.x,
        tiles::SHEET_SIZE_G.y,
        None,
        None,
    ));

    *atlas = SpriteAtlas {
        texture: texture.clone(),
        layout: layout.clone(),
        loaded: false,
    };
}

pub(crate) fn on_loaded(mut atlas: ResMut<SpriteAtlas>, asset_server: Res<AssetServer>) {
    if atlas.loaded {
        return;
    }

    let state = asset_server.recursive_dependency_load_state(atlas.texture.id());
    if !state.is_loaded() {
        trace!("LoadState: sprite texture not ready");
        return;
    }

    atlas.loaded = state.is_loaded();
}
