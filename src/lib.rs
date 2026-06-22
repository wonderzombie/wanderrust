mod actors;
mod ascii_map;
mod atlas;
mod bestiary;
mod camera;
pub mod cell;
mod colors;
mod combat;
pub mod debug;
mod diagnostics;
mod effects;
mod equipment;
mod event_log;
mod fov;
pub mod gamestate;
mod grid;
mod interactions;
mod inventory;
mod ldtk_loader;
pub mod light;
mod loot;
mod macros;
mod map;
mod mobs;
mod parameters;
mod procgen;
mod ptable;
mod sounds;
pub mod tilemap;
pub mod tiles;
mod title_screen;
mod tooltip;
mod you_died_screen;

use bevy::{
    asset::io::web::WebAssetPlugin,
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage},
};
use bevy_inspector_egui::quick::FilterQueryInspectorPlugin;

use crate::{
    actors::*,
    ascii_map::AsciiMapSpec,
    atlas::SpriteAtlas,
    cell::{Cell, PreviousCell},
    gamestate::{GameState, Recovery, Screen, TurnDelay, WorldClock},
    interactions::Interactable,
    ldtk_loader::LdtkProject,
    map::update_level_visuals,
    parameters::Parameters,
    tilemap::{ActiveLevel, EntryId, Portal, TileStorage, WorldSpec},
    tiles::TileIdx,
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use bevy_northstar::{plugin::NorthstarPlugin, prelude::*};

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

fn insert_fq_plugins(app: &mut App) {
    app.add_plugins(FilterQueryInspectorPlugin::<With<Actor>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Interactable>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<TileStorage>>::default());
}

pub fn run() {
    let args = std::env::args().collect::<Vec<_>>();
    let str_map = args.iter().any(|it| it == "-s");
    let query_filter_panes = args.iter().any(|it| it == "-i");
    let proc_map = args.iter().any(|it| it == "-p");

    let mut app = App::new();

    if str_map {
        app.insert_resource(WorldSpec::from(AsciiMapSpec::from_str(ascii_map::MAP_ZERO)));
    } else if proc_map {
        app.insert_resource(WorldSpec::from(AsciiMapSpec::with_ptable(
            procgen::biome_ptable(),
            procgen::tile_idx_for_cell,
            (100, 100),
        )));
    }

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (800, 600).into(),
                    title: "wanderrust".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(WebAssetPlugin {
                silence_startup_warning: true,
            })
            .set(AssetPlugin::default())
            .set(bevy::log::LogPlugin {
                // level: Level::TRACE,
                // filter: "bevy_asset=trace".to_string(),
                ..default()
            }),
    )
    .add_message::<combat::Attack>()
    .init_resource::<actors::PlayerStats>()
    .init_resource::<atlas::SpriteAtlas>()
    .init_resource::<gamestate::WorldClock>()
    .init_resource::<sounds::Sounds>()
    .insert_resource(TurnDelay(0.15))
    .insert_resource(CLEAR_COLOR)
    .insert_resource(SpritePickingSettings {
        // clicking on a sprite ignores alpha transparency
        picking_mode: SpritePickingMode::BoundingBox,
        ..Default::default()
    })
    .insert_resource(event_log::MessageLog::new(10))
    .insert_state(GameState::Starting)
    .add_plugins(EguiPlugin::default())
    .add_plugins(NorthstarPlugin::<CardinalNeighborhood>::default())
    .add_plugins(debug::DebugPlugin)
    .add_plugins(title_screen::TitleScreenPlugin)
    .add_plugins(you_died_screen::YouDiedScreenPlugin)
    .add_plugins(interactions::plugin)
    .add_plugins(inventory::plugin)
    .add_plugins(mobs::plugin)
    .add_plugins(equipment::plugin)
    .add_systems(
        Startup,
        (atlas::load_spritesheet, sounds::load_sounds, load_ldtk),
    )
    .add_systems(
        Update,
        (finalize_starting, atlas::on_loaded, sounds::on_loaded)
            .run_if(in_state(GameState::Starting)),
    )
    .add_systems(
        OnExit(GameState::Starting),
        (camera::setup_camera, tooltip::setup, set_mouse_cursor),
    )
    .add_systems(
        OnTransition::<GameState> {
            exited: GameState::AwaitingInput,
            entered: GameState::Loading,
        },
        tilemap::despawn_tilemap,
    )
    .add_systems(
        OnEnter(GameState::Loading),
        (
            (
                // tilemap::spawn_tilemap,
                tilemap::spawn_worldmap,
                tilemap::initialize_tile_storage,
                tilemap::setup_portals,
            )
                .chain()
                .in_set(GameSystem::SetupTiles),
            (
                grid::setup_spatial_indices,
                grid::spawn_grid,
                light::spawn,
                light::setup.after(light::spawn),
                fov::setup_fov,
            )
                .in_set(GameSystem::SetupGrid)
                .after(GameSystem::SetupTiles),
            finalize_loading.after(GameSystem::SetupGrid),
        ),
    )
    .add_systems(
        OnExit(GameState::Loading),
        (actors::setup_player, interactions::spawn_interxs),
    )
    .add_systems(
        EguiPrimaryContextPass,
        event_log::draw_ui.run_if(in_state(Screen::Playing)),
    )
    .add_systems(
        Update,
        (
            actors::handle_player_input
                .run_if(in_state(GameState::AwaitingInput))
                .before(GameSystem::Ramifications),
            (
                process_actions,
                interactions::process_interactions,
                interactions::process_dialogue,
                inventory::process_acquisitions,
                combat::process_attacks,
                handle_pending_transition,
            )
                .chain()
                .after(PathingSet)
                .in_set(GameSystem::Ramifications),
            event_log::setup_fonts.run_if(not(resource_exists::<event_log::EguiFontsLoaded>)),
            combat::animate_floating_text,
            combat::animate_icons,
            ldtk_loader::generate_ldtk_world.run_if(resource_added::<LdtkProject>),
        ),
    )
    .add_systems(
        PostUpdate,
        (
            // Runs when there's been a change to an tile and updates sprite &
            // gameplay properties.
            map::sync_tiles,
            (actors::update_transforms, actors::sync_occupied_tiles).in_set(GameSystem::ActorSync),
            camera::update.after(GameSystem::ActorSync),
            // Changes to tiles mean updates to pathing and "collision."
            (grid::update_spatial_index, grid::update_grid)
                .chain()
                .in_set(GameSystem::Grid)
                .after(GameSystem::ActorSync),
            // Update the FOV model and/or markers.
            (fov::update_fov_model, fov::update_fov_markers)
                .chain()
                .in_set(GameSystem::Fov)
                .after(GameSystem::ActorSync),
            (
                light::update_emitter_maps,
                light::update_level_maps,
                light::update_level_light_levels,
                light::sync_actor_light_levels,
            )
                .chain()
                .in_set(GameSystem::Light)
                .after(GameSystem::Fov),
            // TODO: consider if check_fov should be in fov
            (mobs::check_fov, grid::pathfind, grid::move_agents)
                .chain()
                .in_set(GameSystem::Grid)
                .after(GameSystem::Fov)
                .run_if(in_state(GameState::Ramifying)),
            combat::detect_belligerents,
            combat::init_combatants,
            grid::init_agents,
            actors::on_player_added,
            effects::apply_params_modifiers,
        ),
    )
    .add_systems(
        Update,
        gamestate::ramify.run_if(in_state(GameState::Ramifying)),
    )
    .add_systems(
        OnEnter(GameState::AwaitingInput),
        tilemap::snapshot_denizens,
    )
    .add_systems(OnExit(GameState::AwaitingInput), snapshot_cells)
    .add_systems(
        OnTransition::<GameState> {
            exited: GameState::Defeat,
            entered: GameState::AwaitingInput,
        },
        gamestate::respawn,
    )
    .add_systems(
        Last,
        (
            map::update_level_visuals,
            map::update_tile_visuals.after(update_level_visuals),
        ),
    )
    .add_observer(click_observer);

    if query_filter_panes {
        insert_fq_plugins(&mut app);
    }

    app.run();
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystem {
    SetupTiles,
    SetupGrid,
    SpawnTestEntities,
    Ramifications,
    ActorSync,
    Fov,
    Light,
    Grid,
}

fn load_ldtk(mut commands: Commands) {
    let fname = "data/wandrs_proto.ldtk";
    let res = ldtk_loader::load_and_import(fname.into()).expect("expected to load ldtk level");
    commands.insert_resource(res);
}

fn finalize_starting(
    mut next: ResMut<NextState<GameState>>,
    atlas: Res<atlas::SpriteAtlas>,
    sounds: Res<sounds::Sounds>,
) {
    trace!("atlas {:?} sounds {:?}", atlas.loaded, sounds.loaded);
    if atlas.loaded && sounds.loaded {
        info!("✅ done [STARTING]");
        next.set(GameState::Loading);
    }
}

fn finalize_loading(mut next: ResMut<NextState<GameState>>) {
    info!("✅ done [LOADING]");
    next.set(GameState::AwaitingInput);
}

fn snapshot_cells(mut query: Query<(Ref<Cell>, &mut PreviousCell)>) {
    for (curr, mut prev) in query.iter_mut() {
        if curr.is_changed() {
            *prev = PreviousCell(*curr);
        }
    }
}

fn click_observer(
    on: On<Pointer<Click>>,
    tile_cells: Query<(&TileIdx, &Cell, Option<&Name>)>,
    mut log: ResMut<event_log::MessageLog>,
) {
    match tile_cells.get(on.event_target()) {
        Ok((tile_idx, &cell, name_opt)) => {
            if on.button == PointerButton::Primary {
                let name = name_opt
                    .map(|it| it.to_string())
                    .unwrap_or(format!("{tile_idx}"));
                log.add(format!("{cell} = {name}"), Color::WHITE);
            }
        }
        Err(err) => {
            trace!("couldn't get_entity() on.event_target(): {err:?}");
        }
    }
}

fn set_mouse_cursor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window: Single<Entity, With<Window>>,
    atlas: Res<SpriteAtlas>,
) {
    let handle: Handle<Image> = asset_server.load(atlas::TRANSPARENT_SHEET);
    commands
        .entity(*window)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            texture_atlas: Some(TextureAtlas {
                layout: atlas.layout.clone(),
                index: TileIdx::Cursor1.into(),
            }),
            ..default()
        })));
}

