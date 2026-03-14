mod cell;
mod colors;
mod editor;
mod event_log;
mod fov;
mod inventory;
mod light;
mod map;
mod player;
mod procgen;
mod ptable;
mod tilemap;
mod tiles;
mod transition;

use std::{collections::HashMap, ops::Add};

use bevy::prelude::*;

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use crate::{
    cell::Cell,
    editor::{DesiredZoom, EditorState},
    event_log::{draw_message_log_ui, setup_egui_fonts},
    light::{Emitter, LightLevel},
    player::PlayerStats,
    tilemap::{TileStorage, TilemapSpec},
    tiles::{MapTile, Occupied, TileIdx, Walkable},
};

use inventory::*;

/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored_packed.png";
/// The tile size in pixels.
const TILE_SIZE_PX: f32 = 16.0;

/// The size of the map in cells.
const MAP_SIZE_G: UVec2 = uvec2(30, 25);

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

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
        .add_plugins(EguiPlugin::default())
        .add_message::<ActionAttempt>()
        .add_message::<Acquisition>()
        .init_resource::<SpatialIndex>()
        .init_resource::<Inventory>()
        .init_resource::<EditorState>()
        .init_resource::<PlayerStats>()
        .insert_resource(CLEAR_COLOR)
        .insert_resource(SpritePickingSettings {
            // clicking on a sprite ignores alpha transparency
            picking_mode: SpritePickingMode::BoundingBox,
            // we have no specifically sprite picking camera yet
            require_markers: false,
        })
        .insert_resource(TilemapSpec::with_ptable(
            procgen::biome_ptable(),
            procgen::tile_idx_for_cell,
            (100, 100),
        ))
        .insert_resource(event_log::MessageLog::new(5))
        .add_plugins(editor::EditorPlugin)
        .add_systems(
            Startup,
            (
                load_spritesheet,
                tilemap::spawn_tilemap.after(load_spritesheet),
                tilemap::initialize_tile_storage.after(tilemap::spawn_tilemap),
                setup_interactables.after(tilemap::initialize_tile_storage),
                setup_player.after(load_spritesheet),
                fov::setup_fov.after(tilemap::initialize_tile_storage),
                setup_camera,
            ),
        )
        .add_systems(
            PostStartup,
            (
                add_test_npc.run_if(run_once).after(load_spritesheet),
                add_test_emitters.run_if(run_once).after(load_spritesheet),
            ),
        )
        .add_systems(Update, setup_egui_fonts.run_if(run_once))
        .add_systems(
            Update,
            (
                handle_player_input,
                process_action_attempts,
                process_acquisitions,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (
                map::sync_tiles,
                sync_actor_sprites,
                sync_occupied_tiles,
                update_actor_transforms,
                update_camera.after(update_actor_transforms),
                update_spatial_index,
                fov::update_fov_model.after(map::sync_tiles),
                fov::update_fov_markers.after(fov::update_fov_model),
                light::update_emitter_lights.after(fov::update_fov_markers),
                light::sync_actor_light_levels.after(light::update_emitter_lights),
            ),
        )
        .add_systems(Last, map::update_tile_visuals)
        .add_systems(EguiPrimaryContextPass, draw_message_log_ui)
        .run();
}

fn add_test_npc(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            transform: Transform::default(),
        },
        Actor,
        TileIdx::Skeleton,
        Interactable::Dialogue {
            name: "Mr. Boney".into(),
            text: "Hello".into(),
        },
    ));
}

fn add_test_emitters(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        TileIdx::Torch,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 47, y: 47 },
            transform: Transform::default(),
        },
        Emitter::new((LightLevel::Light, 1), (LightLevel::Dim, 1)),
    ));
}

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

