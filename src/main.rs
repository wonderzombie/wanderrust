mod cell;
mod event_log;
mod inventory;
mod map;
mod tiles;

use std::{collections::HashMap, ops::Add};

use bevy::prelude::*;

use cell::Cell;
use mrpas::Mrpas;
use tiles::{AtlasIdx, MapTile, TileIdx, Walkable};

use crate::map::MapSpec;

use inventory::*;

/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored_packed.png";
/// The tile size in pixels.
const TILE_SIZE_PX: f32 = 16.0;

/// The size of the map in cells.
const MAP_SIZE_G: UVec2 = uvec2(30, 25);

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

#[derive(Debug, Resource, Deref, DerefMut)]
/// Newtype for field of view model that tracks which cells are transparent for visibility calculations.
struct Fov(Mrpas);

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (800, 600).into(),
                        title: "wanderrust".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_message::<ActionAttempt>()
        .add_message::<Acquisition>()
        .insert_resource(CLEAR_COLOR)
        .insert_resource(MapSpec::from_str(map::MAP))
        .insert_resource(event_log::MessageLog::new(10))
        .init_resource::<SpatialIndex>()
        .init_resource::<Inventory>()
        .add_systems(
            Startup,
            (
                load_spritesheet,
                map::init_map,
                map::decorate_map,
                map::draw_ascii_map,
                setup_interactables,
                setup_camera,
                setup_player,
                event_log::setup_log,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                handle_player_input,
                process_action_attempts,
                process_acquisitions,
                update_camera,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (
                map::update_map_tiles,
                sync_actor_sprites,
                update_piece_transforms,
                update_spatial_index,
                update_fov_model,
                event_log::update_log_display,
            )
                .chain(),
        )
        .run();
}

#[derive(Resource, Debug)]
/// A simple wrapper around an image handle and a texture atlas layout that provides helper methods for creating sprites from the atlas.
pub struct SpriteAtlas {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl SpriteAtlas {
    pub fn new(texture: Handle<Image>, layout: Handle<TextureAtlasLayout>) -> Self {
        Self { texture, layout }
    }

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
}

#[derive(Component, Debug)]
/// A marker component for entities that perform actions in the world, such as the player or NPCs.
pub struct Actor;

#[derive(Resource, Default, Debug, PartialEq, Eq)]
/// A spatial index that tracks which cells are occupied by non-walkable entities in the world.
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
/// A bundle for map pieces that includes a sprite, cell position, and transform.
pub struct PieceBundle {
    pub sprite: Sprite,
    pub cell: Cell,
    pub transform: Transform,
}

fn setup_player(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Player,
        Actor,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell::new(5, 5),
            transform: Transform::from_xyz(5.0 * TILE_SIZE_PX, 5.0 * TILE_SIZE_PX, -1.0),
        },
        TileIdx::Player,
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

fn sync_actor_sprites(
    mut pieces: Query<(&mut Sprite, &TileIdx), (Without<MapTile>, Changed<TileIdx>)>,
) {
    for (mut sprite, tile_idx) in pieces.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = (*tile_idx).into();
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
    query: Query<(Entity, &Cell), Without<Walkable>>,
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
    mut events: MessageWriter<ActionAttempt>,
    input: Res<ButtonInput<KeyCode>>,
    player_query: Query<(Entity, &Cell), With<Player>>,
) {
    let Some(directiom) = get_direction(&input) else {
        return;
    };

    let Ok((player_entity, player_cell)) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    events.write(ActionAttempt {
        interactor: player_entity,
        target_cell: player_cell.add(directiom),
    });
}

fn get_direction(input: &ButtonInput<KeyCode>) -> Option<IVec2> {
    let mut direction = IVec2::ZERO;

    if input.just_pressed(KeyCode::KeyW) {
        direction += IVec2::Y;
    }
    if input.just_pressed(KeyCode::KeyS) {
        direction += IVec2::NEG_Y;
    }
    if input.just_pressed(KeyCode::KeyA) {
        direction += IVec2::NEG_X;
    }
    if input.just_pressed(KeyCode::KeyD) {
        direction += IVec2::X;
    }

    if direction != IVec2::ZERO {
        Some(direction)
    } else {
        None
    }
}

#[derive(Message, Debug)]
/// A message representing an attempt by an actor to interact with a cell in the world, such as moving into it or interacting with an object on it.
pub struct ActionAttempt {
    pub interactor: Entity,
    pub target_cell: Cell,
}

#[derive(Component, Debug)]
/// A component representing an interactable object in the world, such as a door or chest, that can be interacted with by actors.
pub enum Interactable {
    Door { is_open: bool },
    Chest { is_open: bool, contents: Vec<Item> },
}

fn process_action_attempts(
    mut commands: Commands,
    mut interactions: MessageReader<ActionAttempt>,
    mut interactables: Query<(&mut TileIdx, &mut Interactable)>,
    mut acquisitions: MessageWriter<Acquisition>,
    spatial_index: Res<SpatialIndex>,
) {
    for message in interactions.read() {
        let Some(target_entity) = spatial_index.get(message.target_cell) else {
            // No entity at the target cell, so we can assume it's an empty walkable tile.
            // Only non-walkable tiles are added to the spatial index, so if there's no entity, it's safe to move there.
            commands
                .entity(message.interactor)
                .insert(message.target_cell);
            continue;
        };

        let Ok((mut tile_idx, mut interactable)) = interactables.get_mut(target_entity) else {
            info!(
                "Player interacts with an entity at {:?}, but it's not interactable.",
                message.target_cell
            );
            continue; // There is a target entity, but it's not interactable.
        };

        match &mut *interactable {
            Interactable::Door { is_open } => {
                if !*is_open {
                    *is_open = true;
                    *tile_idx = TileIdx::DoorwayBrownThick;
                    info!("Player opens the door.");
                }
            }
            Interactable::Chest { is_open, contents } => {
                if !*is_open {
                    *is_open = true;
                    *tile_idx = TileIdx::ChestBrownOpen;
                    info!("Player opens the chest and finds: {:?}", contents);
                    acquisitions.write(Acquisition {
                        acquirer: message.interactor,
                        items: contents.clone().into(),
                    });
                }
            }
        }
    }
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    player_query: Query<&Cell, With<Player>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    let mut camera_transform = camera_query.single_mut().unwrap();
    camera_transform.translation.x = (player_cell.x as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
    camera_transform.translation.y = (player_cell.y as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
}

pub fn setup_interactables(
    mut commands: Commands,
    tiles: Query<(Entity, &TileIdx), With<MapTile>>,
) {
    for (entity, tile_idx) in tiles.iter() {
        if tile_idx.is_interactable() {
            let bundle = match tile_idx {
                TileIdx::ChestBrownClosed | TileIdx::ChestWhiteClosed => {
                    Some(Interactable::Chest {
                        is_open: false,
                        contents: vec![Item("gold".to_string())],
                    })
                }
                TileIdx::DoorBrownThickClosed1
                | TileIdx::DoorBrownThickClosed2
                | TileIdx::DoorBrownThickClosed3 => Some(Interactable::Door { is_open: false }),
                _ => None,
            };

            if let Some(bundle) = bundle {
                commands.entity(entity).insert(bundle);
            }
        }
    }
}
