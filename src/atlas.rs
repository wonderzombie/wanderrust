use bevy::{
    asset::Handle,
    ecs::resource::Resource,
    image::{Image, TextureAtlas, TextureAtlasLayout},
    prelude::*,
    sprite::Sprite,
};

use crate::{gamestate::GameState, tiles};

/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored_packed.png";

/// A simple wrapper around an image handle and a texture atlas layout that provides helper methods for creating sprites from the atlas.
#[derive(Resource, Debug)]
pub struct SpriteAtlas {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
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
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let texture: Handle<Image> = asset_server.load(SHEET_PATH);
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(tiles::TILE_SIZE_PX as u32),
        tiles::SHEET_SIZE_G.x,
        tiles::SHEET_SIZE_G.y,
        None,
        None,
    ));

    commands.insert_resource(SpriteAtlas {
        texture: texture.clone(),
        layout: layout.clone(),
    });

    next.set(GameState::Loading);
}