fn setup_player(mut commands: Commands, spec: Res<TilemapSpec>, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Player,
        Actor,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: spec.start,
            transform: Transform::from_xyz(
                spec.start.x as f32 * TILE_SIZE_PX,
                spec.start.y as f32 * TILE_SIZE_PX,
                -1.0,
            ),
        },
        TileIdx::Player,
        Emitter::new((LightLevel::Bright, 2), (LightLevel::Light, 1)),
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

/// Syncs changed actor [TileIdx] for [Sprite]s `Without<MapTile>`.
fn sync_actor_sprites(
    mut pieces: Query<(&mut Sprite, &TileIdx), (Without<MapTile>, Changed<TileIdx>)>,
) {
    for (mut sprite, tile_idx) in pieces.iter_mut() {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = tile_idx.into();
        }
    }
}

/// Updates the [Transform] of pieces based on their [Cell] coordinates when the cell changes.
fn update_actor_transforms(
    mut pieces: Query<(&Cell, &mut Transform), (With<Actor>, Changed<Cell>)>,
) {
    for (piece_cell, mut transform) in pieces.iter_mut() {
        transform.translation.x = piece_cell.x as f32 * TILE_SIZE_PX;
        transform.translation.y = piece_cell.y as f32 * TILE_SIZE_PX;
    }
}

/// Updates [SpatialIndex] resource based on the current [Cell] of non-walkable entities in the world.
fn update_spatial_index(
    mut index: ResMut<SpatialIndex>,
    query: Query<(Entity, &Cell), Without<Walkable>>,
) {
    index.clear();
    for (entity, cell) in query.iter() {
        index.insert(*cell, entity);
    }
}

fn sync_occupied_tiles(
    mut commands: Commands,
    actors: Query<(&Cell, Option<&PreviousCell>), (With<Actor>, Changed<Cell>)>,
    storage: Single<&TileStorage>,
) {
    for (curr_cell, prev_cell_opt) in actors.iter() {
        if let Some(tile) = storage.get(curr_cell) {
            commands.entity(tile).insert(Occupied);
        }

        if let Some(prev_cell) = prev_cell_opt
            && let Some(prev_tile) = storage.get(prev_cell) {
                commands.entity(prev_tile).remove::<Occupied>();
            }
    }
}

/// Loads the spritesheet asset and creates a [SpriteAtlas] resource from it.
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

/// Handles player input and sends an [ActionAttempt] message derived from player input.
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
        origin_cell: *player_cell,
        target_cell: player_cell.add(directiom),
    });
}

/// Returns the [IVec2] direction implied by [KeyCode], if any.
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
    pub origin_cell: Cell,
    pub target_cell: Cell,
}

#[derive(Component, Debug)]
/// A component representing an interactable object in the world, such as a door or chest, that can be interacted with by actors.
pub enum Interactable {
    Door {
        is_open: bool,
        requires: Option<Item>,
    },
    Chest {
        is_open: bool,
        contents: Inventory,
    },
    Dialogue {
        name: String,
        text: String,
    },
}

#[derive(Component, Debug, Deref)]
struct PreviousCell(Cell);

/// Processes [ActionAttempt] messages, either moving the player or interacting with an interactable entity at the target [Cell] using [SpatialIndex].
fn process_action_attempts(
    mut commands: Commands,
    mut log: ResMut<event_log::MessageLog>,
    mut interactions: MessageReader<ActionAttempt>,
    mut interactables: Query<(&mut TileIdx, &mut Interactable)>,
    mut acquisitions: MessageWriter<Acquisition>,
    player_inventory: Res<Inventory>,
    spatial_index: Res<SpatialIndex>,
) {
    for message in interactions.read() {
        let Some(target_entity) = spatial_index.get(message.target_cell) else {
            // No entity at the target [Cell], so we can assume it's an empty walkable tile.
            // Changing the [Cell] via insertion will cause the system to move the player sprite.
            commands
                .entity(message.interactor)
                .insert(message.target_cell)
                .insert(PreviousCell(message.origin_cell));

            log.add("You move.", colors::KENNEY_OFF_WHITE);
            continue;
        };

        let Ok((mut tile_idx, mut interactable)) = interactables.get_mut(target_entity) else {
            info!(
                "Player interacts with an entity at {:?}, but it's not interactable.",
                message.target_cell
            );
            continue; // There is a target entity, but it's not interactable.
        };

        handle_interaction(
            &mut tile_idx,
            &mut interactable,
            &player_inventory,
            &mut acquisitions,
            message.interactor,
            &mut log,
        );
    }
}

