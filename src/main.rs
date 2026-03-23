mod actors;
mod atlas;
mod cell;
mod colors;
mod combat;
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

use bevy::{
    asset::LoadedFolder,
    audio::{PlaybackMode, Volume},
    prelude::*,
};

use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_northstar::{
    grid::{Grid, GridSettingsBuilder},
    nav::Nav,
    plugin::NorthstarPlugin,
    prelude::CardinalNeighborhood,
};
use rand::seq::IndexedRandom;

use crate::{
    actors::*,
    atlas::SpriteAtlas,
    cell::Cell,
    combat::CombatStats,
    editor::{DesiredZoom, EditorState},
    event_log::MessageLog,
    fov::{Fov, Vision},
    inventory::*,
    light::{Emitter, LightLevel},
    tilemap::{EntryId, Portal, TilemapLayer, TilemapSpec},
    tiles::{MapTile, TileIdx, Walkable},
};

/// The path to the spritesheet image.
const SHEET_PATH: &str = "kenney_1-bit-pack/Tilesheet/colored_packed.png";

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
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(NorthstarPlugin::<CardinalNeighborhood>::default())
        .add_systems(Startup, spawn_grid.after(tilemap::initialize_tile_storage))
        // TODO: consider whether to combine update_grid and update_spatial_index.
        .add_systems(PostUpdate, update_grid.after(update_spatial_index))
        .add_message::<actors::ActionAttempt>()
        .add_message::<inventory::Acquisition>()
        .add_message::<combat::AttackAttempt>()
        .add_message::<DialogueAttempt>()
        .add_message::<InteractionAttempt>()
        .init_resource::<SpatialIndex>()
        .init_resource::<inventory::Inventory>()
        .init_resource::<editor::EditorState>()
        .init_resource::<actors::PlayerStats>()
        .init_resource::<Sounds>()
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
        .insert_resource(event_log::MessageLog::new(10))
        .add_plugins(editor::EditorPlugin)
        .add_systems(Startup, load_spritesheet)
        .add_systems(
            Startup,
            (
                tilemap::spawn_tilemap.after(load_spritesheet),
                tilemap::initialize_tile_storage.after(tilemap::spawn_tilemap),
                setup_interactables.after(tilemap::initialize_tile_storage),
                actors::setup_player.after(load_spritesheet),
                fov::setup_fov.after(tilemap::initialize_tile_storage),
                setup_camera,
                load_sounds,
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
        .add_systems(Update, event_log::setup_egui_fonts.run_if(run_once))
        .add_systems(EguiPrimaryContextPass, event_log::draw_message_log_ui)
        .add_systems(Update, on_sounds_loaded.run_if(run_once))
        .add_systems(
            Update,
            (
                actors::handle_player_input,
                process_actions,
                process_interactions,
                process_dialogue,
                inventory::process_acquisitions,
                combat::process_attacks,
                handle_pending_transition,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            (
                map::sync_tiles,
                sync_actor_sprites,
                actors::sync_occupied_tiles,
                actors::update_actor_transforms,
                update_camera.after(update_actor_transforms),
                update_spatial_index,
                fov::update_fov_model.after(map::sync_tiles),
                fov::update_fov_markers.after(fov::update_fov_model),
                light::update_emitter_lights.after(fov::update_fov_markers),
                light::sync_actor_light_levels.after(light::update_emitter_lights),
                check_mob_vision.after(fov::update_fov_model),
            ),
        )
        .add_systems(PostUpdate, combat::init_combatants)
        .add_systems(Last, map::update_tile_visuals)
        .run();
}

fn add_test_npc(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            ..default()
        },
        Actor,
        TileIdx::Skeleton,
        Interactable::Speaker {
            nameplate: "Mr. Boney".into(),
            text: "Hello".into(),
        },
        Dialogue::phrases(vec!["hello".into(), "hi".into(), "how are you".into()]),
    ));

    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 49, y: 49 },
            ..default()
        },
        Interactable::Combatant,
        CombatStats {
            nameplate: "Mr. Sandbag".into(),
            max_hp: 12,
            ..default()
        },
        Vision(5),
    ));

    commands.spawn((
        Actor,
        TileIdx::Bat,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 40, y: 40 },
            ..default()
        },
        Interactable::Combatant,
        CombatStats {
            nameplate: "Bat".into(),
            max_hp: 4,
            ..default()
        },
        Vision(1),
    ));
}

fn add_test_emitters(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Actor,
        TileIdx::Torch,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 47, y: 47 },
            ..default()
        },
        Emitter::new((LightLevel::Light, 1), (LightLevel::Dim, 1)),
    ));
}