/// Routes [`Action`] messages. Interaction execution is handled in [`Examine`].
fn process_actions(
    mut commands: Commands,
    action: If<Res<Action>>,
    portals: Query<&Portal>,
    mut interaction_attempts: MessageWriter<interactions::Examine>,
    all_spatial: Query<&grid::SpatialIndex>,
    actors: Query<&ChildOf, With<Actor>>,
    player: Single<&Parameters, With<Player>>,
    clock: Res<WorldClock>,
) {
    trace!("{action:?}");
    commands.remove_resource::<Action>();

    let Some(spatial_index) = actors
        .get(action.entity)
        .and_then(|e| all_spatial.get(e.parent()))
        .ok()
    else {
        warn!("no spatial index for {:?}; dropping action", action.entity);
        return;
    };

    match action.act {
        Act::Direction(_) => {
            let adjusted_cell = action.adjusted_cell();

            match spatial_index.get(adjusted_cell) {
                None => {
                    info!("move: recovery after: {}", player.move_speed);
                    commands
                        .entity(action.entity)
                        .insert(adjusted_cell)
                        .insert(clock.recovery_after(player.move_speed))
                        .trigger(Moved);
                }
                Some(target) if portals.get(target).is_ok() => {
                    let portal = portals.get(target).unwrap();
                    info!("process_actions: portal");
                    // TODO: extract to constant.
                    commands.entity(action.entity).insert(Recovery(1));
                    commands.insert_resource(PendingTransition {
                        arrive_at: portal.arrive_at.clone(),
                    });
                }
                Some(target) => {
                    info!("process_actions: interaction");
                    interaction_attempts.write(interactions::Examine {
                        interactor: action.entity,
                        target,
                    });
                }
            }
        }
        Act::Pass => (),
    }

    trace!("ramifying actions");
    commands.set_state(GameState::Ramifying);
}

/// The destination will be marked by this [`EntryId`].
#[derive(Resource, Debug)]
struct PendingTransition {
    arrive_at: EntryId,
}

/// Handles the pending transition, if any. Matches the [`EntryId`] in
/// [`PendingTransition`] with the portals' [`EntryId`] to find the destination
/// cell.
fn handle_pending_transition(
    mut commands: Commands,
    transition: If<ResMut<PendingTransition>>,
    active_level: Single<Entity, With<ActiveLevel>>,
    portals: Query<(&Portal, &Cell, &ChildOf), With<Actor>>,
    player: Single<Entity, With<Player>>,
) {
    info!("looking for {:?} in {portals:?}", transition.arrive_at);
    for (portal, cell, portal_child_of) in &portals {
        if portal.id == transition.arrive_at {
            info!("ℹ️ portal to {:?} at cell {cell}", portal.arrive_at);

            if portal_child_of.parent() != *active_level {
                commands
                    .entity(portal_child_of.parent())
                    .insert(ActiveLevel);
                commands.entity(*active_level).remove::<ActiveLevel>();
                commands
                    .entity(*player)
                    .insert(ChildOf(portal_child_of.parent()));
            }

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
