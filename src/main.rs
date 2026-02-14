mod cell;
mod event_log;
mod events;
mod states;
mod tiles;

use std::{collections::HashMap, ops::Add};

use bevy::prelude::*;
use itertools::iproduct;

use cell::Cell;
use mrpas::Mrpas;
use tiles::{MapTile, TileIdx, Walkable, Opaque};


/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored-transparent_packed.png";
/// The tile size in pixels.
const TILE_SIZE_PX: f32 = 16.0;

/// The size of the map in cells.
const MAP_SIZE_G: UVec2 = uvec2(30, 25);

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

/// The index of the player sprite in the spritesheet.
const PLAYER_SPRITE_IDX: AtlasIdx = AtlasIdx(27);

#[derive(Debug, Resource, Deref, DerefMut)]
struct Fov(Mrpas);

impl From<TileIdx> for usize {
    fn from(value: TileIdx) -> Self {
        value as usize
    }
}

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
                event_log::setup_log,
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
                update_map_tiles,
                update_piece_transforms,
                update_spatial_index,
                update_fov_model,
                event_log::update_log_display,
            )
                .chain(),
        )
        .insert_resource(MapSpec {
            size: MAP_SIZE_G,
            default_tile: TileIdx::Blank,
        })
        .insert_resource(event_log::MessageLog::new(10))
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

    pub fn sprite_from_coords(&self, coords: UVec2) -> Sprite {
        let index = tiles::atlas_idx(coords.x, coords.y);
        self.sprite_from_idx(AtlasIdx(index))
    }
}

#[derive(Component, Debug, Deref, DerefMut, Clone, Copy)]
pub struct AtlasIdx(pub usize);

#[derive(Component, Debug)]
pub struct Actor;

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
    pub atlas_idx: AtlasIdx,
    pub transform: Transform,
}

fn setup_player(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    let sprite = atlas.sprite_from_idx(PLAYER_SPRITE_IDX);
    commands.spawn((
        Player,
        Actor,
        PieceBundle {
            sprite: sprite,
            cell: Cell::new(5, 5),
            atlas_idx: PLAYER_SPRITE_IDX,
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
        Transform::from_xyz(
            (MAP_SIZE_G.x as f32 * TILE_SIZE_PX) / 2.0 - TILE_SIZE_PX / 2.0,
            (MAP_SIZE_G.y as f32 * TILE_SIZE_PX) / 2.0 - TILE_SIZE_PX / 2.0,
            0.0,
        ),
    ));
}

/// Initializes the map by spawning entities for each cell with the default tile sprite.
fn init_map(mut commands: Commands, atlas: Res<SpriteAtlas>, spec: Res<MapSpec>) {
    let fov = Fov(Mrpas::new(spec.size.x as i32, spec.size.y as i32));
    commands.insert_resource(fov);

    let sprite = atlas.sprite_from_idx(spec.default_tile.into());
    let default: AtlasIdx = spec.default_tile.into();

    for (x, y) in iproduct!(0..spec.size.x, 0..spec.size.y) {
        commands.spawn((
            MapTile,
            PieceBundle {
                sprite: sprite.clone(),
                cell: Cell::at_coords(x, y),
                atlas_idx: default,
                transform: Transform::from_xyz(
                    x as f32 * TILE_SIZE_PX,
                    y as f32 * TILE_SIZE_PX,
                    -3.0,
                ),
            },
            spec.default_tile,
        ));
    }
}

/// Decorates the map by assigning a random ground tile to each cell based on its coordinates.
fn decorate_map(mut tiles: Query<(&mut TileIdx, &Cell), With<MapTile>>) {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::hash::{DefaultHasher, Hash, Hasher};

    let ground_tile_types = [
        TileIdx::Blank,
        TileIdx::Dirt,
        TileIdx::Gravel,
        TileIdx::Grass,
    ];

    for (mut tile_idx, cell) in tiles.iter_mut() {
        let mut hasher = DefaultHasher::new();
        cell.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = StdRng::seed_from_u64(hash);
        let result = rng.next_u32() % (ground_tile_types.len() as u32);
        *tile_idx = ground_tile_types[result as usize];
    }
}

/// Updates the sprites of map tiles when their atlas index changes.
fn update_map_tiles(
    mut commands: Commands,
    mut tiles: Query<(Entity, &mut Sprite, &TileIdx), (With<MapTile>, Changed<TileIdx>)>,
) {
    for (entity, mut sprite, tile_idx) in tiles.iter_mut() {
        let mut entity_command = commands.entity(entity);
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = (*tile_idx).into();
        }
        if tile_idx.is_walkable() {
            entity_command.insert(Walkable);
        } else {
            entity_command.remove::<Walkable>();
        }

        if tile_idx.is_transparent() {
            entity_command.insert(Opaque);
        } else {
            entity_command.remove::<Opaque>();
        }
    }
}

/// Updates the position of pieces based on their cell coordinates when the cell changes.
fn update_piece_transforms(
    mut pieces: Query<(&Cell, &mut Transform), (With<Actor>, Changed<Cell>)>,
) {
    for (piece_cell, mut transform) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * TILE_SIZE_PX;
    }
}

/// Updates the spatial index resource based on the current positions of actors in the world.
fn update_spatial_index(
    mut index: ResMut<SpatialIndex>,
    query: Query<(Entity, &Cell), Without<tiles::Walkable>>,
) {
    index.clear();
    for (entity, cell) in query.iter() {
        index.insert(cell.clone(), entity);
    }
}

/// Updates the field of view model based on the transparency of tiles when their atlas index changes.
fn update_fov_model(
    mut fov: ResMut<Fov>,
    query: Query<(&Cell, &TileIdx), (With<MapTile>, Changed<TileIdx>)>,
) {
    for (cell, tile_idx) in query.iter() {
        let (x, y) = (*cell).into();
        fov.set_transparent((x, y), tile_idx.is_transparent());
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
        tiles::SHEET_SIZE_G.x,
        tiles::SHEET_SIZE_G.y,
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
    mut log: ResMut<event_log::MessageLog>,
    space: Res<SpatialIndex>,
    mut player: Query<&mut Cell, With<Player>>,
) {
    if !pending.is_changed() {
        return;
    }

    let Ok(mut player_cell) = player.single_mut() else {
        return;
    };

    let PendingPlayerAction {
        ref action,
        ref direction,
    } = *pending;

    match (action, direction) {
        (Some(PlayerAction::Move), Some(direction)) => {
            if *direction == IVec2::ZERO {
                pending.clear();
                return;
            }
            let target_cell = player_cell.add(*direction);
            if space.is_occupied(target_cell) {
                info!(
                    "Player tried to move into an occupied cell {:?}",
                    target_cell
                );
                log.add("You bump into something!".to_string());
            } else {
                log.add("You move.".to_string().clone());
                *player_cell = target_cell;
            }
            pending.clear();
        }
        _ => {}
    }
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    player_query: Query<&Cell, With<Player>>,
) {
    let Ok(player_cell) = player_query.single() else {
        return;
    };

    let mut camera_transform = camera_query.single_mut().unwrap();
    camera_transform.translation.x = (player_cell.x as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
    camera_transform.translation.y = (player_cell.y as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
}