fn add_test_portals(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 50, y: 45 },
            ..default()
        },
        Portal {
            id: "door_exit".into(),
            arrive_at: "door_entry".into(),
        },
    ));

    commands.spawn((
        Actor,
        TileIdx::DoorwayBrownThick,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 48, y: 48 },
            ..default()
        },
        Portal {
            id: "door_entry".into(),
            arrive_at: "door_exit".into(),
        },
    ));
}

fn spawn_grid(mut commands: Commands, spec: Res<TilemapSpec>) {
    let grid_settings = GridSettingsBuilder::new_2d(spec.size.width, spec.size.height)
        .chunk_size(16)
        .default_impassable()
        .build();

    commands.spawn(Grid::<CardinalNeighborhood>::new(&grid_settings));
}

fn update_grid(
    grid: Single<&mut Grid<CardinalNeighborhood>>,
    tiles: Query<&Cell, Changed<TileIdx>>,
) {
    let mut grid = grid.into_inner();

    let mut count = 0;
    for walkable_cell in tiles.iter() {
        grid.set_nav(walkable_cell.into(), Nav::Passable(1));
        count += 1;
    }

    if count > 0 {
        info!("updated grid, set {} tiles", count);
        grid.build();
    }
}

fn check_mob_vision(
    mut commands: Commands,
    fov: Res<Fov>,
    visions: Query<(Entity, &TileIdx, &Cell, &Vision), (Without<Player>, With<Actor>)>,
    player: Query<&Cell, (With<Player>, Changed<Cell>)>,
) {
    let Some(player_cell) = player.single().ok() else {
        return;
    };
    for (entity, tile, mob_cell, mob_vision) in visions.iter() {
        let view = fov.from(mob_cell.into(), mob_vision.0);
        if view.has(player_cell.into()) {
            commands.entity(entity).insert(Alerted);
            info!(
                "{:?} @ {} detected player at {}",
                tile, mob_cell, player_cell
            );
        }
    }
}

#[derive(Resource, Default)]
struct Sounds {
    lookup: HashMap<String, Handle<AudioSource>>,
    folder: Handle<LoadedFolder>,
    loaded: bool,
}

fn load_sounds(mut sounds: ResMut<Sounds>, asset_server: Res<AssetServer>) {
    info!("preparing to load sounds");
    let handle = asset_server.load_folder("audio");

    *sounds = Sounds {
        folder: handle,
        loaded: false,
        ..default()
    };
}

