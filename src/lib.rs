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
mod dialogue_modal;
mod effects;
mod equipment;
mod equipment_menu;
mod fov;
pub mod gamestate;
mod grid;
mod interacticator;
mod interactions;
pub mod interxables;
mod intro_screen;
mod inventory;
mod inventory_menu;
mod items;
mod ldtk_loader;
pub mod light;
mod loot;
mod macros;
mod map;
mod message_log;
mod mobs;
mod parameters;
mod procgen;
mod ptable;
mod sounds;
mod status_panel;
pub mod tilemap;
pub mod tiles;
mod title_screen;
mod tooltip;
mod typewriter;
mod ui;
mod you_died_screen;

use bevy::{
    asset::io::web::WebAssetPlugin,
    prelude::*,
    window::{CursorIcon, CustomCursor, CustomCursorImage},
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::FilterQueryInspectorPlugin};

use crate::{
    actors::*,
    ascii_map::AsciiMapSpec,
    atlas::SpriteAtlas,
    bestiary::Bestiary,
    cell::{Cell, PreviousCell},
    gamestate::{
        AddRecovery, AddTurnTimerDelay, DEFAULT_TURN_DELAY, GameState, Modal, Recovery, Screen,
        TurnDelay,
    },
    items::ItemId,
    ldtk_loader::LdtkProject,
    map::update_level_visuals,
    message_log::LogEvent,
    parameters::{Health, Parameters},
    tilemap::{ActiveLevel, EntryId, Level, Portal, WorldSpec},
    tiles::{MapTile, TileIdx},
};
use bevy_northstar::{plugin::NorthstarPlugin, prelude::*};

use clap::Parser;

/// The clear color for the window.
const CLEAR_COLOR: ClearColor = ClearColor(Color::srgb(71.0 / 255.0, 45.0 / 255.0, 60.0 / 255.0));

const DEFAULT_MAP_PATH: &str = "data/wandrs_proto.ldtk";

#[derive(Resource, Debug)]
struct LdtkMapPath(String);

impl Default for LdtkMapPath {
    fn default() -> Self {
        Self(DEFAULT_MAP_PATH.into())
    }
}

fn insert_fq_plugins(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Actor>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<ItemId>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Bestiary>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Recovery>>::default())
        .add_plugins(FilterQueryInspectorPlugin::<With<Level>>::default());
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    use_str_map: bool,

    #[arg(short, long)]
    inspector: bool,

    #[arg(short, long)]
    procedural_map: bool,

    #[arg(short, long, default_value = DEFAULT_MAP_PATH)]
    ldtk_map_path: String,
}

