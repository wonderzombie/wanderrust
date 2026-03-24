mod actors;
mod atlas;
mod camera;
mod cell;
mod colors;
mod combat;
mod editor;
mod event_log;
mod fov;
mod gamestate;
mod interactions;
mod inventory;
mod light;
mod map;
mod mobs;
mod procgen;
mod ptable;
mod sounds;
mod tilemap;
mod tiles;

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    actors::*,
    atlas::SpriteAtlas,
    cell::Cell,
    combat::CombatStats,
    fov::Vision,
    gamestate::GameState,
    light::{Emitter, LightLevel},
    tilemap::{EntryId, Portal, TilemapSpec},
    tiles::{TileIdx, Walkable},
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_northstar::{
    grid::{Grid, GridSettingsBuilder},
    nav::Nav,
    plugin::NorthstarPlugin,
    prelude::*,
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
        .add_message::<actors::ActionAttempt>()
        .add_message::<inventory::Acquisition>()
        .add_message::<combat::AttackAttempt>()
        .add_message::<interactions::DialogueAttempt>()
        .add_message::<interactions::InteractionAttempt>()
        .insert_state(GameState::Starting)
        .init_resource::<SpatialIndex>()
        .init_resource::<inventory::Inventory>()
        .init_resource::<editor::EditorState>()
        .init_resource::<actors::PlayerStats>()
        .init_resource::<sounds::Sounds>()
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
        .add_systems(
            PreStartup,
            (
                (
                    load_spritesheet,
                    tilemap::spawn_tilemap,
                    tilemap::initialize_tile_storage,
                )
                    .chain()
                    .in_set(Systems::SetupTiles),
                sounds::load_sounds,
            ),
        )
        .add_systems(
            Startup,
            (
                spawn_grid,
                interactions::setup_interactables,
                actors::setup_player,
                fov::setup_fov,
                camera::setup_camera,
            ),
        )
        .add_systems(
            PostStartup,
            (add_test_npc, add_test_emitters, add_test_portals).in_set(Systems::SpawnTestEntities),
        )
        .add_systems(
            PostStartup,
            (|mut next_state: ResMut<NextState<GameState>>| {
                info!("going to await input");
                next_state.set(GameState::AwaitingInput);
            })
            .after(Systems::SpawnTestEntities),
        )
        .add_systems(Update, event_log::setup_egui_fonts.run_if(run_once))
        .add_systems(EguiPrimaryContextPass, event_log::draw_message_log_ui)
        .add_systems(Update, sounds::on_sounds_loaded.run_if(run_once))
        .add_systems(
            Update,
            (
                actors::handle_player_input.run_if(in_state(GameState::AwaitingInput)),
                (
                    process_actions,
                    interactions::process_interactions,
                    interactions::process_dialogue,
                    inventory::process_acquisitions,
                    combat::process_attacks,
                    handle_pending_transition,
                )
                    .chain()
                    .in_set(Systems::Ramifications),
            ),
        )
        .add_systems(
            PostUpdate,
            (
                map::sync_tiles,
                (
                    actors::sync_actor_sprites,
                    actors::update_actor_transforms,
                    actors::sync_occupied_tiles,
                )
                    .in_set(Systems::ActorSync)
                    .after(map::sync_tiles),
                camera::update_camera.after(Systems::ActorSync),
                update_spatial_index.after(Systems::ActorSync),
                (fov::update_fov_model, fov::update_fov_markers)
                    .chain()
                    .in_set(Systems::Fov)
                    .after(Systems::ActorSync),
                (light::update_emitter_lights, light::sync_actor_light_levels)
                    .chain()
                    .in_set(Systems::Light)
                    .after(Systems::Fov),
                (
                    mobs::check_mob_fov,
                    mobs::pathfind_agents,
                    mobs::move_agents,
                )
                    .chain()
                    .in_set(Systems::Mobs)
                    .after(Systems::Fov)
                    .run_if(in_state(GameState::Ramifying)),
            ),
        )
        .add_systems(PostUpdate, combat::init_combatants)
        // TODO: consider whether to combine update_grid and update_spatial_index.
        .add_systems(PostUpdate, update_grid.after(update_spatial_index))
        .add_systems(Last, map::update_tile_visuals)
        .add_systems(OnEnter(GameState::Ramifying), gamestate::on_enter_ramifying)
        .add_systems(
            Last,
            (
                (
                    gamestate::finalize_waiting_turns,
                    gamestate::check_turns_complete,
                )
                    .chain()
                    .run_if(in_state(GameState::Ramifying)),
                mobs::handle_dead_mobs.after(Systems::Mobs),
            ),
        )
        .run();
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Systems {
    SetupTiles,
    SpawnTestEntities,
    Ramifications,
    ActorSync,
    Fov,
    Light,
    Mobs,
}

fn add_test_npc(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    grid_entity: Single<Entity, With<Grid<CardinalNeighborhood>>>,
) {
    commands.spawn((
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 53, y: 53 },
            ..default()
        },
        Actor,
        TileIdx::Skeleton,
        interactions::Interactable::Speaker {
            nameplate: "Mr. Boney".into(),
        },
        interactions::Dialogue::phrases(vec!["hello".into(), "hi".into(), "how are you".into()]),
    ));

    commands.spawn((
        Actor,
        TileIdx::Skeleton,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 49, y: 49 },
            ..default()
        },
        interactions::Interactable::Combatant,
        CombatStats {
            nameplate: "Mr. Sandbag".into(),
            max_hp: 12,
            ..default()
        },
        Vision(5),
    ));

    let cell = Cell { x: 40, y: 40 };
    commands.spawn((
        Actor,
        TileIdx::Bat,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: cell,
            ..default()
        },
        interactions::Interactable::Combatant,
        CombatStats {
            nameplate: "Bat".into(),
            max_hp: 4,
            ..default()
        },
        Vision(3),
        AgentPos(cell.into()),
        AgentOfGrid(*grid_entity),
        gamestate::Turn::Idling,
    ));
}

