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

/// A simple wrapper around an image handle and a texture atlas layout that
/// provides helper methods for creating sprites from the atlas.
#[derive(Resource, Debug, Default, Reflect, Clone)]
#[reflect(Resource)]
pub struct SpriteAtlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl SpriteAtlas {
    pub fn sprite(&self) -> Sprite {
        self.sprite_from_idx(0usize)
    }

    pub fn sprite_from_idx(&self, index: impl Into<usize>) -> Sprite {
        let SpriteAtlas { image, layout } = self.clone();

        Sprite {
            image,
            texture_atlas: Some(TextureAtlas {
                layout: layout,
                index: index.into(),
            }),
            ..default()
        }
    }
}

pub(crate) fn default_layout() -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(
        UVec2::splat(tiles::TILE_SIZE_PX as u32),
        tiles::SHEET_SIZE_G.x,
        tiles::SHEET_SIZE_G.y,
        None,
        None,
    )
}

/// Loads the spritesheet asset and creates a [SpriteAtlas] resource from it.
pub(crate) fn load_spritesheet(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image: Handle<Image> = asset_server.load(DEFAULT_SHEET);
    let layout = asset_server.add(default_layout());
    commands.insert_resource(SpriteAtlas { image, layout });
}