pub fn run() {
    let args = Args::parse();
    let mut app = App::new();

    if args.use_str_map {
        app.insert_resource(WorldSpec::from(AsciiMapSpec::from_str(ascii_map::MAP_ZERO)));
    } else if args.procedural_map {
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
    .insert_resource(TurnDelay(0.15))
    .insert_resource(CLEAR_COLOR)
    .insert_resource(LdtkMapPath(args.ldtk_map_path))
    .insert_resource(SpritePickingSettings {
        // clicking on a sprite ignores alpha transparency
        picking_mode: SpritePickingMode::BoundingBox,
        ..Default::default()
    })
    .insert_state(GameState::Starting)
    .insert_state(Modal::None)
    .add_plugins(gamestate::plugin)
    .add_plugins(NorthstarPlugin::<CardinalNeighborhood>::default())
    .add_plugins(typewriter::plugin)
    .add_plugins(debug::DebugPlugin)
    .add_plugins(title_screen::TitleScreenPlugin)
    .add_plugins(inventory_menu::InventoryMenuPlugin)
    .add_plugins(equipment_menu::EquipmentMenuPlugin)
    .add_plugins(dialogue_modal::plugin)
    .add_plugins(equipment::plugin)
    .add_plugins(status_panel::plugin)
    .add_plugins(you_died_screen::YouDiedScreenPlugin)
    .add_plugins(interactions::plugin)
    .add_plugins(inventory::plugin)
    .add_plugins(mobs::plugin)
    .add_plugins(message_log::plugin)
    .add_plugins(intro_screen::plugin)
    .add_plugins(ui::plugin)
    .add_plugins(grid::plugin)
    .add_plugins(effects::plugin)
    .add_systems(
        Startup,
        (atlas::load_spritesheet, sounds::load_sounds, load_ldtk),
    )
    .add_systems(
        Update,
        (
            finalize_starting
                .run_if(resource_exists::<sounds::Sounds>)
                .run_if(resource_exists::<atlas::SpriteAtlas>),
            sounds::on_loaded.run_if(not(resource_exists::<sounds::Sounds>)),
        )
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
        tilemap::despawn_worldmap,
    )
    .add_systems(
        OnEnter(GameState::Loading),
        (
            (
                tilemap::spawn_worldmap,
                map::sync_tiles,
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
        (actors::spawn_player, interactions::spawn_interxs),
    )
    .add_systems(PreUpdate, (snapshot_cells, tilemap::snapshot_denizens))
    .add_systems(
        Update,
        (
            actors::handle_player_input
                .run_if(in_state(GameState::AwaitingInput))
                .before(GameSystem::Ramifications),
            (
                process_actions,
                interactions::process_interactions,
                inventory::process_inventory_changes,
                combat::process_attacks,
                handle_pending_transition,
            )
                .chain()
                .after(PathingSet)
                .in_set(GameSystem::Ramifications),
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
            map::sync_tiles.in_set(GameSystem::SyncTiles),
            (actors::update_transforms, actors::sync_occupied_tiles)
                .in_set(GameSystem::ActorSync)
                .after(GameSystem::SyncTiles),
            camera::update.after(GameSystem::ActorSync),
            // Changes to tiles mean updates to pathing and "collision."
            (grid::update_spatial_index, grid::rebuild_grid)
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
            (mobs::check_fov, grid::pathfind, mobs::consume_turn)
                .chain()
                .in_set(GameSystem::Grid)
                .after(GameSystem::Fov)
                .run_if(in_state(GameState::Ramifying)),
            (
                mobs::detect_mobs,
                combat::init_combatants,
                combat::set_mob_spawns,
                grid::init_agents,
            )
                .in_set(GameSystem::Mobs)
                .after(GameSystem::Grid)
                .chain(),
            actors::on_player_added,
        ),
    )
    .add_systems(
        Last,
        (
            map::update_level_visuals,
            map::update_tile_visuals.after(update_level_visuals),
            gamestate::respawn_player,
            gamestate::respawn_combatants,
            gamestate::reset_doors,
        ),
    )
    .add_observer(click_observer)
    .add_observer(gamestate::player_died);

    if args.inspector {
        insert_fq_plugins(&mut app);
    }

    app.run();
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystem {
    SetupTiles,
    SetupGrid,
    SpawnTestEntities,
    SyncTiles,
    Ramifications,
    ActorSync,
    Fov,
    Light,
    Grid,
    Mobs,
}

fn load_ldtk(mut commands: Commands, map_path: Res<LdtkMapPath>) {
    let fname = map_path.0.as_str();
    let res = ldtk_loader::load_and_import(fname.into()).expect("expected to load ldtk level");
    commands.insert_resource(res);
}

fn finalize_starting(mut next: ResMut<NextState<GameState>>) {
    info!("✅ done [STARTING]");
    next.set(GameState::Loading);
}

fn finalize_loading(
    mut next: ResMut<NextState<GameState>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    info!("✅ done [LOADING]");
    next.set(GameState::AwaitingInput);
    next_screen.set(Screen::Title);
}

fn snapshot_cells(mut query: Query<(Ref<Cell>, &mut PreviousCell), Without<MapTile>>) {
    for (curr, mut prev) in query.iter_mut() {
        *prev = PreviousCell(*curr);
    }
}

fn click_observer(
    on: On<Pointer<Click>>,
    tile_cells: Query<(&TileIdx, &Cell, Option<&Name>)>,
    mut log: MessageWriter<LogEvent>,
) {
    match tile_cells.get(on.event_target()) {
        Ok((tile_idx, &cell, name_opt)) => {
            if on.button == PointerButton::Primary {
                let name = name_opt
                    .map(|it| it.to_string())
                    .unwrap_or(format!("{tile_idx}"));
                log.write((format!("{cell} = {name}").as_str(), Color::WHITE).into());
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
    player: Single<(&Parameters, &mut Health, &mut Flasks), With<Player>>,
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

    let (params, mut health, mut flasks) = player.into_inner();

    match action.act {
        Act::Move(_) => {
            let adjusted_cell = action.adjusted_cell();

            match spatial_index.get(adjusted_cell) {
                None => {
                    trace!("move: recovery after: {}", params.move_speed);
                    commands
                        .entity(action.entity)
                        .insert(adjusted_cell)
                        .queue(AddRecovery(params.move_speed))
                        .trigger(Moved);
                }
                Some(target) if portals.get(target).is_ok() => {
                    let portal = portals.get(target).unwrap();
                    info!("process_actions: portal");
                    // TODO: extract to constant.
                    commands
                        .entity(action.entity)
                        .queue(AddRecovery(params.move_speed));
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
        Act::Pass => {
            commands
                .entity(action.entity)
                .queue(AddRecovery(params.move_speed));
        }
        Act::Flask => {
            if flasks.0 > 0 {
                health.hp = params.max_hp.cast_signed().min(health.hp + 8);
                flasks.0 -= 1;
                commands
                    .entity(action.entity)
                    .queue(AddRecovery(params.move_speed))
                    .commands()
                    .trigger(sounds::Quaffed);
            } else {
                commands.write_message(LogEvent {
                    txt: "no more flasks.".into(),
                    color: Some(colors::KENNEY_RED),
                });
            }
        }
        Act::Attack(_) => todo!(),
    }

    trace!("ramifying actions");
    commands.queue(AddTurnTimerDelay(Some(DEFAULT_TURN_DELAY * 0.5)));
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

pub fn unwrap_collection<T, O>(collection: Option<&T>) -> O
where
    T: Component + RelationshipTarget,
    O: FromIterator<Entity> + Default,
{
    collection
        .map(|coll| coll.iter().collect::<O>())
        .unwrap_or_default()
}