fn on_sounds_loaded(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    asset_server: Res<AssetServer>,
) {
    if sounds.loaded {
        return;
    }

    let handle = asset_server.load_folder("audio");

    let Some(folder) = loaded_folders.get(&handle) else {
        info!("Sounds not ready");
        return;
    };

    info!("sounds loaded; initializing");
    sounds.lookup = folder
        .handles
        .iter()
        .filter_map(|handle| {
            let audio_handle = handle.clone().try_typed::<AudioSource>().ok()?;
            let path = asset_server.get_path(handle.id())?;
            let name = path.path().file_stem()?.to_string_lossy().into_owned();
            Some((name, audio_handle))
        })
        .collect();

    sounds.loaded = true;
    sounds.folder = handle;

    commands.add_observer(on_moved_sounds);

    info!("finished initializing sounds");
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

fn setup_camera(mut commands: Commands, spec: Res<TilemapSpec>) {
    let size = spec.size;
    // Spawn the camera using a 2D orthographic projection.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(
            (size.width as f32 * tiles::TILE_SIZE_PX) / 2.0 - tiles::TILE_SIZE_PX / 2.0,
            (size.height as f32 * tiles::TILE_SIZE_PX) / 2.0 - tiles::TILE_SIZE_PX / 2.0,
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
    Speaker {
        nameplate: String,
        text: String,
    },
    Combatant,
}

const GRASS_FOOTSTEPS: [&str; 5] = [
    "footstep_grass_000",
    "footstep_grass_001",
    "footstep_grass_002",
    "footstep_grass_003",
    "footstep_grass_004",
];

fn on_moved_sounds(_on: On<Moved>, mut commands: Commands, sounds: Res<Sounds>) {
    let mut rng = rand::rng();

    let rand_footstep: &'static str = GRASS_FOOTSTEPS.choose(&mut rng).unwrap();
    let Some(footstep) = sounds.lookup.get(rand_footstep) else {
        error!("footstep sound not found: {}", rand_footstep);
        return;
    };

    commands.spawn((
        AudioPlayer::new(footstep.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(0.1),
            ..default()
        },
    ));
}

/// Routes [ActionAttempt] messages to one of four outcomes: move, portal, interact, or blocked.
/// Interaction execution is handled by [process_interactions].
fn process_actions(
    mut commands: Commands,
    mut log: ResMut<event_log::MessageLog>,
    mut actions: MessageReader<ActionAttempt>,
    portals: Query<&Portal>,
    mut interaction_attempts: MessageWriter<InteractionAttempt>,
    spatial_index: Res<SpatialIndex>,
) {
    for action in actions.read() {
        let Some(target_entity) = spatial_index.get(action.target_cell) else {
            // No entity at the target [Cell], so we can assume it's an empty walkable tile.
            // Changing the [Cell] via insertion will cause the system to move the player sprite.
            let mut entity_cmd = commands.entity(action.entity);
            entity_cmd.insert((action.target_cell, PreviousCell(action.origin_cell)));
            commands.trigger(Moved(action.entity));

            log.add(format!("{}", action.target_cell), colors::KENNEY_OFF_WHITE);
            continue;
        };

        if let Ok(portal) = portals.get(target_entity) {
            commands.insert_resource(PendingTransition {
                arrive_at: portal.arrive_at.clone(),
            });
            continue;
        }

        interaction_attempts.write(InteractionAttempt {
            interactor: action.entity,
            target: target_entity,
        });
    }
}

#[derive(Message, Debug, Copy, Clone)]
struct InteractionAttempt {
    interactor: Entity,
    target: Entity,
}

#[derive(Message, Debug, Copy, Clone)]
struct DialogueAttempt {
    pub entity: Entity,
}

/// Processes [InteractionAttempt] messages, executing the interaction between the player and an [Interactable] entity.
fn process_interactions(
    mut attempts: MessageReader<InteractionAttempt>,
    mut interactables: Query<(Entity, &mut TileIdx, &mut Interactable)>,
    mut acquisitions: MessageWriter<Acquisition>,
    mut attacks: MessageWriter<combat::AttackAttempt>,
    mut speech: MessageWriter<DialogueAttempt>,
    player_inventory: Res<Inventory>,
    mut log: ResMut<event_log::MessageLog>,
) {
    for attempt in attempts.read() {
        let Ok((entity, mut tile_idx, mut interactable)) = interactables.get_mut(attempt.target)
        else {
            info!(
                "Interaction attempted with entity {:?}, but it's not interactable.",
                attempt.target
            );
            continue;
        };

        match interactable.as_mut() {
            Interactable::Door { is_open, requires } => {
                if !*is_open {
                    if let Some(required_item) = requires {
                        if !player_inventory.has_item(required_item) {
                            info!("Player lacks required item: {}", required_item.0);
                            log.add("Locked.", colors::KENNEY_BLUE);
                            continue;
                        } else {
                            info!("Player opens the door with {:?}.", required_item);
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
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
                }
            }
            Interactable::Chest { is_open, contents } => {
                if !*is_open {
                    *is_open = true;
                    tile_idx.set_if_neq(tile_idx.opened_version().unwrap_or(*tile_idx));
                    info!("Player opens chest: {:?}", contents);
                    log.add("Opened chest.", colors::KENNEY_BLUE);
                    log.add_all(contents.summary("got").as_ref(), colors::KENNEY_GREEN);
                    acquisitions.write(Acquisition {
                        items: contents.clone(),
                    });
                }
            }
            Interactable::Speaker { nameplate, .. } => {
                info!("Player talks to {}.", nameplate);
                // log.add(format!("{}: {}", nameplate, text), colors::KENNEY_BLUE);
                speech.write(DialogueAttempt { entity });
            }
            Interactable::Combatant => {
                attacks.write(combat::AttackAttempt {
                    attacker: attempt.interactor,
                    target: entity,
                });
            }
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct Dialogue {
    idx: usize,
    phrases: Vec<String>,
}

impl Dialogue {
    pub fn advance(&mut self) -> &str {
        let phrase = &self.phrases[self.idx];
        self.idx = (self.idx + 1) % self.phrases.len();
        phrase
    }

    pub fn phrases(phrases: Vec<String>) -> Self {
        Self { idx: 0, phrases }
    }
}

fn process_dialogue(
    mut speech: MessageReader<DialogueAttempt>,
    mut log: ResMut<MessageLog>,
    mut dialogues: Query<&mut Dialogue>,
) {
    for attempt in speech.read() {
        let Ok(mut dialogue) = dialogues.get_mut(attempt.entity) else {
            continue;
        };

        log.add(dialogue.advance(), colors::KENNEY_BLUE);
    }
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
        if portal.id == transition.arrive_at {
            info!("portal to {:?} at cell {:?}", portal.arrive_at, cell);
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
        error!("No player entity found in the world.");
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        error!("No camera entity found in the world.");
        return;
    };

    camera_transform.translation.x =
        (player_cell.x as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
    camera_transform.translation.y =
        (player_cell.y as f32 * tiles::TILE_SIZE_PX) + (tiles::TILE_SIZE_PX / 2.0);
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
