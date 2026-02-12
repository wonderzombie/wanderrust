mod cell;
mod events;
mod states;
mod tiles;

use std::{collections::HashMap, ops::Add};

use bevy::prelude::*;
use itertools::iproduct;

use cell::Cell;
use mrpas::Mrpas;
use tiles::TileIdx;

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

const PLAYER_SPRITE_IDX: AtlasIdx = AtlasIdx(27);

#[derive(Debug, Resource, Deref, DerefMut)]
struct Fov(Mrpas);

impl From<TileIdx> for AtlasIdx {
    fn from(tile: TileIdx) -> AtlasIdx {
        AtlasIdx(tile as usize)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(CLEAR_COLOR)
        .add_systems(
            Startup,
            (
                load_spritesheet,
                init_map,
                decorate_map,
                setup_camera,
                setup_player,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (handle_player_input, validate_player_action, update_camera).chain(),
        )
        .add_systems(
            PostUpdate,
            (
                update_tiles,
                update_pieces,
                update_spatial_index,
                update_fov_model,
            )
                .chain(),
        )
        .init_resource::<SpatialIndex>()
        .init_resource::<PendingPlayerAction>()
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

#[derive(Component, Debug, Deref, DerefMut, Clone, Copy)]
pub struct AtlasIdx(pub usize);

#[derive(Resource, Debug)]
pub struct MapSpec {
    pub size: UVec2,
    pub default_tile: TileIdx,
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

    pub fn clear(&mut self) {
        self.occupied.clear();
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

fn setup_player(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Player,
        PieceBundle {
            sprite: atlas.sprite_from_idx(PLAYER_SPRITE_IDX),
            cell: Cell::new(5, 5),
            atlas_index: PLAYER_SPRITE_IDX.into(),
            transform: Transform::default(),
        },
    ));
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
struct MapTile;

fn init_map(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    let spec = MapSpec {
        size: MAP_SIZE_G,
        default_tile: TileIdx::Blank,
    };

    let fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));
    commands.insert_resource(fov);

    for (x, y) in iproduct!(0..spec.size.x, 0..spec.size.y) {
        commands.spawn((
            MapTile,
            PieceBundle {
                sprite: atlas.sprite_from_idx(spec.default_tile.into()),
                cell: Cell::at_coords(x, y),
                atlas_index: spec.default_tile.into(),
                transform: Transform::from_xyz(
                    x as f32 * TILE_SIZE_PX,
                    y as f32 * TILE_SIZE_PX,
                    -3.0,
                ),
            },
        ));
    }

    commands.insert_resource(spec);
}

fn decorate_map(mut tiles: Query<(&mut AtlasIdx, &Cell), With<MapTile>>) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    for (mut atlas_idx, cell) in tiles.iter_mut() {
        let mut hasher = DefaultHasher::new();
        cell.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = StdRng::seed_from_u64(hash);
        let result = rng.next_u32() % 6 + 1;
        atlas_idx.0 = result as usize;
    }
}

fn update_tiles(mut tiles: Query<(&mut Sprite, &AtlasIdx), (With<MapTile>, Changed<AtlasIdx>)>) {
    for (mut sprite, idx) in tiles.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = idx.0;
        }
    }
}

fn update_pieces(mut pieces: Query<(&Cell, &mut Transform), Changed<Cell>>) {
    for (piece_cell, mut transform) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * TILE_SIZE_PX;
    }
}

#[derive(Component)]
struct Solid;

#[derive(Component)]
struct Opaque;

fn update_spatial_index(
    mut index: ResMut<SpatialIndex>,
    query: Query<(Entity, &Cell), With<Solid>>,
) {
    index.clear();
    for (entity, cell) in query.iter() {
        index.insert(cell.clone(), entity);
    }
}

fn update_fov_model(mut fov: ResMut<Fov>, query: Query<&Cell, With<Opaque>>) {
    for cell in query.iter() {
        let (x, y) = (*cell).into();
        fov.set_transparent((x, y), true);
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
    player_query: Query<&Cell, With<Player>>,
    mut pending_action: ResMut<PendingPlayerAction>,
) {
    if let Ok(_) = player_query.single() {
        let mut direction = IVec2::ZERO;

        if keyboard_input.just_pressed(KeyCode::KeyW) {
            direction += IVec2::Y;
        }
        if keyboard_input.just_pressed(KeyCode::KeyS) {
            direction += IVec2::NEG_Y;
        }
        if keyboard_input.just_pressed(KeyCode::KeyA) {
            direction += IVec2::NEG_X;
        }
        if keyboard_input.just_pressed(KeyCode::KeyD) {
            direction += IVec2::X;
        }

        pending_action.action = Some(PlayerAction::Move);
        pending_action.direction = Some(direction);
    }
}

#[derive(Resource, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingPlayerAction {
    pub action: Option<PlayerAction>,
    pub direction: Option<IVec2>,
}

impl PendingPlayerAction {
    pub fn new() -> Self {
        Self {
            action: None,
            direction: None,
        }
    }

    pub fn clear(&mut self) {
        self.action = None;
        self.direction = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    Move,
    Interact,
}

fn validate_player_action(
    mut pending: ResMut<PendingPlayerAction>,
    spatial: Res<SpatialIndex>,
    mut player: Query<&mut Cell, With<Player>>,
) {
    let Ok(mut player_cell) = player.single_mut() else {
        return;
    };

    let PendingPlayerAction {
        ref action,
        ref direction,
    } = *pending;

    match (action, direction) {
        (Some(PlayerAction::Move), Some(direction)) => {
            let target_cell = player_cell.add(*direction);
            if spatial.is_occupied(target_cell) {
                info!(
                    "Player tried to move into an occupied cell {:?}",
                    target_cell
                );
            } else {
                *player_cell = target_cell;
            }
            pending.clear();
        }
        _ => return,
    }
}


fn update_camera(mut camera_query: Query<&mut Transform, With<Camera2d>>, player_query: Query<&Cell, With<Player>>) {
    let Ok(player_cell) = player_query.single() else {
        return;
    };

    let mut camera_transform = camera_query.single_mut().unwrap();
    camera_transform.translation.x = (player_cell.x as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
    camera_transform.translation.y = (player_cell.y as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
}
