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
mod loot;
mod map;
mod mobs;
mod procgen;
mod ptable;
mod sounds;
mod test_entities;
mod tilemap;
mod tiles;
mod title_screen;

use std::collections::HashMap;

use bevy::{
    prelude::*,
    window::{CursorIcon, CursorOptions, CustomCursor, CustomCursorImage},
};

use crate::{
    actors::*,
    atlas::SpriteAtlas,
    cell::Cell,
    gamestate::{GameState, Screen},
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
        .add_message::<actors::Action>()
        .add_message::<inventory::Acquisition>()
        .add_message::<combat::Attack>()
        .add_message::<interactions::Listen>()
        .add_message::<interactions::Examine>()
        .insert_state(GameState::Starting)
        .init_resource::<SpatialIndex>()
        .init_resource::<inventory::Inventory>()
        .init_resource::<actors::PlayerStats>()
        .init_resource::<sounds::Sounds>()
        .init_resource::<gamestate::WorldClock>()
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
        .add_plugins(title_screen::TitleScreenPlugin)
        .add_systems(
            PreStartup,
            (
                (
                    load_spritesheet,
                    tilemap::spawn_tilemap,
                    tilemap::initialize_tile_storage,
                )
                    .chain()
                    .in_set(GameSystem::SetupTiles),
                sounds::load_sounds,
                set_mouse_cursor.after(GameSystem::SetupTiles),
            ),
        )
        .add_systems(
            Startup,
            (
                spawn_grid,
                interactions::setup,
                actors::setup_player,
                fov::setup_fov,
                camera::setup_camera,
                add_click_observer,
            ),
        )
        .add_systems(
            PostStartup,
            (
                test_entities::add_test_npc,
                test_entities::add_test_emitters,
                test_entities::add_test_portals,
            )
                .in_set(GameSystem::SpawnTestEntities),
        )
        .add_systems(Update, event_log::setup_fonts.run_if(run_once))
        .add_systems(
            EguiPrimaryContextPass,
            event_log::draw_ui.run_if(in_state(Screen::Playing)),
        )
        .add_systems(Update, sounds::on_loaded)
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
                    .in_set(GameSystem::Ramifications),
            ),
        )
        .add_systems(
            PostUpdate,
            (
                map::sync_tiles,
                (
                    actors::sync_sprites,
                    actors::update_transforms,
                    actors::sync_occupied_tiles,
                )
                    .in_set(GameSystem::ActorSync)
                    .after(map::sync_tiles),
                camera::update.after(GameSystem::ActorSync),
                update_spatial_index.after(GameSystem::ActorSync),
                (fov::update_fov_model, fov::update_fov_markers)
                    .chain()
                    .in_set(GameSystem::Fov)
                    .after(GameSystem::ActorSync),
                (light::update_emitter_lights, light::sync_actor_light_levels)
                    .chain()
                    .in_set(GameSystem::Light)
                    .after(GameSystem::Fov),
                (mobs::check_fov, mobs::pathfind, mobs::move_agents)
                    .chain()
                    .in_set(GameSystem::Mobs)
                    .after(GameSystem::Fov)
                    .run_if(in_state(GameState::Ramifying)),
            ),
        )
        .add_systems(PostUpdate, combat::init_combatants)
        // TODO: consider whether to combine update_grid and update_spatial_index.
        .add_systems(PostUpdate, update_grid.after(update_spatial_index))
        .add_systems(OnEnter(GameState::Ramifying), gamestate::on_enter_ramifying)
        .add_systems(
            Last,
            (
                map::update_tile_visuals,
                (
                    gamestate::finalize_waiting_turns,
                    gamestate::check_turns_complete,
                )
                    .chain()
                    .run_if(in_state(GameState::Ramifying)),
                mobs::handle_dead.after(GameSystem::Mobs),
            ),
        )
        .run();
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystem {
    SetupTiles,
    SpawnTestEntities,
    Ramifications,
    ActorSync,
    Fov,
    Light,
    Mobs,
}

fn set_mouse_cursor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window: Single<Entity, With<Window>>,
    atlas: Res<SpriteAtlas>,
) {
    let handle: Handle<Image> =
        asset_server.load("kenney_1-bit-pack/Tilesheet/colored-transparent_packed.png");

    let index = tiles::atlas_idx(35, 10);
    commands
        .entity(*window)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: handle,
            texture_atlas: Some(TextureAtlas {
                layout: atlas.layout.clone(),
                index,
            }),
            ..default()
        })));
}

fn add_click_observer(mut commands: Commands) {
    commands.add_observer(click_observer);
}

fn click_observer(
    on: On<Pointer<Click>>,
    input: Res<ButtonInput<KeyCode>>,
    tile_cells: Query<(&TileIdx, &Cell)>,
    player: Single<(Entity, &Cell), With<Player>>,
    mut log: ResMut<event_log::MessageLog>,
    mut actions: MessageWriter<Action>,
) {
    let (entity, &origin_cell) = *player;
    match tile_cells.get(on.event_target()) {
        Ok((tile_idx, &cell)) => {
            let orig = origin_cell;
            let delta = orig - cell;

            if !input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                // Find direction relative to the player
                let d = delta.as_vec().normalize_or_zero();
                if d == Vec2::ZERO {
                    return;
                }
                let direction = Cell::from_vec(d);
                let target_cell = origin_cell - direction;

                if target_cell == origin_cell {
                    return;
                }

                let action = Action {
                    entity,
                    origin_cell,
                    target_cell,
                };
                info!("action: {:?}", action);
                actions.write(action);
            } else {
                log.add(format!("{} = {:?}", cell, tile_idx), Color::WHITE);
            }
        }
        Err(err) => {
            trace!("couldn't get_entity() on.event_target(): {:?}", err);
            return;
        }
    }
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
        info!("ℹ️ updated grid, set {} tiles", count);
        grid.build();
    }
}

/// A spatial index that tracks which cells are occupied by non-walkable entities in the world.
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

/// Routes [`Action`] messages. Interaction execution is handled in [`Examine`].
fn process_actions(
    mut commands: Commands,
    mut actions: MessageReader<Action>,
    portals: Query<&Portal>,
    mut interaction_attempts: MessageWriter<interactions::Examine>,
    spatial_index: Res<SpatialIndex>,
) {
    let mut acted = false;
    for action in actions.read() {
        let direction = action.target_cell - action.origin_cell;
        let adjusted_cell = action.origin_cell + direction;
        let Some(target_entity) = spatial_index.get(adjusted_cell) else {
            // No entity at the target [`Cell`], so we can assume it's an empty walkable tile.
            // Changing the [`Cell`] via insertion will cause the system to move the player sprite.
            commands
                .entity(action.entity)
                .insert((adjusted_cell, PreviousCell(action.origin_cell)))
                .trigger(Moved);

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

        interaction_attempts.write(interactions::Examine {
            interactor: action.entity,
            target: target_entity,
        });
        acted = true;
    }

    if acted {
        commands.set_state(GameState::Ramifying);
    }
}

/// The destination will be marked by this [`EntryId`].
#[derive(Resource, Debug)]
struct PendingTransition {
    arrive_at: EntryId,
}

/// Handles the pending transition, if any.
/// Matches the [`EntryId`] in [`PendingTransition`] with the portals' [`EntryId`] to find the destination cell.
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
            info!("ℹ️ portal to {:?} at cell {:?}", portal.arrive_at, cell);
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
