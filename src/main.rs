mod actors;
mod cell;
mod colors;
mod editor;
mod event_log;
mod fov;
mod inventory;
mod light;
mod map;
mod procgen;
mod ptable;
mod tilemap;
mod tiles;

use std::collections::HashMap;

use bevy::prelude::*;

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use crate::{
    actors::*,
    cell::Cell,
    editor::{DesiredZoom, EditorState},
    event_log::{MessageLog, draw_message_log_ui, setup_egui_fonts},
    inventory::*,
    light::{Emitter, LightLevel},
    tilemap::{EntryId, Portal, TilemapLayer, TilemapSpec},
    tiles::{MapTile, TileIdx, Walkable},
};

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
        .add_message::<Damage>()
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
                add_test_npc.run_if(run_once),
                add_test_emitters.run_if(run_once),
                add_test_portals.run_if(run_once),
            ),
        )
        .add_systems(Update, setup_egui_fonts.run_if(run_once))
        .add_systems(
            Update,
            (
                handle_player_input,
                process_action_attempts,
                process_acquisitions,
                handle_damage,
                handle_pending_transition,
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
            ..Default::default()
        },
        Actor,
        TileIdx::Skeleton,
        Interactable::Dialogue {
            name: "Mr. Boney".into(),
            text: "Hello".into(),
        },
    ));

    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 49, y: 49 },
            ..Default::default()
        },
        Interactable::Combatant {
            combat_stats: CombatStats {
                nameplate: "Mr. Sandbag".into(),
                hp: 10,
            },
        },
    ));
}

fn add_test_emitters(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Actor,
        TileIdx::Torch,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 47, y: 47 },
            ..Default::default()
        },
        Emitter::new((LightLevel::Light, 1), (LightLevel::Dim, 1)),
    ));
}

fn add_test_portals(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    // Exit
    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 50, y: 45 },
            ..Default::default()
        },
        Portal {
            id: "door_exit".into(),
            arrive_at: "door_entry".into(),
        },
    ));

    // Entry (arrival)
    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 48, y: 48 },
            ..Default::default()
        },
        Portal {
            id: "door_entry".into(),
            arrive_at: "door_exit".into(),
        },
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

const CAMERA_LAYER: TilemapLayer = TilemapLayer(0.);

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
            *CAMERA_LAYER,
        ),
    ));
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
    Combatant {
        combat_stats: CombatStats,
    },
}

/// Processes [ActionAttempt] messages, either moving the player or interacting with an interactable entity at the target [Cell] using [SpatialIndex].
fn process_action_attempts(
    mut commands: Commands,
    mut log: ResMut<event_log::MessageLog>,
    mut interactions: MessageReader<ActionAttempt>,
    mut interactables: Query<(&mut TileIdx, &mut Interactable)>,
    portals: Query<&Portal>,
    mut acquisitions: MessageWriter<Acquisition>,
    mut damages: MessageWriter<Damage>,
    player_inventory: Res<Inventory>,
    spatial_index: Res<SpatialIndex>,
) {
    for action in interactions.read() {
        let Some(target_entity) = spatial_index.get(action.target_cell) else {
            // No entity at the target [Cell], so we can assume it's an empty walkable tile.
            // Changing the [Cell] via insertion will cause the system to move the player sprite.
            commands
                .entity(action.entity)
                .insert(action.target_cell)
                .insert(PreviousCell(action.origin_cell));

            log.add("You move.", colors::KENNEY_OFF_WHITE);
            continue;
        };

        if let Ok(portal) = portals.get(target_entity) {
            commands.insert_resource(PendingTransition {
                arrive_at: portal.arrive_at.clone(),
            });
            continue;
        }

        let Ok((mut tile_idx, mut interactable)) = interactables.get_mut(target_entity) else {
            info!(
                "Player interacts with an entity at {:?}, but it's not interactable.",
                action.target_cell
            );
            continue; // There is a target entity, but it's not interactable.
        };

        handle_interaction(
            &mut commands,
            &mut tile_idx,
            &mut interactable,
            &player_inventory,
            &mut acquisitions,
            &mut damages,
            action.entity,
            &mut log,
        );
    }
}

#[derive(Message, Debug, Copy, Clone)]
pub struct Damage {
    pub amount: i32,
    pub origin: Cell,
    pub target: Cell,
}

/// Handles the interaction between the player and an interactable entity with [TileIdx] at the target [Cell].
fn handle_interaction(
    commands: &mut Commands,
    tile_idx: &mut TileIdx,
    interactable: &mut Interactable,
    inventory: &Inventory,
    acquisitions: &mut MessageWriter<Acquisition>,
    _damages: &mut MessageWriter<Damage>,
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
        Interactable::Combatant { .. } => {
            // TODO: sort out whether to do more looking up here or in the damage handler.
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct CombatStats {
    pub nameplate: String,
    pub hp: i32,
}

fn handle_damage(
    mut damages: MessageReader<Damage>,
    mut query: Query<&mut CombatStats>,
    spatial_index: Res<SpatialIndex>,
    mut log: ResMut<MessageLog>,
) {
    damages
        .read()
        .into_iter()
        .filter_map(|damage| {
            let _ = damage.origin;
            let target = damage.target;
            let entity = spatial_index.get(target)?;
            let mut stats = query.get_mut(entity).ok()?;
            stats.hp -= damage.amount;
            Some((stats.nameplate.clone(), damage.amount))
        })
        .for_each(|(nameplate, amount)| {
            log.add(
                format!("{} takes {} damage", nameplate, amount),
                colors::KENNEY_RED,
            );
        });
}

#[derive(Resource, Debug)]
struct PendingTransition {
    /// The destination will be marked by this EntryId.
    arrive_at: EntryId,
}

/// Handles the pending transition, if any.
/// Matches the EntryId in PendingTransition with the portals' EntryId to find the destination cell.
fn handle_pending_transition(
    mut commands: Commands,
    pending_transition: Option<ResMut<PendingTransition>>,
    portals: Query<(&Portal, &Cell), With<Actor>>,
    player: Single<Entity, With<Player>>,
) {
    let Some(transition) = pending_transition.as_ref() else {
        return;
    };

    info!("looking for {:?} in {:?}", transition.arrive_at, portals);
    for (portal, cell) in &portals {
        info!(
            "checking entry_id {:?} against {:?}",
            portal.id, transition.arrive_at
        );
        if portal.id == transition.arrive_at {
            info!(
                "player transitioning to entry_id {:?} at cell {:?}",
                portal.arrive_at, cell
            );
            commands.entity(*player).insert(*cell);
            commands.remove_resource::<PendingTransition>();
            return;
        }
    }
    warn!(
        "Pending transition entry_id {:?} not found in entries.",
        transition.arrive_at
    );

    commands.remove_resource::<PendingTransition>();
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
