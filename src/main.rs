use std::collections::HashMap;
use std::ops::Deref;

use bevy::prelude::*;
use itertools::iproduct;

mod cell;
mod events;
mod states;

use cell::Cell;

/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored-transparent_packed.png";
/// The grid size of the spritesheet.
const SHEET_SIZE_G: UVec2 = uvec2(49, 22);
/// The tile size in pixels.
const TILE_SIZE_PX: f32 = 16.0;

/// The size of the map in cells.
const MAP_SIZE_G: UVec2 = uvec2(10, 10);

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(CLEAR_COLOR)
        .add_systems(
            Startup,
            (load_spritesheet, init_map, decorate_map, setup_camera).chain(),
        )
        .add_systems(Update, handle_player_input)
        .add_systems(PostUpdate, update_tiles)
        .init_resource::<SpatialIndex>()
        .run();
}

#[derive(Resource, Debug)]
pub struct SpriteAtlas {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl SpriteAtlas {
    pub fn new(texture: Handle<Image>, layout: Handle<TextureAtlasLayout>) -> Self {
        Self { texture, layout }
    }

    pub fn sprite_from_idx(&self, index: AtlasIdx) -> Sprite {
        Sprite {
            image: self.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: self.layout.clone(),
                index: index.0,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn sprite_from_coords(&self, xy: UVec2) -> Sprite {
        let index = xy.x + xy.y * SHEET_SIZE_G.x;
        self.sprite_from_idx(AtlasIdx(index as usize))
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct AtlasIdx(pub usize);

impl Deref for AtlasIdx {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource, Debug)]
pub struct MapSpec {
    pub size: UVec2,
    pub default_sprite_idx: AtlasIdx,
}

#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct SpatialIndex {
    occupied: HashMap<Cell, Entity>,
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self {
            occupied: HashMap::new(),
        }
    }

    pub fn insert(&mut self, cell: Cell, entity: Entity) {
        self.occupied.insert(cell, entity);
    }

    pub fn remove(&mut self, cell: Cell) {
        self.occupied.remove(&cell);
    }

    pub fn get(&self, cell: Cell) -> Option<Entity> {
        self.occupied.get(&cell).copied()
    }

    pub fn is_occupied(&self, cell: Cell) -> bool {
        self.occupied.contains_key(&cell)
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct PieceBundle {
    pub sprite: Sprite,
    pub cell: Cell,
    pub atlas_index: AtlasIdx,
    pub transform: Transform,
}

fn setup_camera(mut commands: Commands) {
    // Spawn the camera using a 2D orthographic projection.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_translation(Vec3::new(
            (MAP_SIZE_G.x as f32 * TILE_SIZE_PX) / 2.0 - TILE_SIZE_PX / 2.0,
            (MAP_SIZE_G.y as f32 * TILE_SIZE_PX) / 2.0 - TILE_SIZE_PX / 2.0,
            0.0,
        )),
    ));
}

#[derive(Component, Debug)]
struct Tile;

fn init_map(mut commands: Commands, sprite_atlas: Res<SpriteAtlas>) {
    let map_spec = MapSpec {
        size: MAP_SIZE_G,
        default_sprite_idx: AtlasIdx(0),
    };

    for (x, y) in iproduct!(0..map_spec.size.x, 0..map_spec.size.y) {
        commands.spawn((
            Tile,
            PieceBundle {
                sprite: sprite_atlas.sprite_from_idx(map_spec.default_sprite_idx),
                cell: Cell::at_coords(x, y),
                atlas_index: map_spec.default_sprite_idx,
                transform: Transform::from_translation(Vec3::new(
                    x as f32 * TILE_SIZE_PX,
                    y as f32 * TILE_SIZE_PX,
                    -3.0,
                )),
            },
        ));
    }

    commands.insert_resource(map_spec);
}

fn decorate_map(_map_spec: Res<MapSpec>, mut tiles: Query<(&mut Sprite, &Cell), With<Tile>>) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    for (mut sprite, cell) in tiles.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            let mut hasher = DefaultHasher::new();
            cell.hash(&mut hasher);
            let hash = hasher.finish();

            let mut rng = StdRng::seed_from_u64(hash);
            let result = rng.next_u32() % 6;
            texture_atlas.index = result as usize;
        }
    }
}

fn update_tiles(mut tiles: Query<(&mut Sprite, &AtlasIdx), (With<Tile>, Changed<AtlasIdx>)>) {
    for (mut sprite, idx) in tiles.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = idx.0;
        }
    }
}

fn load_spritesheet(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture: Handle<Image> = asset_server.load(SHEET_PATH);
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE_PX as u32),
        SHEET_SIZE_G.x,
        SHEET_SIZE_G.y,
        None,
        None,
    ));

    commands.insert_resource(SpriteAtlas {
        texture: texture.clone(),
        layout: layout.clone(),
    });
}

#[derive(Component, Debug)]
pub struct Player;

fn handle_player_input(
    mut _commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<&mut Cell, With<Player>>,
) {
    if let Ok(mut player_cell) = player_query.single_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction += Vec3::new(0.0, -1.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction += Vec3::new(-1.0, 0.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        player_cell.combine(direction.into());
    }
}
