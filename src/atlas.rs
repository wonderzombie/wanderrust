use bevy::{
    asset::Handle,
    ecs::resource::Resource,
    image::{Image, TextureAtlas, TextureAtlasLayout},
    sprite::Sprite,
};

#[derive(Resource, Debug)]
/// A simple wrapper around an image handle and a texture atlas layout that provides helper methods for creating sprites from the atlas.
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
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn sprite_from_idx(&self, index: impl Into<usize>) -> Sprite {
        Sprite {
            image: self.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: self.layout.clone(),
                index: index.into(),
            }),
            ..Default::default()
        }
    }
}