/// Handles the interaction between the player and an interactable entity with [TileIdx] at the target [Cell].
fn handle_interaction(
    tile_idx: &mut TileIdx,
    interactable: &mut Interactable,
    inventory: &Inventory,
    acquisitions: &mut MessageWriter<Acquisition>,
    interactor: Entity,
    log: &mut event_log::MessageLog,
) {
    match interactable {
        Interactable::Door { is_open, requires } => {
            if !*is_open {
                if let Some(required_item) = requires {
                    if !inventory.has_item(required_item) {
                        info!("Player does not have the required item to open the door.");
                        log.add("Locked.", colors::KENNEY_BLUE);
                        return;
                    } else {
                        info!("Player uses {:?} to open the door.", required_item);
                        log.add(
                            format!("Opened door with {}.", required_item),
                            colors::KENNEY_BLUE,
                        );
                    }
                } else {
                    info!("Player opens the door.");
                    log.add("Opened door.", colors::KENNEY_BLUE);
                }
                *is_open = true;
                *tile_idx = tile_idx.opened_version().unwrap_or(*tile_idx);
            }
        }
        Interactable::Chest { is_open, contents } => {
            if !*is_open {
                *is_open = true;
                *tile_idx = tile_idx.opened_version().unwrap_or(*tile_idx);
                info!("Player opens the chest and finds: {:?}", contents);
                log.add("Opened chest.", colors::KENNEY_BLUE);
                log.add_all(contents.summary("got").as_ref(), colors::KENNEY_GREEN);
                acquisitions.write(Acquisition {
                    acquirer: interactor,
                    items: contents.clone(),
                });
            }
        }
        Interactable::Dialogue { name, text } => {
            info!("Player talks to {}.", name);
            log.add(format!("{}: {}", name, text), colors::KENNEY_BLUE);
        }
    }
}

fn update_camera(
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    player_query: Query<&Cell, With<Player>>,
    zoom_opt: Option<Res<DesiredZoom>>,
) {
    let Ok(player_cell) = player_query.single() else {
        warn!("No player entity found in the world.");
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        warn!("No camera entity found in the world.");
        return;
    };

    camera_transform.translation.x = (player_cell.x as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
    camera_transform.translation.y = (player_cell.y as f32 * TILE_SIZE_PX) + (TILE_SIZE_PX / 2.0);
    let zoom = zoom_opt.map_or(1.0, |zoom| zoom.0);
    camera_transform.scale = Vec3::splat(zoom);
}

pub fn setup_interactables(
    mut commands: Commands,
    tiles: Query<(Entity, &TileIdx), With<MapTile>>,
) {
    for (entity, tile_idx) in tiles.iter() {
        if !tile_idx.is_interactable() {
            continue;
        }

        let bundle = match tile_idx {
            TileIdx::ChestBrownClosed | TileIdx::ChestWhiteClosed => Some(Interactable::Chest {
                is_open: false,
                contents: Inventory::with_item(Item("gold".to_string()), 10),
            }),
            TileIdx::DoorBrownThickClosed1
            | TileIdx::DoorBrownThickClosed2
            | TileIdx::DoorBrownThickClosed3 => Some(Interactable::Door {
                is_open: false,
                requires: None,
            }),
            _ => None,
        };

        if let Some(bundle) = bundle {
            commands.entity(entity).insert(bundle);
        }
    }
}