fn add_test_emitters(mut commands: Commands, atlas: Res<SpriteAtlas>) {
    commands.spawn((
        Actor,
        TileIdx::Torch,
        PieceBundle {
            sprite: atlas.sprite(),
            cell: Cell { x: 50, y: 48 },
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
    tiles: Query<(&Cell, Option<&Walkable>), Changed<TileIdx>>,
) {
    let mut grid = grid.into_inner();

    let mut count = 0;
    for (cell, walkable_opt) in tiles.iter() {
        let nav = if walkable_opt.is_some() {
            Nav::Passable(1)
        } else {
            Nav::Impassable
        };
        grid.set_nav(cell.into(), nav);
        count += 1;
    }

    if count > 0 {
        info!("updated grid, set {} tiles", count);
        grid.build();
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

/// Routes [ActionAttempt] messages to one of four outcomes: move, portal, interact, or blocked.
/// Interaction execution is handled by [interactions::process_interactions].
fn process_actions(
    mut commands: Commands,
    mut log: ResMut<event_log::MessageLog>,
    mut actions: MessageReader<ActionAttempt>,
    portals: Query<&Portal>,
    mut interaction_attempts: MessageWriter<interactions::InteractionAttempt>,
    spatial_index: Res<SpatialIndex>,
) {
    let mut acted = false;
    for action in actions.read() {
        let Some(target_entity) = spatial_index.get(action.target_cell) else {
            // No entity at the target [Cell], so we can assume it's an empty walkable tile.
            // Changing the [Cell] via insertion will cause the system to move the player sprite.
            commands
                .entity(action.entity)
                .insert((action.target_cell, PreviousCell(action.origin_cell)))
                .trigger(Moved);

            log.add(format!("{}", action.target_cell), colors::KENNEY_OFF_WHITE);
            acted = true;
            continue;
        };

        if let Ok(portal) = portals.get(target_entity) {
            commands.insert_resource(PendingTransition {
                arrive_at: portal.arrive_at.clone(),
            });
            acted = true;
            continue;
        }

        interaction_attempts.write(interactions::InteractionAttempt {
            interactor: action.entity,
            target: target_entity,
        });
        acted = true;
    }

    if acted {
        commands.set_state(GameState::Ramifying);
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
